# Das Ausrüstungs-Raster — 20 Einträge, und zwei Fehler auf dem Weg dahin

> **Stand 2026-08-10: eine Frage ist entschieden, eine bleibt offen.**
>
> | Frage                              | Ergebnis                                                        |
> | ---------------------------------- | --------------------------------------------------------------- |
> | Darf `gearEffects` ein Affix sein? | **NEIN** — entschieden vom Auftraggeber, Begründung unten       |
> | Bleibt `active` in `POOL_NAMES`?   | **offen** — der stille Verlust ist unabhängig davon geschlossen |
>
> Der Rest ist Vorschlag, keine Festlegung. Der Orchestrator legt vor.

> ### ⚠ Bestandszahlen sind ein Datum, keine Wahrheit
>
> **Vor jeder Planung nach diesem Dokument den Ist-Stand messen:**
>
> ```bash
> # Wie viele Einträge trägt der Katalog, und auf welchen Plätzen?
> python3 -c "import json;d=json.load(open('data/items.json'));\
> i=[x for x in d.get('items',[]) if isinstance(x,dict)];\
> print(len(i),'Einträge, Plätze',sorted({x['slot'] for x in i}))"
>
> # Wie viele Plätze kennt der Code?
> grep -oP 'SLOT_COUNT = \K\d+' src/gear/slots.ts
>
> # Welche Affix-Arten lässt der Katalog zu?
> grep -A 8 'POOL_NAMES: readonly' src/gear/pools.ts
> ```
>
> **Stand 2026-08-10**, gemessen an `main`: **1 Eintrag** (`test-relic`), auf Platz **0**.
> **10 Plätze** im Code. **6 zugelassene Affix-Arten**, davon 5 mit Wirkung.
>
> **Der zweite Befehl ist mit Absicht ein Ablesen und kein Zählen.** Mein erster Versuch war
> `grep -c 'export const SLOT_[A-Z]* ='` und meldete **11** — er zählte `SLOT_COUNT` mit. Eine
> Zeilenzahl misst die Schreibweise, nicht die Sache; deshalb steht hier die Konstante, die das
> Programm selbst benutzt.

**Alle Zahlen sind Platzhalter.** Sie stehen da, damit der Balance-Läufer etwas zu messen hat —
nicht als Vorschlag.

---

## Die kurze Fassung

**Die Maschine ist fertig, der Katalog ist leer, und die Wahl ist baubar — aber nicht dort, wo ich
sie zuerst gesucht habe.**

| Woraus die Wahl **nicht** besteht       | Woraus sie besteht                                    |
| --------------------------------------- | ----------------------------------------------------- |
| der **Reihenfolge** der `affixKinds`    | **welche** der fünf wirksamen Arten ein Eintrag nennt |
| der Qualität (wird beim Fall gewürfelt) | dem Platz                                             |

Vorgeschlagen werden **20 Einträge**: zehn Plätze mal zwei Ausrichtungen. Es waren einmal drei —
die dritte setzte auf `gearEffects`, und das ist seit dem 2026-08-10 kein Affix mehr. Dazu zwei
Befunde, die beim Messen aufgefallen sind und beide ein eigenes Vorhaben sind.

---

## Warum dieses Raster anders aussieht als das Zauber-Raster

Beim Zauber-Raster war die Frage „welche Mechanik fehlt". Hier ist sie es **nicht**. Gemessen bei
der Zählung zu Kapitel 13 (Nachricht 6061):

| Was              | Stand                                                            |
| ---------------- | ---------------------------------------------------------------- |
| Die sieben Werte | alle sieben auf **beiden** Seiten verdrahtet, mit Wirkstelle     |
| Die zehn Plätze  | alle zehn im Code, in der Reihenfolge des Designs                |
| Prüfsummen       | keine neue Zeile nötig — je Wert einzeln durch Mutation gemessen |
| Der Katalog      | **ein** Eintrag                                                  |

Es ist ein Katalog, der in eine fertige Maschine gefüllt wird. **Die Maschine bestimmt aber, wie
viel Vielfalt ein Katalog tragen kann** — und das musste gemessen werden, nicht gelesen.

---

## Was gemessen wurde, und was dabei zweimal umfiel

### Messung 1: Das Budget verteilt gleichmäßig

```text
Q0 common     slots=1  budget= 620  ->  [620]
Q1 rare       slots=2  budget= 887  ->  [444, 443]
Q2 epic       slots=3  budget=1219  ->  [407, 406, 406]
Q3 legendary  slots=3  budget=1219  ->  [407, 406, 406]
```

`shareOfSlot = floor((budget + slots - 1 - index) / slots)` (`src/gear/budget.ts`). Der Unterschied
zwischen erstem und letztem Platz ist **ein Promille von 407** — Rundungsrest, keine Betonung.

> **Damit fiel mein erster Entwurf.** Er baute dreißig Einträge auf der _Reihenfolge_ auf: je Platz
> eine CATALYSIS-, eine PURITY- und eine YIELD-betonte Fassung.
> `['catalysis','purity','yield']` gegen `['yield','purity','catalysis']` ist gemessen
> **407/406/406 gegen 407/406/406** — dieselbe Sache mit vertauschten Etiketten.

Auch der naheliegende Ausweg trägt nicht: Wer **weniger** Arten nennt, als die Qualität Plätze
gibt, verliert den Rest ersatzlos.

```text
Epic-Budget gesamt:                 1219
  Eintrag mit 1 affixKind bekommt:   407
  Eintrag mit 3 affixKinds bekommt: 1219
```

Ein „reiner Tempo-Fokus" wäre nicht spezialisiert, sondern um zwei Drittel schwächer.

### Messung 2: Es gibt fünf wirksame Affix-Arten, nicht drei

**Damit fiel mein zweiter Entwurf**, der aus Messung 1 geschlossen hatte, es gebe überhaupt keine
Wahl und zehn Einträge genügten.

`catalog.ts` prüft `POOL_NAMES.includes(kind)` und lässt damit **alle sechs Topf-Namen** zu. Ein
Eintrag mit `["gearEffects","essence","active"]` lädt anstandslos und ergibt:

```text
{"catalysis":0,"purity":0,"yieldBonus":0,"essence":473,"vessel":67,"ward":67,
 "gearEffects":407,"active":406}
```

- **`essence` wirkt** — 473 = 407 aus dem Affix plus 66 aus dem Grundwert. Topf 1.
- **`gearEffects` wirkt** — 407. **Topf 4 ist über einen Affix erreichbar**, ganz ohne Sätze.
- **`active` erzeugt ein Feld, das es nicht gibt** — siehe Befund 1 unten.

**Die Wahl besteht also aus der Auswahl, nicht aus der Sortierung.** Bei fünf wirksamen Arten und
drei Plätzen (Epic) gibt es 10 echte Kombinationen — und sie unterscheiden sich wirklich, weil sie
verschiedene Größen füllen.

---

## Die Zahl: 20 — zehn Plätze mal zwei Ausrichtungen

Die wirksamen Arten zerfallen in zwei Klassen, und daraus folgt die Achse:

| Klasse               | Arten                          | Wirkung                                                      |
| -------------------- | ------------------------------ | ------------------------------------------------------------ |
| **Sonderwerte**      | `catalysis`, `purity`, `yield` | je ein eigener Weg: Tempo, Krit-Chance, Krit-Stärke          |
| **Schadensfaktoren** | `essence`                      | multipliziert den Schaden direkt (Topf 1)                    |
| ~~Topf 4~~           | ~~`gearEffects`~~              | **kein Affix** — entschieden am 2026-08-10, Begründung unten |

**Die Ausrichtung ist, ob ein Platz an den Schadensfaktor geht oder an die Sonderwerte.** Das ist
eine echte Abwägung: Sonderwerte wirken über Umwege (schneller wirken, öfter kritisch treffen),
der Faktor wirkt unmittelbar auf jede Zahl.

| Ausrichtung  | `affixKinds`                 | Was sie ist                               |
| ------------ | ---------------------------- | ----------------------------------------- |
| **`-mix`**   | `catalysis, purity, yield`   | drei Sonderwerte — die Mischung aus 13.1b |
| **`-drive`** | `essence, purity, catalysis` | ein Faktor, zwei Sonderwerte              |

Je Platz eine von beiden → **20 Einträge**. Kein Platz bleibt leer, und auf jedem Platz gibt es
etwas zu entscheiden.

> **Die Auswahl der Ausrichtungen ist meine Auslegung**, kein Design-Satz. Was gemessen ist: dass
> die Arten wirken und sich unterscheiden. Dass genau **diese beiden** Zusammenstellungen die
> interessanten sind, ist eine Setzung — wer sie anders schneidet, ändert die Tabelle unten, nicht
> die Zahl 20.

### Die Ausrichtungen ergeben messbar verschiedene Kämpfe

Derselbe Lauf (20 × KINDLE, Startwert 0, `test-crucible-trial`), nur die `affixKinds` getauscht:

| Ausrichtung        | getragene Werte                            |  Schaden |
| ------------------ | ------------------------------------------ | -------: |
| `-mix`             | `catalysis 407, purity 406, yield 406`     | **2812** |
| `-drive`           | `essence 474, purity 406, catalysis 406`   | **3458** |
| ~~`-core`~~ (raus) | `essence 474, gearEffects 406, purity 406` | **4868** |

**Damit ist belegt, worum es hier geht:** Die Auswahl der Arten verändert den Kampf, die
Reihenfolge nicht. Ein Katalog mit diesen Ausrichtungen bietet eine Wahl.

**Die dritte Zeile steht noch da, obwohl `-core` gestrichen ist** — sie ist der Beleg für die
Entscheidung, nicht für den Vorschlag. Zwei Faktoren schlagen jede Mischung aus Sonderwerten, und
zwar deutlich. Genau das ist die Diablo-Lehre, die 13.1b als **Warnung** führt: „Was multiplikativ
ist, wird gejagt."

> ### ⚠ Diese Zahlen sagen NICHTS über Balance
>
> Der Lauf wirkt zwanzigmal KINDLE **im festen Takt von 1500 ms** — und KINDLEs Abklingzeit ist
> genau 1500 ms. CATALYSIS verkürzt Abklingzeiten; wer trotzdem 1500 ms wartet, verschenkt jede
> Verkürzung. **Der Lauf benachteiligt also strukturell die Ausrichtung, die am meisten Tempo
> trägt** — ausgerechnet `-mix`, die mit der niedrigsten Zahl.
>
> Die Rangfolge oben ist damit ein Artefakt des Messaufbaus, keine Aussage über Stärke. Wer sie
> als Balance liest, zieht genau den falschen Schluss.
>
> Dieselbe Falle hat bei `DISTIL` schon einmal zugeschlagen (`tests/unit/buff-groessen-540.test.ts`
> dokumentiert sie): Ein Verstärker wurde an KINDLE gemessen, dessen Abklingzeit schon am Boden
> war. **Was die Ausrichtungen wirklich wert sind, misst der Balance-Läufer** — mit einer
> Rotation, die wirkt, sobald etwas bereit ist.

### Gruppe A — die 20 Einträge

`-mix` ist auf jedem Platz identisch aufgebaut, `-drive` ebenso; die Vielfalt liegt in der
Ausrichtung, nicht in der Variation je Platz.

| Platz          | `…-mix`                    | `…-drive`                    |
| -------------- | -------------------------- | ---------------------------- |
| **Focus**      | `catalysis, purity, yield` | `essence, purity, catalysis` |
| **Catalyst**   | dito                       | dito                         |
| **Sigil**      | dito                       | dito                         |
| **Relic**      | dito                       | dito                         |
| **Grimoire**   | dito                       | dito                         |
| **Shard**      | dito                       | dito                         |
| **Talisman**   | dito                       | dito                         |
| **Phylactery** | dito                       | dito                         |
| **Anchor**     | dito                       | dito                         |
| **Prism**      | dito                       | dito                         |

Kennungen nach dem Muster `focus-mix`, `focus-drive`.

**Die drei Grundwerte stehen bei keinem dabei, und das ist richtig.** ESSENCE, VESSEL und WARD
trägt **jeder** Gegenstand (13.3); `wornFrom` addiert sie aus `basePermille(itemLevel, …)`, ohne
dass sie im Eintrag stehen. Ein `essence`-Affix kommt **zusätzlich** dazu (gemessen: 473 = 407 + 66)
— das ist kein Doppelzählen, sondern Topf 1, der laut 13.1b „alle Quellen des Grundwerts" additiv
sammelt.

**Stufe und Qualität stehen mit Absicht nicht in der Tabelle.** Sie gehören zum Eintrag, werden
aber beim Fall neu gewürfelt (`qualityOfRoll`, `itemLevelOf` in `src/loot/rates.ts`) — und sind
damit Balance-Zahlen, keine Katalog-Entscheidung. **Unter dem Vorbehalt von Befund 2.**

---

## ENTSCHIEDEN am 2026-08-10: `gearEffects` ist kein Affix

Die Frage lautete: Es **geht** und es **wirkt** — gemessen, ein Eintrag damit füllt Topf 4 ohne
jeden Satz. Darf man es?

**Der Auftraggeber sagt nein**, und die Begründung steht in 13.1b selbst:

> Was multiplikativ ist, wird gejagt — und alles andere wird liegen gelassen.

Ein multiplikativer Faktor über **gewöhnliche** Beute entwertet jeden anderen Affix. Das ist genau
das Muster, das Diablo 4 in Saison 2 zurückdrehen musste; 13.1b führt es als **Warnung**, nicht als
Vorbild. Die Messung oben (`-core` mit 4868 gegen `-mix` mit 2812) zeigt die Größenordnung, um die
es dabei geht — auch wenn die Zahl selbst aus einem verzerrten Aufbau stammt.

**Topf 4 bleibt legendären Effekten und Satz-Boni vorbehalten.** Die Ausrichtung `-core` entfällt,
es bleiben 20 Einträge; die übrige Rechnung ändert sich nicht.

> **Für die Umsetzung heißt das:** `gearEffects` darf im Katalog nicht vorkommen. Eine Sperre im
> Code gibt es dafür **nicht** — die Art ist technisch weiterhin gültig, weil sie ein Feld füllt.
> Wer das absichern will, braucht eine eigene Prüfung; sie steht nicht in diesem Raster, weil sie
> eine Entscheidung über den Katalog ist und nicht über den Katalog-Inhalt.

---

## Gruppe B — heute NICHT baubar

**Nicht der Topf-4-Faktor ist gesperrt, sondern seine beiden Design-Quellen.** Das ist die
Korrektur gegenüber meinem zweiten Entwurf, der den ganzen Topf für unerreichbar hielt.

| Was                                      | Design | Warum gesperrt                                                                  |
| ---------------------------------------- | ------ | ------------------------------------------------------------------------------- |
| **Legendäre Sondereffekte**              | 13.3   | „Echter Sondereffekt, der die Rotation verändert" — es gibt keinen Träger dafür |
| **Satz-Boni**, 6 Sätze × 2/4/6           | 13.5   | `gearEffectsPermille` in `src/gear/sources.ts` ruft **niemand**                 |
| **Rotations-Effekte** höherer Satzstufen | 13.5   | `activeRotationEffects()` und `valueShareOf()` in `sets.ts` ruft **niemand**    |

**Ein Eintrag der Qualität `legendary` wäre heute ein Epic mit anderem Namen** — gemessen: `Q3`
bekommt dieselben `slots=3` und dasselbe `budget=1219` wie `Q2`. Deshalb steht in Gruppe A **kein**
legendärer Eintrag: nicht aus Vorsicht, sondern weil das Wort sonst etwas verspräche, was der Code
nicht einlöst.

**Der Balance-Vergleich aus 13.5 (#118) kann deshalb weiterhin nicht laufen**, auch wenn Topf 4
über Affixe erreichbar ist: Die **Satz-Seite** kommt nicht an.

---

## Befund 1: `active` war ein stiller Wertverlust — GESCHLOSSEN

`POOL_NAMES` lässt `active` als Affix zu. `WornValues` hat aber **kein Feld dafür**. Gemessen:

```text
{… "gearEffects":407, "active":406}
                       ^^^^^^^^^^^^ ein Feld, das der Kampf nicht kennt
```

`wornFrom` schrieb über `summe[feld]` in einen frei benannten Schlüssel. Ein Eintrag mit `active`
lud ohne Fehler, bestand jede Prüfung — und **406 von 1219 Promille verfielen**, ohne dass irgendwo
etwas auffiel. Der Gegenstand sah vollwertig aus und war ein Drittel schwächer.

**Geschlossen am 2026-08-10** (`feldFuerAffixArt` in `worn.ts`, `pruefeAffixArten` in `catalog.ts`):
Ein Affix-Name, der keinen Wert füllt, bricht jetzt den Ladeversuch — mit dem Namen und dem Grund
in der Meldung. Gemessen: Ein Katalog mit `active` lässt den Import abbrechen, bevor ein einziger
Test anläuft.

> **Der Auftraggeber hat diese Sperre der Design-Frage ausdrücklich VORGEORDNET.** Ob `active`
> überhaupt aus Ausrüstung kommen soll, ist noch offen — 13.1b nennt dort Overdrive, Synergien und
> Zersetzungs-Stapel, also Kampf-Quellen. Die Sperre gilt unabhängig davon, weil sie **jeden**
> künftigen Tippfehler in einer Affix-Liste trifft und nicht nur diesen einen Namen.
>
> **Offen bleibt:** ob `active` aus `POOL_NAMES` fliegt oder ein Feld bekommt. Bis dahin bleibt es
> drin und fällt beim Ladeversuch auf. Steht als offener Punkt in
> [`ENTSCHEIDUNGEN-OFFEN.md`](ENTSCHEIDUNGEN-OFFEN.md).

**Die Ursache war eine doppelte Wahrheit**, nicht ein falscher Wert: Die Abbildung von Art auf Feld
stand faktisch zweimal — einmal als Prüfung gegen `POOL_NAMES`, einmal als Ternary beim Rechnen.
Dass die beiden auseinanderliefen, war der Fehler.

## Befund 2: Der Fund verliert seine Stufe und seine Qualität

`rollLoot` liefert einen Fund mit **gewürfelter** Stufe und Qualität. Verbucht wird er in
`recordRun` (`src/analysis/run-recorder.ts`) so:

```ts
[...state.items, { itemId: drop.itemId, upgradeSteps: 0, equippedSlot: null }];
```

`drop.itemLevel` und `drop.quality` werden **nicht übernommen** — `OwnedItem` hat keine Felder
dafür. `wornFrom` liest die Stufe danach aus dem **Katalog**. Gemessen:

```text
FUND  aus Instanz 1        {"itemId":"test-relic","itemLevel":1, "quality":0}
FUND  aus Instanz 14 myth. {"itemId":"test-relic","itemLevel":85,"quality":1}

WERTE tief  {"catalysis":407,"purity":406,"yieldBonus":406,"essence":67,"vessel":67,"ward":67}
WERTE hoch  {"catalysis":407,"purity":406,"yieldBonus":406,"essence":67,"vessel":67,"ward":67}
```

**Identisch.** Ein Fund von Stufe 85 ist genau so stark wie einer von Stufe 1.

Damit kommt 13.4 („Zwei getrennte Achsen: Qualität und Gegenstandsstufe laufen unabhängig. Ein
episches Stück aus einer frühen Instanz ist schwächer als ein seltenes aus dem Endgame — **alter
Inhalt entwertet sich von selbst**") im Spiel nicht an. Beide Achsen werden gewürfelt und verworfen.

**Für dieses Raster heißt das:** Die Begründung, warum Qualität und Stufe keine Katalog-Achsen sind,
steht und fällt damit. Wird der Befund behoben, trägt sie.

**Eigenes Vorhaben**, kein Teil des Katalogs: Es berührt `OwnedItem`, das Spielstand-Format und
`wornFrom`, also auch die Fassung des Spielstands.

## Befund 3: `boundToBoss` ist gebaut und wartet auf einen Träger

`mayDropAt(item.boundToBoss, bossId)` steht in `src/loot/roll.ts` und wird bei jedem Fund geprüft.
Der Kommentar dort sagt: „Heute trägt kein Eintrag eine Bindung; die Prüfung steht trotzdem, weil
sie sonst beim ersten gebundenen Stück vergessen würde."

**Der Katalog kann davon Gebrauch machen, ohne dass etwas gebaut werden muss.** Ich schlage es hier
**nicht** vor, weil die Bosse noch nicht stehen — aber wer die erste Instanz baut, sollte wissen,
dass die Bindung schon da ist.

---

## Was dieses Raster NICHT entscheidet

- **Die Basiswerte selbst.** Wie viel ESSENCE ein Gegenstand auf Stufe 1 trägt, ist laut 13.3
  ausdrücklich eine Balance-Zahl, die gemessen und nicht gesetzt wird.
- **Die Anzeige-Namen.** `focus-mix` und so weiter sind Kennungen. Spieltexte sind englisch
  (Design-Gesetz 5); die Namen gehören zum Katalog-Bau, nicht zum Raster.
- **Wie viele Einträge je Instanz fallen dürfen.** Das ist 13.4 („Obergrenze durch Inhalt") und
  hängt an den Instanzen, die es noch nicht gibt.
- **Ob es Sätze im Aufstieg gibt.** 13.5 sagt nein, und daran ändert dieses Raster nichts.
