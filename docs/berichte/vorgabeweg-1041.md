---
id: vorgabeweg-1041
status: Review
ssot: false
refs:
  docs:
    [
      berichte/paar-blindheit-558.md,
      berichte/pruefstein-509.md,
      berichte/e607-zauberkasten.md,
      SIM-MESSUNGEN.md,
    ]
---

# Der Vorgabeweg-Wechsel aus #797 bewegt keine Ergebniszahl — nur die Zahl der gemessenen Listen

**Issue #1041, deterministischer Teil.** Gemessen am 2026-10-24 von Worker-1 auf
`origin/main` = `23d7c13fba470ee1d6a677bfdd64d042f05724f6`. Release-Bau über
cargo remote v2 (`linux-gnu-bookworm`, Rust 1.98.1), gefahren auf dem Laptop.

Die beiden Zeitläufe `orch-verteilung` und `messumgebung` gehören **nicht** in
diesen Schritt: **folgt am Wellenende im Messfenster** (Abschnitt „Zeitläufe").

## Das Ergebnis

1. **Keine Ergebniszahl hat sich bewegt.** In 27 Läufen über 19 Beispiele — die
   17 deterministischen des festgelegten Umfangs, acht Zusatzaufrufe mit
   Schaltern (sechs aus den Berichten zu Prüfstein-509 und e607, zwei Modi aus
   den Dateien selbst) und zwei nachträglich gefundene Beispiele — ist kein
   Schadenswert, keine Rotation, keine Zauberliste, kein Abbruchgrund und kein
   Exit-Code anders.
2. **Bewegt hat sich genau eine Zahlart: die Zahl der gemessenen Listen.**
   Sichtbar nur in Läufen mit Länge 12 — `rotation-search --max-len 12`
   (Prüfstein-509 und e607) und ein Abschnitt in `pruefstein-558` —, je
   betroffener Suche um +21 bis +30. Das sind Dreiergruppen, die **geprüft wurden
   und verloren haben**. Die Suche hat weder etwas Besseres noch etwas anderes
   gefunden: dieselbe Liste, derselbe Wert, mehr Messungen.
3. **Keine Gruppe hat irgendwo den Ausschlag gegeben.** Die Spur-Fassung zählt
   jeden der 2671 Suchaufrufe dieser 27 Läufe: 2599 liefen mit eingeschalteten
   Gruppen, 86 davon haben Gruppen geprüft (258 Gruppen) — **0** Entscheidungen,
   **0** gewählte Gruppen. Geprüft wurde auch bei Länge 6 und 8; dort drucken die
   Beispiele aber keine Messzahl, und ihre Ausgaben bleiben bitgleich.
4. **Die Mutation kommt an.** Unter B wird
   `ueber_greedy_wendet_die_suche_den_toedlichen_schlag_ab` rot, und die
   Gewichtsreihe fällt von `[1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0]` auf `[]`.
   „Nichts bewegt" heißt also nicht „die Änderung erreichte die Beispiele nicht".

> Kriterium 3 des Issues, in derselben Deutlichkeit: **Ein Vorgabeweg-Wechsel,
> der neunzehn Beispiele inhaltlich nicht bewegt.** Das liegt an der Bauart der
> Suche, nicht am Zufall — siehe „Warum sich nichts bewegt".

Ein Nebenbefund wiegt schwerer als die Zahlen hier: **`suchguete-erweitert`
liefert im Bestand gar kein Ergebnis** — Exit 101 in Fall 1 von 60, in A und B
gleich. Siehe „Befunde am Rand".

## Die Fassungen

| Fassung | Inhalt                                                         | sha256 von `sim/src/rotation_search.rs`                            |
| ------- | -------------------------------------------------------------- | ------------------------------------------------------------------ |
| A       | Bestand                                                        | `6e93744a3ea3ae642285e3e080575d85cfe3069fd5da45f42bdba4ccd6846819` |
| B       | in `greedy_opts`: `tripel: mit_paaren` → `tripel: false`       | `6f27bbe2be268b161d2748dc74e4f5d89ee5eca86627d39831825407035e8a86` |
| A′      | A plus eine Zeile auf stderr je Suchaufruf (Spur, siehe unten) | `da6064b52dc79cde88a8d20d4058172795ddce347a54917f15bfa97397e67351` |

**B und A′ standen nur im Worktree und sind nicht committet.** Der Rückbau ist
zweimal belegt — nach B (Zeile zurückgesetzt) und nach A′ (`git checkout`): je
sha256 von `rotation_search.rs`, `Cargo.lock` und `.cargo-remote.toml` gegen den
Ausgangszustand verglichen (`diff` Exit 0), `git status --porcelain` leer.

**B trifft genau den Vorgabeweg:** `greedy` → `greedy_from` →
`greedy_opts(…, true)` → `greedy_zuege(Zuege { paare: mit_paaren, tripel: mit_paaren })`.
Wer `greedy_zuege` selbst aufruft, bleibt unberührt — das tut kein Beispiel und
kein Programm (`grep -rlwE greedy_zuege sim/examples sim/src/bin`: kein Treffer).
Aufrufe mit `greedy_opts(…, false)` bleiben ebenfalls gleich; sie laufen schon im
Bestand ohne Paare und ohne Gruppen.

**A′ ist die Trägerzeile.** Gleiche Ausgaben in A und B zeigen, dass sich nichts
bewegt — nicht, ob der Gruppen-Zweig überhaupt gefragt wurde. A′ schreibt am Ende
von `greedy_zuege` je Aufruf eine Zeile `SPUR1041` nach stderr: Schalter,
`max_len`, Länge von Start- und Ergebnisliste, Abbruch, `measured`,
`paare_geprueft`, `tripel_geprueft` und zwei Zähler, die es im Bestand nicht gibt
— Runden, in denen eine Gruppe den Ausschlag gab (`tripel_gewinnt`), und
eingefügte Gruppen. **Die Spur verändert kein Ergebnis:** stdout A′ = A in 26 von
27 Läufen bitgleich, im 27. (`e1-1088-suchraum`) nach Ausklammern der
Zeitspalte.

## Bau, Rückkopie, Reproduzierbarkeit

| Bau                                                         | Job                        | Ergebnis                 |
| ----------------------------------------------------------- | -------------------------- | ------------------------ |
| A, `cargo remote -c -- build --release --examples`          | `56c0be29e3b01612677e2bc1` | Exit 0, 36 Binaries      |
| A, `messumgebung` laut Dateikopf (`--features diagnostics`) | `fecf2f527e7ba20db06f7527` | Exit 0                   |
| B, alle Beispiele                                           | `320ee0538a591577376854d4` | Exit 0, 36 Binaries      |
| B, `messumgebung` mit `diagnostics`                         | `6788a5f3a9cfa27e1d9832b2` | Exit 0                   |
| A, Neubau nach dem ersten Rückbau                           | `307549e3c7456584de07e656` | 36 von 36 bitgleich zu A |
| A′, alle Beispiele                                          | `8f16ec362e3d46c29bc6a298` | Exit 0                   |

Jede Zeile trägt im Protokoll `cargo-remote: job <id> | linux-gnu-bookworm | Rust 1.98.1`.

**`-c` holt die Beispiel-Binaries unter v2 zurück — gemessen, nicht angenommen.**
Vor dem ersten Bau gab es im Worktree kein `target/`. Danach lagen 36 Binaries
unter `target/release/examples/`, mtime 04:20:27–04:20:32, innerhalb des
Baulaufs 04:20:08–04:20:37. Nach dem B-Bau trugen alle 36 neue mtimes
(04:21:47–04:21:52, Baulauf 04:21:39–04:21:55). Der Diagnose-Bau ersetzte genau
`messumgebung` (sha256 `4d5dce5e…` → `50573ad8…`).

**Der Bau ist reproduzierbar.** Nach dem ersten Rückbau neu gebaut, waren alle 36
Binaries bitgleich zum ersten A-Bau, bei neuer mtime. Das belegt den Rückbau ein
zweites Mal, diesmal am Erzeugnis.

**Ein abweichendes Binary ist kein Erreichbarkeitsbeleg.** Zwischen A und B
unterscheiden sich 28 Binaries, nicht 22. Sechs davon — `apl`, `balance`,
`e21-0b-machbarkeit`, `orch-subsumption`, `report`, `wertungsmenge-981` —
enthalten kein einziges Token der `greedy`-Familie. Da der Bau reproduzierbar
ist, ist das Code-Erzeugung der geänderten Bibliothek, kein Ausführungspfad. Wer
„betroffen" am Binärvergleich abliest, zählt sechs zu viel.

## Umfang und Erreichbarkeit

**Umfang laut Festlegung des Orchestrators:** die 16 Beispiele, die beim Merge von
PR #1037 (`ef368c48`, 2026-08-21 21:35) `greedy(` enthielten — genau die 16 aus
der Messung von Worker-3 —, dazu `suchguete`, `suchguete-erweitert` und
`tote-zauber-419`, die über `greedy_opts` bzw. `greedy_from` laufen.

Zeilen beziehen sich auf `sim/examples/<name>.rs` im Stand `23d7c13f`.

| Beispiel                  | Aufrufkette von `main`                                                                                                                                                                                                                                                                                                                                 | Klasse          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------- |
| `ac3-umgebaute-wege-419`  | `main` → `greedy` (107); `main` → `greedy_from` (185, je Weg)                                                                                                                                                                                                                                                                                          | deterministisch |
| `buff-ablaeufe`           | `main` → `greedy` (96 und 188, je Boss)                                                                                                                                                                                                                                                                                                                | deterministisch |
| `doomsayer-672`           | `main` → `greedy` (100)                                                                                                                                                                                                                                                                                                                                | deterministisch |
| `gewicht-reihe-419`       | `main` → `messen` → `greedy` (129, je Gewicht)                                                                                                                                                                                                                                                                                                         | deterministisch |
| `kandidaten-homogenitaet` | `main` → `greedy` (91)                                                                                                                                                                                                                                                                                                                                 | deterministisch |
| `orch-trace-stabil`       | `main` → `greedy` (118)                                                                                                                                                                                                                                                                                                                                | deterministisch |
| `resistance-rotation`     | `main` → `greedy` (51, je Boss)                                                                                                                                                                                                                                                                                                                        | deterministisch |
| `resistance-threshold`    | `main` → `greedy` (54, je Widerstandsstufe)                                                                                                                                                                                                                                                                                                            | deterministisch |
| `rotation-search`         | `main` → `greedy` (606, je Boss); `main` → `nachlauf` → `nachlauf_runde` → `greedy_from` (269)                                                                                                                                                                                                                                                         | deterministisch |
| `trace-stabilitaet`       | `main` → `greedy` (405); mit `--folgen`: `main` → `folgen_drucken` → `greedy` (321)                                                                                                                                                                                                                                                                    | deterministisch |
| `trace-wiederverwendung`  | `main` → `greedy` (107)                                                                                                                                                                                                                                                                                                                                | deterministisch |
| `zauber-abdeckung`        | `main` → `erhebe` → `greedy` (162)                                                                                                                                                                                                                                                                                                                     | deterministisch |
| `pruefstein-558`          | `main` → `durchsatz` → `greedy` (182); `main` → `ac1_an_worker1s_zahl` → `greedy_opts(…, true)` (294, 321, 360, 413), `greedy_from` (375, 442); `main` → `traegt_das_paar_ueberhaupt` → `alle_verstaerker` → `verstaerker_in_bester_umgebung` → `greedy` (692), `greedy_from` (714); `main` → `paar_suche_gegen_einzel` → `greedy_opts(…, true)` (786) | deterministisch |
| `stufen-durchsatz`        | `main` → `durchsatz` → `greedy` (191); `main` → `greedy` (230)                                                                                                                                                                                                                                                                                         | deterministisch |
| `suchguete`               | `main` → `greedy_opts(…, true)` (220)                                                                                                                                                                                                                                                                                                                  | deterministisch |
| `suchguete-erweitert`     | `main` → `greedy_opts(…, true)` (498); `main` → `lns` → `greedy_opts(…, true)` (274, 294)                                                                                                                                                                                                                                                              | deterministisch |
| `tote-zauber-419`         | `main` → `greedy_from` (74, je Kandidatenzeile und Gewicht)                                                                                                                                                                                                                                                                                            | deterministisch |
| `orch-verteilung`         | `main` → `greedy` (70)                                                                                                                                                                                                                                                                                                                                 | Zeitlauf        |
| `messumgebung`            | `main` → `suche` (2049, 2784) → `greedy` (807); `suche` → `lns` (816) → `greedy` (544, 588)                                                                                                                                                                                                                                                            | Zeitlauf        |

Die Aufrufe mit `greedy_opts(…, false)` — `pruefstein-558` Z. 293, 412, 785,
`suchguete` Z. 219, `suchguete-erweitert` Z. 496 — sind erreichbar, aber von B
nicht berührt.

**Zusätzlich gefunden, nicht im festgelegten Umfang.** Drei Beispiele entstanden
am 2026-08-22, also nach dem Wechsel, und erreichen den Vorgabeweg ebenfalls:

| Beispiel             | Aufrufkette von `main`                                                                   | Behandlung                   |
| -------------------- | ---------------------------------------------------------------------------------------- | ---------------------------- |
| `e1-1085-doom-suche` | `main` → `messen` → `greedy` (94)                                                        | zusätzlich gefahren          |
| `e1-1088-suchraum`   | `main` → `greedy` (75)                                                                   | zusätzlich gefahren          |
| `e15-0-faehigkeiten` | `main` → `feste_rotation` (284) → `greedy` (118); `main` → `miss` (358) → `greedy` (359) | Zeitwerkzeug, nicht gefahren |

## Ergebnisse je Lauf

Gefahren aus der Worktree-Wurzel, `trace-stabilitaet` aus `sim/` heraus, wie in
den Dateiköpfen. Je Beispiel liefen A und B gleichzeitig, A auf den Kernen 0–7,
B auf 8–15 (`nice -n 10 taskset -c …`), damit die Fremdlast beide gleich traf —
außer `e1-1085-doom-suche`, das als erster Probelauf A und B nacheinander auf
allen Kernen fuhr. „bitgleich" heißt: stdout und stderr per `cmp` identisch.

Die Spalte **Spur A′** nennt: Suchaufrufe · davon mit Gruppenprüfung ·
Entscheidungen durch eine Gruppe.

| Lauf                          | Schalter                                                | Exit A / B | Vergleich A ↔ B                                    | Spur A′      |
| ----------------------------- | ------------------------------------------------------- | ---------- | -------------------------------------------------- | ------------ |
| `ac3-umgebaute-wege-419`      | —                                                       | 0 / 0      | bitgleich, 34 Zeilen                               | 10 · 0 · 0   |
| `buff-ablaeufe`               | —                                                       | 0 / 0      | bitgleich, 23 Zeilen                               | 6 · 0 · 0    |
| `doomsayer-672`               | —                                                       | 0 / 0      | bitgleich, 68 Zeilen                               | 1 · 0 · 0    |
| `gewicht-reihe-419`           | —                                                       | 0 / 0      | bitgleich, 153 Zeilen                              | 36 · 0 · 0   |
| `kandidaten-homogenitaet`     | —                                                       | 0 / 0      | bitgleich, 37 Zeilen                               | 1 · 0 · 0    |
| `orch-trace-stabil`           | —                                                       | 0 / 0      | bitgleich, 24 Zeilen                               | 1 · 0 · 0    |
| `resistance-rotation`         | —                                                       | 0 / 0      | bitgleich, 30 Zeilen                               | 2 · 0 · 0    |
| `resistance-threshold`        | —                                                       | 0 / 0      | bitgleich, 11 Zeilen                               | 7 · 0 · 0    |
| `rotation-search`             | —                                                       | 0 / 0      | bitgleich, 149 Zeilen                              | 4 · 0 · 0    |
| `rotation-search`             | `--bosses` Prüfstein-509 (5 Bosse)                      | 0 / 0      | bitgleich, 211 Zeilen                              | 25 · 0 · 0   |
| `rotation-search`             | `--bosses` Prüfstein-509 `--max-len 12`                 | 0 / 0      | **4 Zeilen anders**: nur Messzahlen                | 65 · 10 · 0  |
| `rotation-search`             | `--bosses` Prüfstein-509 `--max-len 12 --kein-nachlauf` | 0 / 0      | **4 Zeilen anders**: nur Messzahlen                | 5 · 4 · 0    |
| `rotation-search`             | `--bosses` e607 (17 Bosse) `--max-len 6`                | 0 / 0      | bitgleich, 542 Zeilen                              | 561 · 0 · 0  |
| `rotation-search`             | `--bosses` e607 `--max-len 8`                           | 0 / 0      | bitgleich, 578 Zeilen                              | 561 · 0 · 0  |
| `rotation-search`             | `--bosses` e607 `--max-len 12`                          | 0 / 0      | **8 Zeilen anders**: nur Messzahlen                | 833 · 48 · 0 |
| `trace-stabilitaet`           | —                                                       | 0 / 0      | bitgleich, 440 Zeilen                              | 36 · 3 · 0   |
| `trace-stabilitaet`           | `--folgen`                                              | 0 / 0      | bitgleich, 15 Zeilen                               | 12 · 1 · 0   |
| `trace-wiederverwendung`      | —                                                       | 0 / 0      | bitgleich, 78 Zeilen                               | 1 · 0 · 0    |
| `zauber-abdeckung`            | —                                                       | 0 / 0      | bitgleich, 11 Zeilen, „BESTANDEN: 11 von 58"       | 4 · 0 · 0    |
| `zauber-abdeckung`            | `--toepfe-einzeln --mit-ausruestung`                    | 0 / 0      | bitgleich, 47 Zeilen                               | 48 · 2 · 0   |
| `pruefstein-558`              | —                                                       | 0 / 0      | **1 Zeile anders**: nur eine Messzahl              | 81 · 7 · 0   |
| `stufen-durchsatz`            | —                                                       | 0 / 0      | bitgleich, 88 Zeilen                               | 54 · 11 · 0  |
| `suchguete`                   | —                                                       | 0 / 0      | bitgleich, 33 Zeilen                               | 120 · 0 · 0  |
| `suchguete-erweitert`         | —                                                       | 101 / 101  | stdout bitgleich (14 Zeilen); stderr nur Thread-ID | 5 · 0 · 0    |
| `tote-zauber-419`             | —                                                       | 0 / 0      | bitgleich, 136 Zeilen                              | 186 · 0 · 0  |
| `e1-1085-doom-suche` (Zusatz) | —                                                       | 0 / 0      | bitgleich, 77 Zeilen                               | 4 · 0 · 0    |
| `e1-1088-suchraum` (Zusatz)   | —                                                       | 0 / 0      | nur die Zeitspalte `Dauer` anders                  | 2 · 0 · 0    |

Die vollständigen Bosslisten: Prüfstein-509 nach `docs/berichte/pruefstein-509.md`
(`first-crucible,test-onslaught,test-metronome,test-gatekeeper,test-resistant`),
e607 nach `docs/berichte/e607-zauberkasten.md` (`boss-1-1` bis `boss-4-4` und
`first-crucible`).

**Ausgeklammert und benannt** — nur diese zwei Stellen hängen an der Uhr oder am
Prozess:

- `e1-1088-suchraum`: die Spalte `Dauer … s` (Wanduhr). Ersetzt durch
  `Dauer <ZEIT> s` (zwei Treffer, also beide Zeilen gefasst) sind A, B und A′
  bitgleich: `geprueft 2294` / `3944`, `Schaden 6836.0` / `6965.0`.
- `suchguete-erweitert`: die Thread-ID im Panic-Kopf (`thread 'main' (24163)`
  gegen `(24162)`). Maskiert (je ein Treffer) ist stderr in A, B und A′ gleich.

## Die bewegten Zahlen

Kriterium 2 des Issues: welche Zahl, um wie viel, und ob die Suche etwas
Besseres oder etwas anderes gefunden hat.

**`rotation-search`, Prüfstein-509, `--max-len 12`** (mit und ohne Nachlauf
dieselben vier Zeilen):

| Boss              | Wert A = B | Messungen A | Messungen B | Differenz | Abbruch       |
| ----------------- | ---------: | ----------: | ----------: | --------: | ------------- |
| `first-crucible`  |     9742.0 |        5142 |        5115 |       +27 | NoImprovement |
| `test-onslaught`  |     9014.0 |        3241 |        3220 |       +21 | NoImprovement |
| `test-metronome`  |    61115.0 |        8294 |        8294 |        +0 | LengthLimit   |
| `test-gatekeeper` |    20058.0 |        6245 |        6215 |       +30 | NoImprovement |
| `test-resistant`  |    38337.0 |        4140 |        4116 |       +24 | NoImprovement |

**`rotation-search`, e607, `--max-len 12`** — 8 von 17 Bossen:

| Boss             | Wert A = B | Messungen A | Messungen B | Differenz |
| ---------------- | ---------: | ----------: | ----------: | --------: |
| `boss-1-3`       |     9364.0 |        3241 |        3220 |       +21 |
| `boss-2-1`       |     9347.0 |        5142 |        5115 |       +27 |
| `boss-2-3`       |    11432.0 |        5142 |        5115 |       +27 |
| `boss-3-2`       |     9364.0 |        3241 |        3220 |       +21 |
| `boss-3-3`       |     9742.0 |        5142 |        5115 |       +27 |
| `boss-4-1`       |    11587.0 |        6207 |        6177 |       +30 |
| `boss-4-2`       |     9364.0 |        3241 |        3220 |       +21 |
| `first-crucible` |     9742.0 |        5142 |        5115 |       +27 |

Die übrigen neun Bosse zeigen +0: drei mit `LengthLimit` (`boss-1-1`,
`boss-3-4`, `boss-4-3`), zwei mit `NoImprovement` und zu voller Liste
(`boss-1-2` mit 11, `boss-2-2` mit 10 Zeilen) und vier mit `NoImprovement`, bei
denen Platz war, aber keine Gruppe gebildet wurde (`boss-1-4`, `boss-2-4`,
`boss-3-1`, `boss-4-4`).

**`pruefstein-558`**, Abschnitt „voller Kandidatenraum (93 Kandidatenzeilen,
Laenge 12)":

```text
A   mit Paaren      9742   (5142 Messungen, 234 Paare geprueft, Abbruch NoImprovement)
B   mit Paaren      9742   (5115 Messungen, 234 Paare geprueft, Abbruch NoImprovement)
```

**Besser oder anders? Weder noch.** Wert, Liste, Abbruchgrund, Paarzahl,
Kreuzvergleich und Nachlauf-Runden sind gleich; es steigt nur, was die Suche
kostet.

**Die Differenz ist genau eine verlorene Gruppenrunde**, an der Spur
nachgerechnet: drei Gruppen (`tripel_geprueft = 3`) mal `Zeilen + 1`
Einfügestellen. Für den Lauf ohne Nachlauf:

```text
Zeilen  Abbruch        tripel_geprueft  3 × (Zeilen + 1)  gemessene Differenz
   8    NoImprovement         3                27                +27
   6    NoImprovement         3                21                +21
  12    LengthLimit           0                 0                 +0
   9    NoImprovement         3                30                +30
   7    NoImprovement         3                24                +24
```

## Warum sich nichts bewegt

Die Gruppen kommen in `greedy_zuege` nur unter einer Bedingung zur Wahl:

```text
if !einzeln_traegt && zuege.tripel && chosen.len() + 3 <= max_len
```

Kein Einzelzug trägt mehr, **und** es sind noch drei Plätze frei. Der
Doc-Kommentar zu `SearchResult::tripel_geprueft` nennt drei Gründe für „kein
Unterschied" — geprüft und verloren, Liste zu voll, eine der drei Mengen leer.
Die Spur ordnet jeden der 2671 Suchaufrufe einer dieser Klassen zu:

| Klasse                                                               | Aufrufe |
| -------------------------------------------------------------------- | ------: |
| ohne Gruppen-Schalter (`greedy_opts(…, false)`), von B nicht berührt |      72 |
| Einzel- oder Paarzüge trugen bis zur Längengrenze (`LengthLimit`)    |    2244 |
| `NoImprovement`, aber keine drei Plätze mehr frei                    |     169 |
| `NoImprovement`, Platz frei, aber keine Gruppe gebildet              |     100 |
| Gruppen geprüft, nie den Ausschlag gegeben                           |      86 |
| eine Gruppe gab den Ausschlag                                        |   **0** |

**In elf der 19 Standardläufe endet jede einzelne Suche an der Längengrenze**,
auch bei `doomsayer-672` mit `MAX_LEN` 12: Die Einzelzüge tragen bis zur letzten
Zeile, die Gruppen kommen nie zur Wahl. `rotation-search` endet bei Länge 6 an
allen 17 e607-Bossen mit `LengthLimit`, bei Länge 8 an zwölf — dieselben Zahlen
wie in `e607-zauberkasten.md`.

**Geprüft und verloren haben Gruppen in acht Läufen**, nicht nur bei Länge 12:

| Lauf                                          | `max_len` | Zeilen beim Abbruch | Aufrufe mit Gruppenprüfung |
| --------------------------------------------- | --------: | ------------------: | -------------------------: |
| `zauber-abdeckung` mit Ausrüstung             |         6 |                   3 |                          2 |
| `stufen-durchsatz`                            |         8 |                   5 |                         11 |
| `trace-stabilitaet` (mit und ohne `--folgen`) |         8 |                   8 |                      3 + 1 |
| `pruefstein-558`                              |        12 |             6 bis 9 |                          7 |
| `rotation-search` Prüfstein-509, Länge 12     |        12 |             6 bis 9 |                  10 bzw. 4 |
| `rotation-search` e607, Länge 12              |        12 | 6 bis 9, dreimal 12 |                         48 |

In `trace-stabilitaet` und in drei e607-Aufrufen stand die Liste am Ende voll:
Dort verlor die Gruppe in einer Runde gegen einen Paarzug, danach füllte sich die
Liste.

**Sichtbar wird eine geprüfte Gruppe nur, wo das Beispiel `measured` druckt** —
die ersten drei Zeilen der Tabelle drucken es nicht, deshalb sind ihre Ausgaben
bitgleich, obwohl A dort mehr gemessen hat.

„Platz frei, aber keine Gruppe gebildet" heißt nach demselben Doc-Kommentar:
Eine der drei Mengen in `tripel_zuege` war leer. **Welches Glied jeweils fehlte,
hat diese Messung nicht erhoben.**

Die Positivkontrolle zeigt, wo eine Gruppe entscheidet: `test-lethal-strike` mit
`prevented_weight 1.0` und `max_len 10`. Dieser Boss und sein Zwilling kommen in
keinem Beispiel vor (`grep -rln test-lethal-strike sim/examples`: kein Treffer).

## Positivkontrollen

Remote als Prüffälle gefahren (deterministisch; Debug-Profil des Testlaufs).

| Kontrolle                                                                          | A                                                                        | B                                                                 |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------------- |
| `--test greedy_traegt_die_gruppen`                                                 | 2 passed (Job `21243067a077ad953fef8b3d`)                                | **1 passed, 1 failed**, Exit 101 (Job `1a8277f9a04496571e9c5f3b`) |
| `--test ac5_kontrolle_lohnt_sich -- --ignored die_gewichtsreihe_zeigt_ein_fenster` | trennt bei `[1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0]`, `["pin", "bind"]` | **`[]`**, bei keinem Gewicht eine FIXATION-Zeile                  |
| ganze Testfolge, `test --no-fail-fast`                                             | 1268 passed, 0 failed, 7 ignored                                         | 1266 passed, **2 failed**, 7 ignored                              |

Die zwei Jobs der Gewichtsreihe: A `61af206d66559ae376b50b7f`, B
`a6a980efce2061c30815fecd`. Die Testfolgen: A `607a0bfaa8e657cfab886207`, B
`f773e1251b53778db230ef7b`; gezählt über alle 203 `test result:`-Zeilen.

Unter B scheitert in `greedy_traegt_die_gruppen` genau der tödliche Fall
(„ueber greedy() ueberlebt der Spieler den Schlag nicht (7 Zeilen gefunden)"),
der Zwilling bleibt grün — so, wie der Prüffall es für diese Mutation
vorhersagt. **In der ganzen Testfolge bemerken den Wechsel genau zwei von 1268
Prüffällen**: dieser und `ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist`.

## Befunde am Rand

**1. `suchguete-erweitert` liefert im Bestand kein Ergebnis.** Es bricht in Fall 1
von 60 an seiner eigenen Zusicherung ab, in A und B gleich:

```text
thread 'main' panicked at sim/examples/suchguete-erweitert.rs:530:17:
assertion `left == right` failed: Fall 1: gleiche Werte, aber 46 Messungen gegen greedys 52 — dann laufen
die beiden verschiedene Wege zum selben Ergebnis, und die Zerlegung unten rechnet mit einer Gleichheit,
die es nicht gibt
```

Folgen: Die Ergebnistabelle wird nie gedruckt; eine Zahl, die sich bewegen
könnte, gibt es im Bestand nicht. Erreicht werden vor dem Abbruch Z. 496 und 498
(Fall 0 und 1) und Z. 274 (`lns`, Fall 0) — **Z. 294 nie**: `lns` bricht ab,
wenn die Startliste höchstens zwei Zeilen hat, und Fall 0 liefert zwei. Die
Tabelle „Alle fünf Verfahren" in `SIM-MESSUNGEN.md` (Messung 7) nennt ihren
Läufer nicht, trägt aber dieselbe Zusicherungszeile, die dieser Lauf vor dem
Abbruch druckt (`beam k=30 3480 in 156 Messungen`) — sie lässt sich auf `main`
nicht nachfahren.

Vermutete Ursache, **nicht gemessen**: #1088. Fall 1 läuft an `boss-1-2`, einem
doom-Boss; seit #1088 ergänzt `greedy_zuege` dort eine `purge`-Zeile, die
`beam_einfuegen` nicht kennt. Die Differenz 52 − 46 = 6 passt rechnerisch zu
genau einer zusätzlichen, nie gewählten Zeile über drei Runden bei `max_len` 3
(1 + 2 + 3). Den Stand vor #1088 habe ich nicht gebaut. Dafür gehört ein eigenes
Issue her; dieser Bericht ändert das Beispiel nicht.

**2. „mit Paaren" heißt seit #797 „mit Paaren und Dreiergruppen".** Die Zeilen
„mit Paaren" in `pruefstein-558` und „greedy mit Paaren" in `suchguete` und
`suchguete-erweitert` rufen `greedy_opts(…, true)`. Der Doc-Kommentar dort sagt
es selbst: „Seit #797 schaltet dieses Flag beides". Gemessen bewegt das heute
keine Zahl außer der einen Messzahl oben — aber die Bezeichnung beschreibt den
Stand zwischen #558 und #797.

**3. Worker-3s Debug-Messung vom 2026-08-21** (Nachricht 9842) ist im Release-Bau
bestätigt und vervollständigt: Die zwölf dort identischen Beispiele sind es auch
hier. `pruefstein-558` und `stufen-durchsatz` standen dort mit `exit=124` bei
900 s offen; hier liefen sie durch. `pruefstein-558` brauchte auch im Release-Bau
rund 1340 s Wandzeit — unter Fremdlast, also keine Leistungsangabe, aber ein
Hinweis für jede Zeitgrenze: 900 s hätten auch hier nicht gereicht.

## Zeitläufe — folgt am Wellenende im Messfenster

| Beispiel          | Warum nicht hier                                                                                       | Bau A (sha256)  | Bau B (sha256)  |
| ----------------- | ------------------------------------------------------------------------------------------------------ | --------------- | --------------- |
| `orch-verteilung` | misst Kämpfe je Sekunde; der Inhalt hängt an einer per `greedy` gefundenen Rotation                    | `75206e6d4d3b…` | `d04e841c223a…` |
| `messumgebung`    | Durchsatz und Druckprüfung; Bau laut Dateikopf mit `--features diagnostics`, Lauf mit `taskset -c 0-7` | `50573ad8ef1c…` | `afb0b73ed812…` |

Beide sind gebaut, nicht gefahren. Weil der Bau reproduzierbar ist, kann der
Schritt im Messfenster neu bauen und das Erzeugnis gegen diese Hashes prüfen.
Ihre Zeitzahlen gehören nur auf den Laptop, nie auf den Bau-Dienst (kein
AVX2/FMA).

`e15-0-faehigkeiten` ist ebenfalls ein Zeitwerkzeug — `greedy` steht dort in der
festen Rotation und in einer Zeitmessung — und liegt außerhalb des festgelegten
Umfangs. Nicht gefahren.

## Nicht betroffen

Vierzehn Beispiele enthalten kein Token der `greedy`-Familie
(`grep -LwE "greedy|greedy_from|greedy_opts|greedy_zuege" sim/examples/*.rs`):
`apl`, `balance`, `doomsayer-mechaniken-672`, `e21-0b-machbarkeit`,
`gear-balance`, `orch-m8-always`, `orch-subsumption`, `parity`,
`purge-nachladen-672`, `report`, `rotation-balance`, `step1`,
`wertungsmenge-981`, `zielvertrag-erkundung-939`. **Nicht betroffen, weil** in
`sim/src` keine Bibliotheksfunktion die Suche aufruft — außerhalb von
`rotation_search.rs` selbst nur das Programm `rota-sim` —, ein Beispiel ohne
eigenen Aufruf also keinen Weg zu `greedy_opts` hat. `orch-subsumption` nutzt aus
`rotation_search` nur `all_candidates` und `Origin`.

`sim/src/bin/rota-sim.rs` ruft `greedy` (Z. 103), ist aber ein Programm, kein
Beispiel, und gehört nicht zum Gegenstand dieses Issues. Nicht gefahren.

## Was diese Messung nicht belegt

- **Keine Zeit- und keine Leistungsaussage.** Die Wandzeiten oben sind unter
  Fremdlast entstanden (Lastmittel 8 bis 25 auf 16 Threads).
- **Nur die gefahrenen Aufrufe.** Gemessen sind die Standardaufrufe und die
  dokumentierten Schalter. Andere Bosslisten, Längen, Gewichte oder Startwerte
  können die Gruppen zur Entscheidung bringen; die Positivkontrolle zeigt einen
  solchen Aufbau.
- **B wurde nicht mit Spur gefahren.** Unter B kann der Gruppen-Zweig in den
  gefahrenen Aufrufen nicht laufen, weil `zuege.tripel` dort `false` ist; das ist
  aus dem Code abgeleitet und durch die gleichen Ausgaben gestützt, aber nicht
  eigens gezählt.
- **Welches Gruppenglied fehlte**, wo Platz war und keine Gruppe entstand.
- **`suchguete-erweitert` jenseits von Fall 1** und die Ursache seines Abbruchs.
- **Die Prüffälle liefen im Debug-, die Beispiele im Release-Profil.** Beide
  Profile rechnen deterministisch; ein Profilvergleich war nicht Gegenstand.

## Nachfahren

```text
# Bau, aus der Worktree-Wurzel
cargo remote -c -- build --release --examples
cargo remote -c -- build --release --features diagnostics --example messumgebung

# Fassung B: in sim/src/rotation_search.rs, Funktion greedy_opts,
#   tripel: mit_paaren   ->   tripel: false
# bauen, Binaries wegkopieren, zurücksetzen und per sha256 prüfen.

# Fahren, je Fassung, aus der Worktree-Wurzel
./target/release/examples/<beispiel>
(cd sim && ../target/release/examples/trace-stabilitaet)
(cd sim && ../target/release/examples/trace-stabilitaet --folgen)
./target/release/examples/zauber-abdeckung --toepfe-einzeln --mit-ausruestung
./target/release/examples/rotation-search \
  --bosses first-crucible,test-onslaught,test-metronome,test-gatekeeper,test-resistant \
  [--max-len 12] [--kein-nachlauf]
./target/release/examples/rotation-search \
  --bosses boss-1-1,boss-1-2,boss-1-3,boss-1-4,boss-2-1,boss-2-2,boss-2-3,boss-2-4,boss-3-1,boss-3-2,boss-3-3,boss-3-4,boss-4-1,boss-4-2,boss-4-3,boss-4-4,first-crucible \
  --max-len 6|8|12

# Positivkontrollen, je Fassung
cargo remote --no-copy-lock -- test --test greedy_traegt_die_gruppen
cargo remote --no-copy-lock -- test --test ac5_kontrolle_lohnt_sich -- \
  --ignored die_gewichtsreihe_zeigt_ein_fenster --nocapture
cargo remote --no-copy-lock -- test --no-fail-fast
```

Verglichen wird mit `cmp` je stdout und stderr, dazu der Exit-Code jedes Laufs.
Die Spur-Fassung A′ entsteht aus A mit einer `eprintln!`-Zeile am Ende von
`greedy_zuege` und zwei Zählern an den Stellen `if tripel_gewinnt {` und nach dem
Einfügen einer Gruppe; sie ist ein Messwerkzeug für diesen Bericht und gehört
nicht in den Bestand.
