#!/usr/bin/env python3
"""Validate the simulator planning registry and render its three plan tables.

No GitHub working status is stored here. Registry status describes whether
acceptance criteria are documented, never whether implementation has passed.
"""
import argparse
import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
REGISTRY = ROOT / 'docs/sim-issues.json'
PLAN = ROOT / 'docs/SIM-ARCHITEKTUR-PLAN.md'
TASK_ID = re.compile(r'^E\d+\.\d+[a-z]?(?:-(?:core|gpu))?$')
HEADING_ID = re.compile(r'^(E\d+\.\d+[a-z]?(?:-(?:core|gpu))?)\s+[—–]')
TABLES = ('phases', 'dependencies', 'epics')


def slug(title):
    return re.sub(r'\s+', '-', re.sub(r'[^\w\s-]', '', title.lower()).strip())


def headings(text):
    """Markdown headings outside fenced code; duplicate slugs use GitHub suffixes."""
    result, counts, fence = [], collections.Counter(), None
    for number, line in enumerate(text.splitlines()):
        marker = re.match(r'^\s*(`{3,}|~{3,})', line)
        if marker:
            token = marker[1][0]
            if fence is None:
                fence = token
            elif fence == token:
                fence = None
            continue
        match = re.match(r'^(#{1,6})\s+(.+?)\s*$', line)
        if not match or fence:
            continue
        title = match[2]
        base = slug(title)
        anchor = base if not counts[base] else f'{base}-{counts[base]}'
        counts[base] += 1
        result.append({'title': title, 'anchor': anchor, 'level': len(match[1]), 'line': number})
    return result


def package_headings(text):
    start = text.index('### 4.3 E11')
    end = text.index('## Teil 5', start)
    return headings(text[start:end])


def section_text(text, anchor):
    all_headings = headings(text)
    for index, heading in enumerate(all_headings):
        if heading['anchor'] != anchor:
            continue
        end = next((h['line'] for h in all_headings[index + 1:]
                    if h['level'] <= heading['level']), len(text.splitlines()))
        return '\n'.join(text.splitlines()[heading['line'] + 1:end])
    return ''


def validate(registry, plan, assignments=None):
    errors = []
    entries = registry.get('entries', [])
    ids = [entry.get('id') for entry in entries]
    duplicates = [ident for ident, count in collections.Counter(ids).items() if count > 1]
    errors.extend(f'duplicate id: {ident}' for ident in duplicates)
    indexed = {entry.get('id'): entry for entry in entries}
    anchors = {heading['anchor'] for heading in headings(plan)}
    for entry in entries:
        ident = entry.get('id')
        if not isinstance(ident, str) or not ident:
            errors.append('missing id')
        elif not re.fullmatch(r'E\d+(?:\.\d+[a-z]?(?:-(?:core|gpu))?)?', ident):
            errors.append(f'invalid id: {ident}')
        if entry.get('kind') not in {'task', 'epic', 'section', 'forecast'}:
            errors.append(f'{ident}: invalid kind')
        if not entry.get('title'):
            errors.append(f'{ident}: missing title')
        if entry.get('normative_section') not in anchors:
            errors.append(f'{ident}: missing normative section {entry.get("normative_section")}')
        if entry.get('kind') in {'task', 'forecast'} and entry.get('section_scope') == 'shared_section':
            excerpt = entry.get('normative_excerpt')
            if not excerpt or excerpt not in section_text(plan, entry['normative_section']):
                errors.append(f'{ident}: missing normative evidence inside shared section')
        if entry.get('status') not in {'described', 'acceptance_ready'}:
            errors.append(f'{ident}: invalid planning status')
        if entry.get('status') == 'acceptance_ready' and not entry.get('acceptance'):
            errors.append(f'{ident}: acceptance_ready without acceptance criteria')
        if not isinstance(entry.get('acceptance'), list) or any(
                not isinstance(item, str) or not item.strip() for item in entry.get('acceptance', [])):
            errors.append(f'{ident}: invalid acceptance criteria')
        if not isinstance(entry.get('depends_on'), list):
            errors.append(f'{ident}: dependencies must be a list')
        for dep in entry.get('depends_on', []):
            if dep not in indexed:
                errors.append(f'{ident}: unknown dependency {dep}')
        for path in entry.get('writes', []):
            parts = pathlib.PurePosixPath(path)
            if parts.is_absolute() or '..' in parts.parts or '*' in path:
                errors.append(f'{ident}: invalid owned path {path}')
        if set(entry.get('writes', [])) & set(entry.get('do_not_touch', [])):
            errors.append(f'{ident}: file both writable and forbidden')
        if entry.get('writes') and not entry.get('ownership_evidence'):
            errors.append(f'{ident}: ownership without evidence')
    visiting, done = set(), set()

    def visit(ident, trail):
        if ident in visiting:
            errors.append('dependency cycle: ' + ' -> '.join(trail + [ident]))
            return
        if ident in done or ident not in indexed:
            return
        visiting.add(ident)
        for dep in indexed[ident].get('depends_on', []):
            visit(dep, trail + [ident])
        visiting.remove(ident)
        done.add(ident)

    for ident in indexed:
        visit(ident, [])
    for heading in package_headings(plan):
        match = HEADING_ID.match(heading['title'])
        if match:
            if match[1] not in indexed:
                errors.append(f'unregistered package heading: {match[1]}')
            elif indexed[match[1]]['normative_section'] != heading['anchor']:
                errors.append(f'{match[1]}: heading and registry anchor disagree')
        elif heading['anchor'] not in registry.get('unidentified_sections', {}):
            errors.append(f'unclassified heading without task id: {heading["title"]}')
    current = {h['anchor'] for h in package_headings(plan) if not HEADING_ID.match(h['title'])}
    for anchor, classification in registry.get('unidentified_sections', {}).items():
        if anchor not in current or classification.get('classification') not in {'normative', 'overview', 'evidence'}:
            errors.append(f'invalid unidentified section: {anchor}')
        if not classification.get('reason'):
            errors.append(f'unexplained section classification: {anchor}')
    # Explicit inventory includes historical packages that share a parent section.
    for ident in registry.get('historical_issue_inventory', []):
        if ident not in indexed:
            errors.append(f'missing historical issue package: {ident}')
    phase_ids = {phase['id'] for phase in registry.get('phases', [])}
    for entry in entries:
        if entry.get('phase') is not None and entry['phase'] not in phase_ids:
            errors.append(f'{entry["id"]}: unknown phase {entry["phase"]}')
    assigned_files = {}
    for ident, worker in (assignments or {}).items():
        if ident not in indexed:
            errors.append(f'assignment to unknown package: {ident}')
            continue
        if indexed[ident].get('ownership_status') != 'documented_conflict_scope':
            errors.append(f'{ident}: ownership unresolved for the documented assignment scope')
        for path in indexed[ident].get('writes', []):
            if path in assigned_files:
                errors.append(f'ownership conflict: {path}: {assigned_files[path]} and {ident} ({worker})')
            assigned_files[path] = ident
    return errors


def cell(value):
    return str(value).replace('|', '\\|').replace('\n', ' ')


def table(headers, rows):
    values = [[cell(v) for v in row] for row in [headers] + rows]
    widths = [max(3, *(len(row[i]) for row in values)) for i in range(len(headers))]
    def line(row):
        return '| ' + ' | '.join(value.ljust(width) for value, width in zip(row, widths)) + ' |'
    return '\n'.join([line(values[0]), line(['-' * w for w in widths])] + [line(row) for row in values[1:]])


def link(entry):
    return f'[{entry["id"]}](#{entry["normative_section"]})'


def rendered(registry):
    entries = registry['entries']
    packages = [e for e in entries if e['kind'] in {'task', 'forecast'}]
    rows = []
    for phase in registry['phases']:
        members = ' · '.join(e['id'] for e in packages if e.get('phase') == phase['id'])
        unresolved = phase.get('unidentified_prerequisites', [])
        if unresolved:
            members += (' · ' if members else '') + 'Kennung offen: ' + ' · '.join(unresolved)
        rows.append([phase['title'], members or 'Keine Einzelpakete zugeordnet', phase['reason']])
    unphased = [e for e in packages if e.get('phase') is None]
    if unphased:
        rows.append(['Zuordnung offen', ' · '.join(e['id'] for e in unphased), 'Keine Phase aus den Quellen ableitbar.'])
    phases = table(['Phase', 'Arbeitspakete', 'Begründung / Grenze'], rows)
    dependency_entries = [e for e in entries if e['kind'] in {'task', 'forecast'} or e['depends_on']]
    deps = table(['Arbeitspaket', 'Voraussetzungen', 'Weitere Voraussetzung / Quelle'], [
        [link(e), ' · '.join(e['depends_on']) or '—', e.get('dependency_note', '')]
        for e in dependency_entries])
    epics = table(['Epic', 'Arbeitspakete', 'Prognosen', 'Inhalt'], [
        [link(e), sum(p['kind'] == 'task' and p.get('epic') == e['id'] for p in packages),
         sum(p['kind'] == 'forecast' and p.get('epic') == e['id'] for p in packages), e['title']]
        for e in entries if e['kind'] == 'epic'])
    return dict(zip(TABLES, (phases, deps, epics)))


def update_tables(registry, plan):
    for name, body in rendered(registry).items():
        pattern = rf'(?<=<!-- sim-registry:{name}:start -->\n).*?(?=\n<!-- sim-registry:{name}:end -->)'
        plan, count = re.subn(pattern, lambda _: '\n' + body + '\n', plan, flags=re.S)
        if count != 1:
            raise ValueError(f'expected one generated table marker pair: {name}, found {count}')
    return plan


def check(registry, plan, assignments=None):
    errors = validate(registry, plan, assignments)
    try:
        if update_tables(registry, plan) != plan:
            errors.append('generated table drift: run python3 tools/sim-registry.py --write')
    except ValueError as exc:
        errors.append(str(exc))
    return errors


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--check', action='store_true')
    mode.add_argument('--write', action='store_true')
    parser.add_argument('--registry', type=pathlib.Path, default=REGISTRY)
    parser.add_argument('--plan', type=pathlib.Path, default=PLAN)
    parser.add_argument('--assignments', type=pathlib.Path, help='JSON object: package id -> worker; read-only validation')
    args = parser.parse_args(argv)
    registry = json.loads(args.registry.read_text())
    plan = args.plan.read_text()
    assignments = json.loads(args.assignments.read_text()) if args.assignments else None
    errors = validate(registry, plan, assignments)
    if args.write and not errors:
        args.plan.write_text(update_tables(registry, plan))
    elif args.check:
        errors = check(registry, plan, assignments)
    entries = registry['entries']
    print(f'Registry: {sum(e["kind"] == "task" for e in entries)} tasks, '
          f'{sum(e["kind"] == "forecast" for e in entries)} forecasts, '
          f'{sum(e["status"] == "acceptance_ready" for e in entries)} with documented acceptance.')
    raw = package_headings(plan)
    print(f'Plan: {len(raw)} headings, {sum(bool(HEADING_ID.match(h["title"])) for h in raw)} task-id headings.')
    print('Historical baseline (#895): 91 headings / 58 task-id headings / '
          '29 complete plan acceptance sections; current acceptance count uses issue criteria.')
    if assignments is not None:
        print('ASSIGNMENT SCOPE: ' + registry['ownership_scope'])
    for anchor, info in registry['unidentified_sections'].items():
        if info['classification'] == 'normative':
            print(f'NORMATIVE WITHOUT TASK ID: #{anchor} — {info["title"]}')
    for note in registry.get('unresolved', []):
        print('OPEN MAPPING: ' + note)
    for error in errors:
        print('ERROR: ' + error, file=sys.stderr)
    return bool(errors)


if __name__ == '__main__':
    sys.exit(main())
