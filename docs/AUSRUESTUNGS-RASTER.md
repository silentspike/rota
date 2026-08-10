# Das Ausrüstungs-Raster — 30 Einträge, und zwei Fehler auf dem Weg dahin

> **Noch nicht freigegeben.** Der Orchestrator legt dieses Dokument dem Auftraggeber vor. Bis dahin
> ist jede Zeile ein Vorschlag, keine Festlegung.

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

Vorgeschlagen werden **30 Einträge**: zehn Plätze mal drei Ausrichtungen. Dazu zwei Befunde, die
beim Messen aufgefallen sind und beide ein eigenes Vorhaben sind.

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

## Die Zahl: 30 — zehn Plätze mal drei Ausrichtungen

Die fünf wirksamen Arten zerfallen in zwei Klassen, und daraus folgt die Achse:

| Klasse               | Arten                          | Wirkung                                             |
| -------------------- | ------------------------------ | --------------------------------------------------- |
| **Sonderwerte**      | `catalysis`, `purity`, `yield` | je ein eigener Weg: Tempo, Krit-Chance, Krit-Stärke |
| **Schadensfaktoren** | `essence`, `gearEffects`       | multiplizieren den Schaden direkt (Topf 1, Topf 4)  |

**Die Ausrichtung ist, wie viele Plätze an die Schadensfaktoren gehen.** Das ist eine echte
Abwägung: Sonderwerte wirken über Umwege (schneller wirken, öfter kritisch treffen), Faktoren
wirken unmittelbar auf jede Zahl.

| Ausrichtung  | `affixKinds`                   | Was sie ist                               |
| ------------ | ------------------------------ | ----------------------------------------- |
| **`-mix`**   | `catalysis, purity, yield`     | drei Sonderwerte — die Mischung aus 13.1b |
| **`-drive`** | `essence, purity, catalysis`   | ein Faktor, zwei Sonderwerte              |
| **`-core`**  | `essence, gearEffects, purity` | zwei Faktoren, ein Sonderwert             |

Je Platz eine von jeder → **30 Einträge**. Kein Platz bleibt leer, und auf jedem Platz gibt es
etwas zu entscheiden.

> **Die Auswahl der Ausrichtungen ist meine Auslegung**, kein Design-Satz. Was gemessen ist: dass
> die fünf Arten wirken und sich unterscheiden. Dass genau **diese drei** Zusammenstellungen die
> interessanten sind, ist eine Setzung — wer sie anders schneidet, ändert die Tabelle unten, nicht
> die Zahl 30.

### Gruppe A — die 30 Einträge

`-mix` ist auf jedem Platz identisch aufgebaut, `-drive` und `-core` ebenso; die Vielfalt liegt in
der Ausrichtung, nicht in der Variation je Platz.

| Platz          | `…-mix`                    | `…-drive`                    | `…-core`                       |
| -------------- | -------------------------- | ---------------------------- | ------------------------------ |
| **Focus**      | `catalysis, purity, yield` | `essence, purity, catalysis` | `essence, gearEffects, purity` |
| **Catalyst**   | dito                       | dito                         | dito                           |
| **Sigil**      | dito                       | dito                         | dito                           |
| **Relic**      | dito                       | dito                         | dito                           |
| **Grimoire**   | dito                       | dito                         | dito                           |
| **Shard**      | dito                       | dito                         | dito                           |
| **Talisman**   | dito                       | dito                         | dito                           |
| **Phylactery** | dito                       | dito                         | dito                           |
| **Anchor**     | dito                       | dito                         | dito                           |
| **Prism**      | dito                       | dito                         | dito                           |

Kennungen nach dem Muster `focus-mix`, `focus-drive`, `focus-core`.

**Die drei Grundwerte stehen bei keinem dabei, und das ist richtig.** ESSENCE, VESSEL und WARD
trägt **jeder** Gegenstand (13.3); `wornFrom` addiert sie aus `basePermille(itemLevel, …)`, ohne
dass sie im Eintrag stehen. Ein `essence`-Affix kommt **zusätzlich** dazu (gemessen: 473 = 407 + 66)
— das ist kein Doppelzählen, sondern Topf 1, der laut 13.1b „alle Quellen des Grundwerts" additiv
sammelt.

**Stufe und Qualität stehen mit Absicht nicht in der Tabelle.** Sie gehören zum Eintrag, werden
aber beim Fall neu gewürfelt (`qualityOfRoll`, `itemLevelOf` in `src/loot/rates.ts`) — und sind
damit Balance-Zahlen, keine Katalog-Entscheidung. **Unter dem Vorbehalt von Befund 2.**

---

## Die eine Design-Frage, die ich nicht selbst entscheide

**Darf `gearEffects` ein Affix sein?**

Gemessen: Es geht, und es wirkt. Aber 13.1b beschreibt Topf 4 als „legendäre Effekte **und**
Satz-Boni gemeinsam" — ein gewöhnlicher Affix darauf ist eine **dritte** Quelle, die dort nicht
steht.

|                    | dafür                                              | dagegen                                                                                                                      |
| ------------------ | -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Wirkung            | Topf 4 kommt im Spiel an, ohne auf Sätze zu warten | 13.1b nennt nur zwei Quellen                                                                                                 |
| Balance            | gibt der Ausrichtung `-core` ihren zweiten Faktor  | „Was multiplikativ ist, wird gejagt" — die Lehre aus Diablo, die 13.1b zitiert                                               |
| Verhältnis zu 13.5 | —                                                  | der Vergleich „bester Satz-Aufbau gegen besten Affix-Aufbau" (#118) wird unschärfer, wenn beide Seiten denselben Topf füllen |

**Fällt die Antwort „nein", entfällt die Ausrichtung `-core`** und es bleiben 20 Einträge. Die
übrige Rechnung ändert sich nicht.

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

## Befund 1: `active` ist ein stiller Wertverlust

`POOL_NAMES` lässt `active` als Affix zu. `WornValues` hat aber **kein Feld dafür**. Gemessen:

```text
{… "gearEffects":407, "active":406}
                       ^^^^^^^^^^^^ ein Feld, das der Kampf nicht kennt
```

`wornFrom` schreibt über `summe[feld]` in einen frei benannten Schlüssel. Ein Eintrag mit `active`
lädt ohne Fehler, besteht jede Prüfung — und **406 von 1219 Promille verfallen**, ohne dass irgendwo
etwas auffällt.

Das ist die gefährliche Sorte: Der Gegenstand sieht vollwertig aus und ist ein Drittel schwächer.
Zwei Wege stehen offen — `active` aus `POOL_NAMES` entfernen, oder ihm ein Feld geben. Welcher
richtig ist, hängt daran, ob „aktive Verstärkungen" je aus Ausrüstung kommen sollen (13.1b nennt
dort Overdrive, Synergien und Zersetzungs-Stapel, also Kampf-Quellen).

**Eigenes Vorhaben.** Es berührt `pools.ts`, `worn.ts` und die Katalog-Prüfung — nicht den Katalog.

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
