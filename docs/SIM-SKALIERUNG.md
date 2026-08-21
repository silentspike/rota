# Der Kombinationsraum eines Rotations-Simulators — Problemstellung

_Stand 2026-08-14. Alle Zahlen aus dem Bestand gemessen, nicht geschätzt. Dieses Dokument
ist als eigenständige Vorlage gedacht: Wer es liest, kennt das Projekt nicht._

---

## 1. Was das Programm ist

Ein Browser-Spiel („ROTA") plus ein Simulator dazu, nach dem Vorbild von SimulationCraft.
Der Spieler bewegt keine Figur — er drückt Tasten in der richtigen Reihenfolge und zum
richtigen Zeitpunkt. Gemessen wird der Schaden, den eine Zauberfolge („Rotation") gegen
einen Boss in 60 Sekunden erreicht.

Die Spiellogik existiert **zweimal**, in TypeScript (das Spiel) und in Rust (der Simulator).
Beide müssen bitgleich rechnen; das wird über Prüfsummen je Zeitschritt abgesichert.

**Der Simulator ist deterministisch bis auf eine einzige Stelle.** Es gibt keine
Zufalls-Procs im Kit; der einzige Zufall ist der kritische Treffer, und seine
Wahrscheinlichkeit ist `purity / 1000` aus der Ausrüstung. Bei `purity = 0` liefern alle
Durchgänge bitgleich dasselbe Ergebnis (mehrfach gemessen: Streuung 0,0).

---

## 2. Die Achsen des Kombinationsraums

| Achse | heute im Bestand | Ziel laut Plan |
| --- | --- | --- |
| **Zauber** | 58 spielbar | 60 |
| Leistenplätze | 12 auf Höchststufe (2 zu Beginn, 10 durch Aufstieg) | unverändert |
| **Kandidatenzeilen** | 93 | wächst mit den Zaubern |
| **Ausrüstungsplätze** | 10 | unverändert |
| Affix-Arten | 4 in den Daten (`catalysis`, `essence`, `purity`, `yield`), 6 im Design | 6 |
| Affix-Plätze je Stück | 1 / 2 / 3 / 3 nach Qualität (common … legendary) | unverändert |
| Gegenstandsstufen | 1 bis 85 | unverändert |
| **Talentbäume** | 6 | 15 Zweige |
| Talente | 29 | ~50 |
| **Talentpunkte** | 50 verteilbar (voller Ausbau bräuchte 113) | ~50 |
| **Bosse** | 17 echte (plus 18 Prüf-Bosse) | 68 |
| Schwierigkeiten | 4 | 4 |

**„Kandidatenzeile" statt „Zauber":** Eine Rotation ist eine geordnete Liste von Zeilen der
Form `zauber if bedingung`. Derselbe Zauber erzeugt mehrere Zeilen — bedingungslos, an ein
Verstärkerfenster gebunden, an einen Ressourcenschwellwert, an einen laufenden Boss-Zauber.
Aus 58 Zaubern werden so 93 Kandidatenzeilen.

### Größenordnung

```
Rotationen      93 Zeilen, Länge 12, geordnet, ohne Wiederholung   ≈ 2,0 · 10²³
Talente         50 Punkte auf 29 Talente (Oberschranke)            ≈ 1,2 · 10²¹
Ausrüstung      10 Plätze × Affix-Verteilung (rare)                ≈ 6,0 · 10⁷

nur diese drei Achsen, multipliziert                               ≈ 1,5 · 10⁵²
```

Dazu kommen Gegenstandsstufe (85), Qualität (4), Boss (17→68) und Schwierigkeit (4).

**Erschöpfendes Absuchen ist ausgeschlossen.** Die Frage ist nicht, wie man den Raum
durchzählt, sondern wie man die relevanten Gebiete findet.

---

## 3. Was heute gebaut ist

### Der Kampf

Ereignisgesteuert (Prioritätswarteschlange, keine Taktschleife über alle Millisekunden).
Zeitauflösung 10 ms, globale Wartezeit zwischen zwei Aktionen 1500 ms. Ein Kampf dauert
60 Sekunden Spielzeit, also höchstens 40 Spieleraktionen.

### Die Suche

Ein Greedy-Verfahren: Die Liste wird zeilenweise aufgebaut, jede Kandidatenzeile wird an
**jeder** Position eingefügt und die beste Einfügung behalten.

Belegte Güte des Verfahrens:

```
gierig (nur anhängen)            4000,0     verfehlt das Optimum um ein Drittel
einfügen an jeder Stelle         5960,0     erreicht es
erschöpfend (325 Listen)         5960,0     dasselbe Ergebnis

test-metronome (300 s Kampf)    31120,0     übertrifft die Handschrift (31000)
```

Das Greedy-Verfahren trifft also das erschöpfende Optimum in den Fällen, in denen sich
letzteres noch rechnen ließ — mit etwa einem Fünfunddreißigstel der Messungen.

Zusätzlich seit kurzem: **Paarbildung.** Zwei Zeilen werden gemeinsam eingefügt, wenn die
eine ein Verstärkerfenster ist und die andere den Verstärker setzt. Grund: Die Suche
bewertet jede Zeile einzeln; eine Verstärkerzeile allein bringt nichts, erst das Paar
zahlt sich aus. **Diese Paarbildung ist eng:** Sie findet nur Verstärker-Paare, keine
Dreiergruppen und keine Paare ohne Buff-Bedingung.

### Was gemessen wird

Pro Rotation ein Balance-Lauf mit N Durchgängen; ausgegeben werden Mittelwert, Streuung,
Konfidenzintervall, Minimum, Maximum, Siege, Tode, Zeit bis Sieg, Zeit bis Tod, erlittener
Schaden, absorbierter Schaden, Heilung.

### Laufzeiten, gemessen

```
833 Listen je Boss, Länge 6, 2 Bosse, 50 Durchgänge je Liste    einige Minuten
8 Gegenstandsstufen × 4 Messreihen, Suche je Lauf neu           25 min
4 Bosse, Suche je Boss, Länge 6                                 28 s
17 Bosse, Länge 12, --release                                   ~50 min
```

### Hardware, gemessen am 2026-08-14

**Der Build-Server, auf dem der Simulator läuft** (LXC-Container auf einem Proxmox-Host):

```
CPU              Intel Core i7-3930K @ 3.20 GHz, bis 3800 MHz
                 Sandy Bridge-E, Baujahr 2011
Kerne            Host 12 Threads, der Container sieht 8 (nproc)
Befehlssätze     avx  sse4_1  sse4_2
                 KEIN AVX2, KEIN FMA, kein AVX-512
RAM              12 GiB, davon 11 GiB frei
GPU              keine durchgereicht (/dev/nvidia* fehlt)
Platte           320 GB, 94 % belegt — 22 GB frei
```

**Der Entwicklungsrechner** (Laptop, dort läuft das Spiel im Browser, nicht der Simulator):

```
CPU              AMD Ryzen 9 5900HS — 8 Kerne, 16 Threads, bis 3301 MHz
Befehlssätze     avx  avx2  fma  sse4_1  sse4_2
RAM              14 GiB, davon 1,3 GiB verfügbar
GPU              NVIDIA RTX 3050 Ti Laptop, 4 GB VRAM, Compute 8.6, Treiber 580.142
                 dazu integrierte AMD Radeon Vega
```

**Der Proxmox-Host** hat dieselbe CPU wie der Container (i7-3930K, 12 Threads), 39 GiB RAM und
eine AMD Radeon RX 470/480/570/580 — nicht durchgereicht. Neben dem Build-Server laufen dort
weitere Container.

**Was daran für Low-Level-Vorschläge zählt:**

- **AVX2 und FMA fehlen auf der Maschine, die rechnet.** Nur AVX (256 Bit für Gleitkomma;
  ganzzahlige Vektoroperationen bleiben auf 128 Bit SSE). Jeder Vorschlag, der AVX2, FMA oder
  AVX-512 voraussetzt, läuft dort nicht oder fällt auf Skalarcode zurück.
- **Keine GPU am Simulator.** Der Container hat keine; die vorhandenen GPUs sitzen im Laptop
  (4 GB) und im Host (nicht durchgereicht).
- **22 GB freie Platte.** Vorschläge, die auf Zwischenspeicherung großer Ergebnismengen bauen,
  stoßen früh an.
- **Verteilung über mehrere Maschinen ist nicht gebaut**, wäre aber möglich — der Host trägt
  weitere Container.

---

## 4. Drei gemessene Reserven im heutigen Aufbau

### 4.1 Identische Durchgänge

Der einzige Zufall ist der kritische Treffer mit Wahrscheinlichkeit `purity / 1000`.
Bei `purity = 0` — jeder Lauf ohne Purity-Ausrüstung, also die gesamte Rotationssuche mit
neutralen Faktoren — sind alle Durchgänge **bitgleich identisch**.

```
rotation-search.rs     RUNS = 50
stufen-durchsatz.rs    iterations = 200
```

Das sind 50 bzw. 200 identische Rechnungen. **Faktor 50 bis 200, ohne dass sich eine Zahl
ändert.** Sobald Purity im Spiel ist, greift der Hebel nicht mehr — und genau die Läufe für
„Spezialausrüstung" tragen sie.

### 4.2 Abgeschaltete Parallelität

```
rotation-search.rs:129      threads: 1
resistance-threshold.rs:46  threads: 1
doomsayer-672.rs:63         threads: 1
Maschine                    8 Kerne
```

Die Parallelisierung ist gebaut (`thread::scope`) und in den Suchläufen abgeschaltet.

### 4.3 Parallelität auf der falschen Achse

`threads` verteilt die **Durchgänge einer Rotation** auf Kerne. Die unabhängigen Einheiten
sind aber die **Kombinationen**: Rotationen, Ausrüstungsvarianten, Talentverteilungen. Sie
teilen keinen Zustand.

Es gibt einen Grund für die heutige Bauart — Fließkomma-Addition ist nicht assoziativ, und
Ergebnisse werden deshalb nach Startwert sortiert, bevor sie summiert werden, damit derselbe
Lauf zweimal dieselbe Zahl liefert. Diese Sortierung muss jede Parallelisierung erhalten.

---

## 5. Die eigentliche Aufgabe

> Es kommt darauf an, welche der Zauber ein Spieler **aktiv verwendet** und wie viele
> Talentpunkte er hineingesetzt hat. Gebraucht wird ein Simulator, der Millionen von
> Kombinationen prüfen kann — alle Zauber, alle Varianten, die ein Spieler ausgerüstet haben
> kann, und das gegen die Talentpunkte verglichen. Balancing bedeutet dann, dass Skills
> **und** Talente angepasst werden müssen. **Spezialskillungen mit Spezialgear sollen sich
> ebenso lohnen.**

Der letzte Satz ist das Ziel und verwirft eine Annahme, auf der alle bisherigen Messungen
standen: dass es *eine* beste Rotation gibt, gegen die alles andere antritt. Gewollt ist
das Gegenteil — **mehrere Wege, die sich mit passender Ausrüstung und passender
Talentverteilung je für sich lohnen.**

Daraus folgt eine andere Fragestellung als „finde das Optimum":

1. **Nicht ein Maximum, sondern eine Menge lokaler Maxima.** Für jede Zusammenstellung
   (Zauberauswahl + Ausrüstungsprofil + Talentverteilung) die beste Rotation — und dann die
   Frage, ob diese Zusammenstellung mit *ihrer* besten Rotation konkurrenzfähig ist.
2. **Die Achsen sind gekoppelt.** Ein Talent, das Brand-Dauer verlängert, ändert die optimale
   Rotation. Ein Affix, das Abklingzeiten verkürzt, ebenso. Man kann nicht erst die Rotation
   optimieren und dann die Ausrüstung.
3. **Das Ergebnis ist eine Balance-Aussage, keine Empfehlung.** Gesucht wird nicht die beste
   Zusammenstellung für den Spieler, sondern die Antwort: Welche Zauber/Talente/Affixe
   gewinnen *nirgends* — und welche Zahl müsste man ändern, damit sie irgendwo gewinnen?

---

## 6. Konkrete Fragen an ein Verfahren

1. **Suchverfahren.** Greedy mit Einfügen trifft das Optimum in den bisher prüfbaren Fällen.
   Es ist aber blind für Zusammenspiel, das erst ab drei Zeilen entsteht, und für Paare ohne
   Verstärkerbeziehung. Was ersetzt oder ergänzt es sinnvoll — Beam Search, genetische
   Verfahren, Simulated Annealing, Monte-Carlo-Baumsuche, ganzzahlige Programmierung? Was
   passt zu einer Zielfunktion, die *ein Simulationslauf* ist und keine geschlossene Formel?

2. **Gekoppelte Achsen.** Wie sucht man in einem Raum, dessen Teilräume sich gegenseitig
   bedingen, ohne das Produkt aller Achsen zu durchlaufen? Abwechselnde Optimierung? Ein
   gemeinsamer Suchzustand?

3. **Kostenreduktion je Auswertung.** Ein Kampf ist eine ereignisgesteuerte Simulation über
   60 Sekunden mit ≤ 40 Spieleraktionen. Was lässt sich zwischen ähnlichen Rotationen
   wiederverwenden — gemeinsame Präfixe, Zwischenzustände, inkrementelle Neuberechnung? Lohnt
   sich ein Frühabbruch, wenn eine Liste uneinholbar zurückliegt?

4. **Datenlayout und Low-Level.** Der Simulator ist Rust mit Prioritätswarteschlange und
   Ereignissen. Was bringt SIMD, ein anderes Speicherlayout (SoA statt AoS),
   Vermeidung von Allokationen je Takt, oder das Rechnen mehrerer Kämpfe gleichzeitig in
   einem vektorisierten Durchlauf? Wann lohnt sich GPU?

5. **Reproduzierbarkeit.** Die Fließkomma-Summierung nach Startwert-Reihenfolge ist heute
   die Zusicherung dafür, dass zwei Läufe dieselbe Zahl liefern. Wie hält man das bei
   massiver Parallelität — deterministische Reduktion, Ganzzahl-Arithmetik,
   Kahan-Summierung?

6. **Wann ist genug gesucht?** Bei einem Raum dieser Größe gibt es kein „fertig". Welches
   Abbruchkriterium trägt eine Balance-Aussage — Konvergenz über mehrere Startpunkte, ein
   Anteil des abgesuchten Raums, statistische Schranken?

---

## 7. Randbedingungen

- **Determinismus ist nicht verhandelbar.** Zwei Läufe mit denselben Eingaben müssen
  dieselbe Zahl liefern, sonst ist keine Balance-Aussage nachrechenbar.
- **Die Spiellogik existiert zweimal** (TypeScript und Rust) und muss bitgleich bleiben.
  Änderungen an der Kampfrechnung treffen beide Seiten.
- **Eine Maschine, 8 sichtbare Kerne, 12 GB, CPU von 2011 ohne AVX2 und ohne FMA, keine GPU.**
  Siehe die vollständigen Angaben in Abschnitt 3. Verteilung über mehrere Rechner ist möglich,
  aber heute nicht gebaut.
- **Keine Zufalls-Procs im Spielerkit** (Design-Entscheidung). Der einzige Zufall ist der
  kritische Treffer.
- Kampfdauer 60 s, Zeitauflösung 10 ms, globale Wartezeit 1500 ms.
