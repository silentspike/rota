"""Exercise actual measurement-tool readers and control flow, without game runs."""

import contextlib
import importlib.util
import io
import json
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def module(name):
    spec = importlib.util.spec_from_file_location(name.replace('-', '_'), ROOT / 'tools' / f'{name}.py')
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


class MeasurementTools(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='rota-tools-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.hebel = module('groessen-hebel')
        self.streich = module('streich-mutation')

    def test_hebel_real_cli_reads_headers_without_exporting_or_backups(self):
        (self.root / 'tools').mkdir()
        (self.root / 'docs').mkdir()
        shutil.copy2(ROOT / 'tools/groessen-hebel.py', self.root / 'tools/groessen-hebel.py')
        report = self.root / 'docs/groessen-abdeckung-messung.txt'
        report.write_text('# SCHWEIGT FRISCHE are header words\n\n'
                          'dot.tickDamage rot SCHWEIGT\n'
                          'heal.amount salve GESCHUETZT\n'
                          'control.cost pin nur FRISCHE\n')
        before = {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob('*') if p.is_file()}
        run = subprocess.run([sys.executable, str(self.root / 'tools/groessen-hebel.py'), '--list'],
                             text=True, capture_output=True, check=True)
        self.assertEqual(json.loads(run.stdout), [['dot.tickDamage', 'rot'], ['control.cost', 'pin']])
        after = {str(p.relative_to(self.root)): p.read_bytes() for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(before, after)

    def test_hebel_recognizes_all_recording_shapes_and_damage_alias(self):
        fixtures = {
            'flat': {'presses': [{'atMs': 0}], 'expected': {'damage': 100, 'healed': 20}},
            'runs': {'runs': [{'expected': {'absorbed': 12}}, {'expected': {'controlSpent': 3}}]},
            'cases': {'cases': [{'runs': [{'expected': {'damageDealt': 55, 'converted': 9}}]}]},
            'metadata': {'logKinds': ['controlSpent']},
        }
        for name, data in fixtures.items():
            (self.root / f'{name}.json').write_text(json.dumps(data))
        picture = self.hebel.wirkungsbild(self.root)
        self.assertEqual(set(picture), {'flat#0', 'runs#0', 'runs#1', 'cases#0'})
        self.assertEqual(picture['cases#0']['damage'], 55)
        self.assertEqual(picture['runs#1']['controlSpent'], 3)
        self.assertEqual(json.loads(picture['flat#0']['__druecke']), [{'atMs': 0}])

    def test_hebel_empty_recording_set_is_not_a_zero_effect(self):
        with self.assertRaisesRegex(RuntimeError, 'keine einzige'):
            self.hebel.wirkungsbild(self.root)

    def test_hebel_rejects_malformed_measurement_rows(self):
        report = self.root / 'broken.txt'
        report.write_text('SCHWEIGT\n')
        with self.assertRaisesRegex(ValueError, 'Unvollständige'):
            self.hebel.offene_paare(report)

    def test_mutation_preserves_integer_values_and_leaves_non_numbers_alone(self):
        data = {'heal': {'amount': 40}, 'switch': True, 'name': 'x'}
        self.assertEqual(self.streich.wert_aendern(data, ['heal', 'amount'], 1.0), (True, 40, 41))
        self.assertIs(type(data['heal']['amount']), int)
        self.assertEqual(self.streich.wert_aendern(data, ['heal', 'amount'], -2), (True, 41, 39))
        for path in (['switch'], ['name'], ['missing', 'amount']):
            self.assertEqual(self.streich.wert_aendern(data, path, 1), (False, None, None))
        self.assertIs(data['switch'], True)

    def test_failure_names_keep_prefixes_and_separate_files_from_cases(self):
        output = ('test apl_case ... FAILED\n'
                  'test result: FAILED. 1 passed; 1 failed; 0 ignored\n'
                  ' FAIL tests/unit/x.test.ts > suite > case\n'
                  ' × AC2 catches the boundary 12ms\n'
                  ' × AC2 catches the boundary 51ms\n'
                  ' × FAIL is part of this name 1ms\n')
        names, files = self.streich.rote_namen(output)
        self.assertEqual(names, ['apl_case', 'AC2 catches the boundary', 'FAIL is part of this name'])
        self.assertEqual(files, ['tests/unit/x.test.ts'])

    def test_raster_reads_both_real_sources_and_rejects_disagreement(self):
        (self.root / 'src/core').mkdir(parents=True)
        (self.root / 'sim').mkdir()
        ts = self.root / 'src/core/loop.ts'
        rs = self.root / 'sim/build.rs'
        with patch.object(self.streich, 'WURZEL', self.root):
            for raster in (10, 20):
                ts.write_text(f'export const TICK_MS: number = {raster};')
                rs.write_text(f'const RASTER_MS: u64 = {raster};')
                self.assertEqual(self.streich.raster_ms(), raster)
            ts.write_text('export const TICK_MS = 10;')
            with self.assertRaisesRegex(SystemExit, 'auseinander'):
                self.streich.raster_ms()
            ts.write_text('export const RENAMED = 20;')
            with self.assertRaisesRegex(SystemExit, 'nicht lesbar'):
                self.streich.raster_ms()

    def test_failed_build_never_runs_stale_rust_binaries_and_restores_data(self):
        (self.root / 'data/spells').mkdir(parents=True)
        originals = {}
        for school in self.streich.SCHULEN:
            content = json.dumps({'spells': [{'id': school, 'heal': {'amount': 40}}]}) + '\n'
            (self.root / f'data/spells/{school}.json').write_text(content)
            originals[school] = content
        calls = []

        def fake_process(command, _timeout):
            # Only the external build/test boundary is replaced. The production
            # tool selects fields, mutates files, classifies, writes and restores.
            changed = 'amount' not in json.loads((self.root / 'data/spells/common.json').read_text())['spells'][0]['heal']
            calls.append((command, changed))
            if command.startswith('cargo') and changed:
                return 101, 'failed to run custom build command: panicked at sim/build.rs'
            return 0, 'test result: ok. 1 passed; 0 failed'

        previous_handler = signal.getsignal(signal.SIGTERM)
        self.addCleanup(signal.signal, signal.SIGTERM, previous_handler)
        with patch.object(self.streich, 'WURZEL', self.root), \
             patch.object(self.streich, 'BAU_LOG', str(self.root / 'build.log')), \
             patch.object(self.streich, 'eingecheckter_stand', return_value=originals), \
             patch.object(self.streich, 'lauf', side_effect=fake_process), \
             patch.object(sys, 'argv', ['streich-mutation.py', '--feld', 'heal.amount', '--aus', 'result.md']), \
             contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(self.streich.main(), 0)
        self.assertIn('| `heal.amount` | 6 | BAU |', (self.root / 'result.md').read_text())
        self.assertFalse(any('tests-lokal.sh' in command and changed for command, changed in calls))
        self.assertTrue(any('tests-lokal.sh' in command and not changed for command, changed in calls))
        for school, content in originals.items():
            self.assertEqual((self.root / f'data/spells/{school}.json').read_text(), content)

    def test_syntax_check_rejects_broken_tool_without_running_it(self):
        (self.root / 'tools').mkdir()
        file = self.root / 'tools/check.py'
        file.write_text('raise RuntimeError("must not execute")\n')
        command = [sys.executable, str(ROOT / 'tools/python-check.py'), '--root', str(self.root)]
        run = subprocess.run(command, capture_output=True, text=True)
        self.assertEqual(run.returncode, 0, run.stderr)
        self.assertFalse((self.root / 'tools/__pycache__').exists())
        file.write_text('def broken(:\n')
        run = subprocess.run(command, capture_output=True, text=True)
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('SyntaxError', run.stderr)

    def test_every_python_tool_has_an_explicit_behavior_policy(self):
        actual = {str(p.relative_to(ROOT)) for p in (ROOT / 'tools').rglob('*.py')}
        documented = re.findall(r'\| `(tools/[^`]+\.py)`\s+\|',
                                (ROOT / 'docs/PYTHON-WERKZEUGE.md').read_text())
        self.assertEqual(actual, set(documented))
        self.assertEqual(len(documented), len(set(documented)), 'Duplicate tool policy')

    def test_empty_tool_suite_is_rejected(self):
        (self.root / 'tools').mkdir()
        (self.root / 'tools/syntax.py').write_text('value = 1\n')
        (self.root / 'tests/tools').mkdir(parents=True)
        run = subprocess.run([sys.executable, '-B', str(ROOT / 'tools/python-check.py'),
                              '--root', str(self.root), '--tests'], capture_output=True, text=True)
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('No tool tests discovered', run.stderr)


if __name__ == '__main__':
    unittest.main()
