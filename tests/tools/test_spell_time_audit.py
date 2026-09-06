"""Real CLI checks for the #1104 inventory, including a new invalid field."""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class SpellTimeAudit(unittest.TestCase):
    def setUp(self):
        temporary_root = Path(os.environ.get('RUNNER_TEMP', '/work/tmp'))
        temporary_root.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(prefix='rota-time-audit-', dir=temporary_root)
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name in ('src/core/data.ts', 'src/core/loop.ts', 'sim/build.rs', 'sim/time_fields.rs'):
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, destination)
        (self.root / 'data/spells').mkdir(parents=True)

    def run_cli(self, root=None):
        return subprocess.run(
            [sys.executable, '-B', str(ROOT / 'tools/spell-time-audit.py'), '--json',
             '--root', str(root or self.root)],
            capture_output=True, text=True, check=False,
        )

    def data(self, value):
        (self.root / 'data/spells/probe.json').write_text(json.dumps(value))

    def test_real_tree_is_counted_and_the_threshold_is_named(self):
        result = self.run_cli(ROOT)
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report['status'], 'PASS')
        self.assertEqual(report['files'], len(list((ROOT / 'data/spells').glob('*.json'))))
        self.assertGreater(report['grid_occurrences'], 100)
        self.assertEqual(report['threshold_fields'], ['cooldownReductionBelowMs'])
        self.assertGreater(report['threshold_occurrences'], 0)
        self.assertEqual(report['occurrences'], report['grid_occurrences'] + report['threshold_occurrences'])
        self.assertIn('extendShieldMs', report['grid_fields'])
        self.assertIn('extendDecayMs', report['grid_fields'])

    def test_unknown_nested_duration_turns_the_cli_red_without_a_field_list_edit(self):
        self.data({'spells': [{'futureEffect': {'settlingMs': 12000}, 'channelMs': 3000}]})
        before = self.run_cli()
        self.assertEqual(before.returncode, 0, before.stderr)
        self.assertEqual(json.loads(before.stdout)['grid_occurrences'], 2)
        self.data({'spells': [{'futureEffect': {'settlingMs': 12001}, 'channelMs': 3000}]})
        after = self.run_cli()
        self.assertEqual(after.returncode, 1)
        report = json.loads(after.stdout)
        self.assertEqual(report['status'], 'FAIL')
        self.assertEqual(report['errors'], [
            'data/spells/probe.json:spells[0].futureEffect.settlingMs: 12001 is not on the 10-ms grid',
        ])

    def test_threshold_may_be_off_grid_but_not_invalid(self):
        self.data({'cooldownReductionBelowMs': 10005})
        self.assertEqual(self.run_cli().returncode, 0)
        for value in (True, -1, 1.5, '10005', None, 9_007_199_254_740_992):
            with self.subTest(value=value):
                self.data({'cooldownReductionBelowMs': value})
                self.assertEqual(self.run_cli().returncode, 1)

    def test_empty_and_unreadable_sources_never_pass(self):
        self.assertEqual(self.run_cli().returncode, 1)
        self.data({'nothing': 1})
        self.assertEqual(self.run_cli().returncode, 1)
        (self.root / 'data/spells/probe.json').write_text('{')
        self.assertEqual(self.run_cli().returncode, 1)

    def test_grid_and_exception_disagreements_are_rejected(self):
        self.data({'settlingMs': 12000})
        source = self.root / 'src/core/loop.ts'
        source.write_text(source.read_text().replace('TICK_MS = 10', 'TICK_MS = 20'))
        mismatch = self.run_cli()
        self.assertEqual(mismatch.returncode, 1)
        self.assertIn('grid constants disagree', mismatch.stderr)
        shutil.copyfile(ROOT / 'src/core/loop.ts', source)
        source = self.root / 'sim/time_fields.rs'
        source.write_text(source.read_text().replace('cooldownReductionBelowMs', 'otherThresholdMs'))
        mismatch = self.run_cli()
        self.assertEqual(mismatch.returncode, 1)
        self.assertIn('threshold exceptions disagree', mismatch.stderr)


if __name__ == '__main__':
    unittest.main()
