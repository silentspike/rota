#!/usr/bin/env python3
"""Non-blocking PreToolUse/Bash load warning (#1049); never execute input.

The switch decides first. Without the window tool, with a free, released or own
window the hook ends before any command analysis, so nothing is reported there,
not even input it could not split (#1049 point 2). Only while another session
holds the window, or while the switch is unreadable, is the visible command
classified; the classifier (fenster_lastpruefung.py) is imported only then, so
ordinary Bash calls do not pay for compiling it.

The place of the window decides which load counts (rota-fenster --ort): in a
node3 window only the build service (`cargo remote`, `cargo-remote-fmt`), which
runs on that node; in a Laptop window, or with an old switch without a place,
only local load. Undecidable input is reported in both.
"""

import json
import os
import re
import sys

UNREADABLE = 'unreadable'
PLACES = ('laptop', 'node3')   # rota-fenster --ort; a switch without a place is a Laptop window
NO_PERMISSION = ' Keine Sperre; die Warnung erteilt keine Ausführungsberechtigung.'


def which(name):
    """shutil.which without importing shutil before every Bash call."""
    search = os.environ.get('PATH', os.defpath)
    for directory in search.split(os.pathsep) if search else []:
        candidate = os.path.join(directory, name)
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return None


def read_switch():
    """None without the window tool, UNREADABLE, or the validated switch."""
    override = os.environ.get('ROTA_FENSTER_STAND')
    if not override and not which('rota-fenster'):
        return None
    path = override or os.path.join(os.path.expanduser('~'), '.claude', 'rota-fenster.json')
    try:
        with open(path, encoding='utf-8') as source:
            stand = json.load(source)
        if not isinstance(stand, dict) or not {'inhaber', 'abgabe'} <= stand.keys():
            raise ValueError('missing switch fields')
        for key in ['inhaber', 'abgabe', 'zweck', 'seit', 'kennung', 'ort']:
            if stand.get(key) is not None and not isinstance(stand[key], str):
                raise ValueError('invalid switch field type')
        if stand.get('ort') is not None and stand['ort'] not in PLACES:
            raise ValueError('unknown place')
        if held(stand) and not (stand.get('zweck') and stand.get('seit')):
            raise ValueError('held window lacks purpose or timestamp')
    except (OSError, ValueError, RecursionError):
        return UNREADABLE
    return stand


def held(stand):
    return bool(stand['inhaber']) and stand['abgabe'] is None


def own_session(stand, event):
    session = event.get('session_id')
    return isinstance(session, str) and bool(session) and stand.get('kennung') in {
        session, f'session-{session}', f'session-{session[:8]}',
    }


def own_role(cwd):
    import subprocess

    executable = which('rota-ipc')
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


def main():
    try:
        event = json.load(sys.stdin)
    except (ValueError, RecursionError):
        event = None
    if not isinstance(event, dict):
        event = None
    elif event.get('tool_name') != 'Bash':
        return 0
    stand = read_switch()
    if stand is None or (stand != UNREADABLE and not held(stand)):
        return 0
    if stand != UNREADABLE and event and own_session(stand, event):
        return 0
    place = 'laptop' if stand == UNREADABLE else stand.get('ort') or 'laptop'
    try:
        tool_input = event.get('tool_input') if event else None
        if not isinstance(tool_input, dict) or not isinstance(tool_input.get('command'), str):
            raise ValueError('missing shell command')
        from fenster_lastpruefung import BAU, LOKAL, analyse
        cwd = event.get('cwd') if isinstance(event.get('cwd'), str) else None
        kinds, error = analyse(tool_input['command'], cwd=cwd)
        # On node3 the build service shares the CPU with the measurement; there
        # only `cargo remote` disturbs. On the Laptop only local load does. An
        # unreadable switch could be either (rota#1049, decision of 28.10.).
        wanted = {LOKAL, BAU} if stand == UNREADABLE else {BAU} if place == 'node3' else {LOKAL}
        if kinds & wanted:
            undecided = False
        elif error:
            undecided = True     # the undecided part could be the wanted kind
        else:
            return 0
    except (ValueError, TypeError, RecursionError, ImportError):
        undecided = True
    if stand == UNREADABLE:
        warning = '[Fenster] Schalterstand unlesbar oder ungültig; Belegung UNBEKANNT. Keine Sperre.'
    elif not stand.get('kennung') and event and stand['inhaber'] == own_role(event.get('cwd')):
        # Old switches without a session ID: the IPC tool verifies the caller's
        # role against its process. Asked only now, because it costs a process.
        return 0
    else:
        purpose = ' '.join(stand['zweck'].split())[:150]
        where = 'auf node3' if place == 'node3' else 'auf dem Laptop'
        target = 'den Bau-Dienst auf node3' if place == 'node3' else 'die gemeinsame Maschine'
        warning = (f"[MESSFENSTER] {stand['inhaber']} misst seit {stand['seit']} {where}. "
                   f"Zweck: {purpose}. "
                   + ('Hook-Eingabe nicht auswertbar; Lastprüfung UNBEKANNT.' if undecided else
                      f'Dieser Befehl kann {target} belasten.') + NO_PERMISSION)
    # Exit 0 + additionalContext reaches the model without granting or
    # denying the call; systemMessage also makes it visible to the user.
    print(json.dumps({'systemMessage': warning, 'hookSpecificOutput': {
        'hookEventName': 'PreToolUse', 'additionalContext': warning,
    }}, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
