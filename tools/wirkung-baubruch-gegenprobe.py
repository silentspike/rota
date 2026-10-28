#!/usr/bin/env python3
"""Baubruch-Gegenprobe der Wirkungsbeschreibung (Epic #892).

Belegt mit dem echten Übersetzer, dass ein neuer Effekt-Baustein den Bau an
JEDER registrierten Analyse bricht, bis er dort eingeordnet ist. Gebaut wird
eine Wegwerfkopie der Rust-Eingaben; der echte Arbeitsbaum wird nie verändert
und nach dem Lauf per SHA-256 gegengelesen (Muster von #1051,
tools/gegenproben-ci.py).

Die Schritte, jeder ein eigener Bau der Kopie:

  K0  Kontrolle: die unbeschädigte Kopie baut. Ohne sie könnte ein roter Bau
      unten auch am Bestand liegen statt an der Mutation.
  M0  Gegenprobe: ein neuer Baustein ohne Einordnung. Der Bau muss brechen,
      und zwar mit einer E0004-Stelle in JEDEM registrierten Pass.
  K1  Kontrolle: derselbe Baustein, an jeder gemeldeten Stelle eingeordnet,
      baut wieder. Erst das zeigt, dass M0 an der fehlenden Einordnung brach
      und nicht an der eingefügten Variante selbst.
  P   Mutation je Pass: überall eingeordnet außer in genau einem Pass. Der Bau
      bricht, und zwar nur dort.
  G   Gegenprobe der Gegenprobe: ein Auffangzweig `_ =>` im ersten gebrochenen
      Pass. Er muss aus dem Bruch verschwinden, und die Auswertung muss das
      erkennen. Ohne diesen Schritt wäre nicht belegt, dass sie einen Pass, der
      NICHT bricht, überhaupt bemerken kann.

Dasselbe gilt für die übrigen semantischen Typen des Knotens. Welche das sind,
liest das Werkzeug aus `pub struct EffectNode {`: jede Aufzählung des
Effekt-Moduls, die ein Feld trägt — direkt oder über eine Struktur desselben
Moduls. Je Typ, jeder ein eigener Bau:

  T0  ein neuer Wert ohne Einordnung: der Bau bricht, an mindestens einer Stelle.
  T1  derselbe Wert an jeder gemeldeten Stelle eingeordnet: die Kopie baut wieder.

Was der Übersetzer nicht sehen kann, ist eine Stelle, die einen Typ NICHT
erschöpfend auswertet — sie übersetzt mit jedem neuen Wert. Deshalb sucht

  S   in sim/src ohne Prüfmodule (`#[cfg(test)]`) nach Auffangformen über
      EffectOp und jeden dieser Typen: `matches!(…)`, ein Arm `_` oder eine
      bloße Bindung neben einem Arm, der den Typ nennt, `==`/`!=` neben einem
      Wert des Typs, `if let`/`while let`/`let … else` mit einem solchen Muster.
      Jede Fundstelle ist ein Befund. Gelesen wird über Zeichen, nicht über
      Zeilen: Ein umbrochener Ausdruck ist für ein Zeilenmuster unsichtbar.

  Sie hat ihre eigene Positivkontrolle: Mit dem Auffangzweig aus G und je Typ
  mit einem Auffangzweig an der ersten Stelle aus T0 muss sie genau eine
  Fundstelle mehr melden, in der Datei der Stelle. Sonst wäre sie blind.

Was S nicht sieht: einen Wert, der über einen Umweg verglichen wird
(`[Trigger::OnCast].contains(&t)`, `use Trigger::*` mit bloßen Namen), und einen
Platzhalter in einem zusammengesetzten Muster (`(Trigger::OnCast, _)`,
`Some(andere)`).

Die Zuordnung einer Stelle zu einem Pass liest die impl-Blöcke
`impl EffectAnalysisPass for X` in sim/src; die Registry ist der Aufruf von
`registry!`. Beides setzt rustfmt-Form voraus, die der sim-Job erzwingt.

Ausgänge: 0 belegt, 1 Befund, 2 nicht gelaufen.

  --befehl cargo    cargo check --lib (CI: sim-Job)
  --befehl remote   cargo remote --no-copy-lock -- check --lib aus der Wurzel
                    der Kopie (lokal, wo Rust nur der Bau-Dienst baut)
"""

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

VARIANTE = 'WirkungBaubruchGegenprobe'
QUELLEN = 'sim/src'
# Was sim/build.rs und die Bibliothek lesen: data/ und die Vergleichsdateien
# (RECORDINGS_DIR in build.rs), dazu Workspace, Sperrdatei und Compiler-Pin.
# .cargo-remote.toml braucht nur der v2-Client; ohne sie reicht er an den alten
# Client weiter (gemessen am 2026-10-23, siehe .cargo-remote.toml).
KOPIE_ORDNER = ('sim', 'data', 'tests/parity/recordings')
KOPIE_DATEIEN = ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo-remote.toml')
AUSGELASSEN = ('target', '__pycache__', '.git')

CARGO_AUFTRAG = ('check', '--lib')
BEFEHLE = {
    'cargo': ('cargo',),
    'remote': ('cargo', 'remote', '--no-copy-lock', '--'),
}
# Ausgänge des cargo-remote-v2-CLIENTS, nicht von cargo: Gebaut wurde nichts,
# oder das Ergebnis kam nicht an. cargo selbst endet bei Bau- und Testfehlern
# mit 101. Dieselbe Tafel steht in tools/streich-mutation.py (#1132).
DIENST_AUSGAENGE = {
    75: 'kein Platz auf dem Bau-Dienst oder Arbeitsbereich belegt — nichts eingereiht',
    70: 'Verbindung oder Ausführer des Bau-Dienstes verloren',
    74: 'Artefakte nicht übertragbar',
    130: 'abgebrochen',
    2: 'Aufruf oder Projektkonfiguration vom Client abgelehnt',
}

ENUM_KOPF = re.compile(r'^pub enum EffectOp \{\s*$')
AUFZAEHLUNG_KOPF = re.compile(r'^pub enum (\w+) \{\s*$')
STRUKTUR_KOPF = re.compile(r'^pub struct (\w+) \{\s*$')
FELD = re.compile(r'^\s*pub \w+: (.+?),\s*(?://.*)?$')
REGISTRY_KOPF = re.compile(r'^\s*(?:\$?crate::|::)?(?:\w+::)*registry!\s*\{\s*$')
IMPL_KOPF = re.compile(r'^impl\s+(?:[\w:]+::)?EffectAnalysisPass\s+for\s+(\w+)\s*\{\s*$')
PASS_KENNUNG = re.compile(r'const ID: AnalysisPassId = AnalysisPassId\("([^"]+)"\);')
FEHLER = re.compile(r'^error(?:\[(E\d{4})\])?: ')
ZUSAMMENFASSUNG = re.compile(r'^error: (?:could not compile|aborting due to)')
ORT = re.compile(r'^\s*-->\s+(.+?):(\d+):(\d+)\s*$')
DIAGNOSE_KOPF = re.compile(r'^(?:error|warning)(?:\[\w+\])?: ')
FERTIG = re.compile(r'^\s*Finished\b', re.M)
KOPF = re.compile(r'^cargo-remote: job (\w+) \| (\S+) \| Rust (\S+)', re.M)
ABSCHLUSS = re.compile(r'^cargo-remote: (\w+) \| exit (-?\d+) \| [\d.]+s\s*$', re.M)
ANSI = re.compile(r'\x1b\[[0-9;]*[A-Za-z]')


class NichtGelaufen(RuntimeError):
    """Ein Bau hat nicht stattgefunden, oder sein Ergebnis kam nicht an."""


@dataclass
class Pass:
    """Ein `impl EffectAnalysisPass for X`-Block; Zeilen 1-basiert, beide Grenzen exklusiv."""
    name: str
    datei: str
    anfang: int
    ende: int
    kennung: str | None


@dataclass
class Bau:
    """Was ein Bau gemeldet hat. `stellen` sind die E0004 zum Gegenproben-Baustein."""
    gebrochen: bool
    stellen: list
    fremd: list
    job: str | None
    sekunden: float


# ---------------------------------------------------------------- Quellen lesen

def lies_rust(wurzel):
    """Die Rust-Quellen der Bibliothek als {Pfad ab Wurzel: Text}, Zeilenenden unverändert."""
    quellen = Path(wurzel) / QUELLEN
    if not quellen.is_dir():
        raise NichtGelaufen(f'{QUELLEN} fehlt unter {wurzel}')
    return {p.relative_to(wurzel).as_posix(): p.read_bytes().decode('utf-8')
            for p in sorted(quellen.rglob('*.rs'))
            if not set(p.relative_to(wurzel).parts) & set(AUSGELASSEN)}


def finde_enum(texte):
    """(Datei, Zeile) von `pub enum EffectOp {` — genau einmal, sonst lässt sich nichts einfügen."""
    funde = [(datei, nummer) for datei, text in texte.items()
             for nummer, zeile in enumerate(text.splitlines(), 1) if ENUM_KOPF.match(zeile)]
    if len(funde) != 1:
        raise NichtGelaufen(f'`pub enum EffectOp {{` steht {len(funde)}-mal in {QUELLEN}, '
                            'erwartet genau einmal — ohne ihn lässt sich kein Baustein einfügen')
    return funde[0]


def lies_registry(texte):
    """(Datei, Zeile, Namen) des einen `registry!`-Aufrufs, in Registrierungsreihenfolge."""
    funde = []
    for datei, text in texte.items():
        zeilen = text.splitlines()
        for i, zeile in enumerate(zeilen):
            if not REGISTRY_KOPF.match(zeile):
                continue
            namen = []
            for weitere in zeilen[i + 1:]:
                code = weitere.split('//', 1)[0].strip()
                if code.startswith('}'):
                    break
                namen += [teil.strip().rsplit('::', 1)[-1] for teil in code.split(',') if teil.strip()]
            funde.append((datei, i + 1, namen))
    if len(funde) != 1:
        raise NichtGelaufen(f'`registry! {{` steht {len(funde)}-mal in {QUELLEN}, erwartet genau einmal — '
                            'ohne Registry gibt es keine Liste der Analysen, die brechen müssen')
    datei, zeile, namen = funde[0]
    if not namen or not all(re.fullmatch(r'[A-Za-z_]\w*', name) for name in namen):
        raise NichtGelaufen(f'{datei}:{zeile}: die Registry ist leer oder unlesbar: {namen}')
    return funde[0]


def finde_paesse(texte):
    """{Name: Pass} aller `impl EffectAnalysisPass for X`-Blöcke, registriert oder nicht.

    Das Blockende ist die erste Zeile, die genau `}` lautet — die Form, in der
    rustfmt einen Block auf oberster Ebene schließt.
    """
    paesse = {}
    for datei, text in texte.items():
        zeilen = text.splitlines()
        for i, zeile in enumerate(zeilen):
            treffer = IMPL_KOPF.match(zeile)
            if not treffer:
                continue
            ende = next((j for j in range(i + 1, len(zeilen)) if zeilen[j].rstrip() == '}'), None)
            if ende is None:
                raise NichtGelaufen(f'{datei}:{i + 1}: das Ende von `impl ... for {treffer[1]}` fehlt')
            kennung = next((m[1] for z in zeilen[i:ende] if (m := PASS_KENNUNG.search(z))), None)
            paesse[treffer[1]] = Pass(treffer[1], datei, i + 1, ende + 1, kennung)
    return paesse


def zuordnen(stellen, paesse):
    """Teilt Stellen auf die Pässe auf, in deren impl-Block sie liegen; der Rest steht außerhalb."""
    je_pass, ausserhalb = {}, []
    for datei, zeile in stellen:
        traeger = next((p.name for p in paesse.values()
                        if p.datei == datei and p.anfang < zeile < p.ende), None)
        if traeger is None:
            ausserhalb.append((datei, zeile))
        else:
            je_pass.setdefault(traeger, []).append((datei, zeile))
    return je_pass, ausserhalb


def _bloecke(text, kopf):
    """{Name: (Zeile, Zeilen des Rumpfs)} der Blöcke auf oberster Ebene, deren Kopf `kopf` trifft."""
    zeilen = text.splitlines()
    funde = {}
    for i, zeile in enumerate(zeilen):
        treffer = kopf.match(zeile)
        if not treffer:
            continue
        ende = next((j for j in range(i + 1, len(zeilen)) if zeilen[j].rstrip() == '}'), None)
        if ende is not None:
            funde[treffer[1]] = (i + 1, zeilen[i + 1:ende])
    return funde


def semantische_typen(texte, enum):
    """[(Name, (Datei, Zeile))] der Aufzählungen, die ein EffectNode trägt — ohne EffectOp selbst.

    Gelesen aus den Feldern von `pub struct EffectNode {`, in Feldreihenfolge,
    und durch jede Struktur des Effekt-Moduls hindurch, die ein Feld trägt
    (der Monotonie-Hinweis trägt Richtung und Bezug). Varianten-Nutzlasten zählen
    nicht: Sie sind kein Feld des Knotens.
    """
    ordner = enum[0].rsplit('/', 1)[0] + '/'
    aufzaehlungen, strukturen = {}, {}
    for datei, text in texte.items():
        if not datei.startswith(ordner):
            continue
        for name, (zeile, _) in _bloecke(text, AUFZAEHLUNG_KOPF).items():
            aufzaehlungen.setdefault(name, []).append((datei, zeile))
        for name, (_, rumpf) in _bloecke(text, STRUKTUR_KOPF).items():
            strukturen.setdefault(name, []).append(
                [m[1] for z in rumpf if (m := FELD.match(z.split('//', 1)[0]))])
    if len(strukturen.get('EffectNode', [])) != 1:
        raise NichtGelaufen(f'`pub struct EffectNode {{` steht {len(strukturen.get("EffectNode", []))}-mal '
                            f'in {ordner}, erwartet genau einmal — ohne ihn sind die semantischen Typen unbekannt')
    ergebnis, gesehen = [], set()

    def besuche(feldtyp):
        for name in re.findall(r'[A-Za-z_]\w*', feldtyp):
            if name in gesehen:
                continue
            gesehen.add(name)
            if name in aufzaehlungen:
                if len(aufzaehlungen[name]) != 1:
                    raise NichtGelaufen(f'`pub enum {name} {{` steht {len(aufzaehlungen[name])}-mal in {ordner}')
                ergebnis.append((name, aufzaehlungen[name][0]))
            elif len(strukturen.get(name, [])) == 1:
                for weiterer in strukturen[name][0]:
                    besuche(weiterer)

    gesehen.add('EffectNode')
    for feldtyp in strukturen['EffectNode'][0]:
        besuche(feldtyp)
    return [(name, ort) for name, ort in ergebnis if name != 'EffectOp']


# ------------------------------------------------------------- Auffangformen

_MEHRZEICHEN = ('..=', '...', '<<=', '>>=', '::', '=>', '==', '!=', '<=', '>=', '&&', '||', '->', '..',
                '+=', '-=', '*=', '/=', '%=', '^=', '&=', '|=', '<<', '>>')
_ZEICHEN = re.compile(r"'(?:\\u\{[0-9A-Fa-f]+\}|\\.|[^'\\\n])'")
_ROH = re.compile(r'(?:b|c)?r(#*)"')
_MAKRO = re.compile(r'!\s*[(\[{]')
_BEZEICHNER = re.compile(r'[A-Za-z_]\w*')
_ZAHL = re.compile(r'\d[\w]*(?:\.\d[\w]*)?')


def lexeme(text):
    """Die Zeichen eines Rust-Texts als [(Wert, Zeile)] — ohne Kommentare, Literale als `"…"`.

    Von Hand statt über einen Ausdruck, weil Blockkommentare in Rust verschachtelt
    sind und rohe Zeichenketten ihr Ende an der Zahl der `#` tragen. Ein Makroaufruf
    `name!(`, `name![`, `name!{` wird zu einem Zeichen `name!`.
    """
    aus, i, zeile, n = [], 0, 1, len(text)
    while i < n:
        c = text[i]
        if c == '\n':
            zeile += 1
            i += 1
        elif c.isspace():
            i += 1
        elif text.startswith('//', i):
            ende = text.find('\n', i)
            i = n if ende == -1 else ende
        elif text.startswith('/*', i):
            tiefe, j = 1, i + 2
            while j < n and tiefe:
                if text.startswith('/*', j):
                    tiefe, j = tiefe + 1, j + 2
                elif text.startswith('*/', j):
                    tiefe, j = tiefe - 1, j + 2
                else:
                    j += 1
            zeile += text.count('\n', i, j)
            i = j
        elif c in 'bcr' and (roh := _ROH.match(text, i)):
            schluss = '"' + roh[1]
            ende = text.find(schluss, i + roh.end())
            ende = n if ende == -1 else ende + len(schluss)
            aus.append(('"…"', zeile))
            zeile += text.count('\n', i, ende)
            i = ende
        elif c == '"' or (c in 'bc' and text.startswith('"', i + 1)):
            j = i + (1 if c == '"' else 2)
            while j < n and text[j] != '"':
                j += 2 if text[j] == '\\' else 1
            aus.append(('"…"', zeile))
            zeile += text.count('\n', i, j + 1)
            i = j + 1
        elif c == "'" or (c == 'b' and text.startswith("'", i + 1)):
            anfang = i + (0 if c == "'" else 1)
            zeichen = _ZEICHEN.match(text, anfang)
            if zeichen:
                aus.append(("'…'", zeile))
                i = zeichen.end()
            else:
                name = _BEZEICHNER.match(text, anfang + 1)
                aus.append(("'" + (name[0] if name else ''), zeile))
                i = name.end() if name else anfang + 1
        elif c.isalpha() or c == '_':
            name = _BEZEICHNER.match(text, i)
            j = name.end()
            if text.startswith('r#', i) and j == i + 1:
                name = _BEZEICHNER.match(text, i + 2)
                j = name.end()
            wert = name[0]
            if _MAKRO.match(text, j):
                wert += '!'
                j += 1
            aus.append((wert, zeile))
            i = j
        elif c.isdigit():
            zahl = _ZAHL.match(text, i)
            aus.append((zahl[0], zeile))
            i = zahl.end()
        else:
            wert = next((op for op in _MEHRZEICHEN if text.startswith(op, i)), c)
            aus.append((wert, zeile))
            i += len(wert)
    return aus


_PAARE = {'(': ')', '[': ']', '{': '}'}


def _schliessende(lex, i):
    """Index der Klammer, die `lex[i]` schließt."""
    auf, tiefe = lex[i][0], 0
    for j in range(i, len(lex)):
        if lex[j][0] == auf:
            tiefe += 1
        elif lex[j][0] == _PAARE[auf]:
            tiefe -= 1
            if tiefe == 0:
                return j
    return len(lex) - 1


def ohne_pruefmodule(lex):
    """Die Zeichen ohne Elemente hinter `#[cfg(test)]` — sie übersetzt `cargo check --lib` nicht."""
    aus, i = [], 0
    while i < len(lex):
        if [w for w, _ in lex[i:i + 7]] == ['#', '[', 'cfg', '(', 'test', ')', ']']:
            j = i + 7
            while j < len(lex) and lex[j][0] == '#' and j + 1 < len(lex) and lex[j + 1][0] == '[':
                j = _schliessende(lex, j + 1) + 1
            while j < len(lex) and lex[j][0] not in ('{', ';'):
                j = _schliessende(lex, j) + 1 if lex[j][0] in ('(', '[') else j + 1
            i = (_schliessende(lex, j) if j < len(lex) and lex[j][0] == '{' else j) + 1
            continue
        aus.append(lex[i])
        i += 1
    return aus


def _nennt(lex, anfang, ende, typen):
    """Der erste Typ aus `typen`, der in lex[anfang:ende] als Pfad `Typ::` steht."""
    for k in range(max(anfang, 0), min(ende, len(lex)) - 1):
        if lex[k][0] in typen and lex[k + 1][0] == '::':
            return lex[k][0]
    return None


def _bis_tiefe_null(lex, k, ende, halt):
    """Erster Index ab k vor `ende`, an dem `lex` auf Klammertiefe 0 ein Zeichen aus `halt` trägt."""
    tiefe = 0
    while k < ende:
        wert = lex[k][0]
        if tiefe == 0 and wert in halt:
            return k
        if wert in _PAARE:
            tiefe += 1
        elif wert in (')', ']', '}'):
            tiefe -= 1
        k += 1
    return ende


def _arme(lex, auf, zu):
    """[(Anfang, Ende)] der Muster eines match-Rumpfs `lex[auf] … lex[zu]`."""
    muster, k = [], auf + 1
    while k < zu:
        while k < zu and lex[k][0] == '#' and lex[k + 1][0] == '[':
            k = _schliessende(lex, k + 1) + 1
        pfeil = _bis_tiefe_null(lex, k, zu, ('=>',))
        if pfeil >= zu:
            break
        muster.append((k, pfeil))
        k = pfeil + 1
        if k < zu and lex[k][0] == '{':
            k = _schliessende(lex, k) + 1
            if k < zu and lex[k][0] == ',':
                k += 1
        else:
            k = _bis_tiefe_null(lex, k, zu, (',',)) + 1
    return muster


def _ist_auffang(alternative):
    """Ob ein Arm-Muster (eine Alternative, ohne Wächter) jeden Wert nimmt: `_` oder eine bloße Bindung."""
    werte = [w for w in alternative if w not in ('ref', 'mut')]
    bindung = bool(werte) and werte[0] not in ('true', 'false') and re.fullmatch(r'[a-z_]\w*', werte[0])
    return werte == ['_'] or (bindung and (len(werte) == 1 or werte[1:] == ['@', '_']))


def auffangformen(texte, typen):
    """[(Datei, Zeile, Form, Typ)] aller Auffangformen über `typen` in Bibliothekscode ohne Prüfmodule."""
    typen = set(typen)
    funde = []
    for datei, text in sorted(texte.items()):
        lex = ohne_pruefmodule(lexeme(text))
        for i, (wert, zeile) in enumerate(lex):
            if wert == 'matches!' and i + 1 < len(lex) and lex[i + 1][0] == '(':
                if (typ := _nennt(lex, i + 2, _schliessende(lex, i + 1), typen)):
                    funde.append((datei, zeile, 'matches!(…)', typ))
            elif wert == 'match':
                auf = _bis_tiefe_null(lex, i + 1, len(lex), ('{',))
                if auf >= len(lex):
                    continue
                zu = _schliessende(lex, auf)
                muster = _arme(lex, auf, zu)
                genannt = sorted({t for a, b in muster if (t := _nennt(lex, a, b, typen))})
                if not genannt:
                    continue
                for a, b in muster:
                    ende = next((k for k in range(a, b) if lex[k][0] == 'if'), b)
                    alternativen, anfang = [], a
                    while anfang < ende:
                        strich = _bis_tiefe_null(lex, anfang, ende, ('|',))
                        if strich > anfang:
                            alternativen.append([w for w, _ in lex[anfang:strich]])
                        anfang = strich + 1
                    if any(_ist_auffang(alt) for alt in alternativen):
                        text_arm = ' '.join(w for w, _ in lex[a:b])
                        funde.append((datei, lex[a][1], f'Arm `{text_arm}`', '/'.join(genannt)))
            elif wert in ('==', '!='):
                links, k = [], i - 1
                while k >= 0 and (lex[k][0] == '::' or _BEZEICHNER.fullmatch(lex[k][0])):
                    links.insert(0, lex[k])
                    k -= 1
                rechts, k = [], i + 1
                while k < len(lex) and (lex[k][0] == '::' or _BEZEICHNER.fullmatch(lex[k][0])):
                    rechts.append(lex[k])
                    k += 1
                if (typ := _nennt(links, 0, len(links), typen) or _nennt(rechts, 0, len(rechts), typen)):
                    funde.append((datei, zeile, f'`{wert}` mit einem Wert', typ))
            elif wert == 'let':
                gleich = _bis_tiefe_null(lex, i + 1, len(lex), ('=', ';'))
                if (typ := _nennt(lex, i + 1, gleich, typen)):
                    vorher = lex[i - 1][0] if i else ''
                    form = f'{vorher} let' if vorher in ('if', 'while') else 'let … else'
                    funde.append((datei, zeile, form, typ))
    return funde


# ------------------------------------------------------------------- Mutationen

def _zeilen(text):
    return text.splitlines(keepends=True)


def _umbruch(zeile):
    return '\r\n' if zeile.endswith('\r\n') else '\n'


def _einzug(zeile):
    return zeile[:len(zeile) - len(zeile.lstrip(' '))]


def modulpfad(datei):
    """Der Pfad des Moduls, in dem EffectOp steht, z. B. `crate::effects`."""
    teile = list(Path(datei).relative_to(QUELLEN).with_suffix('').parts)
    if teile and teile[-1] in ('mod', 'lib'):
        teile.pop()
    return '::'.join(['crate', *teile])


def mit_baustein(texte, enum):
    """Fügt den Gegenproben-Wert als erste Variante der Aufzählung ein, deren Kopf an `enum` steht."""
    datei, nummer = enum
    zeilen = _zeilen(texte[datei])
    umbruch = _umbruch(zeilen[nummer - 1])
    einzug = _einzug(zeilen[nummer]) if nummer < len(zeilen) and zeilen[nummer].strip() else '    '
    zeilen[nummer:nummer] = [
        f'{einzug}/// Wegwerf-Wert der Baubruch-Gegenprobe (#892), nur in der Kopie.{umbruch}',
        f'{einzug}{VARIANTE},{umbruch}',
    ]
    return {**texte, datei: ''.join(zeilen)}


def _oeffnung(zeilen, nummer, ort):
    """Index der Zeile, die den Rumpf der gemeldeten Fallunterscheidung öffnet."""
    for i in range(nummer - 1, min(nummer + 3, len(zeilen))):
        if zeilen[i].split('//', 1)[0].rstrip().endswith('{'):
            return i
    raise NichtGelaufen(f'{ort}: keine öffnende Klammer der Fallunterscheidung gefunden')


def _schluss(zeilen, i, ort):
    """Index der schließenden Klammer: dieselbe Einrückung wie die öffnende Zeile."""
    tiefe = len(_einzug(zeilen[i]))
    for j in range(i + 1, len(zeilen)):
        if zeilen[j].strip().startswith('}') and len(_einzug(zeilen[j])) == tiefe:
            return j
    raise NichtGelaufen(f'{ort}: keine schließende Klammer der Fallunterscheidung gefunden')


def einordnen(texte, stellen, modul, auffangzweig=False, typ='EffectOp'):
    """Ordnet den neuen Wert an den Stellen ein — oder setzt dort einen Auffangzweig.

    Die Einordnung ist ein eigener Arm `<modul>::<typ>::<Variante> =>
    panic!("…")` direkt hinter der öffnenden Klammer; der Auffangzweig ein
    `_ => panic!("…")` vor der schließenden. Je Datei von unten nach oben,
    damit die gemeldeten Zeilennummern für die übrigen Stellen gültig bleiben.

    `panic!` mit fester Zeichenkette und nicht `unreachable!("…")`: Eine Stelle
    kann in einer `const fn` liegen, und dort verweigert der Übersetzer jedes
    formatierende Makro (E0015, gemessen an `austausch.rs`,
    `kann_durch_krit_ausgeloest_werden`, im ersten Lauf über `Trigger`).
    """
    neu = dict(texte)
    for datei in sorted({d for d, _ in stellen}):
        if datei not in neu:
            raise NichtGelaufen(f'{datei}: die gemeldete Stelle liegt außerhalb von {QUELLEN} der Kopie')
        zeilen = _zeilen(neu[datei])
        for _, nummer in sorted((s for s in stellen if s[0] == datei), key=lambda s: -s[1]):
            ort = f'{datei}:{nummer}'
            i = _oeffnung(zeilen, nummer, ort)
            umbruch, einzug = _umbruch(zeilen[i]), _einzug(zeilen[i]) + '    '
            if auffangzweig:
                zeilen.insert(_schluss(zeilen, i, ort),
                              f'{einzug}_ => panic!("Auffangzweig der Gegenprobe #892"),{umbruch}')
            else:
                zeilen.insert(i + 1, f'{einzug}{modul}::{typ}::{VARIANTE} => '
                                     f'panic!("Baubruch-Gegenprobe #892"),{umbruch}')
        neu[datei] = ''.join(zeilen)
    return neu


# ------------------------------------------------------------------ Kopie, Bau

def fingerabdruck(wurzel):
    """SHA-256 jeder Datei, die in die Kopie geht — der echte Baum muss danach gleich sein."""
    wurzel = Path(wurzel)
    ergebnis = {}
    kandidaten = [wurzel / name for name in KOPIE_DATEIEN]
    for ordner in KOPIE_ORDNER:
        kandidaten += sorted((wurzel / ordner).rglob('*'))
    for pfad in kandidaten:
        relativ = pfad.relative_to(wurzel)
        if pfad.is_file() and not set(relativ.parts) & set(AUSGELASSEN):
            ergebnis[relativ.as_posix()] = hashlib.sha256(pfad.read_bytes()).hexdigest()
    return ergebnis


def kopiere(wurzel, kopie, befehl):
    """Legt die Wegwerfkopie der Rust-Eingaben an."""
    wurzel = Path(wurzel)
    if not (wurzel / 'Cargo.toml').is_file():
        raise NichtGelaufen(f'{wurzel}/Cargo.toml fehlt — keine Workspace-Wurzel')
    if befehl == 'remote' and not (wurzel / '.cargo-remote.toml').is_file():
        raise NichtGelaufen('.cargo-remote.toml fehlt — ohne sie reicht der v2-Client an den alten '
                            'Client weiter, und gebaut würde nicht auf dem v2-Dienst')
    kopie.mkdir()
    for name in KOPIE_DATEIEN:
        if (wurzel / name).is_file():
            shutil.copy2(wurzel / name, kopie / name)
    for ordner in KOPIE_ORDNER:
        if not (wurzel / ordner).is_dir():
            raise NichtGelaufen(f'{ordner}/ fehlt unter {wurzel} — build.rs liest es')
        shutil.copytree(wurzel / ordner, kopie / ordner, ignore=shutil.ignore_patterns(*AUSGELASSEN))


def schreibe(kopie, urtext, zustand):
    """Stellt in der Kopie genau `zustand` her; unveränderte Dateien bleiben unberührt."""
    for datei in sorted(set(urtext) | set(zustand)):
        text = zustand.get(datei, urtext.get(datei))
        ziel = kopie / datei
        if ziel.read_bytes().decode('utf-8') != text:
            ziel.write_bytes(text.encode('utf-8'))


def _schluss_der_ausgabe(text, zeilen=6):
    rest = [z for z in text.strip().splitlines() if z.strip()][-zeilen:]
    return ('\n    ' + '\n    '.join(rest)) if rest else ''


def _normalisiere(pfad, kopie):
    pfad = pfad.replace('\\', '/')
    for basis in (str(kopie).rstrip('/') + '/', os.path.realpath(kopie).rstrip('/') + '/', '/workspace/', './'):
        if pfad.startswith(basis):
            return pfad[len(basis):]
    return pfad


def auswerten(code, text, kopie, job, sekunden):
    """Liest Bruch, E0004-Stellen und fremde Fehler aus einer Bauausgabe."""
    zeilen = text.splitlines()
    stellen, fremd = [], []
    for i, zeile in enumerate(zeilen):
        treffer = FEHLER.match(zeile)
        if not treffer or ZUSAMMENFASSUNG.match(zeile):
            continue
        if treffer[1] == 'E0004' and VARIANTE in zeile:
            ort = None
            for weitere in zeilen[i + 1:]:
                if DIAGNOSE_KOPF.match(weitere):
                    break
                if (ort := ORT.match(weitere)):
                    break
            if ort is None:
                fremd.append(f'{zeile} (ohne Ortsangabe)')
            else:
                stellen.append((_normalisiere(ort[1], kopie), int(ort[2])))
        else:
            fremd.append(zeile)
    if code == 0:
        if not FERTIG.search(text):
            raise NichtGelaufen('Exit 0 ohne „Finished“ — kein Beleg, dass überhaupt gebaut wurde'
                                + _schluss_der_ausgabe(text))
        if stellen or fremd:
            raise NichtGelaufen('Exit 0, aber mit Fehlermeldungen des Übersetzers' + _schluss_der_ausgabe(text))
        return Bau(False, [], [], job, sekunden)
    if not stellen and not fremd:
        raise NichtGelaufen('Exit 101 ohne eine einzige Fehlermeldung des Übersetzers'
                            + _schluss_der_ausgabe(text))
    return Bau(True, stellen, fremd, job, sekunden)


def baue(befehl, kopie, timeout, protokoll, nummer, kurz):
    """Ein Bau der Kopie. Wirft NichtGelaufen, wenn er nicht stattfand oder nichts ankam."""
    argv = [*BEFEHLE[befehl], *CARGO_AUFTRAG]
    env = dict(os.environ, CARGO_TERM_COLOR='never')
    if befehl == 'cargo':
        env['CARGO_TARGET_DIR'] = str(kopie / 'target')
    else:
        # Der Client nimmt seine Quellen aus `git ls-files`, sobald er einen
        # Arbeitsbaum findet. Die Kopie hat keinen eigenen; ohne diese Grenze
        # gälte ein umgebender — mit dessen .gitignore.
        env['GIT_CEILING_DIRECTORIES'] = str(kopie.parent)
    beginn = time.monotonic()
    try:
        prozess = subprocess.run(argv, cwd=kopie, env=env, stdin=subprocess.DEVNULL,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                 timeout=timeout, check=False)
    except FileNotFoundError as grund:
        raise NichtGelaufen(f'{argv[0]} nicht gefunden: {grund}') from grund
    except subprocess.TimeoutExpired as grund:
        hinweis = (' — der Job läuft auf dem Bau-Dienst womöglich weiter; erst `cargo remote --status` '
                   'ansehen' if befehl == 'remote' else '')
        raise NichtGelaufen(f'{" ".join(argv)}: Zeitlimit von {timeout} s überschritten{hinweis}') from grund
    sekunden = time.monotonic() - beginn
    text = ANSI.sub('', prozess.stdout.decode('utf-8', errors='replace'))
    code = prozess.returncode
    if protokoll is not None:
        (protokoll / f'{nummer:02d}-{kurz}.log').write_text(
            f'$ {" ".join(argv)}\n# cwd {kopie}\n# exit {code}, {sekunden:.1f} s\n{text}', encoding='utf-8')
    job = None
    if befehl == 'remote':
        if code in DIENST_AUSGAENGE:
            raise NichtGelaufen(f'Exit {code} — {DIENST_AUSGAENGE[code]}' + _schluss_der_ausgabe(text))
        kopf, abschluss = KOPF.search(text), ABSCHLUSS.findall(text)
        if not kopf:
            raise NichtGelaufen(f'Exit {code} ohne Kopfzeile `cargo-remote: job …` — kein Job vergeben'
                                + _schluss_der_ausgabe(text))
        if not abschluss:
            raise NichtGelaufen(f'Exit {code}, aber die Abschlusszeile `cargo-remote: … | exit … | …s` '
                                'fehlt — das Ergebnis des Jobs ist nicht angekommen' + _schluss_der_ausgabe(text))
        if int(abschluss[-1][1]) != code:
            raise NichtGelaufen(f'die Abschlusszeile meldet Exit {abschluss[-1][1]}, der Client endete mit '
                                f'{code} — welcher gilt, ist offen' + _schluss_der_ausgabe(text))
        job = kopf[1]
    if code not in (0, 101):
        raise NichtGelaufen(f'unerwarteter Exit {code} von `{" ".join(argv)}`' + _schluss_der_ausgabe(text))
    return auswerten(code, text, kopie, job, sekunden)


# ------------------------------------------------------------------- Ablauf

def _orte(stellen):
    return ', '.join(f'{datei}:{zeile}' for datei, zeile in stellen)


def _dauer(bau):
    return f'{bau.sekunden:.1f} s' + (f', Job {bau.job}' if bau.job else '')


def gegenprobe(befehl, timeout, protokoll):
    """Der ganze Lauf. Die Rücklesung des echten Baums gilt auch nach einem frühen Ende."""
    befunde = []

    def befund(text):
        befunde.append(text)
        print(f'BEFUND: {text}', flush=True)

    vorher = fingerabdruck(ROOT)
    if protokoll is not None:
        protokoll.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='rota-wirkung-baubruch-') as temporaer:
        registriert, typen, bauten = _schritte(befehl, timeout, protokoll, Path(temporaer) / 'kopie', befund)
    nachher = fingerabdruck(ROOT)
    if nachher != vorher:
        geaendert = sorted(k for k in vorher.keys() | nachher.keys() if vorher.get(k) != nachher.get(k))
        befund('der echte Arbeitsbaum hat sich während des Laufs verändert: ' + ', '.join(geaendert[:10]))
    if befunde:
        print(f'Ergebnis: BEFUND — {len(befunde)} Abweichung(en) in {bauten} Bauten, siehe BEFUND-Zeilen. '
              'Exit 1', flush=True)
        return 1
    weitere = (f'ein neuer Wert jedes der {typen} weiteren semantischen Typen bricht ihn ebenfalls'
               if typen else 'keine weiteren semantischen Typen')
    print(f'Ergebnis: BELEGT — jede der {registriert} registrierten Analysen bricht den Bau, auch '
          f'einzeln; {weitere}; keine Auffangform; {bauten} Bauten, echter Arbeitsbaum unverändert. '
          'Exit 0', flush=True)
    return 0


def _sieht_auffangzweig(mit, ohne, datei, typen):
    """Ob die Suche in `mit` genau eine Auffangform mehr findet als in `ohne`, und zwar in `datei`.

    Verglichen wird die Zahl, nicht die Zeile: Der eingefügte Arm verschiebt die
    Zeilennummern dahinter.
    """
    vorher, nachher = auffangformen(ohne, typen), auffangformen(mit, typen)
    in_datei = sum(f[0] == datei for f in nachher) - sum(f[0] == datei for f in vorher)
    return len(nachher) == len(vorher) + 1 and in_datei == 1


def _schritte(befehl, timeout, protokoll, kopie, befund):
    """S, K0, die EffectOp-Schritte, T0/T1 je Typ. Gibt (Analysen, weitere Typen, Bauten) zurück."""
    kopiere(ROOT, kopie, befehl)
    urtext = lies_rust(kopie)
    enum = finde_enum(urtext)
    reg_datei, reg_zeile, registriert = lies_registry(urtext)
    paesse = finde_paesse(urtext)
    typen = semantische_typen(urtext, enum)
    alle_typen = ['EffectOp', *(name for name, _ in typen)]
    modul = modulpfad(enum[0])
    bauten = 0

    def bauen(kurz, zustand):
        nonlocal bauten
        bauten += 1
        schreibe(kopie, urtext, zustand)
        return baue(befehl, kopie, timeout, protokoll, bauten, kurz)

    print(f'Wirkungs-Baubruch-Gegenprobe (#892): {" ".join([*BEFEHLE[befehl], *CARGO_AUFTRAG])} '
          'in einer Wegwerfkopie', flush=True)
    print(f'  Registriert: {len(registriert)} Analysen in {reg_datei}:{reg_zeile} — '
          f'{", ".join(registriert)}', flush=True)
    print(f'  Baustein: {modul}::EffectOp::{VARIANTE}, eingefügt in {enum[0]}:{enum[1]}', flush=True)
    print(f'  Weitere semantische Typen aus EffectNode: '
          f'{", ".join(f"{name} ({ort[0]}:{ort[1]})" for name, ort in typen) or "keine"}', flush=True)
    for name in registriert:
        if name not in paesse:
            befund(f'{name} steht in der Registry ({reg_datei}:{reg_zeile}), aber kein '
                   f'`impl EffectAnalysisPass for {name}` in {QUELLEN} — ob er bricht, ist nicht zuordenbar')

    # S: Was der Übersetzer nicht sehen kann — eine Stelle, die mit jedem neuen
    # Wert weiter übersetzt.
    funde = auffangformen(urtext, alle_typen)
    for datei, zeile, form, typ in funde:
        befund(f'S: Auffangform über {typ} in {datei}:{zeile} — {form}; ein neuer Wert liefe hier still durch')
    if not funde:
        print(f'S   Suche nach Auffangformen über {len(alle_typen)} Typen in {len(urtext)} Dateien, ohne '
              'Prüfmodule: keine — wie erwartet', flush=True)

    # K0: Ohne grünen Bestand hätte jeder rote Bau danach keine Aussage.
    k0 = bauen('k0', urtext)
    if k0.gebrochen:
        befund('K0: die unbeschädigte Kopie baut nicht — jeder rote Bau danach wäre ohne Aussage: '
               + '; '.join(k0.fremd[:3] or [_orte(k0.stellen)]))
        return len(registriert), len(typen), bauten
    print(f'K0  Kontrolle, unbeschädigte Kopie: baut ({_dauer(k0)}) — wie erwartet', flush=True)

    _effectop_schritte(urtext, enum, registriert, paesse, modul, alle_typen, bauen, befund)
    for typ, ort in typen:
        _typ_schritte(urtext, typ, ort, alle_typen, bauen, befund)
    return len(registriert), len(typen), bauten


def _typ_schritte(urtext, typ, ort, alle_typen, bauen, befund):
    """T0 und T1 für einen semantischen Typ, dazu die Positivkontrolle der Suche S."""
    modul = modulpfad(ort[0])
    t_text = mit_baustein(urtext, ort)
    t0 = bauen(f't0-{typ}', t_text)
    if not t0.gebrochen:
        befund(f'T0 {typ}: ein neuer Wert ohne Einordnung bricht den Bau NICHT — keine Stelle in {QUELLEN} '
               f'wertet {typ} erschöpfend aus')
        return
    if t0.fremd:
        befund(f'T0 {typ}: der Bau bricht auch aus fremdem Grund: ' + '; '.join(t0.fremd[:3]))
    if not t0.stellen:
        befund(f'T0 {typ}: keine einzige E0004-Stelle nennt {VARIANTE} — nichts einzuordnen')
        return
    print(f'T0  {typ}: neuer Wert ohne Einordnung bricht an {len(t0.stellen)} Stelle(n) ({_dauer(t0)}): '
          f'{_orte(t0.stellen)}', flush=True)
    t1 = bauen(f't1-{typ}', einordnen(t_text, t0.stellen, modul, typ=typ))
    if t1.gebrochen:
        befund(f'T1 {typ}: an allen gemeldeten Stellen eingeordnet, baut die Kopie trotzdem nicht — die '
               'Mutation selbst ist fehlerhaft, oder T0 hat nicht jede Stelle gemeldet: '
               + '; '.join(t1.fremd[:3] or [_orte(t1.stellen)]))
    else:
        print(f'T1  {typ}: an jeder gemeldeten Stelle eingeordnet: baut ({_dauer(t1)}) — wie erwartet', flush=True)
    erste = t0.stellen[0]
    mit_auffang = einordnen(t_text, [erste], modul, auffangzweig=True, typ=typ)
    if _sieht_auffangzweig(mit_auffang, t_text, erste[0], alle_typen):
        print(f'S   Kontrolle {typ}: mit Auffangzweig an {_orte([erste])} meldet die Suche ihn — wie erwartet',
              flush=True)
    else:
        befund(f'S-Kontrolle {typ}: mit Auffangzweig an {_orte([erste])} meldet die Suche nach Auffangformen '
               'nicht genau eine Fundstelle mehr in dieser Datei — sie wäre dort blind')


def _effectop_schritte(urtext, enum, registriert, paesse, modul, alle_typen, bauen, befund):
    """M0, K1, Mutation je Pass und G für EffectOp."""
    # M0: der neue Baustein, nirgends eingeordnet.
    m0_text = mit_baustein(urtext, enum)
    m0 = bauen('m0', m0_text)
    if not m0.gebrochen:
        befund(f'M0: der Bau bricht NICHT, obwohl {VARIANTE} nirgends eingeordnet ist — die Zusicherung, '
               'dass ein neuer Baustein den Bau bricht, gilt nicht')
        return
    if m0.fremd:
        befund('M0: der Bau bricht auch aus fremdem Grund: ' + '; '.join(m0.fremd[:3]))
    if not m0.stellen:
        befund(f'M0: keine einzige E0004-Stelle nennt {VARIANTE} — nichts zuzuordnen')
        return
    m0_paesse = finde_paesse(m0_text)
    je_pass, ausserhalb = zuordnen(m0.stellen, m0_paesse)
    print(f'M0  Gegenprobe, Baustein ohne Einordnung: bricht an {len(m0.stellen)} Stellen '
          f'({_dauer(m0)})', flush=True)
    for name in registriert:
        if name in m0_paesse:
            orte = _orte(je_pass.get(name, [])) or 'KEINE STELLE'
            print(f'      {name:<24} {m0_paesse[name].kennung or "?":<16} {orte}', flush=True)
    for name in sorted(set(je_pass) - set(registriert)):
        print(f'      {name} (nicht registriert): {_orte(je_pass[name])}', flush=True)
    for stelle in ausserhalb:
        print(f'      außerhalb der Analysen: {_orte([stelle])}', flush=True)
    gebrochen = [name for name in registriert if je_pass.get(name)]
    for name in registriert:
        if name in paesse and not je_pass.get(name):
            befund(f'M0: {name} bricht den Bau NICHT — seine Fallunterscheidung über EffectOp hat einen '
                   'Auffangzweig, oder es gibt keine')

    # K1: an jeder gemeldeten Stelle eingeordnet, muss die Kopie wieder bauen.
    k1 = bauen('k1', einordnen(m0_text, m0.stellen, modul))
    if k1.gebrochen:
        befund('K1: an allen gemeldeten Stellen eingeordnet, baut die Kopie trotzdem nicht — die Mutation '
               'selbst ist fehlerhaft, oder M0 hat nicht jede Stelle gemeldet: '
               + '; '.join(k1.fremd[:3] or [_orte(k1.stellen)]))
    else:
        print(f'K1  Kontrolle, Baustein überall eingeordnet: baut ({_dauer(k1)}) — wie erwartet', flush=True)

    # Mutation je Pass: überall eingeordnet außer in genau einem.
    for name in registriert:
        if name not in paesse:
            continue
        eigene = je_pass.get(name, [])
        if not eigene:
            befund(f'Mutation je Pass: {name} hat in M0 keine Stelle; offen gelassen bricht er nichts '
                   '(ohne Bau — der Zustand gleicht K1)')
            continue
        zustand = einordnen(m0_text, [s for s in m0.stellen if s not in eigene], modul)
        bau = bauen(f'pass-{name}', zustand)
        if not bau.gebrochen:
            befund(f'Mutation je Pass: nur {name} offen, und der Bau bricht NICHT')
            continue
        dort, anderswo = zuordnen(bau.stellen, finde_paesse(zustand))
        fremde_paesse = sorted(set(dort) - {name})
        if fremde_paesse or anderswo or bau.fremd or not dort.get(name):
            befund(f'Mutation je Pass: nur {name} offen, aber der Bruch steht nicht nur dort — '
                   f'Pässe {fremde_paesse}, außerhalb {_orte(anderswo) or "—"}, '
                   f'fremd {bau.fremd[:2] or "—"}')
            continue
        print(f'P   Mutation je Pass, nur {name} offen: bricht nur dort ({_orte(dort[name])}; '
              f'{_dauer(bau)}) — wie erwartet', flush=True)

    # G: Ein Auffangzweig muss den Pass aus dem Bruch nehmen — und die
    # Auswertung muss das sehen. Sonst könnte sie "ungebrochen" nie melden.
    if gebrochen:
        ziel = gebrochen[0]
        zustand = einordnen(m0_text, je_pass[ziel], modul, auffangzweig=True)
        bau = bauen('g', zustand)
        dort, _ = zuordnen(bau.stellen, finde_paesse(zustand))
        noch = sorted(n for n in dort if n in registriert)
        erwartet = sorted(set(gebrochen) - {ziel})
        if ziel in dort:
            befund(f'Gegenprobe der Gegenprobe: mit Auffangzweig in {ziel} meldet die Auswertung ihn weiter '
                   'als gebrochen — sie könnte einen Pass, der nicht bricht, gar nicht erkennen')
        elif noch != erwartet or bau.fremd:
            befund(f'Gegenprobe der Gegenprobe: Auffangzweig in {ziel}, erwartet brechen {erwartet}, '
                   f'gemeldet {noch}, fremd {bau.fremd[:2] or "—"}')
        else:
            print(f'G   Gegenprobe der Gegenprobe, Auffangzweig in {ziel}: als ungebrochen erkannt, '
                  f'{len(noch)} weitere brechen ({_dauer(bau)}) — wie erwartet', flush=True)
        # Derselbe Auffangzweig ist die Positivkontrolle der Suche S: Was der
        # Übersetzer nicht mehr meldet, muss sie finden.
        datei = je_pass[ziel][0][0]
        if _sieht_auffangzweig(zustand, m0_text, datei, alle_typen):
            print(f'S   Kontrolle EffectOp: den Auffangzweig aus G in {datei} meldet die Suche — wie erwartet',
                  flush=True)
        else:
            befund(f'S-Kontrolle EffectOp: den Auffangzweig aus G in {datei} meldet die Suche nach '
                   'Auffangformen nicht als genau eine Fundstelle mehr — sie wäre dort blind')


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--befehl', choices=sorted(BEFEHLE), required=True,
                        help='cargo in der CI; remote lokal (cargo remote --no-copy-lock --)')
    parser.add_argument('--timeout', type=float, default=1200.0, help='Sekunden je Bau (Vorgabe 1200)')
    parser.add_argument('--protokoll', type=Path, help='Verzeichnis für die vollständige Ausgabe jedes Baus')
    args = parser.parse_args(argv)
    try:
        return gegenprobe(args.befehl, args.timeout, args.protokoll)
    except NichtGelaufen as grund:
        print(f'NICHT GELAUFEN: {grund}', flush=True)
        print('Ergebnis: NICHT GELAUFEN — kein Urteil über die Wirkungsbeschreibung. Exit 2', flush=True)
        return 2
    except KeyboardInterrupt:
        print('Ergebnis: NICHT GELAUFEN — abgebrochen. Exit 2', flush=True)
        return 2


if __name__ == '__main__':
    sys.exit(main())
