---
id: pruefstein-509
status: Review
ssot: false
refs:
  docs: [GAME-DESIGN.md]
---

# Prüfstein: Gibt es überhaupt etwas, worauf eine Rotation reagieren kann?

**Issue #509.** Gemessen am 2026-08-06 von Worker-2, nachgemessen vom
Orchestrator.

## Die Frage hat sich beim Messen geändert

Ursprünglich: *Findet der Balance-Läufer über verschiedene Bosse hinweg
dieselbe Rotation?* Die erste Messung sagte ja — sieben Bosse, drei wörtlich
identische Kopfzeilen, der einzige Unterschied trennte nach Kampflänge.

**Dieser Befund trug nicht.** Der Sucher konnte gar keine boss-spezifische
Rotation finden: Die APL kennt Ressource, DoTs, Abklingzeiten, Boss-Leben
und Zeit — aber weder Phase noch Zeitleiste, unterbrechbaren Zauber oder
Widerstand. Eine gleiche Antwort auf eine Frage, die man nicht stellen kann,
ist kein Ergebnis.

Die Frage wurde deshalb ersetzt durch: **Gibt es überhaupt etwas, worauf
eine Rotation reagieren könnte?**

## Das Ergebnis

| Mechanik | Wirkt auf die Zahl? | Ändert die Rotation? |
| --- | --- | --- |
| Phasen | nein | nein |
| Zeitleiste | nein | nein — *solange der Spieler überlebt* |
| Widerstände | **ja** | nein |
| **Todesgefahr** | **ja** | **ja** |

### Phasen sind eine Beschriftung

Alle fünf Phasen in drei Bossen tragen `atHpPercent` und `name`, sonst
nichts. Kein Zeitleisten-Eintrag hängt an einer Phase.

Ein Versuch, `boss.phase` in die Sprache aufzunehmen, brachte an einem Boss
+0,46 %. **Der Gewinn kam nicht aus der Phase:** Weil sie an `atHpPercent`
hängt, ist `boss.phase < 1` faktisch `boss.hp_pct > X` — und das gab es
schon. Kein neuer Freiheitsgrad, eine Umbenennung.

### Die Zeitleiste trifft den Spieler, nicht die Rechnung

`test-vessel` (12238) und `test-metronome` (40854) liefern mit und ohne
Zeitleiste **exakt dieselbe** Zahl. Auch bei `test-vessel`, das 2000
Zeitleisten-Schaden gegen 8000 Boss-Leben trägt.

Ein `strike` kostet Lebenspunkte. Er ändert nichts daran, welche
Reihenfolge am meisten Schaden austeilt — solange der Spieler überlebt.

### Widerstände wirken, entscheiden aber nichts

`test-resistant` 30706 gegen 40854 neutral (Faktor 0,7516 bei Datenwert
0,75), `test-vulnerable` 51133 (1,2516 bei 1,25). Die Zahl folgt den Daten
auf drei Stellen.

Die Rotation bleibt gleich, und das ist kein Widerspruch: Ein Widerstand
skaliert **jeden** Zauber einer Schule gleich stark. Er könnte die Auswahl
erst ändern, wenn ein Spieler Zauber aus **mehreren** Schulen trüge.

### Todesgefahr ändert die Rotation

`test-onslaught` ist der einzige Boss, der töten kann — 6500 roh, 5200 nach
WARD, gegen 5000 Leben.

    MIT Zeitleiste                    OHNE (nur `timeline: []`)
    smoulder / searing / pyre         smoulder / pyre / searing
    temper                            kindle
    bellows / kindle
    7522 Schaden                      7882 Schaden

**`temper` kommt in die Rotation** — ein Zauber, der keinen Schaden macht.
Der Sucher nimmt 4,6 % weniger Schaden in Kauf, um zu überleben. Entfernt
man die Zeitleiste, verschwindet er sofort.

Das ist wörtlich die Abwägung aus GAME-DESIGN: *„Überleben kostet Schaden
und Zeit."*

### Und es ist eine Wahl zwischen zwei Wegen

`quench` (unterbrechen) steht im Suchraum und wird **verworfen**:

    quench   cd  5000   drei GCDs für drei Zauber
    temper   cd 15000   ein GCD, schluckt den Schaden

Einmal schlucken ist billiger als dreimal verhindern.

**Offen — Balance, nicht Code:** ob das richtig ist oder ob `quench` zu
teuer beziffert ist. Es ist die erste Stelle, an der eine Balance-Zahl eine
spielerische Entscheidung kippt.

## Was das für das Kernrisiko heißt

GAME-DESIGN 20.3 warnt, das Spiel könne nach zwanzig Stunden nur noch
Ausführung sein. Der Befund dazu ist zweiteilig:

**Tiefe existiert** — aber sie hängt heute an **einem** Boss von acht, und
zwar an der Todesgefahr, nicht an Boss-Mechanik als solcher. Phasen,
Zeitleisten und Widerstände erzeugen keine.

**Was fehlt, ist nicht Mechanik, sondern Anlass.** Die Bauteile sind da; es
gibt nur fast nie einen Grund, sie zu benutzen.

## Zwei Messfehler, die zum Befund gehören

**Die Boss-Auswahl war nach dem Vorhandensein von `timeline` getroffen,
nicht nach dem Inhalt.** Dadurch fehlte `test-onslaught` — der einzige Boss,
bei dem etwas passiert — in der ersten Liste, und `test-caster` galt
fälschlich als Boss mit unterbrechbaren Zaubern (er trägt drei `strike`).

**Eine fehlende Messung sah aus wie eine gemessene Null.** `.get(name, 0)`
lieferte für einen Boss, der gar nicht im Lauf war, den Wert 0 — in einer
Tabelle neben echten Zahlen. Seither steht in jeder Messung
`assert set(mit) == set(ohne)` und eine Zustandsprüfung **vor** jedem Lauf.
