"""Run the real mutation/report pipeline with controlled external process exits.

These verify classification and restoration, not a Rust or gameplay measurement.
"""
import contextlib
import importlib.util
import io
import json
import os
import signal
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
RUST_PASS = ('     Running tests/probe.rs (target/debug/deps/probe-1234)\n'
             'running 2 tests\ntest first ... ok\ntest second ... ok\n'
             'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n')
TS_PASS = 'Tests 2 passed (2)\n'
TS_FAIL = ' × value changes the outcome 4ms\nTests 1 failed | 1 passed (2)\n'
# Verbatim cargo remote v2 output, 2026-10-23: a refusal (exit 75, nothing ran)
# and an excerpt of a passing remote test run with its client header/footer.
V2_REFUSAL = ('cargo-remote: This workspace already has an active job\n'
              '{\n  "job": "7f627388cd55c8fd86aebab6"\n}\n')
V2_PASS = ('cargo-remote: job a91655beba11287563f0ab73 | linux-gnu-bookworm | Rust 1.98.1 | normal | '
           'workspace ad6fb34f55843cbc0a7efaef69c89ba0\n'
           '{"event": "wp14_source_enrollment", "project": "8516fde7-4113-4e17-8899-1665bf581a22", '
           '"state": "all"}\n'
           '    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.06s\n'
           '     Running tests/parity_decay_factor.rs '
           '(/target/current/debug/deps/parity_decay_factor-4542d721c90045d8)\n\n'
           'running 4 tests\ntest a ... ok\ntest b ... ok\ntest c ... ok\ntest d ... ok\n\n'
           'test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n\n'
           '     Running tests/rota_sim_selbsttest.rs '
           '(/target/current/debug/deps/rota_sim_selbsttest-6a42df04a0dd09d6)\n\n'
           'running 6 tests\ntest e ... ok\ntest f ... ok\ntest g ... ok\ntest h ... ok\ntest i ... ok\n'
           'test j ... ok\n\n'
           'test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.20s\n\n'
           'cargo-remote: succeeded | exit 0 | 20.16s\n')
# cargo remote v2 uploads with `subprocess.run(rsync, check=True)` and exits
# with rsync's own code (client.py: CalledProcessError -> exc.returncode).
# 12 and 23 are rsync transfer errors, 255 is ssh failing underneath rsync.
# The texts follow rsync 3.x's message format; the tool decides on the exit
# code alone, so the wording is not load-bearing.
V2_HEADER = ('cargo-remote: job a91655beba11287563f0ab73 | linux-gnu-bookworm | Rust 1.98.1 | normal | '
             'workspace ad6fb34f55843cbc0a7efaef69c89ba0\n')
TRANSPORT_EXITS = {
    12: V2_HEADER + 'rsync error: error in rsync protocol data stream (code 12) at io.c(232) [sender=3.4.1]\n',
    23: V2_HEADER + 'rsync error: some files/attrs were not transferred (see previous errors) '
                    '(code 23) at main.c(1338) [sender=3.4.1]\n',
    255: V2_HEADER + 'rsync: connection unexpectedly closed (0 bytes received so far) [sender]\n'
                     'rsync error: unexplained error (code 255) at io.c(232) [sender=3.4.1]\n',
}


class MutationEvidence(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='rota-mutation-evidence-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        spec = importlib.util.spec_from_file_location('streich_evidence', ROOT / 'tools/streich-mutation.py')
        self.tool = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.tool)
        (self.root / 'data/spells').mkdir(parents=True)
        self.originals = {}
        for school in self.tool.SCHULEN:
            text = json.dumps({'spells': [{'id': school, 'heal': {'amount': 40},
                                           'shield': {'amount': 30}}]}) + '\n'
            (self.root / f'data/spells/{school}.json').write_text(text)
            self.originals[school] = text
        (self.root / 'old.md').write_text(
            '| Feld | rote Fälle | seltenster Färber | Urteil | Fall |\n'
            '| `heal.amount` | 3 | 1/2 | gedeckt | healing |\n'
            '| `shield.amount` | 2 | 1/2 | gedeckt | shielding |\n')
        self.addCleanup(signal.signal, signal.SIGTERM, signal.getsignal(signal.SIGTERM))

    def drive(self, process, *extra, wait=0.01):
        # The wait after a service refusal (exit 75) is shortened here; its real
        # value is pinned in test_the_retry_policy_is_120_seconds_and_ten_attempts.
        args = ['streich-mutation.py', '--aus', 'result.md', '--roh', 'names.json',
                '--nachweis', 'evidence.json', '--vergleich', 'old.md', '--bau-log', 'build.log', *extra]
        self.stdout = io.StringIO()
        with patch.object(self.tool, 'WURZEL', self.root), \
             patch.object(self.tool, 'eingecheckter_stand', return_value=self.originals), \
             patch.object(self.tool, 'lauf', side_effect=process), \
             patch.object(self.tool, 'DIENST_WARTEN_S', wait, create=True), \
             patch.object(sys, 'argv', args), contextlib.redirect_stdout(self.stdout):
            return self.tool.main()

    def missing(self):
        spell = json.loads((self.root / 'data/spells/common.json').read_text())['spells'][0]
        return next((f'{key}.amount' for key in ('heal', 'shield') if 'amount' not in spell[key]), None)

    def assert_restored(self):
        for school, text in self.originals.items():
            self.assertEqual((self.root / f'data/spells/{school}.json').read_text(), text)

    def test_build_panic_is_separate_from_real_ts_failures_and_names_every_reclassified_field(self):
        calls = []

        def process(command, _timeout):
            missing = self.missing()
            calls.append((command, missing))
            if '--no-run' in command:
                if missing == 'heal.amount':
                    return 101, 'failed to run custom build command\nthread panicked at sim/build.rs\n'
                return 0, 'Finished test profile\n'
            if command.startswith('cargo'):
                return 0, RUST_PASS
            return (1, TS_FAIL) if missing else (0, TS_PASS)

        self.assertEqual(self.drive(process), 0)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        failed = evidence['fields']['heal.amount']
        self.assertEqual(failed['status'], 'BAU')
        self.assertEqual(failed['build']['exit'], 101)
        self.assertIn('panicked at sim/build.rs', Path(failed['build']['log']).read_text())
        self.assertEqual(failed['rust'], {'status': 'SKIPPED', 'reason': 'build-failure', 'executed': 0})
        self.assertEqual(failed['ts']['status'], 'FAIL')
        self.assertEqual(failed['ts']['executed'], 2)
        self.assertEqual(failed['failure_names'], {'rust': [], 'ts': ['value changes the outcome']})
        self.assertFalse(any('--no-fail-fast' in c and f == 'heal.amount' for c, f in calls))
        rust_commands = [c for c, _ in calls if c.startswith('cargo')]
        self.assertTrue(all(c.startswith('cargo remote --no-copy-lock -- test ') for c in rust_commands))
        self.assertTrue(any('--no-run --all-targets' in c for c in rust_commands))
        self.assertTrue(any(c.endswith('--all-targets --no-fail-fast') for c in rust_commands))
        self.assertFalse(any(c.startswith('bash') for c, _ in calls))
        self.assertEqual(evidence['fields']['shield.amount']['status'], 'ROT')
        self.assertEqual(evidence['historical_comparison']['new_bau'], [
            {'field': 'heal.amount', 'old_verdict': 'gedeckt', 'build_exit': 101}])
        self.assertTrue(evidence['completed'] and evidence['full_pass'])
        self.assertEqual(evidence['historical_comparison']['unmeasured_old_fields'], [])
        self.assert_restored()

    def test_zero_tests_in_baseline_abort_before_any_mutation(self):
        def process(command, _timeout):
            if command.startswith('npx'):
                return 0, 'Tests 0 passed (0)\n'
            return 0, RUST_PASS

        self.assertEqual(self.drive(process), 3)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(evidence['baseline']['ts']['status'], 'ERROR')
        self.assertEqual(evidence['fields'], {})
        self.assertFalse(evidence['full_pass'])
        self.assert_restored()

    def test_incomplete_rust_suite_cannot_be_credited_for_ts_failures(self):
        def process(command, _timeout):
            if self.missing() and '--no-fail-fast' in command:
                return 1, ('running 1 test\ntest actual_failure ... FAILED\n'
                           'test result: FAILED. 0 passed; 1 failed; 0 ignored\n'
                           'running 1 test\nprocess aborted: signal: 11, SIGSEGV\n')
            if command.startswith('npx'):
                return (1, TS_FAIL) if self.missing() else (0, TS_PASS)
            return 0, RUST_PASS

        self.assertEqual(self.drive(process, '--feld', 'heal.amount'), 1)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(evidence['fields']['heal.amount']['status'], 'FEHLER')
        self.assertEqual(evidence['fields']['heal.amount']['rust']['status'], 'ERROR')
        self.assertFalse(evidence['full_pass'])
        self.assertEqual(evidence['historical_comparison']['new_bau'], [])
        self.assert_restored()

    def test_build_only_and_subset_cannot_claim_a_full_reclassification(self):
        calls = []

        def process(command, _timeout):
            calls.append(command)
            return (101, 'custom build failed') if self.missing() else (0, 'Finished')

        self.assertEqual(self.drive(process, '--feld', 'heal.amount', '--nur-bau'), 0)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertTrue(all(c.startswith('cargo remote --no-copy-lock -- test --no-run ')
                            for c in calls))
        self.assertFalse(evidence['full_pass'])
        self.assertEqual(evidence['fields']['heal.amount']['ts']['status'], 'SKIPPED')
        self.assertEqual(evidence['historical_comparison']['unmeasured_old_fields'], ['shield.amount'])
        self.assert_restored()

    def test_restore_preserves_original_line_endings_byte_for_byte(self):
        path = self.root / 'data/spells/common.json'
        original = path.read_bytes().replace(b'\n', b'\r\n')
        path.write_bytes(original)
        self.assertEqual(self.drive(lambda *_: (0, 'Finished'), '--nur-bau'), 0)
        self.assertEqual(path.read_bytes(), original)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertTrue(all(row['restored'] for row in evidence['fields'].values()))

    def test_timeout_records_partial_evidence_and_restores_real_mutation(self):
        def process(command, _timeout):
            if self.missing():
                raise subprocess.TimeoutExpired(command, 1)
            return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)

        with self.assertRaises(subprocess.TimeoutExpired):
            self.drive(process, '--feld', 'heal.amount')
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(evidence['fields']['heal.amount']['status'], 'ABBRUCH')
        self.assertTrue(evidence['fields']['heal.amount']['restored'])
        self.assertFalse(evidence['completed'] or evidence['full_pass'])
        self.assert_restored()

    def test_service_refusal_in_a_field_build_is_not_a_build_verdict(self):
        def process(command, _timeout):
            if '--no-run' in command:
                return (75, V2_REFUSAL) if self.missing() else (0, 'Finished test profile\n')
            return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)

        with self.assertRaises(self.tool.NichtGelaufen) as raised:
            self.drive(process)
        self.assertIn('Exit 75', str(raised.exception))
        evidence = json.loads((self.root / 'evidence.json').read_text())
        statuses = [entry['status'] for entry in evidence['fields'].values()]
        self.assertEqual(statuses, ['ABBRUCH'])
        self.assertIn('NichtGelaufen', next(iter(evidence['fields'].values()))['detail'])
        self.assertFalse(evidence['completed'] or evidence['full_pass'])
        self.assert_restored()

    def test_service_refusal_in_the_baseline_stops_before_any_mutation(self):
        with self.assertRaises(self.tool.NichtGelaufen):
            self.drive(lambda command, _timeout: (75, V2_REFUSAL))
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(evidence['fields'], {})
        self.assert_restored()

    def test_service_refusal_in_the_rust_stage_is_not_a_failure_verdict(self):
        def process(command, _timeout):
            if '--no-fail-fast' in command and self.missing():
                return 75, V2_REFUSAL
            if '--no-run' in command:
                return 0, 'Finished test profile\n'
            return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)

        with self.assertRaises(self.tool.NichtGelaufen):
            self.drive(process, '--feld', 'heal.amount')
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(evidence['fields']['heal.amount']['status'], 'ABBRUCH')
        self.assert_restored()

    def plain_process(self, command, _timeout):
        """Builds pass, Rust passes, TS colours a mutated field: the field reads ROT."""
        if command.startswith('npx'):
            return (1, TS_FAIL) if self.missing() else (0, TS_PASS)
        if '--no-run' in command:
            return 0, 'Finished test profile\n'
        return 0, RUST_PASS

    @staticmethod
    def refused_first(base, hit, refusals):
        """Exit 75 for the first `refusals` calls that `hit` matches, then `base`.

        Every matching call is recorded with its time and the timeout it got.
        """
        calls = []

        def process(command, timeout):
            if hit(command):
                calls.append((time.monotonic(), timeout))
                if len(calls) <= refusals:
                    return 75, V2_REFUSAL
            return base(command, timeout)
        return process, calls

    def test_two_refusals_then_an_accepted_stage_run_and_count_in_every_stage(self):
        # Exit 75 means the service queued nothing (#1055). One refusal ended
        # the whole 3-4 h pass; with six packages building next to it, one in
        # 60 fields is close to certain. The same path serves the baseline, a
        # probe and a field — each is checked here.
        stages = {
            'baseline/build': lambda c: '--no-run' in c and self.missing() is None,
            'baseline/rust': lambda c: '--no-fail-fast' in c and self.missing() is None,
            'field/build': lambda c: '--no-run' in c and self.missing() == 'heal.amount',
            'field/rust': lambda c: '--no-fail-fast' in c and self.missing() == 'heal.amount',
            'vorprobe-bau/build': lambda c: '--no-run' in c and self.probe_state() == 'bau',
        }
        for stage, hit in stages.items():
            with self.subTest(stage=stage):
                self.fresh_tool()
                extra = ['--feld', 'heal.amount', '--timeout', '7']
                if stage.startswith('vorprobe'):
                    self.with_base_damage()
                    base, extra = self.probe_process()[0], ['--vorproben', *extra]
                else:
                    base = self.plain_process
                process, calls = self.refused_first(base, hit, refusals=2)
                try:
                    code = self.drive(process, *extra)
                except self.tool.NichtGelaufen as refused:
                    self.fail(f'NICHT GELAUFEN after two refusals: {refused}')
                self.assertEqual(code, 0)
                evidence = json.loads((self.root / 'evidence.json').read_text())
                self.assertEqual(evidence['fields']['heal.amount']['status'], 'ROT')
                self.assertEqual(len(calls), 3, 'two refused attempts and one accepted')
                # The wait lies between the attempts, and every attempt gets the
                # full stage limit — the wait does not count against it.
                self.assertTrue(all(b[0] - a[0] >= 0.01 for a, b in zip(calls, calls[1:])))
                self.assertEqual([timeout for _, timeout in calls], [7, 7, 7])
                refusals = evidence['service_refusals']
                self.assertEqual([(r['attempt'], r['exit']) for r in refusals], [(1, 75), (2, 75)])
                self.assertTrue(all(r['stage'].endswith(stage.split('/')[1]) for r in refusals))
                self.assertTrue(all(r['message'] == 'cargo-remote: This workspace already has an active job'
                                    for r in refusals))
                self.assertTrue(all(isinstance(r['started_unix'], float) for r in refusals))
                protocol = [line for line in self.stdout.getvalue().splitlines() if 'Exit 75' in line]
                self.assertEqual(len(protocol), 2, self.stdout.getvalue())
                for attempt, line in enumerate(protocol, 1):
                    self.assertIn(f'Versuch {attempt}/10', line)
                    self.assertRegex(line, r'\b\d\d:\d\d:\d\d\b')
                    self.assertIn('This workspace already has an active job', line)
                self.assert_restored()

    def test_other_service_exits_are_not_retried(self):
        # 70 can leave a job running (`--resume`); a second attempt could
        # stand next to it. The rest are faults someone has to look at.
        for code in sorted(set(self.tool.DIENST_AUSGAENGE) - {75}):
            with self.subTest(code=code):
                self.fresh_tool()
                output = TRANSPORT_EXITS.get(code, f'cargo-remote: exit {code}\n')
                hit = lambda c: '--no-run' in c and self.missing() == 'heal.amount'
                process, calls = self.refused_first(
                    lambda c, t, code=code, output=output: (code, output) if hit(c) else self.plain_process(c, t),
                    hit, refusals=0)
                with self.assertRaises(self.tool.NichtGelaufen) as raised:
                    self.drive(process, '--feld', 'heal.amount')
                self.assertIn(f'Exit {code}', str(raised.exception))
                self.assertEqual(len(calls), 1, 'no second attempt')
                evidence = json.loads((self.root / 'evidence.json').read_text())
                self.assertEqual(evidence.get('service_refusals', []), [])
                self.assertEqual(evidence['fields']['heal.amount']['status'], 'ABBRUCH')
                self.assertNotIn('Versuch', self.stdout.getvalue())
                self.assert_restored()

    def test_eleven_refusals_end_as_not_run_after_exactly_ten_attempts(self):
        hit = lambda c: '--no-run' in c and self.missing() == 'heal.amount'
        process, calls = self.refused_first(self.plain_process, hit, refusals=11)
        with self.assertRaises(self.tool.NichtGelaufen) as raised:
            self.drive(process, '--feld', 'heal.amount')
        self.assertEqual(len(calls), 10)
        self.assertIn('Exit 75', str(raised.exception))
        self.assertIn('10 Versuchen', str(raised.exception))
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual([r['attempt'] for r in evidence['service_refusals']], list(range(1, 11)))
        self.assertEqual(evidence['fields']['heal.amount']['status'], 'ABBRUCH')
        self.assertFalse(evidence['completed'] or evidence['full_pass'])
        self.assertIn('Versuch 10/10', self.stdout.getvalue())
        self.assertIn('Kein weiterer Versuch', self.stdout.getvalue())
        self.assert_restored()

    def test_the_retry_policy_is_120_seconds_and_ten_attempts(self):
        self.assertEqual((self.tool.DIENST_WARTEN_S, self.tool.DIENST_VERSUCHE), (120, 10))

    def test_sigterm_during_the_wait_stops_at_once_and_restores(self):
        # A wait of 120 s in one `time.sleep` would outlast SIGTERM: Python
        # resumes the sleep after a handler that does not raise, and the
        # tool's handler deliberately does not. Whoever sends SIGTERM
        # escalates to SIGKILL — and then nothing is restored.
        done = threading.Event()

        def stop():
            if not done.is_set():
                os.kill(os.getpid(), signal.SIGTERM)

        timer = threading.Timer(0.3, stop)
        self.addCleanup(timer.cancel)
        hit = lambda c: '--no-run' in c and self.missing() == 'heal.amount'
        process, calls = self.refused_first(self.plain_process, hit, refusals=10)

        def refusing(command, timeout):
            self.tool._unterbrechung_pruefen()     # as the real `lauf` does before each start
            if hit(command) and not calls:
                timer.start()
            return process(command, timeout)

        started = time.monotonic()
        try:
            with self.assertRaises(SystemExit) as stopped:
                self.drive(refusing, '--feld', 'heal.amount', wait=30)
        finally:
            done.set()
        self.assertEqual(stopped.exception.code, 143)
        self.assertLess(time.monotonic() - started, 10, 'SIGTERM waited for the end of the wait')
        self.assertEqual(len(calls), 1)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertEqual(evidence['fields']['heal.amount']['status'], 'ABBRUCH')
        self.assertTrue(evidence['fields']['heal.amount']['restored'])
        self.assert_restored()

    def fresh_tool(self):
        """A new module instance: an aborted run leaves the module's abort flag set."""
        spec = importlib.util.spec_from_file_location('streich_evidence', ROOT / 'tools/streich-mutation.py')
        self.tool = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.tool)

    def test_transport_exits_of_the_v2_client_are_not_verdicts(self):
        # Without this, exit 12 in a field build counts as a broken build (BAU)
        # and exit 255 in the Rust stage as a red suite — while nothing was
        # uploaded, let alone compiled.
        for code, output in TRANSPORT_EXITS.items():
            for stage in ('baseline-build', 'field-build', 'field-rust'):
                with self.subTest(code=code, stage=stage):
                    self.fresh_tool()

                    def process(command, _timeout, code=code, output=output, stage=stage):
                        mutated = self.missing() is not None
                        if command.startswith('npx'):
                            return (1, TS_FAIL) if mutated else (0, TS_PASS)
                        if '--no-run' in command:
                            if stage == 'baseline-build' or (stage == 'field-build' and mutated):
                                return code, output
                            return 0, 'Finished test profile\n'
                        if stage == 'field-rust' and mutated:
                            return code, output
                        return 0, RUST_PASS

                    with self.assertRaises(self.tool.NichtGelaufen) as raised:
                        self.drive(process, '--feld', 'heal.amount')
                    self.assertIn(f'Exit {code}', str(raised.exception))
                    evidence = json.loads((self.root / 'evidence.json').read_text())
                    if stage == 'baseline-build':
                        self.assertEqual(evidence['fields'], {})
                    else:
                        entry = evidence['fields']['heal.amount']
                        self.assertEqual(entry['status'], 'ABBRUCH')
                        self.assertIn('NichtGelaufen', entry['detail'])
                        self.assertTrue(entry['restored'])
                    self.assertFalse(evidence['completed'] or evidence['full_pass'])
                    self.assert_restored()

    BUILD_PANIC = ("error: failed to run custom build command for `rota-sim v0.1.0 (/src/sim)`\n"
                   "thread 'main' panicked at sim/build.rs:556:20:\n"
                   "common.json/spells[?]: Feld \"id\" fehlt\n")
    RUST_FAIL = ('     Running tests/probe.rs (target/debug/deps/probe-1234)\n'
                 'running 2 tests\ntest first ... ok\ntest parity_damage_matches ... FAILED\n'
                 'test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n')

    def with_base_damage(self):
        """The probes need a base value that every damage computation reads."""
        for school in self.tool.SCHULEN:
            text = json.dumps({'spells': [{'id': school, 'baseDamage': 100, 'heal': {'amount': 40},
                                           'shield': {'amount': 30}}]}) + '\n'
            (self.root / f'data/spells/{school}.json').write_text(text)
            self.originals[school] = text

    def probe_state(self):
        first = json.loads((self.root / 'data/spells/calcination.json').read_text())['spells'][0]
        common = json.loads((self.root / 'data/spells/common.json').read_text())['spells'][0]
        return 'bau' if 'id' not in common else 'rot' if first['baseDamage'] != 100 else None

    def probe_process(self, bau_build=None, rust_names=True):
        calls = []
        bau_build = bau_build or (101, self.BUILD_PANIC)

        def process(command, _timeout):
            state, field = self.probe_state(), self.missing()
            calls.append((command.split()[0], state, field))
            if '--no-run' in command:
                return bau_build if state == 'bau' else (0, 'Finished\n')
            if command.startswith('cargo'):
                return (101, self.RUST_FAIL) if state == 'rot' and rust_names else (0, RUST_PASS)
            return (1, TS_FAIL) if state or field else (0, TS_PASS)
        return process, calls

    def test_probes_run_after_the_baseline_and_before_any_field(self):
        self.with_base_damage()
        process, calls = self.probe_process()
        self.assertEqual(self.drive(process, '--vorproben'), 0)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        bau, rot = evidence['probes']['bau'], evidence['probes']['rot']
        self.assertEqual(bau['status'], 'BAU')
        self.assertEqual(bau['build']['exit'], 101)
        self.assertFalse(bau['old_rule_detects'], 'the old text rule must miss exactly this case')
        self.assertEqual(bau['rust']['status'], 'SKIPPED')
        self.assertEqual(rot['status'], 'ROT')
        self.assertEqual(rot['failure_names'], {'rust': ['parity_damage_matches'],
                                                'ts': ['value changes the outcome']})
        self.assertTrue(evidence['probes_passed'])
        self.assertTrue(all(p['restored'] and p['sha256']['after_restore'] == p['sha256']['before']
                            for p in (bau, rot)))
        first_field = next(i for i, (_, _, field) in enumerate(calls) if field)
        probe_calls = [i for i, (_, state, _) in enumerate(calls) if state]
        self.assertTrue(probe_calls and max(probe_calls) < first_field)
        self.assertTrue(evidence['historical_comparison']['full_pass'])
        self.assert_restored()

    def test_a_probe_with_the_wrong_outcome_stops_before_any_field(self):
        # Each condition of the BAU probe on its own: a build that passes, one
        # that passes with the panic text in its log, one that breaks elsewhere.
        cases = (((0, 'Finished\n'), True, 'bau'),
                 ((0, "Finished\nwarning: a test named 'panicked at sim/build.rs'\n"), True, 'bau'),
                 ((101, 'error: linking with `cc` failed: exit status: 1\n'), True, 'bau'),
                 (None, False, 'rot'))
        for bau_build, rust_names, failing in cases:
            with self.subTest(failing=failing, build=bau_build):
                self.fresh_tool()
                self.with_base_damage()
                process, calls = self.probe_process(bau_build, rust_names)
                self.assertEqual(self.drive(process, '--vorproben'), 3)
                evidence = json.loads((self.root / 'evidence.json').read_text())
                self.assertEqual(evidence['fields'], {})
                self.assertFalse(evidence['probes_passed'])
                self.assertNotEqual(evidence['probes'][failing]['status'],
                                    evidence['probes'][failing]['expected'])
                self.assertFalse(any(field for _, _, field in calls))
                self.assert_restored()

    def test_without_probes_a_complete_pass_is_no_acceptance(self):
        def process(command, _timeout):
            if command.startswith('npx'):
                return (1, TS_FAIL) if self.missing() else (0, TS_PASS)
            return 0, RUST_PASS

        self.assertEqual(self.drive(process), 0)
        evidence = json.loads((self.root / 'evidence.json').read_text())
        self.assertTrue(evidence['completed'] and evidence['full_pass'])
        self.assertFalse(evidence['historical_comparison']['full_pass'])

    def test_real_cargo_remote_v2_output_is_a_complete_rust_summary(self):
        proof = self.tool.suite_nachweis(0, V2_PASS, 'rust')
        self.assertEqual(proof['status'], 'PASS')
        self.assertTrue(proof['complete_summary'])
        self.assertEqual(proof['executed'], 10)
        self.assertFalse(self.tool.bau_gebrochen(0, V2_PASS))

    def test_build_text_controls_and_suite_counts_are_not_inferred_from_return_code_alone(self):
        for text in ('error[E0308]: mismatched types', 'error: could not compile rota-sim'):
            self.assertTrue(self.tool.bau_gebrochen(0, text))
        self.assertTrue(self.tool.bau_gebrochen(101, 'custom build command failed'))
        self.assertFalse(self.tool.bau_gebrochen(0, 'Finished'))
        self.assertEqual(self.tool.suite_nachweis(0, '', 'rust')['status'], 'ERROR')
        self.assertEqual(self.tool.suite_nachweis(0, 'Tests 0 passed (0)', 'ts')['status'], 'ERROR')
        output = ('running 2 tests\ntest same_name ... FAILED\n'
                  'test result: FAILED. 1 passed; 1 failed; 0 ignored\n'
                  'running 3 tests\ntest same_name ... FAILED\n'
                  'test result: FAILED. 2 passed; 1 failed; 0 ignored\n')
        self.assertEqual(self.tool.suite_nachweis(1, output, 'rust')['executed'], 5)

    def test_a_later_aborted_target_invalidates_preceding_real_failure_summaries(self):
        failed = ('     Running tests/first.rs (target/debug/deps/first-1234)\n'
                  'running 1 test\ntest actual_failure ... FAILED\n'
                  'test result: FAILED. 0 passed; 1 failed; 0 ignored\n')
        for suffix in (
            '     Running tests/second.rs (target/debug/deps/second-1234)\n',
            'running 1 test\nprocess didn\'t exit successfully (signal: 11, SIGSEGV)\n',
            'running 0 tests\n',
            'error: could not execute process: No such file or directory\n',
        ):
            for code in (0, 1):
                with self.subTest(suffix=suffix, code=code):
                    proof = self.tool.suite_nachweis(code, failed + suffix, 'rust')
                    self.assertEqual(proof['status'], 'ERROR')
                    self.assertFalse(proof['complete_summary'])
        # Empty binary harnesses are normal with --all-targets, but do not
        # contribute executed cases and cannot establish a passing run alone.
        empty = ('     Running unittests src/bin/main.rs (target/debug/deps/main-1234)\n'
                 'running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored\n')
        self.assertEqual(self.tool.suite_nachweis(0, empty, 'rust')['status'], 'ERROR')
        self.assertEqual(self.tool.suite_nachweis(1, failed + empty, 'rust')['status'], 'FAIL')

    def test_remote_stdout_stderr_can_append_the_next_target_to_a_summary(self):
        # Real zero-test example line from the first full-run baseline.
        output = (RUST_PASS.rstrip('\n')
                  + '     Running unittests examples/suchguete.rs '
                    '(target/debug/examples/suchguete-59b410dc0c687d9e)\n'
                    'running 0 tests\n'
                    'test result: ok. 0 passed; 0 failed; 0 ignored\n')
        proof = self.tool.suite_nachweis(0, output, 'rust')
        self.assertEqual(proof['status'], 'PASS')
        self.assertEqual(proof['executed'], 2)
        self.assertTrue(proof['complete_summary'])
        aborted = output + '     Running tests/later.rs (target/debug/deps/later-1234)\n'
        self.assertEqual(self.tool.suite_nachweis(1, aborted, 'rust')['status'], 'ERROR')

    def test_ts_worker_errors_or_process_signals_invalidate_preceding_failed_cases(self):
        for suffix in (
            '\nErrors 1 error\n',
            '\nVitest caught 1 unhandled error during the test run.\n',
            '\nVitest found 2 errors not related to your test files.\n',
        ):
            with self.subTest(suffix=suffix):
                proof = self.tool.suite_nachweis(1, TS_FAIL + suffix, 'ts')
                self.assertEqual(proof['status'], 'ERROR')
                self.assertFalse(proof['complete_summary'])
                self.assertEqual(proof['executed'], 2)
        for code in (-9, 137, 143):
            self.assertEqual(self.tool.suite_nachweis(code, TS_FAIL, 'ts')['status'], 'ERROR')
        self.assertEqual(self.tool.suite_nachweis(1, TS_FAIL, 'ts')['status'], 'FAIL')


if __name__ == '__main__':
    unittest.main()
