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

Ursprünglich: _Findet der Balance-Läufer über verschiedene Bosse hinweg
dieselbe Rotation?_ Die erste Messung sagte ja — sieben Bosse, drei wörtlich
identische Kopfzeilen, der einzige Unterschied trennte nach Kampflänge.

**Dieser Befund trug nicht.** Der Sucher konnte gar keine boss-spezifische
Rotation finden: Die APL kennt Ressource, DoTs, Abklingzeiten, Boss-Leben
und Zeit — aber weder Phase noch Zeitleiste, unterbrechbaren Zauber oder
Widerstand. Eine gleiche Antwort auf eine Frage, die man nicht stellen kann,
ist kein Ergebnis.

Die Frage wurde deshalb ersetzt durch: **Gibt es überhaupt etwas, worauf
eine Rotation reagieren könnte?**

## Das Ergebnis

| Mechanik        | Wirkt auf die Zahl? | Ändert die Rotation?                  |
| --------------- | ------------------- | ------------------------------------- |
| Phasen          | nein                | nein                                  |
| Zeitleiste      | nein                | nein — _solange der Spieler überlebt_ |
| Widerstände     | **ja**              | nein                                  |
| **Todesgefahr** | **ja**              | **ja**                                |

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

GAME-DESIGN sagt _„Überleben kostet Schaden und Zeit."_ Gemessen kostet es
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

GAME-DESIGN sieht Boss-Schaden ausdrücklich vor — _„ab Mittelspiel eigene
Aktionen"_, _„Boss-Schaden: Ja, später"_. Es fehlt also keine Entscheidung,
sondern die Umsetzung an sieben von acht **Prüf**-Bössen.

Prüf-Bosse sind Werkzeug, kein Spielinhalt. Der Befund sagt deshalb nichts
darüber, ob am Spiel etwas falsch ist — er sagt, worauf es bei den echten
Bössen ankommt, damit die vorhandene Mechanik zur Wirkung kommt.

Wie hart Bosse treffen sollen, ist eine Design-Entscheidung. Der
Zusammenhang zwischen Bedrohung und Rotations-Tiefe ist gemessen.

## Nachtrag: der Kreuzvergleich selbst (AC2, AC3, AC4)

Gemessen am 2026-08-13 von Worker-3, mit dem Werkzeug aus PR #823. Der
Abschnitt oben beantwortet die _ersetzte_ Frage; hier steht die
**ursprüngliche**, jetzt mit dem Werkzeug, das sie messen kann.

Fünf Bosse über vier Achsen: `first-crucible` (ohne Mechanik),
`test-onslaught` (Zeitleiste **mit Todesgefahr**), `test-metronome`
(Zeitleiste, 300 s), `test-gatekeeper` (Phasen), `test-resistant`
(Widerstände).

### AC2 — wie ähnlich sind die gefundenen Listen?

| Paarung                              | gleiche Zeilen     |
| ------------------------------------ | ------------------ |
| `first-crucible` / `test-gatekeeper` | 6 von 6 (100 %)    |
| `test-metronome` / `test-resistant`  | 6 von 6 (100 %)    |
| `first-crucible` / `test-metronome`  | 5 von 6 (83 %)     |
| `first-crucible` / `test-onslaught`  | **3 von 6 (50 %)** |
| `test-onslaught` / `test-gatekeeper` | **3 von 6 (50 %)** |

Zwei Paare sind **wörtlich identisch**. Der einzige Boss, der aus der Reihe
fällt, ist `test-onslaught`.

### AC3 — was kostet die fremde Rotation?

Der Kern des Issues. Zeile = mitgebrachte Liste, Spalte = gespielter Boss:

```text
                        first-crucible   test-onslaught   test-metronome   test-gatekeeper   test-resistant
  von first-crucible      9364 (+0,0 %)    3720 (+58,7 %)   47848 (+1,7 %)    18990 (+0,0 %)   37445 (+1,2 %)
  von test-onslaught     7514 (+19,8 %)     9014 (+0,0 %)  39029 (+19,8 %)   15311 (+19,4 %)  30958 (+18,3 %)
  von test-metronome       9275 (+1,0 %)   3457 (+61,6 %)   48672 (+0,0 %)    18802 (+1,0 %)   37902 (+0,0 %)
  von test-gatekeeper      9364 (+0,0 %)   3720 (+58,7 %)   47848 (+1,7 %)    18990 (+0,0 %)   37445 (+1,2 %)
  von test-resistant       9275 (+1,0 %)   3457 (+61,6 %)   48672 (+0,0 %)    18802 (+1,0 %)   37902 (+0,0 %)

  Fremde Paarungen        20
  Größter Verlust         61,6 %
  Mittlerer Verlust        1,7 %
  Paare über 10 %          8 von 20
  In JEDER teuren Paarung steckt: test-onslaught
```

**Die Antwort auf AC3 lautet: es kostet fast nichts — außer an einem Boss.**
Sechzehn der zwanzig Paarungen kosten unter 2 %; vier davon exakt 0,0 %.
Alle acht teuren Paarungen haben `test-onslaught` auf einer der beiden
Seiten, und zwar in beide Richtungen: seine Liste anderswo kostet 18–20 %,
eine fremde Liste bei ihm 58–62 %.

Das bestätigt den Befund oben aus einer völlig anderen Richtung. Dort wurde
Mechanik für Mechanik abgeschaltet; hier wurde nichts abgeschaltet, sondern
gemessen, was eine mitgebrachte Rotation kostet. **Beide Wege zeigen auf
dieselbe Größe: Todesgefahr.**

#### Ein Maximum ist keine Aussage — mein eigener Urteilssatz war falsch

Das Werkzeug urteilte zuerst am größten Verlust: 61,6 % → _„Tiefe ist
vorhanden."_ Das ist die falsche Lesart derselben Zahlen. Ein Maximum sagt,
dass es EINEN Fall gibt; über die Regel sagt es nichts. Seither steht die
Verteilung im Bericht, und das Urteil hängt am Mittelwert.

### AC4 — welche Zauber kommen in KEINER Rotation vor?

```text
  über alle 5 Bosse gewirkt    9 von 57
  in KEINER Rotation          48 von 57
```

Neun Zauber tragen alle fünf Rotationen: `pyre`, `searing`, `rot`,
`smoulder`, `bellows`, `kindle`, `cinder`, `riposte`, `scab`. Die letzten
drei kommen **ausschließlich** bei `test-onslaught` und `test-metronome` vor.

Worker-1 hatte an zwei Bossen zwölf von fünfzehn gemessen. Mit fünf Bossen
und 57 spielbaren Zaubern sind es 48 — Leitplanke 2 („jeder Zauber muss
irgendwo die beste Wahl sein") ist damit **nicht** erfüllt.

#### Die Zahl misst zum Teil die Suchlänge, nicht das Kit

Bei sechs Zeilen je Liste können über fünf Bosse höchstens dreißig Zauber
überhaupt vorkommen. Deshalb die Gegenprobe mit `--max-len 12`:

| Suchlänge | gewirkt | tot | mittlerer Verlust |
| --------- | ------- | --- | ----------------- |
| 6         | 9       | 48  | 1,7 %             |
| 12        | 16      | 41  | 19,2 %            |

Sieben Zauber mehr, sieben tote weniger. **Die Zahl 48 gehört also zur Länge
6, nicht zum Spiel** — und der Befund „Leitplanke 2 nicht erfüllt" bleibt
trotzdem stehen, weil auch 41 weit von null entfernt ist.

#### Und der Lauf mit Länge 12 fällt gar kein Urteil

Das ist der wichtigste Befund dieser Messung, und er hätte fast als
Ergebnis durchgehen können:

```text
  ACHTUNG: In 4 Paarungen ist die FREMDE Liste besser als die eigene.
    test-metronome schlägt an first-crucible dessen eigene Liste um 20,9 %
    test-metronome schlägt an test-onslaught dessen eigene Liste um 30,7 %
    test-metronome schlägt an test-gatekeeper dessen eigene Liste um 22,1 %
    test-metronome schlägt an test-resistant dessen eigene Liste um 37,3 %
  -> KEIN Urteil. Erst die Suche verbessern, dann erneut vergleichen.
```

Der ganze Kreuzvergleich rechnet gegen die je Boss **gefundene** Liste und
setzt damit voraus, dass sie dort die beste ist. Bei Länge 12 ist sie es
nicht: Die Greedy-Suche legt sich im größeren Raum früher falsch fest. Die
19,2 % oben sind deshalb **kein Beleg für Rotations-Tiefe**, sondern zum
großen Teil der Abstand der Suche zum Optimum.

Ohne diese Prüfung wäre die Meldung gewesen: _„Mit längeren Listen steigt der
mittlere Verlust von 1,7 auf 19,2 % — die Tiefe ist doch da."_ Das wäre aus
richtigen Zahlen der falsche Schluss gewesen.

Bei Länge 6 tritt der Fall nicht auf; dort gibt es keine negative Paarung.

#### Nachgetragen am 2026-08-13: die 19,2 % waren zu 100 % Suchschwäche

Der Nachlauf aus #558 nimmt jede an einem anderen Boss gefundene Liste als
Startpunkt und sucht von dort weiter — bis sich nichts mehr ändert.

```text
  Runde 1  first-crucible    9742 -> 11801 (+21,1 %) über test-onslaught
  Runde 1  test-onslaught    9014 -> 11781 (+30,7 %) über test-metronome
  Runde 1  test-gatekeeper  20058 -> 24494 (+22,1 %) über test-metronome
  Runde 1  test-resistant   38337 -> 53817 (+40,4 %) über test-onslaught
  Runde 2  first-crucible   11801 -> 12254  (+3,8 %) über test-resistant
  Runde 2  test-onslaught   11781 -> 13054 (+10,8 %) über test-resistant
  Runde 2  test-metronome   61115 -> 62507  (+2,3 %) über test-resistant
  Runde 3: keine Verbesserung mehr — Fixpunkt erreicht.
```

| bei Länge 12      | vorher | nach dem Nachlauf |
| ----------------- | ------ | ----------------- |
| ACHTUNG-Paarungen | 4      | **0**             |
| mittlerer Verlust | 19,2 % | **0,0 %**         |
| größter Verlust   | 66,3 % | 9,8 %             |

**Von den 19,2 % bleibt nichts übrig.** Was nach Rotations-Tiefe aussah, war
vollständig der Abstand der Suche zum Optimum. Der Befund „es kostet fast
nichts, überall dieselbe Liste zu spielen" gilt damit auch bei Länge 12 — er
war dort vorher nur verdeckt.

**Eine Runde reichte nicht**, und das gehört zum Befund: Der erste Entwurf
lief einmal durch, hob vier Bosse um 21–40 % — und die ACHTUNG-Zeile meldete
danach wieder vier Paarungen, nur andere. `test-resistant` war in derselben
Runde auf 53817 gehoben worden, und diese neue Liste stand keinem anderen
Boss mehr als Startpunkt zur Verfügung. Eine Verbesserung ist selbst ein
neuer Startpunkt.

**Der Wächter kann weiterhin anschlagen.** Das ist keine Behauptung, sondern
ein Schalter:

    --kein-nachlauf   ACHTUNG in 4 Paarungen, mittlerer Verlust 19,2 %
    ohne den Schalter keine ACHTUNG-Zeile, mittlerer Verlust 0,0 %

Die Schweige-Regel selbst ist unverändert (Schwelle weiter bei −0,5 %). Sie
wurde nicht entschärft, sondern ihr Anlass beseitigt.

**Bei Länge 6 findet der Nachlauf nichts** — Fixpunkt in Runde 1, keine Zahl
bewegt sich (61,6 / 1,7 % / 8 von 20 / 48 tote Zauber). Der Mangel existierte
dort wirklich nicht, und die AC4-Zahl 48 steht unverändert. Das ist zugleich
die Gegenprobe dazu, dass der Nachlauf nicht einfach Zahlen schönt: Wo nichts
zu holen ist, holt er nichts.

**Was der Nachlauf nicht kann:** Er findet nichts, was an keinem Boss
gefunden wurde. Ein Paar, dessen beide Hälften einzeln verlieren, bleibt
unsichtbar — das ist die Paar-Blindheit aus #558 und hier **nicht** erledigt.

## Was NICHT gemessen ist

**Alle Zahlen dieses Nachtrags stammen von PRÜF-Bossen.** Sieben von acht
können den Spieler nicht töten; an ihnen entsteht keine Rotations-Entscheidung
(siehe „Todesgefahr ändert die Rotation" oben). Das ist eine Eigenschaft der
Prüfwerkzeuge, nicht des Spiels — der Kreuzvergleich sagt deshalb nichts
darüber, wie tief die Rotation an einem echten Boss wäre, der ernsthaft
bedroht.

**Der Widerstands-Fall ist ungeprüft, nicht widerlegt.** Ein Widerstand
könnte die Auswahl ändern, sobald ein Spieler Zauber aus mehreren Schulen
gleichzeitig trägt. Ein solches Kit gibt es heute nicht, also konnte der
Fall nicht entstehen.

**Ob ein Boss existiert, bei dem Unterbrechen gewinnt.** Gemessen ist nur
`test-onslaught`, wo Schlucken billiger ist. Ein Boss mit weiter
auseinanderliegenden Zaubern könnte anders ausfallen.

## Die Messung wiederholen

Der Kreuzvergleich (AC2, AC3, AC4) — **beide Läufe, denn der zweite ist die
Gegenprobe zum ersten:**

    cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
      --example rotation-search -- \
      --bosses first-crucible,test-onslaught,test-metronome,test-gatekeeper,test-resistant

    cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
      --example rotation-search -- \
      --bosses first-crucible,test-onslaught,test-metronome,test-gatekeeper,test-resistant \
      --max-len 12

Beide Schalter sind seit #509 Argumente. **Vorher stand die Boss-Liste als
Konstante im Quelltext** — wer sie zum Messen änderte, maß mit einem Werkzeug,
das nach der Messung anders aussah als davor, und der Bericht nannte einen
Befehl, den niemand nachfahren konnte.

Für eine Mechanik-Messung (der Abschnitt oben):

    # Das Feld in data/bosses/<boss>.json leeren (timeline: [],
    # resistances: {}), Zustand VOR dem Lauf prüfen, danach
    # `git checkout data/bosses/` und erneut prüfen.

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
