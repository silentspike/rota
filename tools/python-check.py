#!/usr/bin/env python3
"""Compile every Python tool and tool test without importing or executing it.

Bytecode goes to a temporary directory. Asset writers and paid-service clients
therefore receive a syntax check without their side effects or credentials.
"""

import argparse
import py_compile
import tempfile
import unittest
from pathlib import Path


def check(root):
    files = sorted((root / 'tools').rglob('*.py'))
    files += sorted((root / 'tests' / 'tools').rglob('*.py'))
    if not files:
        raise ValueError('No Python tools found')
    with tempfile.TemporaryDirectory(prefix='rota-python-') as cache:
        for index, source in enumerate(files):
            py_compile.compile(
                str(source), cfile=str(Path(cache) / f'{index}.pyc'), doraise=True,
            )
    print(f'Python syntax: {len(files)} files passed; source tree unchanged.')
    return len(files)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--tests', action='store_true', help='also run tests/tools; reject an empty suite')
    args = parser.parse_args()
    check(args.root)
    if args.tests:
        suite = unittest.defaultTestLoader.discover(str(args.root / 'tests' / 'tools'), pattern='test_*.py')
        if suite.countTestCases() == 0:
            raise SystemExit('No tool tests discovered')
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        raise SystemExit(0 if result.wasSuccessful() else 1)
