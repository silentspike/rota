---
id: ci-pfadfilter
status: Stable
ssot: true
refs:
  docs: [WORKER-GUIDE.md]
affects: [.github/workflows]
---

# Welche Änderung löst welche CI-Stufe aus?

Erhoben am 2026-08-22 für Issue #1100, AC4. Gelesen aus
`.github/workflows/`, ohne Lauf.

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

| Ausgang | Muster                                                                                              |
| ------- | --------------------------------------------------------------------------------------------------- |
| `code`  | alles, was **nicht** auf `^(docs/\|.*\.md)$` passt                                                  |
| `sim`   | `^(sim/\|data/\|tests/parity/recordings/\|Cargo\.toml$\|Cargo\.lock$\|\.github/workflows/ci\.yml$)` |
| `rules` | dasselbe wie `code`                                                                                 |

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
| **`tools/*.py`**                          | true     | false | typecheck unit build parity |
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

### 2. `tools/` löst keine Stufe aus, die es prüft

Eine Änderung an einem der 21 Skripte startet `typecheck`, `unit`, `build` und
`parity` — keine davon sieht Python an. Ausnahme ist `tools/step1-check.py`, das
`parity` als ersten Schritt fährt.

Das ist der Kern von #1100 und hier nur der Vollständigkeit halber vermerkt.

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

## Die Gegenrichtung: wird eine Stufe von nichts ausgelöst?

**Nein.** Jede der zehn Stufen hat einen erreichbaren Auslöser, und für `sim`
sind es sechs Pfadmuster. Eine Stufe, die nie läuft, sähe in der Übersicht aus
wie eine, die immer grün ist — die Frage gehört deshalb gestellt, auch wenn die
Antwort hier beruhigend ist.

Was es **gibt**, ist die Umkehrung: Stufen, die laufen und nichts ansehen. Bei
einer reinen `tools/*.py`-Änderung sind das vier von zehn.

## Was hier nicht gemessen ist

- **Ob die Stufen tun, was ihr Name sagt.** Diese Seite sagt nur, ob sie
  _laufen_.
- **`abnahme-reihenfolge.yml`** löst auf `issues: [closed]` aus, nicht auf
  Dateien — es steht deshalb in keiner Zeile oben.
- **`meta.yml`** (`pr-title`) hängt am PR, nicht am Pfad.
- **`tools/ci-pfadfilter.sh` selbst** fährt keine Stufe. Es fällt unter denselben
  Befund wie die 20 anderen Werkzeuge (#1100) — wer es ändert, prüft es von Hand.
