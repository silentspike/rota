---
id: ROTA-STEP1-SPEC
status: Stable
ssot: true
refs:
  docs: [GAME-DESIGN, VERIFICATION, WORKER-GUIDE]
---

# Schritt 1 — Spezifikation mit konkreten Zahlen

> Ergänzt [`GAME-DESIGN.md`](GAME-DESIGN.md) 20.2 um alles, was zum Bauen fehlt.
> Jede Zahl hier ist **verbindlich** — sie ist gegen das Abnahme-Kriterium gerechnet, nicht geschätzt.
> Belegt durch [`tools/step1-check.py`](../tools/step1-check.py); das Skript ist zugleich die
> **Referenz, gegen die das Spiel geprüft wird**.

## Was Schritt 1 beantworten soll

Zwei Fragen, mehr nicht:

1. **Fühlt sich das Drücken gut an?** — beurteilt der Auftraggeber
2. **Lohnt sich Können messbar?** — beantwortet die Rechnung unten: **ja, um 48 %**

---

## 1. Die drei Zauber

Alle drei gehören zu **CALCINATION** und zahlen auf deren Mechanik ein: Hitze sammeln und entladen.

### 1.1 KINDLE — der Füller

| Feld         | Wert              |
| ------------ | ----------------- |
| Grundschaden | **100**           |
| Abklingzeit  | **1,5 s**         |
| Wirkzeit     | 0 (Sofort-Zauber) |
| Löst GCD aus | ja                |
| Hitze        | **+3**            |

> **Warum die Abklingzeit genau dem GCD entspricht:** Der Füller muss die Taktlücken schließen können.
> Bei einer längeren Abklingzeit entstünde Zwangs-Leerlauf, den **kein Können vermeiden kann** — das
> wäre kein Rotationsspiel, sondern eine Wartezeit. Der erste Entwurf hatte 3 s und erzeugte
> gemessene **15 Sekunden Leerlauf** in einem 60-Sekunden-Kampf.

### 1.2 SMOULDER — der Brand-Effekt (DoT)

| Feld            | Wert                                                                        |
| --------------- | --------------------------------------------------------------------------- |
| Direktschaden   | 0                                                                           |
| Schaden je Tick | **40**                                                                      |
| Tick-Abstand    | **2 s**                                                                     |
| Dauer           | **12 s** — Ticks bei 2, 4, 6, 8, 10 und **12 s** = **6 Ticks, 240 Schaden** |
| Abklingzeit     | **1,5 s** (= GCD, also praktisch keine)                                     |
| Löst GCD aus    | ja                                                                          |
| Hitze je Tick   | **+1**                                                                      |
| Erneuern        | **setzt zurück — die verbleibende Restzeit ist verloren**                   |

> **Der letzte Tick fällt mit dem Ablauf zusammen** (`aktiv` prüft `Ablaufzeit >= jetzt`, nicht `>`).
> Das war zwischen § 1.2 und dem Referenz-Skript uneinheitlich und wurde am 2026-07-28 zugunsten
> von 6 Ticks entschieden: Der Spieler sieht **12,0 s** Restlaufzeit. Bei 5 Ticks wären die letzten
> zwei Sekunden wirkungslos, und „so spät wie möglich erneuern" (GAME-DESIGN 7.2) hieße heimlich
> „bei 10 s". Eine Anzeige, die zwei Sekunden zu viel verspricht, ist eine versteckte Falle — und
> die DoT-Restlaufzeit ist laut 7.2 **die wichtigste Anzeige neben den Abklingzeiten**.

> **Warum keine eigene Abklingzeit:** Wie in WoW hindert das Spiel niemanden daran, zu früh zu
> erneuern. **Genau darin liegt die Fähigkeit.** Eine Abklingzeit, die zu frühes Erneuern technisch
> verhindert, würde dem Spieler die Entscheidung abnehmen — und damit den halben Lerninhalt von
> Schritt 1.

### 1.3 PYRE — der Entlade-Zauber

| Feld         | Wert                              |
| ------------ | --------------------------------- |
| Schaden      | **12 × Hitze**                    |
| **Bonus**    | **ab 15 Hitze doppelter Schaden** |
| Abklingzeit  | **5 s**                           |
| Wirkzeit     | 0 (Sofort-Zauber)                 |
| Löst GCD aus | ja                                |
| Wirkung      | setzt die Hitze auf **0**         |

**Beispiele:** bei 10 Hitze → 120 · bei 14 Hitze → 168 · bei **15 Hitze → 360** · bei 20 Hitze → 480.

> **Warum eine Schwelle und kein „bei voller Hitze":** Beide Extreme wurden durchgerechnet und
> verworfen.
>
> _Rein linear_ (`12 × Hitze`, ohne Bonus) ist **mathematisch neutral** — jeder Hitzepunkt ist gleich
> viel wert, egal wann man ihn ausgibt. Es gäbe überhaupt keine Entscheidung zu treffen.
>
> _Genau am Maximum_ ist das andere Extrem: Der Verfall zieht die Hitze ständig unter den Cap, sodass
> der Spieler den Moment praktisch nie trifft. **Gemessen: 2 Zündungen in 60 Sekunden statt 8.**
>
> Die Schwelle bei 15 erzeugt zusammen mit dem Cap bei 20 ein **Fenster**:
>
> |                            | Folge                  |
> | -------------------------- | ---------------------- |
> | **zu früh** (unter 15)     | halber Schaden         |
> | **im Fenster** (15–20)     | voller Bonus           |
> | **zu spät** (Cap erreicht) | weitere Hitze verpufft |
>
> Das ist die Fähigkeit, die Schritt 1 lehrt — und sie ist in einem Satz erklärbar.

### 1.4 Hitze — die Schul-Mechanik

| Feld       | Wert                                                    |
| ---------- | ------------------------------------------------------- |
| Obergrenze | **20**                                                  |
| Verfall    | **1 Punkt alle 2 s**                                    |
| Startwert  | 0                                                       |
| Anzeige    | über der Zauberleiste, mit sichtbarer Markierung bei 15 |

**Netto-Aufbaurate bei durchgehendem KINDLE-Einsatz:**
`3 / 1,5 s` (KINDLE) `+ 1 / 2 s` (SMOULDER-Tick) `− 1 / 2 s` (Verfall) = **2,0 Hitze pro Sekunde**

→ Von 0 auf die Bonus-Schwelle dauert **7,5 s**, PYRE ist aber schon nach **5 s** bereit.
**Man muss also bewusst zwei Züge warten** — das ist die Entscheidung.

---

## 2. Globale Regeln

| Regel                            | Wert                                               |
| -------------------------------- | -------------------------------------------------- |
| Globale Wartezeit (GCD)          | **1,5 s** — für alle drei Zauber                   |
| Logik-Takt                       | **10 ms**, ganzzahlige Zeit                        |
| Krit                             | **entfällt in Schritt 1** (PURITY = 0 auf Stufe 1) |
| Ausrüstung, Talente, Widerstände | entfallen — alle Faktoren = 1,0                    |

> **Warum kein Krit in Schritt 1:** Auf Stufe 1 ohne Ausrüstung ist PURITY = 0. Damit ist die
> Rechnung **exakt statt gemittelt** — jeder Durchlauf liefert denselben Wert, und Abweichungen
> zwischen Spiel und Referenz sind sofort ein Fehler und kein Zufall. Die Zufallsströme müssen
> trotzdem von Anfang an angelegt sein (GAME-DESIGN 18.4), nur zieht noch niemand daraus.

---

## 3. Der Testboss

**THE FIRST CRUCIBLE**

| Feld                    | Wert      |
| ----------------------- | --------- |
| Lebenspunkte            | **4.200** |
| Überdruck-Timer         | **60 s**  |
| Schaden auf den Spieler | keiner    |
| Phasen                  | keine     |
| Mechaniken              | keine     |
| Widerstände             | keine     |

> **Warum ausgerechnet 4.200:** Damit die schlechte Rotation **knapp scheitert** — gemessen bei
> 96,3 %. Das ist die Frust-Vermeidung aus GAME-DESIGN 12.8 in Zahlen: Der Versuch bringt etwas
> (die Fortschrittsmarke steht bei 96 %), und die Fehleranalyse sagt, woran es lag. **An einem Boss,
> an dem man mit 60 % scheitert, lernt niemand etwas.**
>
> Die gute Rotation legt ihn bei **44 s mit 16 s Reserve** — großzügig genug, dass menschliche
> Reaktionszeit nicht bestraft wird (GAME-DESIGN 8.3: bewertet wird die Reihenfolge, nicht die
> Präzision am Takt).

---

## 4. Die Rechnung — Nachweis des Abnahme-Kriteriums

**Ausgeführt:** `python3 tools/step1-check.py`

```
GUTE ROTATION   (SMOULDER spät erneuern, PYRE ab 15 Hitze)
    Schaden gesamt         4.240
    Wirkungen           KINDLE 20  SMOULDER  4  PYRE  4
    PYRE-Schaden           1.440
    SMOULDER-Ticks           800
    verlorene DoT-Ticks        0
    verpuffte Hitze            0
    Leerlauf                 0.0 s
    Boss                besiegt bei 40.51 s (19.49 s Reserve)

SCHLECHTE ROTATION  (SMOULDER zu früh, PYRE sofort)
    Schaden gesamt         4.044
    Wirkungen           KINDLE 24  SMOULDER  8  PYRE  8
    PYRE-Schaden             684
    SMOULDER-Ticks           960
    verlorene DoT-Ticks       14
    verpuffte Hitze            0
    Leerlauf                 0.0 s
    Boss                ÜBERLEBT (4044/4200 = 96.3 %)
```

**Vergleich über die vollen 60 s** (ohne Kill-Abbruch, damit beide gleich lang rechnen):

| Rotation                  | Schaden   | gegenüber „gut" |
| ------------------------- | --------- | --------------- |
| **gut**                   | **5.960** | —               |
| nur DoT zu früh erneuert  | 5.368     | **−9,9 %**      |
| nur PYRE zu früh gezündet | 4.524     | **−24,1 %**     |
| beide Fehler              | 4.044     | **−32,1 %**     |

**Differenz gut ↔ schlecht: 1.916 Schaden = 47,4 % mehr.**

> **Kriterium aus GAME-DESIGN 20.3 (mindestens 10 %) erfüllt** — mit 47,4 % weit darüber.
>
> **Die beiden Fehler wiegen unterschiedlich schwer:** Der PYRE-Fehler kostet **24,1 %**, der
> DoT-Fehler **9,9 %**. Wer nur das Entlade-Fenster versteht, macht den größeren Sprung. Das ist
> kein Mangel, sondern eine brauchbare Lernreihenfolge: Die auffälligere Regel (Bonus ab 15 Hitze)
> zahlt sich zuerst aus, die feinere (spät erneuern) danach.
>
> Das 10-%-Kriterium gilt für **gute gegen schlechte Rotation**, nicht für jeden Einzelfehler.
> Vor der Korrektur auf 6 Ticks lagen beide über 10 % — dass der DoT-Fehler jetzt knapp darunter
> liegt, ist die Folge davon, dass auch die schlechte Rotation vom sechsten Tick profitiert.

### Die zwei Rotationen als Prioritätenliste

**Gut:**

```
1. SMOULDER, wenn seine Restlaufzeit unter einem GCD liegt
2. PYRE,     wenn bereit UND Hitze >= 15
3. KINDLE
```

**Schlecht** — plausibel falsch, nicht absichtlich dumm:

```
1. SMOULDER, sobald die halbe Laufzeit vorbei ist   (verschenkt Restzeit und einen GCD)
2. PYRE,     sobald bereit                          (nimmt den Bonus nie mit)
3. KINDLE
```

---

## 5. Datenformat

Zauber und Bosse sind **Datensätze, kein Code** (GAME-DESIGN 18.3). Dieses Format teilen sich das
Spiel (TypeScript) und der Simulator (Rust) — es ist damit ab jetzt eine Schnittstelle.

**`data/spells/calcination.json`**

```json
{
  "schema": 1,
  "school": "CALCINATION",
  "resource": {
    "id": "heat",
    "name": "Heat",
    "cap": 20,
    "decayPerStepMs": 2000,
    "decayAmount": 1,
    "highlightAt": 15
  },
  "spells": [
    {
      "id": "kindle",
      "name": "Kindle",
      "unlockLevel": 1,
      "baseDamage": 100,
      "cooldownMs": 1500,
      "castTimeMs": 0,
      "triggersGcd": true,
      "resourceGain": { "heat": 3 }
    },
    {
      "id": "smoulder",
      "name": "Smoulder",
      "unlockLevel": 1,
      "baseDamage": 0,
      "cooldownMs": 1500,
      "castTimeMs": 0,
      "triggersGcd": true,
      "dot": {
        "tickDamage": 40,
        "tickIntervalMs": 2000,
        "durationMs": 12000,
        "refreshMode": "reset",
        "resourceGainPerTick": { "heat": 1 }
      }
    },
    {
      "id": "pyre",
      "name": "Pyre",
      "unlockLevel": 2,
      "baseDamage": 0,
      "cooldownMs": 5000,
      "castTimeMs": 0,
      "triggersGcd": true,
      "spender": {
        "resource": "heat",
        "damagePerPoint": 12,
        "bonusThreshold": 15,
        "bonusMultiplier": 2,
        "consumesAll": true
      }
    }
  ]
}
```

**`data/bosses/first-crucible.json`**

```json
{
  "schema": 1,
  "id": "first-crucible",
  "name": "The First Crucible",
  "instance": "prologue",
  "hp": 4200,
  "overpressureMs": 60000,
  "phases": [],
  "timeline": [],
  "resistances": {},
  "signature": null
}
```

> **`refreshMode: "reset"`** ist die harte Regel aus GAME-DESIGN 7.2 — beim Erneuern verfällt die
> Restlaufzeit. Das Feld existiert von Anfang an, weil später Zauber dazukommen können, die sich
> anders verhalten; **der Wert `"reset"` ist für V1 aber der einzige zulässige.**

---

## 6. Was der Worker daraus bauen muss

| Anforderung                                          | Nachweis                                                                 |
| ---------------------------------------------------- | ------------------------------------------------------------------------ |
| Die drei Zauber verhalten sich exakt wie oben        | Aufgezeichnete Runde ergibt denselben Schaden wie `step1-check.py`       |
| Hitze baut sich mit 2,0/s auf und verfällt mit 0,5/s | Konsolen-Abfrage des Hitze-Werts über die Zeit                           |
| PYRE verdoppelt ab genau 15, nicht ab 14             | Zwei Zündungen bei 14 und 15 Hitze, Schadenswerte vergleichen            |
| SMOULDER-Erneuern verwirft die Restzeit              | Erneuern bei 6 s Restzeit → nächster Tick 2 s später, keine Überlagerung |
| Der Boss fällt mit der guten Rotation bei ~44 s      | Durchlauf im Spiel, Restzeit ablesen                                     |

---

## 7. Offene Punkte, die NICHT Schritt 1 betreffen

| Punkt                                                       | Wo geklärt                   |
| ----------------------------------------------------------- | ---------------------------- |
| Die restlichen 57 Zauber                                    | GAME-DESIGN 22, vor Epic E10 |
| Krit, Ausrüstung, Talente                                   | ab Schritt 3                 |
| Boss-Mechaniken und Phasen                                  | Epic E4                      |
| Wie die drei Zauber im fertigen Spiel freigeschaltet werden | siehe unten                  |

> **Aufgelöster Widerspruch:** GAME-DESIGN 12.2 nennt als Start-Ausstattung **zwei** Zauber, Schritt 1
> braucht aber **drei**, weil die CALCINATION-Mechanik ohne den Entlade-Zauber nicht funktioniert.
> Auflösung: KINDLE und SMOULDER auf Stufe 1, **PYRE auf Stufe 2** (siehe `unlockLevel` oben). Der
> Prototyp testet den Zustand mit allen dreien; das fertige Spiel führt sie in zwei Schritten ein.
