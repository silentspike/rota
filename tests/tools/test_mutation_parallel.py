"""Exercise concurrent mutation stages, including real child-process cleanup."""
import json
import os
import shlex
import signal
import subprocess
import sys
import threading
import time
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

import test_mutation_evidence as evidence_tests

RUST_PASS, TS_FAIL, TS_PASS = evidence_tests.RUST_PASS, evidence_tests.TS_FAIL, evidence_tests.TS_PASS


class MutationParallel(unittest.TestCase):
    setUp = evidence_tests.MutationEvidence.setUp
    drive = evidence_tests.MutationEvidence.drive
    missing = evidence_tests.MutationEvidence.missing
    assert_restored = evidence_tests.MutationEvidence.assert_restored

    def child(self, name):
        code = ('import os,signal,sys,time; from pathlib import Path; '
                'signal.signal(signal.SIGTERM, signal.SIG_IGN); '
                'Path(sys.argv[1]).write_text(str(os.getpid())); time.sleep(60)')
        return shlex.join([sys.executable, '-c', code, str(self.root / f'{name}.pid')])

    def grandchild(self, name):
        """A worker one level below the shell, as dash always starts it.

        dash 0.5.12 forks for `sh -c 'cmd'` (measured: the command got its own
        PID), bash execs a lone command. The trailing `exit` makes bash fork as
        well, so the worker is a grandchild of the tool under every /bin/sh —
        as the real workers below npx and cargo remote are anyway. Its 32 MiB
        ballast lengthens its exit after SIGKILL to well above the time the
        tool needs to return. The pid file appears atomically: its existence
        implies its content.
        """
        code = ('import os,signal,sys,time; from pathlib import Path; '
                'signal.signal(signal.SIGTERM, signal.SIG_IGN); ballast = bytearray(32 << 20); '
                'tmp = Path(sys.argv[1] + ".tmp"); tmp.write_text(str(os.getpid())); '
                'tmp.rename(sys.argv[1]); time.sleep(20)')
        return shlex.join([sys.executable, '-c', code, str(self.root / f'{name}.pid')]) + '; exit $?'

    @staticmethod
    def process_state(pid):
        """The state letter from /proc, or 'gone' once the process was reaped."""
        try:
            return Path(f'/proc/{pid}/stat').read_text().rpartition(') ')[2][0]
        except (FileNotFoundError, ProcessLookupError):
            return 'gone'

    def kill_leftovers(self):
        """Even a broken implementation must not leave this test's workers behind."""
        for path in self.root.glob('*.pid'):
            pid = int(path.read_text())
            try:
                if str(self.root) in Path(f'/proc/{pid}/cmdline').read_text():
                    os.kill(pid, signal.SIGKILL)
            except (FileNotFoundError, ProcessLookupError):
                pass

    def wait_started(self, name):
        path = self.root / f'{name}.pid'
        deadline = time.monotonic() + 5
        while not path.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(path.exists(), f'{name} never started')

    def assert_children_stopped(self):
        self.assertEqual(self.tool._LAUFEND, set())
        for path in self.root.glob('*.pid'):
            stat = Path(f'/proc/{path.read_text()}/stat')
            # A reparented zombie has exited and cannot read the restored data.
            try:
                state = stat.read_text().split(') ', 1)[1][0]
            except FileNotFoundError:
                continue
            self.assertEqual(state, 'Z', path.name)

    def assert_aborted_and_restored(self):
        self.assert_children_stopped()
        self.assert_restored()
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(len(evidence['fields']), 1)
        self.assertEqual(evidence['fields']['heal.amount']['status'], 'ABBRUCH')
        self.assertTrue(evidence['fields']['heal.amount']['restored'])
        self.assertFalse(evidence['completed'] or evidence['full_pass'])

    def test_ts_overlaps_build_and_rust_without_restoring_between_them(self):
        ts_started = {field: threading.Event() for field in ('heal.amount', 'shield.amount')}
        rust_finished = {field: threading.Event() for field in ts_started}
        observed = []

        def process(command, _timeout):
            field = self.missing()
            if field is None:
                return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)
            if command.startswith('npx'):
                ts_started[field].set()
                self.assertTrue(rust_finished[field].wait(5))
                observed.append(self.missing())
                return 1, TS_FAIL
            self.assertTrue(ts_started[field].wait(5))
            if '--no-fail-fast' in command:
                rust_finished[field].set()
            return 0, RUST_PASS

        self.assertEqual(self.drive(process), 0)
        self.assertEqual(observed, ['heal.amount', 'shield.amount'])
        self.assert_restored()

    def test_timeout_stops_both_real_children_before_restoring_and_never_starts_next_field(self):
        real_run = self.tool.lauf
        worker_exit_data = []

        def process(command, _timeout):
            if self.missing() is None:
                return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)
            if command.startswith('npx'):
                try:
                    return real_run(self.child('ts'), 10)
                finally:
                    worker_exit_data.append(self.missing())
            self.wait_started('ts')
            return real_run(self.child('rust'), 0.5)

        with self.assertRaises(subprocess.TimeoutExpired):
            self.drive(process)
        self.assertEqual(worker_exit_data, ['heal.amount'])
        self.assert_aborted_and_restored()

    def test_ts_exception_stops_real_rust_child_before_restoring(self):
        real_run = self.tool.lauf

        def process(command, _timeout):
            if self.missing() is None:
                return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)
            if command.startswith('npx'):
                self.wait_started('rust')
                raise RuntimeError('TS worker failed')
            return real_run(self.child('rust'), 10)

        with self.assertRaisesRegex(RuntimeError, 'TS worker failed'):
            self.drive(process)
        self.assert_aborted_and_restored()

    def test_sigterm_stops_both_real_children_and_joins_before_restoring(self):
        real_run = self.tool.lauf
        worker_exit_data = []

        def terminate_when_both_started():
            self.wait_started('ts')
            self.wait_started('rust')
            os.kill(os.getpid(), signal.SIGTERM)

        sender = threading.Thread(target=terminate_when_both_started)

        def process(command, _timeout):
            if self.missing() is None:
                return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)
            if command.startswith('npx'):
                try:
                    return real_run(self.child('ts'), 10)
                finally:
                    worker_exit_data.append(self.missing())
            sender.start()
            return real_run(self.child('rust'), 10)

        try:
            with self.assertRaises(SystemExit) as exit_result:
                self.drive(process)
            self.assertEqual(exit_result.exception.code, 143)
        finally:
            if sender.ident is not None:
                sender.join(timeout=5)
        self.assertEqual(worker_exit_data, ['heal.amount'])
        self.assert_aborted_and_restored()

    def test_ending_the_groups_waits_until_no_grandchild_still_runs(self):
        # Every abort path — timeout, a failing branch, SIGTERM — calls
        # `_alle_beenden()` and restores the data right after it returns. So it
        # must not return while a member of a group still runs. `p.wait()`
        # reaps the shell only; the worker is its child.
        errors = []

        def run():
            try:
                self.tool.lauf(self.grandchild('rust'), 10)
            except BaseException as error:  # the abort surfaces in the branch
                errors.append(error)

        branch = threading.Thread(target=run)
        branch.start()
        try:
            self.wait_started('rust')
            worker = int((self.root / 'rust.pid').read_text())
            self.tool._alle_beenden()
            state = self.process_state(worker)
        finally:
            branch.join(timeout=10)
            self.kill_leftovers()
        self.assertFalse(branch.is_alive())
        self.assertIn(state, ('Z', 'gone'), 'still running when the groups counted as ended')
        self.assertEqual([type(error) for error in errors], [InterruptedError])
        self.assertEqual(self.tool._LAUFEND, set())

    def test_sigterm_can_reenter_the_process_registry_lock(self):
        def process(command, _timeout):
            if self.missing() and command.startswith('cargo'):
                with self.tool._LAUFEND_LOCK:
                    signal.raise_signal(signal.SIGTERM)
            return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)

        with self.assertRaises(SystemExit) as exit_result:
            self.drive(process)
        self.assertEqual(exit_result.exception.code, 143)
        self.assert_aborted_and_restored()

    def test_sigterm_inside_popen_waitlock_or_before_registration_cannot_deadlock(self):
        driver = textwrap.dedent('''
            import importlib.util, os, shlex, signal, subprocess, sys
            from pathlib import Path
            tool_path, root, mode = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
            spec = importlib.util.spec_from_file_location('signal_probe', tool_path)
            tool = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(tool)
            tool.WURZEL = root
            tool.eingecheckter_stand = lambda: {
                s: (root / f'data/spells/{s}.json').read_text() for s in tool.SCHULEN}
            actual_popen, actual_run = subprocess.Popen, tool.lauf

            def instrumented_popen(*args, **kwargs):
                child = actual_popen(*args, **kwargs)
                if kwargs.get('start_new_session'):
                    (root / f'{mode}.pid').write_text(str(child.pid))
                    if mode == 'registration':
                        signal.raise_signal(signal.SIGTERM)
                    else:
                        communicate = child.communicate
                        def under_wait_lock(*a, **kw):
                            with child._waitpid_lock:
                                signal.raise_signal(signal.SIGTERM)
                            return communicate(*a, **kw)
                        child.communicate = under_wait_lock
                return child

            subprocess.Popen = instrumented_popen
            command = shlex.join([sys.executable, '-c', 'import time; time.sleep(60)'])
            tool.lauf = lambda *_: actual_run(command, 30)
            sys.argv = ['streich-mutation.py', '--feld', 'heal.amount',
                        '--aus', str(root / 'result.md'), '--nachweis', str(root / 'evidence.json'),
                        '--bau-log', str(root / 'build.log')]
            try:
                tool.main()
            except SystemExit as stop:
                assert stop.code == 143, stop.code
                assert not tool._LAUFEND, 'child lost from cleanup'
                print('signal safely reaped')
            else:
                raise AssertionError('SIGTERM did not stop the run')
        ''')
        for mode in ('waitlock', 'registration'):
            with self.subTest(mode=mode):
                try:
                    result = subprocess.run(
                        [sys.executable, '-B', '-c', driver,
                         str(evidence_tests.ROOT / 'tools/streich-mutation.py'), str(self.root), mode],
                        capture_output=True, text=True, timeout=10, check=False)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn('signal safely reaped', result.stdout)
                    self.assert_children_stopped()
                    self.assert_restored()
                finally:
                    # Even a broken implementation must not leak this test's child.
                    pidfile = self.root / f'{mode}.pid'
                    if pidfile.exists():
                        try:
                            os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                        except ProcessLookupError:
                            pass

    def completion_signal(self, during_restore):
        original_report = self.tool.schreibe
        original_write = Path.write_bytes
        last_report = False

        def report(path, results):
            nonlocal last_report
            original_report(path, results)
            if len(results) == 2 and self.missing() == 'shield.amount':
                last_report = True
                if not during_restore:
                    signal.raise_signal(signal.SIGTERM)

        def restore(path, data):
            result = original_write(path, data)
            if during_restore and last_report and path.name == 'calcination.json':
                signal.raise_signal(signal.SIGTERM)
            return result

        def process(command, _timeout):
            if command.startswith('npx'):
                return (1, TS_FAIL) if self.missing() else (0, TS_PASS)
            return 0, RUST_PASS

        with patch.object(self.tool, 'schreibe', side_effect=report), \
             patch.object(Path, 'write_bytes', restore), \
             self.assertRaises(SystemExit) as exit_result:
            self.drive(process)
        self.assertEqual(exit_result.exception.code, 143)
        self.assert_restored()
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(len(evidence['fields']), 2)
        self.assertTrue(all(field['restored'] for field in evidence['fields'].values()))
        self.assertFalse(evidence['completed'] or evidence['full_pass'])
        if not during_restore:
            self.assertEqual(evidence['fields']['shield.amount']['status'], 'ABBRUCH')

    def test_sigterm_in_last_mutated_field_report_cannot_claim_completion(self):
        self.completion_signal(during_restore=False)

    def test_sigterm_during_last_restore_finishes_all_six_files_but_not_the_run(self):
        self.completion_signal(during_restore=True)


if __name__ == '__main__':
    unittest.main()
