# Das Ausrüstungs-Raster — 10 Einträge sind heute sinnvoll, nicht 30

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
> ```
>
> **Stand 2026-08-10**, gemessen an `main`: **1 Eintrag** (`test-relic`), auf Platz **0**.
> **10 Plätze** im Code.
>
> **Der zweite Befehl ist mit Absicht ein Ablesen und kein Zählen.** Mein erster Versuch war
> `grep -c 'export const SLOT_[A-Z]* ='` und meldete **11** — er zählte `SLOT_COUNT` mit. Eine
> Zeilenzahl misst die Schreibweise, nicht die Sache; deshalb steht hier die Konstante, die das
> Programm selbst benutzt.

**Alle Zahlen sind Platzhalter.** Sie stehen da, damit der Balance-Läufer etwas zu messen hat —
nicht als Vorschlag.

---

## Die kurze Fassung

**Die Maschine ist fertig, der Katalog ist leer — aber ein größerer Katalog bringt heute nichts.**

Gemessen: Zwei Einträge derselben Qualität und Stufe sind praktisch **gleich stark und gleich
verteilt**, egal welche Werte in welcher Reihenfolge darin stehen. Ein Katalog mit dreißig
Einträgen hätte dreißig Namen und eine einzige Wirkung.

Deshalb schlägt dieses Raster **10 Einträge** vor — einen je Platz, damit kein Platz leer bleibt —
und benennt die **drei Entscheidungen**, die vorher fallen müssen, damit ein größerer Katalog
überhaupt eine Wahl anbietet.

Das ist die unbequemere Antwort. Die bequeme wäre eine Tabelle mit dreißig Zeilen gewesen, die
sich in der Spalte „Rolle in der Wahl" unterscheiden und sonst nirgends.

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
viel Vielfalt ein Katalog überhaupt tragen kann** — und genau das war die Überraschung.

---

## Der Befund, der die Zahl bestimmt: das Budget verteilt gleichmäßig

Ein Eintrag nennt `affixKinds`, und `distributeBudget` verteilt das Wertebudget darauf. Gemessen
am 2026-08-10 für einen Gegenstand der Stufe 40:

```text
Q0 common     slots=1  budget= 620  ->  [620]
Q1 rare       slots=2  budget= 887  ->  [444, 443]
Q2 epic       slots=3  budget=1219  ->  [407, 406, 406]
Q3 legendary  slots=3  budget=1219  ->  [407, 406, 406]
```

Die Formel ist `shareOfSlot = floor((budget + slots - 1 - index) / slots)` (`src/gear/budget.ts`).
Der Unterschied zwischen dem ersten und dem letzten Platz ist **ein Promille von 407** — das ist
die Verteilung des Rundungsrestes, keine Betonung.

**Daraus folgen zwei Dinge, die ich beide zuerst falsch angenommen hatte:**

### 1. Die Reihenfolge der `affixKinds` bedeutet nichts

Mein erster Entwurf dieses Dokuments baute darauf dreißig Einträge auf: je Platz eine
CATALYSIS-betonte, eine PURITY-betonte und eine YIELD-betonte Fassung, unterschieden durch die
Reihenfolge. `['catalysis','purity','yield']` gegen `['yield','purity','catalysis']` — gemessen
sind das **407/406/406 gegen 407/406/406**. Dieselbe Sache mit vertauschten Etiketten.

### 2. Weniger Affixe machen einen Gegenstand schwächer, nicht konzentrierter

Der naheliegende Ausweg wäre ein Eintrag, der nur **einen** Wert nennt und dafür alles hineinlegt.
Das tut er nicht:

```text
Epic-Budget gesamt:                  1219
  Eintrag mit 1 affixKind bekommt:    407
  Eintrag mit 3 affixKinds bekommt:  1219
```

`wornFrom` läuft `i < anzahl && i < def.affixKinds.length` — wer weniger Arten nennt, als die
Qualität Plätze gibt, **verliert den Rest ersatzlos**. Ein „reiner Tempo-Fokus" wäre nicht
spezialisiert, sondern um zwei Drittel schwächer.

### Was das für 13.1b bedeutet

13.1b begründet die drei getrennten Faktoren so:

> Weil sie sich multiplizieren, will man alle drei mischen statt einen zu stapeln. Das ist der
> eigentliche Designwert — nicht „mehr Zahl", sondern **keine Monokultur**.

**Heute ist die Mischung nicht gewählt, sondern erzwungen.** Es gibt genau drei Sonderwerte und bei
Epic genau drei Plätze; jeder vollwertige Gegenstand trägt alle drei zu gleichen Teilen. Eine
Monokultur ist gar nicht baubar — und damit ist auch die Entscheidung dagegen keine.

Das ist kein Fehler im Code: `budget.ts` tut genau, was 13.3 sagt („feste Formel, Qualität als
Faktor", „keine Werte-Streuung"). Es ist eine Lücke im **Design**: Woraus die Wahl zwischen zwei
Gegenständen besteht, steht dort nicht.

---

## Gruppe A — heute baubar und sinnvoll: 10 Einträge

Einer je Platz, damit kein Platz leer bleibt. Alle drei Sonderwerte, weil alles andere gemessen
schwächer ist.

**Stufe und Qualität stehen mit Absicht nicht in der Tabelle.** Sie gehören zum Eintrag
(`itemLevel`, `quality`), werden aber beim Fall ohnehin neu gewürfelt — und sind damit
Balance-Zahlen, keine Katalog-Entscheidung. Welchen Wert sie tragen sollen, misst der
Balance-Läufer.

| #   | Platz          | Kennung         | `affixKinds`               |
| --- | -------------- | --------------- | -------------------------- |
| 1   | **Focus**      | `focus-01`      | `catalysis, purity, yield` |
| 2   | **Catalyst**   | `catalyst-01`   | `catalysis, purity, yield` |
| 3   | **Sigil**      | `sigil-01`      | `catalysis, purity, yield` |
| 4   | **Relic**      | `relic-01`      | `catalysis, purity, yield` |
| 5   | **Grimoire**   | `grimoire-01`   | `catalysis, purity, yield` |
| 6   | **Shard**      | `shard-01`      | `catalysis, purity, yield` |
| 7   | **Talisman**   | `talisman-01`   | `catalysis, purity, yield` |
| 8   | **Phylactery** | `phylactery-01` | `catalysis, purity, yield` |
| 9   | **Anchor**     | `anchor-01`     | `catalysis, purity, yield` |
| 10  | **Prism**      | `prism-01`      | `catalysis, purity, yield` |

**Dass alle zehn Zeilen gleich aussehen, ist das Ergebnis, nicht die Nachlässigkeit.** Solange die
Verteilung gleichmäßig ist, gibt es zwischen zwei Einträgen derselben Qualität nichts zu
unterscheiden außer dem Platz.

**Die drei Grundwerte stehen bei keinem dabei, und das ist richtig.** ESSENCE, VESSEL und WARD
trägt **jeder** Gegenstand (13.3); `wornFrom` addiert sie aus `basePermille(itemLevel, …)`, ohne
dass sie im Eintrag stehen. Wer sie in die `affixKinds` schriebe, zählte sie doppelt.

**Die Qualität ist keine Achse dieses Katalogs.** Sie wird beim Fall gewürfelt (`qualityOfRoll` in
`src/loot/rates.ts`) und beschneidet die Affixe: Ein Eintrag mit drei `affixKinds` bedient Common,
Rare und Epic aus einer Vorlage. Vier Einträge je Platz wären vier Kopien. Dasselbe gilt für die
Gegenstandsstufe — `itemLevelOf(instance, difficulty, endlessLevel)` rechnet sie beim Fall aus.
**Beides steht allerdings unter dem Vorbehalt des Befundes weiter unten.**

---

## Die drei Entscheidungen, die einen größeren Katalog erst möglich machen

Alle drei sind **Design-Fragen**, keine Bau-Aufgaben. Ich lege sie vor, statt sie zu beantworten.

### A. Mehr Affix-Arten als Plätze

Heute: drei Sonderwerte, drei Plätze bei Epic. Gäbe es **mehr** wählbare Arten als Plätze, wäre die
Auswahl eine Wahl. Das ist die Bauart von WoW und Diablo, auf die sich 13.1b beruft.

Die vorhandenen Töpfe (`src/gear/pools.ts`) nennen sechs Kennnummern — `essence`, `catalysis`,
`purity`, `yield`, `active`, `gearEffects`. Vier davon sind heute als Affix wählbar.

### B. Konzentration ohne Verlust

Alternativ: Ein Eintrag mit weniger Arten legt das **ganze** Budget auf sie. Dann wäre „reiner
Tempo-Fokus" eine echte Ausrichtung statt einer Strafe. Das ist eine Änderung an
`distributeBudget`, und sie berührt die Prüfsummen — anders als der Katalog selbst.

**Vorsicht, und 13.1b sagt es selbst:** Genau so entsteht die Monokultur, die dort abgelehnt wird.
Die Entscheidung gehört deshalb dem Auftraggeber, nicht dem Bau.

### C. Die Zuleitung zu Topf 4 (siehe Gruppe B)

Sätze und legendäre Effekte sind die Vielfalt, die das Design ohnehin vorsieht — sie kommen nur
nicht an.

---

## Gruppe B — heute NICHT baubar: alles, was `gearEffects` braucht

**Gesperrt heißt:** Der Faktor existiert und ist paritätisch geprüft, aber **nichts füllt ihn**.
Gemessen: `gearEffectsPermille(legendaries, setTiers)` (`src/gear/sources.ts`) und `valueShareOf`
(`src/gear/sets.ts`) werden von **niemandem** gerufen; `worn.gearEffects` ist im Spiel immer 0.

| Was                                      | Design | Warum gesperrt                                                                |
| ---------------------------------------- | ------ | ----------------------------------------------------------------------------- |
| **Legendäre Sondereffekte**              | 13.3   | „Echter Sondereffekt, der die Rotation verändert" — das ist Topf 4            |
| **Satz-Boni**, 6 Sätze × 2/4/6           | 13.5   | `SET_COUNT = 6` und `SET_TIERS = [2,4,6]` stehen im Code, die Zuleitung fehlt |
| **Rotations-Effekte** höherer Satzstufen | 13.5   | `activeRotationEffects()` in `sets.ts` ruft ebenfalls niemand                 |

**Ein Eintrag der Qualität `legendary` wäre heute ein Epic mit anderem Namen** — gemessen: `Q3`
bekommt dieselben `slots=3` und dasselbe `budget=1219` wie `Q2`. Deshalb steht in Gruppe A **kein**
legendärer Eintrag: nicht aus Vorsicht, sondern weil das Wort sonst etwas verspräche, was der Code
nicht einlöst.

**Das ist der Grund, warum der Balance-Vergleich aus 13.5 („bester Satz-Aufbau gegen besten
Affix-Aufbau", #118) heute nicht laufen kann.** Die Satz-Seite kommt im Kampf nicht an.

---

## Zwei weitere Befunde

### Der Fund verliert seine Stufe und seine Qualität

`rollLoot` liefert einen Fund mit **gewürfelter** Stufe und Qualität. Verbucht wird er in
`recordRun` (`src/analysis/run-recorder.ts`) so:

```ts
[...state.items, { itemId: drop.itemId, upgradeSteps: 0, equippedSlot: null }];
```

`drop.itemLevel` und `drop.quality` werden **nicht übernommen** — `OwnedItem` hat keine Felder
dafür. `wornFrom` liest die Stufe danach aus dem **Katalog**. Gemessen am 2026-08-10:

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

**Für dieses Raster heißt das:** Die Begründung, warum Qualität und Stufe keine Katalog-Achsen
sind, steht und fällt damit. Wird der Befund behoben, trägt sie. Wird er es nicht, müsste die
Vielfalt doch aus der Zahl der Einträge kommen — und dann bräuchte jede Qualität und jede
Instanz-Stufe ihren eigenen Eintrag.

**Das ist ein eigenes Vorhaben**, kein Teil des Katalogs: Es berührt `OwnedItem`, das
Spielstand-Format und `wornFrom`, also auch die Fassung des Spielstands.

### `boundToBoss` ist gebaut und wartet auf einen Träger

`mayDropAt(item.boundToBoss, bossId)` steht in `src/loot/roll.ts` und wird bei jedem Fund geprüft.
Der Kommentar dort sagt: „Heute trägt kein Eintrag eine Bindung; die Prüfung steht trotzdem, weil
sie sonst beim ersten gebundenen Stück vergessen würde."

**Der Katalog kann davon Gebrauch machen, ohne dass etwas gebaut werden muss.** Ich schlage es hier
**nicht** vor, weil die Bosse noch nicht stehen (der Auftraggeber hat „erst eine Instanz
vollständig" entschieden) — aber wer diese Instanz baut, sollte wissen, dass die Bindung schon da
ist.

---

## Was dieses Raster NICHT entscheidet

- **Die Basiswerte selbst.** Wie viel ESSENCE ein Gegenstand auf Stufe 1 trägt, ist laut 13.3
  ausdrücklich eine Balance-Zahl, die gemessen und nicht gesetzt wird.
- **Die Anzeige-Namen.** `focus-01` und so weiter sind Kennungen. Spieltexte sind englisch
  (Design-Gesetz 5); die Namen gehören zum Katalog-Bau, nicht zum Raster.
- **Wie viele Einträge je Instanz fallen dürfen.** Das ist 13.4 („Obergrenze durch Inhalt") und
  hängt an den Instanzen, die es noch nicht gibt.
- **Ob es Sätze im Aufstieg gibt.** 13.5 sagt nein, und daran ändert dieses Raster nichts.
