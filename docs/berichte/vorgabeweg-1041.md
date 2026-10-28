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

**Issue #1041.** Gemessen am 2026-10-24 auf `origin/main` =
`23d7c13fba470ee1d6a677bfdd64d042f05724f6`. Release-Bau über cargo remote v2
(`linux-gnu-bookworm`, Rust 1.98.1), gefahren auf dem Laptop: zuerst der
deterministische Teil (Worker-1), danach im Messfenster die beiden Zeitläufe
`orch-verteilung` und `messumgebung` (Abschnitt „Zeitläufe im Messfenster").
Am 2026-10-28 kam die Behebung von `suchguete-erweitert` dazu, gemessen auf
`main` = `b087f63fe88056d70f223c3eeb857a3c7f6526d5` (Abschnitt „Die Behebung
von `suchguete-erweitert`").

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
5. **Die zwei Zeitläufe unterscheiden A und B nicht.** Ihre Suchaufrufe prüfen
   keine Gruppe, und die deterministische Ausgabe ist in beiden Fassungen
   gleich. In `orch-verteilung` liegen beide A-B-Abstände unter der
   A,A-Streuung. In `messumgebung` waren nach der Regel des Auftrags vier von
   acht Lasten auffällig; die Nachprüfung hat das nicht wiederholt, und die zwei
   größten Abstände (−10,5 und −12,1 % in Lasten, die gar nicht suchen) kehrten
   ihr Vorzeichen um.

> Kriterium 3 des Issues, in derselben Deutlichkeit: **Ein Vorgabeweg-Wechsel,
> der neunzehn Beispiele inhaltlich nicht bewegt.** Das liegt an der Bauart der
> Suche, nicht am Zufall — siehe „Warum sich nichts bewegt".

Ein Nebenbefund wog schwerer als die Zahlen hier: **`suchguete-erweitert`
lieferte im Bestand gar kein Ergebnis** — Exit 101 in Fall 1 von 60, in A und B
gleich. **Die Ursache ist #1088, am Bau vor und nach dem Merge belegt, und die
Behebung steht in diesem PR:** Seither läuft der Läufer in A und B mit Exit 0
durch alle 60 Fälle, druckt seine Tabelle, und A und B sind bitgleich. Siehe
„Die Behebung von `suchguete-erweitert`".

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
| B, Neubau vor den Zeitläufen, alle Beispiele                | `b6258057cb3cb276bdcf30ca` | Exit 0, siehe unten      |
| B, Neubau, `messumgebung` mit `diagnostics`                 | `7d2ed021d438beef620f553b` | Exit 0, siehe unten      |
| A, Neubau nach dem zweiten Rückbau, alle Beispiele          | `b6625b697c0da995b3e91d1b` | Exit 0, siehe unten      |
| A, Neubau, `messumgebung` mit `diagnostics`                 | `b71898ba717e60b40c33fd44` | Exit 0, siehe unten      |

Jede Zeile trägt im Protokoll `cargo-remote: job <id> | linux-gnu-bookworm | Rust 1.98.1`.

**Vor den Zeitläufen ist B ein zweites Mal entstanden**, mit derselben Zeile
(sha256 von `rotation_search.rs` wieder `6f27bbe2…`), und wurde wieder
zurückgebaut: `sha256sum` von `rotation_search.rs`, `Cargo.lock` und
`.cargo-remote.toml` gegen den Ausgangszustand, `diff` Exit 0,
`git status --porcelain` leer. Die vier Neubauten (mit `--no-copy-lock`) ergaben
für die Zeitläufe dieselben Binaries wie die ersten Bauten: `orch-verteilung` A
`75206e6d…`, B `d04e841c…`; `messumgebung` mit `diagnostics` A `50573ad8…`, B
`afb0b73e…`; dazu `messumgebung` ohne Merkmal A `4d5dce5e…`, B `6a8169ea…`.
Verglichen wurden diese Binaries, nicht alle 36.

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

Die Zeilen `suchguete` und `suchguete-erweitert` zeigen den Stand **vor** der
Behebung in diesem PR. Danach laufen beide in A und B mit Exit 0 und bitgleich
(`suchguete-erweitert` 69 Zeilen, Spur A′ 408 · 0 · 0; `suchguete` 35 Zeilen,
120 · 0 · 0), siehe „Die Behebung von `suchguete-erweitert`".

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

**1. `suchguete-erweitert` lieferte im Bestand kein Ergebnis — behoben in diesem
PR.** Es brach in Fall 1 von 60 an seiner eigenen Zusicherung ab, in A und B
gleich:

```text
thread 'main' panicked at sim/examples/suchguete-erweitert.rs:530:17:
assertion `left == right` failed: Fall 1: gleiche Werte, aber 46 Messungen gegen greedys 52 — dann laufen
die beiden verschiedene Wege zum selben Ergebnis, und die Zerlegung unten rechnet mit einer Gleichheit,
die es nicht gibt
```

Folgen: Die Ergebnistabelle wurde nie gedruckt; eine Zahl, die sich bewegen
könnte, gab es im Bestand nicht. Erreicht wurden vor dem Abbruch Z. 496 und 498
(Fall 0 und 1) und Z. 274 (`lns`, Fall 0) — **Z. 294 nie**: `lns` bricht ab,
wenn die Startliste höchstens zwei Zeilen hat, und Fall 0 liefert zwei. Die
Tabelle „Alle fünf Verfahren" in `SIM-MESSUNGEN.md` (Messung 7) nennt ihren
Läufer nicht, trägt aber dieselbe Zusicherungszeile, die dieser Lauf vor dem
Abbruch druckt (`beam k=30 3480 in 156 Messungen`).

Die Ursache ist **#1088, gemessen**: Der Bau vor dem Merge von #1088 läuft mit
Exit 0 durch, der Bau des Merges bricht in Fall 1 mit denselben 46 gegen 52 ab
— bei unveränderter Beispieldatei. Ursache, Behebung und das A/B danach stehen
im Abschnitt „Die Behebung von `suchguete-erweitert`".

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

**4. Die zwei Zeitwerkzeuge messen heute etwas anderes, als ihre Ausgabe
nahelegt.** In `orch-verteilung` ist auch die Endausbau-Rate ein Ein-Kampf-Wert,
ohne Kennzeichnung (rund 12 Millionen „Kämpfe" je Sekunde), und das
Endausbau-Profil meldet in allen 3090 Läufen `Schaden mean 0.0`. `messumgebung`
rechnet im Mittel auf einem Kern statt auf den acht erlaubten und verwirft jeden
seiner sechs Läufe selbst (Streuung innerhalb des Laufs bis 69,19 %), obwohl die
Drucksignale vor jedem Lauf unter den Grenzen lagen. Beides gilt in A und B
gleich; die Ursachen habe ich nicht untersucht.

**5. Die Prozentspalte von `tools/last-protokoll.sh` ist ein Lebensdurchschnitt,
keine Momentanlast** (Abschnitt „Lastprotokoll"). Ein frisch gestarteter Prozess
steht dort mit Werten über 100 %, ein dauerhaft laufender mit einem festen Wert.

## Die Behebung von `suchguete-erweitert`

Gemessen am 2026-10-28. Der Zweig trägt dafür `main` = `b087f63f` per
Merge-Commit `51e7b55c`; seit der Basis `23d7c13f` der übrigen Messungen hat
#1133 `sim/src/balance.rs` geändert, das jede Messung des Beispiels benutzt.

### Ursache: #1088, am Bau belegt

| Stand      | Inhalt                                     | Bau-Job                    | Lauf     | Ergebnis                                   |
| ---------- | ------------------------------------------ | -------------------------- | -------- | ------------------------------------------ |
| `65ade0e5` | `main` unmittelbar vor dem Merge von #1088 | `5aebda4d5cfab83f1aca3c5c` | Exit 0   | Tabelle gedruckt, 68 Zeilen, alle 60 Fälle |
| `342022b7` | Merge von #1088 (PR #1093)                 | `81e5b99265f256f929072c3b` | Exit 101 | Fall 1: 46 Messungen gegen greedys 52      |
| `b087f63f` | `main` heute, ohne Behebung                | `c3c406286d95d72ec206542c` | Exit 101 | Fall 1: 46 Messungen gegen greedys 52      |

Die Beispieldatei ist in allen drei Ständen dieselbe
(`git diff --quiet 65ade0e5 342022b7 -- sim/examples/suchguete-erweitert.rs`
und dasselbe von `342022b7` bis `b087f63f`: Exit 0). Der Merge `342022b7`
ändert gegenüber `65ade0e5` genau drei Dateien: `sim/src/rotation_search.rs`,
das neue Beispiel `e1-1088-suchraum` und den neuen Prüffall `sucher_doom`.
Damit ist die Vermutung aus der ersten Fassung dieses Berichts bestätigt.

Die zwei alten Stände entstanden in einem Wegwerfbaum unter `/work/tmp`
(`git archive` von `Cargo.toml`, `Cargo.lock`, `sim`, `data`, `tests/parity`),
mit der heutigen `.cargo-remote.toml` und `rust-toolchain.toml` (1.98.1), denn
`65ade0e5` trägt noch den Eintrag des alten Bau-Dienstes. Gebaut wurde nur
`--example suchguete-erweitert`, gefahren lokal. **Ein Stolperstein, gemessen:**
Der zweite Bau im selben Baum meldete `Finished … in 0.03s` und lieferte das
Binary des ersten bitgleich zurück (sha256 `1ea5b891…`). Vermutlich, weil
`git archive` die Dateizeiten auf die Commit-Zeit setzt; nach `touch` aller
Quellen baute derselbe Befehl neu (`275e8cea…`). Gezählt ist nur dieser Bau.

### Der Mechanismus

Seit #1088 legt `greedy_zuege` an einem doom-Boss für jeden Zauber im Vorrat,
der doom senkt, eine bedingte Zeile dazu (`boss_kandidaten`,
`Origin::DoomClear`). Die übrigen Verfahren des Läufers bekamen weiter den
blanken Vorrat. In Fall 1 (`boss-1-2`, 9 Kandidaten, Länge 3) ist das genau
eine `purge`-Zeile, gemessen in drei Runden und nie gewählt:

```text
greedy ohne Paare             10 + 2·9 + 3·8 = 52   Abbruch NoImprovement nach 2 Zeilen
beam-einfuegen k=1, Abbruch    9 + 2·8 + 3·7 = 46   (blanker Vorrat)
```

Eine Analyse-Fassung (nie committet, Job `dcff124b6981bf1268b42063`) hat alle
60 Fälle nachgerechnet:

- Ergänzt wird in **3 von 60** Fällen: 1, 33 und 45, alle an `boss-1-2`, je
  `purge`.
- `greedy` hat die Zeile in keinem davon gewählt, mit und ohne Paare.
- In **0 von 60** Fällen ändert sie das Optimum der erschöpfenden Suche.
- `beam_einfuegen` mit k = 1 und Abbruch weicht auf dem blanken Vorrat in genau
  diesen drei Fällen von greedys Messungszahl ab (46, 46, 70 gegen 52, 52, 80),
  auf Vorrat plus Ergänzung in keinem.

### Die Behebung: im Beispiel, nicht in der Bibliothek

`greedy` tut, was #1088 festlegt und `sucher_doom` prüft. Falsch war die
Annahme des Läufers, alle Verfahren suchten im übergebenen Vorrat. Deshalb liegt
die Behebung in den zwei Beispielen; `sim/src` bleibt unberührt.

- **`suchguete-erweitert`** (Commit `63087140`): Erschöpfend, Beam, Beam mit
  Einfügen und QD suchen in `suchraum` = Vorrat plus `boss_kandidaten`. `greedy`
  und LNS bekommen weiter den Vorrat, weil `greedy` selbst ergänzt und die Zeile
  sonst doppelt zur Wahl stünde. Eine neue Zusicherung vergleicht je Fall
  greedys eigene Meldung `SearchResult::ergaenzt` mit der nachgerechneten
  Ergänzung, und die Ausgabe nennt, in wie vielen Fällen der Raum über den
  Vorrat hinausgeht. Die Summe der erschöpfenden Messungen kommt aus der
  Schleife; der zweite Durchgang am Ende lief noch über den Vorrat.
- **`suchguete`** (Commit `06d9e027`): derselbe Unterschied, nur ohne Abbruch.
  Der Bezugspunkt lief über den Vorrat, `greedy` über Vorrat plus Ergänzung.
  Das änderte kein Optimum, aber die Messungszahl des Bezugspunkts (62328 statt
  65049); ein `greedy`, das mit der Zeile über das Optimum hinauskäme, würde als
  Verfehlen gebucht. Jetzt sucht auch dort der Bezugspunkt in `suchraum`, mit
  derselben Zusicherung. Sonst bleibt die Ausgabe gleich: Treffer, Regret und
  die Liste der zehn Verfehler sind dieselben wie vor der Behebung.

**Gegenproben**, beide nie committet:

| Fassung | Änderung                                                   | `suchguete`                                                     | `suchguete-erweitert`                   |
| ------- | ---------------------------------------------------------- | --------------------------------------------------------------- | --------------------------------------- |
| M1      | `suchraum` ohne Ergänzung                                  | Exit 101, Fall 1: ``greedy ergaenzte ["purge"], `suchraum` []`` | Exit 101, Fall 1, dieselbe Meldung      |
| M2      | wie M1, alle Zusicherungen als `debug_assert`, Release-Bau | —                                                               | Exit 0, Tabelle über den blanken Vorrat |

Bau-Jobs: M1 `ef91d2265a9bdbb49dd66e5e` (danach per `git checkout`
zurückgebaut, `sha256sum -c` alle `OK`), M2 `fb87246766b382a5e629720e` (eine
ungetrackte Kopie des Beispiels, danach gelöscht). M1 zeigt, dass die
neue Zusicherung greift und die Ursache benennt, bevor die alte
Messungs-Zusicherung nur ihre Folge meldet. M2 ordnet die Zahlen unten zu.

### A und B nach der Behebung

Bauten aus der Worktree-Wurzel, Kopf `06d9e027`: A
`2d1902abf73e325d3b90169e`, B `ef0854be9b07c3c9bb789b3c` (dieselbe Zeile wie
oben, sha256 von `rotation_search.rs` wieder `6f27bbe2…`), Spur-Fassung A′
`dd3d4fe85fe807f0425fe3c8`. Nach B und nach A′ zurückgebaut: `sha256sum -c`
über `rotation_search.rs`, beide Beispiele, `Cargo.lock` und
`.cargo-remote.toml` gegen den Stand nach dem Commit, alle `OK`;
`git status --porcelain` leer.

| Lauf                  | Exit A / B | Vergleich A ↔ B                   | Spur A′     |
| --------------------- | ---------- | --------------------------------- | ----------- |
| `suchguete-erweitert` | 0 / 0      | bitgleich, 69 Zeilen, stderr leer | 408 · 0 · 0 |
| `suchguete`           | 0 / 0      | bitgleich, 35 Zeilen, stderr leer | 120 · 0 · 0 |

stdout A′ = A ist in beiden bitgleich. **Keine Gruppe wird in diesen zwei
Läufern je geprüft**, deshalb kann B dort nichts bewegen:

| Klasse                                                  | `suchguete-erweitert` | `suchguete` |
| ------------------------------------------------------- | --------------------: | ----------: |
| ohne Gruppen-Schalter (`greedy_opts(…, false)`)         |                    60 |          60 |
| Abbruch an der Längengrenze (`LengthLimit`)             |                   230 |          17 |
| `NoImprovement`, keine drei Plätze mehr frei            |                    82 |          25 |
| `NoImprovement`, Platz frei, aber keine Gruppe gebildet |                    36 |          18 |
| Gruppen geprüft                                         |                 **0** |       **0** |

Von den 348 Aufrufen mit Gruppen in `suchguete-erweitert` sind 228
LNS-Nachfüllungen mit vorgegebener Startliste — die Stelle Z. 294 des alten
Stands, die vor der Behebung nie erreicht wurde.

**Die Tabelle, die es vorher nicht gab** (A = B):

```text
Verfahren               Treffer   Quote   Regret-M  Regret-Max   Messungen   schlechtester
greedy ohne Paare      48 / 60   80.0 %     31.58 %   100.00 %       1792   Fall 22
greedy mit Paaren      50 / 60   83.3 %     15.03 %    56.52 %       1914   Fall 31
beam k=1               34 / 60   56.7 %     23.83 %    69.83 %       1306   Fall 38
beam k=3               44 / 60   73.3 %     20.30 %    67.19 %       3012   Fall 38
beam k=10              51 / 60   85.0 %     15.76 %    47.83 %       8107   Fall 31
beam-einf. k=1         50 / 60   83.3 %     16.69 %    56.52 %       2746   Fall 31
beam-einf. k=3         53 / 60   88.3 %     17.32 %    56.52 %       7332   Fall 31
beam-einf. k=10        54 / 60   90.0 %     18.10 %    47.83 %      21631   Fall 31
beam-einf.+stop k=1    48 / 60   80.0 %     31.58 %   100.00 %       1792   Fall 22
beam-einf.+stop k=3    51 / 60   85.0 %     39.51 %   100.00 %       3817   Fall 22
beam-einf.+stop k=10   52 / 60   86.7 %     43.37 %   100.00 %       7165   Fall 22
LNS                    55 / 60   91.7 %     21.74 %    56.52 %      10135   Fall 31
QD (Archiv)            41 / 60   68.3 %     20.98 %    69.83 %       6420   Fall 38
erschoepfend           60 / 60  100.0 %      0.00 %     0.00 %      65049   —

QD-Archiv: 10.0 Nischen je Fall im Mittel
Suchraum ueber den Vorrat hinaus (#1088): 3 von 60 Faellen, Zauber ["purge"]
```

Die festen Sätze am Ende der Ausgabe („Regret-Max 100 %, immer Fall 22",
„56,5 %", „dort steht Fall 31", „nicht nur zwei Faelle") stimmen mit dieser
Tabelle überein.

**Gegen Messung 7 in `SIM-MESSUNGEN.md`:** Der Bau `65ade0e5` gibt jede Zahl
der beiden Tabellen dort wieder, dazu `beam k=1` mit 34 von 60 und 9,9 Nischen.
Auf `main` mit Behebung sind Treffer, Quote und Regret-Max in allen 14 Zeilen
dieselben. Anders sind:

| Größe                                              |   Messung 7, `65ade0e5` | `main` mit Behebung | woher                                        |
| -------------------------------------------------- | ----------------------: | ------------------: | -------------------------------------------- |
| Messungen `greedy` ohne / mit Paaren               |             1770 / 1892 |         1792 / 1914 | #1088: `greedy` misst die ergänzte Zeile mit |
| Messungen LNS                                      |                    9969 |               10135 | #1088, ebenso                                |
| Messungen Beam, Beam mit Einfügen, QD, erschöpfend | z. B. 62328 erschöpfend |   65049 erschöpfend | Behebung: Vorrat plus Ergänzung              |
| Regret-M `beam k=1`                                |                 22,70 % |             23,83 % | Behebung                                     |
| Regret-M QD                                        |                 21,27 % |             20,98 % | Behebung                                     |
| QD-Nischen je Fall                                 |                     9,9 |                10,0 | Behebung                                     |

Zugeordnet mit M2, dem blanken Vorrat auf `main`: M2 weicht von `65ade0e5` nur
in den Messungszahlen von `greedy` und LNS ab (dazu die neue Zeile `Suchraum …`
mit 0 von 60). Die Behebung weicht von M2 nur in den Zeilen der Verfahren ab,
die jetzt in `suchraum` suchen — Beam, Beam mit Einfügen, QD, erschöpfend —, in
der Nischenzahl und in der Zeile `Suchraum …`. `SIM-MESSUNGEN.md` trägt den
Stand vom 2026-08-14 und bleibt unverändert.

## Zeitläufe im Messfenster

**Gemessen am 2026-10-24 im Messfenster:** `rota-fenster geben Orchestrator` um
10:40:22, Abgabe um 11:22:23, beide mit `rota-fenster status` zurückgelesen. Die
Läufe selbst: 10:41:09 bis 11:19:23. Beide Zeitläufe liefen auf dem Laptop, nie
auf dem Bau-Dienst (kein AVX2/FMA); `orch-verteilung` meldet in jedem Lauf
`avx true · avx2 true · fma true`.

**Stand.** Gemessen ist `23d7c13f` wie oben. Auf `main` kam seither PR #1133
(#915) dazu; er ändert `sim/src/balance.rs`, das beide Zeitläufe benutzen. Auf
dem heutigen `main` können die absoluten Zeiten deshalb anders liegen. Der
Vergleich A gegen B ist davon nicht berührt, denn beide Fassungen stehen auf
derselben Basis.

**Umgebung, abgelesen und nicht angenommen:**

- `platform_profile` war bei jeder Ablesung `performance` (10:40:40, 10:41:09,
  11:03:42, 11:04:32, 11:07:36, 11:19:23, 11:19:40). Von 11:08:14 bis 11:19:24
  las ein Wächter Profil, EPP und Boost alle 5 s und sah keinen Wechsel. Für die
  Hauptreihe gibt es nur die Ablesungen davor und danach, denn das Systemjournal
  ist für diesen Benutzer nicht lesbar.
- Treiber `amd-pstate-epp` (aktiv), Governor `powersave`, EPP `performance`,
  **Boost aus**: `cpufreq/boost` = 0, Höchsttakt 3,30 GHz (abgelesen 11:08:14
  und 11:19:40). Mit früheren Baselines sind die absoluten Zahlen deshalb nicht
  vergleichbar.
- Alle Läufe mit `taskset -c 0-7` aus der Worktree-Wurzel. Auf diesem Rechner
  sind das **vier physische Kerne mit je zwei SMT-Threads**
  (`thread_siblings_list`: cpu0 → `0-1`, cpu7 → `6-7`). Beide Programme melden
  8 erlaubte Stränge.
- Die Binaries stammen aus der Bautabelle oben. Vor und nach jedem Abschnitt
  wurden sie per sha256 gegen den Bauhash geprüft.

**Ablauf je Zeitlauf:** A, A (das A,A-Paar), dann A und B direkt nacheinander.
`orch-verteilung` lief je Abschnitt so oft hintereinander, bis 60 s um waren;
ein Lauf dauert rund 0,1 s. `messumgebung` lief je Abschnitt einmal im
Standardmodus (acht Lasten zu je 20 s), das sind rund 178 s.

**Wie verglichen wird:** Die A,A-Streuung ist der Abstand zwischen A1 und A2,
bezogen auf ihr Mittel. Der A-B-Abstand ist B gegen A3, seinen direkten
Vorgänger. Ist die A,A-Streuung größer als der A-B-Abstand, heißt das „nicht
unterscheidbar". Bei `orch-verteilung` steht für jeden Abschnitt sein Median.

**Nachprüfung, nach Sicht der Ergebnisse beschlossen.** In `messumgebung` lag
der A-B-Abstand bei vier von acht Lasten über der A,A-Streuung. Ein einzelner
B-Lauf kann eine Störung nicht von einer Wirkung des Binarys trennen. Deshalb
lief im selben Fenster ein zweites Paar in umgekehrter Reihenfolge, erst B2,
dann A4, für beide Zeitläufe. **Die Vorhersage stand vor dem Lauf fest** (Datei
angelegt 11:06:56, Nachprüfung ab 11:07:36) und wird unten gegen das Ergebnis
gehalten.

### `orch-verteilung`

| Abschnitt | Fassung | Läufe in 60 s | Wandzeit je Lauf, Median | schnellster Lauf | Durchsatz-Zeile, Median | Endausbau-Zeile, Median |
| --------- | ------- | ------------: | -----------------------: | ---------------: | ----------------------: | ----------------------: |
| A1        | A       |           464 |                 0,1037 s |         0,0695 s |                 7027263 |                10850205 |
| A2        | A       |           375 |                 0,1219 s |         0,0729 s |                 6171583 |                 9053859 |
| A3        | A       |           544 |                 0,0937 s |         0,0697 s |                 7783627 |                12092512 |
| B         | B       |           579 |                 0,0927 s |         0,0695 s |                 7946793 |                12062659 |
| B2        | B       |           571 |                 0,0932 s |         0,0707 s |                 7882113 |                12002393 |
| A4        | A       |           557 |                 0,0968 s |         0,0710 s |                 7629400 |                11678739 |

| Größe           | A,A (A1/A2) | A,A über A1–A3 | B gegen A3 | B2 gegen A4 | Urteil               |
| --------------- | ----------: | -------------: | ---------: | ----------: | -------------------- |
| Wandzeit        |     16,11 % |        26,49 % |    −1,04 % |     −3,72 % | nicht unterscheidbar |
| Durchsatz-Zeile |     12,97 % |        23,05 % |    +2,10 % |     +3,31 % | nicht unterscheidbar |
| Endausbau-Zeile |     18,05 % |        28,49 % |    −0,25 % |     +2,77 % | nicht unterscheidbar |

**Der deterministische Teil ist in allen 3090 Läufen gleich**, A wie B: dieselbe
Rotation aus `greedy` (Zauber 24, 4, 51, 42, 0), dieselben Fingerabdrücke,
FNV-1a `0x42210c39918563b9`. Jeder Lauf endete mit Exit 0 bei leerem stderr.

**Bei der Zeit sind A und B nicht unterscheidbar.** Ehrlich dazu: B lag in
beiden Paaren knapp unter seinem A-Nachbarn, und beide B-Mediane liegen unter
allen vier A-Medianen. Die zwei ruhigsten A-Abschnitte (A3, A4) liegen aber
ihrerseits 3,25 % auseinander, und die schnellsten Einzelläufe sind in A und B
gleich (A 0,0695–0,0729 s, B 0,0695–0,0707 s). Ein Unterschied von wenigen
Prozent lässt sich mit dieser Anordnung nicht auflösen, weder als vorhanden noch
als abwesend.

**Was diese Zahlen messen.** Beide Raten sind Ein-Kampf-Werte: Mit der
Standardausrüstung zieht kein Zufallsstrom, also rechnet `balance::run` einen
Kampf und vervielfacht ihn (#908). `orch-verteilung` weist das bei der ersten
Rate aus (`<== KEINE DURCHSATZMESSUNG`), bei der Endausbau-Rate nicht. Dort
stehen rund 12 Millionen „Kämpfe" je Sekunde. Den einzigen Teil, den B berührt,
die eine Suche, hat die Spur-Fassung A′ mitgezählt: ein Aufruf, `max_len` 5, 5
Zeilen, `LengthLimit`, 1615 Messungen, `tripel_geprueft` 0. Die Gruppen kommen
nie zur Wahl.

Nutzer- und Systemzeit aus `/usr/bin/time` haben 10 ms Auflösung. Ihre Mediane
liegen in allen sechs Abschnitten bei 0,06–0,08 s bzw. 0,01–0,02 s und
unterscheiden sich um höchstens einen Takt. Für einen Vergleich sind sie zu
grob.

### `messumgebung`

`full_fights/s` je Last, wie gedruckt:

| Last                          |    A1 |    A2 |    A3 |     B |    B2 |    A4 | A,A (A1/A2) | B gegen A3 | B2 gegen A4 | A-Band über vier A |
| ----------------------------- | ----: | ----: | ----: | ----: | ----: | ----: | ----------: | ---------: | ----------: | -----------------: |
| `nur Schaden`                 | 36911 | 37463 | 38046 | 34035 | 36620 | 34869 |      1,48 % |   −10,54 % |     +5,02 % |             8,63 % |
| `verstaerkungslastig`         | 38956 | 37780 | 36570 | 32142 | 34638 | 33886 |      3,07 % |   −12,11 % |     +2,22 % |            13,78 % |
| `brandlastig`                 | 27469 | 26051 | 26879 | 25895 | 26248 | 23917 |      5,30 % |    −3,66 % |     +9,75 % |            13,62 % |
| `kanallastig`                 | 34420 | 25911 | 33206 | 32112 | 34833 | 35166 |     28,21 % |    −3,29 % |     −0,95 % |            28,76 % |
| `Boss mit HP-Phase`           | 52631 | 43009 | 55934 | 57097 | 51431 | 50192 |     20,12 % |    +2,08 % |     +2,47 % |            25,62 % |
| `Ueberleben und Unterbrechen` | 13977 | 13862 | 13487 | 13764 | 12852 | 12891 |      0,83 % |    +2,05 % |     −0,30 % |             8,01 % |
| `vollstaendiger Greedy-Lauf`  | 22705 | 23114 | 24033 | 22870 | 24396 | 24605 |      1,79 % |    −4,84 % |     −0,85 % |             8,05 % |
| `vollstaendiger LNS-Lauf`     | 25728 | 25165 | 24073 | 24110 | 23161 | 23234 |      2,21 % |    +0,15 % |     −0,31 % |            10,16 % |

`resolved_candidates/s` und `verified_elites/hour` der zwei Suchlasten ergeben
bis auf 0,01 Prozentpunkte dieselben Abstände.

**Der deterministische Teil ist in allen sechs Läufen gleich**, A wie B: je Last
derselbe Trace-Hash, dieselbe Rotation, dieselbe Zahl bewerteter Rotationen
(Greedy 2273, LNS 9092) und dieselbe Schlangenstatistik. **Die Arbeit ist
dieselbe:** Die sechs Kampflasten rufen keine Suche auf, und in den fünf
Suchaufrufen der zwei Suchlasten prüft die Spur-Fassung A′ keine Gruppe
(`messumgebung --trace`: ein Aufruf im Greedy-Lastfall, vier im LNS-Lastfall, je
`max_len` 6, 6 Zeilen, `LengthLimit`, 2273 Messungen, `tripel_geprueft` 0). Die
Kalibrierung zeigt zudem die Reduktion aus #908: Jede Kampflast rechnet 1 Kampf
statt der angeforderten 200, der Greedy-Lastfall 2275 Kämpfe für 2273
Rotationen.

**Nach der Regel des Auftrags (Hauptpaar)** sind vier von acht Lasten nicht
unterscheidbar. Bei vier liegt der A-B-Abstand über der A,A-Streuung:
`nur Schaden` (−10,54 % gegen 1,48 %), `verstaerkungslastig` (−12,11 % gegen
3,07 %), `Ueberleben und Unterbrechen` (+2,05 % gegen 0,83 %) und der Greedy-Lauf
(−4,84 % gegen 1,79 %).

**Die Nachprüfung hat diese Abstände nicht wiederholt.** Im Paar B2/A4 liegt B
bei sechs von acht Lasten höchstens 2,5 % neben A; bei `nur Schaden` sind es
+5,02 %, bei `brandlastig` +9,75 %. Die zwei größten Abstände der Hauptreihe
kehren ihr Vorzeichen um. Gegen die Vorhersage gehalten:

| Vorhersage                                           | Bedingung                                                          | Ergebnis                                                                                                          |
| ---------------------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- |
| H2: Das B-Binary ist in diesen zwei Lasten langsamer | B2 liegt in beiden mindestens 8 % unter A4                         | **widerlegt**: +5,02 % und +2,22 %                                                                                |
| H1: Der B-Lauf war gestört                           | Abstand innerhalb der A,A-Streuung über A1–A3 (3,03 % bzw. 6,32 %) | `verstaerkungslastig` erfüllt (2,22 %); `nur Schaden` **verfehlt** (5,02 %), laut Vorhersage dort „unentschieden" |

Für eine Störung spricht, was schon **vor** der Vermutung aufgezeichnet war. Der
B-Lauf maß `nur Schaden` von 10:59:44 bis 11:00:06 und `verstaerkungslastig` bis
11:00:28. Um 11:00:00 liefen sieben `cron`-Prozesse, `systemd-helper` und
`powerprofilesctl` als root. Die Streuung innerhalb des Laufs stieg genau in
diesen zwei Lasten auf 37,09 % und 35,76 %; in den anderen fünf Läufen lag sie
bei 17,22–20,42 % und 4,44–23,89 %. Der 10-s-Takt zeigt die Störung nur
schwach: Die Zeile 11:00:11 trägt die höchste Belegung des ganzen Laufs (Kerne
0–7 34,4 %, Kerne 8–15 30,2 %). Ihre Nachbarn 11:00:01 und 11:00:21 liegen auf
0–7 mit 28–29 % knapp über dem Rest des Laufs (20,3–27,5 %), auf 8–15 mit
21–23 % darin (13,6–23,8 %). In der Vorhersage-Datei steht für den Rest
„22-24 %"; das war zu eng abgelesen, die Bedingungen von H1 und H2 hängen nicht
daran.

**Die A,A-Streuung des ersten Paars war zu klein geschätzt.** Über vier A-Läufe
reicht das Band je Last von 8,01 % bis 28,76 %, bei `nur Schaden` 8,63 % statt
1,48 %. Von den 16 A-B-Abständen beider Paare liegt nur einer über diesem Band:
`nur Schaden` im Hauptpaar, der gestörte Lauf.

**Bei der Zeit sind A und B auch in `messumgebung` nicht unterscheidbar.** Bei
fünf Lasten wechselt das Vorzeichen zwischen den Paaren. Wo es gleich bleibt
(`kanallastig` −3,29 / −0,95 %, `Boss mit HP-Phase` +2,08 / +2,47 %, Greedy
−4,84 / −0,85 %), liegt der Abstand weit innerhalb des A-Bands.

`time_to_best` ist die Dauer des jeweils letzten Suchaufrufs, also ein
Einzelwert. Beim LNS-Lauf stehen dort B 0,3833 s und B2 0,4517 s gegen A
0,3146–0,3576 s, während die Rate über 20 s in A und B gleich ist (+0,16 % und
−0,31 %). Als Vergleichsgröße taugt er nicht.

**`messumgebung` verwirft selbst alle sechs Läufe:** Exit 1, „ABBRUCH — die Rate
schwankte waehrend der Messung", in jedem Lauf bei sieben oder acht der acht
Lasten. Die Streuung innerhalb eines Laufs lag bei 4,18 bis 69,19 %, die Grenze
bei 5 %. Die Drucksignale vor jedem Lauf blieben dabei unter den Grenzen (PSI
cpu 0,00 bis 0,18). Gerechnet wurde im Mittel auf einem Kern: (Nutzer + System)
/ Wand = 0,99 bis 1,00 bei 8 erlaubten Strängen. Die Zahlen oben sind die, die
der Läufer trotz seines Abbruchs druckt.

### Lastprotokoll

Mitgeschrieben haben zwei Werkzeuge: `tools/last-protokoll.sh` im 30-s-Takt (71
Zeilen über beide Phasen) und ein eigener 10-s-Takt mit load1, PSI und der
Belegung der Kerne 0–7 und 8–15 aus `/proc/stat` (214 Zeilen). Auf 8–15 lief
nichts Gebundenes; dort stehen nur Fremdlast und die Hülle des Treibers.

| Abschnitt                    | Zeit              | load1 Mittel (max) | Kerne 8–15 belegt, Mittel (max) | PSI cpu max | PSI memory max |
| ---------------------------- | ----------------- | -----------------: | ------------------------------: | ----------: | -------------: |
| Grundlast, ohne eigenen Lauf | 10:41:09–10:42:09 |          4,3 (5,4) |                 40,3 % (53,9 %) |         9,4 |           35,5 |
| `orch-verteilung` A1         | 10:42:09–10:43:09 |          5,5 (6,1) |                 28,6 % (48,5 %) |         1,1 |           18,7 |
| `orch-verteilung` A2         | 10:43:19–10:44:19 |          4,9 (5,4) |                 35,3 % (51,6 %) |         3,5 |           19,4 |
| `orch-verteilung` A3         | 10:44:30–10:45:30 |          5,1 (5,4) |                 21,8 % (33,7 %) |         0,8 |            4,2 |
| `orch-verteilung` B          | 10:45:40–10:46:40 |          4,4 (4,6) |                 16,7 % (21,6 %) |         0,0 |            0,0 |
| `messumgebung` A1            | 10:47:50–10:50:48 |          3,4 (4,3) |                 17,2 % (27,5 %) |         0,7 |            0,0 |
| `messumgebung` A2            | 10:51:48–10:54:46 |          4,3 (6,1) |                 22,1 % (72,2 %) |         2,7 |           27,1 |
| `messumgebung` A3            | 10:55:46–10:58:44 |          3,4 (4,0) |                 17,3 % (24,0 %) |         0,8 |            0,0 |
| `messumgebung` B             | 10:59:44–11:02:42 |          4,1 (5,2) |                 18,5 % (30,2 %) |         0,6 |            0,2 |
| `orch-verteilung` B2         | 11:08:06–11:09:06 |          2,4 (2,7) |                 15,0 % (16,8 %) |         0,3 |            0,0 |
| `orch-verteilung` A4         | 11:09:16–11:10:16 |          2,7 (2,8) |                 16,7 % (22,4 %) |         0,0 |            0,0 |
| `messumgebung` B2            | 11:11:26–11:14:25 |          3,0 (3,5) |                 16,2 % (21,6 %) |         0,0 |            0,0 |
| `messumgebung` A4            | 11:15:25–11:18:23 |          2,8 (3,0) |                 17,5 % (26,6 %) |         0,1 |            0,2 |

Der höchste PSI-memory-Wert des Fensters fiel in keinen Messabschnitt: In der
Pause vor dem B-Lauf von `messumgebung` (10:59:01 bis 10:59:11) waren alle Kerne
zu 46–61 % belegt, bei PSI memory bis 36,4. Die Vorabprüfung des Laufs um
10:59:44 las PSI memory 1,22.

**Fremdlast aus dem Projekt und vom System**, aus dem 30-s-Protokoll:

- `cargo-remote`, `ssh`, `rsync` und `cargo-remote-fmt` aus
  `/work/rota-worktrees/w1-1027` um 10:41:11, 10:43:46, 10:44:17, 10:44:47,
  10:46:19, 10:51:22 und 10:52:23. Ein anderer Worktree baute während des
  Fensters über den Bau-Dienst, und dessen lokaler Teil (Hochladen, Rückkopie)
  läuft hier.
- `python3` aus `/work/tmp/plan-v3` sechsmal zwischen 10:41:41 und 10:58:59,
  `piper` und `python3` aus `/work/rota` um 10:41.
- der `cron`-Schub um 11:00:00 (siehe oben).
- **`git worktree add` samt Reflink-Kopie von `node_modules`** (Orchestrator-Agent,
  Worktree `orch-891-registry`) um 10:57:30–10:57:42 — während des Laufs A3 von
  `messumgebung` (Treiber-Log 10:55:46–10:58:44). Nachgetragen am 2026-10-28;
  das 30-s-Protokoll hat ihn nicht, weil er zwischen zwei Zeilen fiel (10:57:27,
  10:57:58). Einordnung unten.
- `powerprofilesctl` als root in 23 von 71 Zeilen. Ob es dabei etwas umgestellt
  hat, ist mangels Journal nicht lesbar; das Profil stand bei jeder Ablesung auf
  `performance`.
- dauerhaft: `firefox` mit Inhaltsprozess, ein `python3` aus
  `/datastore/refinanzioso/…` und ein `python` aus `/work/sysproblems/temps`.

**Der `git worktree add` um 10:57:30 und der Lauf A3.** Die Zeiten stehen auch
im Dateisystem: Verwaltungsdateien und ausgecheckter Baum von
`orch-891-registry` sind 10:57:30,55–10:57:30,95 entstanden, `node_modules`
(5776 Einträge) 10:57:40,78–10:57:41,86 (Geburtszeiten, `stat -c %w` und
`find -printf %B@`). Welche Lasten das traf, folgt aus den Fortschrittszeilen
auf stderr: Jede Kampflast dauert 22,0 s (2 s Aufwärmen, dann vier Abschnitte
zu 5 s), ab Laufbeginn 10:55:46,14; die Summe aller acht Lastdauern trifft die
Wandzeit des Laufs (178,12 s) bis auf 0,3 s. Das Fenster lag damit in zwei
Lasten:

- `Boss mit HP-Phase` (10:57:14–10:57:36): Ende von Abschnitt 3 und ganz
  Abschnitt 4,
- `Ueberleben und Unterbrechen` (10:57:36–10:57:58): Aufwärmen und knapp vier
  der fünf Sekunden von Abschnitt 1; die Kopie von `node_modules` fiel ganz
  hinein.

| Größe                          | `Boss mit HP-Phase`                               | `Ueberleben und Unterbrechen`                        |
| ------------------------------ | ------------------------------------------------- | ---------------------------------------------------- |
| Rate A3 gegen die vier A-Läufe | 55934, der höchste (43009–55934)                  | 13487, im Band (12891–13977)                         |
| Streuung innerhalb von A3      | 4,18 %, die kleinste aller Lasten in allen Läufen | 19,51 %, im Bereich der anderen Läufe (6,81–22,32 %) |
| B gegen A3 (Hauptpaar)         | +2,08 %, A,A 20,12 %                              | **+2,05 %, A,A 0,83 %** — eine der vier auffälligen  |
| B2 gegen A4 (Nachprüfung)      | +2,47 %                                           | −0,30 %                                              |

Im 10-s-Takt liegen die zwei Zeilen im Fenster (10:57:31, 10:57:41) mit 22,5
und 25,5 % auf den Kernen 0–7 und 16,6 und 17,2 % auf 8–15 im Bereich des
übrigen Abschnitts (17,2–28,5 % und 12,1–24,0 %). PSI cpu stieg auf 0,18
(10:57:41) und 0,79 (10:57:51) — der Höchstwert von A3 in der Tabelle
„Lastprotokoll" fällt damit direkt hinter den Vorgang.

**Einordnung: Am Urteil ändert das nichts — A und B bleiben nicht
unterscheidbar.** Für `Ueberleben und Unterbrechen` ist der Vorgang eine zweite
mögliche Ursache des auffälligen Abstands im Hauptpaar, denn er traf A3, den
Bezugslauf von B. Ob er A3 dort gedrückt hat, trennen diese Daten nicht: A3
liegt im Band der vier A-Läufe, seine Streuung im Bereich der anderen Läufe, und
die Nachprüfung (B2 gegen A4) wiederholt den Abstand nicht. Bei `Boss mit
HP-Phase` zeigt sich keine Spur; A3 ist dort der schnellste und ruhigste
A-Lauf.

**Die Prozentspalte von `last-protokoll.sh` ist kein Lastwert.** Sie kommt aus
`ps -o pcpu`, und das ist der Lebensdurchschnitt eines Prozesses (CPU-Zeit
durch Laufzeit). Ein frisch gestarteter `ps` steht dort mit bis zu 1000 %, ein
seit drei Tagen laufender `python3` in allen 71 Zeilen mit 38,1 %. Für die
Belegung gilt deshalb der 10-s-Takt.

**Unter Last, aber nicht unter der angekündigten.** Erwartet war load ~9–10;
gemessen lag load1 über das ganze Fenster bei 1,75 bis 6,15. Fremdlast gab es
trotzdem: In den Messabschnitten der Hauptreihe waren die nicht gebundenen Kerne
im Mittel zu 17–35 % belegt, mit Spitzen bis 72 %, am stärksten in A1/A2 von
`orch-verteilung`; in der Grundlast-Minute davor zu 40 %. In der Nachprüfung
waren es 15–17,5 %, und PSI blieb nahe 0. **Alle absoluten Zeiten oben sind unter Last entstanden**, mit
abgeschaltetem Boost, und sind keine Leistungsangabe. Die Aussage dieses
Abschnitts ist allein der Vergleich A gegen B.

`e15-0-faehigkeiten` ist ebenfalls ein Zeitwerkzeug (`greedy` steht dort in der
festen Rotation und in einer Zeitmessung) und liegt außerhalb des festgelegten
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

- **Keine Leistungsaussage.** Die Zeitläufe vergleichen A mit B. Ihre absoluten
  Zahlen sind unter Fremdlast und mit abgeschaltetem Boost entstanden, die
  Wandzeiten der deterministischen Läufe unter Lastmittel 8 bis 25 auf 16
  Threads.
- **Kleine Zeitunterschiede zwischen A und B bleiben offen.** Die A-Abschnitte
  streuen untereinander bis 26,49 % (`orch-verteilung`, Wandzeit), die A-Läufe
  von `messumgebung` je Last um 8,01 bis 28,76 %. Ein Abstand von wenigen
  Prozent ist mit dieser Anordnung weder zu belegen noch auszuschließen.
- **Die Zeitläufe auf dem heutigen `main`.** Gemessen ist `23d7c13f`; PR #1133
  ändert seither `balance.rs`, das beide Zeitläufe benutzen.
- **Nur die gefahrenen Aufrufe.** Gemessen sind die Standardaufrufe und die
  dokumentierten Schalter. Andere Bosslisten, Längen, Gewichte oder Startwerte
  können die Gruppen zur Entscheidung bringen; die Positivkontrolle zeigt einen
  solchen Aufbau.
- **B wurde nicht mit Spur gefahren.** Unter B kann der Gruppen-Zweig in den
  gefahrenen Aufrufen nicht laufen, weil `zuege.tripel` dort `false` ist; das ist
  aus dem Code abgeleitet und durch die gleichen Ausgaben gestützt, aber nicht
  eigens gezählt.
- **Welches Gruppenglied fehlte**, wo Platz war und keine Gruppe entstand.
- **Die Ursache ist auf den Merge von #1088 eingegrenzt**, nicht auf einen
  einzelnen Commit in dessen Zweig. Der Merge ändert außer
  `sim/src/rotation_search.rs` nur zwei neue Dateien, die das Beispiel nicht
  benutzt.
- **Die alten Stände sind mit Rust 1.98.1 gebaut**, nicht mit dem Compiler von
  damals; `65ade0e5` pinnt keinen.
- **Die CI fährt die zwei Läufer nicht**, sie baut sie nur (als Testziele ohne
  Prüffall). Exit 0 und die Tabelle sind lokal gemessen, nicht in der CI.
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

# Zeitläufe, nur im Messfenster, je Fassung, aus der Worktree-Wurzel.
# Reihenfolge A, A, A, B; Nachprüfung B, A. messumgebung mit --features diagnostics gebaut.
taskset -c 0-7 ./target/release/examples/orch-verteilung   # wiederholt, bis 60 s um sind
taskset -c 0-7 ./target/release/examples/messumgebung      # ein Lauf je Abschnitt

# Spur der Suchaufrufe, mit dem Bau der Spur-Fassung A′ (deterministisch, kein Zeitlauf)
./target/release/examples/orch-verteilung 2>&1 >/dev/null | grep SPUR1041
./target/release/examples/messumgebung --trace 2>&1 >/dev/null | grep SPUR1041

# Ursache des Abbruchs von suchguete-erweitert: alte Stände in einem Wegwerfbaum
git archive <stand> Cargo.toml Cargo.lock sim data tests/parity | tar -x -C <baum>
cp .cargo-remote.toml rust-toolchain.toml <baum>/
find <baum>/sim <baum>/data <baum>/tests -type f -exec touch {} +   # sonst bleibt das alte Binary
(cd <baum> && cargo remote -c --no-copy-lock -- build --release --example suchguete-erweitert)
<baum>/target/release/examples/suchguete-erweitert   # 65ade0e5: Exit 0, 342022b7: Exit 101

# A/B nach der Behebung, aus der Worktree-Wurzel, je Fassung
cargo remote -c --no-copy-lock -- build --release --example suchguete --example suchguete-erweitert
./target/release/examples/suchguete-erweitert
./target/release/examples/suchguete
```

Verglichen wird mit `cmp` je stdout und stderr, dazu der Exit-Code jedes Laufs.
Bei den Zeitläufen sind es die Mediane je Abschnitt (`orch-verteilung`) bzw. die
gedruckten Raten (`messumgebung`); mitgeschrieben haben `tools/last-protokoll.sh`
und ein 10-s-Takt aus `/proc/loadavg`, `/proc/pressure` und `/proc/stat`.
Die Spur-Fassung A′ entsteht aus A mit einer `eprintln!`-Zeile am Ende von
`greedy_zuege` und zwei Zählern an den Stellen `if tripel_gewinnt {` und nach dem
Einfügen einer Gruppe; sie ist ein Messwerkzeug für diesen Bericht und gehört
nicht in den Bestand.
