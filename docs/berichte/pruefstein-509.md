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
    gefunden    7522                  gefunden    7882
    step1-good  2400                  step1-good  5960

**`temper` kommt in die Rotation** — ein Zauber, der keinen Schaden macht.
Entfernt man die Zeitleiste, verschwindet er sofort.

### Die zweite Spalte dreht die Abwägung

`step1-good` ist die Referenz-Handschrift, und **sie kennt kein `temper`**.
Mit Todesgefahr fällt sie von 5960 auf 2400 — sie stirbt.

    temper kostet gegenüber dem Maximum         4,6 %   (7522 gegen 7882)
    Wer temper weglässt und stirbt, verliert    68 %    (2400 gegen 7522)

Die 4,6 % sind der Preis gegenüber einer Welt **ohne** Todesgefahr, also
gegenüber etwas, das es an diesem Boss nicht gibt. Was der Spieler erlebt,
ist die andere Zahl: **dreifacher Schaden dafür, dass er einen GCD abgibt.**

GAME-DESIGN sagt *„Überleben kostet Schaden und Zeit."* Gemessen kostet es
4,6 % und bringt 210 %. Das ist keine Abwägung mehr, sondern eine klare
Entscheidung — **und das ist selbst ein Befund für die Balance.**

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

## Was daraus folgt

Gemessen, nicht empfohlen:

**Ein Boss ohne Todesgefahr erzeugt keine Rotations-Entscheidung.** Sieben
von acht Bössen haben keine — bei ihnen ist die beste Rotation dieselbe wie
gegen einen Boss ganz ohne Mechanik.

Der Zusammenhang gilt in beide Richtungen und ist an derselben Datei
belegt: `test-onslaught` mit Zeitleiste erzeugt eine Entscheidung, ohne
Zeitleiste keine. Es ist nicht die Mechanik, die sie erzeugt, sondern der
Umstand, dass sie den Spieler töten kann.

**Daraus folgt:** Jede weitere Boss-Mechanik bleibt folgenlos, solange der
Boss den Spieler nicht wirklich bedroht — so gut sie gebaut ist. Phasen,
Zeitleisten und Widerstände sind vollständig verdrahtet, werden angezeigt,
gehen in die Prüfsumme ein und haben Prüffälle. Sie ändern trotzdem nichts.

### Das ist kein Versäumnis, sondern ein Hinweis

GAME-DESIGN sieht Boss-Schaden ausdrücklich vor — *„ab Mittelspiel eigene
Aktionen"*, *„Boss-Schaden: Ja, später"*. Es fehlt also keine Entscheidung,
sondern die Umsetzung an sieben von acht **Prüf**-Bössen.

Prüf-Bosse sind Werkzeug, kein Spielinhalt. Der Befund sagt deshalb nichts
darüber, ob am Spiel etwas falsch ist — er sagt, worauf es bei den echten
Bössen ankommt, damit die vorhandene Mechanik zur Wirkung kommt.

Wie hart Bosse treffen sollen, ist eine Design-Entscheidung. Der
Zusammenhang zwischen Bedrohung und Rotations-Tiefe ist gemessen.

## Was NICHT gemessen ist

**Der Widerstands-Fall ist ungeprüft, nicht widerlegt.** Ein Widerstand
könnte die Auswahl ändern, sobald ein Spieler Zauber aus mehreren Schulen
gleichzeitig trägt. Ein solches Kit gibt es heute nicht, also konnte der
Fall nicht entstehen.

**Ob ein Boss existiert, bei dem Unterbrechen gewinnt.** Gemessen ist nur
`test-onslaught`, wo Schlucken billiger ist. Ein Boss mit weiter
auseinanderliegenden Zaubern könnte anders ausfallen.

## Die Messung wiederholen

    # Boss-Liste in sim/examples/rotation-search.rs auf die zu messenden
    # Bosse setzen, dann:
    cargo remote -c -- run --release --example rotation-search

    # Für eine Mechanik-Messung: das Feld in data/bosses/<boss>.json leeren
    # (timeline: [], resistances: {}), Zustand VOR dem Lauf prüfen,
    # danach `git checkout data/bosses/` und erneut prüfen.

**Vor jedem Lauf den Zustand prüfen, nicht danach** — und die Auswertung
muss `assert set(mit) == set(ohne)` enthalten. Warum, steht unten.

## Zwei Messfehler, die zum Befund gehören

**Die Boss-Auswahl war nach dem Vorhandensein von `timeline` getroffen,
nicht nach dem Inhalt.** Dadurch fehlte `test-onslaught` — der einzige Boss,
bei dem etwas passiert — in der ersten Liste, und `test-caster` galt
fälschlich als Boss mit unterbrechbaren Zaubern (er trägt drei `strike`).

**Eine fehlende Messung sah aus wie eine gemessene Null.** `.get(name, 0)`
lieferte für einen Boss, der gar nicht im Lauf war, den Wert 0 — in einer
Tabelle neben echten Zahlen. Seither steht in jeder Messung
`assert set(mit) == set(ohne)` und eine Zustandsprüfung **vor** jedem Lauf.
