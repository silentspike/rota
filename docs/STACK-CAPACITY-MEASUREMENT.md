# Stapelraten und erreichbare Höhen (#1102 AC1/AC3)

Gemessen am 2026-09-06 auf Basis von `8fbcddee`, mit unveränderten Produktionszaubern
und Produktionsregeln. **AC2 bleibt offen:** Diese Messung ändert weder Balance noch Design.

Die pauschale Aussage „acht Zersetzungsstapel sind ohne ROT unerreichbar“ trägt für das
heutige Kit nicht: **SEPSIS erreicht ohne ROT acht Stapel bei 13,5 s**, wenn ein gültiges
Talent die BLOOM-Obergrenze auf acht hebt. Die engere CANKER/BLIGHT/BLOOM-Priorität
erreicht in derselben Messung sieben. ROT bleibt damit eine zusätzliche Stapelquelle,
ist aber nicht die einzige Möglichkeit, acht gleichzeitig liegende Stapel zu erreichen.

## Reproduzieren und Grenzen der Aussage

```bash
npx vite-node tests/measurements/export-stack-capacity.ts > /tmp/rota-stack-capacity.json
npx vitest run tests/unit/stack-capacity-measurement.test.ts
```

Der [Messweg](../tests/measurements/stack-capacity.ts) verwendet die echte Trainingspuppe
aus `buildDummy(DEFAULT_DUMMY, 'CALCINATION')`, `Combat.stepPassive` und `Combat.cast`:
120 s Kampfzeit, 10-ms-Takte, Seed 0, Stufe 100, keine Ausrüstung, keine Set-Effekte.
Alle verwendeten Zauber sind echte, auf dieser Stufe verfügbare Zauber; jede Leiste und
Talentverteilung passt in die tatsächlichen Freischaltungen. Kein privater Zustand,
Zauberwert, Cooldown, GCD, Ablaufzeit oder Deckel wird überschrieben.

Die höchste **beobachtete** Höhe wird nach jedem passiven Takt und jedem akzeptierten
Wurf über `[0, 120 s)` gelesen. Die Raten zählen ausschließlich `[30, 120 s)`, also
90 s nach einem getrennten Anlauf. Die Prioritäten sind reproduzierbare Messfolgen,
keine Rotationsempfehlungen im Spiel und keine vollständige Optimierungssuche.
Gemessen wird an der Trainingspuppe; Bossdruck, Schutzverbrauch durch Treffer, CATALYSIS,
Set-Effekte, andere Talentkombinationen, Cooldown-Resets, SUBLIME und CONDENSE sind
nicht untersucht. Die Zahlen sind weder globale Maximalraten noch eine Balance-Abnahme.

**Zwei verschiedene Raten:** Quellenstapel werden aus tatsächlich ausgelösten
`spell.cast`-/`dot.tick`-Ereignissen gezählt, vor der Deckelung. Aufgenommene Stapel
werden aus den öffentlichen individuellen Ablaufuhren vor/nach einem Takt oder Wurf
ermittelt; gleichzeitig verfallende und neue Stapel gehen dadurch nicht verloren.
SEPSIS verschiebt vorhandene Uhren und zählt nicht als Zugang. Die untersuchten
Talente verändern keine Stapelmenge je Quelle; der Messweg lehnt nicht abgedeckte
Bruchteilsgewinne und Decay Echo ausdrücklich ab.

## AC1: Zersetzung

`Direkt` bedeutet CANKER vor BLIGHT, jeweils sobald bereit. `ROT + Direkt` priorisiert
ROT. BLOOM hat davor Vorrang, sobald mindestens vier Stapel liegen und er bereit ist;
SEPSIS hat im letzten Fall noch davor Vorrang, ebenfalls ab vier Stapeln. Jeder dieser
Würfe bezahlt den tatsächlichen 1.500-ms-GCD. BLOOM und SEPSIS werden wiederholt gewirkt.
`+1 Platz` ist die gültige Verteilung `putrefaction-trunk: 1, putrefaction-build: 3`:
vier Talentpunkte, keine Datenmutation. BLOOM selbst behält seinen Bonus von zwei.

| Messfolge                         | Höchster Deckel | Beobachtete Höhe | Erstmals bei | Quellenstapel / 90 s | Quellen / s | Aufgenommen / s |
| --------------------------------- | --------------: | ---------------: | -----------: | -------------------: | ----------: | --------------: |
| Direkt                            |               5 |                5 |          6 s |                   60 |      0,6667 |          0,4000 |
| Nur ROT                           |               5 |                5 |          5 s |                   90 |      1,0000 |          0,3778 |
| ROT + Direkt                      |               5 |                5 |          3 s |                  120 |      1,3333 |          0,3556 |
| BLOOM + Direkt                    |               7 |                7 |       10,5 s |                   54 |      0,6000 |          0,4000 |
| BLOOM + ROT + Direkt              |               7 |                7 |          6 s |                  110 |      1,2222 |          0,4667 |
| BLOOM + Direkt, +1 Platz          |               8 |                7 |       10,5 s |                   54 |      0,6000 |          0,4667 |
| BLOOM + ROT + Direkt, +1 Platz    |               8 |                8 |        6,5 s |                  108 |      1,2000 |          0,5333 |
| SEPSIS + BLOOM + Direkt, +1 Platz |               8 |                8 |       13,5 s |                   44 |      0,4889 |          0,0000 |

Die Gegenprobe ist der unveränderte Acht-Platz-Deckel in den letzten drei Zeilen:
ROT entfernen lässt die beobachtete Höhe auf sieben fallen; SEPSIS stattdessen
erreicht acht. Die letzte Null ist kein wirkungsloser Erzeuger: Alle acht Plätze sind
bereits vor dem Ratenfenster belegt und werden durch SEPSIS erhalten. Weitere angebotene
Stapel werden abgewiesen. Ebenso bedeutet die geringere Aufnahmerate der ROT-Grundfolge
nicht, dass ROT weniger Quellen erzeugt: Der Fünferdeckel und die individuellen
Ablaufuhren bestimmen die Aufnahme; es gibt hier keinen Entlader.

**Die frühere Schätzung von etwa 1,4 Quellenstapeln/s ist in dieser Folge nicht bestätigt.**
Gemessen sind ohne BLOOM 80 ROT-Ticks und 40 Direktwürfe in 90 s, also 1,3333/s.
ROT wird mit Direktfüllern alle 4,5 s gewirkt, nicht alle 4 s; seine Würfe belegen ebenfalls
GCDs. Mit BLOOM sind es 74 ROT-Ticks und 36 Direktwürfe, also 1,2222/s. Der Schritt von
1,2222 auf 1,2000 im Acht-Platz-Fall folgt aus der zustandsabhängigen Vier-Stapel-Bedingung
für BLOOM und deren verändertem Ablauf, nicht aus einem anderen ROT-Datensatz.

## AC3: weitere untersuchte Grenzen

| Messfolge                     |                                                       Grenze | Beobachtete Höhe | Erstmals bei | Befund in diesem Umfang                                                          |
| ----------------------------- | -----------------------------------------------------------: | ---------------: | -----------: | -------------------------------------------------------------------------------- |
| KINDLE                        |                                                     Hitze 20 |               20 |         12 s | Grundgrenze erreicht                                                             |
| SLAG + KINDLE                 |                                          Hitze 30 im Fenster |               26 |         18 s | Diese Priorität füllt den erhöhten Deckel nicht                                  |
| SLAG + BELLOWS                |                                          Hitze 30 im Fenster |               30 |       10,5 s | Gegenprobe: derselbe Deckel ist mit echtem stärkerem Erzeuger erreichbar         |
| CRUST                         |                                                  3 Schichten |                3 |         12 s | Grundgrenze erreicht                                                             |
| CRUST + Layered Ward          |                                                  8 Schichten |                8 |         42 s | Gemeinsame Ablaufuhr wird erneuert; kein Einzelstapel-Verfall wie bei Zersetzung |
| SCAB + Layered Ward           |                                                  8 Schichten |                8 |       10,5 s | Trotz 8 s Schilddauer erreicht die erneuerte gemeinsame Uhr acht                 |
| PLATE + Layered Ward          |                                                  6 Schichten |                1 |          0 s | Eigene Abklingzeit 20 s ist länger als 15 s Laufzeit; kein GCD-Beweis            |
| HARDEN + PLATE + Layered Ward |                                                  6 Schichten |                6 |        100 s | Gegenprobe: Verlängerung erreicht denselben Deckel                               |
| CRUCIBLE SKIN + Layered Ward  |                                                  6 Schichten |                1 |          0 s | Eigene Abklingzeit 45 s gegen 6 s Laufzeit; nur diese Einzelquelle untersucht    |
| BIND                          | kein endlicher Kontrollpunkte-Deckel im aktuellen Gewinnpfad |              160 |      118,5 s | 80 akzeptierte Würfe; keine gedeckelte Ressource in diesem Vergleich             |

SLAG wird vor dem Erzeuger ab 20 vorhandener Hitze gewirkt, HARDEN vor PLATE sobald
ein Schild existiert und der Zauber bereit ist. `Layered Ward` meint
`coagulation-trunk: 1, coagulation-stacks: 5`, insgesamt sechs gültige Talentpunkte.
Die Schildgrenzen gelten beim jeweiligen Erzeuger an einem gemeinsamen Schildspeicher;
sie sind keine voneinander unabhängigen Schildarten.

Es wurde in diesen Fällen **kein weiterer global unerreichbarer GCD-Deckel nachgewiesen**.
Der unvollständig gefüllte SLAG-Deckel ist ein Befund zur genannten Folge; die Schildfälle
trennen eigene Cooldowns von der GCD. Andere Leisten, Buffkombinationen, Reset-Ladungen,
Ward-/Attribut-Deckel und konsumierende Rotationen wurden nicht systematisch durchsucht.

Die [acht Regressionstests](../tests/unit/stack-capacity-measurement.test.ts) prüfen die
gemessenen Quellen und Höhen, ROT-Entfernung, SEPSIS-/HARDEN-Gegenproben, die GCD-Abstände,
den kompletten 120-s-Lauf und die Unverändertheit des Zustands durch den Beobachter.
Eine aufgezeichnete ROT/BLOOM-Tastenfolge wird zusätzlich über den bestehenden
`replayRecording`-Leser abgespielt: identische akzeptierte Würfe, Schaden und Endprüfsumme.
