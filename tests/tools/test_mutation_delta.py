"""Value mutation (--delta) against the production time contract (#1087).

The real mutation, classification, restoration and `policy()` from
tools/spell-time-audit.py run on a temporary copy; the four files that
`policy()` reads are copied from the repository, not imitated. Only the
external build and test processes are controlled.
"""
import contextlib
import hashlib
import io
import json
import shutil
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

import test_mutation_evidence as evidence_tests

ROOT, RUST_PASS, TS_FAIL, TS_PASS = (evidence_tests.ROOT, evidence_tests.RUST_PASS,
                                     evidence_tests.TS_FAIL, evidence_tests.TS_PASS)
POLICY_SOURCES = ('src/core/loop.ts', 'src/core/data.ts', 'sim/build.rs', 'sim/time_fields.rs')


def spell(school):
    return {'id': school, 'heal': {'amount': 40},
            'shield': {'amount': 30, 'durationMs': 6000},
            'cooldownReductionBelowMs': 10000,
            'buff': {'factor': 1.5, 'target': 'self'}}


class MutationDelta(unittest.TestCase):
    assert_restored = evidence_tests.MutationEvidence.assert_restored

    def setUp(self):
        evidence_tests.MutationEvidence.setUp(self)
        for school in self.tool.SCHULEN:
            text = json.dumps({'spells': [spell(school)]}) + '\n'
            (self.root / f'data/spells/{school}.json').write_text(text)
            self.originals[school] = text
        for source in POLICY_SOURCES:
            (self.root / source).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / source, self.root / source)
        self.calls = []

    def drive(self, process, *extra):
        """Like MutationEvidence.drive, but keeps stdout for the protocol checks."""
        args = ['streich-mutation.py', '--aus', 'result.md', '--roh', 'names.json',
                '--nachweis', 'evidence.json', '--bau-log', 'build.log', *extra]
        self.stdout = io.StringIO()

        def recorded(command, timeout):
            self.calls.append(command)
            return process(command, timeout)

        with patch.object(self.tool, 'WURZEL', self.root), \
             patch.object(self.tool, 'eingecheckter_stand', return_value=self.originals), \
             patch.object(self.tool, 'lauf', side_effect=recorded), \
             patch.object(sys, 'argv', args), contextlib.redirect_stdout(self.stdout):
            return self.tool.main()

    def changed(self):
        """Fields whose value differs from the original in common.json."""
        now = json.loads((self.root / 'data/spells/common.json').read_text())['spells'][0]
        before = spell('common')
        return [f'{group}.{key}' for group in ('heal', 'shield', 'buff')
                for key in before[group] if now[group].get(key) != before[group][key]] + \
               (['cooldownReductionBelowMs'] if now.get('cooldownReductionBelowMs') != 10000 else [])

    def green(self, command, _timeout):
        return (0, TS_PASS) if command.startswith('npx') else (0, RUST_PASS)

    def red_when_changed(self, command, _timeout):
        if command.startswith('npx'):
            return (1, TS_FAIL) if self.changed() else (0, TS_PASS)
        return 0, RUST_PASS

    def evidence(self):
        return json.loads((self.root / 'evidence.json').read_text())

    def file_hashes(self):
        return {f'data/spells/{school}.json': hashlib.sha256(text.encode()).hexdigest()
                for school, text in self.originals.items()}

    # ── Exit 2: a step that cannot measure coverage never starts ──────────

    def test_a_fractional_step_on_a_whole_number_field_is_refused_before_anything_runs(self):
        for field in ('heal.amount', 'shield.durationMs', 'cooldownReductionBelowMs'):
            with self.subTest(field=field):
                self.calls.clear()
                self.assertEqual(self.drive(self.green, '--feld', field, '--delta', '0.5'), 2)
                self.assertEqual(self.calls, [], 'a refused step must not start the baseline')
                self.assertIn('nicht ganzzahlig', self.stdout.getvalue())
                self.assert_restored()

    def test_an_off_grid_step_on_a_grid_field_is_refused_with_the_grid_named(self):
        self.assertEqual(self.drive(self.green, '--feld', 'shield.durationMs', '--delta', '1'), 2)
        self.assertEqual(self.calls, [])
        protocol = self.stdout.getvalue()
        self.assertIn('10-ms-Takt', protocol)
        self.assertIn('#1104', protocol)
        self.assert_restored()

    def test_a_field_without_a_number_is_refused(self):
        self.assertEqual(self.drive(self.green, '--feld', 'buff.target', '--delta', '1'), 2)
        self.assertEqual(self.calls, [])

    # ── Categories from the production rule ───────────────────────────────

    def test_on_grid_and_threshold_steps_run_and_carry_their_kind(self):
        cases = (('shield.durationMs', '10', 'raster'), ('cooldownReductionBelowMs', '1', 'schwelle'),
                 ('heal.amount', '1', 'ganzzahl'), ('buff.factor', '0.5', 'bruchzahl'))
        for field, step, kind in cases:
            with self.subTest(field=field):
                self.fresh()
                self.assertEqual(self.drive(self.red_when_changed, '--feld', field, '--delta', step), 0)
                entry = self.evidence()['fields'][field]
                self.assertEqual(entry['kind'], kind)
                self.assertEqual(entry['status'], 'ROT')
                self.assertIn(f'[{kind}, Schritt {step}', self.stdout.getvalue())

    def test_the_kinds_follow_policy_and_reject_a_disagreeing_rule(self):
        data_ts = self.root / 'src/core/data.ts'
        data_ts.write_text(data_ts.read_text().replace("key !== 'cooldownReductionBelowMs'",
                                                       "key !== 'cooldownReductionMs'"))
        with self.assertRaisesRegex(self.tool.NichtGestartet, 'Zeitregel'):
            self.drive(self.green, '--feld', 'heal.amount', '--delta', '1')
        self.assertEqual(self.calls, [])

    def test_an_unusable_time_rule_ends_the_script_with_exit_2(self):
        # As a script, not through main(): the exit code is the claim here.
        # `raise SystemExit(text)` ended with 1, the code for a finding.
        (self.root / 'tools').mkdir()
        for name in ('streich-mutation.py', 'spell-time-audit.py'):
            shutil.copyfile(ROOT / 'tools' / name, self.root / 'tools' / name)
        loop = self.root / 'src/core/loop.ts'
        loop.write_text(loop.read_text().replace('TICK_MS', 'RENAMED_MS'))
        result = subprocess.run([sys.executable, '-B', str(self.root / 'tools/streich-mutation.py'),
                                 '--feld', 'heal.amount', '--delta', '1'],
                                cwd=self.root, capture_output=True, text=True, timeout=60, check=False)
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('NICHT GESTARTET: Zeitregel nicht verwendbar', result.stderr)
        self.assert_restored()

    def test_a_kind_selects_exactly_its_fields(self):
        self.assertEqual(self.drive(self.red_when_changed, '--art', 'ganzzahl', '--delta', '1'), 0)
        self.assertEqual(sorted(self.evidence()['fields']), ['heal.amount', 'shield.amount'])

    def test_several_fields_can_be_named(self):
        self.assertEqual(self.drive(self.red_when_changed, '--feld', 'heal.amount',
                                    '--feld', 'shield.durationMs', '--delta', '1'), 2)
        self.assertEqual(self.calls, [], 'one refused field refuses the whole pass')
        self.assertEqual(self.drive(self.red_when_changed, '--feld', 'heal.amount',
                                    '--feld', 'shield.amount', '--delta', '1'), 0)
        self.assertEqual(sorted(self.evidence()['fields']), ['heal.amount', 'shield.amount'])

    # ── AC4: fingerprints before, during and after the restore ────────────

    def test_every_field_records_sha256_before_mutated_before_restore_and_after_restore(self):
        for extra in (('--delta', '1'), ()):
            with self.subTest(mode=extra or 'delete'):
                self.fresh()
                self.assertEqual(self.drive(self.red_when_changed, '--feld', 'heal.amount', *extra), 0)
                prints = self.evidence()['fields']['heal.amount']['sha256']
                self.assertEqual(set(prints), {'before', 'mutated', 'before_restore', 'after_restore'})
                self.assertEqual(prints['before'], self.file_hashes())
                self.assertEqual(prints['after_restore'], prints['before'])
                self.assertEqual(prints['before_restore'], prints['mutated'])
                self.assertTrue(all(prints['mutated'][f] != prints['before'][f] for f in prints['before']))
                self.assertIn('Rückbau byteweise identisch', self.stdout.getvalue())
                self.assert_restored()

    def test_data_changed_during_the_run_is_a_failure_and_not_a_verdict(self):
        intruder = self.root / 'data/spells/fixation.json'

        def process(command, timeout):
            if command.startswith('npx') and self.changed():
                intruder.write_text(intruder.read_text().replace('"self"', '"other"'))
            return self.red_when_changed(command, timeout)

        self.assertEqual(self.drive(process, '--feld', 'heal.amount', '--delta', '1'), 1)
        entry = self.evidence()['fields']['heal.amount']
        self.assertEqual(entry['status'], 'FEHLER')
        self.assertIn('während des Laufs verändert', entry['detail'])
        self.assertNotEqual(entry['sha256']['before_restore'], entry['sha256']['mutated'])
        self.assertTrue(entry['restored'])
        self.assert_restored()

    # ── AC7: the question before the run decides how GREEN reads ──────────

    def test_the_decision_on_the_value_stands_in_the_protocol_before_the_run(self):
        site = 'tests/unit/putrefaction-fenster.test.ts:"der Schlag bei 12000 ms"'
        for answer, status in (('ja', 'GRUEN'), ('nein', 'GRUEN?'), (None, 'GRUEN?')):
            with self.subTest(answer=answer):
                self.fresh()
                extra = ('--entscheidung', f'shield.durationMs={answer}:{site}') if answer else ()
                self.assertEqual(self.drive(self.green, '--feld', 'shield.durationMs',
                                            '--delta', '10', *extra), 0)
                entry = self.evidence()['fields']['shield.durationMs']
                self.assertEqual(entry['status'], status)
                self.assertEqual(entry['decision_on_value']['answer'], answer)
                self.assertEqual(entry['decision_on_value']['evidence'], site if answer else None)
                self.assertIn('Entscheidung genau auf dem Wert', entry['decision_on_value']['question'])
                protocol = self.stdout.getvalue()
                question = protocol.index('Entscheidung genau auf dem Wert?')
                self.assertLess(question, protocol.index('shield.durationMs', question),
                                'the question must precede the result line')
                self.assertIn(f'UNGEDECKT: {1 if answer == "ja" else 0} von 1', protocol)

    def test_a_malformed_or_foreign_decision_is_refused(self):
        for decision in ('shield.durationMs', 'shield.durationMs=vielleicht:x',
                         'shield.durationMs=ja:', 'heal.amount=ja:x'):
            with self.subTest(decision=decision):
                self.calls.clear()
                self.assertEqual(self.drive(self.green, '--feld', 'shield.durationMs', '--delta', '10',
                                            '--entscheidung', decision), 2)
                self.assertEqual(self.calls, [])

    def fresh(self):
        """A new module instance and an untouched data set for the next subtest."""
        evidence_tests.MutationEvidence.fresh_tool(self)
        self.calls.clear()
        for school, text in self.originals.items():
            (self.root / f'data/spells/{school}.json').write_text(text)


if __name__ == '__main__':
    unittest.main()
