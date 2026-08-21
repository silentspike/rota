# Die acht Messungen

_Stand 2026-08-14, alles auf dem Ryzen 9 5900HS. Jede Zahl steht in zwei Spalten:
**heute** und **Endausbau laut Plan**. Der Plan ist der Maßstab, nicht der Bestand._

> **Korrektur am Mehrkern-Durchsatz, 2026-08-14 17:44.** Auf die Frage des Auftraggebers
> („ich habe nicht viel Auslastung gesehen") habe ich nachgemessen — und dabei einen eigenen
> Messfehler gefunden. Meine früheren Mehrkernzahlen waren **um rund 30 % zu niedrig**, weil
> die Läufe zu kurz waren (5 000 Kämpfe je Strang ≈ 0,1 s; Thread-Start und Turbo-Hochlauf
> dominierten). Mit 400 000–500 000 Kämpfen je Strang:
>
> ```
>  1 Strang     52 127 Kämpfe/s   1,00×      (vorher gemeldet: 55 554)
>  2            96 410            1,85×
>  4           197 439            3,79×
>  8           308 152            5,91×      (vorher gemeldet: 242 863)
> 16           350 794            6,73×      (vorher gemeldet: 254 031)
> ```
>
> Beleg für die Auslastung, `mpstat` parallel zu einem 20-s-Lauf über 8 Mio. Kämpfe:
> `%usr` 93,4–96,0 %, `%idle` unter 1 %. GPU: 0 %, 15 MiB — erwartet, nichts nutzt CUDA.
>
> **Warum wenig zu sehen war:** Die eigentlichen Messläufe dauern 1–4 Sekunden. Ein Monitor
> mit 1–2 s Abtastung sieht davon fast nichts. Die Worker-Läufe laufen zudem über
> `cargo remote` auf dem Build-Server, erzeugen hier also gar keine Last.

> **Zweite Korrektur, 2026-08-14 18:25 — die Messbedingungen waren schlecht.** Der
> Auftraggeber hat gefragt, ob parallele Worker-Messungen die Zahlen verfälschen. Gemessen,
> während die Messungen liefen:
>
> ```
> load average  22,99 auf 16 Threads
> Swap          20 552 MB von 40 959 belegt
> si/so         bis 16 150 / 12 248 je Sekunde — aktives Swapping
> wa            65 → 89 % I/O-Wait,  bis 51 Prozesse blockiert
> ```
>
> Das System swappte 20 GB. Ursache: ein Worker-Lauf mit 1,8 GB **und fünf Claude-Sessions
> mit zusammen 4,2 GB** bei 14,9 GB Gesamtspeicher.
>
> **Nach Beruhigung neu gemessen** (Auslagern auf 0, `wa` unter 1 %, Restlast ~40 % `us`):
>
> ```
>  1 Strang     51 799 Kämpfe/s     (zuvor 52 127 / 55 554)
>  8 Stränge   315 228              (zuvor 308 152 / 242 863)
> 16 Stränge   363 541              (zuvor 350 794 / 393 848)
> ```
>
> **Die Fremdlast hat weniger verfälscht als befürchtet.** Die Zahlen streuen um **±12 %**
> über alle Läufe — die 393 848 war der Ausreißer nach oben, nicht die 350 794 nach unten.
>
> **Belastbare Baseline: rund 350 000–365 000 vollständige Kämpfe/s auf dem ganzen 5900HS,
> mit ±12 % Streuung.** Für Architektur-Entscheidungen reicht das; für Vorher-Nachher-
> Vergleiche einzelner Optimierungen unter 10 % reicht es **nicht** — dafür müsste die
> Maschine ruhig sein, und mit vier laufenden Sessions wird sie das nicht.
>
> **Nicht betroffen sind alle zählenden Messungen** (Queue-Länge, Präfix-Regel,
> Gruppengrößen, Trefferquoten, Phänotypen, Ablaufzählungen). Sie sind deterministisch;
> Worker-2 hat das für seine mit zwei md5-gleichen Läufen belegt.

**Der Endausbau, damit die Zahlen einen Bezug haben** (aus `greedy-skipping-russell.md`):

```
68 Bosse in 14 Instanzen      8 Aufstiegs-Instanzen à 4 (=32)
                              6 Endgame-Instanzen à 6 (=36)
60 Zauber                     5 Schulen × 12          (heute 58)
12–16 Leistenplätze           (heute 12)
15 Talentzweige + globaler     5 Schulen × 3 Wege     (heute 6 Bäume)
Leiterstufen unbegrenzt        jeder Endgame-Boss ab Mythisch auf Stufe 1, 2, 3 …
Werte GEDECKELT                „ab da wird es reines Können: Der Boss wird
                                stärker, man selbst nicht mehr"
```

Die letzte Zeile ist die wichtigste des ganzen Dokuments — mehr dazu bei Messung 6.

---

## Zuerst: die Zahl, auf die Sie am meisten gesetzt haben, ist widerlegt

> „Das ist potenziell viel wichtiger als AVX2 **und** CUDA zusammen."

**Gemessen: Faktor 5,0. Nicht 140.000.**

Und ich habe die Messung zweimal machen müssen, weil der erste Anlauf nichts gemessen hat —
das gehört dazu, weil der Fehler lehrreich ist.

**Erster Anlauf, falsch:** Ich zählte während eines Kampfes für jeden Zauber, ob
`is_ready()` gilt. Ergebnis: **57 von 58 immer bereit**. Das sah nach Faktor 140.000 aus und
war wertlos — ein Zauber, den die Rotation nie wirkt, lässt seine Abklingzeit nie anlaufen.
Die Zahl beantwortete „ist X bereit, wenn niemand X spielt".

**Zweiter Anlauf, tragfähig:** Für jeden Zauber X eine Rotation aus genau zwei Zeilen —
`X` bedingungslos, darunter ein Marker. Der Marker kommt nur dran, wenn X gerade nicht geht.

```
SPERRT NIE — der Marker kam kein einziges Mal dran (12 von 57):
  smoulder bellows searing ember scab bind tether clasp wisp decant blight miasma

LÄSST DURCH (45), die dichtesten:
  cinder    X  54×   Marker  54×   Marker-Anteil 50 %
  canker    X  54×   Marker  54×   50 %
  decay     X  57×   Marker  57×   50 %
  rot       X  36×   Marker  69×   66 %
  quench    X  35×   Marker  96×   73 %
```

| | heute | Endausbau |
| --- | --- | --- |
| Kandidatenzeilen / Länge | 93 / 12 | ~96 / 16 |
| Permutationen | 1,995e23 | **1,386e31** |
| sperrende Zeilen | 12 | ~13 |
| nach Reduktion | 4,026e22 | 1,308e30 |
| **Faktor** | **4,96** | **10,6** |

Der Endausbau vergrößert den Raum um Faktor 7·10⁷ (vier Leistenplätze mehr wiegen schwerer
als zwei Zauber mehr) und die Reduktion wächst nur auf 10,6.

**Das ist eine gute Nachricht in einer schlechten Verpackung:** Die Rotations-Kanonisierung
über unerreichbare Suffixe ist ein Faktor 5–10, kein Faktor 10⁵. Der Hebel liegt woanders —
und Messung 2 zeigt, wo.

---

## Messung 1 — EventQueue-Belegung

**200 Kämpfe, 4 Bosse, Abtastung bei jedem Schleifendurchlauf:**

```
Schlangenlänge     Mittel   5,90
                   p50         6
                   p95         8
                   p99         9
                   max         9      ← nie mehr

pushes je Kampf    Mittel  112,0     max 140
Schleifendurchläufe je Kampf  45,5
EVENT_CAPACITY im Code: 256    →  Auslastung 3,5 %
```

**Zu `pops/fight`:** nicht getrennt gezählt, aber aus den beiden Zahlen bestimmt. Am Kampfende
stehen höchstens 9 Ereignisse offen, also sind **103–112 der 112 pushes auch gepoppt**. Die
Schlange läuft nicht voll und wird nicht verworfen — sie wird abgearbeitet.

**Die Antwort auf Ihre Frage ist eindeutig: kein BinaryHeap.** Neun Einträge à 8 Byte sind
72 Byte — das passt in eine Cache-Line. Ein linearer Min-Scan über ein festes Array schlägt
den Heap bei dieser Größe deutlich, und Messung 2 beziffert, was das wert ist.

**Und `EVENT_CAPACITY = 256` ist um Faktor 28 überdimensioniert.** Der Kommentar im Code
schätzte „zu keinem Zeitpunkt mehr als eine Handvoll" — das stimmt, nur wurde trotzdem für
256 reserviert. 4 KB Heap-Allokation je Kampf für 72 Byte Bedarf.

**Endausbau:** Die Schlangenlänge hängt an gleichzeitig laufenden Brand-Effekten, Buffs und
Boss-Mechaniken, nicht an der Zahl der Zauber im Buch. Mit 16 statt 12 Leistenplätzen und
reicheren Bossen erwarte ich p99 um 12–16, nicht 100. **Ein festes Array mit 32 Plätzen
reicht mit großem Sicherheitsabstand** — das ist eine Schätzung, keine Messung.

---

## Messung 2 — Wo die Zyklen und die Fehlsprünge wirklich liegen

`perf record` über 150 000 Kämpfe, nach Symbol:

| Bereich | Branch-Misses | Zyklen |
| --- | ---: | ---: |
| **`expire_buffs`** | **20,2 %** | **23,8 %** |
| **`BinaryHeap::pop`** | **15,6 %** | **13,3 %** |
| `resolve_player_cast` | 11,7 % | 11,7 % |
| `apply` (Event-Dispatch) | 10,6 % | 7,0 % |
| `Apl::next_wakeup` | 8,2 % | 8,6 % |
| `RuleEvents::emit` | 7,7 % | 6,5 % |
| `Fight::new` | 2,6 % | 4,2 % |
| **APL-Auswertung** (`decide` + `Condition::holds` + `Value::read`) | **5,1 %** | **4,1 %** |
| Schadenspfad (`deal`, `cast`, `hit_player`) | ~3,0 % | ~2,4 % |
| `Event::partial_cmp` | 1,6 % | 1,6 % |
| `BinaryHeap::push` | — | 1,2 % |

**Das kippt die naheliegende Annahme.** Die APL-Bedingungsauswertung — der offensichtliche
Kandidat für „branchless machen" — kostet **4 %**. Sie zu optimieren wäre verlorene Mühe.

**Die beiden größten Posten sind zusammen 37 % der Zyklen und beide ohne jede
Spiellogik-Änderung angreifbar:**

**`expire_buffs`, 23,8 %** — die Routine ruft **acht** Buff-Sammlungen mit `.expire(now)` auf,
und zwar in **jedem** der 45,5 Schleifendurchläufe:

```rust
let masken = [
    self.control_buffs.expire(now),  self.resource_buffs.expire(now),
    self.tick_buffs.expire(now),     self.stack_buffs.expire(now),
    self.cd_reduce_buffs.expire(now), self.convert_buffs.expire(now),
    self.cap_buffs.expire(now),      self.buffs.expire(now),
];
```

Acht Scans, die fast immer nichts finden. Ein geführter `next_expiry_at`-Zeitstempel je
Sammlung würde fast alle einsparen — `if now < self.next_expiry { return; }`. **Das ist der
größte Einzelposten des ganzen Kernels und ein reines Aufräumproblem.**

### Nachgemessen: „fast immer nichts" heißt **höchstens einmal je Kampf**

_Weil „fast immer nichts" eine Vermutung war und darauf kein Umbau gehört, habe ich es messen
lassen (Worker-3) und mit einem eigenen Lauf nachgeprüft — reproduziert exakt._

```
Boss           Kämpfe   Abläufe   je Kampf   früheste ms   späteste ms
boss-1-1           70        70       1,00        19 500        19 500
boss-1-2           70         0       0,00             —             —
boss-2-3           70         0       0,00             —             —

zusammen          210        70       0,33
Kämpfe GANZ OHNE Ablauf:  140 von 210   (66,7 %)
```

Bei 45,5 Schleifendurchläufen × 8 Sammlungen sind das **360 Aufrufe von `.expire(now)` je
Kampf** — und der Zähler ist **0 oder 1**:

```
boss-1-1     1 Treffer von 360   →   99,7 % der Aufrufe einsparbar
boss-1-2     0 Treffer von 360   →  100 %
boss-2-3     0 Treffer von 360   →  100 %
```

**Die Aufschlüsselung je Sammlung wird dafür nicht gebraucht:** Wenn insgesamt höchstens ein
Ablauf stattfindet, kann keine einzelne Sammlung mehr haben.

Zwei Einschränkungen, die Worker-3 selbst benannt hat und die die Zahl richtig einordnen:

1. Bei `boss-1-2` und `boss-2-3` setzt die gefundene Rotation **gar keine Verstärkungen**. Die
   Null sagt dort mehr über die Rotation als über `expire_buffs`. Für die Frage reicht es —
   die Aufrufe finden trotzdem statt und finden nichts.
2. Alle 70 Abläufe bei `boss-1-1` liegen bei **exakt 19 500 ms**, unabhängig vom Startwert.
   Der Ablauf hängt an der Rotation, nicht am Zufall.

**Und eine Zusicherung, deren Grund wichtiger ist als sie selbst:** Das Kampfprotokoll, aus
dem die 70 Abläufe gezählt werden, ist ein **Ringpuffer**. Wäre er übergelaufen, wären
Einträge verlorengegangen — die gemessene Zahl wäre **zu niedrig**, also genau in der
Richtung, die das Ergebnis stützt.

```
Kämpfe mit übergelaufenem Protokoll:    0 von 210
höchste Belegung:                     199 Einträge  (Kapazität 4096)
```

Die Prüfung steht als `assert_eq!` im Läufer, nicht als Ausgabe: Senkt jemand `log_capacity`
oder steigt die Kampfdauer, bricht die Messung ab, statt still eine zu kleine Zahl zu melden.

> Der Befund gehört Worker-3, und er ist eine Lehre über Messungen im Allgemeinen: **Ein
> Ergebnis, das sich durch seinen eigenen Messfehler bestätigen würde, prüft niemand nach.**
> Genau dort muss die Zusicherung sitzen.

**Damit ist der größte Posten des Kernels nicht nur benannt, sondern beziffert:
23,8 % der Zyklen für Arbeit, von der 99,7 % nichts findet.**

**`BinaryHeap::pop` + `push` + `partial_cmp`, 16,1 %** — bei einer Schlange von maximal neun
Einträgen (Messung 1). Heapify, Zeigerverfolgung und Dreiweg-Vergleich für etwas, das in eine
Cache-Line passt.

**Konservativ geschätzt liegen 25–30 % Laufzeit in diesen beiden Stellen**, bevor irgendeine
Kanonisierung, SIMD oder GPU ins Spiel kommt. Das ist eine Schätzung; gemessen sind nur die
Anteile.

---

## Messung 6 — Fixed-Point: die Bereichsanalyse, und ein Befund, der sie blockiert

**Heute aufgetretene Werte**, 280 Kämpfe über alle 35 Bosse, voll bestückt
(alle Affixe auf 1000 Promille, Talente 300 ‰):

| Faktor | Minimum | Maximum |
| --- | ---: | ---: |
| `essence` | 2,0000 | 2,0000 |
| `talents` | 1,3000 | 1,3000 |
| `gear_effects` | 1,5000 | 1,5000 |
| `buffs` | 1,0000 | 1,0000 |
| `synergy` | 1,0000 | 1,0000 |
| `resistance` | **0,7500** | **1,2500** |
| `vulnerability` | 1,0000 | 1,0000 |
| `crit` | 2,5000 | 2,5000 |
| `base_damage` | 40 | 384 |

```
Produkt der acht Faktoren    7,3125 bis 12,1875
mal größtem base_damage      bis 4 680
```

**Für heute ist Fixed-Point damit trivial machbar.** Bei Promille-Skalierung passt alles
bequem in `u64`.

### Der Befund: im Endausbau ist der Bereich unbeschränkt

Die Formeln sind linear und ungedeckelt:

```
essence_factor = 1 + essence/1000
crit_factor    = (1500 + yield)/1000
```

Und das Budget wächst über 100 Stufen um 5 % je Stufe:

```
epic auf Stufe 100:   Budget 23 707 Promille,  je Affixplatz 7 902

10 Items, alle auf ESSENCE:   79 020 ‰  →  essence_factor  80,0
10 Items, alle auf YIELD:                  crit_factor     80,5
allein essence × crit                                    = 6 443
```

Das Produkt aller acht Faktoren läge damit im fünf- bis sechsstelligen Bereich, mal
`base_damage`. **Bei 8 Faktoren à ~17 Bit reicht `u64` nicht mehr; man bräuchte `u128` oder
Zwischenrundungen** — und Zwischenrundungen sind genau das, was die heutige Formel bewusst
vermeidet („Rundung einmal am Ende").

**Aber der Plan kennt eine Grenze, die niemand implementiert hat:**

> **Obergrenze: Werte gedeckelt, Stufen unbegrenzt.** Ab einer bestimmten Leiter-Stufe wachsen
> Gegenstandsstufen und Spielerstärke nicht mehr — ab da wird es reines Können: Der Boss wird
> stärker, man selbst nicht mehr.

**Im Code existiert dieser Deckel nicht.** Es gibt nur zwei Deckel überhaupt (`purity ≥ 1000`
→ Krit 100 %, `wardCapPermille = 750`), und beide betreffen nicht die Multiplikationsfaktoren.

**Damit ist Ihre Frage nach den formalen Grenzen heute nicht beantwortbar — nicht weil ich
nicht gemessen hätte, sondern weil die Grenze eine offene Design-Entscheidung ist.** Sie ist
aber eine, die ohnehin fallen muss, weil der Plan sie fordert.

**Meine Empfehlung:** Den Deckel als Zahl festlegen, bevor Fixed-Point entworfen wird. Steht
er zum Beispiel bei „Faktorprodukt ≤ 1000", ist `u64` mit Promille-Skalierung komfortabel und
die ganze Frage erledigt sich. Ohne ihn entwirft man gegen einen unbeschränkten Bereich.

### Nachtrag, und er macht den Deckel dringlicher als Fixed-Point

_Gemessen von Worker-2 für #550 AC4, von mir mit eigenem Lauf nachgeprüft — reproduziert
exakt. Der Befund geht weit über die AC hinaus, die ihn ausgelöst hat._

Die Frage war: **Verschiebt Ausrüstung, welche Zauber gewirkt werden — oder nur wie viel
Schaden sie machen?** Gemessen über die vier Bosse der ersten Instanz:

```
NEUTRAL                    11 von 58   kindle smoulder pyre bellows searing quench
                                       purge crucible-skin efflux aether rot

Stufe 13, rare              9 von 58   fällt weg: kindle bellows efflux aether
(catalysis 1230, purity 1220)          kommt dazu: scab decoction

Stufe 85, legendary         1 von 58   nur noch searing
(catalysis/purity/yield je 36 220)
```

**Ausrüstung verschiebt die Auswahl nicht nur — bei voller Bestückung vernichtet sie sie.**
Von elf lohnenden Zaubern bleibt **einer**.

Das trifft Leitplanke 2 aus dem Design frontal: *„jeder Zauber muss irgendwo die beste Wahl
sein."* Im Endgame ist er es nicht — dort ist es immer derselbe.

**Damit ist der Werte-Deckel aus dem Plan keine Fixed-Point-Frage mehr, sondern eine
Spielfrage.** Der Plan formuliert ihn als Fairness-Argument (*„ab da wird es reines Können"*);
gemessen ist er die Bedingung dafür, dass das Zauberbuch überhaupt benutzt wird. Ohne ihn
konvergiert jede Rotation im Endausbau auf denselben einen Zauber.

Warum, ist nicht gemessen, aber die Zahlen legen es nahe: `catalysis 36 220` ist Faktor 36 auf
die Beschleunigung. Wenn Abklingzeiten praktisch verschwinden, zählt nur noch, welcher Zauber
je globaler Wartezeit am meisten trägt — und das ist genau einer.

---

---

## Messung 3 — Trace-Wiederverwendung: **eine Regel, keine Statistik**

_Von Worker-3 gebaut, von mir mit einem eigenen Lauf nachgemessen — reproduziert exakt._

Die Gesamtverteilung sagt zunächst das Erwartbare:

```
563 Varianten aus einer gefundenen Rotation (40 Entscheidungen)
identischer Verlauf über den ganzen Kampf   94 von 563   16,7 %

erste Abweichung bis Entscheidung  0   11,0 %
                                   1   21,5 %
                                   5   62,5 %
                                  20   83,3 %

Median des gemeinsamen Präfix   4 von 40      Mittelwert 9,9
```

Als Mittel gelesen heißt das **„divergiert sofort"**. Das ist richtig und verdeckt das
Entscheidende. Nach **Position** der Änderung aufgeschlüsselt hat die Zahl **null Streuung**:

```
getauscht    Position 0 → Präfix 0,0   (39 Varianten)
             Position 1 → 1,0  (38)     Position 4 → 4,0  (40)
             Position 2 → 2,0  (38)     Position 5 → 5,0  (40)
             Position 3 → 3,0  (38)     Position 6 → 6,0  (40)

vertauscht   dieselbe Folge, Position 0…6 → 0,0 bis 6,0

eingefügt    Position 0 → 12,0          Position 5 → 18,3
             Position 1 → 14,2          Position 7 → 21,8
             Position 3 → 17,0          Position 8 → 40,0   ← die ganze Folge
```

> **Eine Änderung an Zeile `p` lässt genau die ersten `p` Entscheidungen unberührt.**

Bei 38–40 Varianten je Position kein einziger Ausreißer. **Der wiederverwendbare Anteil ist
vor dem Lauf bekannt** — das ist eine Invariante, keine Verteilung, und genau das braucht
Differential Simulation.

Beim **Einfügen** liegt der Präfix *über* der Position, weil die neue Zeile oft gar nicht
drankommt; ans Ende eingefügt bleibt die Folge vollständig gleich. **Und Einfügen ist genau
das, was `greedy` in jeder Runde an jeder Position tut.**

Worker-3 hat diese Aufschlüsselung ungefragt gemacht, weil ihm auffiel, dass „eingefügt 19,4"
und „getauscht 3,7" zu weit auseinanderliegen, um derselbe Mechanismus zu sein. Sie ist der
wertvollste Teil der ganzen Messreihe.

**Eine Abweichung vom Auftrag, die richtig war:** Ich hatte `checksum_at` je Zeitschritt
vorgegeben. Worker-3 hat stattdessen die Cast-Folge verglichen, mit der Begründung: die
Prüfsumme sagt, *dass* sich der Zustand unterscheidet, nicht *an der wievielten Entscheidung*.
Meine Frage war nach der Entscheidung.

---

## Messung 5 — Homogenität: null Einzelgänger

_Ebenfalls von mir nachgemessen, reproduziert exakt._

```
bewertete Rotationen in EINER Suchrunde   558   (93 Kandidaten × 6 Positionen)
verschiedene Gestalten                     76
mittlere Gruppengröße                    7,34
Median                                      6
Einzelgänger                       0 von 76   (0,0 %)

zehn größte   30, 30, 12, 12, 12, 12, 12, 12, 12, 12   → 28,0 % der Runde
```

„Gestalt" heißt: gleicher Boss, gleiche Menge benutzter Zauber, gleiche Zahl bedingter Zeilen.

**Für die GPU-Frage ist das die notwendige, nicht die hinreichende Bedingung.** Es gibt
gleichartige Stapel — aber die größten haben 30 Mitglieder, nicht 30 000. Sie sind zu klein,
um einen Warp sinnvoll zu füllen, ohne über Gruppen hinweg zu bündeln. Zusammen mit dem
Befund aus Messung 2 (verzweigungsgebunden, datenabhängige Kontrollflüsse) spricht das
**gegen CUDA als frühen Schritt**. Das ist meine Einschätzung; gemessen sind die Gruppengrößen.

---

## Messung 7 — Suchqualität gegen exakt lösbare Räume

_Von Worker-2 gebaut, von mir mit eigenem Lauf nachgemessen — reproduziert Zahl für Zahl._

```
Verfahren            Treffer   Quote   Regret-M   Regret-Max   Messungen
greedy ohne Paare     48/60   80,0 %    31,58 %    100,00 %        1 770
greedy mit Paaren     50/60   83,3 %    15,03 %     56,52 %        1 892
erschöpfend           60/60  100,0 %     0,00 %      0,00 %       62 328
```

`Regret-M` ist bewusst das Mittel über die **verfehlten** Fälle — über alle gemittelt sänke
die Zahl, je besser das Verfahren wird, ohne dass die Fehler kleiner werden.

**Der stärkere Befund steht im Regret, nicht in der Quote:** `Regret-Max = 100,00 %` ohne
Paare heißt, dass Greedy dort **gar nichts fand** — Score 0, während das Optimum darüber lag.
Genau der Fall, für den die Paarbildung gebaut wurde: eine Zeile allein bringt nichts, erst
das Paar zahlt. Mit Paaren verschwindet er, und das Regret-Mittel halbiert sich.

Die zehn Verfehler, die auch mit Paaren bleiben (Auszug):

```
Fall 31  boss-1-4   7 Kandidaten, Länge 4   gefunden  360,0   Optimum  828,0   56,52 %
Fall 25  boss-1-2   9 Kandidaten, Länge 3   gefunden  500,0   Optimum  676,0   26,04 %
Fall 33  boss-1-2   9 Kandidaten, Länge 3   gefunden 2010,0   Optimum 2500,0   19,60 %
```

**Reichweite, ausdrücklich:** Diese Zahlen gelten für 6–9 Kandidaten und Länge 3–4. Ob Greedy
im echten Raum (93 Kandidaten, Länge 6, über 600 Milliarden Listen) ebenso trifft, sagt die
Messung **nicht**. Sie sagt: dort, wo sich prüfen lässt, liegt es in **einem von sechs Fällen**
daneben — und die 10-%-Spielbarkeitsschwelle wird dabei in vier Fällen gerissen.

### Alle fünf Verfahren — nachgeliefert

Sie hatten fünf verlangt, mein Auftrag nannte nur drei. Die fehlenden habe ich selbst gebaut
und gegen **dieselben 60 Fälle** gemessen:

```
Verfahren            Treffer   Quote   Regret-M  Regret-Max   Messungen
greedy ohne Paare     48/60    80,0 %    31,58 %   100,00 %       1 770
greedy mit Paaren     50/60    83,3 %    15,03 %    56,52 %       1 892
beam k=3              44/60    73,3 %    20,30 %    67,19 %       2 988
beam k=10             51/60    85,0 %    15,76 %    47,83 %       8 010
LNS                   55/60    91,7 %    21,74 %    56,52 %       9 969
QD (Archiv)           41/60    68,3 %    21,27 %    69,83 %       6 337
erschöpfend           60/60   100,0 %     0,00 %     0,00 %      62 328
```

**LNS gewinnt deutlich: 91,7 % für 5,3× die Messungen von `greedy`.** Zum Vergleich kostet
die erschöpfende Suche das 33-fache für die restlichen 8,3 Punkte.

**Beam mit k=3 ist schlechter als `greedy` — 73,3 % gegen 83,3 %.**

> **Nachtrag: meine erste Erklärung dafür war falsch, und die richtige ist gemessen.**
>
> Ich hatte geschrieben, es liege an der fehlenden Paarbildung — ausdrücklich als Erklärung
> gekennzeichnet, nicht als Messung. Worker-3 hat sie mit einer Rechnung angegriffen: Die
> Paarbildung ist in dieser Reihe **zwei Fälle** wert (50 gegen 48), der Rückstand des Beams
> beträgt **sechs** (44 gegen 50). Sie deckt höchstens ein Drittel.
>
> Zwei Prüfungen haben den echten Grund gefunden:
>
> ```
> Zusicherung (n=6, L=3, k=30 — bei dieser Breite darf nichts abgeschnitten werden):
>   erschöpfend  3480,0 in 156 Messungen
>   beam k=30    3480,0 in 156 Messungen     → gleich, der Beam ist korrekt gebaut
>
> beam k=1     34/60   56,7 %      ← greedy ohne Paare: 48/60
> beam k=3     44/60   73,3 %
> beam k=10    51/60   85,0 %
> ```
>
> **`greedy` fügt an jeder Position ein, mein Beam hängt nur an.**
> (`rotation_search.rs:579` und `:655` — `versuch.insert(stelle, …)`.) Das ist derselbe
> Mechanismus, der im Projekt schon einmal gemessen wurde: *„gierig (nur anhängen) 4000,0
> gegen einfügen an jeder Stelle 5960,0."*
>
> Damit erklärt sich die ganze Spalte: `beam k=1` liegt unter `greedy`, weil ihm das Einfügen
> fehlt; `beam k=10` liegt darüber, weil die Breite den Nachteil überkompensiert. Die
> Paarbildung ist **nicht** der Grund.
>
> **Für die Empfehlung ändert das nichts** — LNS bleibt vorn. Aber die Beam-Zeilen sind
> damit eine Untergrenze: Ein Beam, der einfügt, wäre besser als der gemessene. Die Messung
> läuft.

**QD ist erwartungsgemäß das schwächste am Maximum (68,3 %)** — es sucht nicht das Maximum,
sondern Vielfalt. Gemessen deckt es **9,9 Nischen je Fall** ab. Die Achsen (Länge, Zahl der
bedingten Zeilen) sind **meine Wahl**, weil die eigentliche Nischen-Definition Ihre offene
Entscheidung Nr. 2 ist. Andere Achsen geben andere Zahlen; die Aussage „QD trifft das Maximum
seltener, deckt dafür Nischen ab" hängt am Verfahren, nicht an der Achsenwahl.

**Empfehlung, ausdrücklich als solche:** LNS als Standardverfahren, `greedy mit Paaren` als
schneller Vorlauf. Beam bringt bei k=10 fast nichts gegenüber `greedy+Paare` (+1,7 Punkte für
4× die Messungen) und ist damit der schlechteste Tausch der drei.

**Was das für die Balance-Aussagen bedeutet, und es ist der wichtigere Teil:** Mit `greedy`
liegt jede sechste Aussage „Zauber X gewinnt hier nicht" daneben. Mit LNS ist es jede zwölfte.
Für ein Werkzeug, dessen Zweck es ist, tote Zauber zu finden, ist das kein
Geschwindigkeitsthema — es entscheidet, ob die Antwort stimmt.

Der Determinismus wurde nicht behauptet, sondern gemessen: zwei Läufe, Programmausgabe
`md5 0abce38f8958961d5d460231ab39ca09` beide Male, ohne Zufallszahlengeber.

---

## Messung 4 — Kanonische Phänotypen

### Teil A, Ausrüstung: **aus 17 496 Genotypen werden 56 Phänotypen — Faktor 312**

_Von Worker-1 gemessen. **Nicht von mir nachgemessen** — der Läufer ist noch nicht gepusht;
die Zahl steht unter diesem Vorbehalt._

```
Stücke je Platz     3 3 3 3 3 3 3 2 2 2
Genotypen        17 496    vollständig aufgezählt, keine Stichprobe
Phänotypen           56
Faktor           312,43
```

**Der Grund ist strukturell, nicht zufällig** — und das ist die eigentliche Antwort auf Ihre
Frage:

1. Der Katalog hat nur **drei Affix-Muster** auf 27 Stücken (11× `catalysis,purity,yield`,
   10× `catalysis,essence,purity`, 6× `catalysis`), und alle tragen Level 40, Qualität epic.
2. **Die Summe ist kommutativ.** Für den Kampf zählt, *wie viele* Stücke jedes Musters man
   trägt — nicht welche und nicht auf welchem Platz.

> Solange Ausrüstung nur über additive Werte wirkt, ist der Phänotyp-Raum die Zahl der
> **Multimengen**, nicht der Anordnungen.

Mit Träger-Zusicherung geprüft, damit 312 nicht das Ergebnis eines zu groben Schlüssels ist:
drei Stücke auf Platz 0 ergeben **2** verschiedene Schlüssel (nicht 1), derselbe Aufbau
zweimal denselben.

### Der Befund, der meine frühere Aussage an Sie korrigiert

Ich hatte gemeldet: „Sechs Set-Effekte existieren, jeder mit eigenem Prüffall, jeder als
Wahrheitswert im Kampf." Das stimmt **für den Simulator**. Worker-1 hat nachgesehen, was das
Spiel liefert:

```
data/sets.json    sechs Sätze
data/items.json   SECHS Stücke tragen ein setId — und alle sechs gehören zu Satz 0
                  ("Test Set", Plätze 1 bis 6)

Die Sätze 1 bis 5 haben NULL Gegenstände.
```

**Fünf der sechs Sätze sind im Spiel nicht zusammensetzbar**, egal wie man die 27 Stücke
kombiniert. Die Satz-Effekt-Maske ist heute faktisch eindimensional. Für die Architektur
ändert das nichts — der Platz für die Maske muss vorgesehen werden, weil der Plan sechs Sätze
je Endgame-Instanz vorsieht. Für die *Zahl* 312 heißt es: sie gilt für einen Katalog mit
**einem** belegten Satz.

### Teil A2, Ausrüstung im Endausbau: **Faktor 1162 — die Faltung wird stärker**

_Von Worker-1 gemessen, vollständig aufgezählt (79,6 s), unter drei benannten Annahmen:
sechs Sätze zu sechs Stücken, jeder Satz auf den Plätzen 1–6, 21 satzlose Stücke._

```
                     Genotypen    Phänotypen    Faktor
heutiger Bestand        17 496            56       312
sechs Sätze belegt   6 291 456         5 412     1 162
```

**Der Genotyp-Raum wächst um Faktor 360, der Phänotyp-Raum nur um 97.** Je größer der
Katalog, desto stärker die Faltung — genau, was aus der kommutativen Summe folgt: mehr Stücke
desselben Musters erzeugen keine neuen Zustände.

Die 5 412 sind eine **Untergrenze**: Die hochgerechneten Satz-Stücke tragen dieselben Werte
wie die heutigen und unterscheiden sich nur durch die Satz-Kennung. Ein echter Katalog gäbe
ihnen eigene Affix-Muster.

### Teil B, Talente: **Faktor 234 — und die Ursache ist eine einzige Zahl**

_Von mir gerechnet, analytisch. Worker-1 misst dieselbe Größe unabhängig; beide Zahlen werden
verglichen, sobald seine vorliegt._

**Der erste Anlauf war falsch angelegt** und scheiterte bei 2,1 GB Speicher: Er hielt die
Menge aller Teilzustände über sechs Bäume. Das war auch unnötig — die Struktur löst die Frage
fast von selbst.

**Von den 18 Wirkgrößen wird fast jede von genau einem Talent bestimmt.** `resourceGain` nur
von `calcination-build`, `controlCost` nur von `fixation-cost`, und so fort. Der Phänotyp legt
diese Talente eindeutig fest. Drei Größen teilen sich Talente:

```
dotDuration     = calcination-hold·100 + global-hold·60
shieldDuration  = coagulation-longer·80 + global-hold·60
damage          = (Summe der SECHS Stämme)·20 + calcination-release·25
```

Weil `global-hold` über `decayDuration` eindeutig ist (es ist das einzige Talent dieser
Größe), sind die ersten beiden danach auflösbar. **Es bleibt genau eine echte Faltung: `damage`.**

```
Stamm-Vektoren (sechs Stämme, je 0..3)      4 096   →  auf 19 Summen
gültige (Stammvektor, release)-Kombis      19 456   ← mit requires
verschiedene damage-Werte                      83
FAKTOR                                     234,41

mehrdeutig auf Vektor-Ebene:  75 von 83
mehrdeutig auf Paar-Ebene:    26 von 83
  100‰ ← (0 Stämme, 4 release)  oder  (5 Stämme, 0 release)
  125‰ ← (0, 5)  oder  (5, 1)
```

**Die `requires`-Kette ist eingerechnet** (`calcination-release` braucht
`calcination-trunk` ≥ 1). Ohne sie wären es 24 576 → 86 → Faktor 285,77; mit ihr fallen
5 120 Kombinationen und drei damage-Werte weg. Worker-1 hat das gemessen und meine erste
Zahl damit korrigiert.

**Die beiden Mehrdeutigkeits-Zahlen zählen Verschiedenes** und sind kein Widerspruch: 75
Werte entstehen aus mehr als einem *Stammvektor* (bloße Umverteilung derselben Punktzahl),
26 aus mehr als einem *(Summe, release)-Paar* — also aus strukturell verschiedenen Wegen.
Die zweite ist die interessantere.

**Die Ursache ist eine einzige Design-Entscheidung: alle sechs Stämme tragen dasselbe
`damagePerRankPerMille: 20`.** Damit sind sie für den Simulator untereinander
ununterscheidbar — 4 096 Vektoren falten auf 19 Summen. Dazu kommt, dass 20·5 = 25·4 gilt.

**Einschränkung, die den Faktor zur Obergrenze macht:** Ein Stamm auf Rang 0 sperrt seine drei
Zweige (`requires`). Stammvektoren mit Nullen sind also nur erreichbar, wenn die zugehörigen
Zweige leer bleiben — und dann unterscheiden sie sich in den übrigen Größen ohnehin nicht.

### Die exakte Zahl gültiger Talentverteilungen — nachgeliefert

Sie hatten sie als „wichtigsten fehlenden kombinatorischen Wert" bezeichnet. Sie ist exakt
abzählbar, wenn man nicht aufzählt, sondern **zählt** — dynamische Programmierung über die
Punktsumme, je Baum die Zahl der gültigen Belegungen, dann Faltung auf genau 50:

```
Je Baum, gültig (mit requires) gegen roh:
  calcination     1 189 von 1 728      fixation        1 189 von 1 728
  sublimation     1 189 von 1 728      coagulation     1 189 von 1 728
  putrefaction    1 189 von 1 728      global            649 von   864

GÜLTIGE Verteilungen mit GENAU 50 Punkten     32 149 887 516 572 958   ≈ 3,21·10¹⁶
roh, ohne requires                           481 326 061 513 782 852   ≈ 4,81·10¹⁷
Anteil, den requires wegnimmt                                  93,32 %

GÜLTIGE mit HÖCHSTENS 50 Punkten             151 415 149 326 013 387   ≈ 1,51·10¹⁷
```

**Die `requires`-Ketten nehmen 93,3 % des Rohraums weg.** Das ist die größte Einzelreduktion
im ganzen Dokument — und sie kostet nichts, weil sie eine Spielregel ist und keine Näherung.

**Ihre Zahl 7·10⁴⁵ war eine andere Rechnung**, vermutlich 50 Punkte frei auf 113 Ränge ohne
die Baumstruktur. Die tatsächliche Zahl liegt 29 Größenordnungen darunter.

**Zwei Kontrollzeilen mit vorher feststehendem Wert** (nach Worker-3s Meta-Befund, siehe
unten): bei 0 Punkten genau **1** Verteilung, bei 1 Punkt genau **6** (nur je ein Stamm ist
belegbar). Beide trafen; die Faltung rechnet richtig.

### Der Vollraum, mit dieser Zahl gerechnet

```
GENOTYPEN heute      Talente × Gear × Rotation           1,12·10⁴⁴
                     3,21·10¹⁶ × 17 496 × 2,00·10²³

nach den Faltungen   Talent-Phänotypen   1,37·10¹⁴   (Faktor 234)
                     Gear-Phänotypen            56   (Faktor 312)
                     Rotationen          4,02·10²²   (Faktor 5)
                     Produkt                          3,09·10³⁸
                     Gesamtfaktor                     3,63·10⁵
```

Bei 350 000 Kämpfen/s bräuchte der reduzierte Raum immer noch **2,8·10²⁵ Jahre**. Das ist die
Zahl, die den Plan bestätigt: *„Der Vollraum bleibt unerreichbar — erschöpfend gerechnet
werden nur die Projektionen."* Kanonisierung um Faktor 360 000 ändert daran nichts; sie macht
die **Projektionen** billiger, nicht den Vollraum machbar.

### Ein Befund, der die Talentzahl erst deutbar macht

Worker-1 hat vor der Zählung geprüft, was ich ihm als Vorbehalt mitgegeben hatte:

```
sim/src/balance.rs   61 Treffer für talent_*_permille / talent_*_bonus
sim/src/             NULL Treffer für "requires"
src/progression/talent-view.ts:79   if (talent.requires === null) return true
```

**Der Simulator setzt die `requires`-Ketten nicht durch — er kann es gar nicht.** Er bekommt
fertige Promille-Summen und kann nicht wissen, ob sie aus einer gültigen Punkteverteilung
stammen. Eine ungültige fällt ihm nicht auf.

Für die Architektur heißt das: **„Genotyp" muss bedeuten „eine Verteilung, die die Spielseite
zulässt"**, nicht „eine, die der Simulator schluckt" — sonst zählt man Zustände mit, die kein
Spieler je erreicht, und bekommt eine zu große Zahl.

---

---

# Was ich aus den sieben fertigen Messungen ableiten würde

_Einschätzung, nicht Messung — die Zahlen darin sind gemessen._

**1. Die Reihenfolge Ihrer Architekturskizze sollte sich umdrehen.** Sie haben

```
Exact reductions → Search → Incremental evaluator → Execution dispatcher
```

Gemessen liegt der größte sichere Gewinn ganz unten und ganz billig: `expire_buffs` und der
Heap sind zusammen **37 % der Zyklen**, ohne dass sich eine Spielregel, eine Kanonisierung
oder eine Suchstrategie ändert. **Und beide sind inzwischen nicht mehr nur benannt, sondern
beziffert:** Von den 360 `.expire`-Aufrufen je Kampf findet höchstens **einer** etwas
(99,7 % einsparbar), und die Ereignisschlange hat nie mehr als **neun** Einträge. Zwei
Nachmittage Arbeit für einen Faktor, der über jeder Kanonisierung liegt, die ich beziffern
kann.

**2. Der Incremental Evaluator ist der zweite Schritt, und Messung 3 hat ihn von einer
Wette in eine Rechnung verwandelt.** Der wiederverwendbare Anteil ist keine Statistik,
sondern eine Invariante: Änderung an Position `p` → Präfix `p` unberührt, null Streuung.
`greedy` fügt an jeder Position ein; bei Länge 6 ist der erwartete wiederverwendbare Anteil
damit rund die halbe Folge, beim Einfügen mehr. **Das ist ein Faktor ~2, exakt begründbar,
und er multipliziert mit allem anderen.**

**3. Die Rotations-Kanonisierung ist Faktor 5 (Endausbau 10,6), nicht 10⁵.** Sie bleibt
richtig, trägt aber die Architektur nicht — anders als Sie und ich zunächst vermutet haben.

**4. Die Kanonisierung ist der eigentliche Hebel, und sie multipliziert sich.** Gemessen:

```
Ausrüstung  heute      Faktor    312
            Endausbau  Faktor  1 162
Talente                Faktor    286   (Obergrenze)
Rotation               Faktor      5   (Endausbau 10,6)
                                 ─────
zusammen, Endausbau    Faktor ~35 000
```

Alle drei folgen demselben Mechanismus: **der Simulator sieht Summen, keine Herkunft.** Weil
Summen kommutativ sind, zählt die Multimenge und nicht die Anordnung. Bei den Talenten kommt
eine einzelne Design-Entscheidung dazu, die den Löwenanteil trägt — **alle sechs Stämme tragen
denselben Wert 20**, also sind sie ununterscheidbar.

Die Faktoren sind nicht sauber unabhängig (das Punktebudget koppelt), aber sie greifen an
verschiedenen Achsen an. **Das ist die Größenordnung, in der die Architektur den Gewinn
suchen sollte — nicht bei SIMD.**

**5. Der Krit ist bei `purity = 0` gratis** (gemessen: null Würfe), und die 50–200 Durchgänge
je Rotation sind heute identische Rechnungen — **der einzige Faktor, der ohne jede
Codeänderung sofort verfügbar ist.**

> **KORRIGIERT am 2026-08-14.** Hier stand zusaetzlich, der Krit sei an 15 der 17 echten
> Bossen analytisch behandelbar. Das ist eine Bestandsaussage, die im Endausbau falsch wird:
> 68 Bosse sind geplant, und `purity = 0` gilt nur fuer neutrale Faktoren. **Die Zulaessigkeit
> entscheidet eine Eigenschaftspruefung, nie eine Bossliste.** Begruendung und Vorbehalt
> stehen in der SSOT: `docs/SIM-ARCHITEKTUR-PLAN.md`, Teil 0 und E11.1b.

**6. Gegen CUDA als frühen Schritt sprechen jetzt zwei Messungen.** Der Kernel ist
verzweigungsgebunden (155 Fehlsprünge je Kampf; die teuersten Stellen sind datenabhängige
Kontrollflüsse), und die homogenen Stapel sind zwar lückenlos (null Einzelgänger), aber mit
30 Mitgliedern in der größten Gruppe zu klein, um einen Warp zu füllen. AVX2 auf der
Zielmaschine ist der naheliegendere Weg — und beides kommt nach Punkt 1 und 2.

**7. Die Suche ist schwächer als gedacht — und LNS behebt die Hälfte davon.** Gemessen über
alle fünf Verfahren: `greedy mit Paaren` 83,3 %, `beam k=10` 85,0 %, **LNS 91,7 %**, QD 68,3 %.
Es geht dabei nicht um Geschwindigkeit, sondern um Aussagen, die sonst falsch sind: **Mit
`greedy` liegt jede sechste Aussage „Zauber X gewinnt hier nicht" daneben, mit LNS jede
zwölfte.** Für ein Werkzeug, dessen Zweck das Finden toter Zauber ist, ist das der Unterschied
zwischen brauchbar und irreführend. Empfehlung: LNS als Standard, `greedy+Paare` als Vorlauf.

**8. Vor Fixed-Point muss der Werte-Deckel aus dem Plan eine Zahl bekommen.** Sonst entwirft
man gegen einen Bereich, der bei Stufe 100 sechsstellig wird.

---

## Der Stand in einer Tabelle

| # | Messung | Ergebnis | selbst nachgemessen |
| --- | --- | --- | --- |
| 1 | EventQueue-Belegung | max **9**, p95 8, EVENT_CAPACITY 28× zu groß | ja |
| 2 | Branch-Misses nach Bereich | `expire_buffs` 23,8 %, Heap 16,1 %, **APL nur 4,1 %** | ja |
| 3 | Trace-Wiederverwendung | **Position p → Präfix p, null Streuung** | ja |
| 4A | Gear-Phänotypen heute | 17 496 → 56, **Faktor 312** | nein (Läufer nicht gepusht) |
| 4A2 | Gear-Phänotypen Endausbau | 6 291 456 → 5 412, **Faktor 1 162** | nein (dito) |
| 4B | Talent-Phänotypen | 19 456 → 83, **Faktor 234** (Obergrenze) | ja, zweifach |
| 5 | Homogenität | 76 Gestalten, **null Einzelgänger**, größte 30 | ja |
| 6 | Fixed-Point-Bereiche | heute Produkt 7,3–12,2 · **Endausbau unbeschränkt, Deckel fehlt** | ja |
| 7 | Suchqualität | **LNS 91,7 %** > beam k=10 85,0 % > greedy 83,3 % — alle fünf Verfahren | ja |
| 8 | Always-Spells | **Faktor 5,0** (Endausbau 10,6) — nicht 140 000 | ja |
| — | Durchsatz 5900HS | **350 000–365 000 Kämpfe/s** ±12 %, zweimal korrigiert | ja |
