---
id: sim-architektur-plan
status: Draft
ssot: true
ssot_scope: Simulator-Architektur, Leistung, Betrieb und Verteilung
refs:
  docs:
    - ~/.claude/plans/greedy-skipping-russell.md
    - docs/GAME-DESIGN.md
    - docs/SIM-MESSUNGEN.md
affects:
  - sim/
---

# Simulator: Architektur, Leistung und Betrieb

> **SSOT für alles, was die Simulation betrifft.** Bei einem Widerspruch gewinnt dieses
> Dokument.
>
> | Dokument                                     | Rolle                                                  |
> | -------------------------------------------- | ------------------------------------------------------ |
> | **dieses**                                   | die Entscheidungen und die Zahlen mit ihrem Vorbehalt  |
> | `~/.claude/plans/greedy-skipping-russell.md` | der Gesamtplan des Spiels — Umfang, Epics, Reihenfolge |
> | `docs/GAME-DESIGN.md`                        | das Design — Mechanik, Formeln, Bausteine              |
> | `docs/SIM-MESSUNGEN.md`                      | die Messgrundlage: Läufer, Einstellungen, Rohwerte     |
>
> `SIM-MESSUNGEN.md` ist am Bestand geschrieben. Wo eine seiner Zahlen **Teil 1** widerspricht,
> gilt Teil 1.

## Was dieser Plan zusichert — und was nicht

**Das Ziel ist nicht Geschwindigkeit, sondern eine belastbare Balance-Aussage in bezahlbarer
Zeit.** Konkret:

> Ein vollständiger Balance-Lauf ist nach **zwei bis vier Stunden** fertig — nicht, weil alles
> gerechnet wurde, sondern weil **alle Pflichtprojektionen vollständig**, die vorgeschriebene
> Wechselwirkungsabdeckung **nachgewiesen** und jede Nische adaptiv gesucht ist. Am Ende steht
> ein **Zertifikat**, das sagt, was bewiesen und was nur gesucht wurde.

**Das ist ein Ziel-SLA nach erfolgreicher Abnahme, keine heutige Zusage.** Heute ist kein Issue
dieses Plans gebaut. Und die Zeit gilt nur gegen ein **benanntes Referenzprofil**:

```
Referenzprofil "standard"
  Rechner        Laptop + proxmox + gonet, alle erreichbar
  Profil         balanced — `reserved_physical_cores: 2` auf dem Laptop
                 (NICHT zwei SMT-Stränge: bei 8C/16T sind das 12 statt 14 Stränge)
  Datenstand     versionierter Endausbau-Datensatz
  Vertrag        versionierter CampaignRunContract
```

**Bei weniger Rechnern bleibt die Arbeit gleich — nur die ETA wächst.** Das ist der Zweck des
eingefrorenen Arbeitsbudgets (E21.0): Ein Rechnerausfall darf das **Ergebnis** nicht ändern.

| verfügbar         | Ergebnis      | Zeit                       |
| ----------------- | ------------- | -------------------------- |
| alle drei         | vollständig   | Ziel 2–4 h                 |
| Laptop + proxmox  | **identisch** | länger                     |
| nur Laptop        | **identisch** | nochmals länger            |
| `--hard-deadline` | Teilergebnis  | fest, mit `AnytimePartial` |

**Was ausdrücklich nicht zugesichert wird:** den Vollraum zu rechnen. Bei 350 000 Kämpfen/s
bräuchte allein der **reduzierte** Raum 2,8·10²⁵ Jahre. Kein Issue in diesem Plan darf mit
„dann können wir alles durchrechnen" begründet werden.

**Der Weg dorthin sind vier Hebel, in dieser Rangfolge:**

| Hebel                                  |     Faktor | gilt für                                                          | wo                  |
| -------------------------------------- | ---------: | ----------------------------------------------------------------- | ------------------- |
| **1. nicht simulieren, was feststeht** |    50–200× | **nur** Läufe mit `purity = 0` und 50–200 identischen Durchgängen | E11.1               |
|                                        |       1,7× | **nur** den Greedy-Einfügeoperator (41–52 % gemessen)             | E11.4               |
| **2. nicht zweimal rechnen**           |       1,8× | Suffix-Wiederverwendung — **überlappt mit Hebel 1**               | E11.4b, E11.4f, E19 |
| **3. Kernel**                          | 1,29–1,34× | das Abnahmeziel 470 000 gegen 350 000–365 000                     | E11.2               |
| **4. mehr Rechner**                    |      1,94× | **projizierte Summe**, kein Verbundlauf                           | E15.5               |

> **Diese Faktoren sind nicht multiplizierbar, und drei davon sind enger, als sie aussehen.**
>
> - **50–200×** gilt, solange Krit die einzige Zufallsquelle ist. Bei echten Ausrüstungsständen,
>   Legendär-Auslösern oder neuen Mechaniken fällt er weg (E11.1).
> - **1,7×** wurde am **Einfügeoperator** gemessen — nicht an LNS-Zerstören/Reparieren, Tauschen
>   oder Entfernen. „41–52 % jeder Suchrunde" wäre zu weit gefasst.
> - **1,8× Suffix überlappt mit 1,7×:** Ein völlig unveränderter Kandidat **ist** der maximale
>   Suffix-Fall. Wer beide multipliziert, zählt dieselbe Ersparnis zweimal.
> - **1,66× für den Kernel** setzt voraus, dass die gemessenen 39,9 % **ersatzlos** verschwinden.
>   Das Abnahmeziel entspricht 1,29–1,34×.
>
> **Die einzige Zahl, die am Ende zählt, ist eine andere:**
>
> ```
> End-to-End-Beschleunigung = alte Kampagnenzeit / neue Kampagnenzeit
> ```
>
> Sie wird in E11.0c gemessen, nicht aus Einzelfaktoren hochgerechnet.

**Die Reihenfolge ist trotzdem keine Vorliebe, sondern eine Rechnung:** Wer verteilt, bevor
Hebel 1 gezogen ist, verteilt überwiegend verschwendete Arbeit.

## Warum das ein eigenes Epic ist und kein Issue in E2

Der Plan führt **E2 — Simulator (Rust)** mit fünf Issues: Regel-Bibliothek, Prüffälle,
APL-Sprache, Balance-Läufer, Zauber-Vergleich. Alle fünf sind gebaut und abgenommen. Zur
**Leistung** sagt der Plan genau einen Satz:

> „**Sprache: Rust.** Volle Geschwindigkeit über alle Kerne, ganzzahlige Typen, keine
> Speicheranforderung im heißen Pfad — wie im Vorbild-Projekt."

Das ist keine Verwerfung, sondern eine Lücke: Als der Plan entstand, gab es den Simulator noch
nicht, und niemand konnte wissen, wo seine Zyklen liegen. Inzwischen ist es gemessen.

**Die zweite Plan-Aussage, die dieses Epic begrenzt**, steht ebenfalls schon da und bleibt
gültig:

> **Der Vollraum bleibt unerreichbar** (Ausrüstung × Talente × Zusammenstellung). Erschöpfend
> gerechnet werden nur die Projektionen, auf denen Balance-Entscheidungen fallen. **Der Rest
> bleibt Stichprobe.**

E11 macht die Projektionen billiger. Es macht den Vollraum nicht erreichbar, und kein Issue
darin darf so tun.

---

---

## Teil 1 — Der Maßstab: Endausbau, nicht Bestand

> **Nachgetragen am 2026-08-14 auf Einwand des Auftraggebers, und der Einwand ist
> grundsätzlich:** _„Wir haben doch nie vom Bestand geredet, sondern immer vom Endausbau!
> Sonst stimmt ja der Simulator nie."_
>
> Er hat recht, und der Fehler zieht sich durch alle Messungen. Sie sind am **heutigen
> Bestand** entstanden — und der ist ein Viertel dessen, was der Plan vorsieht. Eine
> Architektur, die auf diesen Bestand optimiert wird, ist im Endausbau falsch.

### Der Größenunterschied, gemessen

| Größe                           | Bestand                               | Plan (V1)           | Faktor     |
| ------------------------------- | ------------------------------------- | ------------------- | ---------- |
| Bosse (Instanz)                 | **16** (4 Instanzen à 4) + 1 Prolog   | **68**              | **4,25**   |
| **Aufstiegs-Instanzen à 4**     | **4 von 8**                           | 8 (= 32 Bosse)      | 2,0        |
| **Endgame-Instanzen à 6**       | **0 von 6**                           | 6 (= 36 Bosse)      | **∞**      |
| Zauber                          | 58 (10+13+2+11+10+12)                 | 60                  | 1,03       |
| Leistenplätze                   | 12                                    | **12–16**           | bis 1,33   |
| **Mechanik-Arten in Bossdaten** | **2** (`strike`, `interruptibleCast`) | **15–20 Bausteine** | **bis 10** |
| Talentzweige                    | 6 Bäume                               | **15 + global**     | 2,7        |
| Sets mit Gegenständen           | **1 von 6** (Satz 0, 6 Stück)         | 6                   | 6,0        |
| Gegenstände                     | 27, **alle `quality: 2`**             | Katalog offen       | —          |
| legendäre Sondereffekte         | **0**                                 | je Endgame-Instanz  | —          |
| Schwierigkeitsstufen            | **0** (feste `difficulty: 1`)         | 3 + Leiterstufen    | —          |

> **KORRIGIERT am 2026-08-14, zweite Runde.** Die Vorfassung nannte „Bosse 17 → 68, Faktor 4,0"
> und „Instanzen 5 → 14, Faktor 2,8". **Beide Zeilen waren zwar rechnerisch richtig und trotzdem
> irreführend**, weil sie zwei verschiedene Dinge zusammenzogen:
>
> - `first-crucible` ist der **Prolog** und gehört nicht zu den 68 (die sind 8 × 4 + 6 × 6). Die
>   Instanz-Bosszahl ist **16**, der Faktor **4,25**.
> - Von den **6 Endgame-Instanzen existiert keine einzige**. Ein Faktor 2,8 über alle Instanzen
>   verdeckt das. Und genau dort leben legendäre Effekte, Schwierigkeitsstufen und die
>   Endlos-Leiter — also die drei Systeme, deren Fehlen die Messungen dieses Plans am stärksten
>   verzerrt.
>
> **Die 27 Gegenstände tragen alle `quality: 2`.** Es gibt im Bestand keinen einzigen
> legendären, keinen epischen, keinen gewöhnlichen — der gemessene „Ausrüstungskollaps bei Stufe
> 85 legendary" beruht auf **hochgerechneten** Qualitätsstufen, nicht auf vorhandenen Daten. Das
> stand so nirgends.

**Die Zeile mit dem größten Gewicht ist die sechste.** Der Simulator kennt 13 Ereignisränge, und
das `mechanic`-Feld der Zeitleisten trägt bis heute genau **zwei** Werte — `strike` (38 Einträge)
und `interruptibleCast` (22). Der Plan sieht 15–20 Bausteine vor.

> **KORRIGIERT am 2026-08-14, beim Abgleich gegen `docs/GAME-DESIGN.md` § 11.** Hier stand:
> _„die Bossdaten führen **sieben** Zusatz-Systeme … aber kein heutiger Boss nutzt mehr als
> **zwei** davon gleichzeitig."_ **Beide Zahlen waren falsch, und zwar zu klein.** Nachgezählt
> über alle 35 Bossdateien:
>
> ```
> elf Systeme:  phases · energy · doom · affliction · race · runaway · stacks
>               escalation · bossHeal · timelineLoopMs · resistances
>
> gleichzeitig genutzt, nur echte Bosse:   0 → 1 Boss   1 → 1   2 → 12   3 → 2   4 → 1
> Maximum:  boss-2-4 mit vier (affliction, race, bossHeal, timelineLoopMs)
> ```
>
> Ich hatte `escalation`, `bossHeal`, `timelineLoopMs` und `resistances` übersehen — die
> Vorfassung stützte sich auf eine unvollständige Feldliste statt auf die Daten.
>
> **Die gute Nachricht dabei:** Alle **neun** Mechanik-Bausteine aus § 11.2 sind im Simulator
> vorhanden, keiner fehlt. Was fehlt, sind die **Varianten** (§ 22: „die restlichen 6–11
> entstehen als Varianten dieser neun") und die Dichte — das Design verlangt
> **„drei bis vier Bausteine plus eine Signatur"** je Boss, der Median liegt heute bei **zwei**.
>
> **Der Faktor in der Tabelle bleibt** — er misst die `mechanic`-Werte, und die sind wirklich
> zwei. Aber die Aussage darunter war ein Bestandsbefund über eine falsche Liste, und genau
> davor warnt dieser Teil 1.

### Welche Aussagen dadurch wackeln — und welche nicht

| Messung                                        | hängt ab von                    | im Endausbau                                                        |
| ---------------------------------------------- | ------------------------------- | ------------------------------------------------------------------- |
| **Queue max 9**                                | gleichzeitig laufenden Systemen | **wächst** — bis 20 Bausteine statt 2                               |
| **`expire_buffs` findet fast nie etwas**       | wie viele Verstärkungen laufen  | **wird kleiner als Hebel** — 6 Sets, Legendäre, mehr Mechaniken     |
| **Krit an 15 von 17 analytisch**               | Zahl der HP-Phasen-Bosse        | **unbekannt** — 51 Bosse sind nicht entworfen                       |
| **Homogenität: größte Gruppe 30**              | Zauber- und Bosszahl            | **unbekannt**                                                       |
| **Gear 56 Phänotypen**                         | 27 Gegenstände, ein Set         | **5 412** — schon hochgerechnet                                     |
| `expire_buffs` 23,8 % / Heap 16,1 % der Zyklen | Kernelstruktur                  | **bleibt** — Struktur ändert sich nicht                             |
| APL nur 4,1 %                                  | Kernelstruktur                  | **bleibt**                                                          |
| LNS 91,7 % > greedy 83,3 %                     | Suchverfahren                   | **bleibt** — Verfahrensvergleich, nicht Datenmenge                  |
| ~~Position p → Präfix p~~                      | **WIDERLEGT**                   | gilt nur für gefundene Rotationen; 99,1 % weichen vorher ab (E11.4) |
| Talente Faktor 234                             | Talentstruktur                  | **ändert sich** — 15 Zweige statt 6                                 |

**Die Kernel-Anteile bleiben, die Kapazitäten wackeln.** Das ist die Trennlinie: Wo eine
Messung eine _Struktur_ misst (welcher Code kostet wie viel), trägt sie. Wo sie eine _Menge_
misst (wie viele Ereignisse, wie viele Bosse ohne Phase), gilt sie nur für heute.

### Was das für die Architektur ändert — drei Regeln

**1. Keine Kapazität ohne Fallback.** `FixedEventQueue<32>` darf nicht auf „max 9 gemessen"
gebaut werden. Die Obergrenze folgt aus der Struktur, nicht aus dem Bestand:

```
  5   periodische Systeme   energy · doom · affliction · runaway · stacks
  4   Einzelereignisse      Zeitleiste · Boss-Cast · eigener Cast · Weckruf
+16   ein Tick je laufendem Brand        (volle Leiste)
+ 8   ein Ablauf je Verstärkungssammlung
─────
 33   und das ist noch ohne Kanal-Ticks, verzögerte Zauberwirkungen,
      Satz- und Legendär-Ereignisse, Overdrive, Spellsteal und die
      68 Boss-Signaturen
```

> **KORRIGIERT am 2026-08-14, dritte Runde.** Hier stand unter derselben Rechnung „25–30
> plausibel" und daraus „**32 reicht knapp**". **Meine eigene Summe ist 33.** Ich hatte die
> Posten aufgeschrieben und dann eine kleinere Zahl darunter gesetzt — ein Additionsfehler in
> der Begründung genau der Kapazität, die er begründen sollte.

**Damit ist 32 nicht knapp, sondern schon nach der eigenen Strukturrechnung zu klein.** Die
Kapazität wird deshalb **nicht vorab festgelegt**, sondern parametriert und gemessen:

```rust
struct EventQueue<const INLINE: usize> {
    inline: [MaybeUninit<Event>; INLINE],
    len: u8,
    overflow: SmallFallbackQueue,     // Pflicht, nicht Absicherung
    overflow_count: u64,              // Telemetrie: hat er je gegriffen?
}
```

**Gemessen wird 32 · 48 · 64** am Endausbau-Lastprofil aus E11.0b; gewählt wird die kleinste
Kapazität mit praktisch null Überläufen. Der Fallback bleibt **in jedem Fall** — eine
Kapazität ohne ihn wäre genau die Bestandsannahme, vor der Teil 1 warnt.

**Und der `u64`-Schlüssel braucht eine bewiesene Bitaufteilung, keine Formel `time | rank | seq`:**

```rust
key = (time_tick as u64) << TIME_SHIFT
    | (rank      as u64) << RANK_SHIFT
    |  sequence  as u64;
```

Nachzuweisen ist der Wertebereich jedes Feldes: `time_tick` für **300-Sekunden-Prüfkämpfe und
Endlos-Leiterstufen** (nicht nur 60 s), `rank` für 13 Ränge mit Reserve, `sequence` für die
maximale Zahl gleichzeitiger Einfügungen. **Ein Überlauf in einem der drei Felder vertauscht
die Ereignisreihenfolge — und das bricht die Parität, still.**

**`next_expiry_at` braucht ebenso ausgeschriebene Invarianten**, sonst ist die Abkürzung eine
Fehlerquelle. Der Wert muss neu bestimmt werden bei: Einfügen · Verlängern · Ersetzen ·
Entfernen · Leeren · **mehreren Effekten mit identischer Ablaufzeit** · und beim Ablauf des
bisher frühesten Effekts. Jeder dieser sieben Fälle gehört als Prüffall in die Abnahme.

**2. Keine Eigenschaft über eine Bestandsliste.** Das gilt für den analytischen Krit
(`crit_affects_control_flow()` statt Bossliste, siehe E11.4d) und genauso für jede andere
Optimierung, die heute „an fast allen Bossen" gilt.

**3. Jedes Abnahmekriterium bekommt einen Endausbau-Vorbehalt.** „Durchsatz ≥ 470 000
Kämpfe/s" gilt für die heutige Bossmischung. Wächst die Zahl der gleichzeitigen Systeme,
sinkt der Durchsatz — das ist kein Rückschritt, sondern mehr Spiel. Die Benchmark-Suite aus
E11.0 braucht deshalb **künstliche Lastprofile** als Stellvertreter für den Endausbau.

### Sieben Dinge aus Design und Plan, die im Simulator noch keine Antwort haben

_Nachgetragen 2026-08-14 aus dem vollständigen Abgleich gegen `docs/GAME-DESIGN.md` (2 806
Zeilen) und `~/.claude/plans/greedy-skipping-russell.md` (2 493 Zeilen). Keiner dieser Punkte
stand in den ersten vier Fassungen._

**1. Der Cooldown-Rücksetzer bricht die Schranke aus E11.5b.** Der Plan nennt ihn selbst als
Härtefall (C.1b):

> „Rotations-technisch sehr interessant … und **für die Optimums-Suche ein anspruchsvoller Fall,
> weil er die Reihenfolge nichtlinear beeinflusst**."

**Er ist bereits da: neun von 58 Zaubern in `sublimation.json` tragen solche Effekte** (vier
`cooldownReductionMs`, zwei `cooldownResetCharges`, dazu `freesNextGcd`, `doublesNextCast`,
`refundsInterruptedGcd`) — und `sim/src/data.rs` kennt sie. Damit gilt
`uᵢ = 1 + ⌊(T−t)/CDᵢ⌋` **heute schon nicht mehr**: Die optimistische Schranke des
Branch-and-Bound-Orakels ist ungültig, und zwar still. Auch die Suffix-Annahme aus E11.4
wackelt — eine spätere Zeile kann die Abklingzeit einer früheren ändern.
**Die Vollzählung und die Gegenmaßnahme stehen in E11.5b.**

**2. `runaway` kehrt die Siegbedingung um — „mehr Schaden" ist dort falsch.** § 11.2, Baustein 6:

|                   | Ring auf 0                      | Zeit abgelaufen               |
| ----------------- | ------------------------------- | ----------------------------- |
| gewöhnlicher Boss | **Sieg**                        | Niederlage                    |
| `runaway`         | Niederlage (die Reaktion kippt) | **Sieg** (sie ist eingedämmt) |

Schaden senkt den Ring, **Schutz und Heilung heben ihn**. Jede Zielfunktion der Form „maximiere
Schaden" findet an diesen Bossen genau die falsche Antwort — und jedes Dominanz-Pruning, das
„mehr ist besser" annimmt, verwirft die richtige. Zwei echte Bosse tragen das heute
(`boss-2-3` und einer der Prüfbosse). **Das ist der konkrete Beleg für den Monotonie-Vorbehalt
in E11.5b**, der dort bisher nur abstrakt stand.

**3. „Jeder Zauber muss irgendwo die beste Wahl sein."** C.1c Punkt 2, ebenfalls mehrfach
betont: _„keine 100 Zauber, von denen 10 gespielt werden."_ **Das ist die Zielfunktion von
E12**, nicht eine Nebenbedingung — und der gemessene Ausrüstungskollaps (bei Stufe 85
legendary bleibt **einer** von 58 Zaubern lohnend) verletzt sie direkt. E12.6 („Nischenverlust
und tote Zauber") ist damit kein Zusatz, sondern das eigentliche Abnahmekriterium des Epics.

**4. „Kein Boss verlangt eine bestimmte Schule" ist prüfbar, wird aber nicht geprüft.** § 11.3:
Jede Mechanik hat mindestens zwei Antworten (unterbrechen ODER FIXATION ODER COAGULATION). Das
ist eine **messbare Eigenschaft** — für jede Mechanik die Zahl der Zusammenstellungen zählen,
die sie lösen. Sie gehört als eigene Prüfung neben E12.6; heute prüft sie niemand.

**5. Overdrive ist eine Entscheidung, die keine APL-Zeile ist.** Einmal je Kampf, eigene Taste,
kein Leistenplatz. **Der Zeitpunkt ist eine eigene Optimierungsdimension.** Ebenso
**Spellsteal** (§ C.0), von dem der Simulator heute nichts weiß.

**Beide brauchen eine eigene Struktur — eine Prioritätsliste kann sie nicht ausdrücken:**

```rust
ActionPolicy {
    priority_list:    CompiledRotation,      // die APL
    once_per_fight:   SmallVec<OnceAction>,  // Overdrive
    reactive_actions: SmallVec<ReactiveAction>, // Spellsteal
}

OverdrivePolicy  { earliest_tick, latest_tick, trigger_condition,
                   required_buff_mask, required_boss_state }
SpellstealPolicy { target_effect, trigger_condition, permitted_window }
```

**Und eigene LNS-Operatoren (E11.5a):** Overdrive-Fenster verschieben · Bedingung ändern ·
Spellsteal-Ziel wechseln · Spellsteal und Rotation gekoppelt reparieren.

**Die Policy gehört in:** `StructuralKey` · `CompiledBuild` · Wirkungs-IR · Checkpoints ·
Abdeckungsfaktoren · Invalidierung · `KernelSignature` · Fortschritts-Orakel. **Ohne das bleibt
Overdrive eine Konstante, die der Optimierer nie bewegt** — obwohl sein Zeitpunkt an einem
Wettrennen-Boss über Sieg und Niederlage entscheiden kann.

**6. `escalation` + `timelineLoopMs` erzeugen mehr Ereignisse über die Zeit.** 15 Bosse laufen
in Schleifen, drei davon mit sich verkürzenden Wirkzeiten. Für die Queue-Kapazität heißt das:
**Die Ereignisdichte ist nicht konstant über den Kampf**, sondern steigt. Eine Messung, die nur
den Höchststand über den ganzen Kampf nennt, verdeckt das — E11.0b braucht die Verteilung
**über die Zeit**, nicht nur ihr Maximum.

**7. 68 Signatur-Regeln sind 68 Sonderfälle.** § 11.1 verlangt je Boss „**eine Regel, die es nur
bei diesem Boss gibt**". Für E11.7b (Kernel-Spezialisierung) ist das die Obergrenze der Idee:
Ein Kernel je Merkmalsmaske funktioniert, solange die Merkmale kombinierbar sind — 68 einmalige
Regeln sind es nicht. **Sie müssen über einen datengetriebenen Weg laufen**, nicht über
generierten Code, sonst wächst die Codegröße mit dem Spielinhalt.

---

---

## Teil 2 — Die Ausgangslage: Simulator und Hardware

_Was heute dasteht, worauf es läuft, und was davon im Endausbau trägt. **Die Messwege stehen
in `docs/SIM-MESSUNGEN.md`**; hier stehen die Zahlen mit ihrem Vorbehalt aus Teil 1._

### 2.1 Drei Rechner, vollständig aufgenommen

_Überarbeitet 2026-08-14 auf ausdrückliche Frage des Auftraggebers. Die erste Fassung stützte
sich auf `/proc/cpuinfo` und eine Größenordnungsrechnung; jetzt ist der Simulator auf jedem
System gelaufen._

**Erster Befund: Es sind drei Systeme, nicht vier.** `rustbuild` ist **CT 155 auf gonet** —
derselbe i7-3930K, nur ein Container darauf.

#### Vollständige Hardware-Aufnahme

**Prozessoren**

|                  | **Laptop** (Ziel)       | **gonet** `10.0.0.106`    | **proxmox** `10.0.0.69`    |
| ---------------- | ----------------------- | ------------------------- | -------------------------- |
| Modell           | Ryzen 9 5900HS          | Core i7-3930K             | Core i5-1235U              |
| Mikroarchitektur | Zen 3 (2021)            | Sandy Bridge-E (**2011**) | Alder Lake (2022)          |
| Kerne / Stränge  | 8 / 16                  | 6 / 12                    | **2 P + 8 E** / 12         |
| Takt max         | 3 301 MHz               | 3 800 MHz                 | 4 400 (P) · 3 300 (E)      |
| Takt min         | 423 MHz                 | 1 200 MHz                 | 400 MHz                    |
| BogoMIPS         | 6 588                   | 6 404                     | 4 992                      |
| L1d / L1i        | 256 + 256 KiB           | 192 + 192 KiB             | 352 + 576 KiB              |
| **L2 je Kern**   | **512 KiB** (4 MiB / 8) | **256 KiB** (1,5 MiB / 6) | 1,25 MiB (P) · 512 KiB (E) |
| L3               | 16 MiB                  | 12 MiB                    | 12 MiB                     |
| NUMA-Knoten      | 1                       | 1                         | 1                          |

> **Der i5-1235U ist heterogen, und das ist kein Detail.** Von seinen 12 Strängen liegen nur
> **vier** auf zwei P-Cores (4,4 GHz, mit HT); die anderen acht sind E-Cores (Gracemont,
> 3,3 GHz, **ohne** HT). Ein Verteiler, der gleich große Pakete auf gleich viele Stränge legt,
> wird von den E-Cores ausgebremst — die langsamsten bestimmen, wann das Paket fertig ist.
> **Auflage für E11.4c:** Paketgrößen dynamisch über einen Zähler vergeben, nicht statisch
> aufteilen. Auf dieser Maschine ist das keine Feinheit, sondern der Unterschied zwischen
> 12 Strängen und effektiv acht.

**Befehlssätze** — das entscheidet, welcher Rechner E11.7 überhaupt fahren kann

|                            | Laptop | gonet    | proxmox             |
| -------------------------- | ------ | -------- | ------------------- |
| SSE … SSE4.2               | ja     | ja       | ja                  |
| **AVX**                    | ja     | ja       | ja                  |
| **AVX2**                   | **ja** | **nein** | **ja**              |
| **FMA**                    | **ja** | **nein** | **ja**              |
| AVX-512                    | nein   | nein     | nein (Consumer-ADL) |
| BMI1 / BMI2                | ja     | **nein** | ja                  |
| F16C                       | ja     | **nein** | ja                  |
| ADX                        | ja     | **nein** | ja                  |
| AES / PCLMULQDQ            | ja     | ja       | ja                  |
| VAES / VPCLMULQDQ          | ja     | **nein** | ja                  |
| SHA-NI                     | ja     | **nein** | ja                  |
| RDRAND / RDSEED            | ja     | **nein** | ja                  |
| CLFLUSHOPT / CLWB          | ja     | **nein** | ja                  |
| AVX-VNNI                   | nein   | nein     | **ja**              |
| GFNI · SERIALIZE · WAITPKG | nein   | nein     | **ja**              |

**gonet fehlt alles nach 2013.** Kein AVX2, kein FMA, kein BMI — der E11.7-Pfad ist dort nicht
lauffähig, und selbst der skalare Code verliert `bmi2`-Bitoperationen, die `BitSet<96>` aus
E11.4 nutzen könnte.

**Der i5-1235U hat mehr als die Zielmaschine** (AVX-VNNI, GFNI, SERIALIZE). Für ROTA ist davon
**nichts** nutzbar: Der Kernel rechnet `f64`, und AVX-VNNI ist eine Integer-Skalarprodukt-
Erweiterung. Erwähnt, damit niemand später darauf baut.

**Arbeitsspeicher**

|                 | Laptop                               | gonet                  | proxmox                  |
| --------------- | ------------------------------------ | ---------------------- | ------------------------ |
| Bestückt        | 2 × 8 GiB **LPDDR4-4266** (verlötet) | 4 Module **DDR3-1333** | 2 × 16 GiB **DDR4-3200** |
| gesamt          | 14,9 GiB nutzbar                     | 39 GiB                 | 30 GiB                   |
| im Betrieb frei | **~2 GiB** — der Engpass             | ~12 GiB                | ~7 GiB                   |
| zram            | 10 GiB                               | —                      | 4 GiB                    |

**DDR3-1333 gegen DDR4-3200 ist Faktor 2,4 in der Bandbreite** — bei einem Kernel, der auf
`Fight` mit 5 600 Byte je Kampf arbeitet, trägt das zum gemessenen Rückstand von gonet bei.

**Grafik** — nur für E13 relevant, und dort ernüchternd

|                    | Laptop                                | gonet                                         | proxmox                            |
| ------------------ | ------------------------------------- | --------------------------------------------- | ---------------------------------- |
| dediziert          | **NVIDIA RTX 3050 Ti Mobile**, GA107M | **AMD Radeon RX 470/480/570/580** (Ellesmere) | —                                  |
| VRAM               | 4 096 MiB                             | (Polaris, 4–8 GB)                             | —                                  |
| Compute Capability | **8,6**                               | — (GCN 4)                                     | —                                  |
| Treiber            | 580.142                               | —                                             | —                                  |
| integriert         | AMD Radeon Vega (Cezanne)             | —                                             | Intel Iris Xe (Alder Lake-UP3 GT2) |

> **Das ändert die Voraussetzung von E13.** Der Plan führt dort „CUDA" — und **nur eine der drei
> Maschinen hat NVIDIA**. gonets Radeon kann kein CUDA (ROCm oder Vulkan wären ein anderer
> Weg, mit eigener Abnahme und eigener Paritätsfrage), die Iris Xe ist eine iGPU ohne eigenen
> Speicher. Ein GPU-Backend bleibt damit **eine Maschine mit 4 GB VRAM**, nicht drei.

**Massenspeicher** — für die Simulation fast bedeutungslos, hier der Vollständigkeit halber

|             | Laptop                                | gonet                                          | proxmox                                       |
| ----------- | ------------------------------------- | ---------------------------------------------- | --------------------------------------------- |
| System      | Crucial **CT2000P310SSD2**, 2 TB NVMe | Samsung **870 QVO 4 TB** SATA (QLC)            | Samsung **MZVLQ256HBJD** 256 GB NVMe          |
| Dateisystem | btrfs, zstd:3                         | ZFS `rpool` 3,62 T                             | ZFS `rpool` 236 G                             |
| Belegung    | 83 % (148 GB frei)                    | **93 %** (230 G frei)                          | **83 %** (39,8 G frei)                        |
| weitere     | zram 10 G                             | 7,3 T HDD · 16,4 T USB-HDD · `backuppool` 84 % | `usbpool` 952 G über **ASM2464 (USB4)**, 34 % |

**Warum das für ROTA fast nichts entscheidet:** Der Simulator liest beim Start ein paar
Kilobyte JSON und rechnet danach vollständig im Speicher — gemessen **97 page-faults auf
200 000 Kämpfe** (§ 1.2c). Platte und Dateisystem stehen hier nur, weil beide ZFS-Pools über
**83 %** liegen und ZFS jenseits von 80 % fragmentiert: Ein neuer LXC braucht Platz, und auf
gonet sind nur 230 GB frei.

#### Zusammengefasst: Belastung und Eignung

| System              | Str. | AVX2/FMA | Belastung im Betrieb                           |
| ------------------- | ---: | -------- | ---------------------------------------------- |
| Laptop (Ziel)       |   16 | **ja**   | die Arbeitsmaschine selbst, RAM-Engpass        |
| gonet `10.0.0.106`  |   12 | **nein** | Load ~3,0 dauerhaft: rustbuild, CI-Runner, pki |
| proxmox `10.0.0.69` |   12 | **ja**   | Load ~0,2 — praktisch im Leerlauf              |

**Gemessen** (`sim/examples/orch-verteilung.rs`, dasselbe Binary, 200 000 Kämpfe, feste
Rotation, Aufwärmlauf davor):

| System           |    Kämpfe/s | je Kampf | Load währenddessen  |
| ---------------- | ----------: | -------: | ------------------- |
| Laptop, 16 Str.  |     210 094 |  4,76 µs | 1,96 — **gedrückt** |
| gonet, 12 Str.   |     121 689 |  8,22 µs | 2,75 — gedrückt     |
| proxmox, 12 Str. | **221 688** |  4,51 µs | 0,73 — ruhig        |

> **Der lokale Wert ist nicht die Laptop-Leistung.** Die ruhige Baseline liegt bei
> **350 000–365 000** (`docs/SIM-MESSUNGEN.md`); während dieser Messung waren 12 von 14 GB
> belegt und die Maschine unruhig. Für die Rechnung unten wird die ruhige Baseline benutzt,
> nicht die 210 094 — sonst würde der Zuwachs durch Verteilung systematisch zu groß aussehen.

**Zweiter Befund, und er ist der wichtigste: Die Ergebnisse sind bitgleich — nachgemessen.**

> **KORRIGIERT am 2026-08-14, vierte Runde. Meine erste Messung war kein Beweis.** Sie
> verglich `Summe aller Mittelwerte 102243,0000000000` — also eine **auf zehn Stellen
> formatierte Dezimalzahl**. Zwei `f64`, die sich im letzten Bit unterscheiden, drucken
> dieselbe Zeile. **Genau der Fehlertyp, vor dem dieses Dokument an fünf Stellen warnt: der
> Messweg erzeugt die Antwort.**

**Richtig gemessen**, über `to_bits()` statt über Text: 48 Werte (Mittel, Minimum, Maximum von
Schaden und erlittenem Schaden) über **4 Bosse × 3 Startwerte** mit krummen Faktoren
(`essence 1,137 · talents 1,0813 · gear 0,9437 · buffs 1,2109 · … · crit 1,0757`), jeder als
Bitmuster in einen FNV-1a-Hash:

```
LOKAL       5900HS   Zen 3           0x42210c39918563b9
gonet       i7-3930K Sandy Bridge    0x42210c39918563b9
proxmox     i5-1235U Alder Lake      0x42210c39918563b9
```

**Identisch.** Der Kernel rechnet skalar in `f64` ohne architekturabhängige Umordnung — das
gilt jetzt bitweise, nicht nur bis zur zehnten Nachkommastelle.

**Der naheliegende erste Test wäre in jedem Fall wertlos gewesen:** Bei neutralen Faktoren ist
jeder Kampf identisch (`min == max == 7800`); eine Summe gleicher Werte deckt keine
Rundungsabweichung auf, egal wie man sie vergleicht.

> **Was dieser Nachweis NICHT abdeckt, und das gehört nach E14.** Er gilt für den **heutigen**
> skalaren Pfad und die heutigen Übersetzungsoptionen. Er sagt nichts über einen AVX2- oder
> FMA-Pfad, und er ist keine Zusicherung für die Zukunft. Der numerische Vertrag muss deshalb
> ausdrücklich festschreiben: **kein Fast-Math · keine Umassoziation · keine unkontrollierte
> FMA-Verschmelzung · fester Rundungsmodus · definiertes Verhalten bei NaN, Unendlich und
> subnormalen Zahlen.** Ohne diese fünf Punkte kann ein Übersetzer-Wechsel den Hash ändern,
> ohne dass jemand eine Zeile Code angefasst hat.

**Damit fällt der stärkste Einwand der ersten Fassung weg.** Verteilung erzeugt heute **kein**
Paritätsrisiko. Das ist gemessen, nicht angenommen.

#### Was die Verteilung bringt

```
Laptop ruhig      365 000
proxmox           222 000        + 61 %
gonet             122 000        + 33 %
──────────────────────────
zusammen          709 000        Faktor 1,94   ← SUMME getrennt gemessener
                                              Hostraten, KEIN Verbundlauf
ohne gonet        587 000        Faktor 1,61
```

#### Und woran sie scheitert, wenn man sie zu fein schneidet

Die Netzwerklatenz entscheidet über die Paketgröße:

```
RTT zu gonet      1,33 ms          bei 2,74 µs je Kampf  =  485 Kämpfe
RTT zu proxmox    1,69 ms                                =  617 Kämpfe
```

| Arbeitseinheit                          |  Kämpfe | Rechenzeit | gegen 1,5 ms RTT   |
| --------------------------------------- | ------: | ---------: | ------------------ |
| ein Kandidat                            |       1 |     2,7 µs | **550× Overhead**  |
| eine `greedy_opts`-Runde (Länge 5)      |     528 |     1,4 ms | **1,0×** — sinnlos |
| eine vollständige greedy-Suche          |  ~1 900 |     5,2 ms | 29 %               |
| eine LNS-Suche                          | ~10 000 |      27 ms | 6 %                |
| ein Boss × alle Gear-Phänotypen (heute) | ~56 000 |     153 ms | 1 %                |
| Orakel-Tabelle im Endausbau             | 368 016 |      1,0 s | 0,15 %             |

**Eine ganze Suchrunde ist genau eine Netzwerk-Roundtrip wert.** Wer innerhalb einer Suche
verteilt, verschenkt die Hälfte der gewonnenen Zeit an Latenz — und LNS ist ohnehin sequenziell,
jede Runde hängt an der vorigen.

**Die Verteilung gehört also eine Ebene höher:** ganze Projektionen, Sweeps, Orakel-Tabellen,
Fortschritts-Läufe. Dort sind die Pakete unabhängig und der Netzanteil klein.

> **KORRIGIERT am 2026-08-14, vierte Runde.** Hier stand, es brauche „**kein verteiltes
> Ausführungsmodell** — ein Arbeitspaket-Format, `scp` und `ssh` genügen". **Das gilt nur für
> die Messung, mit der dieser Absatz entstanden ist**, nicht für den Betrieb: Sobald
> LNS-Runden verteilt werden (E15.5), ein Rechner mitten im Lauf ausfällt oder ein Lauf
> fortgesetzt werden soll, braucht es Leases, Heartbeat und deterministische Epochen.
> **Maßgeblich ist E15.5, nicht dieser Absatz.**

#### Die Einordnung, die über die Reihenfolge entscheidet

| Maßnahme                        | Faktor      | Aufwand                  |
| ------------------------------- | ----------- | ------------------------ |
| E11.1 `purity == 0` → ein Lauf  | **50–200×** | eine Bedingung           |
| E11.2 Kernel                    | ~1,66×      | zwei Umbauten            |
| E11.4 inkrementell              | ~1,8×       | großer Umbau             |
| **Verteilung auf drei Rechner** | **1,94×**   | Paketformat + Verteilweg |

**Gemessen an Aufwand und Nutzen ist die Verteilung nicht der beste Hebel** — solange im Plan
noch ein Faktor 50–200 ungehoben liegt, wäre sie die falsche Reihenfolge. Aber sie hat drei
Eigenschaften, die keine andere Maßnahme hat:

- **Sie ändert keine Zeile Simulator-Code** — kein Paritätsrisiko, gemessen bestätigt.
- **Sie ist multiplikativ zu allem anderen.** Nach E11.1 und E11.2 gilt derselbe Faktor 1,94.
- **Sie entlastet die Arbeitsmaschine.** Der 5900HS ist ein Mobilchip mit gemeinsamem
  thermischem Budget; ein Sweep, der ihn eine Stunde auslastet, macht ihn unbenutzbar. Das ist
  kein Durchsatzargument, sondern ein Arbeitsargument.

#### Empfehlung, gestaffelt

**Zuerst `10.0.0.69` allein, als eigener LXC.** Er ist ruhig (Load 0,2), hat AVX2 und FMA, ist
schneller als der belastete Laptop, und 33 GB Plattenplatz sind frei. **+61 % ohne Konflikt.**
Was dort läuft — `claude-wa`, `backup-monitor`, `media-app`, zwei Test-Nodes — ist im Leerlauf;
ein LXC mit begrenzter Kernzahl stört sie nicht.

**gonet erst danach, und mit Vorbehalt.** Er trägt den GitHub-Actions-Runner und rustbuild. Ihn
auszulasten verlangsamt jede CI und jeden Rust-Build — also genau die Arbeit, die währenddessen
läuft. Seine +33 % sind die teuersten des Pakets.

**Nach E11.7 verschiebt sich das Verhältnis weiter zuungunsten von gonet:** Er hat kein AVX2 und
müsste den skalaren Pfad fahren, während die beiden anderen den vektorisierten nehmen. Sein
Anteil sinkt dann relativ — wie stark, ist erst nach E11.7 messbar.

**Nicht in E11.** Die Verteilung ist kein Architekturthema, sondern ein Betriebsthema: ein
Paketformat (`JobKey` aus dem Urteil ist dafür brauchbar), ein Verteilweg, eine
Ergebnis-Einsammlung. Sie gehört als eigenes, kleines Issue **nach** E11.1 und E11.2 — denn
1,94× auf einer Suche, die noch das 50-Fache zu viel rechnet, verteilt vor allem verschwendete
Arbeit.

---

---

## Teil 3 — Das Zielbild: sieben Schichten

_Dieser Teil beschreibt, **was am Ende dasteht**. Die Epics in Teil 4 bauen es Stück für Stück;
wer nur eines davon umsetzt, sollte hier nachsehen, wo es hingehört._

```
  ┌─ Inhalte ────────────────────────────────────────────────────────────┐
  │  data/*.json                                                          │
  │        ↓  Bauzeit                                                     │
  │  typisierte Wirkungs-IR              E16                              │
  │    ├─ Abhängigkeitsmaske   ├─ Kernelsignatur                          │
  │    ├─ Schrankenfähigkeit   └─ Invalidierungskanten                    │
  └───────────────────────────────────────────────────────────────────────┘
          ↓
  ┌─ Erzeugung gültiger Kandidaten ──────────────────────────────────────┐
  │  Talent-DP  ·  Gear-Phänotypen  ·  Rotations-Normalisierung           │
  │  Policy-Klassen  ·  Abdeckungsmatrix          E11.1 · E11.4f · E21.1  │
  └───────────────────────────────────────────────────────────────────────┘
          ↓
  ┌─ Auswertung: der billigste zulässige Weg ────────────────────────────┐
  │  Router  E11.4b                                                       │
  │    Cache → analytisch → Ledger-Differenz → Trace-Region →             │
  │    unverändert → Suffix → AVX/AVX2 → GPU → skalarer Referenzkampf     │
  │  jeder Weg meldet Kosten UND Garantie                                 │
  └───────────────────────────────────────────────────────────────────────┘
          ↓
  ┌─ Suche ───────────────────────────────────────────────────────────────┐
  │  Greedy+Paare → adaptives LNS → Projektionslöser → B&B mit            │
  │  bewiesenen Schranken → Boss-Active-Set → MAP-Elites   E11.5*         │
  └───────────────────────────────────────────────────────────────────────┘
          ↓
  ┌─ Balance (äußere Schleife) ──────────────────────────────────────────┐
  │  Treppenfunktion → Trust-Region → Abdeckungs-Nebenbedingungen →       │
  │  Herausforderer → exakte Abschlussprüfung              E12            │
  └───────────────────────────────────────────────────────────────────────┘
          ↓
  ┌─ Betrieb ────────────────────────────────────────────────────────────┐
  │  cgroup v2 + interner Regler  ·  HostPerfVector  ·  1+n-Verteilung    │
  │  dauerhafter Laufzustand  ·  Ergebnisspeicher  ·  Fortschritt/ETA     │
  │  Engpassdiagnose                                     E15 · E17 · E19  │
  └───────────────────────────────────────────────────────────────────────┘
          ↓
  ┌─ Vertrag und Bedienung ──────────────────────────────────────────────┐
  │  CampaignRunContract mit eingefrorenem Arbeitsbudget  ·  Kampagnenprofile     │
  │  Qualitätszertifikat  ·  rota-sim mit --help          E20 · E21       │
  └───────────────────────────────────────────────────────────────────────┘
```

**Drei Eigenschaften machen dieses Bild aus, und jede ist eine Entwurfsentscheidung:**

**Die Wirkungs-IR steht ganz oben, nicht am Rand.** Sechs Stellen im System entscheiden, was
ein Build tut — Krit-Zulässigkeit, Schranken, Kernelwahl, Ledger-Gültigkeit, Invalidierung,
Ereigniskanäle. Werden sie von Hand gepflegt, driften sie auseinander, und der Fehler ist
still. Deshalb werden sie **erzeugt** (E16).

**Der Router kennt Kosten und Garantie, nicht nur Zulässigkeit.** Ein Weg, der exakt ist, aber
teurer als der Vollkampf, ist der falsche Weg. Und ein Ergebnis ohne Garantieangabe lässt sich
nicht auditieren (Teil 6, Auflage 2).

**Betrieb und Vertrag sind Schichten, keine Zutaten.** Ein Simulator, der die Maschine belegt,
keinen Fortschritt zeigt, nach einem Absturz von vorn beginnt und nur von seinen Erbauern
bedient werden kann, ist im Alltag nicht benutzbar — unabhängig davon, wie schnell sein Kernel
rechnet.

---

## Teil 4 — Die Epics, in Umsetzungsreihenfolge

### 4.0 Die eine Reihenfolge

_Alle Reihenfolge-Aussagen dieses Plans stehen **hier**. Wo ein Issue unten eine eigene
Bedingung nennt, ergänzt sie diese Tabelle — sie ersetzt sie nicht._

> **KORRIGIERT am 2026-08-15: Die Vorfassung verletzte ihre eigene Abhängigkeitstabelle.** Sie
> listete Phase A als „E15.2 · E15.3 · E11.0" — obwohl 4.1 sagt, **E15.2 hängt an E11.0**. Und
> Phase B als „E11.1 · … · E16", obwohl die `StochasticSourceMask`, auf der E11.1 steht, **von
> E16 erzeugt wird**. Eine Reihenfolge, die ihren eigenen Voraussetzungen widerspricht, ist
> schlimmer als keine.

| Phase                                               | Issues                                                                                                                                                                                        | warum in dieser Reihenfolge                                                                                                                                                                              |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A0 — Identitäten, bevor irgendetwas gebaut wird** | **Issue-Registry** · `LogicalTaskId`+WorkUnit-Vertrag · `JobKey v2` · RNG-Verträge (Kampf **und** Suche) · `ResultGuarantee v2` · `RunManifest v3` + Commit-Vertrag · kanonisches Wire-Format | Ohne Einstiegspunkt können die anderen Sitzungen nichts starten; ohne Metrikvertrag misst niemand vergleichbar. **E11.0 vor E15.2**, weil die Anzeige seine Zähler braucht                               |
| **A0b — bedienbar und messbar**                     | **E20** (Grundgerüst) · **E11.0**                                                                                                                                                             |
| **A1 — sehen, bevor man optimiert**                 | E15.2 · E15.3 · E11.0b                                                                                                                                                                        | Fortschritt und Diagnose stehen auf E11.0. Die Lastprofile ebenfalls                                                                                                                                     |
| **B — semantische Grundlage**                       | **E16** · E19 · RNG-Vertrag · **E14.0** (nur die Deckel)                                                                                                                                      | **E16 zuerst**: E11.1, E11.4d, E11.5b, E15.4 und E21.1 beziehen ihre Korrektheit daraus. E19 früh, weil Nachrüsten teuer ist. Die Werte-Deckel früh, weil sie Festkomma, AVX2-Integer und GPU blockieren |
| **C — die kostenlose Nicht-Simulation**             | E11.1 · E11.1b                                                                                                                                                                                | Der Faktor 50–200 liegt hier — **aber erst, wenn E16 sagen kann, welche Zufallsquellen aktiv sind**                                                                                                      |
| **D — Kernel und Zustand**                          | E11.2 · E11.4 · E11.4d                                                                                                                                                                        | 39,9 % der Zyklen, plus der kompakte Zustand                                                                                                                                                             |
| **E — nicht zweimal rechnen**                       | E11.4b · E11.4f · E11.3 · E11.3a · E18                                                                                                                                                        | Router und Cache bündeln, was B–D erzeugt haben. E11.3 **misst**, bevor E11.3a **beweist**                                                                                                               |
| **F — Betrieb**                                     | E11.4c · E15.0 · E15.1 · E15.4 · E17 · E15.5 · E21.7                                                                                                                                          | Erst jetzt lohnt Verteilung: vorher würde sie verschwendete Arbeit verteilen                                                                                                                             |
| **G — Suche und Balance**                           | E11.5 · E11.5a–c · **E11.5d** · E21.0 · E21.0b · E21.1–E21.4 · **E12**                                                                                                                        | Setzt die Vorgaben aus 6.1 voraus — sie stehen seit dem 2026-08-17                                                                                                                                       |
| **H — rohe Leistung**                               | E11.6 · E11.7 · E11.7b · E11.7c · **E14 vollständig** · E13 · **E15.0b** (hängt an E13)                                                                                                       | Jeder Faktor hier ist kleiner als jeder in B–E                                                                                                                                                           |
| **Z — die einzige Abnahme, die zählt**              | **E11.0c**                                                                                                                                                                                    | Ganz zuletzt: Es misst eine **vollständige Kampagne** über alle Schichten. Es hängt damit an E11.0, E11.4b/c, E13, E15.0–E15.5, E17, E19 und E21 — nicht nur an E11.0 und E20                            |
| **fortlaufend**                                     | E11.4e · E21.6                                                                                                                                                                                | Fortschritts-Simulation, sobald der Ergebnis-Cache steht                                                                                                                                                 |

**Die Rangfolge folgt aus den Faktoren, nicht aus Vorlieben:**

> **Die Faktoren stehen mit ihrem Geltungsbereich im Kopf des Dokuments** („Was dieser Plan
> zusichert") und werden hier bewusst **nicht wiederholt** — eine zweite Liste ohne die
> Einschränkungen war in der Vorfassung genau die Stelle, an der sie wieder absolut aussahen.
>
> Kurz: **50–200×** nur bei redundanten Durchgängen ohne aktive Zufallsquelle · **1,7×** nur am
> Greedy-Einfügeoperator · **Kernel-Abnahmeziel 1,29–1,34×** (1,66× ist die theoretische
> Obergrenze) · **Suffix überlappt** mit „ändert nichts" · **1,94×** ist eine projizierte Summe,
> kein Verbundlauf.

### 4.1 Woran jedes Issue hängt

_Die Phasen oben sagen **wann**, diese Tabelle sagt **warum**. Ein Issue ohne erfüllte
Voraussetzung wird nicht vorgezogen — die Voraussetzung ist der Grund für die Phase._

| Issue                           | hängt an                                                 | Anmerkung                                                                                                                                    |
| ------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| E11.0 · E11.0b                  | —                                                        | Einstieg                                                                                                                                     |
| E11.1 · E11.1b                  | E11.0 · **E16** (`StochasticSourceMask`) · RNG-Vertrag   | „keine aktive Zufallsquelle" ist eine ABGELEITETE Bedingung                                                                                  |
| E11.2                           | E11.0b (Kapazität aus dem Lastprofil)                    |                                                                                                                                              |
| E11.3 → E11.3a                  | E11.0                                                    | E11.3 **misst**, E11.3a **beweist**                                                                                                          |
| E11.4                           | E11.2                                                    | enthält `SearchFight`/`AuditFight`                                                                                                           |
| E11.4b                          | E11.4d (Kostenschätzung), E19                            | Router braucht die Cachetrennung                                                                                                             |
| E11.4c                          | —                                                        | wird von E15.1 und E15.5 vorausgesetzt                                                                                                       |
| E11.4d                          | **E16**                                                  | die Abhängigkeitsmaske wird erzeugt, nicht gepflegt                                                                                          |
| E11.4e                          | E11.4b (die Orakel-Tabelle **ist** der Cache)            | → E21.6 löst sie exakt                                                                                                                       |
| E11.4f                          | E11.4                                                    | lohnt erst nach dem „ändert nichts"-Zweig                                                                                                    |
| E11.5                           | Zielvertrag und Nischen-Achsen aus 6.1                   | beide entschieden, kein Blocker mehr                                                                                                         |
| E11.5a                          | E11.5                                                    |                                                                                                                                              |
| E11.5b                          | **E16** (`UpperBoundCapability`)                         | ohne sie ist die Schranke unsicher                                                                                                           |
| E11.5c                          | Encounter-Aggregation (Teil 6)                           |                                                                                                                                              |
| **E11.5d**                      | E11.5b · E11.4f · **E16**                                | exakter APL-Löser — **Voraussetzung für jedes `CertifiedOptimal`/`CertifiedNecessary`**                                                      |
| E11.6 → E11.7 → E11.7b → E11.7c | E11.0                                                    | jede Stufe misst gegen dieselbe Baseline                                                                                                     |
| E13                             | **E14**, E11.7                                           | ohne Festkomma ist der GPU-Pfad offen                                                                                                        |
| E14                             | Werte-Deckel (Teil 6)                                    | **Leistungs-Engpass**, nicht Detail                                                                                                          |
| E15.0                           | —                                                        | liefert `HostPerfVector` für alles Verteilte                                                                                                 |
| E15.1                           | E11.4c                                                   | Stränge parken statt beenden                                                                                                                 |
| E15.2                           | E11.0 (Metriken)                                         |                                                                                                                                              |
| E15.3                           | E15.2                                                    |                                                                                                                                              |
| E15.4                           | E11.4b (`StructuralKey`)                                 |                                                                                                                                              |
| E15.5                           | E15.0, E11.4c                                            |                                                                                                                                              |
| E16                             | —                                                        | **Voraussetzung für E11.4d, E11.5b, E15.4, E21.1**                                                                                           |
| E17                             | E15.5 (Leases zurückholen)                               |                                                                                                                                              |
| E18                             | E11.3a, E11.5b                                           |                                                                                                                                              |
| E19                             | —                                                        | **je später, desto teurer**                                                                                                                  |
| E20                             | —                                                        | die anderen Sitzungen brauchen es früh                                                                                                       |
| E21.0                           | —                                                        | **hardwareunabhängig**: das Budget kommt aus dem Referenzprofil, nicht aus der Kalibrierung. Nur **E21.0b** braucht E15.0                    |
| E21.1                           | **E16**                                                  | welche Wechselwirkungen zählen überhaupt                                                                                                     |
| E21.2                           | E11.4b, E11.5a                                           |                                                                                                                                              |
| E21.3                           | **E12.6** · E21.1 · E11.5c                               | aggregiert die `ViabilityEvidence` aus E12.6 — **kein Zyklus**: E12.6 hängt NICHT an E21.3                                                   |
| E21.4                           | E20                                                      |                                                                                                                                              |
| E21.6                           | E11.4e                                                   |                                                                                                                                              |
| E21.7                           | E19, E15.4                                               |                                                                                                                                              |
| **E11.0c-core**                 | E11.0 · E11.4b/c · E15.0–E15.5 · E17 · E19 · E21.0–E21.7 | **PFLICHT.** CPU-only · ein und drei Rechner · Drosselung · Rechnerausfall · Fortsetzen · Cache kalt und warm                                |
| **E11.0c-gpu**                  | E11.0c-core · **E13**                                    | **BEDINGT** — nur wenn E13 seinen Break-even-Vertrag bestanden hat. Eine nie rentable GPU darf die Standardkampagne nicht unabnehmbar machen |
| **E14.0–E14.6**                 | Werte-Deckel (6.1)                                       | ausgeschrieben in Teil 4.5                                                                                                                   |
| **E15.0b**                      | E15.0, E13                                               | gemeinsame CPU-/GPU-Kalibrierung                                                                                                             |
| **E15.3b**                      | E15.3                                                    | hostrelative Diagnoseschwellen                                                                                                               |
| **E16.1**                       | E16                                                      | vollständige `EffectNode`-Semantik statt Opcode-Liste                                                                                        |
| **E19.1**                       | E19                                                      | `TrialAccumulator` — Durchgänge ergänzbar statt fertig                                                                                       |
| **E21.0b**                      | E21.0                                                    | **Machbarkeitsprüfung**: passt die Pflichtarbeit ins Budget?                                                                                 |
| **E21.1b**                      | E21.1                                                    | unabhängiger Abdeckungs-Verifizierer                                                                                                         |
| **E21.3b**                      | E21.3                                                    | campaignweite statistische Fehlerkontrolle                                                                                                   |

**E16 taucht viermal als Voraussetzung auf.** Das ist kein Zufall: Es ist das einzige Bauteil,
von dem vier andere ihre Korrektheit beziehen — und ohne das die Fehler still werden.

### 4.2 Umfang aller Epics

Nach dem Muster der Epic-Tabelle in `greedy-skipping-russell.md`:

| Epic                             | Issues | Inhalt                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | Reihenfolge                                                                                                                                                                                                                                                                                                                     |
| -------------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **E11 — Simulator-Architektur**  | **23** | (0) Messumgebung mit Boss-Suite und Metriken · (0b) **Endausbau-Stellvertreter** · (1) kostenlose exakte Gewinne · (1b) Crit-Verteilungs-DP + gestaffelte Stichprobe · (2) Kernel: `expire_buffs` und Ereignisschlange · (3) Trace-Stabilität messen · (3a) Breakpoint-Compiler und Trace-Regionen · (4) inkrementelle Auswertung + `SearchFight`/`AuditFight` · (4b) Evaluator-Router mit Kostenmodell · (4c) Produktions-Scheduler · (4d) `CompiledBuild` + Abhängigkeitsmaske · (4e) Fortschritts-Simulation · (4f) Policy-Äquivalenz · (5) LNS · (5a) adaptive Operatoren · (5b) exakte Löser + B\&B-Orakel · (5c) Boss-Active-Set · (6) Compiler und PGO · (7) AVX2 · (7b) Kernel-Spezialisierung | nach E2; (5) und (5c) zusätzlich nach den Design-Entscheidungen                                                                                                                                                                                                                                                                 |
| **E12 — Balance-Optimierer**     | **10** | **Ausführlich in Teil 4.4.** (0) Ziel und Nebenbedingungen · (1) parametrische Trace-Zellen · (2) minimaler Buff über die inverse Treppenfunktion · (3) blockweise Trust-Region · (4) Warm-Start der inneren Suche · (5) Boss-Constraint-Generation · (6) Nischenverlust und tote Zauber · (7) Herausforderer-Phase · (8) exakte Abschlussprüfung                                                                                                                                                                                                                                                                                                                                                      | nach E11.5                                                                                                                                                                                                                                                                                                                      |
| **E13 — GPU-Bulk-Auswerter**     | offen  | Ledger-Stapel, Kandidat × Boss, Monte-Carlo-Finalisten. **Backend-Grenze offen halten** (`Cuda \| VulkanCompute \| None`), nicht auf CUDA festlegen: nur der Laptop hat NVIDIA (RTX 3050 Ti, 4 GB, cc 8.6), gonets Polaris und proxmox' Iris Xe kämen allenfalls über Vulkan-Compute in Frage — zu entscheiden **durch Kalibrierung, nicht durch Festannahme**. **Hängt an E14**: ein exakter GPU-Pfad in `f64` ist auf einer mobilen Consumer-GPU fraglich; mit dem Festkomma-Vertrag wird er wahrscheinlicher. **Drosselung über kurze Batches und Einschaltdauer**, nicht über Stromprioritäten — die sind nur ein Hinweis und unterbrechen laufende Arbeit nicht                                   |                                                                                                                                                                                                                                                                                                                                 | nach E11.7 **und** einer Break-even-Messung                       |
| **E15 — Betriebsarchitektur**    | **9**  | _(inkl. E15.0b gemeinsame CPU-/GPU-Kalibrierung und E15.3b hostrelative Schwellen)_                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | (0) Fähigkeitserhebung mit Kalibrierlauf · (1) Ressourcen-Balancer, Drosselung ohne Abbruch · (2) Fortschritt, Durchsatz, zweigeteilte ETA · (3) Engpasserkennung mit Diagnoseregeln · (4) Invalidierungs-Matrix · (5) Verteilung 1 + n mit Ausfall und Ausschluss · (6) verzweigungsfreie Auswahl. **Ausführlich in Teil 4.5** | (2)+(3) **vor** den großen E11-Umbauten; (0)+(5) nach E11.1/E11.2 |
| **E14 — Numerischer Vertrag**    | offen  | Fixed-Point, Bereichsbeweis, Rundungsregel. **Entscheidet auch, wie die Wahrscheinlichkeitsmasse der Crit-DP geführt wird** (E11.1b) und ob ein exakter GPU-Pfad möglich ist (E13)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | vertagt (6.1) — die Deckel-Entscheidung ist getroffen, der Bedarf entsteht erst mit der Last                                                                                                                                                                                                                                    |
| **E16 — Wirkungs-IR**            | **3**  | typisierte Effekt-Opcodes · daraus abgeleitete Abhängigkeits-, Kernel-, Schranken- und Invalidierungsanalysen · ein neuer Opcode **bricht den Bau**, bis er überall eingeordnet ist. **Ausführlich in Teil 4.6**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | **vor** E11.4d und E11.5b                                                                                                                                                                                                                                                                                                       |
| **E17 — Lauf-Zustand**           | **3**  | Manifest und Journal · Anhalten an sicheren Punkten · Fortsetzen mit Bitgleichheits-Nachweis nach `SIGKILL`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | sobald ein Lauf über eine Stunde dauert                                                                                                                                                                                                                                                                                         |
| **E18 — Nicht-Simulation, Rest** | **2**  | sparsame Ledger-Differenz (`Parameter → Terme`) · frühe Schranken **im** Kampf, boss- und zielabhängig                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | nach E11.3a und E11.5b                                                                                                                                                                                                                                                                                                          |
| **E19 — Cachetrennung**          | **1**  | `CombatOutcome` / `Score` / `SearchArchive` getrennt — macht die offene Objective-Entscheidung billig                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | **früh**, je später desto teurer                                                                                                                                                                                                                                                                                                |
| **E20 — Kommandozeile**          | **4**  | ein Einstiegspunkt statt 30 Beispiele · `--help` mit lauffähigen Beispielen · `explain` als Trockenlauf · `verify` als Selbsttest                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | **früh** — die anderen Sitzungen brauchen es                                                                                                                                                                                                                                                                                    |
| **E21 — Laufzeitvertrag**        | **9**  | (0) `CampaignRunContract`, Arbeitsbudget einfrieren statt Zeit abschneiden · (1) beschränkte t-Wege-Abdeckung statt „Rest bleibt Stichprobe" · (2) globaler Budgetverteiler + Stufenprüfung · (3) Qualitätszertifikat · (4) Kampagnenprofile, Vollraum ablehnen · (6) Markov/MDP-Fortschritt · (7) dauerhafter Ergebnisspeicher über Läufe. **Ausführlich in Teil 4.7**                                                                                                                                                                                                                                                                                                                                        | **beantwortet die Betriebsfrage** — nach E15.0, parallel zu E11                                                                                                                                                                                                                                                                 |

**E12 ist der wichtigste dieser drei**, und er fehlte in meiner ersten Fassung ganz. E11 macht
den Simulator schnell und die **innere** Buildsuche gut. Es beantwortet aber nicht die Frage,
für die das ganze Werkzeug gebaut wird:

> Wie viel Grundschaden braucht Zauber X, damit er irgendwo die beste Wahl ist — und welche
> Zahl zerstört dabei Zauber Y?

**Und das ist keine Formulierung dieses Plans, sondern eine Zusage des Auftraggebers.** Sie steht
im Plan unter dem, was er im Interview mehrfach betont hat (C.1c Punkt 2):

> „**Jeder Zauber muss irgendwo die beste Wahl sein** — keine 100 Zauber, von denen 10 gespielt
> werden."

Der gemessene Ausrüstungskollaps verletzt sie: Bei Stufe 85 legendary bleibt von 58 Zaubern
**einer** lohnend. **E12.6 ist damit das Abnahmekriterium des ganzen Epics**, nicht sein letzter
Punkt. Daneben gehört die zweite prüfbare Zusage aus § 11.3 — _„Kein Boss verlangt eine bestimmte
Schule"_: Für jede Mechanik zählen, wie viele Zusammenstellungen sie lösen. Beide sind messbar,
und heute misst sie niemand.

Das ist eine zweistufige Optimierung: außen die Balance-Parameter, innen die beste Rotation
zu jedem Parametersatz. Ohne E12 entsteht ein schneller Simulator, aber kein
Balancing-System.

**Und die Schleife lässt sich jetzt ausbuchstabieren**, weil Anhang B.2.3 die Form der Zielfunktion
belegt hat. Für **einen** Zauberparameter:

```
Zauber in seiner Nische erzwingen
        ↓
beste Rotation und Build dazu suchen              (E11.5, warm gestartet)
        ↓
Trace-Region dieses Ergebnisses bestimmen         (E11.3a)
        ↓
minimalen Wert INNERHALB der Region analytisch lösen
        D(x) = C + Σ nᵢ·round(aᵢx + bᵢ)  ist monoton und ganzzahlig
        → Binärsuche über 10–20 Ledger-Auswertungen
        ↓
Lösung liegt am Regionsrand?  → lokale Buildsuche neu starten, nächste Region
```

statt `Wert 100 simulieren · 101 simulieren · 102 simulieren · …`

**Der Ordnungsgedanke dahinter** (Punkt 14 des Urteils, richtig): Es gibt keinen
Universal-Löser. Jede Projektion bekommt das Verfahren, das zu ihr passt — die Tabelle in
E11.5b **ist** diese Zuordnung. Ein eigener „Planer" als Bauteil ist dafür nicht nötig: Die
Zuordnung ist statisch und gehört in den Router aus E11.4b.

**E13 wird vorbereitet, nicht gestrichen.** Der Vorschlag hat recht, dass „`nvcc` fehlt" ein
Setup-Zustand ist und kein Architekturgrund. Deshalb bekommt E11.4b von Anfang an eine
Backend-Grenze:

```rust
trait EvaluatorBackend {
    fn evaluate(&mut self, batch: &[CompactCandidate], out: &mut [Ergebnis]);
}
// ScalarCpuBackend · Avx2CpuBackend · (später) CudaBackend
```

Damit `CompactCandidate` GPU-tauglich bleibt: keine Zeiger, keine Zeichenketten, kein `Vec`,
feste Offsets. Das kostet in E11 fast nichts und spart später einen Umbau.

**Nicht enthalten und bewusst nicht:** APL-Mikrooptimierung (4,1 % der Zyklen) und ein
gemeinsamer WASM-Kern — der würde die Paritätsprüfung nicht lösen, sondern abschaffen.

---

---

### 4.3 E11 — Simulator-Architektur

### E11.0 — Belastbare Messumgebung

**Warum zuerst:** Ohne sie ist jede spätere Abnahme wertlos. Heute streut die Baseline um
±12 %, weil die Maschine nie ruhig ist — ein Umbau, der 10 % bringt, wäre nicht nachweisbar.

```
Läufe ≥ 20 s statt 1–4 s
CPU-Affinität, feste Kernzahl
Lastprotokoll (vmstat) parallel, im Ergebnis mitgedruckt
Abbruch nach DRUCK, nicht nach Load Average:
  CPU-/Memory-/IO-PSI · Taktdrosselung · Kontextwechsel · Steal Time
  Ein-/Auslagerung · Stabilität der Rate über die Messdauer
(Load Average zählt auch I/O-Wartende und mittelt über eine Minute —
 er meldet Druck, wenn er vorbei ist. Begründung in E15.1.)
Referenz-Trace-Hash je Boss, damit „gleiches Ergebnis" prüfbar ist
```

**Fertig, wenn:** über **7–10 A/B-Wechselläufe** der Median um weniger als **3 %** streut, und
der Läufer bricht ab, wenn die Maschine unruhig ist. Zwei Läufe genügen nicht — sie können
zufällig übereinstimmen.

**Eine Voraussetzung, die heute fehlt und die niemand vermisst, bis sie gebraucht wird:** Die
Queue-Instrumentierung (`ORCH_QLEN`, `queue_pushes` in `fight.rs`) ist **nicht mehr im Code**.
Ohne sie liefert auch der Prüfboss aus E11.0b nur Durchsatz — nicht die Verteilung, die über die
`INLINE`-Kapazität entscheidet. **Sie gehört in dieses Issue**, zusammen mit einem Zähler für
`expire_buffs`-Treffer.

**Die Suite misst acht Lasten, nicht einen Boss:** nur Schaden · verstärkungslastig ·
brandlastig · kanallastig · HP-Phasenboss · Überleben/Unterbrechen · ein vollständiger
Greedy-Lauf · ein vollständiger LNS-Lauf. Bewertet wird das geometrische Mittel, nicht der
schnellste Einzelfall — sonst optimiert man auf `boss-1-1` und misst nie, was die Suche
wirklich kostet.

**Und der Durchsatz wird ab hier mehrdimensional geführt.** „Kämpfe/s" allein kann eine
algorithmische Verbesserung um Faktor 50 unsichtbar machen:

| Metrik                  | wozu                                             |
| ----------------------- | ------------------------------------------------ |
| `full_fights/s`         | roher Kernel — die einzige Zahl, die E11.2 misst |
| `resolved_candidates/s` | inklusive Cache, analytisch, Ledger, Suffix      |
| `cache_hit_rate`        | ob die Kanonisierung trägt                       |
| `full_fight_ratio`      | welcher Anteil überhaupt noch teuer ist          |
| `time_to_best`          | Suchqualität gegen Wanduhr                       |
| `verified_elites/hour`  | das eigentliche Balance-Ziel                     |

Die letzte Zahl entscheidet, ob das Werkzeug praktisch funktioniert. Die erste entscheidet nur,
ob der Kernel schnell ist.

### E11.0b — Zwei Endausbau-Lastprofile, nicht eines

> **KORRIGIERT am 2026-08-14, dritte Runde.** Hier stand „ein Prüfboss, der alle **sieben**
> Zusatz-Systeme nutzt" — die Zahl war schon oben korrigiert (**elf**), die Folgestelle nicht.
> Und ein einziges Profil vermischt zwei verschiedene Fragen: _„Was kostet, was heute gebaut
> ist, wenn man es voll ausreizt?"_ und _„Was kostet, was noch kommt?"_

**`max-concurrency-current`** — alle **neun** heute implementierten Grundbausteine gleichzeitig,
dazu volle Leistenlänge, maximal gleichzeitige Brände, Kanäle, Verstärkungssammlungen,
Schutzschichten, `timelineLoopMs`, `escalation`, Phasenwechsel, `bossHeal`, Widerstände.
**Er ist baubar und deshalb der verbindliche Prüfstein** für jede Kapazitätsentscheidung.

**`max-concurrency-future`** — zusätzlich synthetische Stellvertreter für: Varianten der neun
Bausteine (§ 22 des Designs nennt 15–20), Satz- und Legendär-Effekte, Overdrive, Spellsteal
und **eine Boss-Signaturregel**. Kein Spielinhalt, sondern ein **Kapazitäts- und
Regressionstest** — er darf langsamer sein, er darf nicht überlaufen.

> **Zwei Profile genügen nicht — es sind drei.** Der gemessene Faktor 1,40 (Teil 6.2) beantwortet
> nur: _was kosten zehn **Boss**-Systeme?_ Der Spieler wirkte dabei nichts. Die eigentliche
> Frage des Endausbaus ist eine andere:
>
> ```
> boss-max       maximale Bossereignisse, Spieler minimal      ← existiert (orch-maxload)
> player-max     maximale Spielermechanik (16–20 Zauber, DoTs, Kanäle,
>                Verstärkungen, Schutz, Satz, Legendär, Overdrive),
>                Boss minimal                                   ← FEHLT
> combined-max   beides gleichzeitig, über die VOLLE Kampfdauer ← FEHLT
> ```
>
> **`combined-max` ist der einzige, der die Endausbau-Frage wirklich beantwortet** — und er muss
> so ausbalanciert sein, dass **weder Spieler noch Boss früh sterben**, sonst misst er wieder
> die Kampflänge.

> **`max-concurrency-current` EXISTIERT seit 2026-08-14** als
> `data/bosses/orch-maxload.json` — zehn gleichzeitige Systeme gegen höchstens vier bei echten
> Bossen. **Erste Messung: Faktor 1,40× langsamer** als `boss-1-1` (77 058 gegen 108 250
> Kämpfe/s).
>
> **Zwei Lehren aus dem Bau, die für jeden weiteren Prüfboss gelten:**
> Die erste Fassung tötete den Spieler in Sekunden und lief dadurch **schneller** als ein
> normaler Boss — ein Lastprofil muss auf Schadenswert 1 heruntergesetzt werden, sonst misst es
> die Kampflänge statt der Ereignisdichte. Und `escalation` wirkt **nur** mit gesetztem
> `timelineLoopMs` (der Kommentar in `boss-1-1.json` sagt es); ein Prüfboss ohne Schleife hätte
> ein System still nicht ausgelöst.

**Die Profile bilden eine Matrix, keine Liste** — zwei Achsen, bis zu sechs Prüffälle:

```rust
enum LoadAxis      { BossMax, PlayerMax, CombinedMax }
enum ContentTarget { CurrentImplemented, TargetV1 }
```

**Der verbindliche Kapazitätstest ist `TargetV1 × CombinedMax`.** Er darf keinen Überlauf,
keine Speicherexplosion und keinen semantischen Rückfall auslösen.

**Fertig, wenn:** an **allen gebauten Profilen** stehen: Queue-Verteilung (p50/p95/p99/max),
Überläufe je Kapazität (32/48/64), `expire_buffs`-Trefferquote — **und je Profil getrennt**:

```
cycles/event · events/fight · cycles/decision · decisions/fight · full fights/s
```

> **Die letzte Zeile ist der Grund für die getrennte Ausweisung.** Ein komplexerer Boss **darf**
> weniger Kämpfe je Sekunde schaffen, ohne dass der Kernel schlechter geworden ist. Wer nur
> `fights/s` vergleicht, meldet einen Rückschritt, wo mehr Spielinhalt entstanden ist.

**Voraussetzung, die heute fehlt:** Die Queue-Instrumentierung (`ORCH_QLEN`, `queue_pushes` in
`fight.rs`) existiert nicht mehr. **Sie gehört in E11.0** — ohne sie liefert der Prüfboss nur
Durchsatz, nicht die Verteilung, die über die Kapazität entscheidet.

**Das Lastprofil ist nicht erfunden — es steht im Plan, C.1c Punkt 7**, unter dem, was der
Auftraggeber im Interview **mehrfach betont** hat:

> „**Leistung darf nicht leiden** — auch bei 20 Tasten, vielen DoTs und allem gleichzeitig."

Genau das ist der Prüfboss. Er ist keine Erfindung dieses Plans, sondern eine Zusage, die schon
gegeben wurde und bisher nicht gemessen wird.

### E11.0c — Die End-to-End-Kampagnenmessung

**Alle Faktoren dieses Plans sind Einzelmessungen.** Kernel, Cache, Suffix, Verteilung — jeder
für sich gemessen, keiner im Zusammenspiel. **Die Zahl, die der Auftraggeber tatsächlich
erlebt, ist eine andere:**

```
End-to-End-Beschleunigung = alte Kampagnenzeit / neue Kampagnenzeit
```

**Der Abschnitt zerfällt in zwei Issues** — eine GPU, die den Break-even nie erreicht, darf die
Standardkampagne nicht unabnehmbar machen:

#### E11.0c-core — PFLICHT

```
Greedy-Lauf · LNS-Lauf · Standardkampagne
CPU-ONLY · ein Rechner UND drei Rechner
mit Drosselung · mit einem absichtlichen Rechnerausfall
mit Coordinator-Neustart · mit Fortsetzung nach Abbruch
Cache kalt UND warm
```

> **Zwei verschiedene Dinge heißen hier „warm" — und nur eines darf dieselbe Aufgabenmenge
> fordern.**
>
> ```
> Evaluator-Cache warm/kalt   gleicher InitialSearchState → GLEICHE LogicalTaskIds,
>                             gleiches Ergebnis, nur die Wanduhr unterscheidet sich
>
> Sucharchiv warm (E12.4)     alte Elites, Trace-Regionen, Boss-Active-Sets als
>                             Warm-Start → die Kandidatenfolge ist ABSICHTLICH anders.
>                             Der Archiv-Root gehört dann in die CampaignDefinition,
>                             verglichen werden Zeit, Qualität und Garantie
> ```
>
> Ohne diese Trennung fordert der Plan gleichzeitig, dass der Warm-Start die Suche **verändert**
> und dass er **dieselbe** Aufgabenmenge erzeugt.

#### E11.0c-gpu — BEDINGT

Nur wenn E13 seinen Break-even-Vertrag bestanden hat: CPU gegen GPU gegen CPU+GPU,
GPU-Ausfall, CPU-Rückfall.

**Fertig, wenn** — und die Wanduhr allein genügt nicht:

```
dieselbe CampaignDefinition und derselbe CampaignRunContract
dieselbe LOGISCHE Aufgabenmenge in allen Varianten
dasselbe Qualitätszertifikat
dieselben Elites und Pareto-Fronten
dieselben ResultGuarantees
        und ERST DANN: die Wanduhr-Differenz
```

> **Der Nachweis, dass alle Varianten dieselben logischen Aufgaben erzeugen, IST die
> Abnahme.** Eine schnellere Wanduhr bei anderem Suchpfad wäre kein Fortschritt, sondern ein
> anderes Ergebnis.

> **Dieses Issue ist der Prüfstein für die 2–4-Stunden-Zusage.** Solange es nicht gelaufen ist,
> bleibt sie ein Ziel-SLA (siehe Kopf).

### E11.1 — Die kostenlosen exakten Gewinne

Vier Maßnahmen, keine ändert eine Spielregel, alle sind belegt:

| #   | Maßnahme                                                           | Beleg                                                             | erwarteter Gewinn                         |
| --- | ------------------------------------------------------------------ | ----------------------------------------------------------------- | ----------------------------------------- |
| 1   | **keine Zufallsquelle aktiv** → ein Lauf statt 50–200              | gemessen: 0,0 Würfe je Kampf bei `purity = 0`                     | Faktor 50–200, solange die Bedingung gilt |
| 2   | Analytischer Krit, **wo `crit_affects_control_flow()` falsch ist** | heute 15 von 17 — das ist eine Momentaufnahme, **kein Kriterium** | ersetzt Monte-Carlo dort vollständig      |
| 3   | `requires` im Talent-**Generator** statt als Filter                | gemessen: 93,3 % des Rohraums                                     | keine ungültige Verteilung mehr erzeugt   |
| 4   | Gear-Phänotypen beim Start vorkompilieren                          | 56 heute, 5 412 im Endausbau                                      | passt in ein dichtes Array                |

**Maßnahme 1 ist der größte Einzelgewinn im ganzen Epic** und kostet eine Bedingung. Sie steht
zuerst.

> **Aber die Bedingung darf nicht `purity == 0` heißen.** Heute ist der kritische Treffer die
> einzige Zufallsquelle — deshalb stimmt die Abkürzung. Sobald ein legendärer Effekt, eine
> Boss-Signatur oder ein neuer Zauber würfelt, wäre `if purity == 0 { trials = 1 }` **still
> falsch**: Der Läufer würde einen Durchgang rechnen, wo er hunderte bräuchte, und niemand
> bemerkt es.
>
> **Richtig ist eine abgeleitete Maske**, die E16 aus der Wirkungs-IR erzeugt:
>
> ```rust
> StochasticSourceMask { crit, proc, target_selection, random_damage,
>                        boss_randomness, legendary_randomness, … }
>
> if compiled.stochastic_sources.is_empty() { one_trial(); }
> ```
>
> `purity == 0` ist dann ein **Sonderfall innerhalb dieser Ableitung**, kein Kriterium.
> Ein neuer Zufalls-Opcode erzwingt über das vollständige `match` in E16 einen
> Übersetzungsfehler, bis jemand ihn eingeordnet hat.

**Fertig, wenn:** ein Suchlauf mit `purity = 0` **bitgleich dasselbe** liefert wie vorher, bei
1/50 bis 1/200 der Kampfzahl. Die Trace-Hashes aus E11.0 sind der Nachweis.

### E11.1b — Was bleibt, wenn der Krit wirklich würfelt

**Warum dieses Issue existiert, obwohl Maßnahme 1 den Krit heute abschafft:** `purity = 0` gilt
für die **neutralen Faktoren** der Balance-Läufe. Sobald gegen einen realen Ausrüstungsstand
gerechnet wird — und das ist der Zweck der Fortschritts-Simulation (E11.4e) — ist `purity > 0`
und der Zufall zurück. Im Endausbau, mit sechs Sets und legendären Effekten, ist er der
Normalfall, nicht die Ausnahme.

**Der Plan bisher:** analytischer Erwartungswert an den Bossen ohne HP-Phase, Monte-Carlo für
den Rest. Das externe Urteil zeigt zu Recht, dass der Rest nicht Monte-Carlo sein muss.

**(a) Exakte Verteilungs-DP statt Stichprobe.** Bei einem HP-Phasen-Boss beeinflusst der
kumulierte Schaden den Kontrollfluss. Statt zu würfeln, wird die **Verteilung** geführt:

```
Zustand:  (Kontrollzustand, Phase, kumulierter Schaden bis zur nächsten Schwelle)

je Treffer:   P'(d + d₀) += (1-p)·P(d)          normal
              P'(d + d₁) += p·P(d)              kritisch

Schwelle überschritten → dieser Anteil der Wahrscheinlichkeitsmasse wechselt die Phase
gleiche Zustände werden zusammengeführt   — ABER siehe Kasten
```

> **„Gleicher Zustand" muss den Zufalls-Fortsetzungszustand einschließen.** Zwei Pfade können
> denselben sichtbaren `ControlState` erreichen und **verschiedene Ordinalzähler** tragen — dann
> ziehen sie beim nächsten Ereignis verschiedene Würfe. Vorher zusammengelegt, ist die DP
> **nicht mehr exakt**:
>
> ```rust
> DistributionStateKey { control_state, phase, damage_bucket, rng_continuation_state }
> ```
>
> Alternativ ist zu **beweisen**, dass alle künftigen Zähler aus dem zusammengeführten
> Kontrollzustand ableitbar sind. **Und robuster als ein globales `event_ordinal` ist ein
> quellenspezifischer Zähler:** Ein eingefügtes, für diese Quelle belangloses Ereignis darf
> nicht alle folgenden Würfe verschieben.

**Und n identische Treffer werden gruppiert statt einzeln entfaltet:** Die Zahl der Krits ist
binomialverteilt, also genügen `n+1` Fälle statt `2ⁿ` Folgen. Für mehrere Trefferguppen werden
die erzeugenden Funktionen `((1-p)·z^d₀ + p·z^d₁)ⁿ` gefaltet.

> **Das gilt nur, wenn die Treffer AUSTAUSCHBAR sind — und bei HP-Phasen sind sie es nicht.**
> Zwei Folgen mit je genau einem Krit können verschieden verlaufen:
>
> ```
> Krit auf Treffer 1  →  HP-Schwelle früher überschritten  →  Phase wechselt VOR Treffer 2
> Krit auf Treffer 3  →  Treffer 2 läuft noch in der alten Phase
> ```
>
> Gleiche Kritzahl, **verschiedener Kontrollfluss**. Die Gruppierung wäre dann eine stille
> Näherung, die sich „exakt" nennt.
>
> ```rust
> ExchangeabilityCertificate {
>     same_transition_except_reward,
>     no_phase_or_kill_check_between_hits,
>     no_crit_triggered_effect,
>     no_resource_or_timing_feedback,
>     no_interleaving_observable_event,
>     same_target_and_mitigation,
> }
> ```
>
> **Nur mit diesem Nachweis darf nach Kritzahl gruppiert werden** — er wird aus E16 abgeleitet,
> nicht angenommen. Sonst wird die Gruppe **an jeder beobachtbaren Grenze geteilt**, oder es
> läuft die sequenzielle Verteilungs-DP. „Identischer Schaden" allein genügt nicht.

> **Endausbau-Vorbehalt, und er dreht die Begründung des Urteils um.** Das Urteil argumentiert:
> _„Da nur zwei echte Bosse HP-Phasen besitzen, ist ein spezialisierter exakter Algorithmus
> hier sehr attraktiv."_ **Die 2 von 17 ist eine Bestandszahl** — 51 der 68 geplanten Bosse
> sind nicht entworfen, und `phases` ist eines der **elf** Zusatz-Systeme, die der Plan
> ausbauen will. Der DP wird also **nicht** deshalb gebraucht, weil der Fall selten ist,
> sondern weil er **häufig werden wird**. Genau deshalb darf er nicht über eine Bossliste
> aktiviert werden, sondern über `crit_affects_control_flow()` aus E11.4d.

**(b) Gestaffelte Stichprobe, wo der DP nicht trägt.** Explodiert die Zustandsverteilung, bleibt
Monte-Carlo — aber nicht mit einer festen Zahl je Kandidat:

```
alle Kandidaten          2–4 gepaarte Startwerte
offensichtlich schlechte stoppen
unklare                  8–16
enge Finalisten          32–64
letzte Kandidaten        128–256      ← KEINE Obergrenze, siehe Kasten

Stopp, wenn   Obergrenze(Kandidat) < Untergrenze(EINGEFRORENER Amtsinhaber)
```

> **Diese Stufen können eine 99,9-%-Bedingung nicht zertifizieren — nachgerechnet.** E12.0 nennt
> als harte Bedingung `Mechanikerfolg ≥ 99,9 %`. Selbst bei **null** beobachteten Fehlern liegt
> die einseitige 99-%-Untergrenze nach 256 Versuchen bei
>
> ```
> 0,01^(1/256)  =  98,22 %          — nicht 99,9 %
> ```
>
> Für eine 99-%-Untergrenze von 99,9 % braucht es bei null Fehlern
>
> ```
> n ≥ ln(0,01) / ln(0,999)  ≈  4 603 fehlerfreie Versuche
> ```
>
> — und campaignweite Fehlerkontrolle erhöht das weiter. **Eine feste Obergrenze von 256 ist
> mit den harten Schwellen unvereinbar.**
>
> **Der Weg ist ein anderer:** exakte Verteilungs-DP wo möglich (dann sind es null Versuche);
> sonst eine **jederzeit gültige Konfidenzfolge**, die läuft, bis die Schwelle sicher über- oder
> unterschritten ist — und bei Budgetende **`Unresolved`** meldet statt einer Zahl.
>
> **Für seltene Ereignisse** (Tod, verpasste Pflichtmechanik) ist naives Stichprobenziehen
> ohnehin der falsche Weg: Importance Sampling, Splitting oder ein Likelihood-Quotienten-Test
> erreichen dieselbe Aussage mit Größenordnungen weniger Kämpfen.

**Voraussetzung ist ein zählerbasierter Zufall**, der denselben Wurf an dieselbe _semantische_
Stelle bindet statt an die Reihenfolge der Ziehungen. Nur dann ist die **gepaarte Differenz**
zweier Kandidaten aussagekräftig — und die hat deutlich weniger Streuung als zwei unabhängige
Mittelwerte.

> **Fünf Felder reichen dafür nicht.** `(boss, trial, spell, cast, hit)` koppelt Würfe, die
> getrennt sein müssen: zwei Zufallsquellen desselben Zaubers, Auslöser **und** Krit am selben
> Treffer, mehrere Akteure oder Ziele, Boss- und Legendär-Zufall, verzögerte Wirkungen.

```rust
RngCounter {
    run_seed, encounter_hash, trial_id,
    source_domain,        // crit · proc · boss · legendary · target_selection …
    effect_id, actor_id, target_id,
    cast_ordinal, event_ordinal, hit_ordinal,
}
```

**Dazu drei Versionen, die in jeden Schlüssel gehören:** `counter_rng_algorithm` ·
`rng_contract_version` · `probability_mapping_version`.

**Und Wahrscheinlichkeiten werden über ganzzahlige Schwellen entschieden**, nicht über
Fließkomma-Vergleiche:

```rust
random_u32 < probability_threshold_u32
```

Nur so bleibt das Verhalten über CPU, SIMD und GPU **bitgleich** — ein `f64`-Vergleich wäre
genau die Stelle, an der ein GPU-Pfad still abweicht.

**Der Vertrag gehört in:** `StructuralKey` · `JobKey` · `RunManifest` · `TrialAccumulator` ·
den persistenten Ergebnisspeicher.

**Fertig, wenn** — und ein Monte-Carlo-Vergleich allein genügt nicht, der ist eine
Plausibilitätsprüfung, kein Beweis:

1. **Vollständige Aufzählung**: synthetische Kämpfe mit so wenigen Treffern, dass alle `2ⁿ`
   Krit-Folgen einzeln durchgerechnet werden können — der DP muss **exakt** dasselbe liefern.
2. **Massenerhalt**: die Wahrscheinlichkeitsmasse ergibt 1, innerhalb der Toleranz des
   numerischen Vertrags.
3. **Bekannte Binomialfälle**: Erwartungswert und Varianz stimmen mit der geschlossenen Formel.
4. **Phasenübergänge**: die Wahrscheinlichkeit, Phase _k_ zu erreichen, stimmt einzeln.
5. **Erst danach** Monte-Carlo als statistische Gegenkontrolle.

> **„Exakt" ist hier zweideutig, und der Unterschied gehört nach E14.** Der DP ist
> **modell-exakt** (alle Zustände werden geführt, nichts wird gestichprobt), aber mit `f64`
> **nicht numerisch exakt** — Wahrscheinlichkeiten sind Fließkommazahlen, und Summen über
> viele Zustände driften. Ob die Masse als deterministisches `f64`, als Festkommazahl oder
> rational geführt wird, ist eine Entscheidung des numerischen Vertrags, keine des DP.

Schafft der DP das nicht, endet (a) hier, und (b) trägt allein.

### E11.2 — Der Kernel, wo die Zyklen wirklich liegen

```
expire_buffs         23,8 %   → next_expiry_at je Sammlung
BinaryHeap           16,1 %   → festes Array, max 9 Einträge gemessen
                     ──────
zusammen             39,9 %
```

**Reihenfolge innerhalb des Issues:**

1. `next_expiry_at` je Buff-Sammlung (`if now < next_expiry { return; }`)
2. `EventQueue<INLINE>` mit Überlauf-Fallback und bewiesener Bitaufteilung des
   `u64`-Schlüssels — **`INLINE` wird gemessen (32/48/64), nicht gesetzt**; die
   Strukturrechnung in Teil 1 ergibt bereits 33
3. **Erst danach** messen, ob der Event-Kalender noch etwas bringt

Punkt 3 ist bewusst nachgelagert: Der Kalender ist der größere Umbau, und nach 1 und 2 könnte
der Rest zu klein sein, um ihn zu rechtfertigen.

**Fertig, wenn:** Durchsatz **≥ 470 000 Kämpfe/s** unter E11.0-Bedingungen, und alle
Trace-Hashes unverändert. Der Zielwert ist konservativ aus 39,9 % gerechnet — wenn er nicht
erreicht wird, ist die Ursache zu suchen, nicht das Kriterium zu senken.

**Was NICHT in dieses Issue gehört:** die APL-Optimierung (4,1 % — der Vorschlag sagt das
selbst) und `Fight::new` (4,2 %).

### E11.3 — Trace-Stabilität messen, bevor der Ledger gebaut wird

**Das ist der Punkt, an dem ich am deutlichsten vom Vorschlag abweiche.** Er behandelt den
Trace-Ledger als sicheren Hebel. Gemessen ist nur das Gegenteil (Rotationsänderungen
divergieren sofort).

**Die Messung, die vorher fehlt:**

```
Für jede Größe, die der Balance-Optimierer ändern würde:
  baseDamage, essence, yield, talent_damage, Bossresistenz, gear_effects

  Ändere sie um ±1 %, ±10 %, ±50 %
  → Bleibt die Cast-Folge identisch?
  → Bei welcher Änderung kippt sie zuerst?
```

> **TEILWEISE GEMESSEN am 2026-08-14** (Teil 6.2). An vier Bestandsbossen mit einer
> 6-Zeilen-Rotation: **1–5 % bei den schadenswirksamen Größen**, `synergy` und `vulnerability`
> über den ganzen geprüften Bereich stabil, `boss-1-2` überall stabil (er fällt in 60 s nicht).
> **Der Ledger lebt, seine Region ist schmal.** Was fehlt: andere Rotationen, andere Startwerte,
> und die Frage, ob die Grenze bei einem Zauber-`baseDamage` anders liegt als bei einem globalen
> Faktor.

**Fertig, wenn:** je Größe steht, in welchem Intervall der Trace stabil bleibt — und ob es
Größen gibt, bei denen er **nie** kippt.

> **KORRIGIERT am 2026-08-14.** Hier stand: _„Findet die Messung, dass schon 1 % die Folge
> kippt, ist der Ledger gestorben."_ **Der Schluss ist falsch** (Anhang B.2.5). Ein Trace kann bei
> +2,31 % kippen — dann ist er bei ±1 % stabil und bei ±10 % nicht, und die echte Grenze bleibt
> unbekannt. Und ein **schmales** Intervall macht den Ledger nicht wertlos: Eine Binärsuche nach
> dem minimalen Buff (E12.2) prüft hunderte Werte innerhalb genau eines solchen Intervalls.
>
> Die `±1/±10/±50`-Messung ist eine **Erkundung**, kein Beweis und kein Abbruchkriterium. Sie
> sagt, wo sich der genaue Nachweis lohnt — nicht, ob der Ledger lebt.

### E11.3a — Der Breakpoint-Compiler: Trace-Regionen statt Stichproben

**Was E11.3 misst, beweist E11.3a.** Für einen konkreten Trace werden alle Bedingungen
gesammelt, die ihn erzeugt haben — Ressourcenschwellen, Bereitschaften, Brand-Restlaufzeiten,
Ereignisreihenfolgen, Phasenschwellen:

```
resource >= 15          cooldown_remaining <= 0     dot_remaining < 1500
event_A_time <= event_B_time                        damage >= phase_threshold
```

Jede wird als Ungleichung über den Balance-Parametern geführt. Solange ein neuer Parametersatz
alle erfüllt, ist **bewiesen**, dass Ereignisreihenfolge, Aktionsfolge und Phasenfolge
identisch bleiben — dann genügt eine numerische Neuauswertung des bestehenden Traces, ohne
Simulation.

**Aber die gültige Menge ist im Allgemeinen kein einzelnes Intervall.** `snap_up` auf 10 ms,
`round()`, `min`/`max`, Rücksetzer und diskrete Ereignisreihenfolgen können sie zerreißen:

```
gültig:  [10, 17] ∪ [22, 24]        nicht:  [10, 24]
```

Bei mehreren Parametern kann sie nicht konvex sein — dann ist sie **kein** Polyeder. Wer eine
Region als `[min, max]` führt, erklärt Werte für gültig, die es nicht sind, und rechnet sie
ohne Simulation aus. **Für ROTA ist der Ein-Parameter-Fall der wichtige** — der
Balance-Optimierer bewegt eine Zahl und fragt nach dem Kipppunkt —, und dort ist die richtige
Darstellung eine **Intervallmenge**:

```rust
struct TraceRegion1D {
    intervals:       SmallVec<[InclusiveRange<i64>; 4]>,
    boundary_causes: SmallVec<[GuardId; 8]>,   // WELCHE Bedingung kippt am Rand
}
```

> **Die Messung aus Teil 6.2 zeigt, wo die Ränder liegen.** Bei fester Kampfdauer ohne
> Kill-Abbruch ist ein Schadensfaktor **kontrollflussneutral** — die Folge kippt erst, wenn der
> Boss **einen Cast früher fällt**. Die Region ist also im Kern das Intervall zwischen zwei
> Kill-Zeitpunkten, und ihre Ränder sind **berechenbar**, nicht nur suchbar: Man kennt die
> Schadenssumme je Cast und die Boss-HP.

Nachgewiesen wird sie über symbolisches Nachspielen der Wächter, **ganzzahlige** Zeitrechnung
(die 10-ms-Rasterung macht das exakt möglich), rekursive Intervallteilung — und eine
Vollsimulation an jeder Regionsgrenze.

> **Die Grenzprüfung allein ist kein Beweis.** Ein Fehler in der symbolischen Herleitung kann
> genauso **im Inneren** einer Region liegen — und dort prüft niemand. Dazu gehören deshalb:
> Stichproben zufälliger Innenpunkte · metamorphe Tests (dieselbe Region, andere Reihenfolge) ·
> instrumentiertes Nachspielen der Wächter · Erzeugung eines Gegenbeispiels bei Abweichung ·
> und im Zweifel der **Rückfall auf Vollsimulation**.
>
> **Die inverse Treppenfunktion (E12.2) darf nur laufen**, wenn E16 die Monotonie für genau
> diese Kombination bescheinigt: Parameter × Ergebnisgröße × Bossart × Trace-Region. Eine
> globale Monotonie-Annahme wäre bei `runaway` bereits falsch.

**Zwei Anforderungen, die aus dem Bestand folgen:**

**Die Zeitbedingungen sind ganzzahlig und gerastert.** `snap_up` auf 10 ms macht aus jeder
Zeitungleichung eine Bedingung über Vielfachen von 10 — das ist einfacher als der allgemeine
Fall und exakt lösbar, ohne Intervallarithmetik.

**Aber die Zellen eines Builds sind nicht die Zellen eines Zaubers.** Gemessen (Anhang B.2.2): Über
einen globalen Tempo-Wert gibt es bei 14 Zaubern **keine** gemeinsame Zelle. Ein
Breakpoint-Compiler, der Zellen über den ganzen Build bildet, misst nichts. Er muss **je
Parameter** arbeiten, und der Gewinn steht und fällt damit, wie viele Zeiten der Parameter
berührt.

**Fertig, wenn:** für einen zauber-lokalen Parameter (`baseDamage` eines Zaubers) die exakte
Region steht, und ein Wert innerhalb der Region ohne Simulation dasselbe Ergebnis liefert wie
der Vollkampf — bitgleich, gegen die Trace-Hashes aus E11.0.

**Wenn es scheitert**, endet E11.3a mit diesem Ergebnis, und E12 sucht in Rohwerten weiter.
**Das wäre kein Fehlschlag, sondern ein gesparter Umbau** — aber der Schluss steht erst nach
dem Beweisversuch, nicht nach einer Stichprobe.

### E11.4 — Inkrementelle Auswertung für Rotationsmutationen

> **KORRIGIERT am 2026-08-14 nach einem Einwand von außen — und die Korrektur ist gemessen.**
>
> Die erste Fassung dieses Issues sagte: „Mutation an Position p → ab Checkpoint p
> weitersimulieren." Der Einwand: Eine Zeile an Position 8 kann bereits **Entscheidung 0**
> ändern, wenn die acht Zeilen darüber in diesem Moment alle nicht greifen. Position ≠
> Entscheidungsnummer.
>
> **Gegenprobe gefahren**, mit Rotationen, deren obere Zeilen bewusst Lücken lassen
> (bedingte Zeilen oben, Füller unten), Einfügung an Position 3, Vergleich der echten
> Cast-Folge aus dem Kampfprotokoll:
>
> ```
> geprüfte Varianten                             440
> davon mit Abweichung VOR der Einfügeposition   436   (99,1 %)
> früheste beobachtete Abweichung                Entscheidung 0
> ```
>
> **Beide Messungen sind richtig, sie messen verschiedene Rotationsklassen.** Worker-3s
> „Position p → Präfix p, null Streuung" galt für Varianten einer **gefundenen** Rotation —
> dort greift oben zuverlässig etwas. Für Rotationen mit Lücken oben gilt sie nicht.
>
> **Und mein erster Messversuch war selbst fehlerhaft:** Ich baute die „Cast-Folge" aus
> `r.casts`, der Zahl je Zauber — das ist keine Folge, sondern eine nach Zauber-Index
> sortierte Zählung. Sie meldete 78,6 % statt 99,1 %. Derselbe Fehler, den dieser Tag
> fünfmal gezeigt hat: **der Messweg erzeugt die Antwort.** Erst der Vergleich der echten
> Protokollfolge (`LogKind::Cast`, nach Zeit) trägt.

**Was daraus folgt: Der Checkpoint wird berechnet, nicht aus der Position abgeleitet.**

```rust
DecisionCheckpoint {
    state: CompactFightState,
    selected_rule: RuleId,
    valid_rules:  RuleBits<WORDS>,    // NICHT BitSet<96> — siehe Kasten
    ready_spells: SpellBits<WORDS>,   // NICHT BitSet<64>
    event_cursor: …,
    accumulated_result: …,
}
```

> **Feste Bitbreiten widersprechen der Erweiterbarkeit, die dieser Plan zusichert.** `BitSet<64>`
> bei **60 geplanten Zaubern** lässt vier Bits frei; `BitSet<96>` bei 93 Kandidatenzeilen drei.
> Eine einzige neue bedingte Zeile je Schule sprengt beides — und ein Bitset, das überläuft,
> meldet keinen Fehler, es **verliert Regeln**.
>
> **Richtig ist ein spezialisierter schneller Pfad mit generischem Rückfall**, zur Bauzeit
> gewählt:
>
> ```
> ≤ 64 Zauber      1 × u64      schnellster Pfad
> 65–128           2 × u64
> darüber          Blockpfad, generisch
> ```
>
> Dasselbe gilt für `local_to_global_spell[16]` und `compiled_spells[16]` in E11.4d: Die
> Leiste hat 12–16 Plätze, aber der Auftraggeber nennt **„20 Tasten"** als Lastfall
> (Teil 1, Punkt 7). Richtig ist `ActiveSpells<INLINE=16>` mit Überlauf — oder ein aus dem
> Regelwerk **erzeugtes** `MAX_ACTIVE_SPELLS`, das mit den Daten wächst.

Der erste beeinflussbare Checkpoint ergibt sich aus der Art der Mutation:

| Mutation                 | erste mögliche Abweichung                                                                                                                                                          |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Zeile einfügen**       | erster Checkpoint, an dem die neue Bedingung gilt **und** ihr Zauber bereit ist **und** sie vor der damals gewählten Regel steht **und** keine höher priorisierte Regel gültig war |
| **Zeile entfernen**      | erster Checkpoint, an dem die entfernte Regel tatsächlich gewählt wurde                                                                                                            |
| **zwei Zeilen tauschen** | erster Checkpoint, an dem **beide** gültig und bereit waren                                                                                                                        |
| **Zahl ändern**          | erster Checkpoint, an dem ein Prädikat, eine Ereignisreihenfolge, eine Bereitschaft oder eine Phase kippt                                                                          |

```
Mutation
   ↓
ersten semantisch beeinflussbaren Checkpoint suchen
   ↓
keiner vorhanden?  → Ergebnis unverändert übernehmen
   ↓
sonst Zustand davor laden, nur das Suffix rechnen
```

Die Felder `valid_rules` und `ready_spells` sind genau dafür da — ohne sie ist die Suche nach
dem ersten beeinflussbaren Punkt nicht möglich, und dann bleibt nur die Vollsimulation.

**Der wichtigste Zweig ist der oberste — und das ist gemessen, nicht vermutet.**

> **NACHGETRAGEN am 2026-08-14** aus der DAG-Messung (Anhang B.2.1). Der Zweig „keiner vorhanden →
> Ergebnis unverändert übernehmen" stand hier als Randfall. **Er ist der Hauptfall:**
>
> ```
> Basislänge 2:  141 von 273 Kandidaten (51,6 %) erzeugen die Basisfolge unverändert
> Basislänge 5:  216 von 528 Kandidaten (40,9 %)
> ```
>
> Die eingefügte Zeile greift **kein einziges Mal**. Bei 528 Vollkämpfen je Runde sind das 216
> Kämpfe, deren Ergebnis vorher feststeht. Das ist zugleich die Antwort auf den
> Multi-Policy-DAG: Wer diesen Zweig baut, hat 95 % dessen abgeräumt, was ein
> Entscheidungsgraph teilen würde (dort bleiben 1,07×).

**Der Vertrag, der IMMER gilt** — unabhängig vom Operator:

```
berechneter Startcheckpoint  ≤  tatsächlicher erster Divergenzpunkt
optimierter Suchlauf         =  bitgleich dasselbe Ergebnis
```

**Ein zu früher Checkpoint kostet nur Leistung. Ein zu später liefert ein falsches Ergebnis** —
und zwar still.

> **KORRIGIERT am 2026-08-15.** Hier stand „**mindestens 1,8×** bei Greedy **oder** LNS" und
> „Quote unter 35 % heißt Erkennung unvollständig". **Beide Zahlen stammen aus einer Messung am
> Greedy-Einfügeoperator** (40,9–51,6 %). Für LNS-Zerstören/Reparieren, Tauschen, Entfernen,
> gekoppelte Mutationen und Zahlenänderungen gibt es **keine solche Untergrenze** — eine Quote
> von 15 % kann dort völlig richtig sein, weil tatsächlich nur 15 % unverändert sind. Die alte
> Regel hätte eine **korrekte** Umsetzung durchfallen lassen.

**Abnahme je Operator statt einer Quote für alle:**

| Operator                 | Korrektheit                   | Leistung                                 |
| ------------------------ | ----------------------------- | ---------------------------------------- |
| Greedy-Einfügen          | gegen Vollsimulation          | Prüffall reproduziert **40,9–51,6 %**    |
| Entfernen                | erster Divergenzpunkt korrekt | gemessener Gewinn, **keine feste Quote** |
| Tauschen                 | erster Divergenzpunkt korrekt | gemessener Gewinn                        |
| LNS zerstören/reparieren | bitgleiches Suchergebnis      | End-to-End-Gewinn                        |
| Zahlenänderung           | Trace-/Wächter-Nachweis       | Anteil über Ledger und Suffix            |

**Ein globales `≥ 1,8×` gehört erst nach E11.0c in ein SLA.**

**Voraussetzung:** der kompakte Suchzustand. Heute ist `Fight` **5 600 Byte** mit elf Arrays à
70 Einträgen, obwohl ein Build 12–16 Zauber nutzt. Vierzig Checkpoints à 5,6 KB sind 224 KB —
das ist mehr, als der L2 eines Kerns nebenher tragen kann. Deshalb gehört die Verdichtung auf
aktive Zauber **in dieses Issue**, nicht in ein eigenes.

> **PRÄZISIERT am 2026-08-14** nach der Cache-Aufnahme in Teil 2.1. Hier stand: _„224 KB — das
> **passt nicht** in den L2 eines Kerns."_ **Auf der Zielmaschine ist das zu absolut
> formuliert:** Zen 3 hat **512 KiB L2 je Kern**, 224 KB passen dort rein — nur eben zusammen
> mit Code, Bossdaten, Zauberdaten und Ereignisschlange, und dann wird es eng.
>
> **Auf den anderen Systemen stimmt der ursprüngliche Satz wörtlich:**
>
> ```
> Zen 3 (Laptop)        512 KiB je Kern   224 KB = 44 % — eng, aber möglich
> Sandy Bridge (gonet)  256 KiB je Kern   224 KB = 88 % — praktisch ausgeschlossen
> Gracemont E-Core      512 KiB je Kern   wie Zen 3
> ```
>
> Die Maßnahme bleibt richtig, ihre Begründung wird nur genauer: Es geht nicht um ein hartes
> Nichtpassen, sondern um den **Anteil**, den die Checkpoints vom L2 beanspruchen — und der ist
> auf zwei von drei Systemen zu hoch. Die Zahl gehört als Kennwert in E11.0 (Cache-Fehlgriffe
> je Kampf), nicht als Behauptung in die Begründung.

**Und die Verdichtung wird zweigeteilt** (Punkt 17 des Urteils, richtig):

```rust
SearchFight   nur was das Ziel bestimmt: Summen, 12–16 aktive Zauber,
              Ereignisse, Abklingzeiten, Verstärkungen, Ressource, HP,
              Bosszeiger                                  → Millionen Kandidaten

AuditFight    zusätzlich damage_by_spell, casts, lost_time, volles
              Protokoll, Paritäts-Hashes                  → Elites, Fehlerfälle,
                                                            Stichproben
```

Ohne die Trennung trägt jeder Suchkampf die Diagnosefelder mit, die nur die Abnahme braucht —
und die Checkpoints werden erneut zu groß für den Cache. **Beide Pfade müssen für dieselbe
Eingabe dasselbe Ziel liefern**, geprüft über die Trace-Hashes aus E11.0.

### E11.4b — Der Evaluator-Router und der kanonische Ergebnis-Cache

**Diese Ebene fehlte in der ersten Fassung, und ihr Fehlen ist der schwerwiegendste Mangel.**
Ohne sie entstehen aus E11.1 bis E11.4 lauter Sonderpfade, die niemand zusammenführt — und
jeder Suchlauf rechnet Kandidaten erneut, die er schon kennt.

```
Kandidat
   ↓ normalisieren  →  CandidateKey
   ↓
schon bewertet?          → Cache
analytisch exakt?        → Erwartungswertformel
gültiger Ledger?         → Ledger neu auswerten
gemeinsames Präfix?      → Suffix ab berechnetem Checkpoint
homogener Batch?         → AVX2
sonst                    → skalarer Vollkampf
```

**Der Schlüssel wird zweiteilig geführt**, und das ist keine Feinheit:

```rust
StructuralKey {          // was den Kontrollfluss bestimmt
    ruleset_version, boss_id, boss_structure_version,
    talent_phenotype_id, gear_phenotype_id,
    set_mask, legendary_mask,
    rotation_canonical_id, spellset_id, feature_mask,
    // NACHGETRAGEN — ohne diese ist ein Treffer nicht derselbe Kampf:
    difficulty, ladder_level, player_level,
    fight_horizon, stop_on_kill, encounter_semantics,
    apl_language_version, effect_ir_version, numeric_contract_version,
    event_order_version, boss_signature_version, set_legendary_schema,
}

NumericKey {             // was nur Zahlen ändert
    structural_key,
    numeric_parameter_vector,
    rng_semantics_version,
}
```

Bei `baseDamage 100 → 101` bleibt der `StructuralKey` gleich — der Trace ist womöglich
wiederverwendbar. Ein Schlüssel, der nur eine `balance_version` führt, würde genau diese
Wiederverwendung verhindern.

> **Für stochastische Ergebnisse reichen beide Schlüssel nicht.** Ein Eintrag aus 32 Durchgängen
> darf nicht als Treffer für eine Anfrage nach 256 gelten. Dazu gehören:
> `trial_set` bzw. `trial_range` · `rng_version` · **welche Statistik** verlangt wurde
> (Erwartungswert, p10, Siegwahrscheinlichkeit) · die `ResultGuarantee`.
>
> **Und der Job für einen Zusatzrechner ist mehr als (Schlüssel, Bereich, Startwert):**
>
> ```rust
> JobKey { run_contract_hash, engine_semantics_hash, content_hashes,
>          numeric_contract_hash, evaluator_kind, encounter_key,
>          candidate_range, trial_range, objective_or_outcome_kind,
>          result_schema_version }
> ```
>
> Ohne `evaluator_kind` und `result_schema_version` kann ein Rechner ein Ergebnis liefern, das
> formal passt und semantisch etwas anderes ist.

**Der Router entscheidet nicht nur semantisch, sondern nach erwarteten Kosten.**

> **KORRIGIERT am 2026-08-14.** Hier stand als Abnahmekriterium „Trefferquote **über 15 %**".
> **Die Zahl war geraten** (Anhang B.2.5). Richtig ist die Break-even-Rechnung:
>
> ```
> Cache lohnt, wenn   L + (1-h)·(F + I)  <  F
> also wenn           h  >  (L + I) / (F + I)
>
> L  Lookup      I  Insert-/Miss-Aufschlag      F  Kosten der Auswertung      h  Trefferquote
> ```
>
> **Und die Schwelle ist pro Auswertungspfad verschieden.** Vor einem Vollkampf (F groß) lohnt
> schon eine sehr kleine Quote; vor einer Ledger-Auswertung (F klein) womöglich auch 30 %
> nicht. Eine einzige globale Prozentzahl kann beide Fälle nicht treffen.

Dazu schätzt jeder Zweig seine Kosten, statt nur seine Zulässigkeit zu melden:

```rust
CostEstimate { expected_cycles, expected_memory, expected_batch_size, confidence }
```

Damit sind zwei Entscheidungen möglich, die eine rein semantische Reihenfolge nicht treffen
kann: _„kleines Ledger mit teurer Gültigkeitsprüfung oder billiger skalarer Kampf?"_ und
_„noch auf einen AVX2-Batch warten oder jetzt rechnen?"_

**Fertig, wenn:** ein realer LNS-Lauf je Zweig druckt: Trefferquote, gemessenes `L`, `I`, `F`
— und die daraus **berechnete** Break-even-Schwelle daneben. Liegt ein Zweig unter seiner
eigenen Schwelle, wird er abgeschaltet, nicht der Cache insgesamt.

### E11.4c — Der Produktions-Scheduler

E11.0 baut eine **Messumgebung**. Wie die 16 Kerne während einer echten Suche ausgelastet
werden, stand nirgends.

```
ein persistenter Worker-Pool, 8 oder 16 — gemessen, nicht gesetzt
Kandidaten-FENSTER je Epoche, zusammenhängend — NICHT ein globales Array
Batchvergabe über AtomicUsize, Quantum in MILLISEKUNDEN (WorkQuantumPolicy, E15.1)
  — die Kandidatenzahl ist ein abgeleiteter Momentwert, keine Einstellung
worker-lokal: Zustand, Ereignisschlange, Checkpoints, Ergebnispuffer
Schreiben nach result[candidate_id] — deterministisch, keine Reduktion
```

**Kein** Thread je Kampf, **kein** Kanal je Kandidat, **keine** verschachtelten Pools, **keine**
Fließkomma-Reduktion in Ankunftsreihenfolge (die wäre nicht reproduzierbar).

> **Und kein globales Array über den ganzen Lauf.** Für eine LNS-Epoche mit 10 000 Kandidaten
> ist `result[candidate_id]` richtig; für einen Sweep über Milliarden aufgelöste Kandidaten wäre
> allein die Belegt-Bitmap über ein Gigabyte groß.
>
> ```rust
> CandidateWindow { epoch_id, global_id_range, manifest_hash,
>                   candidates_aosoa, local_result_slots }
> ```
>
> Kandidaten werden **deterministisch gestreamt**, nur ein Fenster liegt im Speicher, globale
> Kennungen bleiben stabil, Ergebnisse gehen in Scherben. `result[candidate_id]` heißt intern
> `result[local_slot]`. **Die Fenstergröße gehört in den `MemoryBudgetManager`** — dort fehlt
> sie heute.

**Kandidaten werden vor der Ausführung sortiert** — nach Auswertungsmethode, Boss, Gestalt,
Zaubermenge. Das verbessert Sprungvorhersage, Cache-Wiederverwendung und später die
AVX2-Auslastung.

**Auf heterogenen Maschinen ist die statische Aufteilung falsch.** Der i5-1235U (Teil 2.1) hat
2 P-Cores mit 4,4 GHz und 8 E-Cores mit 3,3 GHz. Wer 12 gleich große Pakete auf 12 Stränge
legt, wartet auf die E-Cores. Die Batchvergabe über `AtomicUsize` gleicht das **teilweise** aus,
weil schnelle Stränge sich mehr Chunks holen — das ist der Grund, warum oben „Batchvergabe" und
nicht „Bereichsaufteilung" steht.

> **Aber der Zähler allein genügt nicht.** Er gleicht Geschwindigkeit aus und verhindert
> **nicht**: Strangwanderung zwischen P und E, ungünstige Cache-Kopplung, unterschiedliche
> L2-Größen (1,25 MiB gegen 512 KiB) und — nach E11.7 — verschiedene nutzbare Befehlssätze.
> **Maßgeblich ist die Pool-Trennung aus E15.0**: eigenes CPU-Set, eigene Ratekurve, eigene
> Chunkgröße je Kernklasse, und **innerhalb** jedes Pools die dynamische Entnahme.

**8 gegen 16 Stränge wird nach jedem Zustandsumbau neu gemessen.** Heute gewinnen 16 noch
14 % gegenüber 8 (350 794 gegen 308 152) — nach der Verdichtung auf 12–16 aktive Zauber kann
das kippen, weil dann mehr in den Cache passt.

### E11.4d — `CompiledBuild`: einmal falten statt je Kampf

Heute baut jeder Kampf seine Faktoren aus Talenten, Ausrüstung und Zauberdaten neu auf. Das
gehört vor die Suche:

```rust
CompiledBuild {
    active_spells: ActiveSpells<16>,   // Inline-Schnellpfad + Überlauf, NICHT [16]
    compiled_rotation: CompiledRotation,
    action_policy: ActionPolicy,       // Overdrive/Spellsteal — siehe E11.4d unten
    folded_talent_values,
    folded_gear_values,
    set_mask, legendary_mask,
    boss_specific_factors,
    dependency_mask,             // trägt E11.3 und den analytischen Krit
    kernel_signature,            // wählt den Kernel in E11.4b
}
```

Die `dependency_mask` ist der Punkt, an dem drei Issues zusammenlaufen: Sie entscheidet, ob
der analytische Krit zulässig ist, ob ein Ledger gilt, und welcher Kernel gewählt wird.

**Und sie ersetzt eine Bossliste, die nicht altern würde.** Der analytische Krit darf **nicht**
über `if boss_id != BOSS_1_2 && boss_id != BOSS_3_4` aktiviert werden — heute stimmt das,
morgen nicht mehr. Richtig ist:

```rust
if !compiled.dependencies.crit_affects_control_flow() { … }
```

Die Analyse prüft: HP-Phasen, HP-Prädikate in Kandidatenzeilen, Tod/Sieg als Abbruch,
schadensabhängige Verstärkungen, Ressourcen, Schutzschichten, Umwandlung.

> **Diese Prüfung allein genügt nicht — sie beantwortet die falsche Hälfte der Frage.**
> `crit_affects_control_flow() == false` sagt: _der Zufall ändert die Aktionsfolge nicht._ Es
> sagt **nicht**: _der gesuchte Ergebniswert ist im Zufall linear._
>
> Für den **erwarteten Gesamtschaden** bei festem Trace ist die analytische Auswertung exakt:
> `E[D] = (1−p)·D_normal + p·D_krit`. Für diese Größen ist sie es **nicht**:
>
> ```
> Wahrscheinlichkeit, den Boss zu legen
> p10 / p90 des Schadens
> Überlebenswahrscheinlichkeit
> Wahrscheinlichkeit, Phase k zu erreichen
> jeder nichtlineare Score
> "fällt der Boss vor Zeitpunkt X?"
> ```
>
> **Der Router braucht deshalb eine zweistellige Prüfung**, nicht eine einstellige:
>
> ```rust
> can_marginalize(stochastic_source, requested_outcome, objective) -> bool
> ```
>
> **Und die Antwort ist nicht „alles analytisch" oder „alles stichprobenweise", sondern eine
> Aufteilung je Quelle:**
>
> ```rust
> StochasticPartition { sampled_sources,        // ändern den Kontrollfluss
>                       marginalized_sources,   // ändern NUR das Ergebnis
>                       requested_outcome, validity_certificate }
> ```
>
> ```
> Bosszielwahl ändert den Trace          →  stichprobenweise
> Krit ohne HP-Phase und ohne Kill       →  BEDINGT analytisch ausintegrieren
> reiner Schadenswurf                    →  analytisch
> ```
>
> **Das ist bedingtes Monte-Carlo (Rao-Blackwellisierung), und es ist der stärkste noch
> ungenutzte Hebel gegen die Versuchszahl:** Der Erwartungswert bleibt exakt, die **Varianz
> sinkt** — und mit ihr die Zahl der Kämpfe, die eine Konfidenzfolge braucht. Das wirkt genau
> dort, wo die 4 603 Versuche von oben sonst anfallen.
>
> Der analytische Pfad liefert `ExpectedDamage` exakt — die **Verteilung** liefert er nicht.
> Welche der beiden gebraucht wird, entscheidet die Zielfunktion (Teil 6.1), und deshalb hängt
> auch dieser Zweig an ihr.

**Und sie trägt eine zweite Aufgabe: die Trennung von Kontroll- und Ergebniszustand.**

> **NACHGETRAGEN am 2026-08-14.** Das externe Urteil führt sie als Unterpunkt seines
> Macro-Transition-Caches; ich hatte sie mit dem Cache zusammen verworfen. **Das war zu
> schnell** — sie ist unabhängig davon richtig und gehört hierher.

```rust
ControlState   Zeit · Abklingzeiten · Ressourcen · Verstärkungen · Brände
               Bosszeiger · Phase · Spieler-HP        → bestimmt den Kontrollfluss

RewardState    Gesamtschaden · Heilung · Absorbiert · Schaden je Zauber
               → geht in das Ergebnis, nicht in die nächste Entscheidung
```

Zwei Kämpfe mit **verschiedenem bisherigem Schaden, aber gleichem `ControlState`** verlaufen ab
dort identisch — solange nichts den Kontrollfluss vom Schaden abhängig macht. **Genau das
entscheidet `crit_affects_control_flow()`**, und deshalb ist die Trennung keine zweite
Maßnahme, sondern die Umsetzung dessen, was die Abhängigkeitsmaske bereits feststellt.

**Der Nutzen ist ein schärferer Cache-Schlüssel:** Er darf den `RewardState` weglassen, wo die
Maske es erlaubt — dann treffen sich Kandidaten im Cache, die sich nur im Ergebnis
unterscheiden.

> **Für stochastische Ergebnisse speichert der Cache keinen Mittelwert, sondern einen
> Akkumulator.** Sonst ist ein Eintrag aus vier Durchgängen wertlos, sobald zwölf gebraucht
> werden — obwohl die vier bereits gerechnet sind:
>
> ```rust
> TrialAccumulator { trial_ids: TrialIdSet,        // RangeSet oder Roaring-Bitmap
>                    count, sum, sum_squares, win_count,
>                    quantiles: ExactHistogram | DeterministicSketch }
> ```
>
> **Und die Vereinigung über Rechner hinweg ist mit `f64` nicht automatisch deterministisch:**
> `(a+b)+c ≠ a+(b+c)`. Kommen Pakete anders geschnitten oder in anderer Reihenfolge zurück,
> ändert sich das letzte Bit von `sum` und `sum_squares` — und damit womöglich eine
> Abbruchentscheidung der gestaffelten Stichprobe. **Ein ausgefallener Zusatzrechner würde das
> Ergebnis ändern**, genau das, was E15.5 verhindern soll.
>
> ```
> Festkomma-Akkumulator nach E14                        (bevorzugt)
> ODER feste Trialblöcke + kanonische binäre Reduktionsreihenfolge
> ODER reproduzierbarer Superakkumulator, falls f64 zwingend bleibt
> ```
>
> **Und „optionale Quantilstruktur" ist als Abnahmekriterium zu unbestimmt:** p10/p90 brauchen
> entweder ein exaktes Histogramm oder einen deterministisch vereinigbaren Sketch, dessen
> Fehler als `BoundedApproximation` ausgewiesen wird.
>
> Damit lassen sich 4 vorhandene um 12 neue **ergänzen**, Ergebnisse von Zusatzrechnern
> deterministisch **vereinigen**, und die gestaffelte Stichprobe (E11.1b) baut direkt darauf auf.
> **Die Kennungen der Durchgänge müssen mitgespeichert werden, nicht nur ihre Anzahl** — sonst
> ist die gepaarte Auswertung mit gemeinsamen Zufallszahlen unmöglich.

**Die Grenze, die man dabei übersieht:** Boss-Lebenspunkte sind eine Funktion des kumulierten
Schadens und stehen im `ControlState`. Bei jedem Boss mit HP-Phase, `runaway` oder
`stop_on_kill` ist der Schaden damit **doch** kontrollflusswirksam. Die Trennung gewinnt also
genau dort, wo die Maske ohnehin grünes Licht gibt — und nirgends sonst. **Kein eigenes Issue,
ein Feld in E11.4d.**

### E11.4f — Policy-Äquivalenz: verhaltensgleiche Rotationen einmal rechnen

**Das ist der Teil des externen Urteils, den die Messung bestätigt hat** (Anhang B.2.1). Eine
Rotation ist eine Funktion „gültige-und-bereite Menge → gewählte Regel". Zwei Rotationen, die
für alle **erreichbaren** Mengen dieselbe Regel wählen, sind semantisch identisch und brauchen
eine Auswertung, nicht zwei.

```
Basislänge 2:  273 Kandidaten → 124 Klassen      Faktor 2,20×
Basislänge 5:  528 Kandidaten → 298 Klassen      Faktor 1,77×
```

**Die Same-Spell-Subsumption (Faktor 1,27, § 1.2b) ist der triviale Sonderfall davon.** Sie
findet 35 Paare über die Bedingungsstruktur; die Äquivalenz findet 230 Kandidaten je Runde
über das tatsächliche Verhalten.

**Wie es gebaut wird — und warum nicht als Vorabanalyse.** Die Signatur über alle
theoretischen Mengen zu bilden ist unbezahlbar (2⁹⁶ Masken). Der Weg ist iterativ, nach dem
Muster gegenbeispielgeführter Verfeinerung:

```
Start:    die Masken, die in bereits gerechneten Kämpfen vorkamen
          Rotationen mit gleicher Signatur zusammenfassen
neue Maske gefunden?    → Klassen verfeinern
Unterschied gefunden?   → er ist das Gegenbeispiel, Klasse teilen
bis keine neuen mehr auftauchen
```

**Der billige Anfang ist aber ein anderer**, und er steht schon in E11.4: Wer 216 von 528
Kandidaten als „ändert nichts" erkennt, hat den größten Teil der Äquivalenzklassen erwischt —
die größte Klasse **ist** die unveränderte Basis. E11.4f lohnt erst für die restlichen 312.

> **NACHGESCHÄRFT am 2026-08-14, dritte Runde — und das ist kein Detail.** Das Verfahren oben
> beweist **nicht**, dass zwei Rotationen auf _allen erreichbaren_ Zuständen gleich sind. Es
> beweist nur, dass auf den **bisher beobachteten** Masken kein Unterschied auftrat. Eine
> erreichbare, aber noch nie gesehene Maske kann sie später trennen.
>
> **Wer daraufhin den Suchraum reduziert, verliert möglicherweise das Optimum — still.**
> Deshalb trägt jede Äquivalenzaussage ihren Geltungsgrad mit:

```rust
enum PolicyEquivalence {
    TraceLocalExact,   // exakt für die Checkpoints DIESES Traces
    FormallyProven,    // für alle erreichbaren Zustände bewiesen
    ObservedOnly,      // bisher kein Gegenbeispiel — kein Beweis
}
```

| Grad              | darf verwendet werden für                                         |
| ----------------- | ----------------------------------------------------------------- |
| `TraceLocalExact` | Cache und Suffix-Auswertung — dort ist der Trace der Bezugsrahmen |
| `FormallyProven`  | **Reduktion des Suchraums**                                       |
| `ObservedOnly`    | **nur Priorisierung.** Finale Kandidaten werden einzeln gerechnet |

**Der Beweis ist erreichbar**, weil der Horizont endlich ist. Symbolische Erreichbarkeitsanalyse
oder beschränkte Modellprüfung liefert `FormallyProven` — ein eigener Schritt, kein Nebenprodukt
der Messung.

> **Die Tiefe darf aber nicht auf „40 Entscheidungen" fest verdrahtet werden.** Diese Zahl gilt
> für den heutigen Bestand. `freesNextGcd`, `refundsInterruptedGcd`, `doublesNextCast` und die
> Abklingzeit-Rücksetzer — **alle neun in `sublimation.json** — verändern genau sie (siehe
E11.5b). Die maximale Tiefe folgt aus `UpperBoundCapability`; ist keine sichere Schranke
ableitbar, wird nach Zeit- und Ereigniszuständen statt nach Entscheidungszahl geprüft, und
bei Zeitüberschreitung bleibt die Klasse `ObservedOnly`.
>
> **Und bei Zufall gilt die Äquivalenz nur, wenn sie für alle gekoppelten Ausgänge gilt** —
> nicht für einen Startwert.

**Fertig, wenn:** der Läufer je Runde druckt, wie viele Kandidaten über eine Äquivalenzklasse
statt über einen Kampf gingen — **getrennt nach Geltungsgrad** — und die Quote **über der von
E11.4 liegt**. Klassen mit `ObservedOnly` zählen dabei nicht als eingespart, sondern als
verschoben.

**Endausbau-Vorbehalt:** 1,77× ist eine Menge, keine Struktur. Mehr Zauber und längere Listen
heißen mehr nie greifende Zeilen, also größere Klassen; mehr Bossmechaniken heißen mehr
Anlässe zur Divergenz, also kleinere. Die Richtung ist **nicht** vorhersagbar — deshalb ist
das Abnahmekriterium relativ formuliert und nicht als Zahl.

### E11.4e — Fortschritts-Simulation: die Schleife über dem Kampf

**Die Frage, die dieses Issue beantwortet:** Der Simulator rechnet, was ein Ausrüstungsstand
leistet. Er rechnet **nicht**, ob dieser Stand erreichbar ist, nach wie vielen Kämpfen, und
mit welcher Streuung. Das ist heute eine gesetzte Annahme.

#### Was der Bestand hat

```ts
erwarteterStand(vorstufe, plaetze) → OwnedItemLike[]
  itemLevel = itemLevelOf(vorstufe, normal, 0)
  bessere   = bessereStuecke(vorstufe, plaetze)   // aus den Fallgewichten
  → so viele Plätze über common, der Rest common
```

Das ist eine **Annahme über den typischen Spieler**, und `src/devtools/expected-gear.ts`
benennt sie ausdrücklich als solche: _„Sie wird gesetzt und benannt, nicht geraten."_ Ein Boss
in Instanz 3 wird gegen den Stand gerechnet, den Instanz 2 im Mittel liefert.

**Was fehlt:** Niemand prüft, ob dieser Stand nach realistischer Spielzeit tatsächlich
erreicht wird.

#### Warum das keine Erweiterung des Kampf-Simulators ist

Die Trennung ist im Code zugesichert, nicht nur Konvention:

```rust
assert_eq!(STREAM_COUNT, 3,
  "Der Beute-Strom gehoert NICHT zu den Kampf-Stroemen … Beute wird beim SIEG
   gewuerfelt, nicht im Kampf — sie kann nichts verschieben.");
assert!(!namen.contains(&"loot"));
```

Fortschritt ist eine **Schleife über** dem Kampf, keine Erweiterung darin:

```
Startstand
   ↓
Boss wählen (was ist freigeschaltet?)
   ↓
Kampf-ORAKEL: schafft dieser Stand diesen Boss? → ja/nein, Zeit, Rest-HP
   ↓
bei Sieg: Beutewurf nach src/loot/rates.ts
   ↓
besseres Stück? → anlegen, Phänotyp neu bestimmen
   ↓
zurück, Zähler erhöhen
```

Monte-Carlo über **Spielsitzungen**, nicht über Kämpfe.

#### Warum es billig ist — und warum es an E11.4b hängt

Der Kampf-Simulator wird darin fast nie aufgerufen:

```
Gear-Phänotypen heute        56        (Endausbau 5 412  [MODELL])
echte Bosse                  17        (Endausbau 68)
                          ──────
Orakel-Tabelle            952 Zellen   (Endausbau 368 016 [ILLUSTRATIV])
```

Jede Zelle trägt „Sieg ja/nein · Zeit bis zum Sieg · verbleibende Lebenspunkte" —
**einmal gerechnet, danach nur nachgeschlagen.** Bei 350 000 Kämpfen/s ist die heutige
Tabelle in Millisekunden voll.

> **Die beiden Endausbau-Zahlen sind keine gemessenen Größen, und sie dürfen keine Zusage
> tragen.** Die **5 412** entstanden unter benannten Modellannahmen (gleichförmige Affixmuster,
> modellierte Satzverteilung, kein realer Endgame-Katalog) und waren ausdrücklich als
> **Untergrenze** bezeichnet. Die **368 016** sind ihr Produkt mit 68 — und gelten nur bei einer
> **fest deklarierten Referenz-Spielweise** ohne Talent-, Schwierigkeits- und Leiterdimension
> (siehe Kasten unten).
>
> **Keine dieser Zahlen darf zur Begründung der 2–4-Stunden-Zusage herangezogen werden.**
> Dasselbe gilt für den Talentfaktor **234**: Die Projektion ist geprüft, die Übertragung auf den
> vollen Phänotypraum ist eine Ableitung, deren Zählung zweimal gescheitert ist (Teil 6.2).

> **KORRIGIERT am 2026-08-14, dritte Runde: Der Schlüssel `Gear-Phänotyp × Boss` ist zu klein.**
> Ob ein Stand einen Boss schafft, hängt nicht nur an der Ausrüstung, sondern auch an Talenten,
> Zaubermenge, Rotation, Schwierigkeit, Leiterstufe und Spielerstufe. **Die Zahl 368 016 gilt
> nur, wenn eine Referenz-Spielweise ausdrücklich festgeschrieben ist.**

Es gibt genau zwei zulässige Fassungen, und die Wahl gehört in das Issue:

```rust
// A — feste Referenz-Spielweise: billig, aber die Aussage ist enger
ProgressOracleKey { gear_phenotype, fixed_reference_build, boss, difficulty, ladder_level }

// B — beste Spielweise: teuer, weil jede Zelle eine innere Suche ist
ProgressOracleKey { gear_phenotype, niche, boss, difficulty, ladder_level, objective_version }
```

**Fassung A ist der Anfang** — sie beantwortet „schafft ein durchschnittlicher Spieler das?"
und muss dann auch so beschriftet werden. **Fassung B beantwortet „ist es schaffbar?"** und
kostet je Zelle einen LNS-Lauf statt eines Kampfes. Beide Fragen sind berechtigt; sie
unterschiedslos „Orakel" zu nennen wäre die Verwechslung.

**Und die Invalidierungs-Matrix (E15.4) braucht dafür eigene Zeilen**, die heute fehlen:
geänderte Fallrate · geänderte Beutetabelle · geänderte Aufwert-Regel · geänderte
Freischaltbedingung · geänderte Anlege-Strategie. **Sie machen die Fortschritts-Simulation
ungültig, nicht die Kampf-Tabelle** — ein Unterschied, der viel Rechenzeit spart.

**Die Tabelle ist der Ergebnis-Cache aus E11.4b**, nur mit dem Gear-Phänotyp als Schlüssel
statt einer Rotation. Wer den Cache baut, bekommt diese Simulation fast geschenkt — das ist
der Grund, warum sie hier steht und nicht in einem eigenen Epic.

Danach kostet eine vollständige Spieler-Laufbahn — hunderte Kämpfe, tausende Beutewürfe —
praktisch nichts. Zehntausend simulierte Spieler liefern **Verteilungen statt Annahmen**:

| Frage                                        | Antwort, die die Schleife liefert     |
| -------------------------------------------- | ------------------------------------- |
| Wie viele Kämpfe bis Instanz 4?              | Median, p10, p90                      |
| Wie oft bleibt jemand dauerhaft stecken?     | der Frust-Indikator                   |
| Ist Instanz 3 zu schwer für den Stand aus 2? | Sieg-Anteil je Zelle                  |
| Lohnt Aufwerten gegenüber Weiterspielen?     | beide Wege simulieren und vergleichen |

#### Was es im Plan schließt

Der Plan führt unter „Noch nicht ausgeplant" den Punkt **„Endlos-Leiter-Zahlen — Prinzip
steht (Werte gedeckelt, Stufen unbegrenzt), Skalierung je Stufe fehlt."** Mit dieser Schleife
wird die Skalierung **messbar statt geraten**: Man setzt eine Kurve, simuliert tausend
Spieler, liest ab, wo sie steckenbleiben.

**Fertig, wenn:** die Verteilung „Kämpfe bis Instanz N" für alle heutigen Instanzen steht,
mit Median und p10/p90 — und der `erwarteterStand` aus `expected-gear.ts` daneben, damit
sichtbar wird, ob die gesetzte Annahme trägt.

#### Zwei Blocker, beide schon benannt

**Der Werte-Deckel** (sonst konvergiert der Endausbau auf einen Zauber, und jede
Fortschrittskurve rechnet gegen einen Zustand, den es so nicht geben soll) und die
**Schwierigkeitsstufen**, die es weder im Spiel noch im Simulator gibt — `rates.ts` kennt drei
(`normal`, `heroic`, `mythic`), `app.ts` führt eine feste `difficulty: 1`.

Ohne die Stufen läuft die Schleife nur auf einer Ebene. Das ist ein brauchbarer Anfang, aber
kein vollständiges Bild.

### E11.5 — Die Suche auf LNS umstellen

Gemessen, 60 Fälle mit bekanntem Optimum:

```
greedy ohne Paare   80,0 %    1 770 Messungen
greedy mit Paaren   83,3 %    1 892
beam k=10           85,0 %    8 010
LNS                 91,7 %    9 969
QD (Archiv)         68,3 %    6 337
erschöpfend        100,0 %   62 328
```

**Die Balance-Aussage hängt daran, nicht die Geschwindigkeit:** Mit greedy liegt jede sechste
Aussage „Zauber X gewinnt hier nicht" daneben, mit LNS jede zwölfte.

Umsetzung wie im Vorschlag — greedy+Paare als Warm-Start, LNS als Hauptsuche, MAP-Elites nur
als Archiv. **Die Nischen-Achsen stehen** (6.1): die Zauberschule als erste Achse, dazu zwei
bis drei aus der Wirkungsbeschreibung abgeleitete. Eine Ersatzachse ist damit nicht mehr
nötig — und ein Archiv, das mit anderen Achsen gebaut wurde, lässt sich nachträglich nicht
verlustfrei umdeuten. Also von Anfang an mit den richtigen.

**Fertig, wenn:** LNS im Läufer steht und die 91,7 % reproduziert. Danach dieselbe Reihe auf
größeren exakt lösbaren Projektionen (10–12 Kandidaten, Länge 5) — trägt der Vorsprung dort
auch?

### E11.5a — Adaptive LNS: Operatoren benennen und ihr Budget messen

„LNS" ist ein Rahmen, kein Verfahren. Was fehlt, sind die Operatoren und die Frage, wie viel
Budget jeder bekommt.

| Ebene         | Operatoren                                                                                                |
| ------------- | --------------------------------------------------------------------------------------------------------- |
| Rotation      | eine Zeile ersetzen · 2–4 Zeilen zerstören und reparieren · Teilfolge umordnen · bedingt ↔ unbedingt      |
| Talente       | einen Zweig leeren · Punkte verschieben · Schlüsseltalent erzwingen · mit der Talent-DP gültig reparieren |
| Ausrüstung    | Nachbar-Phänotyp · dominantes Affix wechseln · Setstufe aktivieren · legendäre Maske ändern               |
| **gekoppelt** | Zauber wechseln **+** abhängige Talente **+** Ausrüstung neu ausrichten **+** Rotation lokal reparieren   |

**Die gekoppelten Operatoren sind der Punkt.** Ein neu eingesetzter Zauber wirkt sonst
künstlich schwach, weil Talente und Ausrüstung noch auf den alten ausgerichtet sind — und
genau das würde als „Zauber X ist zu schwach" in eine Balance-Aussage eingehen. Das ist kein
Suchproblem, sondern eine Fehlerquelle in der Aussage, für die das ganze Werkzeug gebaut wird.

**Budget nach Ertrag statt nach fester Gewichtung.** Jeder Operator wird als Arm eines
mehrarmigen Banditen geführt:

```
Belohnung  =  (Verbesserung + λ · neue Nischenzelle) / EINGEFRORENE logische Werteinheiten
```

> **Nicht „verbrauchte Auswertungen" — das wäre nicht reproduzierbar.** Ob ein Kandidat einen
> Vollkampf kostet oder aus dem Cache kommt, hängt von der Cache-Wärme, einem greifenden
> Ledger, GPU-Verfügbarkeit, einer Remote-Wiederholung, der RAM-Verdrängung und Rechnerausfällen
> ab. Derselbe Lauf würde einem Operator bei warmem Cache mehr Belohnung geben, danach **andere
> Operatoren wählen** — und am Ende andere Kandidaten untersuchen.
>
> ```rust
> LogicalTaskId  { campaign_hash, phase, epoch, operator_id, candidate_id, trial_block }
> WorkAccounting { canonical_units,      // genau EINMAL belastet
>                  physical_attempts,    // nur Telemetrie
>                  physical_cycles }     // nur Platzierung und ETA
> ```
>
> ```
> Cachetreffer · analytisch statt Vollkampf · GPU statt CPU   →  GLEICHE logische Kosten
> Remote-Wiederholung · spekulative Zweitausführung
>   · RAM-bedingte Neuberechnung                              →  NULL zusätzliche Kosten
> ```
>
> **Abnahme:** dieselbe `CampaignDefinition` mit kaltem Cache, warmem Cache, einem Rechner,
> drei Rechnern und mitten im Lauf geänderter CPU-Quote erzeugt **dieselben logischen
> Aufgaben-Kennungen, dasselbe Archiv, dasselbe Zertifikat**. Nur physische Zähler und ETA
> dürfen abweichen.

**Fertig, wenn:** ein Lauf je Operator druckt, wie viel Budget er bekam und was er einbrachte —
und die adaptive Variante gegen feste Gleichverteilung gemessen ist. **Bringt sie nichts, ist
das Ergebnis die feste Gewichtung**, und die Bandit-Ebene entfällt.

### E11.5b — Exakte Löser für die Projektionen, plus ein Optimum-Orakel

**Die Erkenntnis, die der Plan bisher nicht ausspricht:** Verschiedene Projektionen brauchen
verschiedene Verfahren. Ein Universal-Suchalgorithmus ist für keine davon der beste.

| Projektion                      | Verfahren                                      |
| ------------------------------- | ---------------------------------------------- |
| Talentpunkte                    | DP über Baum und Punktsumme (steht)            |
| Ausrüstungsplätze               | DP / Meet-in-the-Middle + Phänotyp-Dedup       |
| Set- und Legendär-Kombinationen | Constraint-Solver                              |
| Zauberpaare                     | vollständige kleine Aufzählung                 |
| Rotation                        | LNS, mit B&B als Referenz                      |
| numerische Zauberwerte          | Trace-Region + inverse Treppenfunktion (E12.2) |
| Krit an Phasenbossen            | Verteilungs-DP (E11.1b)                        |
| Bossmenge                       | Constraint-Generation (E11.5c)                 |

**Das Optimum-Orakel.**

> **„Die Zeit läuft vorwärts, also ist der Graph azyklisch" — das reicht als Begründung nicht.**
> Derselbe Abschnitt zählt weiter unten die Gegenbeispiele auf: `freesNextGcd`,
> `refundsInterruptedGcd`, `doublesNextCast`, Zauber ohne GCD, Kanäle und Rücksetzer erlauben
> **mehrere Entscheidungen beim selben Zeitstempel**. Eine Kette solcher Effekte kann im
> Grenzfall einen **Nullzeit-Zyklus** bilden — und ein DAG-Löser darauf terminiert nicht.
>
> **Nötig ist eine wohlfundierte Fortschrittsfunktion**, die jeder Übergang streng erhöht:
>
> ```
> ρ(s) = (time_tick, −verbleibende endliche Ladungen, event_sequence, Nullzeit-Budget)
> ```
>
> Alternativ: Nullzeit-Aktionen vollständig abschließen, bevor der nächste Zeitknoten entsteht,
> **mit Zyklenerkennung** — ein unbegrenzter Nullzeit-Zyklus wird dann als **Datenfehler
> abgelehnt**, nicht endlos gerechnet.
>
> **Wo echte Rückkanten bleiben, ist kein DAG-Löser zuständig**, sondern allgemeine DP mit
> Zyklenerkennung, Wert-/Strategie-Iteration oder eine Zerlegung in starke
> Zusammenhangskomponenten.

Unter dieser Bedingung ist der Zustandsgraph über den Kampfhorizont azyklisch. Damit ist der beste **Aktionsplan** (nicht: die beste APL) mit
Branch-and-Bound exakt bestimmbar, mit einer optimistischen Schranke aus entspannten
Kapazitäten:

```
K   = ⌊(T - t) / GCD⌋              verbleibende Aktionsplätze
uᵢ  = 1 + ⌊(T - t) / CDᵢ⌋          maximale Einsätze von Zauber i
U(s) = max Σ xᵢ·dᵢᵐᵃˣ    unter   0 ≤ xᵢ ≤ uᵢ ,  Σ xᵢ ≤ K
```

Ressourcen, Verstärkungsfenster und Bossmechaniken werden ignoriert — das Ergebnis ist
absichtlich zu hoch und damit eine sichere Obergrenze.

> **DIESE SCHRANKE IST SCHON HEUTE UNSICHER — nachgemessen am 2026-08-14.**
>
> Die Vorfassung schrieb: _„sobald der Cooldown-Rücksetzer **kommt**"_ und stützte sich darauf,
> dass `cooldown_reset.rs` von keinem Datensatz genutzt werde. **Das war falsch.** Gezählt in
> `data/spells/sublimation.json` — die Schule ist genau dafür gebaut:
>
> ```
> cooldownReductionMs            4 Zauber   vapour · wisp · ascend · decant
> cooldownResetCharges           2          reflux · alembic
> cooldownReductionBelowMs       1          ascend
> cooldownReductionLongestOnly   1          decant
> freesNextGcd                   1          sublime
> doublesNextCast                1          condense
> refundsInterruptedGcd          1          retort
> ───────────────────────────────────────────────────────────
> 9 von 58 Zaubern, und der Simulator kennt sie   (data.rs:424, 437, 458, 501)
> ```
>
> **Beide Schranken fallen, nicht nur eine:**
>
> ```
> uᵢ = 1 + ⌊(T−t)/CDᵢ⌋     bricht bei cooldownReset*, cooldownReduction*
> K  = ⌊(T−t)/GCD⌋          bricht bei freesNextGcd, refundsInterruptedGcd,
>                           doublesNextCast, Zaubern ohne GCD, Kanälen,
>                           und Satz-Effekten, die den GCD ändern
> ```
>
> **Eine zu niedrige „Obergrenze" verwirft den optimalen Ast** — und zwar still. Das ist die
> gefährlichste Fehlerklasse in diesem ganzen Plan: kein Absturz, kein falsches Ergebnis im
> Test, nur eine Balance-Aussage, die die beste Rotation nie gesehen hat.

**Deshalb wird die Zulässigkeit aus dem Build abgeleitet, nicht angenommen.** `CompiledBuild`
(E11.4d) trägt dazu:

```rust
UpperBoundCapability {
    gcd_bypass:              bool,   // freesNextGcd, off-GCD, refundsInterruptedGcd
    cooldown_resets:         bool,   // cooldownResetCharges
    cooldown_reduction:      bool,   // cooldownReduction{Ms,BelowMs,LongestOnly}
    extra_casts:             bool,   // doublesNextCast
    channel_partial_rewards: bool,
    action_count_upper_bound: Option<u32>,
}
```

**Drei Stufen, und die dritte ist die wichtigste:**

| Fall                                                              | Schranke                                                  |
| ----------------------------------------------------------------- | --------------------------------------------------------- |
| alle Effektklassen des Builds werden von der Herleitung abgedeckt | **enge Schranke**, volles Pruning                         |
| nur ein sicherer Höchstschaden je verbleibendem Takt bekannt      | **lockere Schranke**, schwaches Pruning                   |
| eine Effektklasse ist nicht abgedeckt                             | **kein Pruning** — B&B läuft ohne Schranke oder gar nicht |

**Eine Effektart darf nie stillschweigend vergessen werden.** Deshalb wird
`UpperBoundCapability` nicht von Hand gepflegt, sondern aus der Wirkungs-IR erzeugt (Teil 4.6) —
ein neuer Effekt-Opcode erzwingt dort einen Übersetzungsfehler, bis er eingeordnet ist. Liegt `aktueller Schaden + U(s)` unter
der aktuellen Elite, ist der ganze Ast exakt verwerfbar.

> **Der Vorbehalt, den das Urteil nicht nennt, und er begrenzt die Aussage.** Ein freier
> Aktionsplan darf in jedem Moment jeden bereiten Zauber wählen; eine APL mit 12 Zeilen kann
> das nicht. Der B&B-Wert ist deshalb eine **lockere** Obergrenze — er misst die Lücke zwischen
> LNS und dem freien Optimum, nicht die zwischen LNS und der besten APL. **„LNS liegt 20 %
> darunter" heißt nicht, dass LNS 20 % verpasst**, sondern dass Prioritätslisten als
> Ausdrucksmittel begrenzt sind. Nur der umgekehrte Schluss trägt: Liegt LNS **dicht** an der
> Schranke, ist weiteres Suchen nachweislich sinnlos.

**Und wir haben schon eine schärfere Referenz** — `exhaustive()` löst 60 kleine Fälle exakt
(Grundlage der 91,7 %). Der Wert von B&B liegt darin, diese Referenz auf **größere** Fälle
auszudehnen, bei denen erschöpfendes Rechnen aufhört.

**Dominanz-Pruning, mit Beweispflicht.** Zustände und Ausrüstungsprofile, die in allen
relevanten Größen schlechter sind, dürfen verworfen werden.

> **Aber Monotonie ist bei ROTA nicht selbstverständlich, und es gibt einen harten Beleg.**
> Kandidatenzeilen tragen Schwellenbedingungen (`heat >= 15`); mehr Ressource kann eine Regel
> **früher** auslösen und ein Fenster verpassen. **Schlimmer noch bei `runaway`-Bossen**
> (GAME-DESIGN § 11.2, Baustein 6): Dort ist der Ring auf 0 die **Niederlage**, Schaden senkt
> ihn, und Schutz und Heilung heben ihn. „Mehr Schaden" ist dort nicht schwächer monoton,
> sondern **umgekehrt**. Zwei Bosse tragen das heute.
> **Jede Dimension, über die dominiert wird, braucht einen Nachweis in der
> `dependency_mask`**, nicht die Annahme „mehr ist besser". Ohne den Nachweis verwirft das
> Pruning die richtige Antwort, und zwar still.

**Fertig, wenn:** B&B für eine Projektion mit 10–12 Kandidaten dasselbe Optimum liefert wie
`exhaustive()`, und für eine Größe, die `exhaustive()` nicht mehr schafft, eine Schranke steht.

### E11.5d — Der exakte Prioritätslisten-Löser

**Warum das freie Aktionsplan-Orakel dafür nicht genügt** — E11.5b sagt es selbst: Es optimiert
einen **freien** Aktionsplan und ist damit nur eine **lockere** Obergrenze für eine
Prioritätsliste. Daraus folgt:

```
LNS liegt DICHT an der freien Schranke   →  belastbar: weiteres Suchen lohnt nicht
LNS liegt WEIT darunter                  →  sagt NICHTS: schlechte Suche oder
                                            begrenzte Ausdruckskraft der APL?
```

**Für `CertifiedOptimal`, `CertifiedNecessary` und `CertifiedDominated` (E12.6) reicht das
nicht.** Diese drei Aussagen betreffen die beste **APL**, nicht den besten freien Plan.

**Der Löser arbeitet über Präzedenzen, nicht über Permutationen:**

```
binäre Variablen  r_i ≺ r_j          für jedes Regelpaar
Antisymmetrie und Transitivität      als Nebenbedingungen
je erreichbarer enabled-Menge:       die höchstpriorisierte gültige Regel gewinnt
                                     — als LAZY CONSTRAINT, wenn die Menge auftaucht
Symmetriebrechung                    über äquivalente Regeln (E11.4f)
freier Aktionsplan                   als obere Schranke
```

Umsetzbar als eigener Branch-and-Bound über partielle Ordnungen oder mit einem
Constraint-Löser — **im kalten Pfad**, nie zur Kampfzeit.

> **Er muss den Rotationsraum nicht lösen.** Er ist ein **Zertifizierungsorakel für
> Projektionen**, auf denen E12 eine Balance-Aussage trifft — und er misst das LNS-Bedauern
> **innerhalb der APL-Sprache** statt gegen eine Schranke, die keine APL je erreicht.

**Fertig, wenn:** für eine Projektion, die `exhaustive()` noch schafft, dasselbe Optimum
herauskommt — und für eine, die sie nicht mehr schafft, eine **APL-Schranke** steht, die enger
ist als die des freien Plans.

### E11.5c — Boss-Active-Set: nicht jeden Kandidaten gegen 68 Bosse

**Das ist der klarste Endausbau-Punkt des ganzen Urteils.** Der Plan sieht **68 Bosse** vor;
der Bestand hat 17. Jeden Zwischenkandidaten gegen alle zu prüfen, wächst mit derselben Zahl,
die das Spiel größer macht.

```
kleines aktives Bossset
        ↓
Build optimieren
        ↓
Elite gegen ALLE Bosse prüfen
        ↓
neuer Boss, an dem sie scheitert?
        ├─ ja  → ins aktive Set, von vorn
        └─ nein → Elite bestätigt
```

Das aktive Set trägt bevorzugt die Bosse mit der höchsten Todesrate, dem größten Abstand zur
Elite, ungewöhnlicher Resistenz, HP-Phasen und Unterbrechungszwang. Ein Kandidat, der schon am
ersten Pflichtboss lexikographisch verliert, läuft die restlichen gar nicht erst.

**Fertig, wenn:** die bestätigte Elite dieselbe ist wie beim Prüfen gegen alle Bosse — an den
17 heutigen nachgewiesen, mit der Zahl der eingesparten Auswertungen daneben. **Die
Gleichheit ist das Kriterium, nicht die Ersparnis:** Ein aktives Set, das eine andere Elite
liefert, ist ein Fehler, kein schnelleres Verfahren.

**Der Geltungsbereich ist die Instanz, nicht die Bossmenge** (6.1): Es gibt keine globale
Elite. Innerhalb einer Instanz zählt der schwächste ihrer vier Bosse, und „scheitert an Boss
X" heißt: X ist der schwächste Boss dieser Instanz für diesen Aufbau. Das aktive Set arbeitet
damit **innerhalb** einer Instanz — vier Bosse statt siebzehn, im Endausbau vier statt
achtundsechzig. Der Einsparungspfad bleibt also, nur auf der Instanzebene.

### E11.6 — Compiler-Optimierung

```
lto=fat, codegen-units=1, panic=abort
target-cpu JE ZIELPROFIL — NICHT native:
   -C target-cpu=znver3       Laptop
   -C target-cpu=alderlake    proxmox
   -C target-cpu=sandybridge  gonet
   + ein portables Fallback-Artefakt
PGO mit echter Boss-/Build-Mischung
```

**Zuerst die Versionsangleichung** (1.93.1 lokal gegen 1.97.1 auf rustbuild), sonst ist das
Profil wertlos.

> **KORRIGIERT am 2026-08-14, dritte Runde.** Hier stand „**BOLT entfällt** — `llvm-bolt` ist
> nicht installiert." **Ein fehlendes Werkzeug ist ein Einrichtungszustand, keine
> Architekturentscheidung** — dieselbe Verwechslung, die ich bei `nvcc` schon einmal gemacht
> habe. Richtig ist eine Reihenfolge mit einem Auslöser:
>
> ```
> LTO  →  PGO je Zielarchitektur  →  neu messen
>      →  wenn die Diagnose (E15.3) einen Frontend- oder I-Cache-Engpass zeigt:
>         BOLT installieren und A/B messen
> ```
>
> Kein Versprechen, dass BOLT gewinnt — aber auch kein Streichen ohne Messung.

**Und PGO-Profile gehören je Zielprofil erzeugt** — `znver3`, `sandybridge`, `alderlake` —,
jeweils mit identischer Rust- und LLVM-Version. Ein Profil vom Laptop auf gonet anzuwenden
wäre dieselbe stille Fehlanpassung wie die Versionsdrift oben.

**Fertig, wenn:** der PGO-Build gegen den Normalbau gemessen ist, unter E11.0-Bedingungen, mit
identischen Trace-Hashes.

### E11.7 — AVX2, aber nur wo es homogen ist

**Erst nach E11.2 bis E11.4**, und mit der Korrektur des Vorschlags: `f64` heißt **4 Lanes**,
nicht 8. Kein `std::simd` (kein nightly), sondern `std::arch::x86_64` mit
`#[target_feature]` und Laufzeit-Dispatch.

**Reihenfolge nach Homogenität**, gemessen: Gruppen von 7,34 im Mittel, größte 30.

1. Trace-Ledger-Batches — falls E11.3 sie hergibt
2. Gear-/Talent-Phänotyp-Sweeps
3. Vollkämpfe mit gleicher Gestalt

> **NACHGETRAGEN am 2026-08-14: gonet ist nicht auf „skalar" beschränkt.** Der i7-3930K hat
> **AVX** — und AVX1 verarbeitet bereits **vier `f64` in einem 256-Bit-Register**. Was ihm
> fehlt, sind die ganzzahligen 256-Bit-Operationen von AVX2 und das verschmolzene
> Multiplizieren-Addieren.
>
> Für **arithmetische, verzweigungsarme** Arbeit — Ledger-Auswertungen, Gear- und
> Talent-Sweeps, Faktorberechnung — ist ein eigener `AvxF64Backend` deshalb sinnvoll. Nur der
> volle, verzweigte Kampf bleibt dort voraussichtlich skalar.

**Die Backend-Leiter, vollständig:**

```rust
ScalarBackend          überall
AvxF64Backend          gonet  (AVX1, 4 Lanes f64, KEIN FMA)
Avx2F64Backend         Laptop + proxmox
CudaBackend            Laptop
VulkanComputeBackend   später, optional
```

**Ohne FMA rechnet der AVX1-Pfad sogar leichter paritätstreu** — die Rundungsfrage aus der
Auflage unten stellt sich dort nicht.

**Harte Auflage:** FMA ändert die Rundung. Der AVX2-Pfad muss dieselbe
Multiplikationsreihenfolge einhalten wie der Referenzpfad, sonst bricht die Rust/TypeScript-
Parität — und die ist laut Plan Pflichttest bei jeder Änderung.

### E11.7b — `kernel_signature` muss einen Kernel auswählen, sonst ist sie ein totes Feld

**Ein berechtigter Befund am eigenen Plan.** E11.4d führt `kernel_signature` in `CompiledBuild`
— aber nirgends stand, was daraus folgt. Ein Feld ohne Leser ist toter Code.

Aus der Signatur werden spezialisierte Kernel gewählt, statt in einem Universalkernel Zweige zu
prüfen, die dieser Build nie nimmt:

```
DamageOnly · Dot · Channel · Shield · Control · Phase · GenericFallback
```

Ein Build ohne Brand braucht keine Brand-Arrays, keine Brand-Ereignisse, keine
Brand-Verzweigungen. Ein Build ohne Kanal braucht keine Abbruchlogik.

**Warum das mehr sein könnte als eine Mikrooptimierung:** Gemessen sind **155 Fehlsprünge je
Kampf bei IPC 2,65** — der Kernel ist verzweigungsgebunden, nicht rechengebunden. Weggefallene
Zweige wirken dort direkt, während eine schnellere Multiplikation nichts ändert. **Das ist eine
Erwartung, keine Zusage** — sie steht neben E11.2 und wird gegen dieselbe Baseline gemessen.

**Die Grenze ist die Kombinatorik.** Sieben Merkmale sind 128 Masken; jede zu erzeugen ist
Codegrößenverschwendung, die den Befehlscache kostet — und damit genau die Ressource, die das
Issue verbessern will. Deshalb: **die häufigsten Masken monomorphisieren, alles Übrige über den
Fallback.** Welche häufig sind, sagt die Boss-/Build-Mischung aus E11.0, nicht eine Schätzung.

**Fertig, wenn:** die Fehlsprünge je Kampf gegen den Universalkernel gemessen sind, bei
identischen Trace-Hashes — und dokumentiert ist, welche Masken monomorphisiert wurden und wie
viel Codegröße das kostet.

### E11.7c — Gestufte Spezialisierung zur Laufzeit (optional)

Die Kernel-Spezialisierung aus E11.7b entscheidet **zur Bauzeit**. Für `StructuralKey`s, die
millionenfach ausgewertet werden, wäre eine zweite Stufe denkbar: den Kernel zur Laufzeit
erzeugen, wenn eine Strukturklasse eine Häufigkeitsschwelle überschreitet — mit allen konstanten
Effekten, Bossregeln und inaktiven Mechaniken herausgefaltet.

```
N_min  >  Übersetzungskosten / (Kosten generisch − Kosten spezialisiert)
```

**Ein leichtgewichtiger Übersetzer wie Cranelift wäre dafür angemessen**, kein voller LLVM-JIT
je Build. **Auflagen:** nur je heißer Strukturklasse (nie je Kandidat) · Reihenfolge der
Gleitkommaoperationen unverändert · Codecache je Zielarchitektur · Vergleich gegen den skalaren
Referenzpfad · **automatische Abschaltung, wenn sich die Übersetzung nicht amortisiert**.

**Als optional gekennzeichnet, und das ist ernst gemeint:** Er kommt nach PGO und
Kernel-Spezialisierung, und nur wenn die Diagnose (E15.3) zeigt, dass generische Zweige noch
messbar kosten.

> **Und er braucht ein hartes Codebudget, sonst frisst er den Befehlscache, den er verbessern
> soll:**
>
> ```
> Höchstzahl erzeugter Kernel · Höchstgröße des Codecaches
> Verdrängung nach Ersparnis je Byte · lokale Übersetzung je Zielarchitektur
> JIT-, IR- und Übersetzerversion IM Cache-Schlüssel
> ```
>
> **Spezialisiert wird nach `KernelSignature` + Bossstrukturklasse + aktiven Effekt-Opcodes** —
> **nicht** nach vollständigem `StructuralKey`. Sonst entsteht ein Kernel je Kandidat, und die
> Codezahl explodiert genau dort, wo der Gewinn liegen sollte.

### E11.8 — CUDA: nicht in diesem Epic

Drei Gründe, alle gemessen:

```
nvcc                      nicht installiert
größte homogene Gruppe    30 Kandidaten
verzweigungsgebunden      155 Fehlsprünge je Kampf, IPC 2,65
```

Der Vorschlag kommt selbst zu diesem Schluss. **E11 endet vor CUDA.** Wenn die Ledger-Batches
aus E11.3 groß und homogen werden, ist das ein eigenes Epic mit eigener Abnahme — die
Schwelle liegt laut Vorschlag bei 50 000–100 000 Kandidaten je Batch — **ein Messbereich, kein
Kriterium.** Abgenommen wird über die **Break-even-Marge** aus E13.

---

### 4.4 E12 — Der Balance-Optimierer

_Bis 2026-08-14 stand E12 nur als Tabellenzeile und als Ablaufskizze in Teil 4.7. **Das war die
größte Lücke des Plans**: E11 macht den Simulator schnell und die innere Buildsuche gut — E12
ist das, wofür das ganze Werkzeug gebaut wird._

#### E12.0 — Der Zielvertrag: harte Bedingungen vor Optimierungsgrößen

**Keine gewichtete Summe.** Eine Zahl aus `Schaden + a·Verhindert + b·Überlebt` versteckt die
Balance-Entscheidung in ihren Gewichten — und bei `runaway`-Bossen liefert „mehr Schaden ist
besser" die falsche Antwort (Teil 1, Punkt 2).

```rust
BalanceObjective {
    hard_constraints,          // MÜSSEN gelten — sonst ist der Kandidat ungültig
    lexicographic_dimensions,  // Rangfolge, keine Gewichte
    coverage_obligations,      // was NACHGEWIESEN sein muss
    robustness_requirements,
    designer_change_cost,      // wie teuer ist eine Änderung an den Daten
}
```

**Diese fünf Bedingungen gehören auf drei verschiedene Ebenen** — ein einzelner Kandidat kann
gar nicht erfüllen, dass _jeder Zauber_ eine Nische hat:

```rust
CandidateFeasibility {          // gilt für EINEN Kandidaten
    encounter_win_condition,    // bei runaway die umgekehrte
    mandatory_mechanics_solved,
    parameter_bounds,
}

PortfolioConstraints {          // gilt für die MENGE der Lösungen
    every_mechanic_has_multiple_solution_families,
    no_school_is_globally_mandatory,     // § 11.3 des Designs
    robustness_across_encounters,
}

CampaignCoverage {              // gilt für den LAUF als Ganzes
    every_spell_has_viable_niche,        // C.1c Punkt 2
    every_key_talent_is_represented,
    every_intended_gear_archetype_is_represented,
}
```

> **Und „Sieg" darf nicht zweimal auftauchen** — einmal als harte Bedingung und dann wieder in
> der Rangfolge. Er ist eine **Zulässigkeitsbedingung**; die Rangfolge beginnt darunter.
>
> **Bei stochastischen Kämpfen ist eine harte Bedingung eine Wahrscheinlichkeit, kein
> Boolescher Wert eines Durchgangs:**
>
> ```
> Siegwahrscheinlichkeit    ≥ 99 %
> Mechanikerfolg            ≥ 99,9 %
> ```

**Erst danach die Rangfolge:**

```
1  Erfolgswahrscheinlichkeit über der Mindestgrenze
2  → Überlebensreserve
3  → Mechanikqualität
4  → Abstand zum Zielband
5  → Kill- bzw. Eindämmungszeit
6  → Robustheit
7  → möglichst kleine Änderung an den Daten
8  → Ressourceneffizienz
```

**Stufe 8 kam am 2026-08-17 dazu** (6.1). Ein Aufbau, der mit halbem Ressourceneinsatz
dasselbe erreicht, spielt sich merklich besser — ohne eigene Stufe gälte er als gleichwertig.
Sie steht zuletzt, weil sie nur bei Gleichstand aller sieben oberen greifen soll.

**Stufe 3 ist noch nicht definiert.** „Mechanikqualität" steht in dieser Rangfolge, wird aber
nirgends hergeleitet — anders als Mechanik**erfolg**, der als harte Bedingung oben steht. Wer
E12.0 baut, definiert sie prüfbar **oder** streicht sie und begründet das. Eine Stufe, die
niemand messen kann, entscheidet sonst still gar nichts.

> **„Sieg" steht hier NICHT mehr**, weil er eine Zulässigkeitsbedingung ist
> (`CandidateFeasibility`) — ihn zweimal zu führen hieße, einen unzulässigen Kandidaten über
> eine gute Zahl in einer unteren Stufe gewinnen zu lassen. Die Rangfolge beginnt **unterhalb**
> der Zulässigkeit.
>
> **Die Schwellen (99 %, 99,9 %) sind versionierte Felder des `RunContract`**, keine globalen
> Konstanten — ein Endgame-Boss darf eine andere Sicherheit verlangen als ein Instanz-1-Boss.

Die letzte Stufe ist keine Feinheit: Zwischen zwei gleichwertigen Lösungen gewinnt die, die
**weniger Zahlen im Spiel verändert**.

**Fertig, wenn:** der Vertrag als Datenstruktur steht und ein Kandidat, der eine harte Bedingung
verletzt, **nicht** über eine gute Zahl in einer anderen Dimension gewinnen kann — nachgewiesen
an einem `runaway`-Boss.

#### E12.1 — Parameterdomänen: ohne sie ist das äußere Problem undefiniert

```rust
ParameterDomain {
    minimum, maximum, discrete_step,
    allowed_breakpoints,          // aus E11.3a
    monotonicity_certificate,     // aus E16 — NICHT angenommen
    dependencies,
    designer_cost,
}
```

Heute gibt es genau zwei Deckel im ganzen System (`purity >= 1000`, `wardCapPermille = 750`).
**Solange die Domänen fehlen, ist der äußere Optimierer formal unbeschränkt** — das ist der
Grund, warum die Werte-Deckel-Entscheidung (Teil 6.1) nicht nur E14 blockiert, sondern auch
dieses Issue.

#### E12.2 — Ein Parameter: exakt lösen statt suchen

**Nur bei bescheinigter Monotonie** — und die kommt aus E16, nicht aus einer Annahme:

> **KORRIGIERT am 2026-08-15 — die Vorfassung war mathematisch falsch, und der Fehler wäre
> still gewesen.** Sie lautete: _„bestes Build bei x finden → seine Trace-Region bestimmen →
> darin x analytisch lösen → erst am Regionsrand neu suchen."_
>
> **Auch wenn alle Traces stabil bleiben, können sich die Scores kreuzen:**
>
> ```
> Build A ist bei x = 100 besser
> Build B ist bei x = 107 besser
> BEIDE behalten ihren Trace von x = 90 bis 120
> ```
>
> Bei x = 107 wechselt das Optimum — **ohne dass eine Trace-Grenze erreicht wurde**. Die
> Vorfassung hätte A bis 120 fortgeschrieben und einen falschen Wert **zertifiziert**.

**Richtig ist eine Hülle über alle bekannten Herausforderer**, nicht eine Funktion über den
Amtsinhaber. Für einen Zauber `X`:

```
G_X(x) = max  O_b(x)   über alle Builds, die X TATSÄCHLICH EINSETZEN
H_X(x) = max  O_b(x)   über alle Builds ohne X
```

Gesucht ist das kleinste `x` mit `G_X(x) ⪰ H_X(x)` unter allen harten Bedingungen — `⪰` ist die
lexikographische Ordnung aus E12.0.

> **Zwei Präzisierungen, ohne die die Hülle den falschen Sieger zertifiziert.**
>
> **(a) `O_b` ist ein Vektor, kein Skalar.** Die naheliegende Form
> `S_b(x) = C_b + Σ n_bi·round(a_bi·x + c_bi)` beschreibt **eine** Schadensdimension — der
> Zielvertrag hat sieben Stufen. Build A kann mehr Schaden liefern und in der **höher
> priorisierten** Mechanikdimension verlieren:
>
> ```rust
> BoundedValue<T> { lower, estimate, upper, guarantee }
>
> ParametricOutcome {
>     feasibility: ProvenFeasible | ProvenInfeasible | Unresolved,
>     success_probability:  BoundedValue<Probability>,
>     survival_margin:      BoundedValue<Fixed>,
>     mechanic_quality:     BoundedValue<Fixed>,
>     target_band_distance: BoundedValue<Fixed>,
>     completion_time:      BoundedValue<Ticks>,
>     robustness:           BoundedValue<Fixed>,
>     change_cost:          Exact<Fixed>,
> }
> ```
>
> **Punktwerte genügen nicht.** Bei Monte-Carlo-Pfaden sind das Schätzungen — ein
> lexikographisches Maximum über viele Schätzungen erzeugt einen Sieger allein aus
> Stichprobenrauschen. Verglichen wird über Intervalle:
>
> ```
> harte Schwelle:  lower ≥ Schwelle → bewiesen zulässig
>                  upper < Schwelle → bewiesen unzulässig
>                  sonst            → mehr Durchgänge oder Unresolved
> lexikographisch: die erste Dimension mit GETRENNTEN Intervallen entscheidet
>                  überlappen sie  → mehr Durchgänge oder Unresolved
> ```
>
> Verglichen wird lexikographisch. **Auf die skalare Schadensfunktion darf E12.2 nur
> reduzieren, wenn alle höher priorisierten Komponenten über die Region nachweislich konstant
> sind** — das ist eine Prüfung, keine Annahme.
>
> **(b) „Build enthält X" ist nicht „Build nutzt X".** Ein Zauber kann in der Leiste stehen und
> nie gewirkt werden — derselbe Fehler, den die Coverage-Zeugen in E21.1 abfangen. Die Domäne
> von `G_X` braucht deshalb einen Nutzungsnachweis:
>
> ```rust
> SpellUseWitness { spell_id, casts, activated_effects, contribution_interval, trace_id }
> ```
>
> **Und stärker noch ein Ablationstest** — aber **nicht als Vektorsubtraktion**: `O` mischt
> Wahrscheinlichkeiten, Zeiten, Mechanikerfolg und Änderungskosten mit verschiedenen Einheiten
> und Richtungen; `O(b) − O(b ohne X)` ist darauf nicht definiert.
>
> ```rust
> AblationFinding {
>     full_build: OutcomeEnvelope,
>     reoptimized_without_spell: OutcomeEnvelope,
>     lexicographic_effect: Better | Equivalent | Worse | Unresolved,
>     per_dimension_delta: Vec<DirectedInterval>,
>     proof_scope: ViabilityScope,
> }
> ```
>
> **Und der Unterschied entscheidet über die Aussage:** Eine **lokale** Reoptimierung ohne X
> genügt für `WitnessedViable`. Für `CertifiedNecessary` muss das **beste zulässige** Build ohne
> X exakt gelöst oder durch eine sichere Obergrenze beschränkt sein. **„Lokal neu optimiert"
> darf nie als `CertifiedNecessary` ausgegeben werden.**

```
1  bei x0 das innere Optimum finden
2  die exakte Funktion des Amtsinhabers über seine Region bilden
3  Kreuzungspunkte und Zielgrenzen bestimmen
4  AN DIESEN PUNKTEN den Herausforderer-Löser starten
5  findet er Build B:  B in die Hülle aufnehmen, Kreuzungen neu rechnen, zurück zu 3
6  kein Herausforderer schneidet die Hülle mehr:  Ergebnis ZERTIFIZIERT
```

**Das ist Spalten- bzw. Herausforderer-Generierung im Parameterraum.** Sie ist so grundlegend,
dass sie ein eigenes Issue trägt:

#### E12.2b — Die parametrische Herausforderer-Hülle

**Zweck:** Verhindern, dass E12.2 einen Wert als exakt zertifiziert, bei dem in Wahrheit ein
anderes Build gewonnen hätte. Sie führt die Hülle über alle bekannten Herausforderer, hält
`ParametricOutcome`-Vektoren statt Skalaren und ruft den inneren Löser an den Kreuzungspunkten
auf.

**Hängt an:** E12.1 (Domänen und Monotonie-Zertifikat) · E11.3a (Trace-Regionen) · E11.5
(innerer Löser) · E12.0 (lexikographische Ordnung).

**Fertig, wenn:** kleine Projektionen vollständig aufgezählt sind — **mit mindestens einem Fall,
in dem das Optimum mitten in einer gemeinsamen Trace-Region wechselt** — und die Hülle diesen
Wechsel findet. Ein Prüffall, der ohne die Hülle das falsche Ergebnis liefert, gehört dazu:
Ohne ihn ist nicht belegt, dass die Korrektur nötig war.

> **Und die Monotonie, die eine Binärsuche braucht, ist nicht die des Schadens.** Monoton sein
> muss das **Entscheidungsprädikat** — _„existiert ein Build mit Zauber X, der das Zielband
> erreicht?"_. Dass mehr Grundschaden mehr Schaden ergibt, sagt darüber nichts.

**Wo Monotonie nicht bescheinigt ist, gilt sie nicht:** `runaway`, Schwellenbedingungen in
Kandidatenzeilen (`heat >= 15`), schadensabhängige Effekte. Dort läuft E12.3.

**Die gemessene Trace-Region ist 1–5 % breit** (Teil 6.2) — die Binärsuche findet ihre Grenze
also nach wenigen Schritten, und der Regionswechsel ist der Normalfall, nicht die Ausnahme.

#### E12.3 — Mehrere gekoppelte Parameter

```
blockweise diskrete Trust-Region + Musterschritte
+ LNS im PARAMETERRAUM (nicht im Buildraum)
+ exakte Einparameter-Lösungen innerhalb der Blöcke
```

**Keine Gradientenverfahren.** Die Zielfunktion ist diskret, stückweise konstant und **springt**,
sobald der optimale Build wechselt — ein Gradient existiert dort nicht einmal lokal.

#### E12.4 — Warm-Start: das Archiv ist die eigentliche Investition

Bei einer kleinen Parameteränderung werden **weiterverwendet**: alte Elites, Trace-Regionen,
Boss-Active-Sets, LNS-Operatorstatistik, MAP-Elites-Archiv. Ein neuer Zauber macht Elites
**nicht ungültig, sondern zu Warm-Starts** (E15.4).

#### E12.5 — Encounter-Constraint-Generation

Nur **verletzte** Encounter kommen ins aktive Set (E11.5c). Die Abschlussprüfung läuft immer
gegen die **vollständige** Pflichtmenge — sonst bestätigt das Verfahren sich selbst.

#### E12.6 — Viabilität: „keine Elite gefunden" ist kein Urteil

**Das ist die Stelle, an der ein Balance-Werkzeug am leichtesten lügt.** Ein Zauber ohne Elite
kann fünf verschiedene Dinge bedeuten — und nur eines davon rechtfertigt, seine Zahlen zu ändern:

```rust
enum ViabilityFinding {
    WitnessedViable,      // eine gültige, konkurrenzfähige Elite wurde GEFUNDEN
    CertifiedOptimal,     // Optimum in einer definierten Projektion BEWIESEN
    CertifiedNecessary,   // ohne diesen Zauber sinkt das Nischenoptimum nachweislich
    CertifiedDominated,   // selbst die sichere Obergrenze verliert — DAS ist tot
    NotFoundWithinBudget, // die Suche hat ihn nicht gefunden
    Unresolved,           // Nische zu eng, oder Schranken zu locker
}
```

> **„Nachweislich beste Wahl" war zu stark.** Eine LNS-Suche ist heuristisch: Eine gefundene
> Elite ist ein **Zeuge**, kein Beweis, dass sie in ihrer Nische global gewinnt. Der frühere
> `CertifiedViable` hätte im Zertifikat „60/60 nachweislich beste Wahl" gemeldet, wo tatsächlich
> „60 gute Beispiele gefunden" richtig gewesen wäre.
>
> **Deshalb sagt das Zertifikat, welche Garantie verlangt wurde:**
>
> ```
> standard   60/60 mindestens WitnessedViable
> release    Pflichtzauber CertifiedNecessary
>            oder zertifizierte Schranken-Lücke unter definierter Toleranz
> ```

**Nur `CertifiedDominated` darf als „zu schwach" berichtet werden.** Wer `NotFoundWithinBudget`
als tot meldet, buffed einen Zauber, der gar nicht schwach ist.

> **Und es ist kein exklusiver Zustand je Zauber, sondern eine Aussage je Geltungsbereich.**
> Derselbe Zauber kann in Nische A `CertifiedNecessary`, in B nur `WitnessedViable`, in C
> `CertifiedDominated` und auf einer ungeprüften Schwierigkeit `Unresolved` sein:
>
> ```rust
> ViabilityScope    { niche_id, encounter_set_hash, difficulty_range,
>                     ladder_range, parameter_version, projection_hash }
> ViabilityEvidence { scope,
>                     existence:     NotSearched | Witnessed | CertifiedOptimal,
>                     necessity:     Unknown | CertifiedNecessary,
>                     domination:    NotProven | CertifiedDominated,
>                     search_status: Complete | BudgetExhausted | Unresolved }
> ```
>
> ```
> "irgendwo lebensfähig"  mindestens ein gültiger Zeuge
> "global dominiert"      sichere Dominanz in ALLEN Pflicht-Scopes
> "notwendig"             mindestens ein Scope mit nachgewiesenem Ohne-X-Abfall
> ```
>
> **Und `CertifiedDominated` ist nicht der einzige Grund für eine Datenänderung.** Ebenso
> rechtfertigen sie: ein Zauber ist nachweislich zu stark · eine Schule wird global
> verpflichtend · eine Fortschrittskurve blockiert · ein Zielband wird überschritten · eine
> Mechanik hat nur eine Lösungsfamilie.

**Fertig, wenn:** das Qualitätszertifikat (E21.3) **alle sechs** Zustände getrennt ausweist und
kein Lauf einen Zauber als dominiert meldet, ohne eine Schranke dafür zu haben.

#### E12.7 — Der Herausforderer: gezielt nach Gegenbeispielen suchen

Für jede behauptete Elite wird **gesucht, was sie schlägt** — nicht gehofft, dass nichts kommt:
anderer Boss · andere Schwierigkeit · anderer Talentweg · andere Ausrüstungsrichtung ·
gekoppelte Zauber-/Talent-/Gear-Mutation (E11.5a).

#### E12.8 — Abschluss

Skalarer Referenzpfad · alle Pflicht-Encounter · festes Durchgangsset oder exakte Verteilung ·
vollständiges Zertifikat · **reproduzierbare Lauf-Kennung** (E17).

#### Zuschnitt

| Issue      | Inhalt                                                         | hängt an                                                      |
| ---------- | -------------------------------------------------------------- | ------------------------------------------------------------- |
| **E12.0**  | Zielvertrag: harte Bedingungen, Rangfolge, Abdeckungspflichten | Objective-Entscheidung (6.1)                                  |
| **E12.1**  | Parameterdomänen mit Monotonie-Bescheinigung                   | **E16**, Werte-Deckel (6.1)                                   |
| **E12.2**  | ein Parameter exakt                                            | E11.3a, E12.1                                                 |
| **E12.2b** | parametrische Herausforderer-Hülle, `ParametricOutcome`        | E12.2, E11.5                                                  |
| **E12.3**  | gekoppelte Parameter                                           | E12.2                                                         |
| **E12.4**  | Warm-Start aus dem Archiv                                      | E11.5a, E15.4                                                 |
| **E12.5**  | Encounter-Constraint-Generation                                | E11.5c                                                        |
| **E12.6**  | Viabilität: `ViabilityEvidence` je Scope berechnen             | E12.0 · **E11.5d** (für `CertifiedOptimal`) — **NICHT E21.3** |
| **E12.7**  | Herausforderer                                                 | E12.6                                                         |
| **E12.8**  | Abschlussprüfung                                               | alle                                                          |

### 4.5 E13 und E14 — GPU und numerischer Vertrag

### E14 — Der Werte-Deckel ist ein Leistungs-Engpass, deshalb vorziehen

**E14 hängt heute an einer Design-Entscheidung** (Werte-Deckel, Teil 6) und steht deshalb ganz
hinten. **Das ist die falsche Reihenfolge**, denn E14 blockiert seinerseits:

```
ganzzahliges AVX2         8 × i32 statt 4 × f64 je Register
exakte GPU-Pfade          f64 ist auf einer mobilen Consumer-GPU teuer
schnelle Krit-DP          Wahrscheinlichkeitsmasse in Festkomma
CPU/GPU-Numerikvertrag    ohne ihn ist jeder GPU-Pfad eine offene Frage
```

**Nach dem Bereichsbeweis wären zusätzlich zu prüfen:** `MULX` (64×64→128), `ADCX`/`ADOX` für
breite Zwischenwerte, ganzzahlige AVX2-Pfade — **und AVX-VNNI auf dem i5-1235U**, das unter dem
heutigen `f64`-Vertrag nutzlos ist (Anhang B.2.6) und es nach E14 möglicherweise nicht mehr wäre.

**Der Werte-Deckel ist damit keine späte Designfrage, sondern eine Vorbedingung.** Er gehört in
dieselbe Runde wie die Objective-Entscheidung.

##### E14, ausgeschrieben

| Issue     | Inhalt                                                                  |
| --------- | ----------------------------------------------------------------------- |
| **E14.0** | Werte-Deckel je Wirkgröße — die Entscheidung aus Teil 6.1, als Tabelle  |
| **E14.1** | Skalierung je Datentyp: wie viele Nachkommastellen trägt welche Größe   |
| **E14.2** | **Überlaufbeweis** für jede Rechenkette, mit Zwischenwertbreiten        |
| **E14.3** | Rundungsregel und Verhalten negativer Werte                             |
| **E14.4** | **JavaScript-Grenze** — siehe unten                                     |
| **E14.5** | Parität über CPU / SIMD / GPU / TypeScript                              |
| **E14.6** | numerischer Fehler der Krit-DP (E11.1b): `f64`, Festkomma oder rational |

**Fertig, wenn** — je Teil-Issue ein prüfbarer Satz, nicht ein Thema:

```
E14.0  jede Wirkgröße hat einen Deckel ODER eine begründete "unbounded"-Ausnahme
E14.1  Einheit und Skalierung jedes gespeicherten Typs stehen fest
E14.2  jede Rechenkette hat einen MASCHINENPRÜFBAREN Überlaufbeweis
E14.3  positive UND negative Rundungs-Testvektoren bestehen in Rust und TypeScript
       — insbesondere Math.round(-1.5) gegen f64::round(-1.5), die auseinandergehen
E14.4  die number-vs-BigInt-Entscheidung ist getroffen und begründet
E14.5  scalar, AVX, AVX2, GPU und TypeScript erfüllen den festgelegten Vertrag
E14.6  die Fehlergrenze der DP-Masse UND aller daraus abgeleiteten Statistiken ist bewiesen
```

**Festkomma wird nur übernommen, wenn es dreifach besteht:** semantisch korrekt · bereichssicher
· auf mindestens einem **End-to-End**-Pfad messbar schneller. Zwei von drei genügen nicht.

> **Die JavaScript-Grenze entscheidet über den ganzen Ansatz, und sie wird leicht übersehen.**
> TypeScript `number` ist exakt nur bis **2⁵³**. Braucht ein Festkomma-Zwischenwert `u64` oder
> `u128`, gibt es drei Wege — und die Wahl muss **vor** der Umsetzung fallen:
>
> ```
> 1  Deckel so wählen, dass alle Zwischenwerte unter 2^53 bleiben
> 2  Browser-Referenz auf BigInt umstellen        (Kosten im Spiel, nicht im Simulator)
> 3  Festkomma nur als Suchpfad, finale Wahrheit bleibt der f64-Referenzpfad
> ```

> **Zwei verbreitete Irrtümer, die dieses Epic ausräumen muss:**
>
> **„FMA ist vorhanden, also nutzen wir es."** Unter dem heutigen bitgenauen `f64`-Vertrag ist
> FMA für ergebnisrelevante Formeln **abgeschaltet** — es ändert die Rundung. AVX2 verarbeitet
> auch ohne FMA vier `f64`-Bahnen mit getrennter Multiplikation und Addition.
>
> **„Festkomma bringt 8 × i32 statt 4 × f64, also doppelt so schnell."** Viele Produkte
> brauchen breitere Zwischenwerte; dann werden daraus 4 × `i64`, mehrere Teilprodukte oder
> skalare `u128`-Schritte. **Das ist zu messen, nicht vorauszusetzen.**

### E13 — Die CUDA-Pipeline, konkretisiert

E13 nennt bisher nur, **was** auf die GPU soll. Was fehlt, ist **wie**:

```
gerätefeste Tabellen für Zauber, Bosse, Ledger    (einmal übertragen)
SoA-Eingabe statt Struktur-Feldern
seitengesperrter Übergabepuffer auf der Hostseite
doppelte oder dreifache Pufferung
asynchrones Kopieren PARALLEL zur CPU-Kandidatenerzeugung
CUDA-Graphen für wiederkehrende Kernel-/Kopier-Folgen
KURZE Batches — sie sind die einzige verlässliche Drosselung
Messung von Registerdruck, Auslagerung und Belegung
```

**Reihenfolge der Portierung, und der Vollkampf steht bewusst zuletzt:**

```
1  sparsame Ledger-Differenzen        (E18a)
2  Parameter-Sweeps
3  Kandidat × Boss
4  Krit-DP und Monte-Carlo-Finalisten
5  vollständiger Kampf — erst NACH E14 und nur mit Messung
```

**Warum die Drosselung über Batchlänge läuft und nicht über Stromprioritäten:** Prioritäten
sind ein Planungshinweis, sie unterbrechen keine laufende Arbeit. Ein 2-Sekunden-Kernel bleibt
2 Sekunden, egal was der Regler will. **Kurze Batches sind die Reaktionszeit.**

**Fertig, wenn** — sieben Sätze, jeder prüfbar:

```
Transfer-, Pack- und Entpackkosten sind IN der Messung enthalten
VRAM- und seitengesperrtes Speicherbudget werden eingehalten
die maximale Batch-Reaktionszeit der Drosselung wird eingehalten
ein GPU-Fehler führt zur IDEMPOTENTEN Neuvergabe auf der CPU
CPU/GPU-Parität ist bestanden (bitweise, nach E14)
Drosselung ändert KEIN Ergebnis
die GPU schlägt den besten CPU-Pfad um eine vorher festgelegte Mindestmarge
```

> **Die Mindestmarge ist Teil des Vertrags, nicht des Ergebnisses.** „50 000–100 000 Kandidaten
> je Batch" ist ein **Messbereich**, kein Abnahmekriterium — ohne festgelegte Marge wird jeder
> gemessene Wert nachträglich zum Erfolg erklärt.

##### E15.0b — Gemeinsame CPU-/GPU-Kalibrierung

**Eine GPU-Rate allein sagt nicht, ob die GPU sich lohnt.** Auf dem Laptop teilen sich CPU,
iGPU und dGPU ein thermisches Budget und die Speicherbandbreite — die GPU zu belasten **senkt
den CPU-Takt**. Gemessen wird deshalb die Kombination, nicht der Summand:

```
CPU allein · GPU allein · CPU+dGPU · CPU+iGPU · CPU+dGPU+Zusatzrechner
```

```rust
JointPerfProfile { cpu_rate_when_gpu_active, gpu_rate_when_cpu_active,
                   best_cpu_threads, best_gpu_duty_cycle }
```

**Nur der gemessene Gesamtdurchsatz entscheidet**, ob beide gleichzeitig laufen. Dieselbe
Kalibrierung beantwortet die iGPU-Frage aus Teil 2.1, ohne dass jemand sie schätzen muss.

##### Datenlayout: der Simulator wird nicht global umgestellt

```
skalarer, verzweigter Pfad     AoS  — bleibt wie er ist
homogener SIMD-Batch           AoSoA<4> für f64
                               AoSoA<8> erst, wenn Festkomma wirklich passt (E14)
```

**Der Verteiler packt nur ausreichend große homogene Batches um** — und die Kosten dafür
(Packen, Transponieren, Restbahnen, Divergenz, Zurückschreiben) gehören **in `CostEstimate`**,
sonst wählt der Router den SIMD-Pfad auch dort, wo das Umpacken teurer ist als der Gewinn.

### 4.6 E15 — Betriebsarchitektur

_Nachgetragen 2026-08-14 auf Auftrag des Auftraggebers. E11 macht den Simulator **schnell**;
dieser Teil macht ihn **betreibbar**: drosselbar in einer geteilten Umgebung, beobachtbar
während er läuft, erweiterbar ohne Neuberechnung, und lauffähig auf einem bis drei Rechnern —
auch wenn zwei davon fehlen._

> **Warum ein eigenes Epic und kein Anhang zu E11.4c:** Der Scheduler dort verteilt Arbeit auf
> Kerne **einer** Maschine und nimmt an, dass er sie allein hat. Jede der folgenden
> Anforderungen bricht diese Annahme — und zusammen ergeben sie keine Erweiterung, sondern eine
> zweite Schicht **über** dem Evaluator.

### E15.0 — Die Fähigkeitserhebung: messen statt aus Flags raten

Jede Entscheidung dieses Teils braucht dieselbe Grundlage: **Was kann dieser Rechner
tatsächlich?**

```rust
struct HostCaps {
    // gemeldet
    threads_total, threads_usable, l2_per_core, l3, ram_total, ram_free,
    isa: IsaSet { avx, avx2, fma, bmi1, bmi2, f16c, avx512, … },
    gpu: Option<GpuCaps { vendor, vram, cuda_cc, backend }>,
    // GEMESSEN — der eigentliche Wert
    calibrated_fights_per_s: f64,
    calibration_spread: f64,
    measured_at: Timestamp,
}
```

**Der Kalibrierlauf ist der Kern, nicht die Flag-Abfrage.** Er läuft auf jedem Rechner mit
derselben Arbeit — aber **nicht zwei Sekunden lang**.

> **Feste zwei Sekunden wären genau der Fehler, den dieser Plan an fünf Stellen dokumentiert.**
> Die Baseline streute um ±12 %, weil die Läufe zu kurz waren; der erste Durchsatzwert war um
> 30 % daneben. Eine Kalibrierung, die selbst zu kurz misst, verteilt die Arbeit falsch — und
> zwar dauerhaft, weil alle folgenden Entscheidungen auf ihr stehen.
>
> **Adaptiv statt fest:**
>
> ```
> Aufwärmen → messen → weiter, bis ALLE erfüllt sind:
>     Mindestdauer 5 s          ·  Höchstdauer 30 s
>     Streuung unter 3–5 %      ·  Takt und Temperatur stabil
>     PSI-Druck akzeptabel
> ```
>
> **Und nicht bei jedem Start vollständig:** Profile werden nach (Rechner-Kennung ×
> Artefakt-Hash × Kernelklasse × Taktregime) **gespeichert** und beim nächsten Lauf nur kurz
> bestätigt. Er erfasst in einer Zahl, was aus Flags nicht ableitbar ist:

| was Flags nicht sagen  | Beispiel aus unserem Bestand                                                  |
| ---------------------- | ----------------------------------------------------------------------------- |
| heterogene Kerne       | i5-1235U meldet 12 Stränge — vier davon auf P-Cores, acht auf E-Cores         |
| Fremdlast              | gonet läuft dauerhaft bei Load 3–7; 12 Stränge sind nie 12 Stränge            |
| Speicherbandbreite     | DDR3-1333 gegen DDR4-3200, Faktor 2,4 — steht in keinem CPU-Flag              |
| Drosselung durch Wärme | der 5900HS ist ein 35-W-Mobilchip mit gemeinsamem Budget für CPU und iGPU     |
| Container-Grenzen      | ein LXC mit `cpulimit` meldet die Kerne des Hosts, darf sie aber nicht nutzen |

**Deshalb wird Arbeit nach gemessenen Raten verteilt, nie nach Kernzahl.** Die Kalibrierung
wiederholt sich bei jedem Lauf und zusätzlich, wenn die gemessene Rate um mehr als 25 % von der
erwarteten abweicht — dann hat sich am Rechner etwas geändert.

> **KORRIGIERT am 2026-08-14, dritte Runde: EINE Zahl je Rechner genügt nicht.** Ein Host kann
> für verschiedene Arbeitsarten völlig verschieden geeignet sein — und unsere drei sind genau
> so verschieden:
>
> - **gonet** ist bei verzweigtem skalarem Code brauchbar (121 689/s gemessen), bei AVX2
>   **gar nicht** — er hat es nicht.
> - **proxmox** dürfte in einem arithmetischen AVX2-Stapel relativ stärker sein als im
>   verzweigten Vollkampf, weil dort die E-Cores mithalten.
>
> Ein einziger `calibrated_fights_per_s` mittelt das weg und verteilt dann falsch.

```rust
struct HostPerfVector {
    scalar_full_fight: RateModel,
    avx_full_fight:    Option<RateModel>,    // AVX1 auf gonet, AVX2 sonst
    ledger_eval:       RateModel,
    crit_dp:           RateModel,
    branch_and_bound:  RateModel,
    gear_sweep:        RateModel,
    gpu_bulk:          Option<RateModel>,
    network_rtt, network_bandwidth,
    current_pressure,                        // PSI, aus E15.1
}
```

**Und die P-/E-Kerne des i5-1235U bekommen getrennte Arbeitsvorräte.** Die Batchvergabe über
einen Zähler gleicht zwar von selbst aus (E11.4c) — aber Strangwanderung zwischen P und E und
die unterschiedlichen Cachegrößen bleiben vermeidbare Störgrößen:

```
P-Kern-Vorrat    2 Kerne / 4 Stränge, 4,4 GHz, 1,25 MiB L2 je Kern
E-Kern-Vorrat    8 Kerne / 8 Stränge, 3,3 GHz, 512 KiB je Kern
```

je mit eigener Kernbindung, eigener Kalibrierung, eigener Chunkgröße und eigenem
Durchsatzmittel.

**Fertig, wenn:** die Kalibrierung auf allen drei Systemen läuft und ihr Wert die gemessenen
Durchsätze aus Teil 2.1 innerhalb von 10 % trifft. Trifft sie nicht, ist der Kalibrierlauf nicht
repräsentativ für die echte Arbeit — und dann ist er wertlos.

### E15.1 — Der Ressourcen-Balancer: drosseln ohne abzubrechen

**Die Anforderung:** Der Simulator läuft in einer geteilten Umgebung. Wird er zu gierig, muss er
zurückweichen — **ohne den Lauf zu verlieren** und **ohne das Ergebnis zu ändern**.

#### Drei Stellschrauben, drei Mechanismen

| Ressource | Mechanismus                                                                                                                                                                                    | Reaktionszeit    |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| **CPU**   | Stränge **parken**, nicht beenden. Jeder Strang prüft zwischen zwei Chunks eine Marke und legt sich schlafen. **Chunks werden in Millisekunden bemessen, nicht in Kandidaten** (Kasten unten). | eine Chunk-Länge |
| **RAM**   | **Nicht nur der Ergebnis-Cache** (siehe Kasten). Ein zentrales Budget senkt die Teilbudgets nach Kosten-Nutzen; Verworfenes wird **neu gerechnet**, nicht vermisst.                            | sofort           |
| **GPU**   | Batch-Größe senken, im Grenzfall das Backend abmelden — der Router (E11.4b) fällt dann auf den CPU-Pfad zurück, weil er ohnehin jeden Zweig als optional führt.                                | ein Batch        |

**Warum das ohne Abbruch geht, ist eine Eigenschaft der Architektur, keine Zusatzarbeit:** Der
Scheduler aus E11.4c vergibt Arbeit über einen Zähler und schreibt nach `result[candidate_id]`.
Ein Strang, der pausiert, hält kein Zwischenergebnis — er hat seinen Chunk entweder fertig oder
noch nicht angefangen.

> **KORRIGIERT am 2026-08-14, dritte Runde.** Hier stand, der Ergebnis-Cache sei „der einzige
> wachsende Verbraucher". **Das ist falsch, und es sind neun weitere:** MAP-Elites-Archiv ·
> LNS-Elites und Operatorstatistik · Checkpoints · Trace-Ledger · Ergebnisfelder ·
> Pakete unterwegs zu den Zusatzrechnern · GPU-Übergabepuffer · Krit-DP-Zustände ·
> B&B-Transpositionstabelle. **Auf einer Maschine mit 2 GiB frei entscheidet ihre Summe**,
> nicht der größte Einzelposten.

```rust
MemoryBudgetManager {
    total_budget, emergency_reserve,
    result_cache, trace_cache, search_archive,
    checkpoints, dp_states, gpu_staging, inflight,
    candidate_window,                    // E11.4c — fehlte
}
```

> **Aber nicht alles darin darf verdrängt werden.** Ein MAP-Elites-Archiv, eine Pareto-Front,
> ein laufender B&B-Frontier und die Operatorstatistik sind **Bestandteil des
> Suchalgorithmus** — wer sie nach Speicherdruck wegwirft, lässt die Drosselung das **Ergebnis**
> ändern. Genau das soll E15.1 verhindern.
>
> ```rust
> enum MemoryClass {
>     RecomputableCache,      // Outcome-, Trace-, Ledger-Cache → kostenbewertet verdrängen
>     SpillableExactState,    // Archive, Zeugen, Trial-Akkumulatoren → NUR verlustfrei
>                             //   auf NVMe auslagern
>     NonEvictableWorkingSet, // aktiver DP-/B&B-Frontier, laufender GPU-Batch →
>                             //   VOR Jobstart reservieren (Admission Control)
> }
> ```
>
> **Passt ein Job nicht:** aufteilen, anderen Rechner wählen, auslagerungsfähigen Algorithmus
> nehmen oder warten — **nie** das Archiv verkleinern, es sei denn als **versionierte,
> deterministische Kampagnenregel** (dann als `BoundedApproximation` im Zertifikat).

**Und die Verdrängung darf nicht nach LRU laufen.** Ein Eintrag, der einen Vollkampf ersetzt,
ist mehr wert als einer, der eine Ledger-Auswertung ersetzt — LRU kennt diesen Unterschied
nicht und wirft den teuren weg, wenn er länger nicht gebraucht wurde. Bewertet wird nach
**erwarteter Ersparnis je Byte**:

```
V(e) = P(erneuter Treffer) × Kosten der Neuberechnung / Bytes
```

Die Kostenzahl steht bereits zur Verfügung: `CostEstimate.expected_cycles` aus dem Router
(E11.4b). **Ohne sie wäre die Bewertung geraten** — mit ihr ist sie dieselbe Zahl, die auch
über den Auswertungsweg entscheidet.

#### Die Auflage, die alles daran entscheidet

> **Drosselung darf das Ergebnis nicht verändern.** Ein Lauf mit 16 Strängen und einer mit
> 2 Strängen und Drosselung müssen **bitgleich** dasselbe liefern. Das ist heute schon so
> (`base_seed` je Iteration, Schreiben an feste Stellen, keine Gleitkomma-Reduktion in
> Ankunftsreihenfolge) — aber es ist eine **Zusicherung, die geprüft werden muss**, kein
> Nebeneffekt. Ein Prüffall, der mitten im Lauf drosselt und das Ergebnis vergleicht, gehört in
> die Abnahme.

#### Regelung: von außen und von selbst

**Von außen** — eine Steuerdatei, die der Läufer alle N Chunks liest (kein Signal, kein Socket:
eine Datei ist von jeder Seite les- und schreibbar, überlebt einen Neustart und ist im
Nachhinein nachvollziehbar):

```json
{ "cpu_threads": 6, "ram_cache_mb": 512, "gpu": "off", "pause": false }
```

**Von selbst** — und zwar auf **zwei Ebenen**, weil eine allein nicht trägt.

> **KORRIGIERT am 2026-08-14, dritte Runde.** Hier stand ein Regler, der ausschließlich auf
> **freien RAM und Load Average** schaut. Beide Signale sind für diesen Zweck schlecht:
> `load average` zählt auch Prozesse, die auf Platte warten, und mittelt über eine Minute —
> er meldet Druck, wenn er längst vorbei ist. Und „freier RAM" sagt nichts über den
> tatsächlichen Speicherdruck, solange der Seitencache dazwischensteht.

> **Chunks nach Kandidatenzahl zu bemessen ist falsch — die Chunk-Länge IST die Reaktionszeit
> des Reglers.** Ein Cache-Treffer und ein voller HP-Phasen-Kampf unterscheiden sich um
> Größenordnungen: „64–512 Kandidaten" sind einmal 100 µs und einmal mehrere hundert
> Millisekunden. Das verschlechtert Drosselung, Pause, Fairness, Heartbeat und Lastverteilung
> zugleich.
>
> ```rust
> WorkQuantumPolicy { target_cpu_chunk_ms, max_cpu_chunk_ms,
>                     target_gpu_batch_ms, max_gpu_batch_ms }
> ```
>
> ```
> balanced     CPU-Ziel  5–15 ms · CPU-Max 25 ms · GPU-Ziel 10–25 ms · GPU-Max 50 ms
> background   CPU-Max   5–10 ms · GPU aus
> ```
>
> Die **Kandidatenzahl** leitet jeder Strang aus dem laufenden `CostEstimate` ab.

**Ebene A — Betriebssystemgrenzen.** Der Simulator läuft in einer eigenen cgroup v2 bzw. einem
systemd-Scope. Das ist die Hülle, die auch dann hält, wenn der interne Regler versagt:

```
cpu.weight        weiche Priorität bei Konkurrenz
cpu.max           harte Obergrenze
cpu.idle          echter Hintergrundmodus
memory.high       DROSSELT, ohne sofort zu töten      ← der eigentliche Hebel
memory.max        harte Grenze als Notbremse
memory.swap.max   verhindert, dass Auslagerung eskaliert
*.pressure        PSI je cgroup
```

`memory.high` ist genau dafür gebaut: Es bremst den Prozess aus, statt ihn abzuschießen. Auf
einer Maschine mit 2 GiB frei ist das der Unterschied zwischen „läuft langsamer" und „Lauf weg".

**Ebene B — der interne Regler**, gesteuert von **Druckwerten statt Mittelwerten**:

```
CPU-PSI avg10 · Memory-PSI avg10 · IO-PSI avg10
cgroup throttled_usec · Ein-/Auslagerungsrate
Taktdrosselung und Temperatur (Laptop)
```

**Und er regelt multiplikativ zurück, additiv hoch (AIMD):**

```
Druck über Ziel:            worker = max(min_worker, worker × 0,70)
Druck stabil unter Ziel:    worker += 1
```

Schnell weg, langsam zurück — ein symmetrischer Regler verbringt seine Zeit mit Schwingen.

> **Steuerung und Rechenarbeit gehören in GETRENNTE cgroups — sonst drosselt der Regler sich
> selbst.** Läuft der Koordinator in derselben Gruppe wie die Rechenstränge, hält `memory.high`
> oder die CPU-Quote auch **Statusausgabe, Heartbeat, Leases und Pause/Fortsetzen** an. Der
> Koordinator erklärt dann einen gesunden Rechner für tot, weil dessen Heartbeat gedrosselt
> wurde.
>
> ```
> rota-sim-control    kleine RESERVIERTE Quote, nicht drosselbar
>                     Koordinator · Heartbeat · Journal · Status · Pause/Resume
> rota-sim-compute    vollständig drosselbar
>                     CPU-Stränge · GPU-Übergabe · Caches · DP/B&B
> ```
>
> **Und die Platte fehlt bisher ganz.** E21.7 schreibt Scherben, Journal und Verdichtung — auf
> einer zu 83 % belegten NVMe und zwei vollen ZFS-Pools. Ohne Grenze verdrängt eine
> Hintergrundverdichtung Browser, Builds und VMs:
>
> ```
> io.weight · io.max · max_compaction_mb_s · max_journal_fsync_rate
> disk_pressure_threshold · network_egress_limit
> ```

**Fünf Betriebsprofile statt einer Zahl:**

| Profil        | Verhalten                                                        |
| ------------- | ---------------------------------------------------------------- |
| `max`         | volle lokale CPU- und GPU-Auslastung                             |
| `balanced`    | Laptop behält 2 Kerne und VRAM-Reserve — **Voreinstellung dort** |
| `background`  | `cpu.idle`, niedrigstes Gewicht, GPU aus                         |
| `remote-only` | lokal nur Koordinator und Anzeige                                |
| `custom`      | ausdrückliche Grenzen                                            |

**Der Sonderfall Laptop:** Er ist die Maschine, an der gearbeitet wird. Dort gilt zusätzlich
eine Obergrenze, die nicht der Regler setzt, sondern der Nutzer.

> **„Kerne − 2" ist mehrdeutig und muss es nicht sein.** Auf 8C/16T bedeutet es sechs Stränge
> (physisch), vierzehn (logisch) oder zwölf (zwei Kerne samt ihren SMT-Geschwistern
> reserviert). Verbindlich ist das Letzte:
>
> ```yaml
> reserved_physical_cores: 2
> reserved_cpu_set: auto # die zwei Kerne SAMT SMT-Geschwistern
> use_smt_siblings_of_compute_cores: true
> ```
>
> Und die Anzeige nennt es konkret statt als Bruchzahl:
>
> ```
> Compute:  CPUs 0–11
> Reserviert: physische Kerne 6–7 samt SMT-Geschwistern
> ```

Auf den beiden Nodes gibt es diese Grenze nicht.

#### Overprovisioning: gemessen, und billiger als gedacht

> **Auf Hinweis des Auftraggebers gemessen.** Die Frage war, ob ein Simulationslauf mit
> **niedriger Priorität** die freien Kapazitäten der Nodes nutzen kann, ohne den laufenden
> Betrieb zu stören.

```
gonet, Fremdlast 5,5-6,9 während der Messung
  normal,    12 Stränge      120 738 Kämpfe/s
  nice +19,  12 Stränge      127 415        ← nicht langsamer, im Rauschen sogar schneller
  nice +19,   6 Stränge       92 631        ← halbe Strangzahl kostet nur 27 %

proxmox, Leerlauf
  normal,    12 Stränge      213 778
  nice +19,  12 Stränge      210 743        ←  1,4 % Unterschied
  nice +19,   4 Stränge      122 110
  nice +19,   2 Stränge       61 766        ← sauber linear, 2 P-Cores
```

**Zwei Befunde, beide wichtig:**

**`nice +19` kostete in dieser Messung nichts.** Der Simulator ist CPU-gebunden ohne
Latenzanforderung — er nimmt die Lücken, die andere Prozesse lassen (und ein CI-Build wartet
viel auf Platte und Netz). Damit fällt der Vorbehalt gegen gonet aus Teil 2.1 weitgehend weg.

> **Aber „kostet nichts" ist zu absolut, und die Einschränkung gehört dazu.** `nice` steuert
> **nur die CPU-Zuteilung**. Es begrenzt **nicht**: gemeinsame Speicherbandbreite, Belegung des
> letzten Cache-Levels, thermisches Budget. Ein Simulationslauf kann einen parallelen Build
> also weiterhin verlangsamen, ohne ihm eine einzige CPU-Scheibe wegzunehmen — und die Messung
> gilt für **die getestete Fremdlast**, nicht für jede. Für harte Zusicherungen ist `cpu.max`
> das Werkzeug, nicht `nice`.

**Die Skalierung ist nicht linear, und das ist eine Chance.** Auf gonet kosten sechs statt
zwölf Stränge nur 27 % Durchsatz — die Maschine kann ihre 12 SMT-Stränge unter Fremdlast
ohnehin nicht ausfahren. Ein Lauf mit **halber** Strangzahl liefert dort 77 % der Leistung bei
halbem Fußabdruck. Für einen geteilten Rechner ist das das bessere Geschäft.

**Für LXC gilt dasselbe über `cpuunits` statt `nice`:** Ein Container mit hohem `cores`-Wert und
**niedrigem** `cpuunits` nimmt sich alles, was frei ist, und weicht zurück, sobald ein anderer
Container Anspruch anmeldet. **`cores` großzügig, `cpuunits` niedrig** — das ist die
Konfiguration, die Overprovisioning möglich macht. RAM dagegen wird bei LXC hart begrenzt und
nur belegt, was wirklich gebraucht wird; der Simulator braucht davon wenig.

**Freie Kapazität auf `10.0.0.69`**, gemessen: fünf LXC mit zusammen 19 zugeteilten Kernen, drei
laufende VMs mit 8 — bei 12 physischen Strängen. **Der Node ist bereits um Faktor 2,25
überbucht und läuft trotzdem bei Load 0,2.** Ein weiterer Container ändert daran nichts,
solange er der nachgiebigste ist.

### E15.2 — Fortschritt, Durchsatz und eine ehrliche ETA

**Was heute fehlt:** Ein Lauf gibt nichts aus, bis er fertig ist. Bei Sekunden ist das
gleichgültig; bei der Orakel-Tabelle des Endausbaus (368 016 Zellen) ist es unbrauchbar.

```
E11.5 LNS · Boss 12/68 · Runde 7        ██████░░░░░░░░░░  38 %

  Kandidaten     412 300 / 1 084 000          seit 00:04:12
  Durchsatz      487 200 /s   ⌀ 461 800 /s
     davon       Cache 61 %  ·  analytisch 12 %  ·  Suffix 19 %  ·  Vollkampf 8 %
     Kernel      1 240 000 /s roh              (nur die 8 % laufen wirklich durch)
  ETA            00:02:17     ± 0:24
  Systeme        laptop 12/16 · proxmox 12/12 · gonet AUS (Nutzer)
                 (12, nicht 14: reserved_physical_cores = 2 samt SMT-Geschwistern)
  Drosselung     RAM 2,1 GB frei → Cache auf 512 MB begrenzt
  Engpass        Speicherbandbreite  (IPC 1,81 gegen 2,65 Referenz)
  Bestes         14 208,0  seit 00:01:37 unverändert
```

**Die beiden Durchsatzzahlen stehen mit Absicht nebeneinander.** `full_fights/s` misst den
Kernel; `resolved_candidates/s` misst, was die Suche wirklich schafft. Nach E11.1 und E11.4b
laufen die beiden weit auseinander — und **nur die zweite ist die Zahl, auf die es ankommt**
(die Metriktabelle in E11.0 führt beide schon).

#### Die ETA muss zwei Fälle unterscheiden, sonst lügt sie

| Art der Arbeit                        | Menge vorher bekannt?           | was angezeigt wird                                                          |
| ------------------------------------- | ------------------------------- | --------------------------------------------------------------------------- |
| Sweep, Orakel-Tabelle, Parameterreihe | **ja**                          | echte ETA aus Restmenge / gleitendem Durchsatz                              |
| Greedy mit fester Länge               | ja (Runden × Kandidaten)        | echte ETA                                                                   |
| **LNS, MAP-Elites, Loop-until-dry**   | **nein** — läuft bis Konvergenz | **keine ETA.** Stattdessen: „Bestes seit mm:ss unverändert" plus Rundenzahl |

**Eine ETA für eine offene Suche wäre eine erfundene Zahl.** Der Ersatz ist die aussagekräftigere
Angabe: Wie lange hat sich nichts mehr verbessert? Danach entscheidet ein Mensch, ob er
abbricht — oder eine Abbruchregel.

Der gleitende Durchsatz wird über die **letzten 30 Sekunden** gemittelt, nicht über den ganzen
Lauf: Nach einer Drosselung oder einem Systemausfall wäre der Gesamtmittelwert wochenlang falsch.

> **NACHGESCHÄRFT am 2026-08-14: Kandidaten sind nicht gleich teuer.** Ein Cache-Treffer und ein
> voller Phasenboss-Kampf unterscheiden sich um Größenordnungen. `Restkandidaten geteilt durch
Kandidaten je Sekunde` wäre deshalb eine Zahl, die genau dann falsch wird, wenn der Router
> gut arbeitet — also immer.
>
> **Die Gewichte liegen bereits vor:** `CostEstimate.expected_cycles` aus E11.4b.
>
> **Aber eine Division reicht nicht.** `Σ Restzyklen / Σ Zyklen je Sekunde` setzt voraus, dass
> jede Aufgabe auf jedem Rechner mit derselben relativen Leistung läuft. Hier gilt das
> ausdrücklich nicht: **gonet kann kein AVX2**, CUDA läuft nur auf dem Laptop, P- und E-Kerne
> haben verschiedene Raten, manche Aufgaben sind zwischen LNS-Epochen **seriell**, und manche
> brauchen Speicher, den ein Rechner gerade nicht hat.
>
> **Richtig ist eine kleine Ablaufsimulation** über (Aufgabenklasse × Anzahl × Kosten ×
> zulässige Rechner) und (Rechner × Leistungsvektor × Warteschlange × Drosselung) — mit
> Listen-Zuteilung und **dem kritischen Pfad**:
>
> ```
> LNS-Epoche 7  →  Sammelpunkt  →  Epoche 8
> ```
>
> **Zehn Rechner machen eine sequenzielle Epochenkette nicht zehnmal schneller.** Wer das
> ignoriert, verspricht eine ETA, die der Aufbau nicht halten kann.

**Und die beiden ETA-Aussagen dieses Plans widersprechen sich nur scheinbar:**

```
freier LNS-Lauf ohne Vertrag        keine End-ETA (Plateau statt Zahl)
Kampagne mit eingefrorenem Budget   ETA für die verbleibenden Werteinheiten
```

Im zweiten Fall ist die Restarbeit **bekannt** — das ist der ganze Zweck von E21.0.

**Was zusätzlich in die Zeile gehört**, weil es die Fragen beantwortet, die während eines
langen Laufs wirklich auftreten:

```
Lauf-Kennung und Semantik-Hash · aktuelle Garantie (exakt/beschränkt/statistisch)
Phase und Unterphase · Boss/Instanz/Schwierigkeit/Nische
aktive Stränge je Rechner UND je Kernklasse (P/E)
GPU: Backend, Batchgröße, Einschaltdauer
Warteschlangentiefe, Pakete unterwegs, Wiederholungen, Rechnerausfälle
vermiedene Vollkämpfe · Trefferquote je Cache-Klasse
LNS-Operatorbudget und -ertrag · Nischenabdeckung
Zahl weiterhin toter Zauber/Talente/Affixe
Plateau: seit wann keine Verbesserung, in Zeit UND Auswertungen
```

**Und für Vergleichbarkeit über Bosse hinweg** genügen `fights/s` nicht — ein Endgame-Boss mit
vier Mechaniken ist kein einfacher Instanz-1-Boss:

```
cycles/event · cycles/player-decision · events/s · decisions/s
```

**Die Zähler je Strang liegen auf getrennten Cache-Zeilen.** Sonst kostet die Statusanzeige
genau die Bandbreite, die sie messen soll.

**Zwei Ausgabewege, ein Datenstrom:** eine Zeile für den Menschen (überschrieben, im Terminal),
und **JSON Lines** in eine Datei für alles andere. Kein zweiter Zähler, keine zweite Wahrheit.

```json
{
  "schema_version": 1,
  "run_id": "...",
  "event": "progress",
  "timestamp_utc": "2026-08-15T20:31:12.123456Z",
  "elapsed_ns": 123456789,
  "host_id": "proxmox",
  "host_sequence": 4812,
  "phase": "E11.5",
  "metrics": {}
}
```

**Beides, nicht eines.** `elapsed_ns` für lokale Dauern (immun gegen Zeitumstellung), **UTC für
die Zuordnung über Rechner hinweg** — monotone Uhren verschiedener Maschinen sind nicht
vergleichbar, und nach einem Neustart beginnt die monotone Uhr in einem anderen Bezugssystem.
Ohne UTC lässt sich ein verteilter Lauf nicht rekonstruieren. Die `host_sequence` zeigt Lücken
und verspätete Meldungen.

**Und die Unsicherheit der ETA (`± 0:24`) braucht eine Herleitung** aus Restkostenfehler,
Schwankung der Hostraten, kritischem Pfad und Wiederholungswahrscheinlichkeit. Eine Spanne ohne
Herleitung ist eine erfundene Zahl.

**Und die Ausgabe darf nichts kosten:** Der heiße Pfad erhöht ausschließlich einen
`AtomicU64` je Strang. Alles andere — Ableitungen, Prozentwerte, ETA, Formatierung — passiert im
Anzeigestrang, viermal je Sekunde.

### E15.3 — Engpasserkennung: nicht Zahlen, sondern eine Diagnose

**Die Anforderung ist nicht „mehr Messwerte", sondern die Antwort auf: wo lohnt der nächste
Umbau?**

Erhoben wird über `perf_event_open` (dieselben Zähler, die schon die Kernel-Messung geliefert
haben) plus eigene Zeitnahme:

```
IPC · Fehlsprünge/Kampf · L1d-, L2-, L3-Fehlgriffe · Speicher-Stalls
Chunk-Wartezeit je Strang   (Arbeit gegen Warten)
Cache-Trefferquote je Router-Zweig
Netzwerkwartezeit je Paket  ·  Anteil verworfener Pakete
Belegung der Ereignisschlange · Trefferquote von expire_buffs
```

**Daraus wird eine Regeltabelle, die einen Satz ausgibt** — Zahlen allein hat schon perf:

> **Feste Schwellen sind über vier Kernklassen hinweg nicht vergleichbar.** „IPC < 2,0" bedeutet
> auf Zen 3, Sandy Bridge, einem Alder-Lake-P-Kern und einem Gracemont-E-Kern etwas
> Verschiedenes. **Diagnosen laufen deshalb gegen die Baseline desselben Rechners, derselben
> Kernklasse und desselben Lastprofils** — als relative Abweichung:
>
> ```
> Fehlsprünge  +40 % gegenüber der Host-Baseline
> IPC          −25 %
> cycles/event +30 %
> Speicher-Stalls +50 %
> ```
>
> Dazu Architekturadapter für Zen 3 · Sandy Bridge · Alder Lake P · Gracemont E · CUDA.
> **Und `perf_event_open` ist eine Fähigkeit, keine Selbstverständlichkeit** — Container können
> sie sperren; dann fällt die Diagnose auf Zeitmessung und PSI zurück, statt auszufallen.

| Beobachtung (relativ zur Host-Baseline)                          | Diagnose                              | nächster Schritt                        |
| ---------------------------------------------------------------- | ------------------------------------- | --------------------------------------- |
| IPC deutlich unter Baseline **und** Fehlsprünge deutlich darüber | verzweigungsgebunden                  | E11.7b Kernel-Spezialisierung, PGO      |
| L2-Fehlgriffe hoch, L3 niedrig                                   | Zustand zu groß für L2                | `SearchFight` weiter verdichten (E11.4) |
| Stränge warten > 15 %                                            | Chunks zu groß oder zu wenige         | Chunk-Größe senken                      |
| Netzwerkwartezeit > 10 %                                         | Pakete zu klein                       | Paketgröße erhöhen (Teil 2.1-Tabelle)   |
| Cache-Trefferquote unter Break-even                              | dieser Zweig lohnt nicht              | Zweig abschalten (E11.4b)               |
| `expire_buffs`-Trefferquote > 20 %                               | die Annahme aus E11.2 gilt nicht mehr | neu messen, Endausbau erreicht          |
| Ereignisschlange > 80 % voll                                     | Kapazität wird knapp                  | **vor** dem Überlauf handeln (E11.0b)   |

**Die vorletzte Zeile ist der eigentliche Zweck dieses Issues.** Jede Optimierung von E11 beruht
auf einer heute gemessenen Verteilung. Der Engpasswächter ist das Bauteil, das **meldet, wann
eine dieser Annahmen abgelaufen ist** — und im Endausbau werden mehrere ablaufen (Teil 1).

**Fertig, wenn:** der Wächter an einer künstlich herbeigeführten Lage die richtige Diagnose
stellt — Chunks zu groß setzen, prüfen, ob er es sagt. **Ein Wächter, der nie angeschlagen hat,
ist nicht geprüft.**

### E15.4 — Was bei welcher Änderung neu gerechnet werden muss

**Die Kernfrage der Ausbaubarkeit.** Ohne diese Matrix ist jede Inhaltsänderung ein
Komplettlauf — und im Endausbau ist ein Komplettlauf Stunden bis Tage.

**Die Antwort steht schon in E11.4b**, sie muss nur ausformuliert werden: Der zweiteilige
Schlüssel aus `StructuralKey` und `NumericKey` **ist** die Invalidierungslogik. Was den
`StructuralKey` nicht berührt, überlebt.

**Und „ungültig" ist dabei kein einzelner Zustand, sondern drei** — die Unterscheidung spart
den größten Teil der Neuberechnung:

```
CombatOutcome                was der Kampf ROH ergab      — zielfunktions-unabhängig
CandidateScore               Objective(Outcome)
PortfolioParetoStatus        Lage in der Pareto-Front der Nische   (E19)
SearchOptimality             "das war das Beste"
CampaignCoverageCertificate  welche Tupel sind mit Zeugen belegt   (E21.1)
ProgressionResult            Fortschritts-Simulation               (E11.4e)
```

> **Drei Ebenen reichten, solange nur der Kampfpfad persistent war.** Mit Pareto-Archiv,
> Abdeckungszertifikat und Fortschritts-Orakel gibt es sechs Aussagen mit **verschiedener**
> Lebensdauer. Beispiel _neuer Boss_: Outcomes gültig · Scores gültig, sofern die Gewichtung
> bleibt · Pareto-Lage womöglich neu · Optimalität neu · **Abdeckungszertifikat ungültig**, weil
> neue Pflichttupel entstanden sind · Fortschritt abhängig von Freischaltung und Beute.
>
> **Und eine Zeile unten ist missverständlich:** „Abklingzeit geändert → Trace nur bei
> Regionswechsel" liest sich, als bliebe auch das **Ergebnis** unverändert. Richtig ist: Die
> **Trace-Struktur** bleibt innerhalb der Region gültig, das **Outcome wird parametrisch neu
> ausgewertet** — nur die Vollsimulation entfällt.

| Änderung                                   | Outcome                                              | Score                                               | Optimalität                                          |
| ------------------------------------------ | ---------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------- |
| **neuer Boss**                             | **gültig**                                           | neu, **falls** die Encounter-Gewichtung sich ändert | **neu** — ein Encounter kam dazu                     |
| **neue Instanz**                           | **gültig**                                           | dito                                                | **neu**                                              |
| **neuer Zauber**                           | **gültig** — alte Rotationen liefern weiter dasselbe | **gültig**                                          | **ungültig** — alte Elites werden Warm-Start (E12.4) |
| **Objective geändert**                     | **gültig**                                           | neu rechnen (Mikrosekunden)                         | ungültig, Suche **warm** fortsetzen                  |
| **`baseDamage` geändert**                  | nur betroffene Kandidaten (Rückwärtsindex)           | dito                                                | betroffene Nischen                                   |
| **Abklingzeit geändert**                   | Trace nur bei Regionswechsel (E11.3a)                | dito                                                | dito                                                 |
| **Gegenstand, bekannter Phänotyp**         | **gültig**                                           | **gültig**                                          | **gültig**                                           |
| **Gegenstand, neuer Phänotyp**             | nur der neue fehlt                                   | dito                                                | betroffene Nische                                    |
| **neues Affix**                            | Gear-Phänotypen neu                                  | dito                                                | betroffene Nischen                                   |
| **Satz- oder legendärer Effekt**           | nur passende Masken                                  | dito                                                | dito                                                 |
| **neues Talent**                           | Talent-Phänotypen und DP                             | dito                                                | betroffene Zweige                                    |
| **Schwierigkeitsstufe**                    | nur diese Stufe                                      | dito                                                | dito                                                 |
| **Fallrate / Beutetabelle / Aufwertregel** | **gültig**                                           | **gültig**                                          | **gültig** — nur die Fortschritts-Simulation kippt   |
| **Balance-Parameter global**               | nur betroffene Kandidaten                            | alle Scores                                         | alle Aussagen                                        |

**Die letzte Zeile ist der Grund, warum E11.3a und E12 zusammengehören:** Ein Balance-Durchgang
ändert Zahlen, keine Strukturen. Wer die Traces behält, rechnet nur die Zahlen neu.

> **NACHGESCHÄRFT am 2026-08-14: „ungültig" ist zu grob — es sind drei verschiedene Dinge.**
> Die Matrix oben wirft sie zusammen und verwirft dadurch mehr, als sie muss:
>
> ```
> CombatOutcome      was der Kampf ROH ergab   — objective-unabhängig
> Score              Objective(Outcome)        — eine Bewertung davon
> SearchOptimality   "das war das Beste"       — eine Aussage über die Suche
> ```
>
> | Änderung                                 | Outcome            | Score       | Optimalität                                        |
> | ---------------------------------------- | ------------------ | ----------- | -------------------------------------------------- |
> | **neuer Zauber**                         | **gültig**         | **gültig**  | **ungültig** — alte Elites werden Warm-Start       |
> | **Objective geändert**                   | **gültig**         | neu rechnen | ungültig, Suche **warm** fortsetzen                |
> | neuer Gegenstand, **bekannter Phänotyp** | **gültig**         | gültig      | gültig                                             |
> | neuer Gegenstand, **neuer Phänotyp**     | nur der neue fehlt | dito        | betroffene Nische                                  |
> | **neue Fallrate / Beutetabelle**         | **gültig**         | **gültig**  | **gültig** — nur die Fortschritts-Simulation kippt |
>
> **Die zweite Zeile ist die wertvollste** — mit einer Bedingung. Die Objective-Entscheidung
> steht noch aus (Teil 6); wenn der Cache den **rohen Ergebnisvektor** hält statt einer
> Punktzahl, kostet eine geänderte Zielfunktion oft keinen einzigen Kampf:
>
> ```rust
> OutcomeVector { terminal_state, damage, damage_taken, absorbed, healed, boss_healed,
>                 kill_time, death_time, casts, mechanic_results }
> ```
>
> **Aber „kostet keinen Kampf" gilt nur, wenn die neue Zielfunktion mit DIESEN Feldern
> auskommt.** Fragt sie nach Burst in den ersten 10 Sekunden, Streuung des Schadens,
> Verstärkungs-Laufzeit, Schaden je Zauber oder dem Fenster vor einem Boss-Cast, **reichen sie
> nicht** — dann ist die Aussage falsch.
>
> ```rust
> ObjectiveRequirements { required_outcome_fields, required_time_series,
>                         required_distribution_statistics }
> OutcomeSchemaVersion
> ```
>
> ```
> alle Größen vorhanden           → nur neu bewerten
> fehlen, aber Trace vorhanden    → aus dem Trace ableiten
> sonst                           → betroffene Kämpfe neu rechnen
> ```
>
> **Und je Nische wird nicht EINE Elite gespeichert, sondern eine zielfunktions-unabhängige
> Pareto-Front** der Ergebnisvektoren (Schaden · Überleben · Mechanikerfolg · Zeit · Robustheit
> · Ressourcenstabilität). Eine neue Zielfunktion wird dann **auf der vorhandenen Front** neu
> bewertet, statt die Suche von einer alten Elite aus neu zu beginnen. **Das ist der stärkste
> Hebel für wiederholte Balance-Durchgänge** — und er kostet nichts, solange er von Anfang an
> mitgeführt wird.
>
> **Rückwärts-Abhängigkeiten statt Pauschal-Invalidierung.** Eine Änderung an
> `spell_17.baseDamage` macht heute jeden `NumericKey` ungültig. Richtig ist ein Index
> `Parameter → Zauber → Traces → Ergebnisse → Archive`, der nur die Kandidaten trifft, die
> Zauber 17 **tatsächlich verwenden**.
>
> **Und keine globale `data_version`.** Komponentenweise Inhalts-Hashes — sonst macht ein neuer
> Boss alle Zauber- und Ausrüstungsergebnisse ungültig. Der Hash dient dem Nachschlagen; **beim
> Treffer wird der vollständige Schlüssel verglichen**, nie nur der Hash.

**Zwei Dinge, die die Matrix nicht löst und die deshalb hier stehen:**

**Ein neuer Zauber macht jede gefundene Rotation angreifbar** — nicht ungültig (sie liefert
weiter dasselbe Ergebnis), aber möglicherweise nicht mehr optimal. Das ist unvermeidbar und
folgt aus dem Design: Die Leiste wird frei aus allen Zaubern befüllt. **Der Ausweg ist nicht
Invalidierung, sondern Fortsetzung** — die alten Elites sind der Warm-Start für die neue Suche
(E12.4), nicht Müll.

**Einkompilierte Daten heißen: jede Inhaltsänderung braucht einen Neubau.** `sim/build.rs`
verwandelt `data/*.json` zur Bauzeit in statische Werte — bewusst so (GAME-DESIGN 18.3/18.4:
kein Parser, keine Hash-Container, keine Speicheranforderung im heißen Pfad). **Das bleibt so**,
und der Zielkonflikt ist kleiner, als er klingt:

```
Neubau des Simulators       Sekunden
Neuberechnung der Ergebnisse   Stunden bis Tage
```

„Nicht die ganze Simulation neu bauen" heißt deshalb: **nicht alles neu rechnen.** Genau das
leistet die Matrix. Ein Rebuild bleibt Pflicht — und weil die Datenversion in den
`StructuralKey` eingeht, kann ein Ergebnis aus einer anderen Datenversion nie stillschweigend
wiederverwendet werden.

### E15.5 — Verteilung: 1 + n, mit Ausfall als Normalfall

**Die Anforderung:** Der Lauf funktioniert auf einem Rechner. Zusatzrechner sind optional, ihr
Ausfall ist kein Fehler, und der Nutzer kann sie einzeln ausschließen.

```bash
rota-sim run …                              # nimmt, was erreichbar ist
rota-sim run … --hosts local                # ausschließlich lokal
rota-sim run … --hosts local,proxmox        # gonet ausgeschlossen
rota-sim run … --no-remote                  # dasselbe, kürzer
rota-sim run … --remote-nice 19 --remote-threads proxmox=8
```

**Der Ablauf, und jeder Schritt ist ein Rückfallpunkt:**

```
Rechnerliste aus der Konfiguration
        ↓
erreichbar?          → nicht erreichbar: überspringen, EINMAL vermerken, weiterlaufen
        ↓
Semantik-Hash gleich? → ungleich: Artefakt übertragen; scheitert das: überspringen
        ↓
Kalibrieren (E15.0)  → HostPerfVector, je Arbeitsart eine Rate
        ↓
Worker ZIEHT Arbeit, Koordinator vergibt eine Lease
        ↓
Heartbeat aus?       → Lease läuft ab, Job wird neu vergeben (auch lokal)
```

> **KORRIGIERT am 2026-08-14, dritte Runde — zwei Stellen.**
>
> **(a) Der Koordinator soll nicht zuteilen, sondern die Worker sollen ziehen.** Zuteilung nach
> Kalibrierwert setzt voraus, dass der Wert während des ganzen Laufs stimmt — er stimmt aber
> nicht, sobald ein Rechner gedrosselt wird oder Fremdlast bekommt. **Ein Worker, der Arbeit
> anfordert, wenn er frei ist, braucht kein Modell seiner selbst.**
>
> **(b) „Binary-Hash gleich" ist das falsche Kriterium**, sobald je Zielarchitektur optimiert
> wird — `sandybridge`, `znver3` und `alderlake` erzeugen **notwendigerweise** verschiedene
> Binärdateien. Getrennt werden muss:
>
> ```rust
> engine_semantics_hash   // MUSS überall gleich sein — Regeln, Daten, numerischer Vertrag
> artifact_hash           // DARF je Host verschieden sein — Optimierungsziel
> target_profile          // sandybridge | znver3 | alderlake
> ```
>
> Vor dem ersten Paket prüft der Worker: Semantik gleich? Daten-Hashes gleich? numerischer
> Vertrag gleich? passendes Artefakt vorhanden? **Selbsttest bestanden?** Erst dann bekommt er
> Arbeit.

**Der häufigere Fall ist nicht der Ausfall, sondern der Nachzügler.** Auf geteilten Rechnern
ist ein langsames, aber lebendes Paket wahrscheinlicher als ein toter Rechner:

```
weiche Frist    Paket voraussichtlich verspätet
                → spekulative Zweitausführung auf einem freien Rechner
harte Frist     ursprüngliche Lease abgelaufen
Festschreiben   das erste gültige Ergebnis gewinnt, das zweite wird verworfen
```

**Und der Koordinator braucht seinen Lease- und Commit-Zustand persistent** (E17) — sonst
erscheint nach seinem eigenen Neustart jede laufende Fernarbeit entweder doppelt oder verloren.

**„Mindestens einmal" statt „genau einmal".** Doppelte Ausführung ist zulässig und einfacher
als jede Übereinkunft: Kommt ein Ergebnis für eine Job-Kennung, die schon festgeschrieben ist,
wird es verworfen. Deterministische Pakete machen das gefahrlos.

**Determinismus bei adaptiver Suche — die Falle, die man erst im Betrieb sieht.** Adaptive
LNS-Operatorstatistik und MAP-Elites-Archiv dürfen **nie** in der Reihenfolge aktualisiert
werden, in der Ergebnisse aus dem Netz eintreffen. Sonst ändert ein ausgefallener Zusatzrechner
nicht die Laufzeit, sondern **das Ergebnis**:

```
Epoche erzeugt Kandidaten 0..N
        ↓
alle Ergebnisse einsammeln
        ↓
nach Kandidaten-Kennung SORTIEREN
        ↓
Archiv und Operatorstatistik deterministisch fortschreiben
        ↓
nächste Epoche
```

**Warum das ohne verteilten Zustand funktioniert:** Ein Paket ist vollständig durch seinen
**`JobKey`** beschrieben (oben) und damit **deterministisch** — Vertrag, Semantik-Hash,
Inhalts-Hashes, Auswerterart, Durchgangsbereich, Ergebnisschema und RNG-Vertrag inbegriffen.
Es zweimal zu rechnen ist verschwendete Zeit, aber nie ein falsches Ergebnis. Damit braucht es
keine Übereinkunft, keine Sperren und keinen Zustandsabgleich — nur eine Nachvergabe.

**Und die Bitgleichheit über Architekturen ist gemessen** (Teil 2.1): Sandy Bridge, Zen 3 und
Alder Lake liefern denselben **Bit-Hash** über `to_bits()` aller Ergebnisfelder
(`0x42210c39918563b9`). Eine formatierte Dezimalzahl wäre kein Nachweis — zwei `f64`, die sich
im letzten Bit unterscheiden, drucken dieselbe Zeile. Ohne diesen Nachweis wäre die ganze
Verteilung unzulässig; mit ihm ist sie eine Betriebsfrage.

> **Für dauerhafte Korrektheitsentscheidungen reicht FNV-64 nicht.** Er ist gut genug, um eine
> Abweichung zwischen drei Rechnern zu zeigen; für einen persistenten Ergebnisspeicher (E21.7),
> in dem ein Treffer eine Neuberechnung ersetzt, braucht es einen starken Inhalts-Hash
> (BLAKE3-128 oder stärker) **und** den Vergleich des vollständigen kanonischen Schlüssels beim
> Treffer.

**Die Paketgröße folgt aus einer Kostenrechnung, nicht aus einer Prozentgrenze.**

> **KORRIGIERT am 2026-08-14.** Hier stand „mindestens ~50 000 Kämpfe, damit der Aufschlag
> unter 1 % bleibt". **Die 1 % waren willkürlich.** Entscheidend ist nicht der Anteil, sondern
> ob es sich lohnt:
>
> ```
> T_remote = RTT + Serialisieren + Warteschlange + Arbeit / Rate_remote
> ```
>
> **Remote lohnt, wenn `T_remote` kleiner ist als die beste lokale Alternative** — auch bei 5 %
> Netzanteil. Praktisch skaliert der Verteiler auf eine Ziel-Rechenzeit je Paket:
> **100–500 ms** bei interaktiven Läufen, **0,5–5 s** bei Stapelarbeit.

**LNS ist nicht kategorisch lokal.**

> **KORRIGIERT.** Hier stand, bei offener Suche werde „**nicht** verteilt". Das stimmt für die
> **Runden**, nicht für ihren Inhalt: Zwischen zwei Runden ist LNS sequenziell, aber **die
> Bewertungen innerhalb einer Runde sind unabhängig**. Eine Runde mit 10 000 Kandidaten darf
> verteilt werden, wenn das Kostenmodell einen Gewinn ausweist — mit einem deterministischen
> Sammelpunkt am Rundenende (siehe Epochenregel oben). Nur eine Runde mit wenigen hundert
> Kandidaten bleibt lokal, weil sie eine Roundtrip-Zeit wert ist.

### E15.6 — Hardware-Ausnutzung: was trägt, was nicht, und warum kein Assembler

**Der Auftrag war ausdrücklich, das genau zu prüfen — auch Assembler und eine polyglotte
Lösung.** Die Antwort ist in beiden Fällen ein begründetes Nein, und die Begründung steht in
einer Zahl, die schon gemessen ist:

```
IPC 2,65   ·   155 Fehlsprünge je Kampf   ·   59 340 Zyklen je Kampf
```

**Der Kernel ist verzweigungs*lastig* und datenabhängig** — er verbringt seine Zeit mit
Entscheidungen (welche Regel greift, welches Ereignis kommt als Nächstes, ist der Zauber
bereit), nicht mit Arithmetik.

> **„Verzweigungs*gebunden*" wäre zu stark.** Die eigene Rechnung unten ergibt für die
> Fehlsprünge rund **3,9 %** der Zyklen — das belegt einen branch-**heavy** Kernel, nicht einen
> branch-**bound**. Für die Diagnose braucht es die Top-down-Zähler: `bad speculation` ·
> `frontend bound` · `backend bound` (Memory/Core) · `retiring`. Erst sie sagen, ob
> Spezialisierung und PGO (Bad Speculation), BOLT (Frontend), Datenlayout (Memory) oder SIMD
> (Core) der richtige Hebel ist. **Die Entscheidung gegen einen allgemeinen Assembler-Kernel
> bleibt richtig — nur ihre Begründung wird genauer.** Das bestimmt, welche Hardware-Eigenschaft überhaupt etwas ändern
> kann.

#### Befehlssätze, einzeln durchgegangen

| Befehlssatz                  | vorhanden                  | trägt für ROTA?                                                                                                                                                                                                                                                                    |
| ---------------------------- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **AVX2**                     | Laptop, proxmox            | **nur für homogene Batches** (E11.7). `f64` heißt 4 Lanes, nicht 8 — und der Kernel selbst ist nicht vektorisierbar, weil jede Rotation anders verzweigt                                                                                                                           |
| **FMA**                      | Laptop, proxmox            | **vorhanden, im heutigen Ergebnisvertrag NICHT verwendet** — es ändert die Rundung, und die Parität hängt an `Math.round`-Gleichheit. Erst nach E14 neu zu bewerten                                                                                                                |
| **POPCNT**                   | Laptop, proxmox, **gonet** | **ja, eigene Erweiterung** — nicht Teil von BMI1. Populationszählung in `RuleBits`/`SpellBits`. **gonet hat sie** (SSE4.2-Zeitalter)                                                                                                                                               |
| **BMI1** (`TZCNT`, `BLSR`)   | Laptop, proxmox            | **ja** — Bit-Iteration in `RuleBits<WORDS>`/`SpellBits<WORDS>` (E11.4). Compiler nutzt sie mit dem **expliziten Zielprofil**, nicht mit `native`. **gonet: nein**                                                                                                                  |
| **BMI2**                     | Laptop, proxmox            | **nein**, solange kein `PDEP`/`PEXT`-Muster nachgewiesen ist                                                                                                                                                                                                                       |
| **AVX-512**                  | **nirgends**               | entfällt. Zen 3 hat es nicht, Consumer-Alder-Lake hat es deaktiviert                                                                                                                                                                                                               |
| **SHA-NI**                   | Laptop, proxmox            | **nein.** Die Prüfsumme läuft nur im `replay`-Pfad (`fight.rs:1471`), nicht im heißen Pfad — und sie ist ein FNV-Mix, kein SHA                                                                                                                                                     |
| **AES, VAES, PCLMULQDQ**     | Laptop, proxmox            | **nein.** Keine Verschlüsselung im Simulator                                                                                                                                                                                                                                       |
| **AVX-VNNI**                 | **nur proxmox**            | **nein.** Integer-Skalarprodukt für neuronale Netze; unser Kernel rechnet `f64`                                                                                                                                                                                                    |
| **GFNI, SERIALIZE, WAITPKG** | nur proxmox                | **nein.** Galois-Feld-Arithmetik, Speicherordnung, Warteschleifen — nichts davon im heißen Pfad                                                                                                                                                                                    |
| **F16C**                     | Laptop, proxmox            | **nein.** Halbe Genauigkeit ist mit der Paritätsauflage unvereinbar                                                                                                                                                                                                                |
| **RDRAND/RDSEED**            | Laptop, proxmox            | **nein, und ausdrücklich nicht.** Der Zufall muss reproduzierbar sein — ein Hardware-Zufallszahlengeber ist genau das Gegenteil                                                                                                                                                    |
| **CLWB, CLFLUSHOPT**         | Laptop, proxmox            | **nein — sie laden nichts vor.** Beide schreiben zurück bzw. verwerfen Cache-Zeilen. Vorladen wäre `PREFETCHT0/T1/NTA`; bei kleinen, vorhersehbar durchlaufenen Checkpoints genügt der Hardware-Vorlader vermutlich. **Manuelles Vorladen erst nach einem gemessenen Cache-Stall** |

**Zusammengefasst: Von den exotischen Befehlssätzen trägt keiner.** Was trägt, sind AVX2/FMA für
Batches und — weniger sichtbar — **BMI1 plus `POPCNT`** für die Bitmengen: `TZCNT` und
`BLSR` gehören zu BMI1, `POPCNT` ist eine eigene Erweiterung. **BMI2 zählt erst bei einem
nachgewiesenen `PDEP`/`PEXT`-Muster**, und ein solches gibt es hier nicht. Alles davon fehlt
auf gonet.

#### Assembler: nein, und hier ist die Rechnung

Handgeschriebener Assembler lohnt, wenn eine enge Rechenschleife bekannt und stabil ist. Unser
heißer Pfad ist das Gegenteil: 13 Ereignisränge, verzweigte Regelauswertung, datenabhängige
Sprünge. **155 Fehlsprünge je Kampf kosten bei ~15 Zyklen Strafe rund 2 300 Zyklen — 3,9 % der
59 340.** Selbst eine perfekte Sprungvorhersage brächte weniger als vier Prozent, und Assembler
verbessert die Vorhersage nicht.

**Was an derselben Stelle tatsächlich hilft, steht schon im Plan:**

| Maßnahme                                                     | wirkt auf                                    | wo                                  |
| ------------------------------------------------------------ | -------------------------------------------- | ----------------------------------- |
| PGO mit echter Boss-/Build-Mischung                          | Sprungvorhersage und Codeanordnung           | E11.6                               |
| Kernel-Spezialisierung je Merkmalsmaske                      | **entfernt Zweige, statt sie vorherzusagen** | E11.7b                              |
| verzweigungsfreie Auswahl (`cmov`) an wenigen heißen Stellen | unvorhersagbare Sprünge                      | **neu, als Teilschritt von E11.7b** |
| Datenlayout: aktive Zauber statt 70er-Arrays                 | Cache-Fehlgriffe                             | E11.4                               |

**Drei Prüfpunkte gehören dazu, bevor überhaupt an Assembler zu denken ist** — sie kosten
Prozentpunkte, keine Größenordnungen, aber sie sind messbar:

```
AVX/SSE-Grenzen (nur gonet)   Dispatch EINMAL je Batch · keine Wechsel im inneren Loop
                              vzeroupper an Rückkehr- und FFI-Grenzen im Disassemblat prüfen
Ausrichtung                   #[repr(align(64))] für Worker-Zähler (False Sharing)
                              #[repr(align(32))] für AoSoA-Batches — DANACH messen
Bounds-Checks                 cargo asm zeigt, ob LLVM sie in den heißen Arrays entfernt.
                              Erst ein NACHGEWIESEN verbleibender Check rechtfertigt
                              get_unchecked — mit Debug-Assertion und Eigenschaftstest
```

**Der dritte Punkt ist der einzige Ort, an dem etwas Assembler-Nahes lohnt** — und auch dort
zunächst nicht als `asm!`-Block, sondern als verzweigungsfreie Rust-Formulierung, die der
Compiler zu `cmov` übersetzt. Ein `asm!`-Block ist für den Optimierer undurchsichtig und
verhindert genau die Umformungen, die den Gewinn bringen.

> **PRÄZISIERT am 2026-08-14, dritte Runde — meine Rechnung war eine Schätzung, keine
> Obergrenze.** `155 × 15 Zyklen = 3,9 %` unterstellt, dass sich Fehlsprung-Strafen nicht
> überlappen und dass Assembler nur die Sprungvorhersage beträfe. Beides trifft nicht
> zwingend zu: Strafen können überlappen, und Assembler kann auch Befehlszahl und
> Datenanordnung ändern. **Die Schlussfolgerung bleibt** — eine verzweigte, sich ständig
> ändernde Kampfmaschine in Assembler zu schreiben, wäre falsch —, aber sie ist ein
> begründetes Urteil, kein Beweis.
>
> **Deshalb eine Stufenregel statt eines Verbots.** Assembler kommt zuletzt, nicht nie:
>
> ```
> 1  Rust mit passendem Datenlayout
> 2  Intrinsics + #[target_feature]
> 3  PGO
> 4  Disassemblat ansehen (cargo asm)
> 5  Durchsatzanalyse (llvm-mca)
> 6  erst dann ein winziger, stabiler asm!-Mikrokernel — und nur, wenn 4 und 5
>    einen konkreten Compilerfehler zeigen
> ```
>
> **`cmov` gehört ebenso gemessen**: Sind beide Zweige teuer, ist verzweigungsfrei
> **langsamer** als ein gut vorhergesagter Sprung. Auch das ist eine Messung, keine Regel.

#### Hotpath einsprachig — die vollständige Regel steht in „Polyglott — präzisiert" (Teil 4.6)

Der Plan macht Rust/TypeScript-Parität zum **Pflichttest bei jeder Änderung**. Eine dritte
Sprache im Rechenpfad verdreifacht die Fläche, auf der eine Rundungsabweichung entstehen kann —
und die Parität ist das, was den Simulator überhaupt aussagekräftig macht.

**Die eine Ausnahme ist CUDA-C für E13**, und die ist unvermeidbar. Sie ist vertretbar, weil das
GPU-Backend hinter der Grenze `EvaluatorBackend` sitzt und seine Ergebnisse gegen den skalaren
Pfad geprüft werden (Teil 6, Auflage 2) — es ist ein zweiter Rechenweg mit eigener Abnahme,
kein zweiter Wahrheitsanspruch.

#### GPU: eine Maschine, 4 GB, und ein ungünstiger Kernel

```
Laptop    RTX 3050 Ti Mobile · 4 096 MiB · Compute Capability 8,6 · CUDA
gonet     Radeon RX 470-580 (Polaris, GCN 4)         kein CUDA
proxmox   Intel Iris Xe (iGPU, kein eigener Speicher) kein CUDA
```

**Der Vollkampf ist auf der GPU voreingestellt AUS** — 155 Fehlsprünge je Kampf bedeuten
divergente Warps, und das ist der Fall, in dem eine GPU am schlechtesten ist.

> **Das ist eine heutige Erwartung, kein Architekturverbot** (E13 sagt dasselbe, nur andersherum
> — hier war es kategorisch formuliert). Ein Versuch ist zulässig, wenn **alle vier** Bedingungen
> erfüllt sind: E14 abgeschlossen · alle einfacheren GPU-Pfade gemessen · ein repräsentativer
> **homogener** Vollkampf-Batch existiert · End-to-End-Gewinn **inklusive** Packen, Transfers und
> CPU-Taktverlust zu erwarten. Bringt der Versuch nichts, bleibt er dauerhaft aus. **Was auf die GPU
> gehört**, sind die homogenen Teile: Ledger-Auswertungen, Kandidat × Boss, Krit-Verteilungen.
> Genau so steht es in E13, und die Break-even-Schwelle (50 000–100 000 Kandidaten je Batch) ist
> der Prüfstein.

**Die iGPUs sind voreingestellt aus — aber nicht ausgeschlossen.**

> **KORRIGIERT am 2026-08-14, vierte Runde.** Hier stand „Die iGPUs bleiben **außen vor**" —
> das widerspricht der Regel, die für alles andere in diesem Plan gilt: **durch Kalibrierung
> entscheiden, nicht durch Festannahme.**

Die Bedenken bleiben und sind der Grund für die Voreinstellung: Vega und Iris Xe teilen sich den
Systemspeicher mit der CPU und nehmen ihr Bandbreite, die sie laut Engpasstabelle selbst
braucht; auf dem Laptop haben CPU, iGPU und dGPU zudem **ein gemeinsames thermisches Budget**.

**Aber messbar bleiben sie.** Die Regel:

```
Vega (Laptop)     aus  · einschaltbar nur, wenn CPU+iGPU zusammen mehr
                         liefern UND der CPU-Takt nicht fällt
Iris Xe (proxmox) aus  · dieselbe Bedingung
RX 470-580 (gonet) aus · über Vulkan-Compute prüfen, aber erst NACH E14 —
                         mit Festkomma ist ein exakter Pfad wahrscheinlicher
```

**Automatisch eingeschaltet wird keine von ihnen.** Wer sie prüfen will, misst — und die
Messung entscheidet, nicht dieser Absatz.

#### SSD und Arbeitsspeicher

**Die Platte ist kein Hebel** — 97 page-faults auf 200 000 Kämpfe. Der Simulator liest beim
Start und rechnet dann im Speicher.

**Der Arbeitsspeicher ist einer, aber anders als erwartet:** Nicht die Menge (der Kernel braucht
wenig), sondern die **Bandbreite**. DDR3-1333 gegen DDR4-3200 ist Faktor 2,4 und erklärt einen
Teil des gemessenen Rückstands von gonet. Auf dem Laptop ist die **Menge** der Engpass — 14,9
GiB verlötet, im Betrieb ~2 GiB frei. Das ist der Grund, warum der Cache aus E11.4b eine
Obergrenze braucht (E15.1) und nicht wachsen darf, bis das System swappt.

### Polyglott — präzisiert

Teil Anhang B.2.6 sagt „polyglott: nein, außer CUDA-C". **Für den heißen Rechenpfad bleibt das
richtig.** Für die Suite als Ganzes ist es zu streng — der kalte Pfad darf andere Werkzeuge
benutzen, weil er die Parität nicht berührt:

| Bestandteil                                 | Sprache                                      |
| ------------------------------------------- | -------------------------------------------- |
| **Kampfkernel, Scheduler, Router**          | **Rust — einsprachig, keine Ausnahme**       |
| SIMD                                        | Rust `std::arch` (kein `std::simd`, nightly) |
| NVIDIA-GPU                                  | CUDA-C++ hinter schmaler C-Schnittstelle     |
| Abdeckungsmatrizen (E21.1)                  | Constraint-Löser, **kalter Pfad**            |
| formale Nachweise (E11.4f `FormallyProven`) | SMT-Löser, **kalter Pfad**                   |
| gestufte Spezialisierung (E11.7c)           | Cranelift                                    |
| Auswertung, Diagramme                       | Python, **nur offline**                      |
| Browser-Referenz                            | TypeScript (Parität, gesetzt)                |

> **Der Grundsatz: polyglott beim Planen, Beweisen und auf der GPU — einsprachig im heißen
> Pfad.** Und: C++ oder Zig machen den CPU-Kernel **nicht** schneller. Bei gleichem
> LLVM-Unterbau entscheiden Datenlayout, Algorithmus und Verzweigungsverhalten, nicht der Name
> der Sprache.

### 4.7 E16 bis E20 — Bauteile und Bedienung

### E16 — Die Wirkungs-IR: der Abhängigkeitsgraph darf nicht von Hand gepflegt werden

**Das Problem, und es ist die gefährlichste Fehlerquelle im ganzen Plan.** An mindestens sechs
Stellen wird entschieden, was ein Build tut: `crit_affects_control_flow()` (E11.4d),
`UpperBoundCapability` (E11.5b), `kernel_signature` (E11.7b), die Ledger-Gültigkeit (E11.3a),
die Invalidierung (E15.4) und die Ereigniskanäle (E11.2).

**Jede dieser Stellen ist heute eine Liste, die jemand pflegen müsste.** Und ich habe in diesem
Dokument bereits **zweimal** vorgeführt, was dann passiert: Ich habe „sieben Zusatz-Systeme"
geschrieben, wo es elf sind, und „der Cooldown-Rücksetzer kommt noch", wo neun Zauber ihn
bereits tragen. **Beide Male war die Liste unvollständig, beide Male hat es niemand gemerkt.**

Bei einem neuen legendären Effekt wäre die Folge nicht ein falscher Satz in einem Dokument,
sondern: `crit_affects_control_flow()` gibt weiter `false` zurück · ein Ledger wird
fälschlich wiederverwendet · das Kernelprofil passt nicht · die Invalidierung übersieht
Ergebnisse. **Alles still, alles plausibel aussehend.**

**Deshalb wird der Abhängigkeitsgraph erzeugt, nicht gepflegt.** Die Zauber- und Bossdaten
werden zur Bauzeit (`sim/build.rs` tut das ohnehin schon) in eine typisierte Wirkungsdarstellung
übersetzt:

```rust
enum EffectOp {
    DealDamage, ApplyDot, ApplyBuff, ModifyResource,
    ReduceCooldown, ResetCooldown, FreeNextGcd,
    Heal, Shield, ConvertDamage,
    ChangeBossState, ChangePhase,
    CustomSignature(SignatureId),
}
```

Daraus leitet der Übersetzer **automatisch** ab: `DependencyMask` · `KernelSignature` ·
`UpperBoundCapability` · Ereigniskanäle · Kontrollfluss- und Zahlen-Abhängigkeiten ·
Invalidierungskanten.

> **Ein reiner Opcode-Enum reicht dafür nicht.** Aus `DealDamage` allein lässt sich weder
> Monotonie noch Kontrollflusswirkung noch SIMD-Eignung ableiten. Der Knoten muss die Semantik
> **tragen**, nicht nur benennen:
>
> ```rust
> EffectNode {
>     op, trigger, target,
>     reads:  StateMask,          // was liest er
>     writes: StateMask,          // was schreibt er
>     timing: TimingSemantics,    // Momentaufnahme oder live
>     stacking: StackingSemantics,
>     randomness: RandomnessSemantics,   // speist StochasticSourceMask (E11.1)
>     monotonicity: LocalMonotonicityHint, // NUR ein Hinweis — siehe Kasten
>     numeric_bounds: Bounds,            // trägt E14
>     control_flow_effect: ControlFlowEffect,
> }
> ```
>
> Erst daraus lassen sich Abhängigkeitsmaske, Kernelsignatur, `UpperBoundCapability`,
> Ereigniskanäle und Invalidierungskanten **ableiten** statt annehmen.
>
> **Monotonie ist dabei keine Eigenschaft eines Effekts allein.** `baseDamage` ist monoton
> bezüglich _erwartetem Schaden an einem gewöhnlichen Boss bei stabilem Trace_ — und **nicht**
> bezüglich _Sieg an einem `runaway`-Boss_, _Kill-Zeit über einen Phasenwechsel_ oder eines
> Scores mit HP-Prädikaten. Ein einzelnes Enum am Knoten wäre zu vertrauensselig:
>
> ```rust
> MonotonicityCertificate { parameter, requested_outcome, encounter_class,
>                           trace_region, preconditions, direction }
> ```
>
> **E16 liefert den Hinweis, das Zertifikat entsteht erst aus** Wirkungs-IR **+** Bosssemantik
> **+** Zielfunktion **+** Trace-Region. Nur ein vollständiges Zertifikat berechtigt zur
> Binärsuche in E12.2 und zum Dominanz-Pruning in E11.5b.

**Der eigentliche Gewinn ist die Übersetzungszeit-Prüfung.** Jede Analysestelle behandelt
`EffectOp` mit einem vollständigen `match` **ohne** Auffangzweig. Ein neuer Opcode erzeugt dann
**an jeder Stelle einen Übersetzungsfehler**, bis jemand entschieden hat, was er dort bedeutet.

> **Und die Zahl der Analysestellen darf nicht im Text stehen.** „Sechs" war beim Schreiben
> richtig; inzwischen hängen zusätzlich `StochasticSourceMask`, `ExchangeabilityCertificate`,
> Interaktionsgraph, ActionPolicy-Analyse, Monotonie-Zertifikate, Policy-Äquivalenz,
> Outcome-Schema und RNG-Relevanz daran. **Eine feste Zahl altert bei jeder Ergänzung** — genau
> wie die Issue-Zählung.
>
> ```rust
> trait EffectAnalysisPass { const ID: AnalysisPassId;
>                            fn analyze(&self, node: &EffectNode) -> AnalysisResult; }
> ```
>
> Eine **erzeugte Registry** listet die Pflichtanalysen, und der Test lautet: _ein neuer Opcode
> oder eine `CustomSignature` ohne vollständige Semantik lässt **jede registrierte** Analyse
> fehlschlagen_ — nicht „alle sechs".

> **`CustomSignature` ist genau das Schlupfloch, das diese Zusicherung aufheben würde.** Wenn
> jede ungewöhnliche Mechanik als `CustomSignature(id)` durchläuft, bricht der Bau **nie** mehr
> — und die 68 Boss-Signaturen sind per Design ungewöhnliche Mechaniken.
>
> **Deshalb muss auch eine Signatur ihre Semantik deklarieren:** gelesene und geschriebene
> Zustände, ob Zufall beteiligt ist, ob Schaden/HP/Sieg den Kontrollfluss berühren, Bereiche,
> Monotonie. **Fehlt eine dieser Angaben, gilt der konservative Fall:**
>
> ```
> alle Abhängigkeiten annehmen · kein Ledger · keine analytische Marginalisierung
> kein Dominanz-Pruning · keine enge B&B-Schranke · GenericFallback-Kernel
> ```
>
> **Unbekannt muss langsam sein, nie falsch.** Das ist die Regel, an der die ganze
> Erweiterbarkeit hängt.

**Die 68 Boss-Signaturregeln laufen über `CustomSignature` und einen datengetriebenen
Auffangpfad** — sie dürfen nie 68 einkompilierte Vollkernel erzeugen, sonst wächst die
Codegröße mit dem Spielinhalt (steht schon in E11.7b, hier ist der Mechanismus dazu).

**Fertig, wenn:** ein neu hinzugefügter Opcode ohne weitere Änderung **den Bau bricht**, und
zwar an allen sechs Analysestellen. Ein Prüffall, der genau das nachweist, gehört dazu — sonst
ist die Zusicherung nur eine Absicht.

### E17 — Lauf-Zustand: Pause, Absturz und Fortsetzung

**Was heute fehlt:** Die Steuerdatei aus E15.1 überlebt einen Neustart — aber sie enthält nur
die **gewünschten Grenzen**, nicht den Lauf. Ein Absturz nach sechs Stunden kostet sechs
Stunden.

Bei Läufen über Stunden bis Tage — und die Orakel-Tabelle des Endausbaus ist so einer — ist das
kein Randfall, sondern der Normalfall.

```rust
RunManifest {                       // v2 — was ein Lauf braucht, um BITGLEICH fortzufahren
    run_id,
    // Verträge und Semantik
    campaign_definition_hash, run_contract_hash, logical_cost_model_hash,
    engine_semantics_hash, content_component_hashes,
    objective_hash, outcome_schema_version, numeric_contract_hash,
    counter_rng_algorithm, rng_contract_version, probability_mapping_version,
    search_algorithm_version, solver_versions,
    coverage_generator_version, jit_version,

    // Ablauf
    phase, epoch,
    pending_jobs, inflight_leases, completed_bitmap,
    deadline_policy, remaining_runtime,

    // Sucharchive
    map_elites_archive, pareto_archives, lns_state, bandit_state,
    frozen_incumbent_snapshot,            // E18: Schranken gelten gegen DIESEN
    best_candidates, progress_counters,

    // Balance-Zustand (E12)
    balance_parameter_state, current_trust_region,
    parametric_incumbent_envelopes, challenger_state,

    // Abdeckung und Stichprobe
    coverage_matrix_hash, fulfilled_obligations, coverage_witnesses,
    trial_block_state, trial_accumulators,

    persistent_store_commit,
}
```

> **Jedes Feld der unteren drei Blöcke fehlte in der Vorfassung** — und jedes einzelne hätte
> gereicht, um „bitgleich wie ein durchgehender Lauf" zu brechen: andere Budgetsemantik, andere
> Abdeckungsmatrix, verlorene E12-Hüllen, doppelt gezählte Durchgänge, anderer Amtsinhaber.

**Und die Schreibfolge braucht vier Schritte, nicht drei:**

```
temporäre Datei schreiben  →  fsync(Datei)  →  umbenennen  →  fsync(VERZEICHNIS)
```

**Ohne das `fsync` auf das Verzeichnis** ist die atomare Umbenennung nach einem Stromausfall
auf mehreren Linux-Dateisystemen **nicht** garantiert — die Datei existiert, der Verzeichnis­
eintrag nicht.

**Geschrieben wird fortlaufend als Journal plus periodischer Momentaufnahme** — temporäre
Datei, `fsync`, atomares Umbenennen. Ein halb geschriebener Zustand darf nicht entstehen.

**Anhalten in sieben Schritten**, und keiner davon verwirft Arbeit: keine neuen Chunks vergeben ·
laufende kleine Chunks zu Ende rechnen · lange DP- und B&B-Aufgaben an einem sicheren Punkt
sichern · GPU-Batch beenden · Leases der Zusatzrechner zurückholen oder verlängern ·
Momentaufnahme schreiben · Zustand `PAUSED` melden.

**Fortsetzen:** Hashes prüfen · verträgliche Caches öffnen · abgelaufene Leases zurücksetzen ·
Archiv und Operatorstatistik laden · **ab der nächsten deterministischen Epoche** weiterrechnen
(die Epochenregel aus E15.5 macht das möglich).

**Fertig, wenn:** ein Lauf mitten in der Arbeit hart abgebrochen (`SIGKILL`) und fortgesetzt
wird — und **bitgleich dasselbe Endergebnis** liefert wie ein durchgehender Lauf.

### E18 — Zwei Nicht-Simulations-Verfahren, die noch fehlen

**(a) Sparsame Ledger-Auswertung.** Ändert sich `spell_17.baseDamage`, wertet der Ledger heute
**alle** Terme neu aus. Nötig sind nur die, die Zauber 17 betreffen:

```
Parameter 17  →  Terme 8, 12, 19  →  drei neu rechnen  →  Differenz aufaddieren
```

Der Index `parameter_to_terms: Vec<SmallVec<TermId>>` fällt beim Bau des Ledgers ohnehin an.
Bei einem Build mit 16 Zaubern ist das grob eine weitere Größenordnung — und es ist genau die
Rechnung, die E12.2 tausendfach ausführt.

**(b) Frühe Schranken im laufenden Kampf.** Branch-and-Bound (E11.5b) schneidet **Suchäste** ab.
Dasselbe Prinzip gilt **innerhalb** eines Kampfes:

```
bisheriger Wert + sichere optimistische Restschranke  <  EINGEFRORENER Amtsinhaber
        →  Restkampf ist unnötig
```

> **Ein so abgebrochener Kampf hat KEINEN vollständigen Ergebnisvektor** — und darf deshalb
> **nicht** in den `CombatOutcomeCache`, der laut E19 zielfunktions-**unabhängig** sein soll.
> Ein späterer Lauf mit anderer Zielfunktion würde dort einen Torso finden und ihn für ein
> Ergebnis halten.
>
> ```rust
> PruneCertificate { structural_key, objective_hash, frozen_incumbent_hash,
>                    bound_derivation_hash, encounter_key,
>                    partial_outcome, optimistic_bound_vector, prune_tick, guarantee }
> ```
>
> Er gehört in einen **eigenen `SearchEvidenceCache`**, nie in den Ergebnis-Cache.
>
> **Und gegen eine Pareto-Front darf nicht skalar gepruned werden** (E19): Ein Kandidat bricht
> erst ab, wenn sein sicherer Obergrenzen-**Vektor** von der eingefrorenen Front in **allen**
> relevanten Dimensionen dominiert wird.

> **Und „die aktuelle Elite" darf während einer Epoche nicht wechseln.** In einem verteilten,
> adaptiven Lauf hinge sie sonst an der Ankunftsreihenfolge aus dem Netz: Derselbe Kandidat
> würde einmal vollständig gerechnet und einmal früh verworfen. Das Ergebnis mag gleich
> bleiben — **das Arbeitsbudget und die Menge der später untersuchten Kandidaten nicht**, und
> damit ist der Lauf nicht mehr reproduzierbar.
>
> ```
> Epochenbeginn   Amtsinhaber-Momentaufnahme EINFRIEREN
> während         alle Schranken NUR gegen diese Momentaufnahme
> Sammelpunkt     nach Kandidaten-Kennung sortieren, neuen Amtsinhaber deterministisch wählen
> ```
>
> Dasselbe gilt für die gestaffelte Stichprobe (E11.1b), die B&B-Schranken, die Budgetvergabe
> (E21.2) und jede Ersatzmodell-Priorisierung.

**Die Schranke muss boss- und zielabhängig sein**, sonst ist sie falsch:

| Bossart            | Restschranke                                                              |
| ------------------ | ------------------------------------------------------------------------- |
| gewöhnlich         | maximal erreichbarer Restschaden                                          |
| `runaway`          | erreichbare Ring-Intervalle — **nicht** Schaden, der Ring läuft umgekehrt |
| Überlebensboss     | obere Schranke der Überlebenswahrscheinlichkeit bzw. Restheilung          |
| Unterbrechungsboss | verbleibende Lösbarkeit der Pflichtmechanik                               |

Bei einer lexikographischen Zielfunktion wird ein **Schrankenvektor** geführt, kein Skalar —
und ein Kandidat darf erst verworfen werden, wenn er in der **ersten noch unterscheidenden**
Dimension sicher verliert. Dieselbe Regel gilt für die gestaffelte Stichprobe aus E11.1b: Deren
Abbruchentscheidungen brauchen **sequenziell gültige** Konfidenzaussagen, weil wiederholtes
Hinsehen bei festen 95-%-Intervallen die Fehlerrate stillschweigend erhöht.

### E19 — Drei Caches statt einem

**Der Ergebnis-Cache aus E11.4b speichert heute implizit eine Bewertung.** Das ist genau dann
teuer, wenn die Zielfunktion sich ändert — und sie ist noch **nicht entschieden** (Teil 6).

```
CombatOutcomeCache    rohe Ergebnisse, zielfunktions-UNABHÄNGIG
ScoreCache            Objective(Outcome)
SearchArchive         Optimalität, Nischenbelegung
```

```
Objective geändert
   →  CombatOutcome bleibt vollständig gültig
   →  nur Score neu rechnen        (Mikrosekunden statt Kampf)
   →  Suche warm fortsetzen
```

**Die Trennung bleibt auch nach der Zielentscheidung nötig.** Die Zielfunktion steht seit dem
2026-08-17 (6.1: Rangfolge Überleben → Schaden → Ressourcen) — aber eine Rangfolge kann sich
ändern, und dann kostet ein Zielwechsel ohne diese Trennung jede gerechnete Kampfstunde
erneut. Sie sichert nicht gegen eine offene Entscheidung, sondern gegen jede künftige.

### E20 — Ein Kommandozeilenwerkzeug, das man ohne Quelltextlektüre bedienen kann

> **Anweisung des Auftraggebers, 2026-08-14:** _„der simulator soll am ende ein einfach zu
> bedienendes CLI-Tool sein, mit guter `--help`, damit die anderen Sessions es leicht bedienen
> können bzw. verstehen, ohne alles analysieren zu müssen."_

**Der Bestand ist das Gegenteil davon**, und das ist messbar:

```
sim/examples/*.rs        30 Stück
sim/src/bin/             existiert nicht
[[bin]] in Cargo.toml    keins
Argument-Parser          keiner
```

**Dreißig Einstiegspunkte, jeder mit eigener Argumentbehandlung, keiner mit `--help`.** Wer
heute etwas messen will, muss wissen, dass `suchguete.rs` die Verfahren vergleicht und
`stufen-durchsatz.rs` etwas ganz anderes tut — das steht nur im Quelltext.

**Das ist dieselbe Falle wie bei den acht abgenommenen Bossen, die im gebauten Stand
unerreichbar waren:** Ein Werkzeug, das nur seine Erbauer bedienen können, ist für die anderen
Sitzungen nicht vorhanden.

#### Ein Einstiegspunkt, Unterbefehle nach Aufgaben

```
rota-sim <BEFEHL> [OPTIONEN]

  run        eine Suche oder einen Sweep fahren
  bench      Durchsatz messen (E11.0-Bedingungen, mit Lastprüfung)
  verify     Parität, Trace-Hashes, Selbsttest der Zusicherungen
  hosts      Rechner auflisten, kalibrieren, Erreichbarkeit prüfen
  status     laufenden Lauf anzeigen  ·  pause | resume | stop
  explain    was WÜRDE dieser Lauf tun — ohne ihn zu starten
  data       Inhalte prüfen, Phänotypen zählen, Invalidierung anzeigen
```

#### Sechs Eigenschaften, ohne die es nicht taugt

**1. `--help` beantwortet die Frage, nicht die Syntax.** Jeder Unterbefehl nennt zwei bis drei
**vollständige, lauffähige Beispiele** — nicht nur die Optionsliste:

```
rota-sim run --help

  Sucht die beste Rotation für einen Build gegen einen oder mehrere Bosse.

  BEISPIELE
    # schnellster Einstieg: ein Boss, Voreinstellungen
    rota-sim run --boss boss-1-1

    # alle Bosse einer Instanz, mit LNS, Ergebnis als JSON
    rota-sim run --instance instance-2 --search lns --json ergebnis.json

    # nur lokal rechnen, halbe Maschine, im Hintergrund
    rota-sim run --boss boss-3-4 --hosts local --profile background
```

**2. Fehlermeldungen nennen den Ausweg.** Nach dem Muster, das sich bei `rota-ipc` bewährt hat
(`ok / hint / affordances / near_matches`):

```
FEHLER  Boss 'boss-2-5' gibt es nicht.
        Meintest du 'boss-2-4'?
        Alle anzeigen:  rota-sim data bosses
```

**3. `explain` vor `run`.** Ein Trockenlauf, der ausgibt, was passieren würde: wie viele
Kandidaten, welche Auswertungswege, welche Rechner, geschätzte Dauer, geschätzter Speicher.
**Bei Läufen über Stunden ist das der wichtigste Unterbefehl** — er kostet Sekunden und
verhindert einen falsch parametrierten Lauf über Nacht.

**4. Voreinstellungen, die stimmen.** `rota-sim run --boss boss-1-1` muss ohne weitere Optionen
etwas Sinnvolles tun: Profil `balanced` auf dem Laptop, erreichbare Zusatzrechner mitnehmen,
Fortschritt anzeigen, Ergebnis auf die Standardausgabe. **Wer eine Option braucht, um
überhaupt etwas zu bekommen, hat kein Werkzeug.**

**5. Zwei Ausgabearten, ein Datenstrom.** Für Menschen eine überschriebene Zeile (E15.2), für
Automatisierung `--json` bzw. `--jsonl`. **Nie zwei getrennte Zähler** — die Anzeige liest
dieselben Werte wie die Datei.

**6. `rota-sim verify` als eine Zeile.** Parität, Trace-Hashes, Determinismus unter Drosselung,
Bitgleichheit über Rechner. Der Selbsttest, den eine Worker-Sitzung ausführen kann, **bevor**
sie eine Messung meldet — und der die Antwort auf „habe ich richtig gemessen?" gibt, ohne dass
jemand die Messumgebung verstehen muss.

**Die 30 Beispiele bleiben.** Sie sind Messwerkzeuge mit dokumentiertem Weg und werden nicht
gelöscht — aber sie sind nicht mehr der Zugang. Was regelmäßig gebraucht wird, wandert als
Unterbefehl ins Werkzeug; der Rest bleibt, wo er ist.

**Fertig, wenn:** eine Worker-Sitzung, die dieses Dokument **nicht** gelesen hat, allein mit
`rota-sim --help` einen Suchlauf über eine Instanz startet, sein Ergebnis als JSON abholt und
`verify` besteht. **Das wird an einer echten Sitzung geprüft, nicht behauptet** — es ist die
einzige Abnahme in diesem Plan, die ein Mensch und kein Messwert entscheidet.

### 4.8 E21 — Laufzeitvertrag und Qualitätszertifikat

_Nachgetragen 2026-08-14 aus dem vierten externen Urteil. Es trifft die Betriebsanforderung,
um die es dem Auftraggeber eigentlich geht — und die dieser Plan bisher nicht beantwortet:_

> **Ein sinnvoll definierter Balance-Lauf soll nach Minuten oder wenigen Stunden mit einem
> belastbaren Ergebnis fertig sein.**

**Was der Plan dazu bisher sagt, ist zu wenig.** Er erklärt richtig, dass der Vollraum
unerreichbar bleibt und nur Projektionen erschöpfend gerechnet werden (Teil 6, Auflage 3) — aber
für die offene Suche steht dort nur „**keine ETA**; ein Mensch oder eine Abbruchregel
entscheidet". **Die Abbruchregel selbst ist nirgends festgelegt**, ebensowenig das garantierte
Mindestprogramm und die Qualität eines Zwei-Stunden-Ergebnisses.

**„Fertig" darf nicht heißen „alles gerechnet".** Es muss heißen:

> Innerhalb eines festgelegten Budgets **alle Pflichtprüfungen erfüllt**, die vorgeschriebenen
> Wechselwirkungen und Nischen **nachweisbar abgedeckt**, danach adaptiv weitergesucht — und am
> Ende ein **Nachweis darüber, was bewiesen und was nur gesucht wurde**.

### E21.0 — Zeitbudget und Arbeitsbudget sind zwei verschiedene Dinge

**Ein harter Zwei-Stunden-Abbruch ist nicht reproduzierbar.** Fällt ein Zusatzrechner aus,
werden weniger Kandidaten geprüft; wird der Laptop gedrosselt, endet der Lauf woanders. **Das
Ergebnis hinge dann an der Tageslast** — und wäre als Balance-Grundlage wertlos.

> **KORRIGIERT am 2026-08-15: Die Vorfassung machte das Budget doch hostabhängig.** Sie lautete
> „Rechner kalibrieren → daraus das Budget rechnen → einfrieren". **Wenn nur der Laptop
> erreichbar ist, entsteht aus denselben zwei Stunden ein kleineres Budget** — und damit ein
> anderes Ergebnis. Genau das sollte der Einfrier-Schritt verhindern.

**Die Arbeitsmenge gehört zur Kampagne, nicht zur Tagesform der Rechner:**

```rust
CampaignDefinition {          // VERSIONIERT, hardwareunabhängig
    profile_version,
    canonical_work_budget,    // aus dem REFERENZPROFIL abgeleitet, dann fest
    exact_obligations, coverage_spec, quality_targets,
}

RunDeployment {               // je Lauf, dynamisch
    available_hosts, cgroup_quotas, runtime_perf_profiles, estimated_eta,
}
```

```
Die Kalibrierung bestimmt:   WO läuft die Arbeit · WIE LANGE dauert sie
Sie bestimmt NICHT:          WIE VIEL Arbeit zur Standardkampagne gehört
```

`--target-time 2h` darf ein **neues** Profil erzeugen. Ein bereits versioniertes Profil hat
feste Werteinheiten.

**Und „Rechner erreichbar" genügt als Referenzprofil nicht** — in einer geteilten Umgebung
sagt es nichts über die Zeit. Das Profil nennt Mindestressourcen:

```
Laptop    physische Kerne · CPU-Quote · GPU-Einschaltdauer · RAM-Budget
proxmox   garantierte CPU-Quote und RAM
gonet     garantierte Mindestquote ODER ausdrücklich "best effort"
Netz      Obergrenze für RTT und Untergrenze für Bandbreite
Thermik   kein aktives Drosseln
```

**Nur ein ausdrücklich verlangter `--hard-deadline 2h` bricht wirklich nach Zeit ab** — und sein
Ergebnis trägt eine eigene Garantie, nicht dieselbe wie ein abgearbeitetes Budget:

> **Ein prozesslokales `Instant` überlebt keinen Neustart** und ist damit für einen Lauf, der
> pausiert und fortgesetzt werden kann (E17), unbrauchbar. Die Semantik muss benannt sein:
>
> ```rust
> enum DeadlinePolicy {
>     WallClockUtc  { deadline: DateTimeUtc },   // "um 08:00 muss es fertig sein"
>     ActiveRuntime { remaining: Duration },     // "zwei Stunden RECHENZEIT"
> }
> ```
>
> Dazu drei Festlegungen: Zählt Pausezeit? Zählt ein Koordinator-Ausfall? Zählt Wartezeit auf
> Zusatzrechner? **Für einen reproduzierbaren `AnytimePartial`-Lauf ist `ActiveRuntime` die
> sauberere Wahl** — die Wanduhr ist die richtige, wenn ein Termin dahintersteht.

```rust
ResultGuarantee::AnytimePartial { work_completed, work_planned }
```

Das ergänzt die vier Garantiearten aus Teil 4 um die fünfte, die im Betrieb am häufigsten
vorkommt.

> **`CampaignRunContract` und `RunContract` sind zwei verschiedene Dinge — der Plan hat sie
> bis zum 2026-08-22 beide `RunContract` genannt.**
>
> | | trägt | Stand |
> | --- | --- | --- |
> | **`RunContract`** (Zielvertrag, Abschnitt 2582) | die Schwellen 99 % / 99,9 %, `determinism`, `target_band`, `vessel_max` | **gebaut**, `sim/src/objective.rs:432`, aus #939, zwei Prüfdateien hängen daran |
> | **`CampaignRunContract`** (Laufvertrag, hier) | Mission, Arbeitsbudget, Fristen, Projektionen | E21.0, #914 |
>
> **Null Überschneidung in den Feldern.** Der eine sagt, was ein guter Kandidat erfüllen muss;
> der andere, wie viel überhaupt gerechnet wird. Gefunden hat es Worker-1 beim Bau von #914,
> dessen AC0 lautet: „Der Laufvertrag ist die Struktur aus dem Plan, nicht eine neu erfundene" —
> und der Plan bot zwei an.
>
> Umbenannt wurde der **hier beschriebene**, nicht der gebaute: Eine Umbenennung im Code fasst
> fremde Prüfdateien an, für einen Namen, den #914 ohnehin neu vergibt. Das Risiko gehört zur
> neuen Änderung, nicht zur alten.

```rust
struct CampaignRunContract {
    mission:                   Mission,
    target_wall_time:          Duration,
    campaign_definition_hash:  Hash,             // WOHER das Budget stammt
    deterministic_work_budget: WorkUnits,        // aus CampaignDefinition, NICHT aus
                                                 // der Kalibrierung
    hard_deadline:             Option<DeadlinePolicy>,  // nur auf ausdrücklichen Wunsch

    exact_obligations:      ProjectionSet,   // was VOLLSTÄNDIG gerechnet werden MUSS
    interaction_coverage:   CoverageSpec,    // t-Wege-Abdeckung, siehe E21.1
    minimum_niche_coverage: f64,
    independent_restarts:   u16,
    plateau_work_units:     WorkUnits,       // die Abbruchregel, endlich benannt
    maximum_bound_gap:      Option<f64>,
    confidence_requirement: ConfidenceSpec,
    final_audit:            AuditPolicy,
}
```

#### E21.0b — Die Machbarkeitsprüfung: lieber melden als still weniger prüfen

**Was passiert, wenn die Pflichtarbeit nicht ins Budget passt?** Ohne diese Prüfung tut das
System das Naheliegende und das Falsche: Es prüft **still weniger** und meldet trotzdem
„fertig".

```
C_Pflichtarbeit  ≤  B_eingefroren   ?
```

Wenn nein, wird **vor dem Start** gemeldet — mit Wegen, nicht nur mit einem Fehler:

```
Standardkampagne in 2 h nicht erfüllbar.
  Pflichtarbeit:  3,4 h auf der aktuellen Rechnermenge
  Wege:
    weitere Rechner zulassen        (--hosts …)
    Zielzeit auf 4 h erhöhen        (--target-time 4h)
    Teilergebnis akzeptieren        (--hard-deadline, liefert AnytimePartial)
```

**Fertig, wenn:** ein Lauf mit absichtlich zu kleinem Budget **abbricht statt zu kürzen** — und
die drei Wege nennt.

### E21.1 — Beschränkte t-Wege-Abdeckung statt „der Rest bleibt Stichprobe"

**Der heutige Satz „der Rest bleibt Stichprobe" ist ehrlich, aber nicht überprüfbar.** Eine
Stichprobe kann eine Wechselwirkung systematisch verfehlen, und niemand merkt es.

**Die Alternative ist eine Abdeckungszusage**, die sich beweisen lässt:

```
t = 2   jede relevante Zweier-Wechselwirkung kommt mindestens einmal vor
t = 3   jede relevante Dreier-Wechselwirkung ebenso
t = 4   nur für mechanisch verbundene Hochrisiko-Synergien
```

**Und sie ist bezahlbar, weil ein Build viele Paare gleichzeitig abdeckt:**

```
60 Zauber  →  C(60,2) = 1 770 Paare
ein Build mit 16 Zaubern deckt  C(16,2) = 120 Paare ab
                              ─────────────────────
untere Schranke:  ⌈1 770 / 120⌉  =  15 Builds

Dreier:  C(60,3) = 34 220,  je Build C(16,3) = 560
untere Schranke:  ⌈34 220 / 560⌉  =  62 Builds
```

**Fünfzehn bzw. zweiundsechzig statt 10⁴⁴.** Die echten Zahlen liegen höher, weil Talent-,
Schul-, Ausrüstungs- und Rotationsbedingungen als **Nebenbedingungen** dazukommen — aber sie
bleiben eine endliche Matrix, die in Stunden läuft.

> **Die Rechnung oben ist eine untere Packungsgrenze, kein Nachweis.** Sie zeigt, dass 15 bzw.
> 62 Builds **nicht unterschritten** werden können — nicht, dass sie genügen. Und
> „zwei Zauber kommen gemeinsam vor" ist **nicht** dasselbe wie „ihre Wechselwirkung wurde unter
> einem Zustand geprüft, in dem sie sich zeigt".

**Die Faktoren, über die abgedeckt wird, sind deshalb mehr als die Zaubermenge:**

```
Zauber UND seine bedingte Regelvariante · Talentweg und Rangklasse
Affixausrichtung · Satz/Legendär · Bossmechanik · Schwierigkeit · Leiterbereich
Ressourcenlage · Priorität in der Leiste · Overdrive-Fenster · Spellsteal-Kontext
```

**Und die Stärke wird gemischt, nicht global gesetzt:**

```
t=2   alle relevanten Faktorpaare
t=3   verbundene Wirkungsgraphen
t=4   ausdrücklich markierte Hochrisiko-Kanten
höher für Boss-Signaturen und Legendär-Ketten
```

> **„Keine gemeinsame Wirkgröße" reicht als Ausschluss nicht.** Zwei Zauber wechselwirken auch
> **transitiv**: A verbraucht eine Ressource, B braucht dieselbe; C belegt das GCD-Fenster, das
> D für den Boss-Cast braucht. Der Wirkungsgraph aus E16 muss diese Kanten enthalten, sonst
> streicht die Abdeckung genau die Paare, die interessant sind.

**Erzeugt wird die Matrix im kalten Pfad**, nie zur Kampfzeit; ungültige Kombinationen sind
Nebenbedingungen, gefundene Elites werden bevorzugt wiederverwendet.

> **Der Erzeuger darf nicht sein eigener Prüfer sein.** Ein unabhängiger Verifizierer hakt alle
> verlangten Tupel **aus der erzeugten Matrix** erneut ab. Ein IPOG- oder Constraint-Löser mit
> Zeitlimit, der seine eigene Vollständigkeit bestätigt, hat genau einen Fehlerfall — und der
> bleibt still.

**Und „im Build enthalten" ist noch keine geprüfte Wechselwirkung.**

```
Zauber A und B stehen beide in der Leiste.
B wird im ganzen Kampf NIE gewirkt.
→ das Tupel ist syntaktisch abgedeckt und semantisch ungeprüft.
```

**Deshalb zählt nur ein Tupel mit Aktivierungszeugen:**

```rust
CoverageObligation { factor_tuple, activation_predicate,
                     required_encounter_classes, required_witness_count }

CoverageWitness { obligation_id, trace_id, activation_tick,
                  participating_effects, relevant_state_snapshot }
```

Der Simulator liefert den Zeugen aus dem Kampfprotokoll; **der Verifizierer zählt nur Tupel mit
gültigem Zeugen**.

> **Aber „beide waren aktiv" beweist noch nicht, dass die Wechselwirkung stattfand.**
>
> ```
> A erzeugt Ressource · B verbraucht Ressource · beide werden gewirkt
> → B verbraucht aber ausschließlich Ressource aus einer ANDEREN Quelle
> → beide aktiv, die zu prüfende Kante ungenutzt
> ```
>
> Nötig ist ein **Herkunftsnachweis** über die Kante selbst:
>
> ```rust
> InteractionWitness {
>     obligation_id, interaction_edge_id,
>     producer_effect, consumer_effect,
>     produced_state_version, consumed_state_version,   // A schreibt v17, B liest v17
>     interaction_kind: ReadAfterWrite | ResourceCompetition | PriorityConflict
>                     | TimingWindow | TriggerChain | MitigationInteraction,
>     activation_tick, trace_id,
> }
> ```
>
> Bei einer APL-Prioritätskante lautet der Zeuge: _beide Regeln gültig, beide Zauber bereit,
> die Reihenfolge entschied tatsächlich_. **Ohne das ist die Matrix eine
> Aktivierungsabdeckung, keine Interaktionsabdeckung** — und genau der Unterschied ist der
> Zweck der Übung.

**Damit bekommt „wir haben den Raum ausreichend abgedeckt" zum ersten Mal eine prüfbare
Bedeutung.**

### E21.2 — Ein globaler Budgetverteiler, nicht nur ein Bandit je LNS-Operator

E11.5a verteilt Budget zwischen **Nachbarschaftsoperatoren**. Das ist die falsche Ebene für die
Frage, die im Betrieb zählt: **Wohin gehört die nächste Stunde Rechenzeit?**

```
                    Pflichtlücke + Unsicherheit + erwartete Verbesserung + Balance-Risiko
Priorität(Aufgabe) = ────────────────────────────────────────────────────────────────────
                            EINGEFRORENE logische Werteinheiten
```

Der Zähler sorgt dafür, dass eine zehnmal bestätigte Nische **nicht** dasselbe Budget bekommt
wie ein bislang toter Zauber, eine Nische ohne Elite, ein Boss mit ungewöhnlichem Abstand oder
eine Zelle mit großer Schranken-Lücke.

> **Der Nenner darf aber NICHT die Laufzeitschätzung sein — sonst ist das eingefrorene Budget
> aus E21.0 wirkungslos.** Wenn `expected_cycles` sich während des Laufs mit der
> Rechnergeschwindigkeit, der Drosselung oder neuen Messwerten ändert, ändert sich auch die
> **Auswahl** der Aufgaben — und das Ergebnis hängt wieder an der Tageslast. Genau das sollte
> E21.0 verhindern.
>
> **Zwei getrennte Modelle:**
>
> ```rust
> LogicalCostModel     // versioniert, beim Start EINGEFROREN
>   normalisierte Werteinheiten je Aufgabenklasse
>   → entscheidet: WELCHE Aufgaben bekommen das Budget
>
> RuntimePlacementModel // dynamisch, jederzeit aktualisiert
>   Hostraten · Druck · Warteschlange · Netz · Temperatur
>   → entscheidet: WO läuft eine Aufgabe
> ```
>
> Ein neu gemessenes logisches Modell darf frühestens in der **nächsten deterministischen
> Epoche** oder im nächsten Lauf greifen.
>
> **Und jeder Operator braucht eine Mindesterkundung.** Ohne sie verhungert ein anfangs
> unterschätzter Operator dauerhaft — womöglich genau der, der eine seltene Zauber-Synergie
> gefunden hätte.

**Die Kandidatenprüfung läuft dabei in Stufen**, und jede darf nur weiterreichen:

```
Stufe 0   Nebenbedingungen · Phänotyp · Cache · sichere Schranken
Stufe 1   analytisch oder Ledger
Stufe 2   kleines aktives Bossset            (E11.5c)
Stufe 3   Greedy-Warmstart
Stufe 4   adaptives LNS                       (E11.5a)
Stufe 5   Herausforderer gegen alle 68 Bosse
Stufe 6   skalarer Referenz-Audit             (Teil 6, Auflage 2)
```

**Endgültig verworfen werden darf nur, wo eine bewiesene Schranke es erlaubt** (E18b). Sonst
ordnet die Stufe nur die Reihenfolge — sie entscheidet nicht.

### E21.3 — Das Qualitätszertifikat: ein Lauf meldet nicht „fertig"

```
Pflichtprojektionen vollständig          100,0 %
Paar-Abdeckung (t=2)                     100,0 %
verbundene Dreier-Abdeckung (t=3)         96,4 %
Zellen mit Schranken-Lücke < 1 %          71,2 %
ungeklärte Zellen                            38

Zauber, Viabilität nach E12.6 (alle sechs Zustände, wörtlich):
   WitnessedViable                         52 / 60      Zeuge gefunden
   CertifiedOptimal                         2 / 60      in einer Projektion bewiesen
   CertifiedNecessary                       1 / 60      ohne ihn sinkt das Nischenoptimum
   CertifiedDominated                       1 / 60      ← DAS ist ein Balance-Befund
   NotFoundWithinBudget                     2 / 60      ← das ist ein SUCH-Befund
   Unresolved                               2 / 60      Nische zu eng oder Schranken zu locker

   verlangte Garantie laut CampaignRunContract:  standard = mindestens WitnessedViable

skalarer Abschluss-Audit               bestanden
stochastische Garantie                     99,0 %   (campaignweit, siehe unten)

BALANCE-VERTRAG:  NICHT ERFÜLLT  — 60/60 verlangt, 55 bescheinigt
```

> **Die vier Viabilitäts-Zeilen dürfen nie zu einer werden.** „Zauber ohne Elite: 2" vermischt
> einen Balance-Befund mit einem Such-Befund — und wer daraufhin die Zahlen eines Zaubers
> anhebt, der gar nicht schwach ist, verschiebt die Balance in die falsche Richtung. Nur
> `CertifiedDominated` rechtfertigt eine Datenänderung.
>
> **Und die Nischenquote entscheidet nicht über den Lauf.** 99,1 % klingt gut und bedeutet
> trotzdem, dass ein Pflichtzauber fehlt. Der Lauf ist dann **abgeschlossen**, aber der
> **Balance-Vertrag ist verletzt** — zwei getrennte Aussagen, zwei getrennte Zeilen.
>
> **Die 99-%-Garantie ist campaignweit zu führen, nicht je Vergleich.** Ein einzelnes
> 95-%-Intervall, tausendfach angewandt, ist keine 95-%-Gesamtaussage. Nötig sind
> jederzeit-gültige Konfidenzfolgen, Alpha-Verteilung über die Zwischenstopps der gestaffelten
> Stichprobe (E11.1b) und gepaarte Auswertung mit gemeinsamen Zufallszahlen.

Und das Zertifikat beantwortet die Betriebsfrage: **Lohnen weitere acht Stunden?** Bei 96,4 %
Dreier-Abdeckung und 38 offenen Zellen: ja, gezielt. Bei 100 % und 0: nein.

### E21.4 — Kampagnenprofile statt freier Parameter

| Profil         |  Zielzeit | Umfang                                                                                         |
| -------------- | --------: | ---------------------------------------------------------------------------------------------- |
| `smoke`        |   1–5 min | Parität, Datenprüfung, wenige Referenzprojektionen                                             |
| `impact`       |  5–30 min | **nur die von einer Änderung betroffene Abhängigkeitsscheibe** (E15.4)                         |
| `interactive`  | 30–60 min | lokale Balance-Iteration mit Warm-Starts                                                       |
| **`standard`** | **2–4 h** | alle Pflichtprojektionen · Nischenabdeckung · adaptives LNS · Herausforderer · Abschluss-Audit |
| `deep`         |    8–12 h | größere Wechselwirkungsabdeckung, mehr Neustarts, stärkere Herausforderer                      |
| `release`      |   12–24 h | volle Matrix: 68 Bosse × Schwierigkeiten × Fortschritt × Referenzprüfungen                     |

**Und der Vollraum wird nicht angeboten, sondern abgelehnt** — mit einem Vorschlag statt einer
Fehlermeldung:

```
FEHLER  Geschätzte Laufzeit überschreitet jedes Kampagnenbudget.
        Vollraum ist nicht ausführbar (Teil 6, Auflage 3).
        Vorschlag:  --coverage constrained-3  --campaign standard
```

### E21.5 — Was die Hardware in diesem Rahmen leisten kann

**Abgeleitet aus den Zielwerten dieses Plans, nicht gemessen** — der Vermerk ist Teil der
Aussage:

```
projizierte Summe der Hostraten        709 000 Vollkämpfe/s   (Teil 2.1)
  — kein End-to-End-Verbundlauf: Koordinator, Netz, Leases und
    gleichzeitig aktive Rechner sind darin NICHT enthalten
E11.2-Ziel Laptop                      470 000/s
  × Verbundfaktor 1,94                 ≈ 912 000/s
HYPOTHETISCH × 1,8                    ≈ 1,64 Mio. aufgelöste Kandidaten/s
  ↑ NUR falls die später in E11.0c GEMESSENE Kampagnen-Inkrementalität 1,8×
    beträgt. E11.4 kennt KEIN globales 1,8×-Ziel mehr (Abnahme je Operator).
```

| Laufzeit | aufgelöste Kandidaten — **illustratives Szenario, keine konservative Prognose** |
| -------: | ------------------------------------------------------------------------------: |
|   30 min |                                                                      ≈ 3,0 Mrd. |
|      2 h |                                                                     ≈ 11,8 Mrd. |
|      4 h |                                                                     ≈ 23,6 Mrd. |

**Ohne** sparsame Ledger-Differenz, GPU, dauerhaften Ergebnisspeicher, Boss-Active-Set und
Abdeckungsmatrix — die kommen obendrauf.

**Und der größte Einzelposten ist immer noch der billigste:** Heute laufen 50–200 identische
Durchgänge je Kandidat. Bei `purity = 0` liefert der Verbund deshalb real nur
`709 000 / 50 = 14 180` bzw. `709 000 / 200 = 3 545` nützliche Kandidaten je Sekunde. Gegen das
Ziel von 1,64 Mio./s sind das **116× bzw. 463×** — der Faktor 50–200 aus E11.1 ist der Grund,
warum eine Projektion von Stunden auf Minuten fällt.

> **Alle Zahlen dieses Abschnitts sind Ableitungen aus Zielwerten**, die noch niemand erreicht
> hat. Sie gehören in die Planung, **nicht in einen Bericht** — und in kein Abnahmekriterium.

### E21.6 — Fortschritt exakt statt gewürfelt

**E11.4e simuliert heute zehntausend Spielerlaufbahnen per Monte-Carlo.** Nach dem Aufbau der
Orakel-Tabelle ist das billig — aber oft unnötig, weil die äußere Schleife **exakt lösbar** ist.

**Das naheliegende Zustandstupel `(Gear-Phänotyp, Freischaltstand, Instanz, Strategie)` ist
nicht Markov** — und der Abschnitt darunter nennt selbst die Gründe. Zwei Inventare können
heute denselben Kampfschaden liefern und eine völlig verschiedene Zukunft haben: verschiedene
Satzteile, andere Gegenstands-Identitäten beim gleichen Phänotyp, andere Mitleids-Zähler,
andere Währungen, andere Sperrzeiten.

```rust
ProgressionState {
    combat_phenotype: CombatPhenotypeId,      // genügt für die KAMPF-Tabelle
    inventory_identity, equipped_items, set_collection_state,
    currencies, pity_counters, duplicate_protection_state, lockouts,
    unlock_state, instance_state, ladder_level, action_policy,
}
```

**Zusammengefasst werden dürfen zwei Zustände nur, wenn sie nicht bloß kampfgleich, sondern
übergangsgleich sind:**

```rust
TransitionEquivalenceCertificate {
    same_available_actions, same_loot_transition_distribution,
    same_upgrade_transition_distribution, same_unlock_transitions,
}
```

**Der `CombatPhenotype` ist die richtige Achse für die Orakel-Tabelle — für die äußere
Markov-Schleife ist er zu grob.** Beute erzeugt die Übergänge.

> **„Ausrüstung wird nur besser" ist keine Ordnung, sondern eine Annahme — und sie hält nicht.**
> Ein höheres Gegenstandslevel kann kampfmechanisch **schlechter** sein oder eine andere Nische
> öffnen: Satzboni brechen beim Tausch, legendäre Effekte ändern die Spielweise, Seitwärts-Funde
> sind je nach Boss besser oder schlechter. Dazu kommen Währungen, Mitleids-Zähler,
> Duplikatschutz und Sperrzeiten — alles Zustand, der **Geschichte trägt**.
>
> **Die geschlossene Formel unten gilt nur mit einer bewiesenen monotonen Potentialfunktion.**
> Ohne sie ist es eine allgemeine absorbierende Markov-Kette oder ein Entscheidungsprozess —
> lösbar, aber nicht mit dieser Abkürzung.
>
> **Und die Endlos-Leiter braucht eine endliche Auswertungsgrenze:** bis zur ersten nachweislich
> unschaffbaren Stufe, bis zu einem Quantil, oder per Binärsuche nach dem Fortschrittslimit.
> Eine „vollständige Release-Matrix" kann keine unendlich vielen Stufen enthalten.

Wo die Monotonie **bescheinigt** ist, folgt die erwartete Kampfzahl geschlossen:

```
           1 + Σ_{t≠s} P(s,t)·E(t)
E(s)  =  ─────────────────────────
                 1 − P(s,s)
```

**Der Nenner entfernt die nutzlose Beute-Selbstschleife algebraisch** — statt sie zehntausendmal
auszuwürfeln. Für Verteilungen (Median, p10, p90 — genau das, was E11.4e liefern soll) läuft
eine dünn besetzte Zustandsfortschreibung; für optimierbare Boss- und Anlege-Entscheidungen ein
Markow-Entscheidungsprozess; **Monte-Carlo bleibt der Rückfall**, wenn der Zustandsraum platzt.

### E21.7 — Der dauerhafte Ergebnisspeicher: wichtiger als 10 % CPU

**E17 sichert den Zustand _eines laufenden_ Laufs.** Was fehlt, ist die Wiederverwendung
**über Läufe hinweg** — und im echten Arbeitsablauf ist das der größere Hebel.

```
RAM-Hot-Cache           kostenbewertet, klein          (E15.1)
      ↓ Fehltreffer
NVMe-Ergebnisspeicher   dauerhaft, über Läufe hinweg   ← fehlt
      ↓ Fehltreffer
echte Auswertung
```

Gespeichert wird der **rohe Ergebnisvektor** (E19), nicht die Punktzahl — dazu gültige
Trace-Ledger, Phänotyptabellen, exakte Projektionsergebnisse, B&B-Schranken und Auditnachweise.
Schlüssel sind die Komponenten-Hashes aus E15.4.

**Eigenschaften:** nur anhängend, in Bündeln geschrieben, **kein Plattenzugriff im
Kampfkernel**, Verdichtung nur im Leerlauf, Ergebnisse der Zusatzrechner als idempotente
Scherben.

**Der Vertrag, ohne den ein persistenter Speicher gefährlicher ist als keiner:**

```
Dateiformat und Schemaversion · starker Inhalts-Hash (BLAKE3, nicht FNV)
Indexstruktur · Wiederanlauf nach Absturz · Prüfsummen je Scherbe
Speicherquote · Verdrängung und Verdichtung · Plattendruck an E15.1 gemeldet
```

**Jeder Eintrag trägt mehr als sein Ergebnis:**

```
vollständiger kanonischer Schlüssel   (Hash nur zum Nachschlagen)
Ergebnis · Garantie · Gültigkeitsbereich · Backend-Semantik
Artefakt-, IR- und Numerikvertrags-Version · Auditstatus
```

> **B&B-Schranken dürfen nur wiederverwendet werden, wenn auch die Version ihrer Herleitung
> übereinstimmt.** Eine Schranke aus einer älteren `UpperBoundCapability` kann zu niedrig sein —
> und eine zu niedrige Schranke verwirft still die richtige Antwort (E11.5b).

**Der Ertrag:** Ein zweiter Balance-Lauf nach einer kleinen Zahlenänderung kann große Teile der
Strukturergebnisse wiederverwenden — bei unveränderten `StructuralKey`s ist das der Unterschied
zwischen Minuten und Stunden. **Die genaue Quote ist zu messen, nicht zu versprechen.**

**Und der Platz ist knapp:** beide ZFS-Pools über 83 % (Teil 2.1). Der maßgebliche Speicher gehört
auf die Laptop-NVMe oder ein eigens reserviertes Dataset; auf den Zusatzrechnern bleiben die
Scherben klein und entbehrlich.

### Ziel-Laufzeiten je Änderungsart

**Zielwerte für die fertige Suite, keine gemessenen Zusagen** — sie machen prüfbar, ob der Plan
sein eigenes Versprechen einlöst:

| Änderung                                 |                 Zielzeit |
| ---------------------------------------- | -----------------------: |
| `baseDamage` eines Zaubers               |     Sekunden bis Minuten |
| Talentwert ohne Strukturänderung         |     Sekunden bis Minuten |
| Abklingzeit oder Wirkzeit eines Zaubers  |                  Minuten |
| neuer Gegenstand, **bekannter Phänotyp** |  **keine Kampfrechnung** |
| neuer Gegenstand, neuer Phänotyp         |                  Minuten |
| neuer Satz- oder legendärer Effekt       |             30 min – 4 h |
| neuer Boss                               |                10–60 min |
| neue Instanz mit 4–6 Bossen              |                    1–4 h |
| neuer Zauber                             |       1–4 h (`standard`) |
| neues Affix                              |                    1–4 h |
| **vollständiger Standard-Balance-Lauf**  |                **2–4 h** |
| tiefer Release-Lauf                      |                   8–24 h |
| Vollraum                                 | **wird nicht angeboten** |

## Teil 5 — Auflagen, die jedes Issue binden

Drei Auflagen aus dem Plan, die jedes Issue bindet:

**1. Bitgleichheit bleibt.** Der Plan verlangt: _„Beide Seiten — Simulator (Rust) und Spiel
(Browser) — müssen jede Datei bestehen, als Pflichttest bei jeder Änderung."_ Jede Optimierung,
die einen Trace-Hash ändert, ist ein Fehler, kein Fortschritt — auch wenn sie schneller ist.

**2. Jede finale Balance-Aussage läuft über den skalaren Referenzpfad.** Die Auflage bleibt —
aber ihre Begründung wird präzisiert.

> **KORRIGIERT am 2026-08-14.** Hier stand: _„Analytischer Krit, Ledger, AVX2: alle liefern
> **Zwischenergebnisse**."_ **Das wirft Exaktes und Genähertes in einen Topf**, und das Urteil
> hat recht, dass die Semantik feiner ist:
>
> ```rust
> enum ResultGuarantee {
>     CertifiedExact,                       // analytischer Krit ohne Rückkopplung;
>                                           // Ledger innerhalb bewiesener Trace-Region
>     BitExactOptimized,                    // AVX2 mit identischer Operationsreihenfolge
>     BoundedApproximation { lower, upper }, // Ersatzmodelle
>     StatisticalEstimate  { confidence },   // Monte-Carlo
>     AnytimePartial { work_completed,      // Lauf per --hard-deadline abgeschnitten
>                      work_planned },      // (E21.0) — die häufigste im Betrieb
> }
> ```
>
> Ein analytischer Erwartungswert **ist** das exakte Ergebnis, keine Näherung davon.
>
> **Die Auflage bleibt trotzdem, und zwar als Auditregel:** Ein Pfad ist exakt, solange seine
> **Voraussetzung** gilt — `crit_affects_control_flow() == false`, Trace-Region eingehalten,
> Operationsreihenfolge unverändert. Diese Voraussetzungen sind Code, und Code hat Fehler. Der
> skalare Nachlauf prüft nicht die Mathematik, sondern **ob die Voraussetzung wirklich galt**.
> Deshalb wird er nicht dadurch überflüssig, dass ein Pfad `CertifiedExact` meldet.

**3. Der Vollraum bleibt unerreichbar.** Bei 350 000 Kämpfen/s bräuchte der **reduzierte** Raum
2,8·10²⁵ Jahre. Kein Issue in E11 darf mit „dann können wir alles durchrechnen" begründet
werden.

> **Was stattdessen zugesichert wird, steht in Teil 4.7:** nicht „alles gerechnet", sondern
> **alle Pflichtprojektionen vollständig, die vorgeschriebene Wechselwirkungsabdeckung
> nachgewiesen, jede Nische adaptiv gesucht — und ein Zertifikat darüber, was davon bewiesen
> und was nur gesucht ist.** Ohne diesen Vertrag ist „der Rest bleibt Stichprobe" ehrlich, aber
> nicht überprüfbar.

---

## Teil 6 — Die Vorgaben und was noch offen ist

### 6.0 Der Stand in einem Satz

**Der Umfang in drei ehrlichen Zahlen** — sie sind verschieden, und der Unterschied ist die
Aussage:

```
87   Issues auf GitHub angelegt              — 11 Epics + 76 Tasks, Stand 2026-08-16
                                               E11 25 · E12 11 · E13 2 · E14 8 · E15 10
                                               E16 3 · E17 4 · E18 3 · E19 3 · E20 5 · E21 12
                                               + Issue-Registry
 0   Kennungen des Plans ohne Issue          — maschinell abgeglichen; E11.0c und E11.8
                                               tragen bewusst keines (Container bzw.
                                               ausdrücklich „kein Issue")
```

> **Diese Zahl ist Handpflege und wird altern — zum letzten Mal.** Die Issue-Registry
> (Phase A0) erzeugt sie künftig aus einer Quelle. Bis dahin gilt: **maßgeblich sind die
> GitHub-Issues, nicht diese Zeile.** Die frühere Angabe „56 ausgeschrieben, 29 davon
> umsetzungsfertig" ist überholt — jedes angelegte Issue trägt inzwischen eigene
> Abnahmekriterien, die drei Auflagen aus Teil 5 und keine Querverweise.

**Mechanisch nachgezählt am 2026-08-15** (`### `/`#### `-Überschriften in Teil 4, ohne E11.8,
das ausdrücklich **kein** Issue ist): E11 23 · E12 10 · E15 7 · E21 9, dazu E13, E14 und
E16–E20.

> **Diese Zahl ist VIERMAL auseinandergelaufen, und das ist ein Werkzeugproblem, kein
> Sorgfaltsproblem.** Eine Zahl in Fließtext altert bei jedem neuen Abschnitt — zuletzt ergab
> die eigene Rechnung im Dokument 56, während die Zeile darüber 54 nannte.
>
> **Die Registry ist deshalb keine spätere Möglichkeit mehr, sondern Teil von Phase A0**
> (Teil 4.0) — vor jedem weiteren Issue:
>
> ```yaml
> - id: E11.5d
>   title: Exakter Prioritätslisten-Löser
>   depends_on: [E11.5b, E11.4f, E16]
>   normative_section: e11-5d
>   status: planned
>   acceptance:
>     - matches_exhaustive_small_projection
>     - reports_verified_apl_bound
> ```
>
> **Die CI erzeugt daraus** Reihenfolgetabelle, Abhängigkeitstabelle, Epic-Zählung, Status und
> Inhaltsverzeichnis — und prüft: jede Kennung einmal · jede Abhängigkeit vorhanden · **keine
> Zyklen** (E12.6/E21.3 wäre so nie entstanden) · jedes gezählte Issue mit Abschnitt · jedes
> „umsetzungsfertige" mit prüfbarer Abnahme.
>
> **Dann existiert jede Information genau einmal.** Bis dahin ist jede Zahl hier eine
> Momentaufnahme.

**Die Lücke zwischen ausgeschrieben und umsetzungsfertig** sind Issues, die Zweck und
Abhängigkeit haben, aber noch kein prüfbares „Fertig, wenn" — wer eines davon zieht, formuliert
es zuerst. **E12 ist seit dem 2026-08-15 vollständig ausgeschrieben** (Teil 4.4, E12.0–E12.8)
und seit dem 2026-08-17 auch umsetzbar: Die Vorgaben in 6.1 sind entschieden.

**Gebaut: keines.** Dieser Plan ist Planung; der Simulator hat heute den Stand aus Teil 2.

**Vor dem Beginn zu lesen: 6.1.** Dort stehen die Vorgaben des Auftraggebers, die jedes Issue
voraussetzt. Offen sind nur noch die Messungen aus 6.2 — und die klärt man durch Messen, nicht
durch Fragen.

**Der erste Meilenstein umfasst nicht den ganzen Plan.** Entschieden am 2026-08-17: Gebaut wird
zuerst ein schlanker Kern, der Balance-Aussagen möglich macht; die Endausbau-Infrastruktur
(GPU-Pfad E13, Verteilung und Drosselung E15, Fortsetzbarkeit E17, Ledger-Schranken E18,
numerischer Vertrag E14, die Zertifikatsebene aus E21) ist **vertagt, nicht gestrichen**. Sie
löst Lastprobleme, die 17 Bosse noch nicht erzeugen. Welche Issues zum Kern gehören, führt die
Issue-Registry (#895) — nicht dieser Absatz, denn eine Liste an zwei Orten altert an einem von
beiden.

### 6.1 Die Vorgaben des Auftraggebers

**Entschieden am 2026-08-17.** Diese dreizehn Punkte sind Vorgaben, nicht Vorschläge — jedes
Issue setzt sie voraus und darf ihnen nicht widersprechen.

#### Der Zielvertrag

| Vorgabe               | Wert                                                                                                                                         |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Form der Zielfunktion | **lexikographische Rangfolge**, harte Bedingungen vor allen Dimensionen                                                                      |
| Grobstruktur          | **Überleben → Schaden → Ressourcen**                                                                                                         |
| „Überleben" misst     | **Siegwahrscheinlichkeit** über die Zufallsströme, nicht Restleben                                                                           |
| Zulässigkeitsschwelle | **Sieg ≥ 99 %, Mechanikerfolg ≥ 99,9 %** — nachgewiesen über die **untere Konfidenzgrenze**, nie über den Schätzwert                         |
| Schwellen sind        | **versionierte Felder des Laufvertrags**, keine globalen Konstanten — ein Endgame-Boss darf mehr Sicherheit verlangen als ein Instanz-1-Boss |

**Die ausgeschriebene Rangfolge steht in E12.0** und hat acht Stufen. Die Grobstruktur oben ist
ihre Zusammenfassung, nicht ihr Ersatz: Stufen 1–2 sind „Überleben", Stufe 5 ist „Schaden",
Stufe 8 ist „Ressourcen". Die Zwischenstufen entscheiden nur bei Gleichstand aller oberen — in
der Praxis fällt die Entscheidung fast immer auf Stufe 1 oder 2.

Eine skalare gewichtete Summe ist damit ausgeschlossen. Der Grund ist kein Geschmack: Ein
Aufbau, der an einem Boss stirbt, darf das nicht durch Schaden an einem anderen aufwiegen —
und genau das täte jede Summe.

**Die Siegwahrscheinlichkeit ist exakt berechenbar, nicht nur schätzbar.** Der Kampf hat
**zwei** aktive Zufallsquellen (`sim/src/rng.rs`: `Crit` und `DotTick`; `Heal` ist reserviert
und zieht heute nichts). Bei zwei Quellen ist die gemeinsame Verteilung führbar — die exakte
Rechnung aus E11.1b ist der **Hauptweg** zur Siegwahrscheinlichkeit, die Stichprobe ist der
Rückfall, wenn der Zustandsraum platzt.

**Deshalb kostet eine strengere Schwelle hier fast nichts.** Über eine Stichprobe wäre der
Nachweis von 99 % rund zwanzigmal so teuer wie der von 95 %; über die exakte Verteilung ist es
derselbe Rechenweg mit einer anderen Vergleichszahl. Die Schwelle darf also nach dem
Spielanspruch gewählt werden, nicht nach den Rechenkosten.

Daraus folgt für die statistische Fehlerkontrolle (E21.3b): Sie ist **nur dort** nötig, wo
tatsächlich gestichprobt wird. Wo exakt gerechnet wird, gibt es kein Irrtumsrisiko und damit
auch kein Mehrfachvergleichsproblem — wo geschätzt wird, gibt es beides, und zwar hart: Bei
zehntausend Kandidaten, jeder gegen die 99-Prozent-Schwelle mit 1 % Irrtumsrisiko geprüft,
sind rund **einhundert** falsch als zulässig eingestuft. Eine Einzelprüfung, die für sich
korrekt ist, wird über die Menge zur Fehlerquelle.

#### Zusammenfassung über Bosse

| Vorgabe                 | Wert                                                                        |
| ----------------------- | --------------------------------------------------------------------------- |
| über alle Bosse hinweg  | **keine** globale Zusammenfassung, keine globale Elite                      |
| innerhalb einer Instanz | **schwächster Boss zählt** — eine Instanz läuft man mit einer Rotation      |
| Test- und Prüfbosse     | **maschinell aus jeder Wertung ausgeschlossen**, der Bau bricht bei Verstoß |

**Gemessen am 2026-08-17:** Die Instanzstruktur trägt das ohne Umbau — jeder Spielboss hat
genau eine `instance`: vier Instanzen mit je vier Bossen, dazu `prologue` mit
`first-crucible`. Ein `optional`-Feld gibt es nicht, alle vier Bosse einer Instanz sind
Pflicht. Daneben liegen **18 Test- und Prüfbosse** (`test-*`, `orch-maxload`) im selben
Verzeichnis — eine Elite, die versehentlich gegen einen Prüfboss optimiert wurde, sieht im
Ergebnis vollkommen plausibel aus. Deshalb ist der Ausschluss maschinell und nicht als
Warnung ausgeführt.

#### Werte-Deckel und Startwerte

| Vorgabe           | Wert                                                                                        |
| ----------------- | ------------------------------------------------------------------------------------------- |
| Höhe der Deckel   | **technisch maximal** — so hoch, wie Überlaufbeweis und Browser-Grenze es tragen            |
| Rolle des Deckels | **Rechenschranke, keine Balance-Aussage**                                                   |
| Base Values       | **Bestand ist Startpunkt**, der Optimierer verschiebt sie innerhalb des Deckels             |
| zusätzlich        | ein **Vergleichslauf von neutralen Startwerten**, um den Einfluss des Startpunkts zu messen |

Der Deckel begrenzt den **Suchraum**, nicht das Ergebnis — er ist eine Eingabe der Suche und
wird von ihr nicht mitbalanciert. Andernfalls verschiebt der Optimierer seine eigene Grenze
und ist wieder unbeschränkt.

**Daraus folgt: der Deckel bremst den Ausrüstungskollaps nicht.** Bleibt im Endausbau ein
Zauber übrig, ist das über Base Values und Optimierer zu lösen, nicht über die Obergrenze.
Läuft der Optimierer bei einer Wirkgröße dennoch gegen ihren Deckel, wird **diese eine Zeile
gemeldet** — sie ist dann ein Befund, keine Randbedingung.

#### Vielfalt

| Vorgabe        | Wert                                                                     |
| -------------- | ------------------------------------------------------------------------ |
| erste Achse    | **Zauberschule**                                                         |
| weitere Achsen | **zwei bis drei aus der Wirkungsbeschreibung abgeleitet** (E16.0, E16.1) |

Schulen allein genügen nicht: Zwei Feuer-Aufbauten können sich vollkommen verschieden spielen
und gälten als dieselbe Nische — dann findet die Suche genau die Vielfalt nicht, die gesucht
wird. Abgeleitete Achsen allein genügen auch nicht: Es könnte keine Frost-Nische ausgewiesen
werden, weil Frost-Aufbauten sich rechnerisch wie Feuer verhalten. Deshalb beides.

#### Bossmechanik und das Kontroll-Loch

| Vorgabe                          | Wert                                                                             |
| -------------------------------- | -------------------------------------------------------------------------------- |
| HP-Phasen                        | **bleiben selten** — Bestand: **2 von 17** (`boss-1-2`, `boss-3-4`, je 2 Phasen) |
| laufender Boss-Zauber in der APL | **wird aufgenommen** (`boss.casting` plus Restzeit)                              |

Das zweite ist kein Widerspruch zum ersten. Die Suchsprache kennt Bosszustände längst —
`BossHp`, `BossHpPercent`, `Control`, `DotActive` und weitere stehen im `enum Value`. Es fehlt
**ein** Wert, und sein Fehlen ist gemessen: FIXATION wird bei **keinem** Gewicht gewählt, auch
an einem Boss mit tödlichem Cast, weil die Bedingung unausdrückbar ist. Kein Wert von
`prevented_weight` ändert das. Das ist ein Werkzeug-Loch, kein Balance-Loch — und ein Teil der
Erklärung dafür, dass 32 von 43 Zaubern in keiner besten Rotation vorkommen.

Zu prüfen, nicht zugesichert: `boss.casting` hängt an der **Bosszeitleiste** und die ist
deterministisch — anders als `BossHpPercent`, das am zufälligen Schadensverlauf hängt. Trägt
das, kostet die Aufnahme den analytischen Krit-Pfad nichts.

---

---

### 6.2 Der Stand der offenen Messungen

_Vier davon sind am 2026-08-14 nachgemessen worden. **Jede trägt ihren Maßstab**: gegen
Bestandsdaten gemessen, oder gegen einen Endausbau-Stellvertreter._

#### Gemessen

**Trace-Stabilität bei Zahlenänderungen** — betrifft E11.3, E11.3a, E12.
_Maßstab: Bestandsdaten (58 Zauber, 4 echte Bosse), eine greedy-Rotation mit 6 Zeilen._

```
Ab welcher Änderung kippt die Cast-Folge?
                  boss-1-1  boss-1-2  boss-1-3  boss-1-4
essence               1,0 %    >100 %     5,0 %     5,0 %
talents               1,0 %    >100 %     5,0 %     5,0 %
gear_effects          1,0 %    >100 %     5,0 %     5,0 %
buffs                50,0 %    >100 %    >100 %   100,0 %
synergy              >100 %    >100 %    >100 %    >100 %
resistance            1,0 %    >100 %     5,0 %     5,0 %
vulnerability        >100 %    >100 %    >100 %    >100 %
crit                  1,0 %    >100 %     5,0 %     5,0 %
```

**Der Ledger lebt — aber die Region ist schmal.** 1–5 % bei den schadenswirksamen Größen. Das
bestätigt zweierlei: dass mein früherer Schluss („1 % kippt → Ledger gestorben") falsch war,
**und** dass die Intervall-Darstellung aus E11.3a nötig ist statt einer Weite-Angabe.

> **Zwei Kontrollzeilen liefen mit, und die zweite hat einen Fehlbefund verhindert.** Der erste
> Lauf ergab **überall >100 %** — scheinbar perfekte Stabilität. Die Kontrollzeile „Faktor 1000
> muss die Folge ändern" schlug an: **sie tat es nicht.** Der Grund: mit fester Kampfdauer und
> ohne Kill-Abbruch kann Schaden den Kontrollfluss gar nicht berühren. Erst mit
> `stop_on_kill = true` misst der Weg etwas — dann kippt die Folge dort, wo der Boss **einen
> Cast früher** fällt.
>
> **Daraus folgt eine Präzisierung für E11.3a:** Die Trace-Region ist im Kern das Intervall
> zwischen zwei Kill-Zeitpunkten. `boss-1-2` bleibt >100 % stabil, weil er in 60 s nicht fällt.
> **Endausbau-Vorbehalt:** Mehr Bosse mit HP-Phasen und Schwierigkeitsstufen heißt mehr Bosse,
> die überhaupt fallen — die Regionen werden **enger**, nicht weiter.

**DAG-Teilung an vier Bossen** — betrifft Anhang B.2.1.
_Maßstab: Bestandsdaten, vier Bosse mit unterschiedlicher Mechanikdichte._

```
Boss        Kandidaten   Teilung  Äquivalenz  Rest-DAG
boss-1-1           528     1,91x       1,77x     1,07x
boss-1-2           528     2,22x       1,81x     1,10x
boss-2-2           528     2,74x       2,18x     1,20x
boss-3-4           528     2,22x       1,90x     1,17x
```

**Der Befund hält: 1,07–1,20× für den DAG nach der Deduplizierung.** Die größte benannte
Schwäche der ursprünglichen Messung (nur ein Boss) ist damit ausgeräumt.

> **Und eine meiner Vermutungen ist widerlegt.** Ich hatte geschrieben, mehr gleichzeitige
> Mechaniken bedeuteten **weniger** Teilung. `boss-2-2` ist der mechanikreichste der vier
> (affliction + race + timelineLoopMs) und hat die **höchste** Teilung (2,74×) und den höchsten
> DAG-Rest (1,20×). Die Richtung war falsch — der Abstand zwischen billiger Deduplizierung und
> teurem Graphen bleibt aber in allen vier Fällen groß.

**Durchsatz am Endausbau-Lastprofil** — betrifft E11.0b, E11.2.
_Maßstab: **gemischt** — der Boss ist ein Endausbau-Stellvertreter (`data/bosses/orch-maxload.json`,
**zehn** gleichzeitige Systeme gegen höchstens vier bei echten Bossen), Zauber und Rotation
sind Bestand._

```
boss-1-1  (2 Systeme)    108 250 Kämpfe/s
maxload  (10 Systeme)     77 058 Kämpfe/s      Faktor 1,40x langsamer
```

**Zehn gleichzeitige Systeme kosten rund 40 % Durchsatz.** Das ist die erste Zahl im Plan, die
den Endausbau nicht schätzt, sondern misst.

> **Zwei Vorbehalte, beide erheblich.** Der Boss musste auf **Schadenswert 1** heruntergesetzt
> werden, damit der Kampf überhaupt durchläuft: Die erste Fassung tötete den Spieler in
> Sekunden und lief dadurch **schneller** als `boss-1-1` (117 326 gegen 104 607) — die Zahl
> hätte „wie schnell stirbt er" gemessen statt „was kosten zehn Systeme". Der gemessene Schaden
> ist deshalb **0**, der Spieler wirkt nichts. **Die Zahl misst die Kosten der Bossmechanik,
> nicht die eines vollen Kampfes.**
>
> Und die **Queue-Verteilung fehlt weiterhin**: Das Messwerkzeug dafür braucht eine
> Instrumentierung in `fight.rs` (`ORCH_QLEN`, `queue_pushes`), die nicht mehr im Code ist. Die
> Kapazitätsentscheidung 32/48/64 steht damit weiter aus.

**Talent-Phänotypen des vollen Raums** — betrifft E11.1.
_Maßstab: Bestandsdaten (29 Talente, 6 Bäume, 50 Punkte)._

```
KONTROLLZEILE  gültige Verteilungen: 3,214989e16   erwartet 3,21e16   TRIFFT
```

**Zwei Anläufe, beide gescheitert:**

```
naive Aufzählung aller Vektoren   Abbruch bei Talent 12/29 — 5,7 Mio Teilvektoren
Faktorisierung über damage +
  Ränge der 15 Sondertalente      Zeitlimit 900 s überschritten
```

**Der Faktor 234 bleibt damit eine Ableitung**, und die offene Frage ist präziser als vorher:
Nicht „wie zählt man das", sondern **„welche Zerlegung ist billig genug"**. Die
Größenverhältnisse sind bekannt — 29 Talente in 6 Bäumen, 16 Wirkgrößen, davon bewegen **26
Talente nur `damage`** und die übrigen 15 Größen haben je ein bis zwei Träger. Eine Zerlegung
je Baum mit anschließender Faltung über das Punktebudget sollte reichen; sie ist nicht gebaut.

> **Und der Messlauf selbst hat eine Falle gezeigt, die im Plan steht und die ich trotzdem
> getreten habe:** Der Hintergrundlauf meldete **Exit-Code 0**. Der stammte vom abschließenden
> `date`-Aufruf derselben Befehlskette — die Messung darüber war vom `timeout` gekillt worden.
> **Ein Exit-Code gilt für den letzten Befehl der Kette, nicht für den interessanten.**

#### Weiterhin offen, mit Grund

| offen                                           | betrifft      | warum es (noch) nicht geht                                                                                                                                                                                                                                          |
| ----------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Queue-Verteilung am Lastprofil**              | E11.0b, E11.2 | Die Instrumentierung in `fight.rs` fehlt — sie ist Teil von E11.0                                                                                                                                                                                                   |
| **`expire_buffs`-Trefferquote im Endausbau**    | E11.2         | dieselbe Instrumentierung                                                                                                                                                                                                                                           |
| **Talent-Phänotypen, exakt**                    | E11.1         | braucht die faktorisierte DP, nicht die naive Aufzählung                                                                                                                                                                                                            |
| **Cache-Trefferquote in einem realen LNS-Lauf** | E11.4b        | setzt E11.4b voraus — Code existiert nicht                                                                                                                                                                                                                          |
| **Bitgleichheit über AVX2/FMA**                 | E11.7, E14    | setzt E11.7 voraus — der Nachweis in Teil 2.1 gilt für den **skalaren** Pfad                                                                                                                                                                                        |
| **Ob ein LXC unter Dauerlast die Nodes stört**  | E15.5         | **es gibt keinen LXC.** Alle bisherigen Messungen liefen direkt als root auf dem Host, per `scp`+`ssh`. Der Plan beschreibt `cpuunits` und cgroup v2 — beides ungetestet. Einen Container anzulegen ist ein Eingriff in die Infrastruktur und braucht eine Freigabe |

### 6.3 Was dieser Plan bewusst nicht löst

**Vollraum-Enumeration** (Teil 5, Auflage 3) · **Multi-Policy-DAG** (gemessen 1,07×, mit
Wiedervorlage-Auslöser, Anhang B.2.1) · **Regel-Interaktionsgraph über Bedingungen** (0,82 %,
kippt erst mit zustandsabhängigen Kandidatenzeilen) · **Assembler als allgemeine Kernelsprache**
(Stufenregel in E15.6) · **iGPUs im Normalbetrieb** (voreingestellt aus, messbar).

---

## Anhang A — Die Messungen, auf die sich dieser Plan stützt

Alle in `docs/SIM-MESSUNGEN.md` mit ihrem Läufer und ihren Einstellungen. Die für E11
tragenden:

```
Baseline            350 000–365 000 Kämpfe/s, ±12 %      (5900HS, 16 Stränge)
Kernel              expire_buffs 23,8 % · Heap 16,1 % · APL 4,1 %
                    59 340 cycles/fight · IPC 2,65 · 155 branch-misses
Eventschlange       max 9 · p95 8 · 112 pushes je Kampf · EVENT_CAPACITY 256
Buff-Abläufe        höchstens 1 je Kampf · 140 von 210 Kämpfen ohne einen
Zustand             size_of::<Fight>() 5 600 B · 11 Arrays à 70
Krit                0 Würfe bei purity = 0 · 2 von 17 Bossen mit HP-Phase
Rotation            Position p → Präfix p, null Streuung
Suche               LNS 91,7 % > beam 85,0 % > greedy+Paare 83,3 %
Faltung             Gear 312 (Endausbau 1 162) · Talente 234 · Rotation 5,0
Talentraum          3,21·10¹⁶ gültige Verteilungen, requires nimmt 93,3 %
Homogenität         76 Gestalten je Runde, größte 30, Median 6, null Einzelgänger
Subsumption         alle 35 bedingten Zeilen subsumierbar, ~21 % bei Länge 12
```

**Nachgetragen 2026-08-14** (`sim/examples/orch-policy-dag.rs`, boss-1-1, 60 s, Startwert 5,
zwei Kontrollzeilen — 558 identische Folgen ergeben exakt 558,0, 558 aus 251 Mustern ergeben
2,22 = 558/251):

```
Suchrunde           528 Vollkämpfe bei Basislänge 5 (rotation_search.rs:576)
Präfix-Teilung      21 074 Entscheidungen -> 11 041 Zustände       Faktor 1,91
Policy-Äquivalenz   528 Kandidaten -> 298 Klassen                  Faktor 1,77
größte Klasse       216 — alle mit UNVERÄNDERTER Basisfolge        40,9 %
Rest für den DAG    nach Deduplizierung                            Faktor 1,07
  Gegenprobe ohne Zeitstempel (günstigste Lesart)                  Faktor 1,08
Regelpaare          4 278 gesamt, 35 mit toter Anordnung           0,82 %
Haste-Breakpoints   je Einzelzeit Faktor 1,7–26,6
                    je Build mit 14 Zaubern, catalysis 0..1000     Faktor 1,00
Zweitrechner        rustbuild i7-3930K: avx, KEIN avx2, KEIN fma
```

**Nachgetragen 2026-08-14** (`sim/examples/orch-verteilung.rs`, dasselbe Binary auf drei
Systemen, 200 000 Kämpfe, Aufwärmlauf davor):

```
Laptop 5900HS  16 Str.   210 094 Kaempfe/s   Load 1,96 — gedrueckt
                                             ruhige Baseline bleibt 350 000-365 000
gonet i7-3930K 12 Str.   121 689             Load 2,75, KEIN avx2/fma
proxmox i5-1235U 12 Str. 221 688             Load 0,73, avx2+fma+vnni

Bitgleichheit  FNV-1a ueber to_bits() von 48 Werten, 4 Bosse x 3 Startwerte,
               krumme Faktoren:  0x42210c39918563b9  auf ALLEN DREI
               (die frueher genannte Dezimalsumme 102243,0000000000 war KEIN
                Bitvergleich; mit NEUTRAL waere der Test ohnehin wertlos,
                weil min == max == 7800)

Netzwerk       RTT  gonet 1,33 ms · proxmox 1,69 ms
               = 485 bzw. 617 Kaempfe — eine ganze Suchrunde
```

Hardware der drei Systeme (vollstaendig in Teil 1c.7):

```
                  Laptop 5900HS      gonet i7-3930K      proxmox i5-1235U
Kerne/Straenge    8 / 16             6 / 12              2P + 8E / 12
L2 je Kern        512 KiB            256 KiB             1,25 MiB P / 512 KiB E
L3                16 MiB             12 MiB              12 MiB
RAM               LPDDR4-4266 15G    DDR3-1333 39G       DDR4-3200 30G
AVX2 / FMA        ja                 NEIN                ja
BMI · F16C · ADX  ja                 NEIN                ja
GPU               RTX 3050 Ti 4G     Radeon RX 470-580   Iris Xe (iGPU)
                  CUDA cc 8.6        kein CUDA           kein CUDA
Systemdisk        NVMe 2T btrfs 83%  SATA-QLC 4T zfs 93% NVMe 256G zfs 83%
```

Bossdaten, nachgezaehlt ueber alle 35 Dateien:

```
Systeme        elf, nicht sieben (escalation, bossHeal, timelineLoopMs,
               resistances waren in der Vorfassung nicht gezaehlt)
gleichzeitig   max 4 (boss-2-4), Median 2 — Design verlangt 3-4 plus Signatur
mechanic-Werte 2 (strike 38 · interruptibleCast 22)
Bausteine      alle NEUN aus GAME-DESIGN 11.2 sind im Simulator vorhanden
```

**Nachgetragen 2026-08-14, zweite Messrunde** (Belege und Vorbehalte in Teil 6.2):

```
Trace-Stabilitaet    1-5 % bei essence/talents/gear/resistance/crit
                     >100 % bei synergy/vulnerability und an boss-1-2
                     Kontrollzeile deckte einen Fehlbefund auf: ohne
                     stop_on_kill sind Schadensfaktoren kontrollflussneutral

DAG an 4 Bossen      Rest nach Deduplizierung 1,07x / 1,10x / 1,20x / 1,17x
                     boss-2-2 (mechanikreichster) hat die HOECHSTE Teilung —
                     meine Vermutung "mehr Mechanik = weniger Teilung" ist falsch

Endausbau-Lastprofil 10 Systeme gegen 2:  77 058 gegen 108 250 Kaempfe/s
                     Faktor 1,40x  (orch-maxload.json, Schaden auf 1 gesetzt)

Talentraum           Kontrollzeile 3,214989e16 TRIFFT
                     volle Phaenotyp-DP nicht durchgelaufen (5,7 Mio Teilvektoren
                     bei Talent 12/29) — Faktor 234 bleibt eine Ableitung
```

**Wie diese Zahlen im Endausbau zu lesen sind:** Alle sind _Mengen_, keine _Strukturen_ (Teil 1).
Der Teilungsfaktor sinkt mit mehr gleichzeitigen Bossmechaniken, die Deduplizierung steigt mit
mehr Zaubern und längeren Listen. **Der Abstand zwischen beiden — und damit der Befund gegen den
DAG — wird dadurch eher größer.** Der Haste-Faktor 1,00 kann nicht steigen: mehr Zauber je Build
heißt mehr verschiedene Basiszeiten, also feiner zerschnittene Intervalle.

---

## Anhang B — Die geprüften externen Vorschläge

_Vier externe Architekturvorschläge sind in diesen Plan eingegangen. Dieser Anhang hält fest,
**was geprüft wurde und was dabei herauskam** — er ist Beleg, nicht Planung. Wer umsetzt,
braucht ihn nicht; wer wissen will, warum eine Entscheidung so und nicht anders fiel, findet
sie hier._

### B.1 Erster Vorschlag — die Zahlen und drei Überdehnungen

#### B.1.1 Die zitierten Zahlen stimmen

Alle Messwerte, auf die sich der Vorschlag stützt, stammen aus `docs/SIM-MESSUNGEN.md` und
sind dort mit ihrem Läufer belegt:

| Behauptung                                                    | geprüft                                                                   |
| ------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Queue max 9, Mittel 5,90                                      | ✔ eigene Messung, 200 Kämpfe                                              |
| `expire_buffs` 23,8 % der Zyklen, 99,7 % der Prüfungen leer   | ✔ perf + eigener Lauf                                                     |
| BinaryHeap 16,1 % der Zyklen                                  | ✔ perf                                                                    |
| APL-Auswertung nur 4,1 %                                      | ✔ perf                                                                    |
| Suffix-Reduktion Faktor 5,0 (Endausbau 10,6)                  | ✔ eigene Messung                                                          |
| Gear 17 496 → 56, Endausbau 6 291 456 → 5 412                 | ✔ nachgefahren                                                            |
| Talente 19 456 → 83, Faktor 234                               | ✔ zweifach (Worker-1 und eigene Python-Rechnung)                          |
| gültige Talentverteilungen 3,21·10¹⁶, `requires` nimmt 93,3 % | ✔ DP mit zwei Kontrollzeilen                                              |
| greedy 83,3 %, beam k=10 85,0 %, LNS 91,7 %, QD 68,3 %        | ✔ 60 Fälle mit bekanntem Optimum                                          |
| Position p → Präfix p unberührt, null Streuung                | ✔ 563 Varianten — **gilt aber nur für gefundene Rotationen, siehe E11.4** |
| Homogenität: 76 Gestalten, größte 30, Median 6                | ✔ nachgefahren                                                            |
| `f64` durchgehend, kein `f32`                                 | ✔ 231 Vorkommen, kein `f32`                                               |
| Baseline 350 000–365 000 Kämpfe/s ±12 %                       | ✔ zweimal korrigiert                                                      |

#### B.1.2 Drei Stellen, an denen der Vorschlag zu weit geht

**(a) „Trace-Ledger: zig Millionen Auswertungen/s"** — als `[ANNAHME]` gekennzeichnet, aber
die Voraussetzung ist ungeprüft. Der Ledger setzt voraus, dass ein Trace bei einer
Parameteränderung **stabil** bleibt. Gemessen ist das Gegenteil für Rotationsänderungen
(Position p → sofortige Divergenz ab p). Für **Zahlenänderungen** war es zum Zeitpunkt dieser
Prüfung ungemessen — das war der Grund, E11.3 vor E11.3a zu setzen.

> **Inzwischen gemessen** (Teil 6.2, 2026-08-14): Die Region ist **1–5 % breit** bei den
> schadenswirksamen Größen. Der Ledger ist damit nicht widerlegt, aber sein Nutzen hängt daran,
> wie eng der Optimierer sucht.

**(b) „Same-Spell-Subsumption entfernt Rotationen"** — gemessen, und der Umfang ist kleiner
als er klingt.

**Zur Einordnung, weil die Zahlen leicht zu verwechseln sind:**

```
58   spielbare Zauber            (60 im Endausbau laut Plan)
93   Kandidatenzeilen der Suche

davon  58   bedingungslose Zeilen  — jeder Zauber hat genau eine
       35   bedingte Zeilen        — verteilt auf nur 25 Zauber
```

Die übrigen **33 Zauber** tauchen in der Suche ausschließlich bedingungslos auf — für sie gibt
es nichts zu subsumieren. Beispiel für einen der 25: `pyre` hat `pyre` und
`pyre if heat >= 15`; `kindle` dagegen bringt keine Bedingung mit und hat nur eine Zeile.

```
Zauber mit bedingungsloser Zeile        58
Zauber mit mindestens einer bedingten   25
davon mit BEIDEM (subsumierbar)         25   ← alle 25
betroffene bedingte Zeilen              35   ← alle 35
```

**Von den 25 Zaubern mit bedingter Zeile hat jeder auch eine bedingungslose**, deshalb sind
alle 35 bedingten Zeilen subsumierbar, sobald die bedingungslose davorsteht.

Über zufällige Listen gerechnet enthalten bei Länge 12 rund **42 %** mindestens ein Paar,
davon ist die Hälfte tot — also grob **21 %** des Raums, Faktor 1,27. Das ist exakt und
billig, aber es ist keine Größenordnung. **Und die Rechnung ist eine Wahrscheinlichkeit über
zufällige Listen, keine Messung an der Suche** — die zieht nicht zufällig.

**(c) „`Fight::new` durch Template-Reset ersetzen"** — die Begründung trägt nicht. Gemessen
sind **97 page-faults auf 200 000 Kämpfe**: Der heiße Pfad allokiert praktisch nicht.
`Fight::new` kostet laut perf **4,2 % der Zyklen**, nicht mehr. Der Umbau ist richtig, aber er
gehört nicht zu den ersten fünf Maßnahmen.

#### B.1.3 Die beiden Talentzahlen gehören zu verschiedenen Räumen

Zu Recht bemängelt: Der Plan nannte `3,21·10¹⁶` und `19 456 → 83` nebeneinander, ohne die
Beziehung. Sie ist:

```
3,21·10¹⁶   gültige Punkteverteilungen mit genau 50 Punkten
            (alle 29 Talente, maxRanks und requires berücksichtigt)

19 456      (Stammvektor, calcination-release)-Kombinationen
            — NUR die Träger der einen faltenden Wirkgröße `damage`
    → 83    verschiedene damage-Werte,  Faktor 234
```

**Die 19 456 sind kein Teilraum der 3,21·10¹⁶, sondern eine Projektion:** Von den 18
Wirkgrößen wird jede außer `damage` von genau einem Talent bestimmt und ist damit im
Phänotyp eindeutig. Nur `damage` hat sieben Träger (sechs Stämme à 20 ‰ plus
`calcination-release` à 25 ‰), und nur dort können verschiedene Verteilungen kollidieren.

**Der Faktor 234 überträgt sich auf den vollen Talentraum, weil die eindeutigen Größen
nichts zur Faltung beitragen** — zwei Verteilungen, die sich in `resourceGain` unterscheiden,
sind im Phänotyp verschieden, egal wie ihre Stämme stehen. Formal: Der Phänotypraum ist das
Produkt aus (eindeutige Größen, unverändert) × (damage-Werte, gefaltet).

**Das ist eine Ableitung, keine Messung.** Wer sie prüfen will, zählt die Phänotypen des
vollen Raums — das ist mit derselben DP möglich, die schon die 3,21·10¹⁶ liefert, aber
niemand hat es getan.

#### B.1.4 Zwei Werkzeuge fehlen

```
llvm-profdata   /usr/bin/llvm-profdata      ✔ PGO möglich
llvm-bolt       FEHLT                       ✘ Phase 5 unvollständig
nvcc (CUDA)     FEHLT                       ✘ Phase 8 nicht startbar
nightly-Toolchain  0                        → std::simd nicht verfügbar
```

Der Vorschlag empfiehlt `std::arch::x86_64` statt `std::simd` — **das ist hier nicht nur die
bessere Wahl, sondern die einzige.**

#### B.1.5 Ein Fallstrick, den der Vorschlag nicht nennt

```
rustc lokal (5900HS)     1.93.1
rustc rustbuild          1.97.1
```

**PGO-Profile sind an die Compiler-Version gebunden.** Wer auf rustbuild profiliert und lokal
baut, bekommt keine Optimierung, sondern eine stille Fehlanpassung. Phase 5 braucht als
ersten Schritt eine Versionsangleichung — und die Projektregel „Rust-Projekte bauen über
`cargo remote -c`" kollidiert damit, weil die Zielmaschine der Laptop ist.

#### B.1.6 Wo gebaut und wo ausgeführt wird — die Regel und ihre zwei Fallen

**Festgelegt vom Auftraggeber am 2026-08-18:** rustbuild baut, dazu `clippy` und
`cargo fmt --check`. **Alles, was Code ausführt — Tests, Simulationen, Messungen — läuft auf
dem Laptop.** Der Simulator ist ein Binary; getestet wird, was kompiliert wurde.

Der Weg, der beides erfüllt, ohne die lokale cargo-Sperre zu berühren:

```
1. cargo remote -c -- test --no-run          baut die Testbinaries remote
2. ln -sfn <klon> /tmp/builds/<id>           vor JEDEM Lauf, nicht einmalig
3. cd sim && ../target/debug/deps/<name>     lokal starten, ohne cargo
```

**Mutationsproben sammeln ihre Fallnamen aus dem LAUF, nicht aus der Erwartung.** Am
2026-08-18 meldete eine Fertigmeldung „zwei brechende Prüffälle", gemessen waren es vier —
die Zahl war aus der Erwartung geschrieben statt aus der Ausgabe abgelesen. Im selben Zug
lief eine zweite Probe korrekt, aber nicht aus Einsicht: Dort sammelte ein Skript die roten
Fallnamen selbst ein. Der Satz des Workers dazu ist die Regel:

> „Der Schutz lag im Werkzeug, nicht im Kopf."

Wer den Unterschied nicht macht, verlässt sich beim nächsten Mal auf eine Einsicht, die er
nie hatte. Dasselbe gilt spiegelbildlich für den umgekehrten Fehler — aus einer richtigen
Messung eine falsche Regel abzuleiten: **Eine Messung belegt, _was_ schiefging, nicht _welche
Regel_ gelten soll.** Dazwischen liegt die Frage, ob es einen bekannten Weg gibt, den man
gerade nicht gegangen ist. Eine falsche Regel mit einer echten Messung darunter ist
gefährlicher als eine ohne, weil die Zahl das Nachdenken beendet.

**`--all-targets` ist beim Testen NICHT die gründlichere Variante, sondern ein anderer
Ausschnitt.** Am 2026-08-18 gemessen, drei Läufe gegen denselben Stand:

| Aufruf                                         | Umfang                | was fehlt               |
| ---------------------------------------------- | --------------------- | ----------------------- |
| `cargo test` (**der CI-Befehl**, `ci.yml:222`) | 164 Suiten, 903 Tests | —                       |
| `cargo test --all-targets`                     | 187 Ziele, 898 Tests  | **alle 5 Doc-Tests**    |
| lokal über die Binaries                        | 163 Ziele, 898 Tests  | ebenfalls die Doc-Tests |

`--all-targets` fährt die 24 Beispiele mit, die **null** Prüffälle tragen, und dafür keine
Doc-Tests. Mehr Ziele, weniger Tests. Wer nur die Zielzahl vergleicht, hält den größeren Lauf
für den vollständigeren.

**Beim `clippy` ist derselbe Schalter dagegen richtig** — ohne ihn bleiben die Testdateien
ungesehen. Derselbe Schalter, gegensätzliche Wirkung; wer ihn als „gründlicher" liest, nimmt
ihn überall und verliert beim Testen die Doc-Tests.

Doc-Tests sind über den lokalen Weg grundsätzlich nicht fahrbar: Sie sind kein Binary, sondern
entstehen erst beim Lauf über `rustdoc`. Sie laufen in der CI, und das genügt — ein zweiter
Lauf brächte dieselben fünf Fälle auf einer anderen Maschine und keine Abdeckung dazu. **Die
Form der Meldung lautet deshalb „N lokal ausgeführt, plus M Doc-Tests in der CI"** — zwei
Zahlen mit ihrem Ort, nicht eine Summe.

**Schritt 2 gehört in den Ablauf, nicht in die Umgebung.** `/tmp` ist eine tmpfs
(`findmnt -no FSTYPE,SIZE /tmp` → `tmpfs 10.9G`); der Verweis liegt im Arbeitsspeicher und ist
nach jedem Neustart weg. Wer sich auf „einmal gesetzt" verlässt, sieht dann vier rote Suiten,
ohne dass sich etwas geändert hat. `ln -sfn` ist idempotent und kostet nichts — die Bau-ID
selbst ist stabil, der Verweis auf sie ist es nicht.

**Die Begründung ist ausschließlich der Durchsatz, nicht die Rechengenauigkeit.** Am
2026-08-18 gemessen: `target-cpu` und `RUSTFLAGS` sind nirgends gesetzt,
`is_x86_feature_detected` und `target_feature` haben null Treffer in `sim/src`, und `objdump`
findet im fertigen Binary **null** AVX2/FMA-Befehle bei 6955 SSE-Befehlen als Kontrollzeile.
Es gibt keinen Vektorpfad — rustbuild und CI rechnen dieselben Zahlen wie der Laptop. Was
dort **nicht** gilt, sind Zeiten (121 689 gegen 210 094 Kämpfe/s, siehe Tabelle oben).

Zwei Fallen, beide gemessen, beide kosten sonst einen falschen Befund:

| Falle                       | Symptom                                            | Ursache                                                                                             |
| --------------------------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| falsches Arbeitsverzeichnis | „data/spells fehlt", 13 Fälle                      | `cargo test` setzt CWD auf `sim/`, eine eigene Schleife nicht. Tests lesen `../data/…`              |
| eingebrannte Baupfade       | „/tmp/builds/…/decay-factor.json fehlt"            | `env!("CARGO_MANIFEST_DIR")` und `env!("CARGO_BIN_EXE_*")` werden zur **Bauzeit** ersetzt           |
| eingefrorene Datenlage      | Prüffall meldet einen Boss, den es nicht mehr gibt | `build.rs` kompiliert `data/` ein. Das Binary kennt den Stand seines **Baus**, nicht den der Platte |

**Die dritte Falle ist die teuerste, weil ihre Fehlermeldung stimmt.** Am 2026-08-19 war
`parity_boss_state::ac1_beide_seiten_sehen_dieselbe_boss_menge` rot, weil eine untrackte Datei
`data/bosses/orch-maxload.json` im Klon lag. Die Datei beiseitegeschoben, erneut gemessen —
**immer noch rot, mit derselben Meldung und demselben Bossnamen**:

```
let vom_spiel     = json["allBossIds"]           <- Laufzeit, liest die Datei
let vom_simulator = BOSSES.iter().map(|b| b.id)  <- build.rs, EINKOMPILIERT
```

Erst ein `cargo remote -c -- test --no-run --test parity_boss_state` und ein erneuter Lauf
lieferten 8 von 8 grün. Die Diagnose war richtig und die Gegenprobe widerlegte sie trotzdem,
weil die Gegenprobe das alte Binary befragte.

Nichts daran sieht veraltet aus: Das Binary liegt da, ist ausführbar, läuft durch und nennt in
der Fehlermeldung sogar den richtigen Boss. Wer die Ursache im Code sucht, findet einen Code,
der in Ordnung ist. **Nach jeder Änderung an `data/` neu bauen, bevor gemessen wird — auch
nach dem Wegnehmen einer Datei, nicht nur nach dem Hinzufügen.** `build.rs` läuft beim Bau,
nicht beim Start.

Es ist dieselbe Klasse wie `env!("CARGO_MANIFEST_DIR")` eine Zeile darüber: zur Bauzeit
eingesetzt, zur Laufzeit unveränderlich. Wer das eine kennt, sollte das andere miterwarten.

Betroffen sind vier Suiten (`parity_decay_factor`, `parity_spender_talent`,
`rota_sim_beispiele`, `tote_symbole`). Die Bau-ID hängt am Quellverzeichnis, nicht am Bau —
über drei Läufe im selben Klon blieb sie identisch, ein anderer Worktree bekommt eine andere.
Der Verweis wird deshalb einmal je Arbeitsplatz gesetzt und bleibt gültig.

**Zwei Merkposten, bewusst ohne eigenes Issue** (der Auftraggeber hat neue Issues untersagt):

1. **Dauerhafte Lösung für die Baupfade:** die betroffenen Prüffälle von
   `env!("CARGO_MANIFEST_DIR")` auf einen zur Laufzeit bestimmten Pfad umstellen. Dann
   entfällt der Verweis. Es ist eine Änderung an fremden Prüffällen und gehört in das Issue,
   das diese Dateien ohnehin anfasst.
2. **Wächter gegen einen künftigen Vektorpfad:** Setzt später jemand `target-cpu=native` oder
   `RUSTFLAGS`, ist die ganze Frage zurück, ohne dass es auffällt. Ein Prüfschritt, der bricht,
   sobald AVX2-Befehle im Binary auftauchen, würde das fangen.

#### B.1.6a Eine Durchsatzmessung und ein Testlauf schließen einander aus

_Am 2026-08-19 dreimal aufgetreten, zweimal beinahe von mir verursacht._

Die Messumgebung aus #896 bricht ab, sobald die Rate während der Messung schwankt. Worker-2s
erster Versuch für #909 AC1 scheiterte deshalb in **allen acht Lastarten** — Ursache war
Worker-1s Gesamtlauf über alle Testbinaries auf derselben Maschine (`load 7,66` gegen `3,21`
vorher, PSI `cpu avg60 4,85` gegen `avg10 0,00`).

Das ist kein Fehler eines Workers, sondern eine **Belegung, die koordiniert werden muss**.

**Was auf dieser Maschine liegt und was nicht** — die Unterscheidung, die ich zuerst nicht
gemacht hatte:

| Schritt                            | Wo er läuft            | belastet die Messung |
| ---------------------------------- | ---------------------- | -------------------- |
| `cargo remote -c -- test --no-run` | rustbuild, CT 155      | nein                 |
| CI (`gh pr update-branch`, Läufe)  | Runner CT 150 in gonet | nein                 |
| **Ausführen der Testbinaries**     | **hier**               | **ja**               |

Ein Abnahmelauf über 200 Ziele mit acht Strängen ist damit dieselbe Belegung, über die sich
der blockierte Worker beschwert — nur vom Orchestrator. **Ich hätte das Fenster zugesagt und
im selben Zug selbst zugebaut.**

**Kernbindung hilft nicht, und der Grund ist gemessen.** Die SMT-Geschwister liegen paarweise
benachbart:

```
cpu0  thread_siblings_list   0-1
cpu8  thread_siblings_list   8-9      16 Threads, 8 physische Kerne
```

Ein `taskset -c 0-7` belegt also die physischen Kerne 0–3 vollständig, und 8–15 wären frei —
ich hatte das Gegenteil angenommen (je ein Thread von acht Kernen). Aber:

```
/sys/devices/system/cpu/cpu0/cache/index3/shared_cpu_list   0-15
```

**Der L3 ist über alle sechzehn Threads geteilt.** Getrennte physische Kerne heißen nicht
getrennte Speicherhierarchie.

Und Worker-2s Argument dazu ist das entscheidende: Die Messumgebung bricht bei **Schwankung**
ab, eine **gleichmäßige Verschiebung** durch L3-Druck erkennt sie nicht — und die sieht dann
wie ein Ergebnis aus. **Eine Störung, die als Messwert durchgeht, ist schlimmer als eine, die
den Lauf abbricht.**

**Verfahren:** Wer durchsatzmisst, bekommt ein Fenster. In diesem Fenster führt niemand
Binaries aus, der Orchestrator eingeschlossen. Bauen und CI laufen weiter, weil sie auf
anderen Rechnern liegen.

**Ein Fenster hat einen BERECHTIGTEN, nicht nur einen Zustand.** Die Formulierung ist
Worker-1s, und sie behebt einen echten Fehler: Der Orchestrator hat in einer Nachricht zwei
Dinge vermischt — die **Rückgabe seines eigenen Anspruchs** und die **Zuteilung an
Worker-2** — und daraus wurde „Worker-1: keine Einschränkung mehr", während Worker-2s Lauf
drei Sekunden vorher gestartet war.

```
"Frei für Worker-2"   ist etwas anderes als   "frei"
```

Worker-1 hat die Freigabe **sinngemäß statt wörtlich** gelesen, an `ps` nachgemessen (etimes 3)
und sich zurückgehalten. Sein Satz dazu ist der Grund, warum es zählt: _„Praktisch kostet mich
das nichts — ich sage es, weil die Freigabe beim nächsten Mal jemanden treffen könnte, der
etwas zu rechnen hat."_

**Und ein Fenster gilt erst als offen, wenn beide anderen quittiert haben UND `pgrep` leer
ist** — nicht, wenn die Anordnung raus ist. Am 2026-08-19 wurde einem messenden Worker der
ruhige Zustand **zugesagt**, ohne ihn zu messen; sieben Minuten Fremdlast lagen mitten in
seinem Lauf, weil die Anordnung 13 Minuten ungelesen im Postfach eines Dritten lag.

Dessen Folgerung gilt für alle: **`rota-ipc offen` vor jedem Gesamtlauf**, nicht nur vor der
Fertigmeldung — ein Gesamtlauf ist die Handlung, die andere stört.

#### B.1.6a-2 Die Streuung hing an der Abschnittsdauer, nicht an der Fremdlast

_Der Befund relativiert den Abschnitt darüber, und das ist der Grund, warum er direkt darunter
steht._

Nach zwei Stunden Fenster-Koordination — Anordnungen, Reihenfolgen, Prioritäten — hat Worker-2
die eigentliche Ursache gefunden, und sie hat mit Fremdlast **nichts** zu tun:

```
ROTA_MESSDAUER_S=60 (dreifache Abschnittsdauer), taskset -c 0-1, Basispunkt:
    ALLE ACHT Lastarten unter fünf Prozent Streuung (1,03 bis 3,39 %), kein ABBRUCH
    Boss mit HP-Phase   369 602/s bei 1,99 %      <- vorher 6,14 %
```

**Die Lastart „Boss mit HP-Phase" hat mit 2600 Stichproben die kürzesten Abschnitte von
allen achten.** Kurze Abschnitte mitteln die Schwankung nicht aus — deshalb streute
ausgerechnet sie, und deshalb half die dreifache Mittelungszeit.

Der zweite Hebel war ebenfalls nicht Ruhe, sondern **weniger eigene Belegung**:

```
taskset -c 0-7   (4 physische Kerne)   PSI 5,13 bis 11,05   alles verworfen
taskset -c 0-3   (2 physische Kerne)   PSI 5,01 bis  7,36   7 von 8 verworfen
taskset -c 0-1   (1 physischer Kern)   PSI 1,09 bis  2,11   alles durchgekommen
```

**Die Messung selbst erzeugte den Druck, an dem sie scheiterte.**

Das Fenster war nicht falsch — die ersten Läufe wurden von echter Fremdlast verworfen, das ist
gemessen. Aber es war nicht die Lösung, und es wurde für die Lösung gehalten.

**Nebenertrag:** 369 602/s bestätigt die Planbasis von 350 000–365 000 (knapp darüber). Sie ist
damit nachgemessen und nicht mehr nur zitiert — und genau diese Basis, nicht die 108 250 aus
einem Zwei-System-Lastprofil, ist der Bezugspunkt des Abnahmeziels 470 000 (Zeile 73).

**Und keine Dauer aus einem Schutzwert hochrechnen.** Aus `timeout 2400` habe ich auf acht mal
vierzig Minuten geschlossen und kam auf über fünf Stunden — dabei ist 2400 die Obergrenze, und
alle acht Lastarten laufen in **einem** Durchgang von fünf Minuten. Der tatsächliche Bedarf
lag bei 40 Minuten. Gefragt statt gerechnet hätte es sofort geklärt.

#### B.1.6b Der Simulator zählt bei umgekehrtem Ziel die Niederlagen als Siege

_Gefunden von Worker-3 beim Bau von #939, am Quelltext gemeldet und später am laufenden
Programm belegt. Vom Orchestrator mit eigenen Befehlen nachgemessen._

**Das Spiel unterscheidet, der Simulator nicht.** `combat.ts:4615` führt die Umkehrung als
Tabelle:

|            | Ring auf 0     | Zeit abgelaufen |
| ---------- | -------------- | --------------- |
| gewöhnlich | Sieg           | Niederlage      |
| `runaway`  | **Niederlage** | **Sieg**        |

```ts
const umgekehrt = this.boss.runaway !== null;
if (this.damageDealtValue >= this.boss.hp) {
  this.stateValue = umgekehrt ? FIGHT_OVERPRESSURE : FIGHT_WON;
```

`fight.rs:1430` und `:1569` setzen `killed_at` bei `self.damage >= self.boss.hp` — **ohne jede
Fallunterscheidung**. `BalanceResult.kills` zählt daraus.

**Gemessen, eigene Befehle, je 20 Durchgänge:**

```
boss-2-3   apl/e939-schutz.apl        kills:  0    killRate 0.0
boss-2-3   apl/e607-flashpoint.apl    kills: 20    killRate 1.0
```

Der Kandidat, der die Frist übersteht, meldet null Siege; der Kandidat, der Überdruck erzeugt,
meldet zwanzig. In **beide** Richtungen das Gegenteil.

> Der gemessene Teil ist `kills: 0` gegen `kills: 20`. Dass das die Umkehrung ist, folgt aus der
> Tabelle in `combat.ts` — ein Schluss, kein zweiter Messwert.

**Warum es niemand gemerkt hat, und das ist der eigentliche Befund:** Von 28 Bossen in den
Paritäts-Aufzeichnungen ist boss-2-3 dabei. Aber **keine einzige Aufzeichnung führt ein
Zustandsfeld** — weder `state` noch `FIGHT_WON`, `FIGHT_OVERPRESSURE`, `won` oder `outcome`.
Verglichen werden Schaden, Ticks und Zustands-Prüfsummen; der **Ausgang** wird nicht verglichen.

Die Paritätsprüfung deckt alles ab außer der Frage, wer gewonnen hat — und genau dort weichen
die Seiten ab. Dieselbe Klasse wie B.1.7: eine Prüfung, die grün meldet, weil sie die Stelle
nicht ansieht.

**Reichweite, gemessen:**

```
runaway aktiv        boss-2-3 (Spielboss), test-runaway (Testboss)
                     drei weitere führen das Feld auf null
.kills in sim/src    5 Verwendungen
Testdateien          3
```

Betroffen ist auch die Ausgabe von `rota-sim run` aus #897 — sie druckt die rohe Quote
unkommentiert.

#### B.1.6d Kein Näherungswert, wo Exaktheit erwartet wird — dreimal unabhängig gebaut

Am 2026-08-19 entstanden an drei Stellen unabhängig dieselbe Entscheidung, von drei
verschiedenen Rollen:

| Wo                                        | Statt eines Wertes | Quelle             |
| ----------------------------------------- | ------------------ | ------------------ |
| `erwarteter_schaden` bei `stop_on_kill`   | `None`             | #908, AC4          |
| ein Messblock mit PSI über der Schwelle   | `ABBRUCH`          | #896, Messumgebung |
| die gestaffelte Stichprobe bei Budgetende | `UNGEKLÄRT`        | #915, AC3          |

**Der gemeinsame Kern ist nicht Vorsicht, sondern Unterscheidbarkeit.** Worker-1s Formulierung:

> Einem Näherungswert sieht man nicht an, ob seine Voraussetzung galt. Eine Zahl, die manchmal
> exakt und manchmal ungefähr ist, ist schlimmer als keine — weil der Empfänger den Unterschied
> nicht mehr herstellen kann.

Ein `None` ist unbequem und **lesbar**. Ein Näherungswert ist bequem und sieht aus wie eine
Messung. Genau das ist der Grund, warum die Messumgebung ihren Wert hat: Dass sie eine
unbrauchbare Zahl **verwirft** statt sie zu liefern, ist keine Härte, sondern ihr Zweck.

Dieselbe Klasse wie B.1.6c, nur auf der Ausgabeseite: Dort wird ein Fehler neben einem Erfolg
als Erfolg gelesen — hier wird eine Näherung neben einer Messung als Messung gelesen.

**Warum solche Vorrichtungen im Betrieb immer wie Hindernisse aussehen.** Der Orchestrator hat
die Messumgebung zwei Stunden als Hindernis behandelt und Koordination um sie herum gebaut,
bevor er begriff, dass ihr Verweigern der Zweck ist. Worker-1s Erklärung:

> Eine Vorrichtung, die etwas **verweigert**, sieht im Betrieb immer wie ein Hindernis aus —
> der Nutzen ist unsichtbar, weil er in den Zahlen liegt, die man dadurch **nicht** bekommen
> hat.

Sein eigenes Beispiel dazu, dieselbe Sache im Kleinen: Die Trägerzusicherung „mit Krit muss
mehr Schaden entstehen" hat den `stop_on_kill`-Fall gefangen (#908 AC4) — und stand vorher zwei
Stunden als eine Zeile da, die nur Arbeit machte.

**Folgerung für die Bauform:** So etwas muss **laut scheitern statt still zu warnen.** Ein
Hindernis, das man umgehen kann, wird umgangen — und dann ist die Vorrichtung da, ohne zu
wirken. Das ist derselbe Grund, aus dem `caps.rs` seinen Wächter über eine Destrukturierung
ohne `..` baut (Bruch beim Übersetzen, nicht Warnung zur Laufzeit) und aus dem AC2 von #908
einen `match` ohne Auffangzweig verlangt.

#### B.1.6c Ein Fehler neben einem Erfolg wird als Erfolg gelesen

_Der Satz ist Worker-3s. Er beschreibt eine Fehlerklasse, die am 2026-08-19 in drei
verschiedenen Gestalten auftrat._

**Die drei Gestalten:**

| Wo                         | Was gemeldet wurde                             | Was daneben stand                     |
| -------------------------- | ---------------------------------------------- | ------------------------------------- |
| CI, `diagnose_im_kampf`    | `running 0 tests`                              | `test result: ok`                     |
| `rota-ipc senden`          | `bash: todesfelder_stimmen: command not found` | `Nachricht zugestellt`                |
| Prüffall „drei gegen eins" | nichts                                         | grün, obwohl der Name mehr verspricht |

In allen drei Fällen war die Information **vorhanden** und wurde von einer daneben
stehenden Erfolgsmeldung überstimmt.

**Der Messweg, der es versteckt** — gemessen, nicht vermutet:

```
... 2>&1 | tail -1     ->  ZUSTELLZEILE
                           (die Fehlermeldung ist WEG)

... 2>&1               ->  bash: nichtvorhanden: command not found
                           ZUSTELLZEILE

... | tail -1          ->  bash: nichtvorhanden: command not found
   (ohne 2>&1)             ZUSTELLZEILE
```

**Die Abhilfe ist die dritte Form:** `| tail -N` **ohne** `2>&1`. stdout wird gekürzt,
stderr geht unverändert durch. Kontrollzeile: Ohne Fehler bleibt die Ausgabe genauso kurz —
die Kürzung kostet nichts.

**Der konkrete Vorfall.** Eine Nachricht mit Backticks in doppelten Anführungszeichen: bash
führt den Inhalt als Kommandosubstitution aus und **ersetzt ihn durch seine Ausgabe** — bei
einem unbekannten Befehl also durch nichts. Die Nachricht kommt syntaktisch vollständig und
semantisch löcherig an, genau dort, wo die tragenden Bezeichner standen. Der Versand meldet
Erfolg.

Dass die Fehlermeldung überhaupt sichtbar war, lag daran, dass sie aus dem Heredoc-Kontext
direkt aufs Terminal ging statt durch die Pipe. Mit dem sonst benutzten Muster wäre nur
„zugestellt" zu lesen gewesen.

**Zwei Regeln daraus:** Keine Backticks in einer Zeile, die durch bash läuft — Heredoc mit
gequotetem Begrenzer. Und kein `2>&1` vor einem `tail`.

**Die zweite Hälfte, und sie ist schlimmer** — gefunden von Worker-3, nachgemessen:

```
nichtvorhandenerbefehl123 | tail -1     ->  $? = 0                FALSCH
nichtvorhandenerbefehl123               ->  $? = 127              richtig
nichtvorhandenerbefehl123 | tail -1     ->  PIPESTATUS[0] = 127   richtig

if nichtvorhandenerbefehl123 | tail -1; then …      ->  hält es für ERFOLG
( set -o pipefail; nichtvorhandenerbefehl123 | tail -1 )  ->  $? = 127
```

Der Exit-Code hinter einer Pipe ist der des **letzten** Gliedes, und `tail` gelingt fast
immer. Die Abhilfe oben macht den Fehler damit **sichtbar, aber nicht prüfbar**.

Worker-3s Einordnung: dieselbe Klasse wie die verschluckte Meldung, nur eine Stufe tiefer
**und schlimmer — weil eine Maschine sie liest statt eines Menschen.** Ein Mensch sieht die
Zeile daneben; ein `&& melde_erfolg` sieht sie nicht.

**Drei Regeln, in dieser Reihenfolge:**

1. Wo ein Ergebnis von einem Exit-Code abhängt: **keine Pipe dazwischen.** In eine Datei
   umleiten und den Code direkt lesen.
2. Wo eine Pipe sein muss und das rechte Glied **alles liest** (`tail`, `grep`, `awk`, `wc`):
   `set -o pipefail`.
3. **Bei `head`: nicht `pipefail`, sondern `${PIPESTATUS[0]}`.**

**Warum Regel 3 eine eigene Regel ist und keine Alternative** — Worker-3 hat es als Verdacht
aus Vorwissen gemeldet und ausdrücklich als _nicht gemessen_ gekennzeichnet, weil er im
Messfenster nichts ausführen durfte. Nachgemessen:

```
( set -o pipefail; seq 1 200000 | head -3 )   ->  $? = 141      SIGPIPE
(                  seq 1 200000 | head -3 )   ->  $? = 0
( set -o pipefail; seq 1 200000 | tail -3 )   ->  $? = 0        tail liest alles
( set -o pipefail; echo eins    | head -3 )   ->  $? = 0        head liest alles
```

`head` schließt die Pipe, sobald es genug hat; der Schreiber links bekommt SIGPIPE und endet
mit 141. **Mit `pipefail` wird daraus ein Fehlschlag der ganzen Kette, obwohl nichts
schiefging** — und zwar genau dann, wenn die Ausgabe lang genug ist. Ein `pipefail` als
Pauschalregel hätte also den _umgekehrten_ Fehler eingebaut: Fehlalarm statt verschluckter
Meldung.

Die beiden Kontrollzeilen sind der Grund, warum die Regel so und nicht allgemeiner steht: Bei
`tail` und bei kurzer Ausgabe bleibt `pipefail` folgenlos. Der Unterschied liegt nicht am
Werkzeug, sondern daran, **ob das rechte Glied die Pipe vorzeitig schließt**.

> Der Vorgang ist selbst ein Beispiel für den Abschnitt: Ein Vorbehalt, der als _Verdacht_
> gemeldet und als solcher gekennzeichnet wurde, hat eine falsche Regel verhindert, die schon
> im Plan stand.

**Die dritte Gestalt: ein aggregierender Filter macht aus der Störung einen Messwert.**
Gefunden von Worker-1, nachgemessen:

```
… 2>&1 | awk '{print $2}' | sort | uniq -c
      1 line          <- aus der bash-Meldung, sieht wie ein Check aus
      2 pass

…      | awk '{print $2}' | sort | uniq -c      (ohne 2>&1)
bash: nichtvorhanden999: command not found
      2 pass          <- Zählung korrekt, Meldung sichtbar

… 2>&1 | grep -c pass   ->  1                   (lautlos verschluckt)
```

Die Rangfolge der Gefährlichkeit ist nicht die, die man erwartet:

| Filter    | Was passiert | Was man sieht             |
| --------- | ------------ | ------------------------- |
| `tail`    | verschluckt  | nichts                    |
| `grep`    | verschluckt  | nichts                    |
| `uniq -c` | **addiert**  | eine Zahl, die falsch ist |

Worker-1s Satz: _„Bei `uniq -c` wird der Fehler sogar zum Erfolg addiert."_ Das ist
schlimmer als Verschlucken, weil eine falsche Zahl nach einer Messung aussieht.

**4. Regel:** Ein aggregierender Filter (`uniq -c`, `wc -l`, `awk` mit Summe) darf **nie**
stderr mitlesen.

**Präzisierung von Worker-3, und sie ist nötig:** _„Nie stderr mitlesen" heißt nicht „stderr
wegwerfen."_ Wer es weglässt, verliert die Panics und damit den Grund, warum ein Ziel rot war.
Die Regel trägt als **Trennen statt Zusammenführen** — zwei Ströme, zwei Zwecke: der eine wird
gezählt, der andere gelesen.

**Und die Zuordnung eines Werkzeugs zu Regel 2 oder 3 wird nicht abgeleitet, sondern
gemessen.** Worker-3 hatte vermutet, `grep -q` verhalte sich wie `head`, weil es beim ersten
Treffer logisch fertig ist. Gemessen — und widerlegt:

```
seq 1 200000 | grep -q     ->  0
seq 1 200000 | grep -m1    ->  0
seq 1 200000 | head -3     ->  141      <- die Kontrollzeile
1 288 895 Bytes            ->  weit mehr als ein Pipe-Puffer
```

Ohne die dritte Zeile hieße die Null nur, dass der Aufbau kein SIGPIPE erzeugt. Mit ihr heißt
sie: **der Aufbau kann es, `grep -q` tut es nicht.**

Sein Satz dazu ist die eigentliche Regel: _„Früh fertig und die Pipe so schließen, dass SIGPIPE
entsteht, sind zwei verschiedene Dinge."_ Man sieht es einem Werkzeug weder am Namen noch an
seiner Logik an — **die Eigenschaft ist gemessen oder unbekannt.** Zwei Zeilen, mit `head` als
Kontrollzeile daneben.

**5. Regel:** Ein `echo`, das einen Erfolg behauptet, ist kein Beleg. Am 2026-08-19 scheiterte
ein `gh issue close` am Backtick-Problem, und direkt darunter stand `#908 geschlossen` — ein
`echo`, das trotzdem lief. Der Fehler war **schlimmer als die verschluckte Meldung**, weil eine
verschluckte Meldung immerhin ein Zeuge war und ein `echo` keiner ist.

```
1. Nach jedem Zustandswechsel  ->  eine ABFRAGE des Zustands (nicht der Exit-Code)
2. Längere Texte an gh         ->  --body-file, nie --body/--comment
3. "geschlossen"/"gemergt"     ->  nur mit der Abfrage daneben, die es belegt
```

**Die Reihenfolge ist Worker-1s, und sie ist präziser als die ursprüngliche.** Er hat einen
eigenen Fall derselben Klasse gemeldet:

```
git push -u origin … 2>&1 | tail -5
echo "PUSH EXIT: $?"          <- der Exit von tail, nicht von push
```

**Aufgefallen ist es ihm nicht.** Getragen hat eine Zustandsabfrage danach (`git rev-parse HEAD`
gegen `origin/zweig`, zweimal derselbe Hash) — und die war zufällig da, nicht geplant.

Seine Folgerung: _„Nicht der Exit-Code ist die Absicherung, sondern die Abfrage des Zustands
danach. Ein Exit-Code kann durch eine Pipe verlorengehen; ein abgefragter Zustand nicht."_

Punkt 2 beseitigt die Ursache, Punkt 3 die falsche Meldung — **aber nur Punkt 1 fängt auch das,
woran niemand gedacht hat.** Deshalb steht er zuerst.

Nebenbei: `gh issue close` kennt kein `--comment-file`. Der Weg ist `gh issue comment
--body-file` plus `gh issue close --reason completed` — zwei Aufrufe, zwei Exit-Codes.

**Und eine Zahl ohne Fehlerbalken liest sich als exakt.** Worker-3 hat einen Störzeitraum als
`09:26:14 bis 09:31:35` gemeldet — das waren die **mtimes**, also Endzeiten. Seine Last begann
früher, wie viel früher war nicht mehr feststellbar. Der Leser hielt die Untergrenze für
scharf, weil nichts daneben stand, das sie als offen kennzeichnet. Richtig wäre gewesen: _„ab
spätestens 09:26:14, Beginn unbekannt."_

> Eine Dateizeit belegt, wann etwas **aufhörte**, nie wann es anfing. Wer einen Störzeitraum
> meldet, schreibt den Beginn mit, **während** er läuft — hinterher ist er nicht mehr da.

**6. Regel:** Ein langer Lauf gehört nicht in den Hintergrund eines Werkzeugs, sondern
abgekoppelt (`nohup`).

Am 2026-08-19 endete ein 40-Minuten-Messlauf um 10:38:21 **mitten im Satz** — letzte Zeile
„nach 47.0 s — Abschnitt 4/4", danach nichts. Kein Timeout (3000 s gesetzt, drei Minuten
gelaufen), kein Fehler in der Ausgabe. Er lief als Hintergrund-Task des eigenen Werkzeugs und
wurde von diesem eingesammelt; der Task-Status lautete wörtlich `killed`.

**Die Gegenprobe ist saubererer als jede Ausschlussliste:** Der Neustart per `nohup` lief durch
dieselbe Lastart, dieselbe Zeile im Ablauf — und weiter. Gleiche Bedingungen, anderer Ausgang,
**der Unterschied ist allein die Abkopplung.**

Vorher waren drei Verdächtige im Umlauf (fremde Testläufe, ein Backtick-Unfall des
Orchestrators, der OOM-Killer). Keiner war es, und alle drei mussten einzeln ausgeschlossen
werden — der OOM über das Kernel-Log, die Fremdlast über gemeldete Zeitstempel, der
Backtick-Unfall über die Feststellung, dass `command not found` keinen Prozess beendet.

**Und der Griff, der es sofort gezeigt hätte** — Worker-3s Zusicherung aus seinem Prüfskript,
eine Ebene höher angewandt:

> Nicht „wie viele haben gemeldet", sondern **„hat jeder gemeldet, der gestartet ist"**.

Der Lauf hatte drei von acht Lastarten gemeldet und keine Abschlusszeile. Ohne den Blick in die
Datei sah die Auswertung aus wie ein Lauf, den es nie gab.

> **Nachtrag zur Beweislage:** Der Orchestrator hatte genau diese Ursache als Hypothese
> hingeschrieben und als _ungemessen_ gekennzeichnet — obwohl sie in zwei Minuten an einem
> eigenen Hintergrundjob prüfbar gewesen wäre.

**Wann eine Kennzeichnung trägt und wann sie eine Ersatzhandlung ist.** Der erste Anlauf dieses
Nachtrags lautete „eine richtige Vermutung, die man hätte messen können, ist Glück mit einer
Kennzeichnung davor" — und griff zu weit. Worker-3 hat ihn präzisiert, weil er seinen eigenen,
korrekt gekennzeichneten Verdacht mitgetroffen hätte:

> Entscheidend ist nicht, ob eine Vermutung **ungemessen war**, sondern ob sie ungemessen
> **bleiben musste**.

Das Kriterium ist eine Frage: **Kann ich die Messung jetzt fahren, ohne eine Regel zu brechen?**
Und die Antwort hat **drei** Ausgänge, nicht zwei — den dritten hat Worker-3 aus #939
nachgetragen:

| Lage                    | Was die Kennzeichnung dann ist                                      |
| ----------------------- | ------------------------------------------------------------------- |
| **messbar jetzt**       | Ersatzhandlung — dann messen statt kennzeichnen                     |
| **messbar später**      | das Richtige — Messung nachliefern, sobald es geht                  |
| **heute nicht messbar** | **das Ergebnis** — und sie gehört in den Code, nicht in die Meldung |

Alle drei traten am 2026-08-19 auf:

**Messbar jetzt.** Der Hintergrundjob-Verdacht war in zwei Minuten an einem eigenen Job prüfbar
und wurde nur gekennzeichnet.

**Messbar später.** Der `grep -q`-Verdacht — ein Messfenster lief, und der Melder hatte nach
einem vorherigen Verstoß bewusst aufgehört, selbst über Last zu entscheiden. Er hat gemeldet,
gekennzeichnet, nach der Freigabe gemessen und lag falsch. Die Kennzeichnung war dort nicht
Glück mit einem Etikett, sondern der einzige verfügbare Weg.

**Heute nicht messbar.** Stufe 3 des Zielvertrags (Mechanikqualität, #939): Kein Mechanikwert
verlässt den Kampf, und der Kampfkern war im Issue ausdrücklich ausgenommen. Dort half weder
messen noch warten.

Die richtige Antwort im dritten Fall waren drei Schritte: die **Messvorschrift schreiben**, den
Zustand ausdrücklich als **ungemessen führen**, und dafür sorgen, dass er **nichts still
entscheidet**. Worker-3s Regel dazu — „nicht gemessen ist nicht erfüllt" — macht einen
Kandidaten ohne Messwert **unzulässig**, nicht zulässig.

> **Und deshalb gehört die Kennzeichnung dort in den Code:** Eine Meldung verfällt, ein
> `Unmeasured`-Zustand bleibt. Das ist die Verbindung zu B.1.6d — ein Zustand, dem man ansieht,
> dass er ungemessen ist, ist genau das Gegenteil eines Näherungswertes, dem man es nicht
> ansieht.

**Warum die Regel nötig ist, obwohl es nicht schiefgegangen ist.** Der Orchestrator hat dieses
Muster am 2026-08-19 etwa zehnmal für CI-Zusammenfassungen benutzt
(`gh pr checks NNNN 2>&1 | awk … | uniq -c`). Ein zusätzlicher Eintrag hätte wie ein zwölfter
Check ausgesehen. Aufgefallen wäre er — **durch Vorwissen**, weil rota elf Checks hat, nicht
durch den Messweg. Bei einem Repository mit unbekannter Checkzahl wäre die Zahl gelesen und
geglaubt worden.

Das ist weder „richtig gemacht" noch „ein Werkzeug hat mich bewahrt", sondern eine Zahl, die
zufällig im Kopf war.

**Und der Fall ist real aufgetreten**, bei Worker-1 um 07:26:

```
./target/debug/examples/parity 2>&1 | tail -8
/bin/bash: ./target/debug/examples/parity: No such file or directory
EXIT: 0
```

Das Binary existierte nicht — `cargo test --no-run` baut Beispiele nur als Testziele. Er hat
die Null als Exit von `tail` erkannt, **weil die Fehlermeldung darüber stand**. Mit `grep`
statt `tail` oder zwei Zeilen weniger hätte er einen bestandenen Paritätslauf gemeldet, den es
nicht gab.

#### B.1.7 Was die CI prüft — und was sie bis zum 2026-08-19 nicht geprüft hat

Zwei Lücken, gefunden von Worker-1 und beim Beheben der ersten die zweite. Beide sind
geschlossen (PR #995), beide gehören hierher, weil sie dieselbe Frage stellen: **Woran merkt
man, dass eine Prüfung nichts angesehen hat?**

**Lücke 1 — 25 Prüffälle liefen nie.** Der Schritt lautete `cargo test`, ohne Schalter:

| Ursache          | Datei                               | Fälle |
| ---------------- | ----------------------------------- | ----- |
| Merkmalsschalter | `sim/tests/diagnose_im_kampf.rs`    | 8     |
| fehlende Ziele   | `sim/examples/messumgebung.rs`      | 14    |
|                  | `sim/examples/trace-stabilitaet.rs` | 3     |

Die erste Ursache ist die gefährliche. `diagnose_im_kampf.rs` trägt
`#![cfg(feature = "diagnostics")]` und übersetzt ohne das Merkmal zu einer **leeren Suite**.
Gemessen im selben CI-Lauf, beide Übersetzungen nebeneinander:

```
diagnose_im_kampf-6bc1fa8221b8aad3   running 0 tests   test result: ok. 0 passed
diagnose_im_kampf-9dc5e78eea01f715   running 8 tests
```

Nicht übersprungen, nicht ignoriert — **grün**. Eine Suite, die „ok" meldet und null Fälle
angesehen hat, ist von einer bestandenen Suite in keiner Zusammenfassung zu unterscheiden.

Die zweite Ursache ist harmloser, aber leicht zu verwechseln: `cargo test` **baut** Beispiele,
führt ihre `#[test]` aber nicht aus. Beides sieht im Log gleich aus.

Der Schritt heißt jetzt:

```yaml
- run: cargo test # bleibt unberührt
- run: cargo test --all-targets --features diagnostics # kommt dazu
```

Der erste Lauf bleibt, weil er eine eigene Zusicherung trägt: Die Bibliothek übersetzt **ohne**
das Merkmal und ist grün. Zwei Läufe, zwei verschiedene Aussagen. Kosten: der `sim`-Job
braucht 6 min 48 s statt vorher gut vier.

**Lücke 2 — die Änderung konnte sich selbst nicht prüfen.** Beim ersten Lauf von PR #995:

```
sim    skipping    0
```

Das Muster des Pfadfilters kannte alle **Eingaben** der `sim`-Stufe
(`sim/`, `data/`, `tests/parity/recordings/`, `Cargo.toml`, `Cargo.lock`) — aber nicht die
Datei, in der die Stufe **definiert** ist. Wer den Testbefehl ändert, löst die Stufe nicht
aus, und `ci-pass` meldet grün.

Es ist dieselbe Klasse wie #134 (Protokoll-Nummernraum), #138 (Spielseite), #159
(`FACTOR_NAMES`) und #173 (die Vergleichsdateien) — nur eine Ebene höher: Dort fehlten
**Eingaben** der Prüfung, hier fehlte ihre **Definition**. `.github/workflows/ci.yml` steht
jetzt im Muster.

**Der Wächter dazu ist geprüft, nicht angenommen.** `tests/unit/ci-sim-filter.test.ts` leitet
das Muster aus dem Workflow ab und die Pfade aus den `include_str!`-Zeilen — er zählt nichts
ab. Gegenprobe mit entferntem Eintrag:

```
Muster vollständig            3 bestanden
tests/parity/recordings/ raus 2 durchgefallen, 1 bestanden
zurückgebaut                  3 bestanden
```

Er schlägt an **und** schweigt wieder. Ein Wächter, von dem nur das eine belegt ist, ist kein
Wächter.

**Summenprobe, lokal gegen CI.** Beide Zahlenreihen sind unabhängig entstanden:

```
lokal   cargo test                                       169 Ziele,  950 Fälle
lokal   cargo test --all-targets --features diagnostics  199 Ziele,  975 Fälle
                                                                    ---------
                                                                     1925
CI      beide Läufe zusammen                             369 Suiten, 1930 Fälle
                                                                    ---------
Rest    5  ->  Doc-Tests, die über den lokalen Weg nicht fahrbar sind (B.1.6)
```

Der Rest von 5 ist erklärt, nicht übrig. Eine Kette ohne erklärten Rest ist keine Kette.

---

### B.2 Zweites Urteil — gemessen statt übernommen

_Nachgetragen 2026-08-14. Das Urteil nennt zwölf fehlende Verfahren, ordnet sie in fünf Stufen
und erklärt eines davon zur wichtigsten Maßnahme überhaupt. Ich habe die vier Behauptungen
gemessen, die messbar sind._

**Vorweg, weil es die Reihenfolge dieses Abschnitts erklärt:** Das Urteil hat in meinem Plan
**zwei echte Fehler** gefunden (Anhang B.2.5), und acht seiner Punkte sind richtig und werden
übernommen (Anhang B.2.6). Dass die drei gemessenen Punkte durchfallen, macht das Urteil nicht
wertlos — es macht nur den Unterschied zwischen einer plausiblen Rechnung und einer Messung
sichtbar.

#### B.2.1 Der Multi-Policy Decision DAG — die Zahl, um die es geht

**Die Behauptung:** In einer Suchrunde werden „beispielsweise 558 Rotationen gleichzeitig
bewertet"; statt 558 getrennter Kämpfe solle ein gemeinsamer Entscheidungsgraph nur die
einzigartigen Zustandsübergänge rechnen. Erwartung des Urteils: _„selbst wenn nur drei oder
vier Kandidaten im Mittel dieselben Transitionen teilen, wäre das bereits mehr als der
erwartete Gewinn vieler Low-Level-Optimierungen zusammen."_

**Die 558 stimmt.** `rotation_search.rs:576` ist genau diese Schleife:

```rust
for (pos, &kandidat) in offen.iter().enumerate() {   // bis 93 Zeilen
    for stelle in 0..=chosen.len() {                 // jede Einfügestelle
        let gemessen = measure(config, "versuch", &versuch);   // VOLLER KAMPF
```

Bei einer Basisliste der Länge 5 sind das **528 Vollkämpfe in einer Runde** — dieselbe
Größenordnung, alle im selben Startzustand, mit demselben Build, demselben Boss, demselben
Startwert. Der Fall ist also genau der, für den der DAG gebaut wäre.

**Gemessen** (`sim/examples/orch-policy-dag.rs`, boss-1-1, 60 s, Startwert 5): Präfixbaum über
die echten Cast-Folgen aus dem Kampfprotokoll. Zwei Rotationen mit identischem Präfix haben
danach denselben Zustand — ein DAG müsste dieses Präfix einmal rechnen.

| Größe                   | Basislänge 2 | Basislänge 5 |
| ----------------------- | -----------: | -----------: |
| Kandidaten in der Runde |          273 |          528 |
| Entscheidungen zusammen |        8 385 |       21 074 |
| einzigartige Zustände   |        3 789 |       11 041 |
| **Teilungsfaktor**      |    **2,21×** |    **1,91×** |

Erster Befund: **1,91×, nicht 3–4×.** Das allein wäre kein Ausschlusskriterium. Der zweite
Befund ist es:

```
verhaltensgleiche Klassen        298 von 528 Kandidaten
größte Klasse                    216
davon Folge = unveränderte Basis 216      ← alle 216
```

**216 von 528 Kandidaten (40,9 %) erzeugen die unveränderte Cast-Folge der Basisrotation.**
Die eingefügte Zeile greift kein einziges Mal. Bei Basislänge 2 sind es 141 von 273 (51,6 %).

Damit zerfällt der Faktor 1,91 in zwei sehr ungleiche Teile:

```
1,77×   Deduplizierung        billig: ein Ergebnis-Cache, oder die Prüfung
                              "greift diese Zeile je?" — beides steht schon
                              in E11.4 und E11.4b

1,07×   echtes Präfix-Teilen  teuer: der Multi-Policy-DAG
```

**Gegengeprüft in der günstigsten Lesart:** Mein Trie-Schlüssel ist `(Zeit, Zauber)` — wer nur
die Aktionsfolge ohne Zeitstempel betrachtet, misst mehr Teilung. Als Zustandsmaß ist das
falsch (verschiedene Zeit heißt verschiedener Zustand), aber es ist die Obergrenze dessen, was
der Vorschlag überhaupt versprechen kann. Ergebnis: **1,08× statt 1,07×.** Der Befund hält.

**Zwei Kontrollzeilen liefen mit**, weil sonst nicht auffällt, wenn der Messweg die Antwort
erzeugt: 558 identische Folgen ergeben Faktor 558,0 (exakt), und 558 Folgen aus 251
verschiedenen Mustern ergeben 2,22 — das ist 558/251 auf zwei Stellen. Der Zähler zählt richtig.

**Was daraus folgt — und was nicht.** Der DAG ist nicht falsch, er ist **die teure Hälfte einer
Maßnahme, deren billige Hälfte schon im Plan steht**. Und beide holen dieselbe Ersparnis:

> **Und „jetzt nicht bauen" heißt nicht „für immer streichen".** Die Messung gilt für
> Greedy-Einfügung am heutigen Bestand. **Wiedervorlage, wenn beides zutrifft:**
>
> ```
> Anteil einzigartiger Übergänge < 0,70          (heute: 0,93 — deshalb 1,07x)
> UND erwarteter Zusatzgewinn    > 1,20x
> ```
>
> Beide Zahlen misst die Diagnose aus E15.3 ohnehin mit. Damit entscheidet der Messwert über
> die Wiedervorlage, nicht die Erinnerung an dieses Kapitel.

> **E11.4 und ein Multi-Policy-DAG sind nicht additiv.** Wer die 216 unveränderten Ergebnisse
> abfängt, hat 95 % dessen abgeräumt, was der Graph teilen würde.

**Was der Plan daraus übernimmt**, ist deshalb nicht der Graph, sondern die Zahl: In E11.4
stand „keiner vorhanden? → Ergebnis unverändert übernehmen" als Randfall. **Es ist der
Hauptfall — 41 bis 52 % jeder Suchrunde.** Das gehört in das Abnahmekriterium.

**Endausbau-Vorbehalt:** 1,91× und 1,77× sind _Mengen_, keine Strukturen — sie gelten für
heutige Bossdichte und Poolgröße.

> **NACHGEMESSEN am 2026-08-14, und eine Vermutung ist dabei gefallen.** Hier stand: _„Mehr
> gleichzeitige Mechaniken heißt mehr Ereignisse, an denen zwei Rotationen auseinanderlaufen,
> also **weniger** Teilung für den DAG."_ **Die Richtung ist falsch:**
>
> ```
> Boss        Systeme  Teilung  Äquivalenz  Rest-DAG
> boss-1-1          2    1,91x       1,77x     1,07x
> boss-1-2          2    2,22x       1,81x     1,10x
> boss-2-2          3    2,74x       2,18x     1,20x   ← mechanikreichster, HÖCHSTE Teilung
> boss-3-4          3    2,22x       1,90x     1,17x
> ```
>
> Mehr Mechanik gab **mehr** Teilung, nicht weniger. Plausibel im Nachhinein: Bossereignisse
> treffen alle Rotationen gleich und ziehen ihre Verläufe zusammen, statt sie zu trennen.
>
> **Der Befund selbst hält an allen vier Bossen** — der DAG-Rest bleibt bei 1,07–1,20×, und der
> Abstand zur billigen Deduplizierung bleibt groß. Was fällt, ist meine Begründung für die
> Endausbau-Richtung, nicht das Ergebnis.

**Der Macro-Transition-Cache (Punkt 2 des Urteils) misst dieselbe Größe.** „Einzigartige
Zustandsübergänge" ist genau die Trie-Knotenzahl: 11 041 bei 21 074 Entscheidungen. Er kann
per Konstruktion nicht mehr holen als der DAG, dem er zuarbeitet.

> **Was diese Messung NICHT abdeckt.** Sie lief inzwischen an **vier** Bossen (siehe Kasten
> oben), aber weiterhin mit **einem** Startwert und nur für den Einfüge-Operator von
> `greedy_opts`. Nicht gemessen sind: die LNS-Zerstöre-und-repariere-Runde, Tausch- und
> Entferne-Operatoren, und der Fall, dass zwei Verläufe nach einer Divergenz **wieder
> zusammenlaufen** — den findet ein Präfixbaum grundsätzlich nicht.
>
> **Der letzte Punkt ist der einzige, der zugunsten des DAG wirken könnte**, und er ist die
> ehrliche Restunsicherheit dieses Befunds. Er wäre mit einem Zustands-Hash je Entscheidung
> messbar statt mit einem Präfix — das ist der Weg, falls die Frage je wieder aufkommt. Der
> Abstand zwischen 1,77× (billig) und 1,07× (teuer) müsste sich dafür allerdings **umkehren**,
> und das ist bei 40,9 % unveränderter Ergebnisse unwahrscheinlich.

#### B.2.2 Breakpoints bei 10-ms-Auflösung — richtig gerechnet, falsch verallgemeinert

**Die Behauptung:** Weil die Zeitauflösung 10 ms beträgt, erzeugen Tausende `catalysis`-Werte
nur wenige verschiedene Cooldown-Werte. Der äußere Optimierer solle deshalb nicht in Rohwerten
suchen, sondern in Timing-Zellen — „ein Bereich von Tausenden Zahlen auf einige Dutzend".

**Die Voraussetzung stimmt.** `time.rs:23` führt `TICK_MS = 10`, und `haste.rs:96` rundet auf:

```rust
let scaled = base_ms * 1000 / (1000 + permille);
snap_up(scaled)                       // auf das nächste Vielfache von 10
```

**Für eine einzelne Zeit trägt die Rechnung auch.** Über `catalysis` 0…3000 gemessen
(Kontrollzeile: `hasted(8000, 200)` muss 6670 sein — trifft):

| Basiszeit | verschiedene Werte | von 3001 | Faktor |
| --------: | -----------------: | -------: | -----: |
|  1 500 ms |                113 |    3 001 |  26,6× |
|  3 000 ms |                226 |    3 001 |  13,3× |
|  8 000 ms |                601 |    3 001 |   5,0× |
| 30 000 ms |              1 716 |    3 001 |   1,7× |

**Und hier bricht die Verallgemeinerung.** Ein Build nutzt nicht eine Zeit, sondern zwölf bis
sechzehn. Die Zellen des Builds sind die Schnittmenge aller Einzelzellen — und die ist fast
leer. Gemessen über die **20 realen Basiszeiten der Zauberdaten**, gemittelt über 20 zufällige
Builds mit je 14 Zaubern:

```
catalysis 0..300     301 Rohwerte  ->  301 Zellen     Faktor 1,00
catalysis 0..1000   1001 Rohwerte  -> 1001 Zellen     Faktor 1,00
catalysis 0..3000   3001 Rohwerte  -> 2848 Zellen     Faktor 1,05
```

**Faktor 1,00.** Über den ganzen realistischen Bereich gibt es **keinen einzigen**
`catalysis`-Schritt, bei dem sich nicht mindestens eine Zauberzeit ändert. Der Grund ist
arithmetisch, nicht zufällig: Die Breakpoints einer Basiszeit `b` liegen bei den `permille`,
an denen `b·1000/(1000+p)` eine Zehnerschwelle überschreitet — bei zwanzig verschiedenen `b`
liegen sie fast überall verschieden, und zwei Zeiten reichen schon, um die meisten
gemeinsamen Intervalle zu zerschneiden.

**Was bleibt.** Für einen Parameter, der **genau einen** Zauber betrifft — dessen `cooldownMs`,
`castMs`, `channelMs` —, ist die Zellenrechnung richtig und billig. Für einen **globalen
Wert**, der alle Zeiten gleichzeitig skaliert, ist sie wertlos. Das ist genau die
Unterscheidung, die das Urteil nicht trifft: Es rechnet `catalysis` als ob es ein einzelner
Cooldown wäre.

**Endausbau-Vorbehalt:** Er verschärft den Befund. 60 Zauber statt 58 und 16 Leistenplätze
statt 12 heißt **mehr** verschiedene Basiszeiten je Build, also noch feiner zerschnittene
Intervalle. Der Faktor 1,00 kann im Endausbau nicht steigen.

#### B.2.3 Die Treppenfunktion beim Schaden — bestätigt, und sie bleibt

**Die Behauptung:** Bei festem Trace ist der Schaden
`D(x) = C + Σ nᵢ·round(aᵢx + bᵢ)` — eine monotone ganzzahlige Treppenfunktion, die man
invertieren kann, statt Werte einzeln zu simulieren.

**Die Voraussetzung stimmt und ist im Code zugesichert.** `factors.rs:146`:

```rust
value.round() as u32        // wie Math.round in JavaScript — die Parität hängt daran
```

Der Schaden ist eine Summe gerundeter Produkte. Solange der Trace steht, ist die Form richtig,
und die Umkehrung („welcher Grundschaden macht Zauber X hier konkurrenzfähig?") ist eine
ganzzahlige Binärsuche über 10–20 Auswertungen statt hunderter Kämpfe.

**Der Unterschied zu Anhang B.2.2 ist wichtig:** `baseDamage` eines Zaubers beeinflusst **keine
Zeit** — es kann den Kontrollfluss nur über HP-Schwellen und schadensabhängige Bedingungen
kippen. Genau das prüft die `dependency_mask` aus E11.4d. **Hier trägt der Ansatz, und er
gehört nach E12.** Das ist die einzige der drei gemessenen Behauptungen, die vollständig hält.

#### B.2.4 Partial-Order Reduction über Bedingungen — im Bestand nicht zu haben

**Die Behauptung:** Zwei Regeln, die nie gleichzeitig gültig sein können, kommutieren; eine
`93 × 93`-Bitmatrix aus Bedingungsanalyse würde große Teile des Anordnungsraums streichen.

**Gemessen**, strukturell und konservativ:

```
Paare gesamt                    4 278
Paare mit demselben Zauber         45
davon eine Anordnung tot           35   = 0,82 % aller Paare
```

Die 35 sind die bereits bekannte Same-Spell-Subsumption (Faktor 1,27, § 1.2b). **Darüber
hinaus findet eine Betrachtung der Bedingungen nichts**, und das hat einen strukturellen
Grund: Die Bedingungen der Kandidatenzeilen sind Ressourcenschwellen und Brand-Restlaufzeiten,
keine sich ausschließenden Bosszustände. `heat >= 15` und `heat >= 20` schließen einander
nicht aus — sie ordnen sich.

**Was das Urteil richtig sieht und was hier fehlt:** Sein Beispiel („Regel A nur während
Boss-Cast, Regel B nur wenn keiner läuft") wäre ein echter Ausschluss — **im Endausbau gibt es
solche Bedingungen**, sobald Bosszustände wie `phases` oder `runaway` in Kandidatenzeilen
auftauchen. Heute gibt es sie nicht. **Der Befund ist also ein Bestandsbefund und kippt mit dem
Endausbau** — aber er kippt nur, wenn die Kandidatenerzeugung solche Zeilen überhaupt
erzeugt, und das ist eine Design-Entscheidung, keine Optimierung.

**Deshalb: nicht bauen, sondern als Voraussetzung notieren.** Der Regel-Interaktionsgraph
gehört in das Issue, das zustandsabhängige Kandidatenzeilen einführt — nicht in E11.

#### B.2.5 Zwei echte Fehler in meinem Plan

**(a) E11.3, der falsche Schluss.** Mein Satz lautete:

> „Findet die Messung, dass schon 1 % die Folge kippt, ist der Ledger gestorben."

**Das ist logisch falsch, und das Urteil hat recht.** Ein Trace kann bei +2,31 % kippen — dann
ist er bei ±1 % stabil und bei ±10 % nicht, und die Grenze bleibt unbekannt. Wichtiger: **Ein
schmales Stabilitätsintervall macht den Ledger nicht wertlos**, wenn der äußere Optimierer
viele Werte innerhalb genau dieses Intervalls prüft — und genau das tut eine Binärsuche nach
dem minimalen Buff (Anhang B.2.3).

Die `±1/±10/±50`-Messung ist eine brauchbare **Erkundung**, aber sie ist kein Beweis und darf
kein Abbruchkriterium sein. Korrektur in E11.3.

**(b) E11.4b, die willkürliche Cache-Schwelle.** Mein Kriterium war „Trefferquote über 15 %,
sonst lohnt der Cache nicht". Die Zahl ist geraten. Richtig ist die Break-even-Rechnung:

```
Lohnt, wenn   L + (1-h)·(F + I)  <  F
also wenn     h  >  (L + I) / (F + I)

L  Lookupkosten      I  Insert-/Miss-Aufschlag      F  Kosten der Auswertung
```

Bei einem Vollkampf (F groß) lohnt schon eine sehr kleine Trefferquote; bei einer
Ledger-Auswertung (F klein) womöglich auch 30 % nicht. **Die Schwelle folgt aus gemessenen
Kosten, nicht aus einer Prozentzahl** — und sie ist **pro Auswertungspfad verschieden**.
Korrektur in E11.4b.

#### B.2.6 Was übernommen wird, und in welcher Form

| #   | Punkt des Urteils                        | Urteil                              | wohin                                                    |
| --- | ---------------------------------------- | ----------------------------------- | -------------------------------------------------------- |
| 1   | Multi-Policy Decision DAG                | **1,07× gemessen**                  | verworfen; die Zahl geht als Abnahmekriterium nach E11.4 |
| 2   | Macro-Transition-Cache                   | misst dieselbe Größe                | verworfen mit 1                                          |
| 2b  | **Control-/Reward-State-Trennung**       | eigenständig richtig                | **E11.4d**, schärft den Cache-Schlüssel                  |
| 3   | Symbolische Trace-Regionen               | richtig                             | **E11.3a**, ersetzt die ±-Messung als Nachweis           |
| 4   | Breakpoint-Zellen für Statwerte          | **Faktor 1,00 gemessen**            | nur für zauber-lokale Parameter, → E12.1                 |
| 5   | Inverse Treppenfunktion                  | bestätigt                           | **E12.2**                                                |
| 6   | Exakte Crit-Verteilungs-DP               | richtig                             | **E11.1b**                                               |
| 7   | A*/Branch-and-Bound als Orakel           | richtig, mit Vorbehalt              | **E11.5b**                                               |
| 8   | Pareto-/Antichain-Pruning                | richtig, mit Beweispflicht          | **E11.5b**, Teilschritt                                  |
| 9   | Policy-Äquivalenz                        | **1,77× gemessen — trägt**          | **E11.4f**                                               |
| 10  | Partial-Order Reduction                  | 0,82 % gemessen                     | Voraussetzung notiert, nicht gebaut                      |
| 11  | Boss-Active-Set                          | richtig, **Endausbau-Punkt**        | **E11.5c**                                               |
| 12  | Adaptive LNS mit Bandit                  | richtig                             | **E11.5a**                                               |
| 13  | Sequential Racing                        | richtig, aber erst bei `purity > 0` | **E11.1b**, zweiter Teil                                 |
| 14  | Projection Planner                       | richtig als Ordnungsprinzip         | Teil 6, kein eigenes Issue                               |
| 15  | Router-Kostenmodell                      | **findet meinen Fehler**            | E11.4b korrigiert                                        |
| 16  | Kernel-Spezialisierung                   | richtig, mein Feld war folgenlos    | **E11.7b**                                               |
| 17  | `SearchFight` / `AuditFight`             | richtig                             | E11.4, ausformuliert                                     |
| 18  | `ResultGuarantee` statt „alles Näherung" | richtig als Semantik                | Teil 4 präzisiert                                        |
| 19  | Verteilte Ausführung, zweiter Rechner    | **kollidiert mit einer Vorgabe**    | Teil 2.1                                                 |

### B.3 Wo dieser Plan von den Vorschlägen abweicht

| Vorschlag                                   | mein Plan                                                   | Grund                                                                                                                                         |
| ------------------------------------------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Trace-Ledger als Phase 3 bauen              | erst **messen**, ob Traces bei Zahlenänderungen stabil sind | Die Messung ist inzwischen gefahren: **1–5 % Stabilität** (Teil 6.2). Die Reihenfolge war richtig — der Ledger lebt, aber mit schmaler Region |
| `Fight::new` in Phase 2 ersetzen            | später, eigenes Issue                                       | 97 page-faults auf 200 000 Kämpfe — der Pfad allokiert nicht                                                                                  |
| Event-Kalender als Endfassung               | erst nach `next_expiry_at` + festes Array **neu messen**    | der Rest könnte zu klein sein für den Umbau                                                                                                   |
| BOLT in Phase 5                             | **bedingter A/B-Test nach PGO**                             | Ein fehlendes Werkzeug ist ein Einrichtungszustand, keine Architekturentscheidung — Auslöser ist ein Frontend-/I-Cache-Befund aus E15.3       |
| CUDA als Phase 8                            | eigenes Epic, nicht Teil von E11                            | `nvcc` fehlt, Gruppen zu klein                                                                                                                |
| Same-Spell-Subsumption als exakte Reduktion | ja, aber als **kleiner** Posten                             | gemessen ~21 % bei Länge 12, Faktor 1,27                                                                                                      |
| Kompakter Zustand als eigene Maßnahme       | in E11.4 gebündelt                                          | 40 Checkpoints à 5,6 KB passen sonst nicht in den Cache                                                                                       |

**Und gegenüber dem zweiten Urteil** (Belege in Teil 1c):

| Urteil                                          | mein Plan                                             | Grund                                                                                                |
| ----------------------------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Multi-Policy DAG als stärkste fehlende Maßnahme | **verworfen**, die Zahl geht als Kriterium nach E11.4 | gemessen 1,07× nach Deduplizierung; 216 von 528 Kandidaten ändern nichts                             |
| Macro-Transition-Cache als eigene Ebene         | **verworfen**                                         | „einzigartige Übergänge" **ist** die Trie-Knotenzahl — dieselbe Messung                              |
| Breakpoint-Zellen für Statwerte                 | nur für **zauber-lokale** Parameter                   | über einen globalen Tempo-Wert gemessen: Faktor **1,00** bei 14 Zaubern                              |
| Partial-Order Reduction über Bedingungen        | **nicht gebaut**, als Voraussetzung notiert           | 0,82 % der Paare; die Bedingungen ordnen sich, statt sich auszuschließen                             |
| Verteilte Ausführung auf dem zweiten Rechner    | **nicht in E11**                                      | kein AVX2, kein FMA — er kann E11.7 nicht ausführen; und die Vorgabe des Auftraggebers steht dagegen |
| „Ledger gestorben, wenn 1 % kippt"              | **mein Fehler**, korrigiert                           | ein schmales Intervall reicht, wenn der Optimierer genau darin sucht                                 |
| Cache-Schwelle 15 %                             | **mein Fehler**, durch Break-even ersetzt             | die Schwelle ist pro Auswertungspfad verschieden                                                     |
| Projection **Planner** als eigenes Bauteil      | als Zuordnungstabelle in E11.5b                       | die Zuordnung ist statisch — sie braucht keinen eigenen Entscheider                                  |

---
