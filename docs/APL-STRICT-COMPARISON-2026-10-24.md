---
id: apl-strict-comparison-2026-10-24
status: Review
ssot: false
refs:
  docs: [APL-STRICT-COMPARISON-2026-09-06.md, GAME-DESIGN.md]
affects:
  [sim/src/apl/mod.rs, sim/tests/apl_strikte_vergleiche.rs, sim/examples/e1027-operator-measure.rs]
---

# Strikte APL-Vergleiche: Fassung B auf dem 10-ms-Takt (#1027)

Gemessen am 24. Oktober 2026 auf `23d7c13fba470ee1d6a677bfdd64d042f05724f6`. Auftrag ist die
Antwort `f-apl-strikte-vergleiche-2` vom 22.08.: „Erst den Vorher-Nachher-Lauf, dann entscheiden".
**Eine Messung, keine Semantikänderung.** Fassung B lag nur als Wegwerf-Patch im Arbeitsbaum und ist
nicht eingecheckt; `sim/src/apl/mod.rs` hat vor und nach jedem Lauf den SHA-256 `377742d1…` des
Bestands.

Gegenüber dem Versuch vom 06.09. ([APL-STRICT-COMPARISON-2026-09-06.md](APL-STRICT-COMPARISON-2026-09-06.md))
ändert sich zweierlei: Fassung B weckt auf dem **10-ms-Takt** statt auf der Millisekunde, und
gemessen wird über **alle 42 Bossdateien** statt über acht ausgewählte Suchen.

## Ergebnis

- **Fassung B trifft die Takt-Referenz überall.** In 84 Wurfproben (7 Aufbauten mit 6 verschiedenen
  zeitabhängigen Werten × 6 Vergleichsformen × 2 Operandenreihenfolgen) wirft Fassung B genau dort,
  wo ein an jedem Takt entscheidender Lauf wirft; Fassung A verfehlt 24 der 84. In allen
  protokollierten Kämpfen des Messprogramms liegt unter A wie unter B kein Eintrag neben dem
  10-ms-Raster.
- **Verspätet sind unter A nicht `<` und `>=`.** Bei fallenden Restzeiten verpassen `<`, sein
  Spiegel `T > v` und `!=` ihren Umschlag; bei `time` sind es `>`, sein Spiegel `T < time` und
  `!=`. `>=` trifft in allen Aufbauten. `!=` war bisher nicht gemessen.
- **Gefundene Rotationen:** Unter B verschiebt sich die gefundene Liste in **15 von 84 Suchen**
  (15 von 42 Bossen). In der Einstellung des Suchberichts (`rotation-search.rs`) sind es **2 von 42
  Bossen (4,8 %)**, in der Einstellung des AC5-Prüffalls **13 von 42 (31,0 %)**. Dieselbe Liste
  zählt unter B bei 2 bzw. 5 Bossen anders; in der Nachbarschaft der gefundenen Listen verschieben
  sich 5,78 % bzw. 11,59 % der Bewertungen, in 83 der 84 Nachbarschaften mindestens eine (Ausnahme:
  `kontrolle/orch-maxload`).
- **Der #797-AC5-Prüffall fällt unter B**: 2 bestanden, 1 rot
  (`ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist`), 1 ignoriert. Die gefundene
  Liste ist zeilengleich mit der „Experiment"-Liste des 1-ms-Versuchs vom 06.09. Dieselbe A-Liste
  wendet den Schlag auch unter B ab (18.950 unverändert) — die gierige Suche erreicht sie nur nicht
  mehr.
- **Nebenfrage `GCD_MS`:** In Rotationen mit Füller ist der Unterschied folgenlos (alle vier
  Brände unter A und B identisch). Ohne Füller legt B früher nach; bei SEARING kostet das einen
  Tick (1260 → 1170 Schaden in 30 s), weil Erneuern die Restzeit verwirft.
- **Handgeschriebener Bestand:** 97 von 2394 Läufen (57 Dateien × 42 Bosse) ändern sich, und zwar
  genau die 6 Dateien mit einer `quench`-Zeile (6 von 6; 0 von 51 ohne). `step1-good.apl` und
  `step3-good.apl` bleiben an allen 42 Bossen gleich.

Was das für die Entscheidung heißt, steht unter [Für die Entscheidung](#für-die-entscheidung).

## Was Fassung B hier heißt

`switch_point` bekommt den Vergleichsoperator und legt den Weckruf auf den ersten 10-ms-Takt, an dem
der Vergleich tatsächlich kippt. Bei vertauschten Seiten wird die Form gespiegelt (`T < v` ist
`v > T`). Fallende Restzeiten kippen bei `<` und `>=` mit dem Zielwert `T - 1`, bei `>` und `<=` mit
`T`, bei `==` und `!=` an beiden; die steigende Uhr entsprechend bei `T` bzw. `T + 1`. Der Zeitpunkt
wird auf den nächsten Takt aufgerundet, weil das Spiel seinen Zustand nur an Takt-Grenzen prüft
(`sim/src/time.rs`, `TICK_MS = 10`).

Unverändert wie am 06.09.: Wahrheitswerte (`cooldown.*.ready`, `dot.*.active`), Verstärkungen,
Vergleiche zweier zeitabhängiger Seiten und die GCD-Sperre in `fight.rs`. `fight.rs` ist in keiner
Fassung angefasst.

Der Patch (Datei-SHA-256 `d95aada2…`, erzeugt aus dem Bestand, reproduziert B bytegenau mit dem
SHA-256 `3680b9c5…`). Im Text unten hat prettier die Leerzeichen der leeren Kontextzeilen entfernt;
`git apply` ergibt aus diesem Text trotzdem bytegenau `3680b9c5…` (geprüft):

<details>
<summary>Fassung B, 126 Zeilen</summary>

```diff
--- a/sim/src/apl/mod.rs
+++ b/sim/src/apl/mod.rs
@@ -121,7 +121,7 @@

 use crate::data::{Spell, MAX_LEVEL, RESOURCE, SPELLS};
 use crate::fight::Fight;
-use crate::time::{Millis, GCD_MS};
+use crate::time::{Millis, GCD_MS, TICK_MS};

 /// Eine Zahl, wie die Sprache sie kennt: ganzzahlig, mit Vorzeichen.
 ///
@@ -382,6 +382,18 @@
 }

 impl Compare {
+    /// FASSUNG B (#1027, Wegwerf-Patch): die Form bei vertauschten Seiten.
+    const fn reversed(self) -> Self {
+        match self {
+            Compare::Less => Compare::Greater,
+            Compare::LessEqual => Compare::GreaterEqual,
+            Compare::Greater => Compare::Less,
+            Compare::GreaterEqual => Compare::LessEqual,
+            Compare::Equal => Compare::Equal,
+            Compare::NotEqual => Compare::NotEqual,
+        }
+    }
+
     #[must_use]
     const fn apply(self, left: Number, right: Number) -> bool {
         match self {
@@ -515,7 +527,10 @@

         for rule in &self.rules {
             for term in rule.condition.terms() {
-                for (value, other) in [(term.left, term.right), (term.right, term.left)] {
+                for (value, other, compare) in [
+                    (term.left, term.right, term.compare),
+                    (term.right, term.left, term.compare.reversed()),
+                ] {
                     if !value.is_time_dependent() {
                         continue;
                     }
@@ -529,7 +544,7 @@
                     if other.is_time_dependent() {
                         continue;
                     }
-                    if let Some(at) = switch_point(value, other.read(fight), fight) {
+                    if let Some(at) = switch_point(value, other.read(fight), compare, fight) {
                         if at > now {
                             earliest = Some(earliest.map_or(at, |best| best.min(at)));
                         }
@@ -567,16 +582,43 @@
 /// Wert an der Uhr?" statt an der Frage „wann schlägt er um?". Die zweite ist
 /// die, deren Antwort gebraucht wird. Ohne Auffang-Zweig muss jeder neue Wert
 /// hier beantwortet werden — auch mit `None`, dann aber sichtbar und begründet.
-fn switch_point(value: Value, threshold: Number, fight: &Fight) -> Option<Millis> {
+fn switch_point(value: Value, threshold: Number, compare: Compare, fight: &Fight) -> Option<Millis> {
     let now = fight.time();
-    let at = |remaining_now: Millis, target: Number| -> Option<Millis> {
-        // Ein Restwert sinkt je Millisekunde um eins: Er erreicht `target` in
-        // `remaining_now - target` Millisekunden.
-        let delta = Number::from(remaining_now) - target;
-        if delta <= 0 {
-            return None;
+    // FASSUNG B (#1027, Wegwerf-Patch, nie einchecken). Der Weckruf liegt auf
+    // dem ersten Takt, an dem der Vergleich kippt: Das Spiel prueft seinen
+    // Zustand nur an Takt-Grenzen (time::TICK_MS).
+    let auf_takt = |t: Millis| -> Option<Millis> {
+        let rest = t % TICK_MS;
+        if rest == 0 {
+            Some(t)
+        } else {
+            t.checked_add(TICK_MS - rest)
         }
-        u32::try_from(delta).ok().map(|d| now.saturating_add(d))
+    };
+    // Fallender Wert v(now + d) = remaining_now - d. Er kippt, wenn v den
+    // Zielwert erreicht: < und >= bei threshold - 1, > und <= bei threshold,
+    // == und != bei beiden. Restwerte enden bei 0; ein negativer Zielwert wird
+    // nie erreicht und darf keinen Weckruf stellen.
+    let at = |remaining_now: Millis, threshold: Number| -> Option<Millis> {
+        let ziele: [Option<Number>; 2] = match compare {
+            Compare::Less | Compare::GreaterEqual => [threshold.checked_sub(1), None],
+            Compare::Greater | Compare::LessEqual => [Some(threshold), None],
+            Compare::Equal | Compare::NotEqual => [Some(threshold), threshold.checked_sub(1)],
+        };
+        ziele
+            .into_iter()
+            .flatten()
+            .filter_map(|ziel| {
+                if ziel < 0 {
+                    return None;
+                }
+                let delta = Number::from(remaining_now).checked_sub(ziel)?;
+                if delta <= 0 {
+                    return None;
+                }
+                auf_takt(now.checked_add(u32::try_from(delta).ok()?)?)
+            })
+            .min()
     };

     match value {
@@ -591,8 +633,20 @@
             (remains > 0).then(|| now.saturating_add(remains))
         }
         Value::Time => {
-            let target = u32::try_from(threshold).ok()?;
-            (target > now).then_some(target)
+            // Steigende Uhr: < und >= kippen bei threshold, > und <= bei
+            // threshold + 1, == und != bei beiden.
+            let ziele: [Option<Number>; 2] = match compare {
+                Compare::Less | Compare::GreaterEqual => [Some(threshold), None],
+                Compare::Greater | Compare::LessEqual => [threshold.checked_add(1), None],
+                Compare::Equal | Compare::NotEqual => [Some(threshold), threshold.checked_add(1)],
+            };
+            ziele
+                .into_iter()
+                .flatten()
+                .filter_map(|ziel| u32::try_from(ziel).ok())
+                .filter_map(auf_takt)
+                .filter(|&ziel| ziel > now)
+                .min()
         }
         Value::FightRemains => at(fight.fight_remaining_ms(), threshold),
         // Der Wert, für den diese Funktion exhaustiv gemacht wurde (#797). Er
```

</details>

Für die offene Frage aus `apl_strikte_vergleiche.rs` lief zusätzlich die **Mutation M**: der Versuch
vom 22.08., `Value::BossNextStrikeIn => None` (Patch-SHA-256 `49f3e394…`, ergibt `7e77a7af…`).

## Aufbau und Befehle

Das Messprogramm `sim/examples/e1027-operator-measure.rs` (SHA-256 `802bbea3…`) ist für alle drei
Fassungen dieselbe Datei. Gebaut wurde remote, gefahren lokal nach `-c`; alle Aufrufe aus der
Worktree-Wurzel:

```bash
cargo remote -c -- build --release --example e1027-operator-measure   # je Fassung
M=./target/release/examples/e1027-operator-measure                    # nach /work/tmp gesichert
$M > a.jsonl            # Bestand; dasselbe mit dem B- und dem M-Bau
$M kreuz a.jsonl b.jsonl > kreuz-a.jsonl      # mit dem A-Bau; kreuz-b.jsonl mit dem B-Bau
$M vergleich a.jsonl b.jsonl kreuz-a.jsonl kreuz-b.jsonl > vergleich-ab.json
$M vergleich a.jsonl m.jsonl > vergleich-am.json
$M vergleich a.jsonl a2.jsonl > vergleich-aa.json
```

| Fassung | `mod.rs` SHA-256 | Bau-Job                    | Binary SHA-256 | Ausgabe SHA-256 | Wiederholung |
| ------- | ---------------- | -------------------------- | -------------- | --------------- | ------------ |
| A       | `377742d1…`      | `d0f1e66b997d8911a11da052` | `c3cdf951…`    | `8f473dfe…`     | bytegleich   |
| B       | `3680b9c5…`      | `d712e0ae97d184e78177196d` | `b304bc21…`    | `16f1c7de…`     | bytegleich   |
| M       | `7e77a7af…`      | `b39865a9d6a2a8da5f68a06d` | `2db4b5ac…`    | `28e39049…`     | —            |

Alle fünf Messläufe (A, A wiederholt, B, B wiederholt, M): Exit 0, keine Fehlerausgabe, je 2731
Datensätze — 1 Metadatensatz, 144 Zustandsproben, 84 Wurfproben, 8 Rasterproben, 16 GCD-Proben, 2394
Bestandsläufe, 84 Suchen. Beide Kreuzläufe: Exit 0, je 84 Datensätze. Die Bosszahl liest das
Programm zweimal und verlangt Gleichheit: `BOSS_COUNT` = 42 und 42 Dateien in `data/bosses/`. A gegen
A ergibt in allen 2731 Datensätzen 0 Änderungen. Der Vergleich bricht ab, wenn sich Fallmengen
unterscheiden oder die Kreuzbewertung die eigene Suche nicht bitgleich trifft. Er zählt außerdem,
ob sich ein Wahrheitswert ändert (0 Fälle) und ob die Takt-Referenz unter A und B verschieden
ausfällt (0 Fälle).

**`idle_ms` steht in keinem Fingerabdruck.** Es ist neben jedem Ergebnis gespeichert, aber nicht
vergleichbar; siehe [Nebenbefunde](#nebenbefunde).

## Zustandsproben: Wahrheitswert und Weckruf

An vier Werten, die nicht an der Kampfdauer hängen, je einen Takt vor, auf und nach dem Umschlag
(144 Proben): Der **Wahrheitswert ändert sich in keiner Probe**, der Weckruf in 48. Das Muster ist
an allen vier Werten dasselbe, hier an `dot.smoulder.remains` (Umschlag 10500, Schwelle 1500); die
gespiegelten Formen (`1500 > v` wie `v < 1500`) verhalten sich wie ihr Gegenstück:

| Zeitpunkt | Form                  | A: Weckruf | B: Weckruf |
| --------- | --------------------- | ---------- | ---------- |
| 10490     | `<`, `>=`             | 10500      | 10510      |
| 10490     | `<=`, `>`, `==`, `!=` | 10500      | 10500      |
| 10500     | `<`, `>=`, `==`, `!=` | keiner     | 10510      |
| 10500     | `<=`, `>`             | keiner     | keiner     |
| 10510     | alle sechs            | keiner     | keiner     |

**Am Umschlag stellt A für keine Form einen Weckruf** — auch nicht für `<` und `!=`, die dort falsch
sind und einen Takt später wahr werden. Das ist der fehlende zweite Weckruf, direkt gemessen.

## Würfe aller sechs Formen gegen die Takt-Referenz

Die Messlatte ist kein hergeleiteter Zeitpunkt, sondern ein zweiter Lauf: dieselbe Liste plus eine
angehängte Zeile, die nie greift und für jeden Takt des Fensters `time >= t and time < 0` trägt.
Sie weckt an jedem Takt und wirft nie; `>=` auf der steigenden Uhr kippt in A und B bei `t`, die
Referenz hängt also an keiner Fassung (gemessen: unter A und B dieselben Würfe).

| Wert                     | Boss                     | Umschlag | A trifft | B trifft | verspätet unter A                 |
| ------------------------ | ------------------------ | -------: | -------: | -------: | --------------------------------- |
| `boss.next_strike_in`    | `test-survivable-strike` |     9000 |   8 / 12 |  12 / 12 | `<`, `3000 >`, `!=` ×2 (10000)    |
| `boss.next_strike_in`    | `boss-1-2`               |     7000 |  10 / 12 |  12 / 12 | `<`, `3000 >` (9000)              |
| `dot.smoulder.remains`   | `first-crucible`         |    10500 |   8 / 12 |  12 / 12 | `<`, `1500 >`, `!=` ×2 (12000)    |
| `boss.cast_remains`      | `boss-1-2`               |    27500 |  10 / 12 |  12 / 12 | `<`, `500 >`: wirft nie           |
| `cooldown.purge.remains` | `test-survivable-strike` |     6000 |   8 / 12 |  12 / 12 | `<`, `1500 >`, `!=` ×2: wirft nie |
| `fight.remains`          | `test-survivable-strike` |    57000 |   8 / 12 |  12 / 12 | `<`, `3000 >`, `!=` ×2 (58000)    |
| `time`                   | `test-survivable-strike` |     9000 |   8 / 12 |  12 / 12 | `>`, `9000 <`, `!=` ×2 (10000)    |
| **Summe**                |                          |          |  60 / 84 |  84 / 84 | 24                                |

Unter B liegen alle 24 verspäteten Würfe genau auf dem ersten Referenzwurf (9010, 7010, 10510,
27510, 6010, 57010, 9010). Bei `boss-1-2` trifft `!=` auch unter A (beide 9000). „Wirft nie" heißt:
Das Fenster schließt sich, bevor ein fremdes Ereignis weckt — der Cast landet bei 28000, `purge`
wird bei 7500 wieder gewirkt.

## Gefundene Rotationen je Boss

Zwei Sucheinstellungen, beide aus dem Bestand übernommen, je Boss eine gierige Suche
(`rotation_search::greedy`, alle Kandidaten, `stop_on_kill: false`, Startwert 0, ein Thread):

- `bericht` wie `config()` in `sim/examples/rotation-search.rs`: 50 Durchgänge, Länge 6, Gewicht 0 —
  ohne dessen bossübergreifenden Nachlauf
- `kontrolle` wie `konfiguration()` in `sim/tests/ac5_kontrolle_lohnt_sich.rs`: Gewicht 1 auf
  Verhindertes, Länge 10

**Eine Rotation gilt als verschoben, wenn die gefundene Liste eine andere ist.** In allen 84 Suchen
fallen Liste, Suchwert und Nachlauf-Fingerabdruck zusammen: Wo sich eines ändert, ändern sich alle
drei.

| Einstellung | Bosse | Liste verschoben | Anteil | A-Liste unter B anders | Nachbarn verschoben    |
| ----------- | ----: | ---------------: | -----: | ---------------------: | ---------------------- |
| `bericht`   |    42 |                2 |  4,8 % |                      2 | 1490 / 25790 (5,78 %)  |
| `kontrolle` |    42 |               13 | 31,0 % |                      5 | 3869 / 33373 (11,59 %) |
| zusammen    |    84 |               15 | 17,9 % |                      7 | 5359 / 59163 (9,06 %)  |

Die 15 verschobenen Suchen liegen an 15 verschiedenen Bossen; kein Boss verschiebt sich in beiden
Einstellungen. Der Suchwert sinkt in 10 und steigt in 5 Fällen; die größten Ausschläge sind
`test-lethal-strike` (−38,6 %, siehe AC5), `orch-maxload` (−10,3 %), `test-onslaught` (−9,9 %),
`test-caster` (−8,3 %), `test-metronome` (−7,7 %) und `boss-2-4` (−7,5 %).

- **„A-Liste unter B anders"** bewertet die unter A gefundene Liste unter B — dieselbe Rotation,
  andere Semantik. Das trennt „die Zahl einer bestehenden Rotation verschiebt sich" von „die Suche
  wählt eine andere".
- **„Nachbarn"** sind genau die Listen, die der nächste Suchschritt ansähe: jede Zeile der A-Liste
  einmal entfernt, jeder übrige Kandidat an jeder Stelle eingefügt. Sie zeigen, wie empfindlich die
  Bewertung in der Nähe des Gefundenen ist.

`orch-maxload` ist laut eigener Datei ein Messwerkzeug („KEIN SPIELINHALT") mit Bossheilung. Seine
A-Liste fällt unter B von 696 auf 0: Sie unterbricht 5- statt 7-mal. Dass die Bossheilung
(`bossHeal` 1600) den Nettoschaden aufzehrt, ist aus den Daten geschlossen, nicht eigens gemessen.

| Boss                     | `bericht` A → B   | `kontrolle` A → B   | A-Liste unter B (`bericht` / `kontrolle`) | Nachbarn verschoben `bericht` | Nachbarn verschoben `kontrolle` | Bestand verschoben |
| ------------------------ | ----------------- | ------------------- | ----------------------------------------- | ----------------------------: | ------------------------------: | -----------------: |
| `boss-1-1`               | 9.272 (gleich)    | **14.130 → 13.833** | gleich / **14.130 → 13.624**              |              13 / 615 (2,1 %) |              698 / 923 (75,6 %) |             3 / 57 |
| `boss-1-2`               | 7.544 (gleich)    | 8.459 (gleich)      | gleich / gleich                           |               6 / 622 (1,0 %) |                17 / 934 (1,8 %) |             1 / 57 |
| `boss-1-3`               | 9.364 (gleich)    | 10.520 (gleich)     | gleich / gleich                           |               5 / 615 (0,8 %) |                 7 / 773 (0,9 %) |             1 / 57 |
| `boss-1-4`               | 8.163 (gleich)    | 9.439 (gleich)      | gleich / gleich                           |             76 / 615 (12,4 %) |                71 / 773 (9,2 %) |             5 / 57 |
| `boss-2-1`               | 8.522 (gleich)    | 15.634 (gleich)     | gleich / gleich                           |               9 / 615 (1,5 %) |                13 / 923 (1,4 %) |             1 / 57 |
| `boss-2-2`               | 8.183 (gleich)    | **15.563 → 15.844** | gleich / **15.563 → 15.595**              |            249 / 615 (40,5 %) |              736 / 923 (79,7 %) |             3 / 57 |
| `boss-2-3`               | 11.392 (gleich)   | **13.375 → 13.487** | gleich / gleich                           |               8 / 615 (1,3 %) |              214 / 923 (23,2 %) |             3 / 57 |
| `boss-2-4`               | **8.123 → 7.511** | 9.764 (gleich)      | **8.123 → 6.570** / gleich                |            535 / 615 (87,0 %) |                 8 / 695 (1,2 %) |             6 / 57 |
| `boss-3-1`               | 7.744 (gleich)    | 11.882 (gleich)     | gleich / gleich                           |               6 / 615 (1,0 %) |                17 / 923 (1,8 %) |             5 / 57 |
| `boss-3-2`               | 9.364 (gleich)    | 9.693 (gleich)      | gleich / gleich                           |               5 / 615 (0,8 %) |                11 / 923 (1,2 %) |             1 / 57 |
| `boss-3-3`               | 9.364 (gleich)    | 12.448 (gleich)     | gleich / gleich                           |               4 / 615 (0,7 %) |                16 / 923 (1,7 %) |             1 / 57 |
| `boss-3-4`               | 8.721 (gleich)    | 9.813 (gleich)      | gleich / gleich                           |               7 / 622 (1,1 %) |                26 / 859 (3,0 %) |             2 / 57 |
| `boss-4-1`               | 11.224 (gleich)   | 11.630 (gleich)     | gleich / gleich                           |               6 / 615 (1,0 %) |                 7 / 849 (0,8 %) |             1 / 57 |
| `boss-4-2`               | 9.364 (gleich)    | 9.364 (gleich)      | gleich / gleich                           |               5 / 615 (0,8 %) |                 5 / 615 (0,8 %) |             1 / 57 |
| `boss-4-3`               | 10.553 (gleich)   | 15.108 (gleich)     | gleich / gleich                           |               8 / 615 (1,3 %) |                11 / 923 (1,2 %) |             1 / 57 |
| `boss-4-4`               | 9.364 (gleich)    | 13.415 (gleich)     | gleich / gleich                           |              11 / 615 (1,8 %) |                15 / 923 (1,6 %) |             6 / 57 |
| `boss-5-1`               | 9.364 (gleich)    | 14.446 (gleich)     | gleich / gleich                           |               5 / 615 (0,8 %) |                15 / 923 (1,6 %) |             1 / 57 |
| `boss-5-2`               | 8.550 (gleich)    | 8.977 (gleich)      | gleich / gleich                           |               5 / 615 (0,8 %) |                 7 / 695 (1,0 %) |             1 / 57 |
| `boss-5-3`               | 10.922 (gleich)   | **12.381 → 12.614** | gleich / gleich                           |              12 / 615 (2,0 %) |               77 / 615 (12,5 %) |             3 / 57 |
| `boss-5-4`               | 8.721 (gleich)    | 9.464 (gleich)      | gleich / gleich                           |               7 / 622 (1,1 %) |                 8 / 782 (1,0 %) |             1 / 57 |
| `first-crucible`         | 9.364 (gleich)    | 9.742 (gleich)      | gleich / gleich                           |               4 / 615 (0,7 %) |              154 / 773 (19,9 %) |             1 / 57 |
| `orch-maxload`           | **696 → 624**     | 139 (gleich)        | **696 → 0** / gleich                      |            349 / 622 (56,1 %) |                 0 / 187 (0,0 %) |             3 / 57 |
| `test-caster`            | 48.672 (gleich)   | **53.859 → 49.415** | gleich / gleich                           |              11 / 615 (1,8 %) |                16 / 923 (1,7 %) |             4 / 57 |
| `test-collision`         | 9.364 (gleich)    | 12.948 (gleich)     | gleich / gleich                           |              10 / 615 (1,6 %) |                22 / 923 (2,4 %) |             3 / 57 |
| `test-crucible-trial`    | 18.990 (gleich)   | 21.559 (gleich)     | gleich / gleich                           |               7 / 615 (1,1 %) |                16 / 773 (2,1 %) |             6 / 57 |
| `test-doomsayer`         | 5.921 (gleich)    | 6.047 (gleich)      | gleich / gleich                           |               9 / 622 (1,4 %) |                 8 / 703 (1,1 %) |             1 / 57 |
| `test-gatekeeper`        | 18.990 (gleich)   | 20.058 (gleich)     | gleich / gleich                           |               3 / 615 (0,5 %) |              115 / 849 (13,5 %) |             1 / 57 |
| `test-lapse`             | 7.566 (gleich)    | 8.753 (gleich)      | gleich / gleich                           |               6 / 615 (1,0 %) |                 7 / 773 (0,9 %) |             1 / 57 |
| `test-lapse-interrupt`   | 9.364 (gleich)    | **9.364 → 9.553**   | gleich / gleich                           |               5 / 615 (0,8 %) |                 5 / 615 (0,8 %) |             1 / 57 |
| `test-lethal-strike`     | 8.440 (gleich)    | **18.950 → 11.627** | gleich / gleich                           |               2 / 615 (0,3 %) |                22 / 923 (2,4 %) |             1 / 57 |
| `test-metronome`         | 48.672 (gleich)   | **53.859 → 49.711** | gleich / gleich                           |              15 / 615 (2,4 %) |                26 / 923 (2,8 %) |             2 / 57 |
| `test-onslaught`         | 9.014 (gleich)    | **13.656 → 12.301** | gleich / **13.656 → 13.727**              |               9 / 615 (1,5 %) |              479 / 695 (68,9 %) |             2 / 57 |
| `test-phase`             | 48.672 (gleich)   | 50.390 (gleich)     | gleich / gleich                           |               4 / 615 (0,7 %) |                 9 / 773 (1,2 %) |             1 / 57 |
| `test-phasenbindung`     | 48.672 (gleich)   | **50.244 → 49.999** | gleich / **50.244 → 49.415**              |               7 / 615 (1,1 %) |              423 / 695 (60,9 %) |             4 / 57 |
| `test-racer`             | 17.798 (gleich)   | **19.036 → 18.984** | gleich / gleich                           |              10 / 615 (1,6 %) |                14 / 773 (1,8 %) |             4 / 57 |
| `test-resistant`         | 37.902 (gleich)   | **38.337 → 38.721** | gleich / gleich                           |              11 / 615 (1,8 %) |                10 / 695 (1,4 %) |             4 / 57 |
| `test-runaway`           | 3.361 (gleich)    | 3.361 (gleich)      | gleich / gleich                           |               1 / 533 (0,2 %) |                 1 / 533 (0,2 %) |             1 / 57 |
| `test-stacked`           | 7.504 (gleich)    | 8.123 (gleich)      | gleich / gleich                           |               9 / 622 (1,4 %) |                 9 / 782 (1,2 %) |             1 / 57 |
| `test-survivable-strike` | 10.244 (gleich)   | 11.619 (gleich)     | gleich / gleich                           |               8 / 615 (1,3 %) |                 7 / 773 (0,9 %) |             1 / 57 |
| `test-vessel`            | 14.047 (gleich)   | 17.792 (gleich)     | gleich / gleich                           |               9 / 615 (1,5 %) |                14 / 923 (1,5 %) |             1 / 57 |
| `test-vulnerable`        | 59.390 (gleich)   | 59.718 (gleich)     | gleich / gleich                           |              10 / 615 (1,6 %) |                11 / 773 (1,4 %) |             4 / 57 |
| `test-warden`            | 9.364 (gleich)    | **9.984 → 9.904**   | gleich / **9.984 → 9.901**                |               9 / 615 (1,5 %) |              522 / 773 (67,5 %) |             3 / 57 |

## Der #797-AC5-Prüffall unter Fassung B

```bash
cargo remote --no-copy-lock -- test --test ac5_kontrolle_lohnt_sich -- --nocapture
```

| Fassung | Job                        | bestanden | rot | ignoriert | roter Fall                                                      |
| ------- | -------------------------- | --------: | --: | --------: | --------------------------------------------------------------- |
| B       | `ba1de25f480302bf0dd08d8a` |         2 |   1 |         1 | `ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist` |

Meldung: „Gewicht 1: keine FIXATION-Zeile am tödlichen Boss". Unter A: 3 bestanden, 0 rot,
1 ignoriert (voller Lauf, siehe [Prüffälle](#prüffälle)).

Unter B findet die Suche bei `test-lethal-strike` die Liste `searing if dot.searing.remains < 1500`,
`aether`, `plate`, `crucible-skin`, `crust`, `riposte`, `efflux`, `kindle` — **Zeile für Zeile
dieselbe wie im 1-ms-Versuch**, mit demselben Suchwert 11.627 und dem Spielertod bei 12.000 ms. Unter
A sind es `pin`, `quench` und `bind` mehr, Suchwert 18.950.

**Die FIXATION-Rotation wird unter B nicht schlechter.** Die A-Liste erzielt unter B unverändert
18.950 (Kreuzbewertung) und überlebt. Die gierige Suche kommt unter B nur nie an die Stelle, an der
sie eine Dreiergruppe prüfen dürfte: Einzelzüge tragen bis Länge 8, bei Grenze 10 ist dann kein
Platz für drei Zeilen. Genau daran brechen unter B auch
`ueber_greedy_wendet_die_suche_den_toedlichen_schlag_ab` (`greedy_traegt_die_gruppen.rs`) und
`der_schalter_schaltet`, `erst_mit_dreiergruppen_findet_die_suche_die_zeile` (`tripel_suche.rs`;
Meldung: keine einzige Dreiergruppe geprüft, Stopp `NoImprovement`, Länge 8).

Die Gewichtsreihe des Prüffalls (`--ignored die_gewichtsreihe_zeigt_ein_fenster`):

| Gewicht                            | 0   | 1      | 2      | 5      | 10     | 20     | 50     | 100    |
| ---------------------------------- | --- | ------ | ------ | ------ | ------ | ------ | ------ | ------ |
| A (Job `e847a311a3ac5718be3a847f`) | —   | trennt | trennt | trennt | trennt | trennt | trennt | trennt |
| B (Job `73cb489b29972d79261ae808`) | —   | —      | trennt | trennt | trennt | trennt | trennt | trennt |

„Trennt" heißt `pin` und `bind` am tödlichen Boss, keine FIXATION am Zwilling. Unter B verliert das
Fenster seinen unteren Rand 1; ab 2 trennen beide Fassungen.

## Die Nebenfrage: `< GCD_MS`

Die Suche erzeugt für jeden der vier Brände die Zeile `dot.<brand>.remains < 1500` (`GCD_MS`).
Gemessen an `first-crucible` über 30 s, je mit und ohne Füller und gegen die Takt-Referenz über die
ganze Zeit:

| Brand / Liste                   | A: Nachlegen bei           | A: Schaden, Ticks | B: Nachlegen bei           | B: Schaden, Ticks | B = Takt-Referenz |
| ------------------------------- | -------------------------- | ----------------- | -------------------------- | ----------------- | ----------------- |
| `cinder` `<= 1500` allein       | 3000, 6000, 9000, 12000 …  | 870, 29           | 3000, 6000, 9000, 12000 …  | 870, 29           | ja                |
| `cinder` `<= 1500` + `kindle`   | 3000, 6000, 9000, 12000 …  | 1870, 29          | 3000, 6000, 9000, 12000 …  | 1870, 29          | ja                |
| `cinder` `< 1500` allein        | 3000, 6000, 9000, 12000 …  | 870, 29           | 3000, 6000, 9000, 12000 …  | 870, 29           | ja                |
| `cinder` `< 1500` + `kindle`    | 3000, 6000, 9000, 12000 …  | 1870, 29          | 3000, 6000, 9000, 12000 …  | 1870, 29          | ja                |
| `rot` `<= 1500` allein          | 4000, 8000, 12000, 16000 … | 917, 29           | 4000, 8000, 12000, 16000 … | 917, 29           | ja                |
| `rot` `<= 1500` + `kindle`      | 4500, 9000, 13500, 18000 … | 2439, 26          | 4500, 9000, 13500, 18000 … | 2439, 26          | ja                |
| `rot` `< 1500` allein           | 4000, 8000, 12000, 16000 … | 917, 29           | 4000, 8000, 12000, 16000 … | 917, 29           | ja                |
| `rot` `< 1500` + `kindle`       | 4500, 9000, 13500, 18000 … | 2439, 26          | 4500, 9000, 13500, 18000 … | 2439, 26          | ja                |
| `searing` `<= 1500` allein      | 4500, 9000, 13500, 18000 … | 1170, 13          | 4500, 9000, 13500, 18000 … | 1170, 13          | ja                |
| `searing` `<= 1500` + `kindle`  | 4500, 9000, 13500, 18000 … | 2470, 13          | 4500, 9000, 13500, 18000 … | 2470, 13          | ja                |
| `searing` `< 1500` allein       | 6000, 12000, 18000, 24000  | 1260, 14          | 4510, 9020, 13530, 18040 … | 1170, 13          | ja                |
| `searing` `< 1500` + `kindle`   | 6000, 12000, 18000, 24000  | 2760, 14          | 6000, 12000, 18000, 24000  | 2760, 14          | ja                |
| `smoulder` `<= 1500` allein     | 10500, 21000               | 560, 14           | 10500, 21000               | 560, 14           | ja                |
| `smoulder` `<= 1500` + `kindle` | 10500, 21000               | 2260, 14          | 10500, 21000               | 2260, 14          | ja                |
| `smoulder` `< 1500` allein      | 12000, 24000               | 560, 14           | 10510, 21020               | 560, 14           | ja                |
| `smoulder` `< 1500` + `kindle`  | 12000, 24000               | 2260, 14          | 12000, 24000               | 2260, 14          | ja                |

**Antwort: mit Füller folgenlos, ohne Füller wirkt B — und nicht zum Vorteil.** Mit `kindle` darunter
entscheidet die Rotation an GCD-Enden. SMOULDER (12000 ms) und SEARING (6000 ms) sind Vielfache von
1500: Ihre Restzeit steht an einem GCD-Ende genau auf 1500, wo `< 1500` falsch ist. Dort wird der
Füller gewirkt, und der Weckruf von B einen Takt später fällt in dessen GCD — A und B legen beide erst
am Ablauf nach, mit identischen Würfen und Schaden. Ohne Füller wartet die Rotation; B legt dann einen
Takt nach dem Umschlag nach (10510, 4510), A erst am Ablauf. Weil Erneuern die Restzeit verwirft
(`refreshMode: reset`), kostet das bei SEARING einen Tick. CINDER und ROT ändern sich nicht. Ob der
Fehler bei `GCD_MS` „am schärfsten" wirkt: Er verschiebt in vollen Rotationen nichts, und in leeren
macht B das Nachlegen früher, nicht besser.

Aus derselben Tabelle, unabhängig von der Fassung: **Mit Füller ist die strikte Form bei SEARING die
bessere** — `< 1500` legt am Ablauf nach und behält alle 14 Ticks (2760 Schaden), `<= 1500` legt
einen GCD früher nach und verwirft einen (2470). Bei SMOULDER, CINDER und ROT ergeben beide Formen
dasselbe.

## Handgeschriebener Bestand

Jede der 57 Dateien aus `sim/apl/` gegen jeden der 42 Bosse, ein Kampf je Paar (Startwert 0, volle
Zeit): **97 von 2394 Läufen (4,05 %)** ändern sich, an jedem Boss mindestens einer.

| Datei                              | verschoben an Bossen |
| ---------------------------------- | -------------------: |
| `e607p-paar.apl`                   |              42 / 42 |
| `e558-paar-bedingt.apl`            |              15 / 42 |
| `e797-mit-quench.apl`              |              10 / 42 |
| `instance1-boss4-unterbrechen.apl` |              10 / 42 |
| `instance1-rein.apl`               |              10 / 42 |
| `instance1-unterbrechen.apl`       |              10 / 42 |
| übrige 51 Dateien                  |               0 / 42 |

Gemeinsam ist genau diesen sechs eine `quench`-Zeile — der einzige Zauber mit `triggersGcd: false` —
neben einer Nachlege-Zeile `< 1500` bzw. `< gcd`. Keine der 51 übrigen Dateien hat eine
`quench`-Zeile. Von den 97 Läufen ändert sich in 95 der Schaden, in 33 der Siegzeitpunkt, in keinem der Spielertod.
**Für die Suchen trennt `quench` nicht so sauber**: Es steht in 10 der 15 verschobenen, aber auch in
6 der 69 unveränderten Listen.

## Woher der Verzug kommt

Die offene Frage aus `sim/tests/apl_strikte_vergleiche.rs` („Woher dieses Raster kommt, ist offen")
ist beantwortet. Die Aufbauten der dortigen Prüffälle, ohne deren `time < grenze`, unter A, M und B:

| Boss                     | Zeile                                   | `purge` dahinter (gewirkt bei) |     A |     M |    B | Takt-Referenz |
| ------------------------ | --------------------------------------- | ------------------------------ | ----: | ----: | ---: | ------------: |
| `test-survivable-strike` | `kindle if boss.next_strike_in <= 3000` | ja (0, 7500)                   |  9000 |  9000 | 9000 |          9000 |
| `test-survivable-strike` | `kindle if boss.next_strike_in < 3000`  | ja (0, 7500)                   | 10000 | 10000 | 9010 |          9010 |
| `test-survivable-strike` | `kindle if boss.next_strike_in <= 3000` | nein                           |  9000 | 10000 | 9000 |          9000 |
| `test-survivable-strike` | `kindle if boss.next_strike_in < 3000`  | nein                           | 10000 | 10000 | 9010 |          9010 |
| `boss-1-2`               | `kindle if boss.next_strike_in <= 3000` | ja (0)                         |  7000 |  7000 | 7000 |          7000 |
| `boss-1-2`               | `kindle if boss.next_strike_in < 3000`  | ja (0)                         |  7500 |  7500 | 7010 |          7010 |
| `boss-1-2`               | `kindle if boss.next_strike_in <= 3000` | nein                           |  7000 |  7000 | 7000 |          7000 |
| `boss-1-2`               | `kindle if boss.next_strike_in < 3000`  | nein                           |  8000 |  8000 | 7010 |          7010 |

- **Die erste Erklärung war richtig.** Am Umschlag ist `<` falsch, und A stellt keinen zweiten
  Weckruf (siehe Zustandsproben); B legt `<` auf 9010 bzw. 7010.
- **Die Mutation vom 22.08. konnte das nicht zeigen.** In ihren Aufbauten weckt am Umschlag ohnehin
  etwas: `purge` wird bei 7500 gewirkt, sein GCD endet bei 9000; bei `boss-1-2` tickt das Verhängnis
  (`perStepMs` 1000) bei 7000. Ohne `purge` fällt `<= 3000` unter M von 9000 auf 10000.
- **Das Raster ist das nächste schon eingereihte Ereignis**: der Hitzeverfall alle 2000 ms
  (`decayPerStepMs`, 10000), das Ende der `purge`-Abklingzeit (7500), der Verhängnis-Takt (8000).

## Prüffälle

Neu in `sim/tests/apl_strikte_vergleiche.rs`: alle sechs Formen in beiden Reihenfolgen an
`boss.next_strike_in`, `dot.smoulder.remains` und `time` gegen die Takt-Referenz, die Trägerprüfung
der Referenz (an einem Fall mit zwei zeitabhängigen Seiten, den keine Fassung weckt) und der Wächter
`der_weckruf_traegt_ohne_fremdes_ereignis`. Die Tabellen halten Fassung A fest.

```bash
cargo remote --no-copy-lock -- test --test apl_strikte_vergleiche -- --nocapture
cargo remote --no-copy-lock -- test --no-fail-fast        # volle Suite
```

| Lauf | `apl_strikte_vergleiche.rs`                         | volle Suite `cargo remote -- test`                                                            |
| ---- | --------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| A    | 9 bestanden, 0 rot (Job `47e3e00d2b0d56eb99b69063`) | 203 Suiten: 1273 bestanden, 0 rot, 7 ignoriert, 0 gefiltert (Job `5535a9b84bdc2b68449e458c`)  |
| B    | 3 bestanden, 6 rot (Job `87a374827c1dd80ef3c167ee`) | 203 Suiten: 1263 bestanden, 10 rot, 7 ignoriert, 0 gefiltert (Job `5045840f84b9334d0884fda3`) |
| M    | 8 bestanden, 1 rot (im vollen Lauf)                 | 203 Suiten: 1269 bestanden, 4 rot, 7 ignoriert, 0 gefiltert (Job `58699a9b9a519faa75cdb957`)  |

Unter A zusätzlich wie die CI: `cargo-remote-fmt --check` Exit 0, `cargo remote -- clippy
--all-targets -- -D warnings` Exit 0 (Job `8368cd9fe85702722d46617a`), `cargo remote -- test
--all-targets --features diagnostics` 239 Läufe: 1308 bestanden, 0 rot, 7 ignoriert (Job
`a5ecbe260c56de7a0aabb0ed`).

- **Rot unter B** (10): in `apl_strikte_vergleiche.rs` die drei neuen Formtabellen und die drei
  bestehenden Verzugsfälle, dazu die vier Suchfälle aus dem AC5-Abschnitt. Jede neue Tabelle bricht
  an ihrer ersten verspäteten Zeile, mit (Ereignis, Referenz) = (9010, 9010) bzw. (10510, 10510).
- **Grün unter B** bleiben der Wächter, die Trägerprüfung der Referenz und die Messmethoden-Prüfung.
- **Rot unter M** (4): der neue Wächter (10000 statt 9000) und drei bestehende Fälle in
  `next_strike_in.rs`. Die vier älteren Fälle in `apl_strikte_vergleiche.rs` bleiben unter M grün —
  wie am 22.08. notiert.

## Für die Entscheidung

- **Der Preis von Fassung B, gemessen:** 2 von 42 Bossen in der Einstellung des Suchberichts, 13 von
  42 bei Gewicht 1 auf Verhindertes; 97 von 2394 Bestandsläufen; 7 bestehende Prüffälle (3 davon in
  `apl_strikte_vergleiche.rs`, die den Ist-Zustand ausdrücklich festhält) und die 3 neuen
  Formtabellen.
- **Was B richtig macht:** Jede Form wirft dort, wo das Spiel werfen könnte (84 von 84), und kein
  Wurf liegt neben dem Takt.
- **Was B nicht ändert:** Die Nachlege-Zeile `< 1500` legt in vollen Rotationen weiter erst am
  Ablauf nach — das ist die Strenge des Vergleichs am GCD-Ende, kein Weckruf-Fehler, und bei SEARING
  gemessen die bessere Wahl.
- **Was B kostet, ohne falsch zu sein:** Die AC5-Suche verliert FIXATION bei Gewicht 1, obwohl die
  FIXATION-Rotation unter B gleich viel wert ist. Das ist eine Eigenschaft des gierigen Suchpfads.

## Nebenbefunde

- **`idle_ms` zählt Leerlauf mehrfach — in beiden Fassungen.** `fight.rs` zählt beim Warten die
  Spanne bis zum nächsten bereits eingereihten Ereignis; der Weckruf der Entscheidung wird erst danach
  eingereiht. Gemessen: Die Takt-Referenz meldet 1.846.410 ms Leerlauf in einem 30-s-Kampf, und schon
  A meldet bei `searing if dot.searing.remains < 1500` (ohne Füller) 30.000 ms bei fünf gewirkten
  GCDs, also 7.500 ms mehr als möglich. Hier nicht behoben (`fight.rs` war nicht Auftrag); der Wert
  steht deshalb in keinem Fingerabdruck.
- **Zwei Formen fallen unter A ganz aus**, statt sich zu verspäten: `boss.cast_remains < 500` (wie
  bekannt) und `cooldown.purge.remains < 1500` — beide Fenster schließen sich vor dem nächsten fremden
  Ereignis.
- **`cargo-remote-fmt --check` stürzt bei Abweichungen im Client ab**, sobald eine Diff-Zeile für
  ihn gültiges JSON ohne Objekt ist: `AttributeError` in `render` (`client.py`). Der Exit-Code 1 ist
  dann nicht von „Format weicht ab" zu unterscheiden. Mit sauberem Format trat er nicht auf.
- **Die Spielseite hat keine zweite APL-Auswertung**: Suche nach `next_wakeup`, `nextWakeup` und
  `switchPoint` in `src/` und `tests/` ohne Treffer; `src/devtools/reference-run.ts` nutzt
  Rotationsfunktionen. Fassung B betrifft also nur den Simulator.

## Was nicht gemessen ist

- **Keine Laufzeit- oder Leistungsaussage.** Gebaut wurde auf dem Bau-Dienst, gefahren auf diesem
  Rechner unter Fremdlast; Zeiten stehen bewusst nicht in den Datensätzen.
- **Suchen nur in zwei Einstellungen**, ohne Ausrüstung, ohne Talente, ohne den bossübergreifenden
  Nachlauf aus `rotation-search.rs` und ohne Paar-/Gruppenvariation. Die Balance-Zahlen anderer
  Läufer (`rotation-balance.rs`, Machbarkeit) sind nicht nachgefahren.
- **Bestand und Proben je ein Kampf mit Startwert 0.** Ohne Ausrüstung würfelt nichts (`balance::run`
  reduziert dann auf einen Durchgang); mit Krit-Werten ist es nicht gemessen.
- **Fassung B ist eine mögliche Umsetzung**, nicht die einzige. Wahrheitswerte wie `dot.*.active`
  behandelt sie wie der Bestand — ob dort ein eigener Verzug steckt, ist nicht gemessen. Vergleiche
  zweier zeitabhängiger Seiten bleiben in beiden Fassungen ungeweckt (Trägerprüfung der Referenz,
  unter A und B grün: Referenz bei 6010, der ereignisgesteuerte Lauf später).

## Rohdaten

Messausgaben, Vergleiche, Patches und Binaries liegen unter `/work/tmp/w1-1027-mess/`,
`/work/tmp/w1-1027-patches/` und `/work/tmp/w1-1027-bin/` — **nicht dauerhaft**; die Befehle oben
erzeugen sie neu. Vergleich A gegen B: `vergleich-ab.json`, SHA-256 `c58cb5ba…`.
