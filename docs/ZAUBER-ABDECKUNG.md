---
id: zauber-abdeckung
status: Review
ssot: true
refs:
  docs: [GAME-DESIGN.md, SIM-ARCHITEKTUR-PLAN.md]
affects:
  [
    sim/examples/zauber-abdeckung.rs,
    data/bosses/test-late-lethal-strike.json,
    .github/workflows/nightly.yml,
  ]
---

# Zauber-Abdeckung (#550): Messweg, leere Schulen, Anforderung an die Bosse

Masterplan, Anforderung 2 an den Simulator (Zauber-Vergleichstabelle) und Leitplanke C.1c Punkt 2:
„Jeder Zauber muss irgendwo die beste Wahl sein." Paket #550, seit dem 2026-10-29 eine Zeile in
#891.

**Warum diese Datei hier steht.** Sie trägt denselben Namen wie ihr Läufer
`sim/examples/zauber-abdeckung.rs` und folgt dem Muster von `docs/GROESSEN-ABDECKUNG.md`: Messweg
und Ergebnis an einem Ort, die Zahlen mit Datum und Stand. Die Anforderung an die Bosse aus
Abschnitt 3 ist **gemessen, nicht entschieden** — sie gehört deshalb nicht in `GAME-DESIGN.md`,
solange die Frage aus Abschnitt 4 offen ist. Die Bosse aus #1177 Abschnitt 1 erreicht sie über
einen Kommentar dort, der hierher verweist (die Vorlage geht an den Orchestrator).

## 1. Der Messweg (AC1)

```bash
cargo run --release --example zauber-abdeckung -- --stand "$(git rev-parse HEAD)"
cargo run --release --example zauber-abdeckung -- --stand "$(git rev-parse HEAD)" --schulen
```

Der Kopf jedes Laufs nennt **Datum** (UTC, aus der Uhr) und **Stand**; darunter steht je Boss der
ersten Instanz die Zahl und in der Zeile darunter die Zauber, die der Lauf der besten Rotation
wirkt. Der Stand kommt aus `--stand`, sonst aus `GITHUB_SHA`, sonst steht `unbekannt` da — der
Läufer kann ihn nicht selbst ablesen, weil `cargo remote` keinen Git-Verlauf überträgt und auf
node3 ein kopiertes Programm ohne Arbeitsbaum läuft.

Der Nightly (`.github/workflows/nightly.yml`, Schritt „Zauber-Abdeckung der ersten Instanz") gibt
`--stand "${GITHUB_SHA}"` mit und schreibt die ganze Ausgabe in die Zusammenfassung des Laufs; der
Exit-Code des Schritts bleibt der des Läufers (1, wenn die Untergrenze `MINDESTENS = 11` fällt).

Die Prüffälle `ac1_*` im Läufer rechnen auf festen Eingaben, keiner hängt an der Uhr.

## 2. Schulen über die gebauten Bosse (AC3, gezählt am 2026-10-29)

Lauf: `--schulen` auf Stand `7cd1599e813c0f58b61616329b01bfe4c0160a6f`, 2026-10-29 15:04:59 UTC,
`cargo remote`-Job `9803dc1324595f05f8b8c04b`. 21 wertungsfähige Bosse (Prolog und Instanz 1–5,
aus der Klassifikation `scoring::class_of`, nicht aus einer Liste), 58 spielbare Zauber, Länge
bis 6, ohne Ausrüstung. Je Boss lief die Viabilität aus #946 vollständig („Lauf abgeschlossen"
bei allen 21).

Zwei Zählungen, weil sie Verschiedenes sagen:

- **gewirkt** — der Lauf der besten gefundenen Rotation wirkt den Zauber (dieselbe Zählung wie
  die Untergrenze; in der ersten Instanz die bessere von freier Suche und Bestliste).
- **bezeugt** — Befund „mindestens bezeugt" aus `sim/src/viabilitaet.rs`, auch über eine
  erschöpfende Projektion, wenn die Suche ihn nicht fand. Projektionen bis 1 000 000 Listen.

| Schule       | Zauber | gewirkt | bezeugt | gewirkt (an wie vielen Bossen)                                                |
| ------------ | -----: | ------: | ------: | ----------------------------------------------------------------------------- |
| CALCINATION  |     10 |       6 |       8 | kindle (11), smoulder (20), pyre (19), bellows (10), searing (21), cinder (2) |
| COAGULATION  |     13 |       2 |       5 | riposte (1), crucible-skin (1)                                                |
| **FIXATION** |     11 |   **0** |       3 | —                                                                             |
| SUBLIMATION  |     12 |       3 |       7 | vapour (1), efflux (1), aether (1)                                            |
| PUTREFACTION |     10 |       1 |       5 | rot (21)                                                                      |
| ohne Schule  |      2 |       2 |       2 | quench (4), purge (1)                                                         |

```text
LEER nach gewirkt:  FIXATION
LEER nach bezeugt:  keine
```

**Die Zahl „drei leere Schulen" aus AC3 (Stand 07.08.) gilt nicht mehr.** COAGULATION und
SUBLIMATION kommen heute vor — jeweils an genau einem Boss.

**Nach „bezeugt" ist FIXATION nur der Form nach besetzt.** Ihre drei Zeugen — `anchor`,
`capstone`, `griplock` — stehen alle an `boss-4-3`, und ihre Zeugen-Listen geben **null
Kontrollpunkte** aus (Zeile „Kontrollpunkte ausgegeben in der Zeugen-Liste" im Lauf). Jede
FIXATION-Wirkung kostet Kontrollpunkte, auch GRIPLOCK verstärkt nur den Gewinn aus Erzeugern. Der
gemessene Beitrag kann also nur aus der globalen Wartezeit stammen, die der Zauber belegt. Ein
Zeuge belegt einen Beitrag, nicht welchen (siehe Abschnitt 5).

**Gemeint ist in Abschnitt 3 deshalb FIXATION**, unter beiden Zählungen: nach „gewirkt" leer,
nach „bezeugt" ohne eine einzige Liste, in der die Schule ihre Mechanik ausübt.

## 3. Was ein Boss verlangen muss, damit FIXATION die beste Wahl wird

> **Anforderung:** ein **Schlag** (`strike`, kein `interruptibleCast`), der **über der
> Verteidigungs-Obergrenze** tötet und **in der Mitte des Kampfes** fällt.

Am Prüfboss gemessen (5200 Leben, 60 s, Spieler mit 5000 Leben): **Schaden ab 10 000** — bis 9600
hält CRUST — und **Zeitpunkt 16 bis 30 s**. Dann wählt die Suche BIND und PIN; PIN macht den
Schlag unterbrechbar, QUENCH bricht ihn ab, der Spieler überlebt.

**Warum `strike`:** Einen `interruptibleCast` löst QUENCH allein, ohne FIXATION (Kommentar in
`test-lethal-strike.json`). **Warum über der Obergrenze:** Darunter ist Aushalten billiger.
**Warum nicht früh und nicht spät:** Die Suche misst Schaden, ein toter Spieler hört auf, Schaden
zu machen — aber RIPOSTE wirft die Hälfte des **angekommenen** Schlags zurück, auch wenn er tötet
(Abschnitt 5). Die Zeitreihe unten zeigt beide Ränder: Bei 6, 12 und 14 s und ab 36 s bringt
eine Liste, die mit RIPOSTE stirbt, mehr Schaden als eine, die mit FIXATION überlebt. Warum der
frühe Rand bei 14 s liegt, ist nicht zerlegt gemessen.

### Prüfboss und Zwillinge

`data/bosses/test-late-lethal-strike.json`, Instanz `testing`, Zwilling von `test-lethal-strike`
(#797): dieselbe Datei bis auf den Zeitpunkt des Schlags.

| Boss                      | Schlag             | Wert | gewirkt                                       | tot  | unterbrochen | ohne FIXATION  |
| ------------------------- | ------------------ | ---: | --------------------------------------------- | ---- | -----------: | -------------- |
| `test-late-lethal-strike` | 18400 bei 24000 ms | 6933 | smoulder, searing, quench, **bind, pin**, rot | nein |            1 | 3720, **tot**  |
| `test-lethal-strike`      | 18400 bei 12000 ms | 8440 | kindle, smoulder, searing, cinder, riposte    | ja   |            0 | 8440, tot      |
| Schaden-Zwilling          | 4600 bei 24000 ms  | 9364 | smoulder, pyre, bellows, searing, rot         | nein |            0 | 9364, überlebt |

**Vorher nicht, nachher in der besten Rotation:** an keinem der 21 gebauten Bosse und an keinem
der beiden Zwillinge ein FIXATION-Zauber; am Prüfboss BIND und PIN, und der Schlag wird
abgewendet. Der Prüffall `ac3_am_pruefboss_kommt_fixation_in_die_beste_rotation` hält das fest.
Er war rot, solange der Schlag bei 12000 ms lag (Commit `e4ee1e38`), und wird rot, wenn der
Schlag auf 36000 ms rückt oder der Schaden auf 9600 sinkt (Gegenprobe in `7cd1599e`).

### Schwellenreihen am Prüfboss

Lauf `--pruefboss` auf Stand `2c71c33c21ef541d9fd1fedd18a29c0f3cec1ef4`, 2026-10-29 16:03:08 UTC,
Job `3e96199b1db0c213a8d76ea2` — Werte bitgleich zum Prüfboss-Teil des `--schulen`-Laufs aus
Abschnitt 2, dazu die gewirkten Zauber. Die Zeilen sind zur Laufzeit gebildete Abwandlungen des
Prüfbosses (`mit_schlag`), keine eigenen Dateien. „tot" ist der gezählte Kampf, der wie im Spiel
mit dem Sieg endet; „Wert" ist die Suche, die die vollen 60 s rechnet.

| Schaden | bei ms |  Wert | gewirkt                                       | tot  | ohne FIXATION |
| ------: | -----: | ----: | --------------------------------------------- | ---- | ------------- |
|    4600 |  24000 |  9364 | smoulder, pyre, bellows, searing, rot         | nein | 9364, lebt    |
|    9300 |  24000 |  7655 | kindle, smoulder, pyre, searing, crust, rot   | nein | 7655, lebt    |
|    9600 |  24000 |  7655 | kindle, smoulder, pyre, searing, crust, rot   | nein | 7655, lebt    |
|   10000 |  24000 |  6933 | smoulder, searing, quench, bind, pin, rot     | nein | 3720, tot     |
|   11500 |  24000 |  6933 | smoulder, searing, quench, bind, pin, rot     | nein | 3720, tot     |
|   14000 |  24000 |  6933 | smoulder, searing, quench, bind, pin, rot     | nein | 3720, tot     |
|   18400 |  24000 |  6933 | smoulder, searing, quench, bind, pin, rot     | nein | 3720, tot     |
|   18400 |   6000 |  8944 | pyroclasm, riposte, efflux, aether, sublime   | ja   | 8944, tot     |
|   18400 |  12000 |  8440 | kindle, smoulder, searing, cinder, riposte    | ja   | 8440, tot     |
|   18400 |  14000 |  8931 | kindle, smoulder, searing, riposte, rot       | ja   | 8931, tot     |
|   18400 |  16000 |  5884 | searing, quench, bind, pin, rot               | nein | 2340, tot     |
|   18400 |  30000 |  6933 | smoulder, searing, quench, bind, pin, rot     | nein | 4714, tot     |
|   18400 |  36000 | 12269 | kindle, smoulder, pyre, searing, riposte, rot | ja   | 12269, tot    |
|   18400 |  48000 | 14260 | kindle, smoulder, pyre, searing, riposte, rot | nein | 14260, lebt   |

Wo FIXATION fehlt und der Spieler stirbt, steht in jeder Zeile RIPOSTE.

Bei 48 000 ms ist der Boss im gezählten Kampf schon besiegt, bevor der Schlag fällt; die Suche
rechnet weiter und stirbt mit RIPOSTE. Ein Schlag nach dem Sieg stellt im Spiel keine Anforderung.

## 4. Was die Anforderung kostet — eine offene Entscheidung

**An jedem gemessenen Punkt, an dem FIXATION gewählt wird, stirbt die beste Liste ohne
FIXATION.** Die Schule ist dort nicht die beste, sondern die **einzige** Antwort, die die Suche
findet. Einen Bereich, in dem FIXATION gewählt wird und Aushalten trotzdem gelingt, gibt es in der
Schadensreihe nicht: Bei 9600 hält CRUST, ab 10 000 findet die Suche ohne FIXATION keine Liste
mehr, die überlebt.

Das widerspricht GAME-DESIGN 11.3 („Kein Boss verlangt eine bestimmte Schule … Jede
Zusammenstellung hat einen Weg") und Masterplan C.1c Punkt 3 („Der Spieler soll spielen können wie
er will"). Mit den Zahlen vom 2026-10-29 stehen damit zwei Leitplanken gegeneinander:

| Weg                                                          | erfüllt             | verletzt                                        |
| ------------------------------------------------------------ | ------------------- | ----------------------------------------------- |
| (a) einzelne Bosse stellen die Anforderung                   | C.1c-2 für FIXATION | GAME-DESIGN 11.3, C.1c-3                        |
| (b) kein Boss stellt sie                                     | GAME-DESIGN 11.3    | C.1c-2 für FIXATION                             |
| (c) Zauberwerte oder Bewertung ändern, bis ein Band entsteht | beides, vielleicht  | Auflage von #550: „nicht an den Zaubern drehen" |

Diese Wahl ist eine Charakter-Frage und gehört dem Auftraggeber. Dieses Dokument trifft sie nicht;
die Bosse aus #1177 Abschnitt 1 sollten die Anforderung erst nach ihr übernehmen.

## 5. Befunde unterwegs — nicht behoben, mit Grund

1. **Der Rückschlag ist nicht am Leben gedeckelt.** `Fight::hit_player` reicht den angekommenen
   Schaden an `reflect_damage` weiter, auch wenn er tötet (`sim/src/fight.rs`; im Spiel
   `Combat.hitPlayer`). In jeder gemessenen Liste, die am Prüfboss oder seinen Abwandlungen
   stirbt und trotzdem gewinnt, steht RIPOSTE (Schwellenreihen oben). Wie viel davon der
   Rückschlag des tödlichen Treffers ist, ist nicht zerlegt gemessen. Kampfkern beider Seiten mit
   Paritätspflicht, und ob das Absicht ist, ist eine Design-Frage; nicht Teil dieses Pakets.
2. **Die Suche misst Summen, nicht Siege** (#814). Ein toter Spieler mit hohem Schaden schlägt
   einen lebenden; die Zeilen bei 6, 12, 14 und 36 s zeigen es. An `test-lethal-strike` (12 s)
   wählt die Suche FIXATION, wenn verhinderter Schaden mitzählt (`prevented_weight = 1`, Länge 10,
   `sim/tests/ac5_kontrolle_lohnt_sich.rs`, #797 AC5) — das ist eine andere Bewertung als die der
   Abdeckung.
3. **RIPOSTE lässt sich in der Suche nicht auf einen Schlag legen.** Nur PIN bekommt eine Zeile
   auf `boss.next_strike_in` (`sim/src/rotation_search.rs`, `candidates_for`). Ob eine auf den
   Schlag gelegte RIPOSTE das Zeitfenster aus Abschnitt 3 verschiebt, ist nicht gemessen; das
   Fenster gilt im Raum der Suche.
4. **„Mindestens bezeugt" zählt Wartezüge** (#946). An `boss-4-3` sind 26 von 58 Zaubern bezeugt;
   die FIXATION-Zeugen dort geben keinen Kontrollpunkt aus (Abschnitt 2). Der Befund belegt, dass
   ein Zauber beiträgt, nicht wodurch.

## 6. AC4 — angekündigt, nicht gemessen

Nach dem Merge von PR #1173 (analytischer Krit, #920) fährt `--mit-ausruestung` einmal neu; laut
w5-920 finden Suchen mit PURITY > 0 danach andere Listen, der Pfad ohne Ausrüstung bleibt gleich.
Das Ergebnis kommt neben das vom 14.08. (Commit `56cda48e`):

| Stand      | NEUTRAL   | Stufe 13, RARE                                                             | Stufe 85, LEGENDARY |
| ---------- | --------- | -------------------------------------------------------------------------- | ------------------- |
| 2026-08-14 | 11 von 58 | 9 von 58 — fällt weg kindle, bellows, efflux, aether; dazu scab, decoction | 1 von 58 (searing)  |
| nach #1173 | offen     | offen                                                                      | offen               |
