"""The one-command start of the full mutation pass (#1055) refuses without the window.

`rota-fenster` is replaced by a script that prints what the real tool prints;
the node_modules source is a stand-in directory built here. git, the disk check
and the command assembly are real. Only --trocken runs here: a real start
creates a worktree and runs for hours.
"""
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOL = ROOT / 'tools/streich-volllauf.py'

FREE = ('Inhaber:   w2-933  (seit 28.10. 14:55:58)\nZweck:     x\nAbgabe:    28.10. 15:27:38\n'
        'Erkannt:   0 Prozess(e)  — erkannt, nicht: es laeuft nichts\n', 0)
HELD_BY = ('Inhaber:   {role}  (seit 28.10. 18:00:00)\nZweck:     #1055 Volllauf\n'
           'Abgabe:    LIEGT NICHT VOR\nErkannt:   0 Prozess(e)  — erkannt, nicht: es laeuft nichts\n', 1)


class StreichVolllauf(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='rota-volllauf-')
        self.addCleanup(temporary.cleanup)
        self.temp = Path(temporary.name)
        (self.temp / 'bin').mkdir()
        # A stand-in for the node_modules source, built here instead of read
        # from the checkout under test: the first version named the repo
        # itself, and in a fresh checkout without node_modules four cases
        # failed with "oder dort fehlt node_modules" (acceptance of PR #1160).
        self.head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True,
                                   text=True, check=True).stdout.strip()
        self.source = self.temp / 'quelle'
        (self.source / 'node_modules').mkdir(parents=True)
        (self.source / 'package-lock.json').write_bytes(subprocess.run(
            ['git', 'show', f'{self.head}:package-lock.json'], cwd=ROOT,
            capture_output=True, check=True).stdout)

    def window(self, text, code):
        # Written line by line: a dedent over the inserted status text left
        # the shebang indented, the start failed, and the PATH search went on
        # to the REAL rota-fenster. The marker proves this one ran.
        self.fake = self.temp / 'bin/rota-fenster'
        self.marker = self.temp / 'fake-called'
        self.fake.write_text('#!/bin/sh\n'
                             f'touch {self.marker}\n'
                             '[ "$1" = status ] || exit 9\n'
                             "cat <<'STATUS'\n" + text.rstrip() + '\nSTATUS\n'
                             f'exit {code}\n')
        self.fake.chmod(0o755)

    def run_tool(self, *args):
        # The node_modules source is always named: its default /work/rota
        # exists on this machine and not on CI — as /work itself does not,
        # which a disk check on a fixed '/work' met there (lint, PR #1160).
        env = dict(os.environ, ROTA_FENSTER=str(self.fake))
        args = (*args, '--node-modules-von', str(self.source))
        result = subprocess.run([sys.executable, '-B', str(TOOL), *args], cwd=ROOT, env=env,
                                capture_output=True, text=True, timeout=60, check=False)
        self.assertTrue(self.marker.exists(), 'the fake rota-fenster was not the one asked')
        return result

    def test_a_free_window_is_not_an_assigned_one(self):
        self.window(*FREE)
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(self.temp / 'lauf'))
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('nicht zugeteilt', result.stdout)

    def test_a_window_held_by_another_role_refuses(self):
        self.window(HELD_BY[0].format(role='w3-912'), HELD_BY[1])
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(self.temp / 'lauf'))
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('w3-912', result.stdout)

    def test_a_window_the_role_already_handed_back_refuses(self):
        handed_back = HELD_BY[0].format(role='w3-streich').replace('LIEGT NICHT VOR', '28.10. 21:00:00')
        self.window(handed_back, 0)
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(self.temp / 'lauf'))
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Abgabe liegt vor', result.stdout)

    def test_the_assigned_window_shows_the_exact_command_without_creating_anything(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        target = self.temp / 'lauf'
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(target))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True,
                              text=True, check=True).stdout.strip()
        self.assertIn(head, result.stdout)
        for part in ('tools/streich-mutation.py', '--vorproben', f'--vergleich {target}/historical-report.md',
                     f'--nachweis {target}/evidence.json', 'setsid', 'nohup', 'TROCKEN'):
            self.assertIn(part, result.stdout)
        self.assertFalse(target.exists(), 'a dry run must not create the output directory')

    def test_a_probe_needs_a_free_window_and_passes_its_fields(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        held = self.run_tool('--rolle', 'w3-streich', '--probe', 'clearsDoom', '--stand', 'HEAD',
                             '--trocken', '--aus', str(self.temp / 'probe'))
        self.assertEqual(held.returncode, 2, held.stdout + held.stderr)
        self.assertIn('belegt', held.stdout)
        self.window(*FREE)
        free = self.run_tool('--rolle', 'w3-streich', '--probe', 'clearsDoom', '--probe', 'heal.amount',
                             '--stand', 'HEAD', '--trocken', '--aus', str(self.temp / 'probe'))
        self.assertEqual(free.returncode, 0, free.stdout + free.stderr)
        for part in ('--vorproben', '--feld clearsDoom', '--feld heal.amount', 'frei (Probe)'):
            self.assertIn(part, free.stdout)
        self.assertFalse((self.temp / 'probe').exists())

    def test_the_checkout_lies_where_rota_fenster_attributes_its_processes(self):
        # rota-fenster knows a role only as a directory under
        # /work/rota-worktrees (its _kennungen()) and matches
        # `rota-worktrees/<name>/` in a process line. The probe of 2026-10-28
        # ran under /work/tmp, and the window was assigned in the middle of it.
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(self.temp / 'lauf'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('git worktree add --detach /work/rota-worktrees/w3-streich-1055-volllauf ',
                      result.stdout)
        self.window(*FREE)
        probe = self.run_tool('--rolle', 'w3-streich', '--probe', 'clearsDoom', '--stand', 'HEAD',
                              '--trocken', '--aus', str(self.temp / 'probe'))
        self.assertIn('git worktree add --detach /work/rota-worktrees/w3-streich-1055-probe ',
                      probe.stdout)

    def test_free_space_is_measured_where_output_and_checkout_go(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(self.temp / 'neu/lauf'), '--baum', str(self.temp / 'neu/baum'))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f'MB frei unter {self.temp.resolve()} ', result.stdout)
        self.assertNotIn('unter /work ', result.stdout)

    def test_an_existing_checkout_directory_refuses(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        (self.temp / 'baum').mkdir()
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(self.temp / 'lauf'), '--baum', str(self.temp / 'baum'))
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Checkout', result.stdout)

    def test_a_source_without_node_modules_or_with_another_lockfile_refuses(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        refusal = (f'NICHT GESTARTET: package-lock.json von {self.head[:12]} gleicht nicht dem von '
                   f'{self.source}, oder dort fehlt node_modules — dann wäre `npm ci` nötig')
        (self.source / 'node_modules').rmdir()
        missing = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                                '--aus', str(self.temp / 'lauf'))
        self.assertEqual(missing.returncode, 2, missing.stdout + missing.stderr)
        self.assertIn(refusal, missing.stdout)
        (self.source / 'node_modules').mkdir()
        lock = self.source / 'package-lock.json'
        lock.write_bytes(lock.read_bytes() + b'\n')
        other = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                              '--aus', str(self.temp / 'lauf'))
        self.assertEqual(other.returncode, 2, other.stdout + other.stderr)
        self.assertIn(refusal, other.stdout)

    def test_an_existing_output_directory_refuses(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        target = self.temp / 'lauf'
        target.mkdir()
        (target / 'evidence.json').write_text('{}')
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'HEAD', '--trocken',
                               '--aus', str(target))
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('existiert', result.stdout)

    def test_an_unknown_revision_refuses(self):
        self.window(HELD_BY[0].format(role='w3-streich'), HELD_BY[1])
        result = self.run_tool('--rolle', 'w3-streich', '--stand', 'kein-solcher-stand',
                               '--trocken', '--aus', str(self.temp / 'lauf'))
        # Exit 2 alone proves nothing: python itself ends with 2 when the
        # script is missing. The tool must say which precondition failed.
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("kein Commit: 'kein-solcher-stand'", result.stdout)


if __name__ == '__main__':
    unittest.main()
