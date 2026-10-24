---
id: ci-pfadfilter
status: Stable
ssot: true
refs:
  docs: [WORKER-GUIDE.md]
affects: [.github/workflows]
---

# Welche Änderung löst welche CI-Stufe aus?

Erhoben am 2026-08-22 für Issue #1100, AC4. Am 2026-09-06 um die
Python-Prüfung und die isolierten Gegenproben im stets laufenden `lint`-Job
ergänzt. Am 2026-10-24 für Epic #892 um die Werkzeuge ergänzt, die der
`sim`-Job selbst fährt (Befund 4). Der Prüfvertrag steht in
[PYTHON-WERKZEUGE.md](PYTHON-WERKZEUGE.md).

> ### ⚠ Die Tabelle unten ist ein DATUM. Der Ist-Stand steht in einem Befehl
>
> ```bash
> bash tools/ci-pfadfilter.sh                 # die Standardliste
> bash tools/ci-pfadfilter.sh pfad/meine.ts   # eigene Pfade
> ```
>
> Das Werkzeug **liest die Muster aus `ci.yml`**, statt sie abzuschreiben. Eine
> Abschrift kann nicht bemerken, dass sie veraltet ist — derselbe Grund, aus dem
> `tests/unit/ci-sim-filter.test.ts` existiert.

## Die zwei Filter

`ci.yml` kennt drei Ausgänge, aber nur **zwei** Muster: `rules` ist wörtlich
dasselbe wie `code`.

| Ausgang | Muster                                                                                                                                                                                  |
| ------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `code`  | alles, was **nicht** auf `^(docs/\|.*\.md)$` passt                                                                                                                                      |
| `sim`   | `^(sim/\|data/\|tests/parity/recordings/\|Cargo\.toml$\|Cargo\.lock$\|rust-toolchain\.toml$\|\.github/workflows/ci\.yml$\|tools/rust-pin\.py$\|tools/wirkung-baubruch-gegenprobe\.py$)` |
| `rules` | dasselbe wie `code`                                                                                                                                                                     |

## Die Stufen und ihre Bedingung

| Stufe         | läuft bei               |
| ------------- | ----------------------- |
| `changes`     | **immer**               |
| `lint`        | **immer** (kein Filter) |
| `actionlint`  | **immer**               |
| `secret-scan` | **immer**               |
| `ci-pass`     | **immer** (`always()`)  |
| `typecheck`   | `code`                  |
| `unit`        | `code`                  |
| `build`       | `code`                  |
| `parity`      | `rules` (= `code`)      |
| `sim`         | `sim`                   |

`lint` hat bewusst keinen Filter: `npm run lint` fährt `prettier --check .` über
**alle** Dateien, auch Markdown. Der Kommentar in `ci.yml` nennt den Vorfall,
der dazu geführt hat (PR #75).

## Die Tabelle

| geänderte Datei                           | `code`   | `sim` | zusätzliche Stufen          |
| ----------------------------------------- | -------- | ----- | --------------------------- |
| `docs/GAME-DESIGN.md`                     | false    | false | —                           |
| `README.md`                               | false    | false | —                           |
| **`docs/groessen-abdeckung-messung.txt`** | **true** | false | typecheck unit build parity |
| `src/core/combat.ts`                      | true     | false | typecheck unit build parity |
| `tests/unit/x.test.ts`                    | true     | false | typecheck unit build parity |
| `tests/parity/payloads.ts`                | true     | false | typecheck unit build parity |
| `tests/parity/recordings/x.json`          | true     | true  | + sim                       |
| `sim/src/fight.rs`                        | true     | true  | + sim                       |
| `sim/build.rs`                            | true     | true  | + sim                       |
| `data/spells/calcination.json`            | true     | true  | + sim                       |
| `Cargo.toml`                              | true     | true  | + sim                       |
| `rust-toolchain.toml`                     | true     | true  | + sim                       |
| **`tools/*.py`**                          | true     | false | typecheck unit build parity |
| `tools/rust-pin.py`                       | true     | true  | + sim                       |
| `tools/wirkung-baubruch-gegenprobe.py`    | true     | true  | + sim                       |
| `.github/workflows/ci.yml`                | true     | true  | + sim                       |
| **`.github/workflows/nightly.yml`**       | true     | false | typecheck unit build parity |
| `package.json`                            | true     | false | typecheck unit build parity |
| `assets/sfx/x.mp3`                        | true     | false | typecheck unit build parity |

### Gegengeprüft an drei echten Läufen

Nicht nur gerechnet — gegen die Checks dreier PRs desselben Tages gehalten:

| PR      | geändert                                | Vorhersage                    | gemessen          |
| ------- | --------------------------------------- | ----------------------------- | ----------------- |
| `#1091` | nur `docs/*.md`                         | nur die immer laufenden       | 5 pass, 5 skip    |
| `#1098` | `docs/*.md`, `docs/*.txt`, `tools/*.py` | + typecheck unit build parity | 10 pass, sim skip |
| `#1096` | `data/`, `sim/`, `tests/parity/`        | alles                         | 11 pass           |

## Drei Befunde

### 1. `docs/` schützt nur `.md`, nicht das Verzeichnis

Das Muster lautet `^(docs/|.*\.md)$` — die Alternative `docs/` steht **vor
einem `$`**. Sie passt deshalb nur auf eine Zeile, die exakt `docs/` heißt, und
auf keine Datei darin.

Eine Änderung an `docs/groessen-abdeckung-messung.txt` zählt damit als **Code**
und startet vier Stufen. Der Kommentar darüber sagt „Reine Doku-Änderungen
überspringen alle Code-Jobs"; das gilt nur für `.md`.

**Die Richtung ist die harmlose** — es läuft zu viel, nicht zu wenig. Aber wer
den Kommentar liest, rechnet mit dem Gegenteil, und beim nächsten Umbau des
Musters ist unklar, was Absicht war.

### 2. Python-Werkzeuge werden im stets laufenden `lint`-Job geprüft

Der ursprüngliche Befund vom 22.08. war eine fehlende Python-Prüfung trotz
laufender TypeScript-Jobs. Seit dem 06.09. übersetzt `lint` alle
`tools/**/*.py` mit `py_compile` und fährt `tests/tools/` sowie die drei
isolierten Gegenproben. Die Einstufung jedes Python-Werkzeugs benennt, welches
Verhalten dabei geprüft wird und welches bewusst nicht automatisch läuft.

### 3. `nightly.yml` steht nicht im `sim`-Muster, `ci.yml` schon

`ci.yml` hat sich selbst aufgenommen, und der Kommentar dort begründet es
ausführlich: Ohne das ändert jemand den Testbefehl, die Stufe läuft nicht, und
`ci-pass` meldet grün (gemessen an PR #995).

`nightly.yml` fährt ebenfalls Rust — `cargo run --example report`, `parity`,
`zauber-abdeckung` — und steht **nicht** im Muster. Eine Änderung dort startet
`actionlint` (immer) und die vier `code`-Stufen, aber nicht `sim`.

**Ob das ein Loch ist, hängt daran, was `sim` für `nightly.yml` prüfen würde.**
`actionlint` fängt Syntax; ein falscher `--example`-Name fiele erst nachts auf.
Das ist eine kleinere Klasse als bei `ci.yml`, wo die Definition der PR-Prüfung
selbst betroffen war — aber dieselbe Form.

### 4. Werkzeuge, die der `sim`-Job selbst fährt, gehören ins Muster

Dieselbe Klasse wie `ci.yml` im Muster, eine Ebene tiefer: Ein Werkzeug, das
der `sim`-Job ausführt, ist Teil der Prüfung. Fällt es durch den Filter, ändert
jemand die Prüfung, die Stufe läuft nicht, und `ci-pass` meldet grün.

Seit dem 2026-10-24 fährt der `sim`-Job als letzten Schritt
`python3 -B tools/wirkung-baubruch-gegenprobe.py --befehl cargo` (#892). Der
Fall in `tests/unit/ci-sim-filter.test.ts`, der dafür dazukam, liest die
Werkzeuge aus dem Job selbst statt aus einer Liste. Er war schon vor dem neuen
Schritt rot: `tools/rust-pin.py` läuft im `sim`-Job („Rust-Pin lesen“,
„Gepinnter Compiler aktiv“) und stand nicht im Muster. Beide stehen jetzt drin.

Dass der Schritt auch angeschlossen bleibt, prüft ein zweiter Fall derselben
Datei am Schritt selbst: genau einmal im Job (Kommentarzeilen zählen nicht),
nur die Schlüssel `name`, `id`, `run` und `timeout-minutes`, und genau dieser
Befehl ohne Zusatz. Seine erste Fassung suchte nur die Befehlszeile und blieb
grün, als der Schritt auskommentiert, mit `if: false` übersprungen, mit
`continue-on-error: true` entschärft oder sein Ausgang mit `|| true`
verschluckt war. Nicht erfasst ist das Abschalten des ganzen `sim`-Jobs: Mit
`if: false` am Job blieben alle Fälle der Datei grün (gemessen am 2026-10-24),
und `ci-pass` wertet `skipped` als bestanden.

## Die Gegenrichtung: wird eine Stufe von nichts ausgelöst?

**Nein.** Jede der zehn Stufen hat einen erreichbaren Auslöser, und für `sim`
sind es seit dem 2026-10-24 neun Pfadmuster. Eine Stufe, die nie läuft, sähe in
der Übersicht aus wie eine, die immer grün ist — die Frage gehört deshalb
gestellt, auch wenn die Antwort hier beruhigend ist.

Die zusätzlichen TypeScript-Jobs laufen bei einer reinen `tools/*.py`-Änderung
weiterhin. Die Python-Änderung selbst ist jetzt durch die Werkzeugprüfung in
`lint` abgedeckt.

## Was hier nicht gemessen ist

- **Ob die Stufen tun, was ihr Name sagt.** Diese Seite sagt nur, ob sie
  _laufen_.
- **`abnahme-reihenfolge.yml`** löst auf `issues: [closed]` aus, nicht auf
  Dateien — es steht deshalb in keiner Zeile oben.
- **`meta.yml`** (`pr-title`) hängt am PR, nicht am Pfad.
- **`tools/ci-pfadfilter.sh` selbst** fährt keine Stufe. Es fällt unter denselben
  Befund wie die 20 anderen Werkzeuge (#1100) — wer es ändert, prüft es von Hand.
