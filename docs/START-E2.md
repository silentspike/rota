# Startbriefing — Epic E2: Simulator (Rust)

> **Diesen Text in die Simulator-Session kopieren.**
> Läuft parallel zu E1 (Kampf-Kern) und E9 (Grafik) und berührt deren Dateien mit keiner Zeile.

---

## Dein Auftrag

Du baust **Epic E2 — den Simulator**: eine eigenständige Rust-Bibliothek unter `sim/`, die dieselben
Regeln rechnet wie das Spiel, nur ereignisgesteuert und tausendfach schneller.

**Er läuft nie beim Spieler.** Das Spiel selbst braucht überhaupt keine Simulation — es rechnet nur
aus, was tatsächlich gewirkt wurde. Der Simulator ist ein reines Entwicklungs-Werkzeug für die
**Balance**: Schadensverhältnisse, Zauberstärke, Talente, Ausrüstung.

**Warum du jetzt startest, obwohl E1 noch läuft:** Du arbeitest ausschließlich in `sim/`. Die
Regeln, gegen die du baust, sind seit Issue #9 und #11 abgenommen und geprüft — du baust gegen
etwas Fertiges, nicht gegen eine Vermutung.

---

## Zuerst lesen — in dieser Reihenfolge

| #   | Datei                                   | Warum                                                                      |
| --- | --------------------------------------- | -------------------------------------------------------------------------- |
| 1   | `CLAUDE.md`                             | Die sechs Design-Gesetze und die technischen Rahmenbedingungen             |
| 2   | `docs/GAME-DESIGN.md` **18.4 und 18.5** | Determinismus und Simulator — dein Kern                                    |
| 3   | `docs/STEP1-SPEC.md`                    | Die Zahlen und die beiden Prioritätenlisten                                |
| 4   | `tools/step1-check.py`                  | **Die Referenz.** Dein Simulator ist die dritte Umsetzung derselben Regeln |
| 5   | `docs/WORKER-GUIDE.md`                  | Der verbindliche Ablauf je Issue                                           |

> `GAME-DESIGN.md` Kapitel 21 (verworfen) und 22 (noch nicht ausgeplant) gelten auch für dich.
> Kapitel 22 nennt unter anderem die 60 Zauber und die Talent-Inhalte — die gibt es noch nicht,
> und du sollst sie nicht erfinden.

---

## Dein Arbeitsplatz

```bash
cd /work/rota-worktrees/e2
git fetch origin
git switch wt-e2
git reset --hard origin/main
```

`wt-e2` ist der **Ruhe-Branch** — auf ihm wird nie gearbeitet. Für jedes Issue legst du davon einen
Feature-Branch an.

**Dein Dateibereich: `sim/`.** Sonst nichts. Zwei Ausnahmen, beide klein und angekündigt:

- `.github/workflows/ci.yml` — der `sim`-Job existiert bereits und läuft, sobald `sim/**` berührt
  wird. Wenn er angepasst werden muss, melde es
- `tests/parity/` — dort liegen die gemeinsamen Prüffälle. Die TypeScript-Seite gehört E1; du
  ergänzt die Rust-Seite

---

## Die fünf Issues, in dieser Reihenfolge

| #       | Issue                                                         | Warum diese Stelle                                                               |
| ------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| **#53** | E2-1: Regel-Bibliothek, ereignisgesteuert und deterministisch | Der Kern. Alles Weitere setzt darauf                                             |
| **#54** | E2-2: Gemeinsame Prüffälle — Simulator gegen Spiel            | **Der harte Prüfstein.** Ab hier ist bewiesen, dass du dasselbe Spiel simulierst |
| **#55** | E2-3: APL-Sprache und ihre Auswertung                         | Erst mit ihr lassen sich Rotationen beschreiben statt zu durchsuchen             |
| **#56** | E2-4: Balance-Läufer mit Skalierungsfaktoren                  | Braucht die APL                                                                  |
| **#57** | E2-5: Zauber-Vergleich, Boss-Machbarkeit, Erfolgs-Prüflauf    | Die drei Auswertungen, für die das alles gebaut wird                             |

> **#54 ist deine wichtigste Hürde.** Wenn dein Simulator für die gute Rotation nicht exakt
> **4240 Schaden bei 40510 ms** ergibt, stimmt eine Regel nicht — und zwar bevor darauf aufgebaut wird.

---

## Die sieben Regeln, an denen der Determinismus hängt

Aus `GAME-DESIGN.md` 18.4. Jede einzelne davon ist schon einmal jemandem zum Verhängnis geworden.

1. **Ganzzahlige Zeit in Millisekunden.** Abklingzeit 8 s bei Tempo-Faktor 1,17 ergibt krumme Werte;
   Fließkomma-Timer driften über hunderte Ticks
2. **Getrennte Zufallsströme je Quelle** (Krit, DoT-Tick, Heilung). Ein gemeinsamer Strom bedeutet:
   Eine zusätzliche Ziehung verschiebt alles Nachfolgende
3. **Feste Ereignis-Reihenfolge, keine Hash-Container.** Sonst weichen Spiel und Simulator ab —
   und man sucht tagelang
4. **Parallel zwischen Durchgängen, nie innerhalb.** Ein Schritt hängt vom vorherigen ab
5. **Ergebnisse nach Startwert sortieren vor dem Zusammenzählen.** Fließkomma-Addition ist nicht
   assoziativ; sonst schwankt der Mittelwert je nach Laune des Betriebssystems
6. **Keine Speicheranforderung im Schritt**, zusammenhängende Listen
7. **Daten einkompiliert** statt zur Laufzeit geladen

---

## Bauen: bitte remote

Für Rust-Projekte unter `/work` gilt: **remote bauen**, um den Laptop zu entlasten (16 GB RAM).
Seit 2026-10-23 ist der Bau-Dienst **cargo remote v2** (`rustbuild-ovh` auf node3). Jeder Aufruf
startet in der Wurzel des Repositorys, nicht in `sim/`:

```bash
cargo remote -- test                                       # wie die CI
cargo remote -- test --all-targets --features diagnostics  # wie die CI
cargo remote -- clippy --all-targets -- -D warnings        # wie die CI
cargo-remote-fmt --check             # wie die CI, vor jedem Push
cargo-remote-fmt                     # formatiert und schreibt zurück — NICHT `cargo remote -- fmt`
cargo remote -c -- build --release   # Programm nach target/release/rota-sim
```

> **`cargo remote -- fmt` formatiert nur die Kopie auf dem Server**, lokal ändert sich nichts. Die
> Änderungen landen dann nie im Repo, und die CI-Stufe `fmt` kippt. Gemessen am 2026-10-23.

Der frühere offene Punkt — SSH zum alten Bau-Server CT 155 abgelehnt, geprüft am 2026-07-28 — ist
mit der Umstellung gegenstandslos: ROTA benutzt den alten Dienst nicht mehr.

**Was das für dich heißt:** Schlägt ein Aufruf fehl, **melde es und weiche nicht stillschweigend
auf einen lokalen Bau aus** — die Remote-Regel existiert aus einem Grund. Lokal sperrt ein Wächter
Rust (`cargo metadata` endete am 2026-10-23 mit „LOKALES RUST IST AUF DIESEM GERAET GESPERRT");
ob der Laptop das tragen soll, entscheidet der Auftraggeber, nicht du. **Exit 75** ist dabei kein
Fehler im Code: Der Dienst hatte keinen Platz oder in deinem Arbeitsverzeichnis lief schon ein Job,
und gebaut wurde nichts — `cargo remote --status` ansehen, dann erneut.

---

## Woran die Master-Session dich messen wird

Der Simulator hat kein HUD, deshalb entfallen die Bild-Messungen aus `VERIFICATION.md` § 3.1 bis
§ 3.2. Stattdessen:

| Prüfung        | Kriterium                                                                                                                  |
| -------------- | -------------------------------------------------------------------------------------------------------------------------- |
| **Gleichlauf** | Gute Rotation exakt **4240** Schaden, Sieg bei **40510 ms** — dieselben Werte wie `tools/step1-check.py` und wie das Spiel |
| Determinismus  | Zwei Läufe mit gleichem Startwert byteweise gleich                                                                         |
| Zufallsströme  | Eine zusätzliche Krit-Ziehung verschiebt die DoT-Tick-Folge nicht                                                          |
| Parallelität   | Lauf über N Kerne ergibt dasselbe wie über einen                                                                           |
| Speicher       | Keine Anforderung im Schritt (Zähl-Allocator)                                                                              |
| Clippy         | `--all-targets -- -D warnings` sauber                                                                                      |

Die Sollwerte stehen in `docs/STEP1-SPEC.md` § 4. **Maßgeblich ist aber immer der frische Lauf von
`python3 tools/step1-check.py`**, nicht die Tabelle — sie kann veralten, das Skript nicht.

---

## Wie du meldest

Format in `docs/WORKER-GUIDE.md` § 6: was gebaut, welche Befehle mit welcher **tatsächlichen**
Ausgabe, was **nicht** geprüft wurde und warum, Zuversicht in Prozent.

Der Abschnitt „Was NICHT geprüft wurde" bleibt nicht leer.

---

## Drei Sätze, die für dich besonders gelten

1. **Der Simulator ist die dritte Umsetzung derselben Regeln.** Python-Referenz, TypeScript-Spiel,
   Rust-Simulator. Weicht deiner ab, ist das ein starker Hinweis — auf einen Fehler bei dir **oder**
   in einer der anderen beiden. Melde die Abweichung, statt sie wegzurechnen.
2. **Die gefundenen Prioritätenlisten erscheinen nirgends im Spiel** (GAME-DESIGN 21.2), auch nicht
   als Lernhilfe. Das Herausfinden der Reihenfolge ist der Kern des Spiels.
3. **Bei einer Design-Lücke: fragen, nicht raten.** Kapitel 22 listet auf, was bewusst offen ist.
