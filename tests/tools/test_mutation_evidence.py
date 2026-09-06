"""Run the real mutation/report pipeline with controlled external process exits.

These verify classification and restoration, not a Rust or gameplay measurement.
"""
import contextlib
import importlib.util
import io
import json
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
RUST_PASS = ('     Running tests/probe.rs (target/debug/deps/probe-1234)\n'
             'running 2 tests\ntest first ... ok\ntest second ... ok\n'
             'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n')
TS_PASS = 'Tests 2 passed (2)\n'
TS_FAIL = ' × value changes the outcome 4ms\nTests 1 failed | 1 passed (2)\n'


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

    def drive(self, process, *extra):
        args = ['streich-mutation.py', '--aus', 'result.md', '--roh', 'names.json',
                '--nachweis', 'evidence.json', '--vergleich', 'old.md', '--bau-log', 'build.log', *extra]
        with patch.object(self.tool, 'WURZEL', self.root), \
             patch.object(self.tool, 'eingecheckter_stand', return_value=self.originals), \
             patch.object(self.tool, 'lauf', side_effect=process), \
             patch.object(sys, 'argv', args), contextlib.redirect_stdout(io.StringIO()):
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
