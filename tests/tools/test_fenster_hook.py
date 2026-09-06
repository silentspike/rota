"""Exercise the configured hook with real disposable switch files, never live IPC."""

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
HOOK = ROOT / 'tools' / 'fenster-hook.py'


class FensterHookTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='rota window hook ')
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.stand = self.base / 'stand.json'
        self.bin = self.base / 'bin'
        self.bin.mkdir()
        for name, content in {
            'rota-fenster': '#!/bin/sh\nexit 0\n',
            'rota-ipc': '#!/bin/sh\n[ "$1" = wer ] || exit 91\nprintf "Rolle: LocalReview\\n"\n',
        }.items():
            path = self.bin / name
            path.write_text(content)
            path.chmod(0o700)
        self.env = {**os.environ, 'PATH': str(self.bin), 'ROTA_FENSTER_STAND': str(self.stand)}
        self.state()

    def state(self, **changes):
        value = {'inhaber': 'OtherReview', 'zweck': 'Isolated measurement',
                 'seit': '2026-09-06 09:00:00', 'abgabe': None, 'kennung': 'session-other'}
        value.update(changes)
        self.stand.write_text(json.dumps(value))

    def call(self, command='npm run lint', session='local', env=None, raw=None):
        before = hashlib.sha256(self.stand.read_bytes()).hexdigest() if self.stand.exists() else None
        payload = raw if raw is not None else json.dumps({
            'hook_event_name': 'PreToolUse', 'tool_name': 'Bash',
            'session_id': session, 'cwd': str(self.base), 'tool_input': {'command': command},
        })
        result = subprocess.run([sys.executable, '-B', str(HOOK)], input=payload,
                                text=True, capture_output=True, env=env or self.env, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        after = hashlib.sha256(self.stand.read_bytes()).hexdigest() if self.stand.exists() else None
        self.assertEqual(before, after)
        if not result.stdout:
            return None
        value = json.loads(result.stdout)
        self.assertNotIn('decision', value)
        self.assertNotIn('continue', value)
        specific = value['hookSpecificOutput']
        self.assertEqual(specific['hookEventName'], 'PreToolUse')
        self.assertNotIn('permissionDecision', specific)
        self.assertEqual(specific['additionalContext'], value['systemMessage'])
        return value['systemMessage']

    def test_registered_for_bash_without_changing_permissions(self):
        settings = json.loads((ROOT / '.claude/settings.json').read_text())
        commands = [hook['command'] for rule in settings['hooks']['PreToolUse']
                    if rule.get('matcher') == 'Bash' for hook in rule['hooks']]
        self.assertIn('python3 -B "${CLAUDE_PROJECT_DIR}/tools/fenster-hook.py"', commands)

    def test_incident_commands_warn_with_owner_purpose_and_time(self):
        for command in [
            'npm run lint', 'npx prettier --write docs/WORKER-GUIDE.md',
            "jq '.casts' tests/recordings/*", 'cargo remote --no-copy-lock -- test',
            'cargo-remote-fmt --check', 'bash tools/gegenprobe-grid.sh',
            'python3 tools/streich-mutation.py', 'git worktree add /tmp/unused branch',
        ]:
            with self.subTest(command=command):
                warning = self.call(command)
                self.assertIn('MESSFENSTER', warning)
                for detail in ['OtherReview', 'Isolated measurement', '2026-09-06 09:00:00']:
                    self.assertIn(detail, warning)

    def test_quiet_commands_in_the_same_foreign_window(self):
        for command in ['git status --short', 'cat docs/README.md', 'pwd',
                        'echo "npm run lint; npx prettier"', "printf '%s' 'cargo test'",
                        "jq '.name' package.json", 'npm --version',
                        "python3 -c 'print(\"npm run lint\")'"]:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))

    def test_own_free_and_released_windows_are_quiet(self):
        self.assertIsNone(self.call(session='other'))
        self.state(inhaber='LocalReview', kennung=None)
        self.assertIsNone(self.call())
        self.state(inhaber=None, zweck=None, seit=None, kennung=None)
        self.assertIsNone(self.call())
        self.state(abgabe='2026-09-06 09:01:00')
        self.assertIsNone(self.call())

    def test_missing_invalid_or_wrong_schema_is_unknown_without_blocking(self):
        for content in ['broken {{{', '[]', '{}', '{"inhaber": 17}',
                        '{"inhaber": "OtherReview", "abgabe": false}']:
            with self.subTest(content=content):
                self.stand.write_text(content)
                self.assertIn('UNBEKANNT', self.call())
        self.stand.unlink()
        self.assertIn('UNBEKANNT', self.call())

    def test_absent_installation_is_quiet(self):
        env = {**self.env, 'PATH': str(self.base / 'missing-bin')}
        env.pop('ROTA_FENSTER_STAND')
        self.assertIsNone(self.call(env=env))

    def test_wrappers_pipelines_and_command_substitution_are_classified(self):
        for command in ['cd sim && cargo remote -- test', 'FOO=bar npm run lint',
                        'env FOO=bar npx prettier --write docs/README.md',
                        'bash -lc "npm run lint"',
                        "cat tests/recordings/* | jq '.casts'",
                        'echo $(npm run lint)', 'true\nnpx prettier --check docs/README.md']:
            with self.subTest(command=command):
                self.assertIn('MESSFENSTER', self.call(command))

    def test_input_is_never_executed_and_errors_never_grant_permission(self):
        marker = self.base / 'must-not-exist'
        self.assertIn('MESSFENSTER', self.call(f'npm run lint; touch "{marker}"'))
        self.assertFalse(marker.exists())
        for payload in ['{broken', '[]', '{"tool_name":"Bash","tool_input":null}']:
            with self.subTest(payload=payload):
                self.assertIn('UNBEKANNT', self.call(raw=payload))


if __name__ == '__main__':
    unittest.main()
