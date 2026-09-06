#!/usr/bin/env python3
"""Inventory spell-time fields against the two production grid contracts (#1104).

This is a data inventory, not a substitute for executing the TS/Rust validators.
Every nested *Ms occurrence is counted, including fields introduced after this tool.
"""

import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SAFE_INTEGER = 9_007_199_254_740_991


def policy(root):
    ts = (root / 'src/core/data.ts').read_text()
    rust = (root / 'sim/time_fields.rs').read_text()
    ts_tick = re.search(r'\bTICK_MS\s*(?::\s*number\s*)?=\s*(\d+)',
                        (root / 'src/core/loop.ts').read_text())
    rust_tick = re.search(r'\bconst\s+RASTER_MS\s*:\s*u64\s*=\s*(\d+)',
                          (root / 'sim/build.rs').read_text())
    if not ts_tick or not rust_tick:
        raise ValueError('Cannot read both production grid constants')
    tick = int(ts_tick[1])
    if tick <= 0 or tick != int(rust_tick[1]):
        raise ValueError('Production grid constants disagree or are zero')
    ts_exceptions = set(re.findall(r"key\s*!==\s*'([^']+Ms)'", ts))
    rust_exceptions = set(re.findall(r'key\s*!=\s*"([^"]+Ms)"', rust))
    if ts_exceptions != rust_exceptions or ts_exceptions != {'cooldownReductionBelowMs'}:
        raise ValueError('Production threshold exceptions disagree or need explicit review')
    return tick, ts_exceptions


def occurrences(value, path=''):
    if isinstance(value, list):
        for index, child in enumerate(value):
            yield from occurrences(child, f'{path}[{index}]')
    elif isinstance(value, dict):
        for key, child in value.items():
            location = f'{path}.{key}' if path else key
            if key.endswith('Ms'):
                yield key, location, child
            yield from occurrences(child, location)


def inventory(root):
    tick, exceptions = policy(root)
    files = sorted((root / 'data/spells').glob('*.json'))
    if not files:
        raise ValueError('No spell JSON files found; zero files is not a passing audit')
    result = {
        'tick_ms': tick, 'files': len(files), 'occurrences': 0,
        'grid_occurrences': 0, 'threshold_occurrences': 0,
        'grid_fields': [], 'threshold_fields': [], 'errors': [],
    }
    grid_fields, threshold_fields = set(), set()
    for file in files:
        for key, path, value in occurrences(json.loads(file.read_text())):
            result['occurrences'] += 1
            threshold = key in exceptions
            result['threshold_occurrences' if threshold else 'grid_occurrences'] += 1
            (threshold_fields if threshold else grid_fields).add(key)
            location = f'{file.relative_to(root)}:{path}'
            if type(value) is not int or not 0 <= value <= SAFE_INTEGER:
                result['errors'].append(f'{location}: expected a nonnegative safe integer')
            elif not threshold and value % tick:
                result['errors'].append(f'{location}: {value} is not on the {tick}-ms grid')
    if result['occurrences'] == 0:
        raise ValueError('No *Ms fields found; zero fields is not a passing audit')
    result['grid_fields'] = sorted(grid_fields)
    result['threshold_fields'] = sorted(threshold_fields)
    result['status'] = 'FAIL' if result['errors'] else 'PASS'
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=ROOT)
    parser.add_argument('--json', action='store_true', help='print the complete machine-readable inventory')
    args = parser.parse_args()
    try:
        report = inventory(args.root)
    except (OSError, ValueError) as error:
        parser.exit(1, f'FAIL: {error}\n')
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print(f"{report['status']}: {report['files']} files, {report['occurrences']} time occurrences; "
              f"{len(report['grid_fields'])} grid field names / {report['grid_occurrences']} occurrences; "
              f"{len(report['threshold_fields'])} threshold exceptions / {report['threshold_occurrences']} occurrences")
        print('GRID: ' + ', '.join(report['grid_fields']))
        print('THRESHOLD: ' + ', '.join(report['threshold_fields']))
        for error in report['errors']:
            print('INVALID: ' + error)
    return 1 if report['errors'] else 0


if __name__ == '__main__':
    raise SystemExit(main())
