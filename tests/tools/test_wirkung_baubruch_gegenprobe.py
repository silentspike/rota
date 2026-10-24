"""Exercise tools/wirkung-baubruch-gegenprobe.py through its real CLI (#892).

The compiler is replaced by a small fake `cargo` on PATH that emulates rustc's
exhaustiveness check (E0004) on a public fixture tree. These tests verify the
tool's decisions: copy isolation, the control and mutation steps, the exit
contract and the cargo remote v2 client exits. They are not a measurement of
rustc; the real compiler runs the same tool in the sim job.
"""

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TOOL = 'tools/wirkung-baubruch-gegenprobe.py'
JOB = '0123456789abcdef01234567'
# The variant the tool inserts; RealSimulatorSources checks it against the tool.
VARIANTE = 'WirkungBaubruchGegenprobe'

MOD_RS = '''//! Fixture der Wirkungsbeschreibung.

pub mod passes;

/// Die Bausteine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectOp {
    /// Schaden.
    DealDamage,
    /// Heilung.
    Heal,
    /// Eine Bossregel.
    CustomSignature(&'static str),
}

impl EffectOp {
    /// Der Name.
    pub fn name(self) -> &'static str {
        match self {
            EffectOp::DealDamage => "DealDamage",
            EffectOp::Heal => "Heal",
            EffectOp::CustomSignature(id) => id,
        }
    }
}

/// Ein Knoten.
pub struct EffectNode {
    /// Der Baustein.
    pub op: EffectOp,
    /// Eine Stufe.
    pub stufe: u8,
}

/// Der Name einer Analyse.
pub struct AnalysisPassId(pub &'static str);

/// Eine Analyse.
pub trait EffectAnalysisPass {
    /// Ihr Name.
    const ID: AnalysisPassId;
    /// Ihr Urteil.
    fn analyze(&self, node: &EffectNode) -> u8;
}
'''

PASSES_RS = '''//! Fixture der Analysen.

use super::{AnalysisPassId, EffectAnalysisPass, EffectNode, EffectOp};

crate::registry! {
    ErsterPass,
    ZweiterPass,
    DritterPass,
}

/// Mit verschachtelter Fallunterscheidung, die einen eigenen Auffangzweig hat.
pub struct ErsterPass;

impl EffectAnalysisPass for ErsterPass {
    const ID: AnalysisPassId = AnalysisPassId("erster");

    fn analyze(&self, node: &EffectNode) -> u8 {
        match node.op {
            EffectOp::DealDamage => match node.stufe {
                0 => 1,
                _ => 2,
            },
            EffectOp::Heal | EffectOp::CustomSignature(_) => 3,
        }
    }
}

/// Mit mehrzeiligem Oder-Muster hinter `let`.
pub struct ZweiterPass;

impl EffectAnalysisPass for ZweiterPass {
    const ID: AnalysisPassId = AnalysisPassId("zweiter");

    fn analyze(&self, node: &EffectNode) -> u8 {
        let wert = match node.op {
            EffectOp::DealDamage
            | EffectOp::Heal => 4,
            EffectOp::CustomSignature(_) => 5,
        };
        wert
    }
}

/// Schlicht.
pub struct DritterPass;

impl EffectAnalysisPass for DritterPass {
    const ID: AnalysisPassId = AnalysisPassId("dritter");

    fn analyze(&self, node: &EffectNode) -> u8 {
        match node.op {
            EffectOp::DealDamage => 6,
            EffectOp::Heal => 7,
            EffectOp::CustomSignature(_) => 8,
        }
    }
}
'''

UNREGISTERED_PASS = '''
/// Geschrieben, aber nicht registriert.
pub struct VierterPass;

impl EffectAnalysisPass for VierterPass {
    const ID: AnalysisPassId = AnalysisPassId("vierter");

    fn analyze(&self, node: &EffectNode) -> u8 {
        match node.op {
            EffectOp::DealDamage => 9,
            EffectOp::Heal => 10,
            EffectOp::CustomSignature(_) => 11,
        }
    }
}
'''

# A deliberately small stand-in for rustc's exhaustiveness check on the
# rustfmt-shaped fixture above: a `match` whose top-level arms name EffectOp and
# miss a variant without `_ =>` yields one E0004 at the match line. Switches
# select the failure modes of compiler and client per invocation ("n:value").
FAKE_CARGO = r'''#!{python}
import json, os, re, sys, time
from pathlib import Path

ARGS = sys.argv[1:]
REMOTE = ARGS[:3] == ['remote', '--no-copy-lock', '--']
protokoll = Path(os.environ['FAKE_PROTOKOLL'])
with protokoll.open('a') as stream:
    stream.write(json.dumps({'argv': ARGS, 'cwd': os.getcwd(),
                             'target_dir': os.environ.get('CARGO_TARGET_DIR'),
                             'ceiling': os.environ.get('GIT_CEILING_DIRECTORIES')}) + '\n')
nummer = len(protokoll.read_text().splitlines())


def schalter(name):
    for teil in filter(None, os.environ.get(name, '').split(',')):
        n, _, wert = teil.partition(':')
        if int(n) == nummer:
            return wert or '1'
    return None


if (ARGS[3:] if REMOTE else ARGS) != ['check', '--lib']:
    print('fake cargo: unexpected arguments ' + ' '.join(ARGS), file=sys.stderr)
    sys.exit(1)
if schalter('FAKE_AUSGANG'):
    code = int(schalter('FAKE_AUSGANG'))
    if REMOTE:
        # 75 and 2 are refused before a job exists; the others end a reserved job.
        if code not in (75, 2):
            print(f'cargo-remote: job {JOB} | linux-gnu-bookworm | Rust 1.98.1 | normal | workspace 00ff')
        print('cargo-remote: simulated client failure', file=sys.stderr)
    else:
        # cargo itself crashing, e.g. a panic: an exit code without a single diagnostic.
        print("thread 'main' panicked at src/bin/cargo/main.rs:1:1: simulated", file=sys.stderr)
    sys.exit(code)
if schalter('FAKE_ECHT_AENDERN'):
    with open(os.environ['FAKE_ECHTE_DATEI'], 'a') as stream:
        stream.write('changed during the run\n')
if schalter('FAKE_SCHLAF'):
    time.sleep(float(schalter('FAKE_SCHLAF')))

texte = {p.as_posix(): p.read_text() for p in sorted(Path('sim/src').rglob('*.rs'))}


def tiefe(zeile):
    return len(zeile) - len(zeile.lstrip(' '))


varianten = []
for text in texte.values():
    zeilen = text.splitlines()
    for i, zeile in enumerate(zeilen):
        if zeile.strip() == 'pub enum EffectOp {':
            for weitere in zeilen[i + 1:]:
                code = weitere.strip()
                if code.startswith('}'):
                    break
                if code and not code.startswith(('//', '#')):
                    varianten.append(re.match(r'\w+', code).group(0))

stellen, alle = [], []
for datei, text in texte.items():
    zeilen = text.splitlines()
    for i, zeile in enumerate(zeilen):
        if not re.search(r'\bmatch\b.*\{$', zeile.rstrip()):
            continue
        arme = []
        for weitere in zeilen[i + 1:]:
            if weitere.strip().startswith('}') and tiefe(weitere) == tiefe(zeile):
                break
            if tiefe(weitere) == tiefe(zeile) + 4:
                arme.append(weitere.strip())
        if not any('EffectOp::' in arm for arm in arme):
            continue
        ort = (datei, i + 1, zeile.index('match') + 7)
        alle.append(ort)
        gedeckt = set(re.findall(r'EffectOp::(\w+)', ' '.join(arme)))
        auffang = any(arm.startswith(('_ =>', '_ if')) for arm in arme)
        offen = [v for v in varianten if v not in gedeckt]
        if offen and not (auffang and os.environ.get('FAKE_BLIND') != '1'):
            stellen.append(ort + (offen[0],))

if os.environ.get('FAKE_IMMER') == 'gruen' or schalter('FAKE_GRUEN'):
    stellen = []
elif os.environ.get('FAKE_IMMER') == 'rot' and not stellen:
    stellen = [alle[0] + ('DealDamage',)]
if stellen and os.environ.get('FAKE_NICHTLOKAL') == '1':
    stellen = [ort + (stellen[0][3],) for ort in alle]


def nach_marke(marke):
    """The first EffectOp match after the first line containing `marke` — or a loud failure."""
    for datei, text in texte.items():
        for nummer, zeile in enumerate(text.splitlines(), 1):
            if marke in zeile:
                ort = next((o for o in alle if o[0] == datei and o[1] > nummer), None)
                if ort is not None:
                    return ort
    print(f'fake cargo: no EffectOp match after {marke!r}', file=sys.stderr)
    sys.exit(3)


# One extra site of the probe variant, or one site fewer, at exactly one
# invocation: each isolates a single sub-condition of the tool's verdicts.
if schalter('FAKE_ZUSATZ'):
    stellen.append(nach_marke(schalter('FAKE_ZUSATZ')) + ('{VARIANTE}',))
if schalter('FAKE_OHNE'):
    weg = nach_marke(schalter('FAKE_OHNE'))
    if weg not in [s[:3] for s in stellen]:
        print(f'fake cargo: nothing to remove at {weg}', file=sys.stderr)
        sys.exit(3)
    stellen = [s for s in stellen if s[:3] != weg]

ausgabe = []
if REMOTE and not schalter('FAKE_OHNE_KOPF'):
    ausgabe.append(f'cargo-remote: job {JOB} | linux-gnu-bookworm | Rust 1.98.1 | normal | workspace 00ff')
ausgabe.append(f'    Checking fixture v0.1.0 ({os.getcwd()}/sim)')
fehler = 0
for datei, zeile, spalte, variante in stellen:
    fehler += 1
    ausgabe += [f'error[E0004]: non-exhaustive patterns: `EffectOp::{variante}` not covered',
                f'   --> {datei}:{zeile}:{spalte}', '    |',
                'note: `EffectOp` defined here', '   --> sim/src/effects/mod.rs:7:10']
if schalter('FAKE_FREMD'):
    fehler += 1
    ausgabe += ['error[E0308]: mismatched types', '   --> sim/src/lib.rs:1:1']
if schalter('FAKE_FREMDE_VARIANTE'):
    fehler += 1
    ausgabe += ['error[E0004]: non-exhaustive patterns: `Trigger::Neu` not covered',
                '   --> sim/src/effects/passes.rs:18:15']
code = 101 if fehler else 0
if schalter('FAKE_ERFOLG_TROTZ_FEHLER'):
    # A contradictory result: diagnostics, "Finished" and exit 0 at once.
    code = 0
if fehler and code:
    ausgabe.append(f'error: could not compile `fixture` (lib) due to {fehler} previous errors')
elif not code and not schalter('FAKE_OHNE_FINISHED'):
    ausgabe.append('    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.01s')
if REMOTE and not schalter('FAKE_OHNE_ABSCHLUSS'):
    gemeldet = 0 if schalter('FAKE_ABSCHLUSS_FALSCH') else code
    ausgabe.append(f'cargo-remote: {"failed" if gemeldet else "succeeded"} | exit {gemeldet} | 0.05s')
print('\n'.join(ausgabe))
sys.exit(code)
'''


class WirkungBaubruchGegenprobe(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='rota-baubruch-test-')
        self.addCleanup(temporary.cleanup)
        base = Path(temporary.name)
        self.root = base / 'repo'
        self.tmp = base / 'tmp'
        self.bin = base / 'bin'
        self.log = base / 'aufrufe.jsonl'
        for folder in (self.root / 'tools', self.tmp, self.bin):
            folder.mkdir(parents=True)
        shutil.copy2(ROOT / TOOL, self.root / TOOL)
        files = {
            'Cargo.toml': '[workspace]\nmembers = ["sim"]\nresolver = "2"\n',
            'Cargo.lock': '# fixture\nversion = 4\n',
            'rust-toolchain.toml': '[toolchain]\nchannel = "1.98.1"\n',
            '.cargo-remote.toml': '[cargo_remote]\nschema = 1\n',
            'data/spells/x.json': '{}\n',
            'tests/parity/recordings/r.json': '{}\n',
            'sim/Cargo.toml': '[package]\nname = "fixture"\nversion = "0.1.0"\nedition = "2021"\n',
            'sim/src/lib.rs': '//! Fixture.\npub mod effects;\n',
            'sim/src/effects/mod.rs': MOD_RS,
            'sim/src/effects/passes.rs': PASSES_RS,
        }
        for name, text in files.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        fake = self.bin / 'cargo'
        fake.write_text(FAKE_CARGO.replace('{python}', sys.executable).replace('{JOB}', JOB)
                        .replace('{VARIANTE}', VARIANTE))
        fake.chmod(0o755)

    def snapshot(self):
        return {str(p.relative_to(self.root)): p.read_bytes()
                for p in self.root.rglob('*') if p.is_file() and '__pycache__' not in p.parts}

    def edit(self, name, old, new):
        path = self.root / name
        text = path.read_text()
        self.assertEqual(text.count(old), 1, f'fixture anchor {old!r} is not unique in {name}')
        path.write_text(text.replace(old, new))

    def run_tool(self, *args, real_tree_may_change=False, **switches):
        before = self.snapshot()
        env = {k: v for k, v in os.environ.items() if not k.startswith('FAKE_')}
        env.update(PATH=f'{self.bin}{os.pathsep}{os.environ.get("PATH", "")}', TMPDIR=str(self.tmp),
                   FAKE_PROTOKOLL=str(self.log), FAKE_ECHTE_DATEI=str(self.root / 'data/spells/x.json'),
                   PYTHONDONTWRITEBYTECODE='1')
        env.update({f'FAKE_{key.upper()}': value for key, value in switches.items()})
        result = subprocess.run([sys.executable, '-B', str(self.root / TOOL), *args], env=env,
                                text=True, capture_output=True, check=False, timeout=120)
        if not real_tree_may_change:
            self.assertEqual(before, self.snapshot(), 'The real input checkout was mutated')
        self.assertEqual(list(self.tmp.iterdir()), [], 'The disposable copy was left behind')
        return result

    def calls(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()] if self.log.exists() else []

    @staticmethod
    def findings(result):
        return [line for line in result.stdout.splitlines() if line.startswith('BEFUND:')]

    def assert_inside_copy(self, call):
        cwd = os.path.realpath(call['cwd'])
        self.assertNotEqual(cwd, os.path.realpath(self.root))
        self.assertTrue(cwd.startswith(os.path.realpath(self.tmp) + os.sep), cwd)

    def test_every_registered_pass_breaks_the_real_build_alone(self):
        result = self.run_tool('--befehl', 'cargo')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Ergebnis: BELEGT', result.stdout)
        self.assertEqual(self.findings(result), [])
        for name in ('ErsterPass', 'ZweiterPass', 'DritterPass'):
            self.assertIn(f'nur {name} offen', result.stdout)
        # The E0004 in EffectOp::name is reported, but it is not an analysis.
        self.assertIn('außerhalb der Analysen: sim/src/effects/mod.rs:', result.stdout)
        self.assertIn('Gegenprobe der Gegenprobe, Auffangzweig in ErsterPass', result.stdout)
        calls = self.calls()
        # K0, M0, K1, one mutation per registered pass, counter-check.
        self.assertEqual(len(calls), 3 + 3 + 1)
        for call in calls:
            self.assertEqual(call['argv'], ['check', '--lib'])
            self.assert_inside_copy(call)
            self.assertTrue(os.path.realpath(call['target_dir']).startswith(
                os.path.realpath(call['cwd']) + os.sep), call)

    def test_remote_mode_runs_the_v2_client_from_the_copy_root(self):
        result = self.run_tool('--befehl', 'remote')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('Ergebnis: BELEGT', result.stdout)
        self.assertIn(JOB, result.stdout)
        calls = self.calls()
        self.assertEqual(len(calls), 7)
        for call in calls:
            self.assertEqual(call['argv'], ['remote', '--no-copy-lock', '--', 'check', '--lib'])
            self.assert_inside_copy(call)
            # The client must not discover a surrounding Git work tree.
            self.assertEqual(os.path.realpath(call['ceiling']),
                             os.path.realpath(Path(call['cwd']).parent))

    def test_protocol_keeps_every_build_log(self):
        logs = Path(self.tmp.parent) / 'protokoll'
        result = self.run_tool('--befehl', 'cargo', '--protokoll', str(logs))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        files = sorted(logs.iterdir())
        self.assertEqual(len(files), 7)
        broken = (logs / '02-m0.log').read_text()
        self.assertIn('$ cargo check --lib', broken)
        self.assertIn('# exit 101', broken)
        self.assertIn('error[E0004]', broken)

    def test_a_catch_all_arm_in_a_registered_pass_is_a_finding(self):
        self.edit('sim/src/effects/passes.rs', 'EffectOp::CustomSignature(_) => 8,', '_ => 8,')
        result = self.run_tool('--befehl', 'cargo')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('Ergebnis: BEFUND', result.stdout)
        findings = self.findings(result)
        # Both steps that can see the gap name it: the missing E0004 in M0 and
        # the per-pass mutation that has nothing left to open.
        self.assertEqual(len(findings), 2, findings)
        self.assertTrue(findings[0].startswith('BEFUND: M0: DritterPass bricht den Bau NICHT'), findings)
        self.assertTrue(findings[1].startswith('BEFUND: Mutation je Pass: DritterPass'), findings)
        self.assertFalse(any('ErsterPass' in line or 'ZweiterPass' in line for line in findings), findings)

    def test_a_mutation_that_never_breaks_the_build_is_a_finding(self):
        result = self.run_tool('--befehl', 'cargo', immer='gruen')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn('Ergebnis: BEFUND', result.stdout)
        # The specific verdict, not the fallback "no E0004 to attribute": both end
        # in exit 1, only one says what happened.
        self.assertEqual(len(self.findings(result)), 1, result.stdout)
        self.assertTrue(self.findings(result)[0].startswith('BEFUND: M0: der Bau bricht NICHT'),
                        result.stdout)
        # Nothing broke, so there is nothing to attribute or to classify.
        self.assertEqual(len(self.calls()), 2)

    def test_an_undamaged_copy_that_does_not_build_is_a_finding(self):
        result = self.run_tool('--befehl', 'cargo', immer='rot')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertTrue(any('K0' in line for line in self.findings(result)), result.stdout)
        self.assertEqual(len(self.calls()), 1)

    def test_client_exits_that_mean_nothing_ran_end_with_exit_2(self):
        # The reason names the client's meaning of the exit, not merely its number:
        # a missing header or final line would also end in exit 2, for another reason.
        reasons = {75: 'nichts eingereiht', 70: 'Verbindung', 74: 'Artefakte', 130: 'abgebrochen',
                   2: 'abgelehnt'}
        for code, reason in reasons.items():
            with self.subTest(code=code):
                self.log.unlink(missing_ok=True)
                result = self.run_tool('--befehl', 'remote', ausgang=f'2:{code}')
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn(f'NICHT GELAUFEN: Exit {code} — ', result.stdout)
                self.assertIn(reason, result.stdout)
                self.assertIn('Ergebnis: NICHT GELAUFEN', result.stdout)
                self.assertEqual(self.findings(result), [])
                self.assertEqual(len(self.calls()), 2)

    def test_the_same_exits_from_plain_cargo_are_not_a_verdict_either(self):
        for code in (75, 2):
            with self.subTest(code=code):
                self.log.unlink(missing_ok=True)
                result = self.run_tool('--befehl', 'cargo', ausgang=f'1:{code}')
                self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
                self.assertIn(f'unerwarteter Exit {code}', result.stdout)

    def test_a_missing_final_client_line_is_not_a_build(self):
        result = self.run_tool('--befehl', 'remote', ohne_abschluss='1')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Abschlusszeile', result.stdout)
        self.assertEqual(len(self.calls()), 1)

    def test_a_final_line_that_contradicts_the_exit_is_not_a_build(self):
        result = self.run_tool('--befehl', 'remote', abschluss_falsch='2')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Abschlusszeile', result.stdout)

    def test_a_result_without_the_client_header_is_not_a_build(self):
        result = self.run_tool('--befehl', 'remote', ohne_kopf='1')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Kopfzeile', result.stdout)

    def test_an_e0004_for_another_type_is_not_the_probe(self):
        result = self.run_tool('--befehl', 'cargo', fremde_variante='2')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertEqual(len(findings), 1, findings)
        self.assertIn('Trigger::Neu', findings[0])

    def test_exit_0_without_finished_is_not_a_build(self):
        result = self.run_tool('--befehl', 'cargo', ohne_finished='1')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Finished', result.stdout)

    def test_counter_check_fails_when_a_catch_all_does_not_remove_the_break(self):
        result = self.run_tool('--befehl', 'cargo', blind='1')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertEqual(len(findings), 1, findings)
        # The catch-all pass itself is named as still broken, not merely a
        # mismatch in the list of the others.
        self.assertIn('Gegenprobe der Gegenprobe: mit Auffangzweig in ErsterPass meldet die '
                      'Auswertung ihn weiter als gebrochen', findings[0])

    def test_diagnostics_outside_the_open_pass_fail_the_per_pass_mutation(self):
        result = self.run_tool('--befehl', 'cargo', nichtlokal='1')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertTrue(any('Mutation je Pass' in line for line in self.findings(result)), result.stdout)

    def test_a_foreign_compiler_error_in_the_mutation_is_a_finding(self):
        result = self.run_tool('--befehl', 'cargo', fremd='2')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertTrue(any('E0308' in line for line in self.findings(result)), result.stdout)

    def test_a_control_that_does_not_build_after_classification_is_a_finding(self):
        # Invocation 3 is K1: the variant classified everywhere M0 reported it.
        result = self.run_tool('--befehl', 'cargo', fremd='3')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertEqual(len(findings), 1, findings)
        self.assertTrue(findings[0].startswith('BEFUND: K1:'), findings)

    def test_an_open_pass_that_does_not_break_alone_is_a_finding(self):
        # Invocation 4 is the first per-pass mutation: only ErsterPass is open.
        result = self.run_tool('--befehl', 'cargo', gruen='4')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertEqual(len(findings), 1, findings)
        self.assertIn('nur ErsterPass offen, und der Bau bricht NICHT', findings[0])

    # One case per sub-condition. A case that trips two conditions at once (as
    # `nichtlokal` does for the per-pass mutation) stays green when either one
    # is deleted: in the review of PR #1136, 8 of 30 single-condition mutations
    # survived the earlier cases (measured there, and again before this change).
    # Each switch below acts at exactly one invocation:
    # 1 K0, 2 M0, 3 K1, 4-6 the per-pass mutations (4 = only ErsterPass open), 7 G.

    def test_a_probe_site_in_another_pass_fails_the_per_pass_mutation(self):
        result = self.run_tool('--befehl', 'cargo', zusatz='4:for DritterPass')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.findings(result), [
            'BEFUND: Mutation je Pass: nur ErsterPass offen, aber der Bruch steht nicht nur dort — '
            "Pässe ['DritterPass'], außerhalb —, fremd —"])
        self.assertEqual(len(self.calls()), 7)

    def test_a_probe_site_outside_the_passes_fails_the_per_pass_mutation(self):
        result = self.run_tool('--befehl', 'cargo', zusatz='4:impl EffectOp {')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertEqual(len(findings), 1, findings)
        self.assertRegex(findings[0], r'^BEFUND: Mutation je Pass: nur ErsterPass offen, aber der Bruch '
                                      r'steht nicht nur dort — Pässe \[\], außerhalb '
                                      r'sim/src/effects/mod\.rs:\d+, fremd —$')

    def test_a_foreign_error_in_the_per_pass_mutation_is_a_finding(self):
        result = self.run_tool('--befehl', 'cargo', fremd='4')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.findings(result), [
            'BEFUND: Mutation je Pass: nur ErsterPass offen, aber der Bruch steht nicht nur dort — '
            "Pässe [], außerhalb —, fremd ['error[E0308]: mismatched types']"])

    def test_counter_check_fails_when_another_pass_stops_breaking(self):
        result = self.run_tool('--befehl', 'cargo', ohne='7:for ZweiterPass')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.findings(result), [
            'BEFUND: Gegenprobe der Gegenprobe: Auffangzweig in ErsterPass, erwartet brechen '
            "['DritterPass', 'ZweiterPass'], gemeldet ['DritterPass'], fremd —"])

    def test_counter_check_fails_on_a_foreign_error(self):
        result = self.run_tool('--befehl', 'cargo', fremd='7')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.findings(result), [
            'BEFUND: Gegenprobe der Gegenprobe: Auffangzweig in ErsterPass, erwartet brechen '
            "['DritterPass', 'ZweiterPass'], gemeldet ['DritterPass', 'ZweiterPass'], "
            "fremd ['error[E0308]: mismatched types']"])

    def test_exit_0_with_compiler_errors_is_not_a_build(self):
        result = self.run_tool('--befehl', 'cargo', fremd='1', erfolg_trotz_fehler='1')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('NICHT GELAUFEN: Exit 0, aber mit Fehlermeldungen des Übersetzers', result.stdout)
        self.assertEqual(self.findings(result), [])
        self.assertEqual(len(self.calls()), 1)

    def test_exit_101_without_any_diagnostic_is_not_a_finding(self):
        # A crash of cargo itself: exit 101, but nothing the compiler said. Taken
        # for a broken build, K0 would turn it into a finding ("does not build").
        result = self.run_tool('--befehl', 'cargo', ausgang='1:101')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('NICHT GELAUFEN: Exit 101 ohne eine einzige Fehlermeldung des Übersetzers',
                      result.stdout)
        self.assertEqual(self.findings(result), [])
        self.assertEqual(len(self.calls()), 1)

    def test_m0_breaking_only_for_a_foreign_reason_names_the_missing_sites(self):
        # M0 breaks, but no E0004 names the probe: nothing to attribute, so no
        # pass may be blamed and nothing further may be built.
        result = self.run_tool('--befehl', 'cargo', gruen='2', fremd='2')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertEqual(self.findings(result), [
            'BEFUND: M0: der Bau bricht auch aus fremdem Grund: error[E0308]: mismatched types',
            f'BEFUND: M0: keine einzige E0004-Stelle nennt {VARIANTE} — nichts zuzuordnen'])
        self.assertEqual(len(self.calls()), 2)

    def test_a_change_to_the_real_tree_during_the_run_is_a_finding(self):
        result = self.run_tool('--befehl', 'cargo', real_tree_may_change=True, echt_aendern='1')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertEqual(len(findings), 1, findings)
        self.assertIn('echte Arbeitsbaum', findings[0])
        self.assertIn('data/spells/x.json', findings[0])

    def test_a_registered_pass_without_impl_is_a_finding(self):
        self.edit('sim/src/effects/passes.rs', '    DritterPass,\n}', '    DritterPass,\n    FuenfterPass,\n}')
        result = self.run_tool('--befehl', 'cargo')
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        findings = self.findings(result)
        self.assertTrue(findings)
        self.assertTrue(all('FuenfterPass' in line for line in findings), findings)

    def test_an_unregistered_pass_is_named_but_not_required(self):
        path = self.root / 'sim/src/effects/passes.rs'
        path.write_text(path.read_text() + UNREGISTERED_PASS)
        result = self.run_tool('--befehl', 'cargo')
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('VierterPass (nicht registriert)', result.stdout)

    def test_without_a_registry_nothing_can_be_proven(self):
        self.edit('sim/src/effects/passes.rs', 'crate::registry! {', 'crate::liste! {')
        result = self.run_tool('--befehl', 'cargo')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Ergebnis: NICHT GELAUFEN', result.stdout)
        self.assertEqual(self.calls(), [])

    def test_a_build_that_exceeds_the_timeout_is_not_run(self):
        result = self.run_tool('--befehl', 'cargo', '--timeout', '1', schlaf='1:5')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('Zeitlimit', result.stdout)

    def test_the_command_must_be_chosen_explicitly(self):
        result = self.run_tool()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertEqual(self.calls(), [])

    def test_remote_mode_requires_the_v2_configuration_in_the_copy(self):
        (self.root / '.cargo-remote.toml').unlink()
        result = self.run_tool('--befehl', 'remote')
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn('.cargo-remote.toml', result.stdout)
        self.assertEqual(self.calls(), [])


class RealSimulatorSources(unittest.TestCase):
    """The parser must understand the checked-in simulator, not only the fixture."""

    def setUp(self):
        spec = importlib.util.spec_from_file_location('wirkung_baubruch', ROOT / TOOL)
        self.tool = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.tool)
        self.texts = self.tool.lies_rust(ROOT)

    def test_every_registered_pass_has_an_impl_block(self):
        _, _, registered = self.tool.lies_registry(self.texts)
        passes = self.tool.finde_paesse(self.texts)
        self.assertTrue(registered, 'empty registry: the probe would prove nothing')
        self.assertEqual([name for name in registered if name not in passes], [])
        for name in registered:
            found = passes[name]
            self.assertLess(found.anfang, found.ende, name)
            self.assertIsNotNone(found.kennung, f'{name} has no readable ID')

    def test_the_probe_variant_is_inserted_exactly_once_into_the_real_enum(self):
        self.assertEqual(self.tool.VARIANTE, VARIANTE, 'the fake would report a different variant')
        enum = self.tool.finde_enum(self.texts)
        mutated = self.tool.mit_baustein(self.texts, enum)
        changed = [name for name in self.texts if mutated[name] != self.texts[name]]
        self.assertEqual(changed, [enum[0]])
        self.assertEqual(mutated[enum[0]].count(f'    {self.tool.VARIANTE},\n'), 1)
        self.assertNotIn(self.tool.VARIANTE, self.texts[enum[0]])


if __name__ == '__main__':
    unittest.main()
