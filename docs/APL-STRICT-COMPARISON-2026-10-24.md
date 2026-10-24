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

Gemessen am 24. Oktober 2026 auf `23d7c13fba470ee1d6a677bfdd64d042f05724f6`, am selben Tag zweimal
nachgebessert: nach Abnahme und Wirkungsprüfung, dann nach einer zweiten Wirkungsprüfung. Auftrag ist
die Antwort `f-apl-strikte-vergleiche-2` vom 22.08.: „Erst den Vorher-Nachher-Lauf, dann entscheiden".
**Eine Messung, keine Semantikänderung.** Fassung B lag nur als Wegwerf-Patch im Arbeitsbaum und ist
nicht eingecheckt. `sim/src/apl/mod.rs` hat im Bestand den SHA-256 `377742d1…`; dieser Zweig ergänzt
darin nur Kommentarzeilen (`77388248…`, siehe [Woher der Verzug kommt](#woher-der-verzug-kommt)). Vor
und nach jedem Lauf mit B, M, M2 oder MT steht die Datei wieder auf dem Stand dieses Zweigs, per
SHA-256 geprüft.

Gegenüber dem Versuch vom 06.09. ([APL-STRICT-COMPARISON-2026-09-06.md](APL-STRICT-COMPARISON-2026-09-06.md))
ändert sich zweierlei: Fassung B weckt auf dem **10-ms-Takt** statt auf der Millisekunde, und
gemessen wird über **alle 42 Bossdateien** statt über acht ausgewählte Suchen.

**Die Nachbesserung** beantwortet die Nebenfrage neu — die erste Fassung hatte aus Proben ohne
GCD-freien Zug auf alle vollen Rotationen geschlossen. Dazu kommen ein Wächter für den Weckruf von
`dot.*.remains` und drei Gegenproben im Messprogramm: die Suche mit `<=` in den Nachlegezeilen, die
Nachlegezeilen neben einem GCD-freien Zug gegen die Takt-Referenz und die Wurfreihe jeder gefundenen
Liste unter beiden Fassungen.

**Die zweite Nachbesserung** misst den Weg jeder Suche Schritt für Schritt und an der ersten Gabelung
zwischen A und B beide Listen gegen die Takt-Referenz über den ganzen Kampf (siehe
[Am Weg der Suche](#am-weg-der-suche)). Sie berichtigt die Zuordnung der ersten Nachbesserung: Die
folgenreichste Verschiebung, die der AC5-Suche, entsteht nicht neben einem GCD-freien Zug, sondern an
einer Liste ohne Füller — und dort trifft B die Takt-Referenz, A nicht. Dazu kommt die Mutation MT
(`Value::Time => None`).

## Ergebnis

- **Ohne GCD-freien Zug trifft Fassung B die Takt-Referenz in jeder Probe.** In 84 Wurfproben
  (7 Aufbauten mit 6 verschiedenen zeitabhängigen Werten × 6 Vergleichsformen × 2
  Operandenreihenfolgen) wirft Fassung B genau dort, wo ein an jedem Takt entscheidender Lauf wirft;
  Fassung A verfehlt 24 der 84 (18 verspätet, 6 fallen ganz aus). In den 16 GCD-Proben ohne
  GCD-freien Zug trifft B 16, A 14. In allen protokollierten Kämpfen des Messprogramms liegt unter A
  wie unter B kein Eintrag neben dem 10-ms-Raster.
- **Betroffen sind unter A nicht, wie bisher angenommen, `<` und `>=`**, sondern bei fallenden
  Restzeiten `<`, sein Spiegel `T > v` und `!=`, bei `time` `>`, sein Spiegel `T < time` und `!=`.
  Nur diese Formen verpassen ihren Umschlag, in 24 der 28 Wurfproben mit ihnen; `>=` trifft in allen
  Aufbauten. `!=` war bisher nicht gemessen.
- **Gefundene Rotationen:** Unter B verschiebt sich die gefundene Liste in **15 von 84 Suchen**
  (15 von 42 Bossen). In der Einstellung des Suchberichts (`rotation-search.rs`) sind es **2 von 42
  Bossen (4,8 %)**, in der Einstellung des AC5-Prüffalls **13 von 42 (31,0 %)**. Dieselbe Liste
  zählt unter B bei 2 bzw. 5 Bossen anders; in der Nachbarschaft der gefundenen Listen verschieben
  sich 5,78 % bzw. 11,59 % der Bewertungen, in 83 der 84 Nachbarschaften mindestens eine (Ausnahme:
  `kontrolle/orch-maxload`). **Alle 15 hängen an den Nachlegezeilen `dot.<brand>.remains < 1500`:**
  Dieselbe Suche mit `<=` in diesen Zeilen verschiebt sich unter B in **0 von 84** Fällen.
- **Der #797-AC5-Prüffall fällt unter B**: 2 bestanden, 1 rot
  (`ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist`), 1 ignoriert. Die gefundene
  Liste ist zeilengleich mit der „Experiment"-Liste des 1-ms-Versuchs vom 06.09. Dieselbe A-Liste
  wendet den Schlag auch unter B ab (18.950 unverändert) — die gierige Suche erreicht sie nur nicht
  mehr. **Ihr Weg trennt sich in Schritt 3, an einer Liste ohne Füller und ohne GCD-freien Zug:**
  `searing if dot.searing.remains < 1500`, `plate`, `crust` zählt unter A 4740, unter B 4560, und B
  trifft dort die Takt-Referenz (dieselben Würfe, derselbe Wert), A nicht. Unter B gewinnt stattdessen
  `kindle` am Ende (4700); der Weg endet bei Länge 8, ohne Platz für die Dreiergruppe.
- **Nebenfrage `GCD_MS`: weder folgenlos noch überall.** Die Nachlegezeilen `< 1500` sind die
  einzige Stelle, an der B die Suche überhaupt verschiebt. Mit Füller und ohne GCD-freien Zug wirken
  sie nicht (4 von 4 Bränden, 0 von 1764 Bestandsläufen, 0 von 43.006 Nachbarn der gefundenen
  Listen). Sie wirken in zwei Lagen. **Ohne Füller** wartet die Rotation am Umschlag; B legt einen
  Takt danach nach, wie die Takt-Referenz, A erst am Ablauf — an `first-crucible` verliert SEARING
  dabei einen Tick (1260 → 1170 Schaden in 30 s), und genau so trennt sich der Weg der AC5-Suche.
  **Neben einem GCD-freien Wurf** — QUENCH, oder der Wurf, den SUBLIME befreit —, nach dem eine
  Nachlegezeile umschlägt, entscheidet B einen Takt später neu, A erst am nächsten fremden Ereignis,
  400 bis 1500 ms später. So trennen sich die Wege der übrigen 14 Suchen, und so beginnen alle 7
  A-Listen, die unter B anders zählen.
- **Wer an den Gabelungen recht hat:** An den 15 Gabelungen ändert sich der Wert von 18 Listen. Über
  den ganzen Kampf gegen die Takt-Referenz gefahren, trifft A sie in **keiner** davon, B in **6** (die
  AC5-Liste und 5 mit GCD-freiem Zug), in 12 keine der beiden. Von den beiden Listen einer Gabelung
  bewertet die Takt-Referenz in 10 der 15 die höher, die B wählt, in 4 die, die A wählt, in einer beide
  gleich.
- **Dahinter liegt ein Befund, der nicht an A oder B hängt:** Nach einem GCD-freien Wurf entscheidet
  der Simulator erst am nächsten Ereignis oder Weckruf neu, die Takt-Referenz einen Takt später.
  `quench` vor dem Füller an `first-crucible`: Füller bei 2000 statt bei 10, 1400 statt 2000 Schaden
  in 30 s. In allen 18 Proben mit GCD-freiem Zug sind A und B gleich, in 13 davon liegen beide neben
  der Referenz.
- **Handgeschriebener Bestand:** 97 von 2394 Läufen (57 Dateien × 42 Bosse) ändern sich, und zwar
  genau die 6 Dateien mit einer `quench`-Zeile (6 von 6; 0 von 51 ohne). Die 42 Dateien mit einer
  strikten Nachlegezeile, aber ohne GCD-freien Zug ändern sich in 0 von 1764 Läufen; `step1-good.apl`
  und `step3-good.apl` bleiben an allen 42 Bossen gleich.

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
vom 22.08., `Value::BossNextStrikeIn => None` (Patch-SHA-256 `49f3e394…`, ergibt `7e77a7af…`). Für den
Wächter des zweiten Werts lief die **Mutation M2**: `Value::DotRemains(_) => None`. Sie entsteht aus
dem Bestand, indem die Zeile `Value::DotRemains(i) => at(fight.dot_remaining_ms(i), threshold),`
ersetzt und davor die Kommentarzeile `// MUTATION M2 (#1027, Wegwerf): kein Weckruf fuer dot.*.remains.`
eingefügt wird; das ergibt `6d6f3cce…`. Die reine Ersetzung ohne Kommentarzeile ergäbe `2c6b0b81…` und
ist nicht gefahren. Für die `time`-Tabelle lief die **Mutation MT**: Die vier Zeilen des Zweigs
`Value::Time => { … }` werden durch die eine Zeile `Value::Time => None,` ersetzt, ohne Kommentarzeile
(ergibt `7b75ef22…`).

## Aufbau und Befehle

Das Messprogramm `sim/examples/e1027-operator-measure.rs` (SHA-256 `30c4bce0…`; in der ersten Fassung
`802bbea3…`) ist für alle drei Fassungen dieselbe Datei. Gebaut wurde remote, gefahren lokal nach
`-c`; alle Aufrufe aus der Worktree-Wurzel:

```bash
cargo remote -c -- build --release --example e1027-operator-measure   # je Fassung
M=./target/release/examples/e1027-operator-measure                    # nach /work/tmp gesichert
$M > a.jsonl            # Bestand; dasselbe mit dem B- und dem M-Bau
$M kreuz a.jsonl b.jsonl > kreuz-a.jsonl      # mit dem A-Bau; kreuz-b.jsonl mit dem B-Bau
$M vergleich a.jsonl b.jsonl kreuz-a.jsonl kreuz-b.jsonl > vergleich-ab.json
$M vergleich a.jsonl m.jsonl > vergleich-am.json
$M vergleich a.jsonl a2.jsonl > vergleich-aa.json   # ebenso b.jsonl gegen b2.jsonl
# zweite Nachbesserung
$M pfad a.jsonl > pfad-a.jsonl          # mit dem A-Bau; pfad b.jsonl > pfad-b.jsonl mit dem B-Bau
$M pfadkreuz pfad-a.jsonl pfad-b.jsonl > pfadkreuz-a.jsonl   # mit dem A-Bau; -b mit dem B-Bau
$M pfadvergleich pfad-a.jsonl pfad-b.jsonl pfadkreuz-a.jsonl pfadkreuz-b.jsonl > pfadvergleich.json
$M nachbarn a.jsonl kreuz-a.jsonl kreuz-b.jsonl > nachbarn.json
$M proben > proben-a.jsonl              # mit dem A-Bau; proben-mt.jsonl mit dem MT-Bau
```

| Fassung | `mod.rs` SHA-256 | Bau-Job                    | Binary SHA-256 | Ausgabe SHA-256 | Wiederholung |
| ------- | ---------------- | -------------------------- | -------------- | --------------- | ------------ |
| A       | `77388248…`      | `1bfbf0cf64f23e80456b614f` | `f31a5412…`    | `f5875952…`     | bytegleich   |
| B       | `3680b9c5…`      | `43308c02eef28b1769388715` | `df51c2ee…`    | `b81df7dc…`     | bytegleich   |
| M       | `7e77a7af…`      | `0147db42ab9ba6f85447cead` | `675ae7ef…`    | `16d73862…`     | —            |

A ist der Stand dieses Zweigs. B und M entstehen aus dem Bestand `377742d1…`, B mit dem Patch oben, M
mit dem Patch `49f3e394…`; nach jedem Bau steht `mod.rs` wieder auf `77388248…`.

Alle fünf Messläufe (A, A wiederholt, B, B wiederholt, M): Exit 0, keine Fehlerausgabe, je 2833
Datensätze — 1 Metadatensatz, 144 Zustandsproben, 84 Wurfproben, 8 Rasterproben, 32 GCD-Proben, 2
Proben `gcdfrei`, 2394 Bestandsläufe, 84 Suchen und 84 Suchen `search_le`. Beide Kreuzläufe: Exit 0,
je 84 Datensätze. Die Bosszahl liest das Programm zweimal und verlangt Gleichheit: `BOSS_COUNT` = 42
und 42 Dateien in `data/bosses/`. A gegen A und B gegen B ergeben in allen 2833 Datensätzen 0
Änderungen. Der Vergleich bricht ab, wenn sich Fallmengen unterscheiden oder die Kreuzbewertung die
eigene Suche nicht bitgleich trifft. Er zählt außerdem, ob sich ein Wahrheitswert ändert (0 Fälle)
und ob die Takt-Referenz unter A und B verschieden ausfällt (0 Fälle, auch in den neuen Proben).

**Die erste Messung steht unverändert darin.** Die 2731 Datensätze der ersten Fassung (Ausgabe
`8f473dfe…`) kommen in der neuen A-Ausgabe gleich heraus, in derselben Reihenfolge, wenn man die neuen
Felder `idle_ms_reference`, `gcd_free_lines` und `strict_time_terms` weglässt — mit dem kommentierten
`mod.rs` und dem erweiterten Programm. Der A-Bau vor `cargo-remote-fmt` (Job
`a81f35ab1d047a6e0766e8a0`) liefert dieselbe Ausgabe wie der danach. `vergleich` ergibt für die
Tabellen der ersten Fassung dieselben Felder wie damals; sie gelten weiter.

**Die zweite Nachbesserung** erweitert das Programm um die Unterbefehle `pfad`, `pfadkreuz`,
`pfadvergleich`, `nachbarn` und `proben` (SHA-256 jetzt `dff9f823…`). Drei Bauten, alle aus dieser
Datei:

| Fassung | `mod.rs` SHA-256 | Bau-Job                    | Binary SHA-256 |
| ------- | ---------------- | -------------------------- | -------------- |
| A       | `77388248…`      | `0e651f6a6a6c483482618cff` | `f06e0871…`    |
| B       | `3680b9c5…`      | `d88780732e52e41abbb4c1c6` | `a35e3037…`    |
| MT      | `7b75ef22…`      | `c1195f1d1cc0352561045957` | `6c282803…`    |

**Die Erweiterung ändert keine bisherige Zahl.** Mit den neuen Bauten sind die Messung unter A
(`f5875952…`) und unter B (`b81df7dc…`) und beide Kreuzläufe (`6474ba12…`, `b176c8b9…`) bytegleich mit
denen der ersten Nachbesserung; `vergleich` über deren Eingaben ergibt bytegleich `cfa8cc45…`. Der
Weg (`pfad`) hat je Bau 672 Datensätze (42 Suchen mit Grenze 1 bis 6, 42 mit Grenze 1 bis 10) und
endet in allen 84 Suchen bitgleich bei Liste und Suchwert der Messung; sonst bräche er ab. Wiederholt
ist er bytegleich, unter A (`65958f05…`) wie unter B (`f97b51a1…`), auch mit einem zweiten Bau.

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

Unter B liegen alle 24 Würfe, die A verfehlt, genau auf dem ersten Referenzwurf (9010, 7010, 10510,
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
`test-caster` (−8,3 %), `test-metronome` (−7,7 %) und `boss-2-4` (−7,5 %). Woher die Verschiebungen
kommen — allein aus den Nachlegezeilen `< 1500`, in 14 Suchen neben `quench` oder `sublime`, in der
AC5-Suche an einer Liste ohne Füller —, steht unter [Am Weg der Suche](#am-weg-der-suche).

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
A stehen zusätzlich `pin`, `quench` und `bind` darin, dafür fehlt `kindle` (10 statt 8 Zeilen),
Suchwert 18.950.

**Die FIXATION-Rotation wird unter B nicht schlechter.** Die A-Liste erzielt unter B unverändert
18.950 (Kreuzbewertung) und überlebt. Die gierige Suche kommt unter B nur nie an die Stelle, an der
sie eine Dreiergruppe prüfen dürfte: Einzelzüge tragen bis Länge 8, bei Grenze 10 ist dann kein
Platz für drei Zeilen. Genau daran brechen unter B auch
`ueber_greedy_wendet_die_suche_den_toedlichen_schlag_ab` (`greedy_traegt_die_gruppen.rs`) und
`der_schalter_schaltet`, `erst_mit_dreiergruppen_findet_die_suche_die_zeile` (`tripel_suche.rs`;
Meldung: keine einzige Dreiergruppe geprüft, Stopp `NoImprovement`, Länge 8). Alle drei fahren
dieselbe Suche wie der AC5-Fall: `test-lethal-strike`, Gewicht 1, Länge 10, alle Kandidaten, sonst
die Voreinstellung mit `stop_on_kill: false`, über `greedy` oder `greedy_zuege` mit `Zuege::ALLE`
(gelesen in `konfiguration`, `ueberlebt_mit_greedy` und `config` der drei Dateien).

**Wo sich der Weg trennt** (Datensätze `pfad` und `pfadkreuz`): Bis Schritt 2 wählen beide Fassungen
`plate` und `crust` (4200). In Schritt 3 setzt A `searing if dot.searing.remains < 1500` an die
Spitze, B hängt `kindle` an:

| Liste in Schritt 3                                                | unter A | unter B | Takt-Referenz | Würfe = Referenz |
| ----------------------------------------------------------------- | ------: | ------: | ------------: | ---------------- |
| A-Wahl: `searing if dot.searing.remains < 1500`, `plate`, `crust` |    4740 |    4560 |          4560 | A nein, B ja     |
| B-Wahl: `plate`, `crust`, `kindle`                                |    4700 |    4700 |          4700 | A ja, B ja       |

Die A-Wahl hat keinen Füller und keinen GCD-freien Zug. Nach `crust` bei 3000 wartet die Rotation;
die Restzeit von SEARING (gewirkt bei 0, 6000 ms Laufzeit) steht beim GCD-Ende 4500 auf 1500 (aus
Wurfreihe und Zauberdaten). B und die Takt-Referenz legen bei 4510 nach, A erst beim Ablauf bei 6000
— über den ganzen Kampf (Tod bei 12.000) sind die Würfe von B die der Referenz. **Hier bewertet A
falsch, nicht B:** A zählt die Liste 180 höher als die Referenz. Mit 4740 gewinnt sie unter A gegen
die B-Wahl (4700); mit dem Referenzwert 4560 läge sie darunter. Ob A ohne diese Fehlbewertung einen
anderen Weg zur Dreiergruppe fände, ist nicht gemessen.

Unter A hat die Liste in Schritt 7 schon 11.627 bei sieben Zeilen; bei den Grenzen 8 und 9 hält die
Suche dort an (`NoImprovement`), erst die Grenze 10 lässt die Dreiergruppe `pin`, `quench`, `bind` zu
(18.950). Unter B steht `kindle` seit Schritt 3 in der Liste; dieselben 11.627 kosten dort acht
Zeilen, und bei Grenze 10 ist keine Dreiergruppe mehr geprüft (`triples` 0).

Die Gewichtsreihe des Prüffalls (`--ignored die_gewichtsreihe_zeigt_ein_fenster`):

| Gewicht                            | 0   | 1      | 2      | 5      | 10     | 20     | 50     | 100    |
| ---------------------------------- | --- | ------ | ------ | ------ | ------ | ------ | ------ | ------ |
| A (Job `e847a311a3ac5718be3a847f`) | —   | trennt | trennt | trennt | trennt | trennt | trennt | trennt |
| B (Job `73cb489b29972d79261ae808`) | —   | —      | trennt | trennt | trennt | trennt | trennt | trennt |

„Trennt" heißt `pin` und `bind` am tödlichen Boss, keine FIXATION am Zwilling. Unter B verliert das
Fenster seinen unteren Rand 1; ab 2 trennen beide Fassungen.

## Die Nebenfrage zu `GCD_MS`

Die Suche erzeugt für jeden der vier Brände die Zeile `dot.<brand>.remains < 1500` (`GCD_MS`). Sie
ist die einzige Kandidatenart mit einem strikten Vergleich auf einem zeitabhängigen Wert
(`candidates_for` und `boss_kandidaten` in `sim/src/rotation_search.rs`): Die übrigen vergleichen mit
`>=` auf Werten, die nicht an der Uhr hängen (Ressource, `boss.casting`, `doom`) oder in keiner Fassung
einen Umschlag haben (`buff.*.active`), und mit `<=` auf `boss.next_strike_in`. Gemessen in drei
Schritten: an `first-crucible` gegen die Takt-Referenz, an den gefundenen Listen und an der Suche
selbst.

### An `first-crucible` gegen die Takt-Referenz

Jede Nachlegezeile über 30 s, mit und ohne Füller, gegen die Takt-Referenz über die ganze Zeit
(Datensätze `gcd`):

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

Dieselben Zeilen mit einem GCD-freien Zug vor dem Füller — `quench` löst keine Wartezeit aus,
`sublime` befreit den Wurf danach —, dazu der freie Zug vor dem Füller ganz ohne Brand (Datensätze
`gcdfrei`). **In allen 18 Zeilen sind A und B gleich**, Würfe wie Schaden:

| Liste                                       | A = B: Nachlegen bei | Takt-Referenz: Nachlegen bei | Schaden A = B / Referenz | A, B = Takt-Referenz             |
| ------------------------------------------- | -------------------- | ---------------------------- | ------------------------ | -------------------------------- |
| `cinder` `< 1500` + `quench` + `kindle`     | 3500, 6500, 10000 …  | 3010, 6010, 9020 …           | 1710 / 1870              | nein                             |
| `cinder` `<= 1500` + `quench` + `kindle`    | 3500, 6500, 10000 …  | 3010, 6010, 9020 …           | 1710 / 1870              | nein                             |
| `cinder` `< 1500` + `sublime` + `kindle`    | 3000, 7000, 10000 …  | 3000, 6010, 9010 …           | 1670 / 1870              | nein                             |
| `cinder` `<= 1500` + `sublime` + `kindle`   | 3000, 7000, 10000 …  | 3000, 6010, 9010 …           | 1670 / 1870              | nein                             |
| `rot` `< 1500` + `quench` + `kindle`        | 5000, 10000, 15000 … | 4510, 9020, 13520 …          | 2258 / 2458              | nein                             |
| `rot` `<= 1500` + `quench` + `kindle`       | 5000, 10000, 15000 … | 4510, 9020, 13520 …          | 2258 / 2458              | nein                             |
| `rot` `< 1500` + `sublime` + `kindle`       | 4000, 8500, 12500 …  | 4000, 8500, 12500 …          | 2158 / 2158              | ja                               |
| `rot` `<= 1500` + `sublime` + `kindle`      | 4000, 8500, 12500 …  | 4000, 8500, 12500 …          | 2158 / 2158              | ja                               |
| `searing` `< 1500` + `quench` + `kindle`    | 5000, 10000, 15000 … | 4510, 9020, 13530 …          | 2280 / 2470              | nein                             |
| `searing` `<= 1500` + `quench` + `kindle`   | 5000, 10000, 15000 … | 4510, 9020, 13520 …          | 2280 / 2470              | nein                             |
| `searing` `< 1500` + `sublime` + `kindle`   | 6000, 12000, 18000 … | 6000, 12000, 18000 …         | 2260 / 2260              | ja                               |
| `searing` `<= 1500` + `sublime` + `kindle`  | 4500, 9000, 14500 …  | 4500, 9000, 13510 …          | 2080 / 2170              | nein                             |
| `smoulder` `< 1500` + `quench` + `kindle`   | 11000, 22500         | 10520, 21040                 | 1920 / 2260              | nein                             |
| `smoulder` `<= 1500` + `quench` + `kindle`  | 11000, 22500         | 10520, 21040                 | 1920 / 2260              | nein                             |
| `smoulder` `< 1500` + `sublime` + `kindle`  | 12000, 24000         | 12000, 24000                 | 1760 / 1760              | ja                               |
| `smoulder` `<= 1500` + `sublime` + `kindle` | 10500, 21000         | 10500, 21000                 | 1860 / 1860              | nein (nur Füller und freier Zug) |
| `quench` + `kindle`, ohne Brand             | Füller ab 2000       | Füller ab 10                 | 1400 / 2000              | nein                             |
| `sublime` + `kindle`, ohne Brand            | Füller ab 1500       | Füller ab 1500               | 1500 / 1500              | ja                               |

### An den gefundenen Listen

In den 7 Suchen, deren A-Liste unter B anders zählt, beginnt die Abweichung jedes Mal gleich: Ein
GCD-freier Wurf fällt auf ein GCD-Ende, an dem eine Nachlegezeile genau auf 1500 steht. B wirft einen
Takt danach weiter, A erst am nächsten fremden Ereignis. Gemessen an den Wurfreihen der A-Liste unter
beiden Fassungen (`kreuz`, Feld `a_casts`; `vergleich` nennt die erste Abweichung):

| Suche                          | freier Wurf                                  | auf 1500   | A wirft weiter bei | B wirft weiter bei |
| ------------------------------ | -------------------------------------------- | ---------- | -----------------: | -----------------: |
| `bericht/boss-2-4`             | `quench` bei 27500                           | `searing`  |              28000 |              27510 |
| `bericht/orch-maxload`         | `quench` bei 15500                           | `searing`  |              15900 |              15510 |
| `kontrolle/boss-1-1`           | `quench` bei 14000                           | `smoulder` |              14500 |              14010 |
| `kontrolle/boss-2-2`           | `quench` bei 26000                           | `searing`  |              26500 |              26010 |
| `kontrolle/test-onslaught`     | `quench` bei 11000                           | `searing`  |              12000 |              11010 |
| `kontrolle/test-phasenbindung` | `searing` bei 24500, befreit durch `sublime` | `smoulder` |              25000 |              24510 |
| `kontrolle/test-warden`        | `quench` bei 4500                            | `searing`  |               5000 |               4510 |

„Auf 1500" ist abgeleitet, nicht vom Messprogramm ausgegeben: letzter Wurf des Brandes in der
Wurfreihe plus seine Laufzeit aus den Zauberdaten, minus der Zeitpunkt des freien Wurfs. Dieselbe
Rechnung an den 16 Proben mit freiem Zug oben trifft an keinem freien Wurf 1500 — dort sind A und B
gleich. Ein GCD-freier Zug allein genügt also nicht: 8 der 69 Suchen, die sich nicht verschieben,
enthalten ebenfalls einen.

Dazu zwei Gegenproben an jeder der 84 A-Listen, unter beiden Fassungen bewertet (`kreuz`):

| A-Liste                                           | unter A und B gleich | darunter die 7, die sich sonst ändern |
| ------------------------------------------------- | -------------------: | ------------------------------------: |
| mit `<=` statt `<` in den Nachlegezeilen          |            84 von 84 |                               7 von 7 |
| ohne ihre GCD-freien Züge (19 Listen haben einen) |            84 von 84 |                               7 von 7 |

Aussagekräftig sind davon 6 von 7: Bei `bericht/orch-maxload` liegen beide Gegenproben unter A und B
bei 0. Die Bossheilung kann den Schaden nicht unter 0 drücken (`undo_progress` in `fight.rs`), zwei
Nullen sagen dort nichts. Dieselbe Einschränkung gilt für die Entfernungs-Nachbarn: In jeder der 7
Listen macht das Entfernen der `quench`-Zeile bzw. bei `test-phasenbindung` der `sublime`-Zeile beide
Fassungen gleich, ebenso das Entfernen der Nachlegezeile, deren Brand an der Abweichung auf 1500
stand — bei `orch-maxload` wieder mit 0 gegen 0.

### An der Suche selbst

Dieselben 84 Suchen mit `<=` statt `<` in den vier Nachlegekandidaten, sonst unverändert (Datensätze
`search_le`): **Unter B verschiebt sich keine** (0 von 84). Die Verschiebungen kommen also allein aus
den Nachlegezeilen; keine andere Kandidatenart ändert unter B ihr Ergebnis. Jede der 15 verschobenen
Suchen enthält in der A- oder B-Liste einen GCD-freien Zug: 11 `quench`, 5 `sublime`,
`kontrolle/boss-2-3` beide. In den A-Listen allein steht `quench` in 10 der 15. **Das sagt nichts
über die Ursache:** In der AC5-Suche steht `quench` nur in der Dreiergruppe der A-Liste, die erst
bei Grenze 10 dazukommt — getrennt hat sich der Weg in Schritt 3, an einer Liste ohne freien Zug.
Wo sich jeder Weg trennt, steht im nächsten Abschnitt.

Nebenbei, ohne Fassungsfrage: Mit `<=` findet die Suche unter A in 73 der 84 Fälle eine schlechter
bewertete Liste als mit `<`, in 9 eine besser bewertete, in 2 dieselbe Zahl.

### Am Weg der Suche

Der Weg jeder Suche ist `greedy` selbst, mit wachsender Längengrenze von 1 bis zur Grenze der
Einstellung (`pfad`): Solange in jeder Runde ein Einzelzug trägt, ist das der Stand der vollen Suche
nach so vielen Runden. An der ersten Stelle, an der A und B verschiedene Listen haben, stehen zwei
Listen aus derselben Nachbarschaft — die A-Wahl und die B-Wahl. Beide sind unter beiden Fassungen
bewertet und mit angehängter Takt-Referenz über den ganzen Kampf gefahren (`pfadkreuz`), bewertet
durch `measure` wie in der Suche. Unter der eigenen Fassung trifft die Bewertung den Weg bitgleich,
und die Referenz fällt unter A und B gleich aus; beides prüft `pfadvergleich`, statt es anzunehmen.

**Die Wege trennen sich in genau den 15 Suchen, deren Ergebnis sich verschiebt** — in den übrigen 69
an keiner Stelle. In 11 der 15 sind beide Schritte an der Gabelung Einzelzüge; in vier hält eine
Fassung dort an, wo die andere noch eine Zeile findet (`orch-maxload` und `test-warden` B,
`boss-5-3` und `test-lapse-interrupt` A).

| Suche                            | Schritt | Wert ändert sich | GCD-freier Zug | Füller              | A → B (Takt-Referenz)                              | trifft die Referenz | Referenz wertet höher |
| -------------------------------- | ------: | ---------------- | -------------- | ------------------- | -------------------------------------------------- | ------------------- | --------------------- |
| `bericht/boss-2-4`               |       6 | beide            | `quench`       | `kindle`            | 8123 → 6570 (6724); 7348 → 7511 (6135)             | keine; keine        | A-Wahl                |
| `bericht/orch-maxload`           |       6 | A-Wahl           | `quench`       | `clasp`             | 696 → 0 (0)                                        | keine               | gleich                |
| `kontrolle/boss-1-1`             |       4 | B-Wahl           | `quench`       | `scab`              | 9996 → 10.029 (10.246)                             | keine               | A-Wahl                |
| `kontrolle/boss-2-2`             |      10 | beide            | `quench`       | `kindle`            | 15.563 → 15.595 (15.887); 15.553 → 15.844 (15.844) | keine; B            | A-Wahl                |
| `kontrolle/boss-2-3`             |       5 | B-Wahl           | `quench`       | `kindle`            | 11.646 → 11.752 (11.613)                           | keine               | B-Wahl                |
| `kontrolle/boss-5-3`             |       7 | B-Wahl           | `quench`       | `bellows`, `kindle` | 12.050 → 12.560 (12.520)                           | keine               | B-Wahl                |
| `kontrolle/test-caster`          |       7 | B-Wahl           | `sublime`      | `kindle`            | 48.008 → 49.415 (49.453)                           | keine               | B-Wahl                |
| `kontrolle/test-lapse-interrupt` |       7 | B-Wahl           | `quench`       | `bellows`, `kindle` | 7744 → 9535 (9535)                                 | B                   | B-Wahl                |
| `kontrolle/test-lethal-strike`   |       3 | A-Wahl           | —              | —                   | 4740 → 4560 (4560)                                 | B                   | B-Wahl                |
| `kontrolle/test-metronome`       |       7 | B-Wahl           | `sublime`      | `kindle`            | 48.008 → 49.415 (49.453)                           | keine               | B-Wahl                |
| `kontrolle/test-onslaught`       |       3 | B-Wahl           | `quench`       | `kindle`            | 8610 → 10.120 (8710)                               | keine               | A-Wahl                |
| `kontrolle/test-phasenbindung`   |       7 | A-Wahl           | `sublime`      | `kindle`            | 50.244 → 49.415 (49.453)                           | keine               | B-Wahl                |
| `kontrolle/test-racer`           |       7 | B-Wahl           | `quench`       | `kindle`            | 17.356 → 18.984 (18.984)                           | B                   | B-Wahl                |
| `kontrolle/test-resistant`       |       7 | B-Wahl           | `sublime`      | `kindle`            | 37.587 → 38.635 (38.659)                           | keine               | B-Wahl                |
| `kontrolle/test-warden`          |       8 | beide            | `quench`       | `bellows`, `kindle` | 9984 → 9901 (9901); 9902 → 9904 (9904)             | B; B                | B-Wahl                |

Wert jeweils unter A → unter B, in Klammern gegen die Takt-Referenz. Bei „beide" stehen Werte und
Treffer der A-Wahl vor denen der B-Wahl. „Trifft die Referenz" heißt: über den ganzen Kampf dieselben
Würfe wie die Liste mit angehängter Takt-Referenz.

„Füller" heißt eine unbedingte Zeile eines Zaubers, der spätestens nach einem GCD wieder bereit ist
(KINDLE, BELLOWS, SCAB, CLASP …; aus den Zauberdaten, `ist_fueller`). Steht einer in der Liste, wird
an jedem GCD-Ende gewirkt, und ein Weckruf zwischen zwei GCD-Enden fällt in einen GCD.

- **14 Gabelungen liegen neben einem GCD-freien Zug.** Jede der 17 Listen, deren Wert sich dort
  ändert, enthält `quench` oder `sublime` und einen Füller, und in allen 17 gehen die Wurfreihen von A
  und B direkt nach einem GCD-freien Wurf auseinander: B wirft 10 ms danach weiter, A 400 bis 1500 ms
  danach. Über den ganzen Kampf trifft B die Takt-Referenz in 5 dieser Listen, A in keiner; in den
  übrigen 12 weichen beide schon früher von ihr ab, und zwar ebenfalls direkt nach einem GCD-freien
  Wurf — dort weckt auch B nicht, und beide warten bis zum nächsten Ereignis.
- **Eine Gabelung liegt ohne Füller und ohne freien Zug: die der AC5-Suche** (siehe
  [Der #797-AC5-Prüffall](#der-797-ac5-prüffall-unter-fassung-b)). Dort trifft B die Takt-Referenz,
  A nicht.
- **Welche der beiden Listen die Takt-Referenz höher bewertet:** in 10 der 15 Gabelungen die B-Wahl,
  in 4 die A-Wahl (`boss-2-4`, `boss-1-1`, `boss-2-2`, `test-onslaught`), in einer keine
  (`orch-maxload`, beide 0). Das vergleicht nur die zwei Listen der Gabelung, nicht die ganze
  Nachbarschaft; welche Liste eine Suche gegen die Referenz wählen würde, ist nicht gemessen.

Dieselbe Einteilung über alle Nachbarn der 84 A-Listen, mit ihren Werten aus `kreuz` (`nachbarn`):

| Nachbarn der 84 A-Listen         | Bewertungen | unter B anders |
| -------------------------------- | ----------: | -------------: |
| GCD-freier Zug, mit Füller       |      15.944 |           5322 |
| GCD-freier Zug, ohne Füller      |          20 |              9 |
| kein GCD-freier Zug, ohne Füller |         193 |             28 |
| kein GCD-freier Zug, mit Füller  |      43.006 |              0 |
| zusammen                         |      59.163 |           5359 |

**Ohne GCD-freien Zug und mit Füller ändert B in 43.006 Bewertungen keine einzige.** Jede Änderung
liegt neben einem GCD-freien Zug oder in einer Liste ohne Füller. Die Einteilung der 84 A-Listen
selbst: Die 7, die unter B anders zählen, haben alle einen GCD-freien Zug und einen Füller; von den
65 ohne freien Zug ändert sich keine.

### Antwort

**Weder folgenlos noch überall.** Die Nachlegezeilen `< 1500` sind die einzige Stelle, an der B
die Suche verschiebt, und damit die Quelle aller 15 verschobenen Suchen; in den 6 geänderten
Bestandsdateien sind sie die einzigen strikten Zeitbedingungen. Ob sie wirken, hängt an der Lage:

- **Mit Füller und ohne GCD-freien Zug folgenlos.** Die Rotation entscheidet an GCD-Enden. SMOULDER
  (12000 ms) und SEARING (6000 ms) sind Vielfache von 1500: Ihre Restzeit steht an einem GCD-Ende genau
  auf 1500, wo `< 1500` falsch ist. Dort wird der Füller gewirkt, und der Weckruf von B einen Takt
  später fällt in dessen GCD — A und B legen beide erst am Ablauf nach, mit identischen Würfen und
  Schaden, wie die Takt-Referenz. Im Bestand ändern sich die 42 Dateien mit einer strikten
  Nachlegezeile ohne GCD-freien Zug in 0 von 1764 Läufen.
- **Ohne Füller** wartet die Rotation; B legt dann einen Takt nach dem Umschlag nach (10510, 4510),
  A erst am Ablauf. Weil Erneuern die Restzeit verwirft (`refreshMode: reset`), kostet das bei
  SEARING einen Tick. CINDER und ROT ändern sich nicht. **In dieser Lage trennt sich der Weg der
  AC5-Suche**, und B ist dort die richtige Bewertung (siehe [Am Weg der Suche](#am-weg-der-suche)).
- **Neben einem GCD-freien Wurf, der genau auf die Schwelle fällt, wirkt der fehlende Weckruf.** Nach
  einem Wurf ohne Wartezeit steht keine GCD-Wiedervorlage in der Schlange. Steht die Nachlegezeile in
  diesem Moment auf 1500, weckt B einen Takt später, A nicht; A wartet bis zum nächsten fremden
  Ereignis, in den 7 Listen 400 bis 1000 ms. Die Bewertung der A-Liste sinkt dadurch unter B in 5 der
  7 und steigt in 2. In dieser Lage trennen sich die Wege der übrigen 14 Suchen.

Aus der ersten Tabelle, unabhängig von der Fassung: **Mit Füller ist die strikte Form bei SEARING die
bessere** — `< 1500` legt am Ablauf nach und behält alle 14 Ticks (2760 Schaden), `<= 1500` legt
einen GCD früher nach und verwirft einen (2470). Bei SMOULDER, CINDER und ROT ergeben beide Formen
dasselbe.

### Nach einem GCD-freien Wurf: ein Befund ohne Fassung

`quench` vor dem Füller, ohne jede zeitabhängige Bedingung: Hier gibt es keinen Weckruf aus
`switch_point`, und A und B sind gleich. QUENCH wirft bei 0, der Füller erst bei 2000; die Liste liest
keinen zeitabhängigen Wert, die Entscheidung bei 2000 kommt also von einem eingereihten Ereignis. Die
Takt-Referenz wirft den Füller bei 10. Über 30 s: 1400 statt 2000 Schaden, QUENCH sechs- statt
fünfmal. `idle_ms` meldet dabei 0 ms.

In allen 13 Proben, in denen A und B neben der Referenz liegen, beginnt die Abweichung genau so: Nach
einem GCD-freien Wurf wirft die Takt-Referenz einen Takt später weiter, der ereignisgesteuerte Lauf
erst am nächsten Ereignis, 500 bis 2000 ms später (aus den Wurfreihen der Datensätze; die drei Zauber
jeder Liste ergeben dort die ganze Reihe). Das ist der Boden, auf dem 14 der 15 Verschiebungen
entstehen: In der Lage oben verkürzt B diese Wartezeit auf einen Takt, weil eine Nachlegezeile gerade
dann umschlägt — so, wie die Takt-Referenz an jedem freien Wurf einen Takt später entscheidet. Über
ganze Kämpfe gemessen ist das an den Listen der Gabelungen ([Am Weg der Suche](#am-weg-der-suche)): B
trifft die Referenz in 5 der 17 geänderten Listen mit freiem Zug, A in keiner; in den übrigen 12
warten beide schon an einem früheren freien Wurf, an dem auch B nicht weckt. An den gefundenen
Endlisten ist die Referenz nicht gefahren.
`fight.rs` war nicht Auftrag und ist nicht angefasst.

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
`quench`-Zeile, keine Datei hat eine `sublime`-Zeile. Von den 97 Läufen ändert sich in 95 der Schaden,
in 33 der Siegzeitpunkt, in keinem der Spielertod.

Nach dem Aufbau der Dateien, aus dem Parser gelesen statt aus dem Text gesucht (`vergleich`, Feld
`bestand_by_structure`; „strikt" heißt ein Vergleich, der erst nach dem Erreichen seiner Schwelle wahr
wird):

| Aufbau der Datei                           | Dateien | Läufe | verschoben |
| ------------------------------------------ | ------: | ----: | ---------: |
| GCD-freier Zug und strikte Zeitbedingung   |       6 |   252 |         97 |
| strikte Zeitbedingung, kein GCD-freier Zug |      42 |  1764 |          0 |
| keins von beiden                           |       9 |   378 |          0 |

Im Bestand verschiebt die strikte Nachlegezeile ohne GCD-freien Zug also nichts; erst neben `quench`
wirkt sie — dieselbe Lage wie in 14 der 15 Suchen (siehe [Am Weg der Suche](#am-weg-der-suche)).
Einzeln nachgezeichnet sind die 97 Läufe nicht.

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

**Der Befund steht jetzt auch an `switch_point`** (Issue-AC4: „als Kommentar an `at`"): Der Weckruf
liegt dort, wo der Wert die Schwelle erreicht, nicht dort, wo der Vergleich kippt; fallend `<` (auch
`T > v`) und `!=`, auf der Uhr `>` (auch `T < time`) und `!=` greifen erst am nächsten fremden
Ereignis. Der Satz im Modul-Doc von `sim/src/apl/mod.rs`, die Grenze aus #53 sei „nicht nur
beschrieben, sondern weg", ist auf die Formen eingeschränkt, die am Erreichen kippen. Nur
Kommentarzeilen: Alle 25 geänderten Zeilen beginnen mit `//`
(`git diff -U0 23d7c13f -- sim/src/apl/mod.rs`), und die erste Messung kommt damit unverändert
heraus (siehe [Aufbau und Befehle](#aufbau-und-befehle)).

## Prüffälle

Neu in `sim/tests/apl_strikte_vergleiche.rs`: alle sechs Formen in beiden Reihenfolgen an
`boss.next_strike_in`, `dot.smoulder.remains` und `time` gegen die Takt-Referenz, die Trägerprüfung
der Referenz (an einem Fall mit zwei zeitabhängigen Seiten, den keine Fassung weckt) und drei Wächter
für den Weckruf ohne fremdes Ereignis: `der_weckruf_traegt_ohne_fremdes_ereignis` für
`boss.next_strike_in`, mit der Nachbesserung `der_weckruf_von_dot_remains_traegt_ohne_fremdes_ereignis`
(`smoulder if dot.smoulder.remains <= 1500` an `first-crucible` ohne `purge`, Wurf bei 10500) und,
nachgetragen nach der Prüfung, `der_weckruf_von_cooldown_remains_traegt_ohne_fremdes_ereignis`
(`purge if time == 0`, `kindle if cooldown.purge.remains <= 1000` an `test-survivable-strike`, Wurf
bei 6500, unter MC bei 7500, siehe [Was nicht gemessen ist](#was-nicht-gemessen-ist)). Die
Tabellen halten Fassung A fest. **Alle drei Formtabellen messen nur den Takt nach dem Umschlag**,
nicht den Weckruf: An jedem der drei Umschläge endet der GCD von `purge` (9000 bei
`boss.next_strike_in` und `time`, 10500 bei `dot.smoulder.remains`), entschieden wird dort ohnehin.
Gemessen ist das je Wert mit einer Mutation, die seinen Weckruf abschaltet: Unter M bleibt die
Tabelle an `boss.next_strike_in` grün, unter M2 die an `dot.smoulder.remains` (in der ganzen Suite
fällt allein der zweite Wächter). Unter MT bleibt die Ereignisspalte der `time`-Tabelle in allen
zwölf Zeilen gleich (`proben` unter A gegen MT, dieselben Aufbauten); rot wird sie dort nur über ihre
Referenzspalte, weil die Takt-Referenz selbst an `time >= t` hängt. **Den Weckruf von `time` hält
die Trägerprüfung der Referenz fest** (`die_takt_referenz_weckt_und_wirft_nie`: unter MT `Some(8000)`
statt `Some(6010)`), dazu `zeitabhaengige_bedingungen_wecken_den_simulator` in `apl.rs`
(`Some(4000)` statt `Some(3000)`) und
`ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster` in `apl_boss_cast.rs`.

```bash
cargo remote --no-copy-lock -- test --test apl_strikte_vergleiche -- --nocapture
cargo remote --no-copy-lock -- test --no-fail-fast        # volle Suite unter B, M, M2 und MT
cargo remote --no-copy-lock -- test                       # volle Suite unter A, wie die CI
```

| Lauf | `apl_strikte_vergleiche.rs`          | volle Suite                                                                                   |
| ---- | ------------------------------------ | --------------------------------------------------------------------------------------------- |
| A    | 10 bestanden, 0 rot (im vollen Lauf) | 203 Suiten: 1274 bestanden, 0 rot, 7 ignoriert, 0 gefiltert (Job `9f999bdef85dd9af4f11b8e6`)  |
| B    | 4 bestanden, 6 rot (im vollen Lauf)  | 203 Suiten: 1264 bestanden, 10 rot, 7 ignoriert, 0 gefiltert (Job `0632095f518f7e48913848b3`) |
| M    | 9 bestanden, 1 rot (im vollen Lauf)  | 203 Suiten: 1270 bestanden, 4 rot, 7 ignoriert, 0 gefiltert (Job `a49ae2def4c32739529e227b`)  |
| M2   | 9 bestanden, 1 rot (im vollen Lauf)  | 203 Suiten: 1273 bestanden, 1 rot, 7 ignoriert, 0 gefiltert (Job `f77d5681be9de04ce854b9c4`)  |
| MT   | 6 bestanden, 4 rot (im vollen Lauf)  | 203 Suiten: 1268 bestanden, 6 rot, 7 ignoriert, 0 gefiltert (Job `f86db90453c3856c54413287`)  |

Unter A zusätzlich wie die CI, in dieser Reihenfolge und auf dem Stand der zweiten Nachbesserung:
`cargo-remote-fmt --check` Exit 0 (Job `2f10f8f04eee02ad4652c84f`),
`cargo remote --no-copy-lock -- clippy --all-targets -- -D warnings` Exit 0 (Job
`ee92b300c4d31a16da3b33c9`), der volle Lauf oben und `cargo remote --no-copy-lock -- test --all-targets
--features diagnostics` mit 239 Läufen: 1309 bestanden, 0 rot, 7 ignoriert (Job
`c6b344027dcaf826d61d46dc`). `Cargo.lock` und `.cargo-remote.toml` blieben nach jedem Lauf
unverändert. Die Läufe unter B, M und M2 stammen aus der ersten Nachbesserung; geändert haben sich
seitdem nur das Messprogramm und ein Kommentar in `apl_strikte_vergleiche.rs`.

- **Rot unter B** (10): in `apl_strikte_vergleiche.rs` die drei neuen Formtabellen und die drei
  bestehenden Verzugsfälle, dazu die vier Suchfälle aus dem AC5-Abschnitt. Jede neue Tabelle bricht
  an ihrer ersten verspäteten Zeile, mit (Ereignis, Referenz) = (9010, 9010) bzw. (10510, 10510).
- **Grün unter B** bleiben beide Wächter, die Trägerprüfung der Referenz und die
  Messmethoden-Prüfung; der nachgetragene dritte ebenso (eigener Lauf von `apl_strikte_vergleiche.rs`
  unter B: 5 bestanden, dieselben 6 rot, Job `6a46960c61f157f9c10aab4a`).
- **Rot unter M** (4): der Wächter für `boss.next_strike_in` (10000 statt 9000) und drei bestehende
  Fälle in `next_strike_in.rs`. Die vier älteren Fälle in `apl_strikte_vergleiche.rs` bleiben unter M
  grün — wie am 22.08. notiert.
- **Rot unter M2** (1): allein der Wächter für `dot.*.remains`, `Some(12000)` statt `Some(10500)`.
  Alle zwölf Zeilen der Formtabelle an `dot.smoulder.remains` bleiben grün.
- **Rot unter MT** (6): in `apl_strikte_vergleiche.rs` die Trägerprüfung der Referenz und die drei
  Formtabellen, jede an ihrer ersten verspäteten Zeile und dort in der Referenzspalte (etwa
  `9000 < time`: (10000, 10000) statt (10000, 9010)); dazu `zeitabhaengige_bedingungen_wecken_den_simulator`
  (`apl.rs`) und `ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster`
  (`apl_boss_cast.rs`). Die beiden Wächter bleiben grün, der nachgetragene dritte ebenso (eigener Lauf
  von `apl_strikte_vergleiche.rs` unter MT: 7 bestanden, dieselben 4 rot, Job
  `4527a27ac1d763d1cc18a350`).

## Für die Entscheidung

- **Der Preis von Fassung B, gemessen:** 2 von 42 Bossen in der Einstellung des Suchberichts, 13 von
  42 bei Gewicht 1 auf Verhindertes; 97 von 2394 Bestandsläufen; 7 bestehende Prüffälle (3 davon in
  `apl_strikte_vergleiche.rs`, die den Ist-Zustand ausdrücklich festhält) und die 3 neuen
  Formtabellen.
- **Woher der Preis kommt:** allein aus den Nachlegezeilen `< 1500` — mit `<=` in diesen Zeilen
  verschiebt B keine der 84 Suchen —, und zwar in zwei Lagen. **14 der 15 Suchen trennen sich neben
  einem GCD-freien Zug**; alle 97 geänderten Bestandsläufe stehen in den 6 Dateien mit `quench`, und
  die 42 Dateien mit strikter Nachlegezeile ohne GCD-freien Zug ändern sich nicht. **Die AC5-Suche
  trennt sich an einer Liste ohne Füller und ohne GCD-freien Zug** — sie und ihre 4 brechenden
  Prüffälle liegen damit außerhalb des Bereichs, in dem beide Fassungen falsch liegen.
- **Was B richtig macht:** Ohne GCD-freien Zug wirft jede Form dort, wo die Takt-Referenz wirft (84
  von 84 Wurfproben, 16 von 16 GCD-Proben), und kein Wurf liegt neben dem Takt. An den 15 Gabelungen
  der Suche trifft B die Referenz über den ganzen Kampf in 6 der 18 Listen, deren Wert sich ändert, A
  in keiner.
- **Was B nicht ändert:** Mit Füller und ohne GCD-freien Zug legt die Nachlegezeile `< 1500` weiter
  erst am Ablauf nach — das ist die Strenge des Vergleichs am GCD-Ende, kein Weckruf-Fehler, und bei
  SEARING gemessen die bessere Wahl. In 43.006 Nachbarn dieser Bauart ändert B keinen Wert.
- **Was keine der beiden Fassungen richtig macht:** Nach einem GCD-freien Wurf entscheidet der
  Simulator erst am nächsten Ereignis oder Weckruf neu, die Takt-Referenz einen Takt danach (Proben
  mit freiem Zug: A und B gleich, in 13 von 18 neben der Referenz). In diesem Bereich entsteht der
  Preis von B in 14 der 15 Suchen: B verkürzt die Wartezeit nach dem freien Wurf auf einen Takt, weil
  eine Nachlegezeile gerade umschlägt. An 12 der 17 geänderten Listen dieser Gabelungen liegen A und B
  beide neben der Referenz, an 5 trifft B sie.
- **Was B bei AC5 kostet, ohne falsch zu sein:** Die AC5-Suche verliert FIXATION bei Gewicht 1, obwohl
  die FIXATION-Rotation unter B gleich viel wert ist. Ihr Weg trennt sich, weil B die Liste
  `searing if dot.searing.remains < 1500`, `plate`, `crust` so bewertet wie die Takt-Referenz (4560)
  und A sie überbewertet (4740). **Der Weg, auf dem AC5 unter A besteht, führt über diese
  Fehlbewertung.** Unter B trennt erst Gewicht 2 (Gewichtsreihe oben); ob die Suche gegen die
  Takt-Referenz bei Gewicht 1 trennte oder A ohne die Fehlbewertung einen anderen Weg fände, ist nicht
  gemessen.

## Nebenbefunde

- **`idle_ms` zählt Leerlauf mehrfach — in beiden Fassungen.** `fight.rs` zählt beim Warten die
  Spanne bis zum nächsten bereits eingereihten Ereignis; der Weckruf der Entscheidung wird erst danach
  eingereiht. Gemessen an den `gcd`-Datensätzen (`first-crucible`, 30 s): Die Takt-Referenz zu
  `smoulder if dot.smoulder.remains < 1500` (ohne Füller, Referenz über 0 bis 30.000 ms) meldet
  1.846.410 ms Leerlauf (Feld `idle_ms_reference` von `smoulder/strikt`, unter A und B gleich), und
  schon A meldet bei `searing if dot.searing.remains < 1500` (ohne Füller) 30.000 ms bei fünf
  gewirkten GCDs (`idle_ms` von `searing/strikt`), also 7.500 ms mehr als möglich. **Umgekehrt fehlt
  die Wartezeit nach einem GCD-freien Wurf ganz:** `quench+fueller` (Art `gcdfrei`) meldet 0 ms,
  obwohl der Füller nach dem ersten QUENCH bei 0 erst bei 2.000 kommt. Hier nicht behoben (`fight.rs`
  war nicht Auftrag); der Wert steht deshalb in keinem Fingerabdruck.
- **Nach einem GCD-freien Wurf entscheidet der Simulator erst am nächsten Ereignis oder Weckruf neu**
  — in beiden Fassungen, siehe [Die Nebenfrage](#die-nebenfrage-zu-gcd_ms). Ohne zeitabhängige
  Bedingung kostet das an `first-crucible` 30 % des Füllerschadens (1400 statt 2000 in 30 s).
- **Sechs der 84 Wurfproben fallen unter A ganz aus**, statt sich zu verspäten: `<` in beiden
  Reihenfolgen an `boss.cast_remains` (`< 500`, wie bekannt) und an `cooldown.purge.remains`
  (`< 1500`), dazu `!=` in beiden Reihenfolgen an `cooldown.purge.remains`. Die Fenster schließen sich
  vor dem nächsten fremden Ereignis; die übrigen 18 der 24 verfehlten Würfe kommen verspätet.
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
- **Die Takt-Referenz über ganze Kämpfe** ist nur an den beiden Listen jeder Gabelung gefahren (30
  Listen an 15 Gabelungen), nicht an den gefundenen Endlisten. Welche Liste eine Suche gegen die
  Referenz wählen würde, ist nicht gemessen; `reference_prefers` vergleicht nur die zwei Listen einer
  Gabelung.
- **Der Weg der Suche** ist `greedy` mit wachsender Grenze. Er gibt die volle Suche genau wieder,
  solange in jeder Runde ein Einzelzug trägt (`extends`); an vier Gabelungen hält eine Fassung an,
  dort beschreibt der Datensatz die Suche mit dieser Grenze. Ob A an der AC5-Gabelung ohne die
  Fehlbewertung einen anderen Weg zur Dreiergruppe fände, ist nicht gemessen.
- **Einzeln nachgezeichnet** sind die 15 Gabelungen und die 7 A-Listen, die unter B anders zählen;
  die 97 geänderten Bestandsläufe sind nur nach dem Aufbau ihrer Dateien zugeordnet. „Auf 1500" und
  „direkt nach einem GCD-freien Wurf" sind aus Wurfreihe und Zauberdaten abgeleitet (siehe
  [Die Nebenfrage](#die-nebenfrage-zu-gcd_ms)); der Kampf meldet nicht, ob ein Wurf befreit war.
- **In den Proben mit freiem Zug** steht `quench` ohne Bedingung, an `first-crucible`; in den
  Suchlisten steht es mit `boss.casting >= 1`. Dass die Proben die Lage „freier Wurf auf der
  Schwelle" nie treffen, ist eine Eigenschaft dieser Aufbauten, keine allgemeine.
- **Mutationen der übrigen Weckrufe** sind gefahren, zwei davon aber nur bei der Prüfung. Gemessen
  bei der Prüfung, je volle Suite: MF (`Value::FightRemains => None`) 1 rot,
  `next_strike_in.rs::ohne_schlag_ist_der_wert_die_restzeit` (Job `01bf5b71582cd257827eddb5`); MBC
  (`Value::BossCastRemains => None`) 1 rot,
  `apl_boss_cast.rs::ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster` (Job
  `65108d803b9af965ab41cc03`); MC (`Value::CooldownRemains(_) => None`) vor dem Nachtrag des dritten
  Wächters 0 rot (Job `173a78f8e3b380cf9bdaff78`). Nach dem Nachtrag hier gemessen, MC als Ersatz
  dieser einen Zeile im Stand dieses Zweigs (ergibt `30986183…`): 203 Suiten, 1274 bestanden, 1 rot,
  7 ignoriert, 0 gefiltert (Job `b89459ded628f8f9faa2753b`); rot ist allein
  `der_weckruf_von_cooldown_remains_traegt_ohne_fremdes_ereignis`, `Some(7500)` statt `Some(6500)`.
  MF und MBC sind hier nicht nachgefahren. `Time` ist mit MT gefahren (siehe [Prüffälle](#prüffälle)).

## Rohdaten

Messausgaben, Vergleiche und Binaries der Nachbesserung liegen unter `/work/tmp/w1-1027-nb/mess/` und
`/work/tmp/w1-1027-nb/bin/`, die Fassungen von `mod.rs` unter `/work/tmp/w1-1027-nb/fassungen/`, die
Patches und die erste Messung unter `/work/tmp/w1-1027-patches/` und `/work/tmp/w1-1027-mess/` —
**nicht dauerhaft**; die Befehle oben erzeugen sie neu. Kreuzläufe: `kreuz-a.jsonl` `6474ba12…`,
`kreuz-b.jsonl` `b176c8b9…`. Vergleich A gegen B: `vergleich-ab.json`, SHA-256 `cfa8cc45…` (enthält
die Dateipfade der beiden Eingaben).

Die zweite Nachbesserung unter `/work/tmp/w1-1027-nb2/` (`mess/`, `bin2/`, `fassungen/`), ebenso nicht
dauerhaft: `pfad-a.jsonl` `65958f05…`, `pfad-b.jsonl` `f97b51a1…`, `pfadkreuz-a.jsonl` `69706f7d…`,
`pfadkreuz-b.jsonl` `48cecdd0…` (wiederholt jeweils bytegleich), `nachbarn.json` `eb991b54…`
(enthält die Dateipfade seiner Eingaben), `proben-a.jsonl` `039b172e…`, `proben-mt.jsonl`
`41a4f0b5…`; `pfadvergleich.json` `10c34918…` enthält die Dateipfade seiner Eingaben.
