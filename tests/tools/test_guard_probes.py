"""Exercise the real CI wrapper and guard in disposable, public fixtures."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


class GuardProbeGate(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='rota-guard-gate-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for folder in ('tools', 'docs', 'data', 'src', 'tests/unit', 'tests/parity/recordings'):
            (self.root / folder).mkdir(parents=True)
        for name in ('gegenproben-ci.py', 'guide-abschnittsnummern.sh',
                     'guide-abschnittsnummern-gegenprobe.sh'):
            shutil.copy2(ROOT / 'tools' / name, self.root / 'tools' / name)
        for name in ('VERIFICATION', 'one', 'two', 'three', 'four'):
            (self.root / 'docs' / f'{name}.md').write_text('## 1.1 Public fixture\n')
        for name in ('package.json', 'tsconfig.json', 'tests/unit/spell-order.json',
                     'tests/parity/recordings/baseline.json'):
            (self.root / name).write_text('{}\n')
        (self.root / 'tests/unit/spell-order.test.ts').write_text('// Not executed by this fixture.\n')

    def run_probe(self, name='guide-abschnittsnummern-gegenprobe.sh'):
        before = {str(p.relative_to(self.root)): p.read_bytes()
                  for p in self.root.rglob('*') if p.is_file()}
        result = subprocess.run(
            [sys.executable, '-B', str(self.root / 'tools/gegenproben-ci.py'), '--probe', name],
            text=True, capture_output=True, check=False,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
        )
        after = {str(p.relative_to(self.root)): p.read_bytes()
                 for p in self.root.rglob('*') if p.is_file()}
        self.assertEqual(before, after, 'The actual input checkout was mutated')
        return result

    def test_real_guard_passes_its_positive_and_negative_cases(self):
        result = self.run_probe()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('B echte Dublette', result.stdout)
        self.assertIn('Alarm, Exit 1', result.stdout)
        self.assertIn('Guard probes: 1 passed', result.stdout)

    def test_broken_real_guard_contract_makes_the_ci_command_fail(self):
        guard = self.root / 'tools/guide-abschnittsnummern.sh'
        original = guard.read_text()
        self.assertEqual(original.count('Doppelt vergebene'), 1)
        guard.write_text(original.replace('Doppelt vergebene', 'Mehrfach vergebene'))
        result = self.run_probe()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('die GEGENPROBE passt nicht mehr', result.stdout)
        self.assertIn('failed with exit 1', result.stderr)

    def test_successful_exit_cannot_hide_changed_top_level_input(self):
        (self.root / 'tools/ac6-gegenprobe.sh').write_text(
            '#!/bin/sh\nprintf "changed\\n" >> package.json\nexit 0\n')
        result = self.run_probe('ac6-gegenprobe.sh')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('left changed files: package.json', result.stderr)

    def test_failed_exit_cannot_hide_behind_restored_files(self):
        (self.root / 'tools/ac6-gegenprobe.sh').write_text('#!/bin/sh\nexit 7\n')
        result = self.run_probe('ac6-gegenprobe.sh')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('failed with exit 7', result.stderr)


if __name__ == '__main__':
    unittest.main()
