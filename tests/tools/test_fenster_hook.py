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

    def test_configured_command_resolves_project_paths_with_spaces(self):
        scripts = self.base / 'project with spaces' / 'tools'
        scripts.mkdir(parents=True)
        (scripts / HOOK.name).write_bytes(HOOK.read_bytes())
        (self.bin / 'python3').symlink_to(sys.executable)
        settings = json.loads((ROOT / '.claude/settings.json').read_text())
        command = next(hook['command'] for rule in settings['hooks']['PreToolUse']
                       if rule.get('matcher') == 'Bash' for hook in rule['hooks'])
        result = subprocess.run(['/bin/bash', '-c', command], input=json.dumps({
            'tool_name': 'Bash', 'tool_input': {'command': 'npm run lint'},
            'session_id': 'other', 'cwd': str(self.base),
        }), text=True, capture_output=True, env={
            **self.env, 'CLAUDE_PROJECT_DIR': str(scripts.parent),
        }, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, '')  # same owning session

    def test_incident_commands_warn_with_owner_purpose_and_time(self):
        for command in [
            'npm run lint', 'npx prettier --write docs/WORKER-GUIDE.md',
            "jq '.casts' tests/parity/recordings/*",
            "jq -s length /work/rota/tests/parity/recordings/*.json",
            'bash tools/gegenprobe-grid.sh',
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
                        "echo '&&' npm run lint", 'command -v npm', 'command -V cargo',
                        "jq '.name' package.json", 'npm --version',
                        "python3 -c 'print(\"npm run lint\")'"]:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))

    def test_own_free_and_released_windows_are_quiet(self):
        self.assertIsNone(self.call(session='other'))
        self.state(kennung='session-12345678')
        self.assertIsNone(self.call(session='12345678-abcd-1234-5678-123456789abc'))
        self.state(inhaber='LocalReview', kennung='session-different')
        self.assertIn('MESSFENSTER', self.call())
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
        for command in ['FOO=bar npm run lint',
                        'env FOO=bar npx prettier --write docs/README.md',
                        'FOO="a b" npm run lint', 'env -u NODE_OPTIONS npm run lint',
                        'env -C /work/rota npm run lint', '/usr/bin/time -f %E npm run lint',
                        'git -C /work/rota push',
                        'bash -lc "npm run lint"',
                        "cat tests/parity/recordings/* | jq '.casts'",
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

    def report(self, command, expected, **kwargs):
        message = self.call(command, **kwargs)
        self.assertIsNotNone(message, f'quiet, but {expected!r} was expected')
        self.assertIn(expected, message)
        return message

    # B1: the switch decides first. Nobody else holds the window, so nothing is
    # reported -- not even for input the classifier cannot split (#1049 point 2).
    UNSPLITTABLE = ["cat <<'EOF' | wc -l\nIt's data\nEOF", "echo 'unterminated",
                    'npm run lint "unterminated']

    def assert_window_silent(self, session='local', env=None):
        for command in self.UNSPLITTABLE + ['npm run lint']:
            with self.subTest(command=command, session=session):
                self.assertIsNone(self.call(command, session=session, env=env))

    def test_free_window_never_reports_even_unsplittable_input(self):
        self.state(inhaber=None, zweck=None, seit=None, kennung=None)
        self.assert_window_silent()
        self.assertIsNone(self.call(raw='{broken'))

    def test_released_window_never_reports_even_unsplittable_input(self):
        self.state(abgabe='2026-09-06 09:01:00')
        self.assert_window_silent()
        self.assertIsNone(self.call(raw='{broken'))

    def test_own_window_never_reports_even_unsplittable_input(self):
        self.assert_window_silent(session='other')
        self.state(kennung='session-12345678')
        self.assert_window_silent(session='12345678-abcd-1234-5678-123456789abc')
        self.state(inhaber='LocalReview', kennung=None)    # old switch: verified role
        self.assert_window_silent()

    def test_absent_window_tool_never_reports_even_unsplittable_input(self):
        env = {**self.env, 'PATH': str(self.base / 'missing-bin')}
        env.pop('ROTA_FENSTER_STAND')
        self.assert_window_silent(env=env)

    def test_unsplittable_input_in_foreign_window_names_holder(self):
        for command in ["echo 'unterminated", 'echo "unterminated', 'echo `unterminated']:
            with self.subTest(command=command):
                warning = self.report(command, 'UNBEKANNT')
                for detail in ['OtherReview', 'Isolated measurement', '2026-09-06 09:00:00']:
                    self.assertIn(detail, warning)
        # Bash runs earlier lines before it reaches a syntax error.
        self.report("npm run lint\necho 'unterminated", 'belasten')

    # B2: heredoc bodies are data unless an interpreter reads them as code or the
    # unquoted delimiter lets the shell expand $( ) and backquotes in them.
    def test_heredoc_bodies_are_data(self):
        for command in [
            "cat <<'EOF' | wc -l\nIt's data\nEOF",
            "git commit -F - <<'EOF'\nnpm run lint\nEOF",
            'git commit -m "$(cat <<\'EOF\'\nfix: don\'t run npm run lint twice\nEOF\n)"',
            "rota-ipc senden Orchestrator - <<'ENDE'\nnpx prettier --write docs/WORKER-GUIDE.md "
            "lief, cargo remote -- test auch\nENDE",
            "gh pr create --title t --body-file - <<'EOF'\n## Checks\n\nnpm run lint     clean\n"
            "cargo remote -c -- test   0 FAILED\nEOF",
            "python3 - <<'EOF'\nprint('npm run lint')\nEOF",
            "python3 - <<'EOF'\npath = 'tools/ac6.sh'\ntext = open(path).read()\nEOF",
            "if true; then\n\tcat <<-'EOF'\n\tnpm run lint\n\tEOF\nfi",
        ]:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))

    def test_heredocs_that_run_code_warn(self):
        for command in [
            "bash <<'EOF'\nnpm run lint\nEOF",
            "python3 - <<'EOF'\nimport subprocess\nsubprocess.run(['npx', 'vitest', 'run'])\nEOF",
            "python3 - <<'EOF'\nimport importlib.util\n"
            "spec = importlib.util.spec_from_file_location('m', 'tools/streich-mutation.py')\nEOF",
            'cat <<EOF\n$(npm run lint)\nEOF',
            'cat > notes.md <<EOF\nRun `npm run lint`\nEOF',
            "cat <<'EOF'\ntext\nEOF\nnpm run lint",
            "cat <<'EOF' && npx prettier --check docs/README.md\ntext\nEOF",
        ]:
            with self.subTest(command=command):
                self.report(command, 'belasten')

    def test_long_heredoc_stays_within_configured_hook_timeout(self):
        # Re-splitting an open quote token by token took 6 s for this message.
        settings = json.loads((ROOT / '.claude/settings.json').read_text())
        limit = next(hook['timeout'] for rule in settings['hooks']['PreToolUse']
                     if rule.get('matcher') == 'Bash' for hook in rule['hooks'])
        body = "It's the report.\n" + '\n'.join(f'line {i}: value {i}, no deviation' for i in range(200))
        command = f"rota-ipc senden Orchestrator - <<'ENDE'\n{body}\nENDE"
        payload = json.dumps({'tool_name': 'Bash', 'session_id': 'local', 'cwd': str(self.base),
                              'tool_input': {'command': command}})
        try:
            result = subprocess.run([sys.executable, '-B', str(HOOK)], input=payload, text=True,
                                    capture_output=True, env=self.env, timeout=limit)
        except subprocess.TimeoutExpired:
            self.fail(f'hook exceeded its configured {limit} s timeout; the warning would be lost')
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))

    # B3: process wrappers start the command they are given.
    def test_process_wrappers_are_unwrapped(self):
        for command in [
            'timeout -k 5 60 npm run lint',
            'timeout --signal=KILL 5m npx prettier --check docs/README.md',
            'nohup npm run lint > /work/tmp/lint.log 2>&1 &',
            'stdbuf -oL npm run lint', 'ionice -c3 npm run lint',
            'git diff --name-only | xargs -r npx prettier --check',
            'timeout 600 nice -n 5 python3 tools/streich-mutation.py',
            'if git push origin HEAD; then echo ok; fi',
            'run() { npx vitest run; }; run', 'sudo -u root npm run lint',
            "find docs -name '*.md' -exec npx prettier --check {} +", "eval 'npm run lint'",
            'source tools/env.sh', '. tools/env.sh',
        ]:
            with self.subTest(command=command):
                self.report(command, 'belasten')
        for command in ['timeout 5 git status --short', 'nohup sleep 1 &', 'ls | xargs echo',
                        'taskset -p 1234', 'ionice -p 1234', 'timeout --help']:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))

    # B4: help and version forms start no build, test or format run -- also with
    # a subcommand. Cargo Remote v2 uses -h for --sync-hidden, which still builds.
    def test_help_and_version_forms_are_quiet(self):
        # `cargo-remote-fmt -h` is help: its format mode parses with argparse's
        # own -h before any remote action (measured 28.10.: usage, exit 0, no
        # job). Only `cargo remote` maps -h to --sync-hidden. This list said
        # the opposite until the review of 24.10. found it.
        for command in ['cargo remote --help', 'cargo remote --version', 'cargo test --help',
                        'cargo help build', 'cargo --version', 'npx prettier --help',
                        'npx vitest --version', 'npm run lint --help', 'rustc -V',
                        'cargo-remote-fmt --help', 'cargo-remote-fmt -h']:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))
        for command in ['npm run lint -- --help',
                        'cargo test -- --help', 'npx tsx tools/x.ts --help', 'yarn lint --help']:
            with self.subTest(command=command):
                self.report(command, 'belasten')

    def test_commands_hidden_in_quotes_comments_and_names(self):
        # $( ) and backquotes also run inside double quotes; single quotes are text.
        for command in ['echo "lint: $(npm run lint 2>&1 | tail -1)"', 'echo "a `npm run lint` b"',
                        'rm -f x  # comment\nnpx prettier --write docs/README.md',
                        'bash <(git show origin/main:tools/ac6-gegenprobe.sh)',
                        'bash -o pipefail -c "npm run lint | tail -1"', 'bash - tools/ac6.sh']:
            with self.subTest(command=command):
                self.report(command, 'belasten')
        for command in ["echo 'a `npm run lint` b'", 'echo hi  # npm run lint',
                        'exec -a /work/rota/target/debug/deps/probe_1 sleep 1']:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))
        # A program named only at run time is undecidable, not quiet.
        warning = self.report('B=$(ls -t target/debug/deps/x-* | head -1)\n(cd sim && "../$B")',
                              'UNBEKANNT')
        self.assertIn('OtherReview', warning)
        # So is code that reaches an interpreter through a pipe.
        for command in ['cat run.sh | bash', "echo 'npm run lint' | sh"]:
            with self.subTest(command=command):
                self.report(command, 'UNBEKANNT')

    # Findings of the review of 24.10. (f-lastwarnung-hook-3): each of these
    # stayed quiet in a foreign window.
    def test_flock_is_unwrapped_like_the_other_wrappers(self):
        for command in ['flock -w 900 /work/tmp/rota-ts-schwer.lock npx vitest run x.test.ts --maxWorkers=1',
                        'flock -w 900 /work/tmp/rota-ts-schwer.lock npm run lint',
                        'flock /work/tmp/x.lock -c "npm run lint"', 'flock -x -n /work/tmp/x.lock npx tsc']:
            with self.subTest(command=command):
                self.report(command, 'belasten')
        for command in ['flock 9', 'flock -u 9', 'flock /work/tmp/x.lock -c "git status"']:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))

    def test_load_tools_are_recognised_by_path_and_as_script_operands(self):
        for command in ['node_modules/.bin/tsc --noEmit', './node_modules/.bin/eslint .',
                        'node_modules/.bin/vite build', 'npx --no-install tsc -p .',
                        'node node_modules/vitest/vitest.mjs run',
                        'tsc --noEmit', 'eslint .', 'vite build', 'tsx tools/x.ts']:
            with self.subTest(command=command):
                self.report(command, 'belasten')
        for command in ['tsc --version', 'eslint --help', 'vite --version']:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))

    def test_scripts_are_classified_by_what_they_run(self):
        (self.base / 'laeuft.sh').write_text('#!/bin/sh\nnpx vitest run\n')
        (self.base / 'still.sh').write_text('#!/bin/sh\necho fertig\n')
        (self.base / 'probe.py').write_text("import subprocess\nsubprocess.run(['npm', 'run', 'lint'])\n")
        (self.base / 'leise.py').write_text("print('npm run lint')\n")
        for command in [
            # written and run in the same call: the file does not exist yet
            "cat > /work/tmp/never-written.sh <<'EOF'\nnpx vitest run\nEOF\nbash /work/tmp/never-written.sh",
            "tee /work/tmp/never-written-2.sh > /dev/null <<'EOF'\nnpm run lint\nEOF\n"
            "sh /work/tmp/never-written-2.sh",
            # written earlier (by another tool) and run now
            'bash laeuft.sh', f'sh "{self.base}/laeuft.sh"', 'python3 probe.py', 'source laeuft.sh',
        ]:
            with self.subTest(command=command):
                self.report(command, 'belasten')
        for command in ['bash still.sh', 'python3 leise.py',
                        "cat > /work/tmp/never-written-3.sh <<'EOF'\necho hi\nEOF\nbash /work/tmp/never-written-3.sh"]:
            with self.subTest(command=command):
                self.assertIsNone(self.call(command))
        # A script that is neither visible nor readable is not known to be quiet.
        self.report('bash gibt-es-nicht.sh', 'UNBEKANNT')

    # The place of the window (rota-fenster --ort, rota#1049): on node3 the
    # build service shares the CPU with the measurement, so there a foreign
    # `cargo remote` disturbs and local load does not; on the Laptop it is the
    # other way round (f-lastwarnung-hook-3, decision of 28.10.). Every
    # command below was a Laptop-window case before; none is dropped.
    BUILD_SERVICE = [
        'cargo remote --no-copy-lock -- test',
        'cargo-remote-fmt --check',
        'cd sim && cargo remote -- test',
        'sh -s <<EOF\ncargo remote -- test\nEOF',
        'timeout 1800 cargo remote -c -- test --no-run',
        'nice -n 10 cargo remote -- test',
        'taskset -c 0-3 cargo remote -- test',
        'setsid nohup cargo-remote-fmt --check &',
        '{ time cargo remote -c -- test; } 2>&1',
        'xargs -I{} sh -c "cargo remote -- test {}"',
        'cargo remote -h -- build',
        'flock --timeout 60 /work/tmp/x.lock cargo remote -- test',
        'bash ~/bin/cargo-remote-fmt --check',
        'sh /home/jan/bin/cargo-remote-fmt',
        'cargo remote --resources=large -c -- build --release',
        "python3 -c \"import subprocess; subprocess.run(['cargo', 'remote', '--', 'test'])\"",
    ]

    def tools_scripts(self):
        (self.base / 'tools').mkdir(exist_ok=True)
        (self.base / 'tools/remote.sh').write_text('#!/bin/sh\ncargo remote --no-copy-lock -- test\n')
        (self.base / 'tools/lokal.sh').write_text('#!/bin/sh\nnpx vitest run\n')
        (self.base / 'tools/beides.sh').write_text('#!/bin/sh\nnpx vitest run\ncargo remote -- test\n')

    def test_build_service_commands_warn_only_in_a_foreign_node3_window(self):
        self.tools_scripts()
        for command in self.BUILD_SERVICE:
            with self.subTest(command=command):
                self.state(ort='node3')
                warning = self.report(command, 'belasten')
                for detail in ['OtherReview', 'Isolated measurement',
                               'misst seit 2026-09-06 09:00:00 auf node3', 'den Bau-Dienst auf node3']:
                    self.assertIn(detail, warning)
                self.state(ort='laptop')
                self.assertIsNone(self.call(command), 'a Laptop window is not disturbed by the build service')
                self.state()
                self.assertIsNone(self.call(command), 'an old switch without a place is a Laptop window')
                self.state(ort='node3', abgabe='2026-09-06 09:01:00')
                self.assertIsNone(self.call(command))

    def test_local_load_warns_only_in_a_foreign_laptop_window(self):
        self.tools_scripts()
        for command in ['npm run lint', 'npx prettier --write docs/WORKER-GUIDE.md',
                        "jq '.casts' tests/parity/recordings/*", 'bash tools/lokal.sh',
                        'flock -w 900 /work/tmp/rota-ts-schwer.lock npx vitest run', 'cargo build']:
            with self.subTest(command=command):
                self.state(ort='laptop')
                self.assertIn('misst seit 2026-09-06 09:00:00 auf dem Laptop',
                              self.report(command, 'die gemeinsame Maschine belasten'))
                self.state(ort='node3')
                self.assertIsNone(self.call(command), 'local load does not reach node3')

    def test_a_tools_script_carries_the_load_of_what_it_runs(self):
        self.tools_scripts()
        # A tools/ script always counts as local load — many compute in Python
        # without starting anything — and as build-service load, too, if its
        # text names `cargo remote` (tests-lokal.sh, streich-mutation.py).
        for place in ['laptop', 'node3']:
            with self.subTest(place=place):
                self.state(ort=place)
                self.report('bash tools/beides.sh', 'belasten')
                self.report('bash tools/remote.sh', 'belasten')
                # unreadable: both places, because it could be either
                self.report('bash tools/gibt-es-nicht.sh', 'belasten')

    def test_build_service_queries_are_quiet_in_every_window(self):
        for place in ['laptop', 'node3']:
            for command in ['cargo remote --status', 'cargo remote --logs 07073adf4ce6251799dcfb03',
                            'cargo remote --cancel 07073adf4ce6251799dcfb03',
                            'cargo remote --resume 07073adf4ce6251799dcfb03']:
                with self.subTest(place=place, command=command):
                    self.state(ort=place)
                    self.assertIsNone(self.call(command))

    def test_undecidable_input_is_reported_in_both_places(self):
        for place in ['laptop', 'node3']:
            with self.subTest(place=place):
                self.state(ort=place)
                self.report("echo 'unterminated", 'UNBEKANNT')
                self.report('B=$(ls target/debug/deps/x-*)\n"$B"', 'UNBEKANNT')

    def test_an_unknown_place_makes_the_switch_unreadable(self):
        for place in ['mars', 17, '']:
            with self.subTest(place=place):
                self.state(ort=place)
                self.report('npm run lint', 'Belegung UNBEKANNT')

    def test_configured_command_loads_classifier_next_to_hook_only_when_needed(self):
        scripts = self.base / 'project with spaces' / 'tools'
        scripts.mkdir(parents=True)
        for source in (ROOT / 'tools').glob('fenster*.py'):
            (scripts / source.name).write_bytes(source.read_bytes())
        (self.bin / 'python3').symlink_to(sys.executable)
        settings = json.loads((ROOT / '.claude/settings.json').read_text())
        command = next(hook['command'] for rule in settings['hooks']['PreToolUse']
                       if rule.get('matcher') == 'Bash' for hook in rule['hooks'])

        def run(shell_command):
            result = subprocess.run(['/bin/bash', '-c', command], input=json.dumps({
                'tool_name': 'Bash', 'tool_input': {'command': shell_command},
                'session_id': 'local', 'cwd': str(self.base),
            }), text=True, capture_output=True, env={
                **self.env, 'CLAUDE_PROJECT_DIR': str(scripts.parent),
            }, timeout=5)
            self.assertEqual((result.returncode, result.stderr), (0, ''))
            return json.loads(result.stdout)['systemMessage'] if result.stdout else ''

        self.assertIn('belasten', run('timeout 60 npm run lint'))
        self.assertFalse((scripts / '__pycache__').exists())
        (scripts / 'fenster_lastpruefung.py').unlink()
        self.assertIn('UNBEKANNT', run('timeout 60 npm run lint'))   # never a traceback
        self.state(abgabe='2026-09-06 09:01:00')
        self.assertEqual(run('timeout 60 npm run lint'), '')        # free: classifier unused


if __name__ == '__main__':
    unittest.main()
