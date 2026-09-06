#!/usr/bin/env python3
"""Non-blocking PreToolUse/Bash load warning (#1049); never execute input.

This classifies visible shell commands, not arbitrary shell programs. Commands
hidden behind aliases, variables or unrecognised scripts remain outside scope.
"""

import json
import os
import re
import shlex
import sys


ASSIGNMENT = re.compile(r'[A-Za-z_][A-Za-z0-9_]*=')
LOAD_TOOLS = {'npm', 'npx', 'pnpm', 'yarn', 'cargo', 'cargo-remote-fmt',
              'rustc', 'vitest', 'prettier', 'rota-sim'}


def segment_load(words, command, depth):
    while words and (ASSIGNMENT.match(words[0]) or words[0] in {'!', 'then', 'do'}):
        words = words[1:]
    if not words:
        return False
    tool = os.path.basename(words[0])
    args = words[1:]
    if tool in {'env', 'command', 'exec', 'time'}:
        if tool == 'command' and any(arg in {'-v', '-V'} for arg in args[:1]):
            return False
        valued = {'env': {'-u', '--unset', '-C', '--chdir'},
                  'time': {'-f', '--format', '-o', '--output'}}.get(tool, set())
        while args:
            if args[0] == '--':
                args = args[1:]
                break
            if args[0] in valued:
                args = args[2:]
            elif args[0].startswith('-') or ASSIGNMENT.match(args[0]):
                args = args[1:]
            else:
                break
        return segment_load(args, command, depth)
    if tool in LOAD_TOOLS:
        return args not in [['--version'], ['-v'], ['--help'], ['-h']]
    if tool == 'git':
        # Read-only status/log/show calls stay quiet; worktree changes and pushes
        # also appeared in the reported interference incidents.
        while args and args[0].startswith('-'):
            args = args[2:] if args[0] in {'-C', '-c', '--git-dir', '--work-tree'} else args[1:]
        return bool(args and (args[0] == 'push' or args[:2] in [
            ['worktree', 'add'], ['worktree', 'remove'],
        ]))
    if tool == 'jq':
        return bool(re.search(r'(^|[\s/])recordings/', command))
    if tool in {'bash', 'sh', 'zsh'}:
        for index, arg in enumerate(args):
            if re.fullmatch(r'-[a-z]*c[a-z]*', arg) and index + 1 < len(args):
                return load_command(args[index + 1], depth + 1)
    if tool in {'bash', 'sh', 'zsh', 'python3', 'python', 'node'}:
        # Options such as -c/-e contain code, not a tools/ script filename.
        if args and args[0] in {'-c', '-e', '--eval'}:
            return False
        paths = [arg for arg in args if not arg.startswith('-')]
        return bool(paths and re.search(r'(^|/)tools/', paths[0]))
    return bool(re.search(r'(^|/)tools/', words[0]) or
                re.search(r'(^|/)target/(debug|release)/(examples/|deps/)', words[0]))


def load_command(command, depth=0):
    if depth > 8:
        raise ValueError('shell nesting exceeds classifier limit')
    # Keep quotes until AFTER separating operators: echo '&&' npm run lint
    # is text, whereas echo ok && npm run lint starts real work.
    lexer = shlex.shlex(command, posix=False, punctuation_chars=';&|()\n')
    lexer.whitespace = ' \t\r'
    lexer.whitespace_split = True
    words = []
    for token in lexer:
        if token and all(char in ';&|()\n' for char in token):
            if segment_load(words, command, depth):
                return True
            words = []
        else:
            while True:
                try:
                    decoded = shlex.split(token, posix=True)
                    break
                except ValueError:
                    following = next(lexer, None)
                    if following is None:
                        raise
                    token += ' ' + following
            if len(decoded) != 1:
                raise ValueError('unsupported shell token')
            words.append(decoded[0])
    return segment_load(words, command, depth)


def own_role(cwd):
    import shutil
    import subprocess

    executable = shutil.which('rota-ipc')
    if not executable:
        return None
    try:
        result = subprocess.run([executable, 'wer'], cwd=cwd, capture_output=True,
                                text=True, timeout=1, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode:
        return None
    match = re.search(r'^Rolle:\s+(\S+)\s*$', result.stdout, re.MULTILINE)
    return match[1] if match else None


def switch_warning(event):
    from pathlib import Path
    import shutil

    override = os.environ.get('ROTA_FENSTER_STAND')
    if not override and not shutil.which('rota-fenster'):
        return None
    path = Path(override) if override else Path.home() / '.claude/rota-fenster.json'
    try:
        stand = json.loads(path.read_text(encoding='utf-8'))
        if not isinstance(stand, dict) or not {'inhaber', 'abgabe'} <= stand.keys():
            raise ValueError('missing switch fields')
        for key in ['inhaber', 'abgabe', 'zweck', 'seit', 'kennung']:
            if stand.get(key) is not None and not isinstance(stand[key], str):
                raise ValueError('invalid switch field type')
        owner = stand['inhaber']
        if not owner or stand['abgabe'] is not None:
            return None
        if not stand.get('zweck') or not stand.get('seit'):
            raise ValueError('held window lacks purpose or timestamp')
    except (OSError, ValueError):
        return '[Fenster] Schalterstand unlesbar oder ungültig; Belegung UNBEKANNT. Keine Sperre.'
    session = event.get('session_id')
    if isinstance(session, str) and session and stand.get('kennung') in {
        session, f'session-{session}', f'session-{session[:8]}',
    }:
        return None
    # The existing IPC command verifies the role against the caller process.
    # A read failure cannot establish ownership: keep the foreign warning.
    if not stand.get('kennung') and owner == own_role(event.get('cwd')):
        return None
    purpose = ' '.join(stand['zweck'].split())[:150]
    return (f"[MESSFENSTER] {owner} misst seit {stand['seit']}. Zweck: {purpose}. "
            'Dieser Befehl kann die gemeinsame Maschine belasten. Keine Sperre; '
            'die Warnung erteilt keine Ausführungsberechtigung.')


def main():
    try:
        event = json.load(sys.stdin)
        if not isinstance(event, dict):
            raise ValueError('invalid hook event')
        if event.get('tool_name') != 'Bash':
            return 0
        tool_input = event.get('tool_input')
        if not isinstance(tool_input, dict) or not isinstance(tool_input.get('command'), str):
            raise ValueError('missing shell command')
        if not load_command(tool_input['command']):
            return 0
        warning = switch_warning(event)
    except (ValueError, TypeError, RecursionError):
        warning = '[Fenster] Hook-Eingabe nicht auswertbar; Lastprüfung UNBEKANNT. Keine Sperre.'
    if warning:
        # Exit 0 + additionalContext reaches the model without granting or
        # denying the call; systemMessage also makes it visible to the user.
        print(json.dumps({'systemMessage': warning, 'hookSpecificOutput': {
            'hookEventName': 'PreToolUse', 'additionalContext': warning,
        }}, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
