#!/usr/bin/env python3
"""Run the three destructive guard probes in a disposable repository (#1051).

The real checkout and its Git history are never changed. AC6 compares a clean
copy to its own baseline, so unrelated PR recording changes and shallow clones
cannot masquerade as failures of the detector. The existing probes still make
their real mutations and run the real guards, including the spell-order test.
"""

import argparse
import hashlib
import os
import shutil
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PROBES = (
    'ac6-gegenprobe.sh',
    'guide-abschnittsnummern-gegenprobe.sh',
    'reihenfolge-gegenprobe.sh',
)


def fingerprint(root):
    result = {}
    for folder in ('data', 'src', 'docs', 'tools', 'tests'):
        for file in sorted((root / folder).rglob('*')):
            if file.is_file() and '__pycache__' not in file.parts:
                result[str(file.relative_to(root))] = hashlib.sha256(file.read_bytes()).hexdigest()
    return result


def run(probes=PROBES):
    if not (ROOT / 'node_modules' / '.bin' / 'vitest').exists() and 'reihenfolge-gegenprobe.sh' in probes:
        raise RuntimeError('Install the locked npm dependencies before running the probes')
    with tempfile.TemporaryDirectory(prefix='rota-gegenproben-') as temporary:
        base = Path(temporary)
        workspace = base / 'checkout'
        workspace.mkdir()
        for folder in ('data', 'src', 'tools'):
            shutil.copytree(ROOT / folder, workspace / folder, ignore=shutil.ignore_patterns('__pycache__'))
        (workspace / 'docs').mkdir()
        for file in (ROOT / 'docs').glob('*.md'):
            shutil.copy2(file, workspace / 'docs' / file.name)
        (workspace / 'tests' / 'unit').mkdir(parents=True)
        for name in ('spell-order.test.ts', 'spell-order.json'):
            shutil.copy2(ROOT / 'tests' / 'unit' / name, workspace / 'tests' / 'unit' / name)
        shutil.copytree(ROOT / 'tests' / 'parity' / 'recordings', workspace / 'tests' / 'parity' / 'recordings')
        for name in ('package.json', 'tsconfig.json'):
            shutil.copy2(ROOT / name, workspace / name)
        # Keep Vitest's writable cache inside the disposable checkout as well.
        (workspace / 'vite.config.ts').write_text(
            "import { defineConfig } from 'vitest/config';\n"
            "export default defineConfig({ cacheDir: '.cache/vite', "
            "test: { include: ['tests/unit/spell-order.test.ts'], environment: 'node' } });\n"
        )
        if (ROOT / 'node_modules').exists():
            (workspace / 'node_modules').symlink_to(ROOT / 'node_modules', target_is_directory=True)
        (base / 'tmp').mkdir()
        (base / 'window.json').write_text('{"inhaber":null}')
        env = dict(os.environ, TMPDIR=str(base / 'tmp'), PYTHONDONTWRITEBYTECODE='1',
                   ROTA_FENSTER_STAND=str(base / 'window.json'), NO_COLOR='1')
        env.pop('FORCE_COLOR', None)

        def git(*args):
            return subprocess.run(['git', '-c', 'user.name=ROTA CI', '-c', 'user.email=rota-ci@example.invalid',
                                   '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null', *args],
                                  cwd=workspace, env=env, text=True, capture_output=True, check=True).stdout.strip()

        git('init', '-q')
        git('add', 'tests/parity/recordings')
        git('commit', '-qm', 'Isolated recording baseline')
        env['AC6_BASIS'] = git('rev-parse', 'HEAD')
        git('commit', '--allow-empty', '-qm', 'Probe tip')
        expected = fingerprint(workspace)
        for probe in probes:
            print(f'Guard probe: {probe}', flush=True)
            result = subprocess.run(['bash', f'tools/{probe}'], cwd=workspace, env=env, check=False)
            actual = fingerprint(workspace)
            changed = sorted(k for k in expected.keys() | actual.keys() if expected.get(k) != actual.get(k))
            if changed:
                raise RuntimeError(f'{probe} left changed files: {", ".join(changed)}')
            if result.returncode != 0:
                raise RuntimeError(f'{probe} failed with exit {result.returncode}')
            print(f'{probe}: PASS; {len(expected)} source files byte-identical.', flush=True)
    print(f'Guard probes: {len(probes)} passed; disposable checkout removed.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--probe', choices=PROBES, action='append')
    run(parser.parse_args().probe or PROBES)
