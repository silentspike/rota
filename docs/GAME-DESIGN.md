---
id: ROTA-GAME-DESIGN
status: Stable
ssot: true
refs:
  docs: [WORKER-GUIDE, VERIFICATION, STEP1-SPEC, suno-sfx, ASSETS]
---

# ROTA — Game Design (SSOT der Umsetzung)

> **Dies ist die verbindliche Grundlage für alle Dev-Sessions.**
> Bei Widerspruch zwischen Code, Kommentaren und diesem Dokument gewinnt **immer dieses Dokument**.
> Was hier nicht steht, ist nicht entschieden — dann **fragen, nicht raten**.

## Wie dieses Dokument zu benutzen ist

| Du willst …              | Lies                                                            |
| ------------------------ | --------------------------------------------------------------- |
| überhaupt anfangen       | Kapitel 1–3                                                     |
| eine Spielmechanik bauen | Kapitel 4–7, 11                                                 |
| am HUD arbeiten          | Kapitel 8–10                                                    |
| am Fortschritt arbeiten  | Kapitel 12–14                                                   |
| Technik/Architektur      | Kapitel 18–19                                                   |
| **eine Idee einbringen** | **Kapitel 21 ZUERST** — dort steht, was bereits verworfen wurde |

> **Kapitel 21 ist Pflichtlektüre vor jeder Design-Entscheidung.** Jede Zeile dort war eine bewusste
> Wahl des Auftraggebers. Nichts davon ohne ausdrückliche Rücksprache wieder aufnehmen.

**Verhältnis zum Planungs-Plan:** Die Planung wird in `~/.claude/plans/greedy-skipping-russell.md`
geführt (Master-Session). Dieses Dokument ist die daraus abgeleitete Umsetzungs-Fassung.
Bei Änderungen wird **zuerst der Plan gepflegt, dann hierher gespiegelt** — nie umgekehrt.

---

## 1. Was ROTA ist

Ein im Browser spielbares **Spell-Rotation-DPS-Spiel**. Keine Spielfigur, keine Bewegung, keine
Positionierung. Der gesamte Spielinhalt ist das Beherrschen der optimalen Zauber-Reihenfolge gegen
einen Boss unter Zeitdruck.

**Leitmotiv: easy to learn, very hard to master.** Gelegenheitsspieler sollen Spaß haben und
durchkommen; Hardcore-Spieler jagen das Maximum und erreichen es **nie ganz**, damit immer ein
letzter Anreiz bleibt.

**Zwei Spielphasen mit völlig unterschiedlichem Charakter:**

| Phase           | Charakter                                                                                       | Ziel-Spielzeit   |
| --------------- | ----------------------------------------------------------------------------------------------- | ---------------- |
| **Stufe 1–100** | Ein **Durchspielen** — man kommt voran, lernt Zauber und Mechaniken, scheitert selten dauerhaft | **~24 Stunden**  |
| **Endgame**     | Ein **harter Grind** — viele Versuche je Boss, hohe Anforderungen                               | **~300 Stunden** |

**Kein WoW-Klon.** World of Warcraft dient ausschließlich als Vorbild für **Mechanik**. Namen, Welt,
Begriffe und Ästhetik sind eigenständig.

**Die Welt: Alchemie.** Ein namenloser Alchemist an seinem Werktisch; die Bosse sind
**Transmutationen, die schiefgehen**. Der Spieler _ist_ der Alchemist — nie gezeigt, weil man aus
seinen Augen schaut. Das HUD ist sein Werktisch.

**Der Name ROTA** ist der Spieler-Ausdruck für Rotation und zugleich lateinisch für „Rad" — das
klassische alchemistische Sinnbild für den Kreislauf der Umwandlung.

---

## 2. Nicht verhandelbare Design-Gesetze

Wortgleich aus `/work/rota/CLAUDE.md`. Diese sechs Punkte stehen nicht zur Diskussion.

1. **Keypress = Trigger.** Ein Tastendruck löst genau eine Aktion aus. NIEMALS Tasten halten,
   niemals unterschiedlich lange Drücke. Cast-Balken und DoTs laufen nach dem Druck von selbst.
2. **Nur Keybinds.** Zauber werden ausschließlich über die Tastatur ausgelöst (einfache Tasten,
   keine Strg/Alt-Kombinationen). Die Zauberleiste ist sichtbar, aber nicht klickbar.
3. **Feature-Parität Fenster/Vollbild.** Vollbild ist rein kosmetisch — identisches Verhalten,
   identische Features in beiden Modi. Keine exklusiven Vollbild-Features.
4. **Kein Figuren-Artwork.** Keine Spielfigur, keine illustrierten Boss-Kreaturen. Bosse sind
   abstrakte "Targets" (geometrisch, mit Zustands-Feedback). Das Spiel IST das HUD — Design-Qualität
   fließt in Zauberleiste, Cooldown-Anzeigen, Zahlen, Balken, Meter.
5. **Spieltexte nur Englisch.** UI, Zaubernamen, Talente, Tutorials: English only.
   Kommunikation mit dem Auftraggeber: Deutsch.
6. **Determinismus.** Keine Zufalls-Procs im Spieler-Kit. Synergien sind 100% planbar.
   Zufall existiert nur beim Zauberbuch-Draft (gelenkter Zufall).

> **Zu Gesetz 6, wichtige Präzisierung:** _Krit ist die eine bewusste Ausnahme._ Kritische Treffer
> werden echt gewürfelt (siehe 6.5). Gemeint ist: keine zufällig auslösenden Zauber-Effekte, keine
> „20 % Chance, dass X passiert"-Mechaniken im Spieler-Kit.

### 2.1 Leitplanken des Auftraggebers

Beim Entwerfen jeder Einzelheit gelten zusätzlich diese acht Sätze:

1. **Kein WoW-Klon** — Mechanik ja, Namen und Welt nein
2. **Jeder Zauber muss irgendwo die beste Wahl sein** — keine 60 Zauber, von denen 10 gespielt werden
3. **Der Spieler soll spielen können, wie er will** — keine erzwungenen Umbauten
4. **Viele Versuche ≠ frustrierende Versuche** — jeder Versuch muss etwas einbringen
5. **Aufwand nicht überschätzen** — ein Boss ist eine Datendatei, realistisch 1–3 Stunden
6. **Erweiterbarkeit ist Grundprinzip** — ein neuer Zauber ist ein Datensatz, kein Code
7. **Leistung darf nicht leiden** — auch bei 20 Tasten, vielen DoTs und allem gleichzeitig
8. **Der Auftraggeber sieht beim Testen nur das Spiel** — alle Messwerte laufen unsichtbar mit

---

## 3. Spielkern

| Thema            | Festlegung                                                                                                                                                                                                                             |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Spieltiefe**   | **Nur Abklingzeiten.** Keine globale Ressource, kein Mana. Die Schwierigkeit entsteht aus der Menge gleichzeitig zu verwaltender Cooldowns                                                                                             |
| **Synergien**    | **Feste, planbare Kombinationen** — 100 % deterministisch. Zauber A verstärkt Zauber B, immer                                                                                                                                          |
| **Eingabe**      | **Nur Tastatur.** Nur einfache Tasten und Umschalt-Kombinationen (kein Strg/Alt)                                                                                                                                                       |
| **Zauber-Typen** | Sofort-Zauber · Zauber mit Wirkzeit (ab Mittelspiel) · DoTs · **Kanalisierungen** (Wirkung fällt laufend an, siehe 7.2). **Keine** Kanal-Zauber mit **Tastenhalten** — eine Kanalisierung startet mit einem Druck und läuft von selbst |
| **Umfang V1**    | **68 Bosse in 14 Instanzen** — 8 Aufstiegs-Instanzen à 4 Bosse (32) plus 6 Endgame-Instanzen à 6 Bosse (36)                                                                                                                            |
| **Höchststufe**  | **100**                                                                                                                                                                                                                                |
| **Talentpunkte** | **~50** (etwa jede zweite Stufe einer)                                                                                                                                                                                                 |

**Erfahrungskurve** (entschieden am 2026-08-05, Issue #248):

```
Erfahrung je Sieg        100 × Boss-Stufe
Bedarf Stufe n → n+1     200 × n^1.5
Summe                    7.900.243 Erfahrung, ~1325 Kämpfe, ~22 Stunden
```

> **Wie die Summe nachzurechnen ist** (korrigiert am 2026-08-05): `7.900.243` ist die Summe der
> **gerundeten** Tabelleneinträge — die ungerundete Reihe ergibt 7.900.245, und wer nachrechnet,
> muss wissen, welche der beiden dasteht. Die `~1325` gelten bei **Übernahme** des Überschusses;
> beim Verfallen wären es 1365. Die erste Fassung dieses Absatzes nannte „~1350" — das war ein
> Mittelwert aus zwei Rechnungen und keine von beiden.

Drei Alternativen wurden durchgerechnet und verworfen: konstant (16,5 h, aber Stufe 99 dauert so
lange wie Stufe 2), linear (165 h) und quadratisch (82,5 h) — bei beiden letzten explodiert die
Kampfzahl je Stufe auf 99 bzw. 198.

Die Kurve liegt als **Tabelle in den Daten**, nicht als Formel: `n^1.5` ist eine
Fließkomma-Potenz, und `Math.pow` in JS ist gegenüber `f64::powf` in Rust nicht garantiert
bitgleich (18.4 verlangt Bitgleichheit). Die erzeugende Formel steht als Kommentar in der Datei.

**Die Erfahrung hängt an der BOSS-Stufe**, nicht an der des Spielers. Der abnehmende Ertrag aus
12.6 sinkt mit dem **Stufenabstand** — einen Abstand gibt es nur, wenn beide Seiten eine Stufe
haben.

**Erfahrung wird beim Aufstieg übernommen, nicht zurückgesetzt.** Verfallen bestrafte den, der
einen starken Boss wählt: Wer mit 190 von 200 in einen Kampf geht und 1000 gewinnt, hätte 990
verloren. Das kehrt 12.6 um, wo der Ertrag bei einem **zu leichten** Boss sinken soll. |
| **Zauber-Bestand** | **60** (12 je Schule), davon **12–16 gleichzeitig auf der Leiste** |

Man besitzt also deutlich mehr Zauber, als man mitnehmen kann. Das ist Absicht: Die Zusammenstellung
ist eine der beiden Kernaufgaben (die andere ist die Reihenfolge).

---

## 4. Die fünf Schulen

Die Schulen sind **mechanische Rollen**, nicht nur Farben. Jede spielt sich anders.
Namen sind echte alchemistische Prozesse, zugeordnet nach ihrer historischen Bedeutung.

| Schule           | Bedeutung   | Rolle                      | Charakter                                                                                                                                                                      |
| ---------------- | ----------- | -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **CALCINATION**  | Verbrennen  | Aufbau / Schaden über Zeit | Effekte laufen weiter, Stärke sammelt sich an. Schwacher Start, starkes Ende; muss aufrechterhalten werden. Passt zu langen Endkämpfen                                         |
| **FIXATION**     | Festmachen  | Kontrolle des Bosses       | Greift in das ein, was der Boss tut: verlangsamt seine Zauber (größeres Unterbrechen-Fenster), verzögert Angriffe. Die einzige Schule, die den Kampfverlauf selbst beeinflusst |
| **SUBLIMATION**  | Umwandeln   | Cooldown-Manipulation      | Wirkt auf das Rotations-System selbst: verkürzt Abklingzeiten. Der Rücksetzer-Zauber gehört hierher. Die Schule für Könner                                                     |
| **COAGULATION**  | Verfestigen | Überleben                  | Heilung, Schutzschilde, Reflexion. Auf hohen Schwierigkeitsstufen muss man Leisten-Plätze dafür opfern — echte Abwägung Schaden gegen Sicherheit                               |
| **PUTREFACTION** | Zersetzen   | Schwächung des Bosses      | Erhöht nicht den eigenen Schaden, sondern lässt den Boss **von allen Quellen** mehr Schaden nehmen. Wirkt mit ALLEN anderen Schulen zusammen                                   |

> Jeder Name trägt seine Mechanik in sich — wer die Bedeutung einmal liest, vergisst die Zuordnung
> nicht mehr. Das rechtfertigt die sperrigen Begriffe. Im Spiel steht überall der Klartext daneben:
> „COAGULATION (Survival)".

### 4.1 Die fünf zentralen Mechaniken

**Keine Liste fester Zauber-Paare, sondern fünf Systeme.** Jede Schule hat eine tragende Mechanik,
in die ihre zwölf Zauber einzahlen.

| Schule           | Zentrale Mechanik                                                                                                                                                                                                                                                                                                                                   |
| ---------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **CALCINATION**  | **Hitze sammeln und entladen.** Jede Wirkung erzeugt Hitze; **ein Entlade-Zauber verbraucht die gesamte angesammelte Hitze** und schlägt entsprechend hart zu. Klarer Rhythmus aus Aufbauen und Entladen — und die Entscheidung _wann_ entladen wird, ist echte Planung. Der Entlade-Zauber ist damit Pflichtzauber der Schule                      |
| **FIXATION**     | **Kontroll-Punkte als Vorrat zum Ausgeben.** Fixierungs-Zauber bauen Punkte auf; der Spieler gibt sie gezielt aus, um Boss-Handlungen zu beeinflussen: einen Zauber verlangsamen, einen Schlag verzögern, eine Phase hinauszögern. Ein **Planungswerkzeug**, keine passive Wirkung                                                                  |
| **SUBLIMATION**  | **Automatische Verkürzung aller Abklingzeiten.** Sublimations-Zauber verkürzen laufend alle Abklingzeiten um einen kleinen Betrag. Der **Cooldown-Rücksetzer** bleibt als eigener Zauber daneben bestehen: er setzt den _nächsten gewirkten_ Zauber zurück                                                                                          |
| **COAGULATION**  | **Drei Wege nebeneinander:** (1) **Schutzschicht**, die eingehenden Schaden aufnimmt und sich stapeln lässt — belohnt Vorausschauen, weil sie VOR dem Schlag stehen muss; (2) **klassische Heilung**, die Fehler nachträglich verzeiht; (3) **Schaden-in-Schutz-Umwandlung**, die einen Teil des ausgeteilten Schadens laufend in Schutz verwandelt |
| **PUTREFACTION** | **Zersetzungs-Stapel auf dem Boss.** Jede Wirkung legt einen Stapel an; je mehr Stapel, desto mehr Schaden nimmt der Boss **von allen Quellen** — **6 % je Stapel** (entschieden am 2026-08-05, Issue #224). Stapel verfallen langsam, mit Obergrenze. Allein gespielt schwach — braucht immer Partner                                              |

> **Warum fünf Systeme statt 120 Einzelregeln:** Man lernt fünf Mechaniken; jede Schule fühlt sich
> völlig anders an; und **ein neuer Zauber fügt sich automatisch ein**, weil er auf eine bestehende
> Mechanik einzahlt. Genau das Erweiterbarkeits-Ziel.
>
> **Risiko:** Diese fünf Mechaniken tragen das ganze Spiel. Sie müssen sehr sorgfältig entworfen
> werden und sind der erste Prüfstein im Prototyp (siehe 20.3).

#### Zwei Festlegungen zur Bauart, entschieden am 2026-08-04

**Putrefaktion: jeder Stapel hat seine eigene Uhr.** Der älteste verfällt zuerst, die jüngeren
bleiben. Wer regelmäßig nachlegt, hält den Aufbau oben.

> Ein gleitender Zustand statt eines Alles-oder-nichts. Der Spieler darf den Aufbau bewusst absacken
> lassen, wenn gerade Wichtigeres ansteht, und verliert nicht alles — ein gemeinsamer Verfall hätte
> jeden Zwischenfall im Kampf den ganzen Aufbau gekostet.
>
> **Preis, den die Anzeige tragen muss:** mehrere Restzeiten gleichzeitig. Das ist eine Anforderung
> an 12.3 (Anzeige der Schul-Mechaniken über der Leiste), keine Nebensache.

**Sublimation: ein fester Betrag, kein Anteil.** Jede Wirkung nimmt allen laufenden Abklingzeiten
dieselbe Zeitspanne.

> **Der Spieler kann abzählen.** Er weiß, wann etwas bereit wird, und kann darauf planen — ein
> Prozentsatz auf Restzeiten wäre im Kopf nicht mehr zu rechnen und widerspräche der Forderung nach
> planbaren Abklingzeiten (6.2).
>
> **Folge für die Ausrichtung der Schule:** Ein fester Betrag wirkt bei kurzen Abklingzeiten
> stärker als bei langen. Sublimation ist damit die Schule für schnelle Rotationen — was zur
> Beschreibung „die Schule für Könner" passt.

**Die Zahlen dazu werden gemessen, nicht gesetzt** (Teil F.6): wie viel Prozent ein Stapel trägt, wie
viele es höchstens gibt, wie lange sie halten, um wie viel Sublimation verkürzt. Bis zur Messung
stehen vorläufige Werte im Code — an genau einer Stelle je Größe und ausdrücklich als vorläufig
benannt, damit das Ersetzen eine Zeile bleibt.

> **Kein Begründungs-Anschein.** Eine vorläufige Zahl trägt den Vermerk „wird gemessen", nicht eine
> Herleitung, die sie geprüft aussehen lässt.

### 4.2 Ressourcen

**Keine globale Ressource.** Kein Mana. Aber einzelne Schulen arbeiten mit **Ladungen**

> **Warum 6 % und nicht 3 %.** Der Break-even, ab dem sich der Leisten-Platz überhaupt
> rechnet, liegt bei **5,4 %** je Stapel. Mit 3 % wäre PUTREFACTION **nie** die beste Wahl,
> und das ist etwas anderes als „allein gespielt schwach".
>
> **Die Gesamtwirkung folgt aus vier Werten — hier mit ihrer Quelle, damit die nächste
> Prüfung zwei Minuten dauert statt einer Messung:**
>
> | Größe                     | Wert     | steht in                                             |
> | ------------------------- | -------- | ---------------------------------------------------- |
> | Obergrenze der Stapel     | **5**    | `MAX_DECAY_STACKS`, `src/core/combat.ts`             |
> | je Stapel                 | **6 %**  | `DECAY_STACK_PERCENT`, ebenda                        |
> | Haltbarkeit eines Stapels | **12 s** | `DECAY_STACK_DURATION_MS`, ebenda                    |
> | Abklingzeit des Erzeugers | **2 s**  | `canker.cooldownMs`, `data/spells/putrefaction.json` |
>
> **Gesamtwirkung: 5 × 6 % = 30 %** mehr Schaden aus allen Quellen. Erreichbar mit `CANKER`
> allein nach **acht Sekunden** (t = 0, 2, 4, 6, 8; der erste verfällt erst bei 12) — ohne
> `BLIGHT`, der bei 1,5 s Abklingzeit noch schneller füllt.
>
> **Die 30 % sind nicht die Zahl, mit der entschieden wurde.** Issue #224 rechnete am
> 2026-08-05 mit 4,5 s Abklingzeit und drei Stapeln und kam auf 18 %. Beide Werte stimmen
> heute nicht mehr; die Entscheidung „6 %" bleibt trotzdem richtig, weil sie über dem
> Break-even liegt — bei fünf Stapeln erst recht. Was **nicht** geprüft ist: ob 30 % zu
> „allein gespielt schwach" (Kapitel 11) passen. Das ist eine Balance-Frage und liegt beim
> Auftraggeber (Issue #496, AC3).
>
> Der Wert steht auf beiden Seiten: `src/core/combat.ts` und `sim/src/fight.rs`. Dass die
> Tabelle oben zu den Konstanten passt, prüft `tests/unit/design-herleitung.test.ts` —
> **diese Rechnung ist einmal still gealtert, und das soll sie nicht wieder.**

(Hitze-Stapel, Kontroll-Punkte, Zersetzungs-Stapel) — das gibt ihnen Tiefe und ein eigenes
Spielgefühl, ohne die Grundentscheidung „Abklingzeiten sind die alleinige Schwierigkeit" anzutasten.

> **Jede Schul-Größe bekommt eigene Felder, keine gemeinsame Struktur.**
> _Festgehalten am 2026-08-04, nachdem E3-3b dieselbe Frage stellte, die E3-4 schon beantwortet hatte._
>
> Im Kampf-Zustand steht heute:
>
> ```
> resource, resource_overflow                                  Hitze (CALCINATION)
> shield, shield_expires_at, shield_stacks, shield_absorbed     Schutzschicht (COAGULATION)
> ```
>
> **Diese Größen sind verschiedene Dinge, nicht verschiedene Instanzen desselben Dings.** Hitze
> verfällt mit der Zeit. Die Schutzschicht nimmt Schaden auf und stapelt. Kontroll-Punkte werden
> gezielt ausgegeben. Zersetzungs-Stapel wirken auf den Boss statt auf den Spieler.
>
> Eine gemeinsame Ressourcen-Struktur würde sie gleich behandeln — und die erste Sonderregel bricht
> sie wieder auf. Vier Felder mit klaren Regeln sind ehrlicher als eine Abstraktion mit vier
> Ausnahmen.
>
> **Und der Nebeneffekt trägt:** `build.rs` prüft jeden `resourceGain` scharf gegen die Kennung der
> Hitze. Diese Schärfe aufzugeben, um ein zweites Feld unterzubringen, wäre der schlechteste Tausch
> — sie fängt genau die stillen Fehler, die sonst erst Monate später auffallen.
>
> Sollte sich später zeigen, dass mehrere Größen dieselben Regeln teilen, ist die Zusammenlegung ein
> eigenes Issue mit eigenem Nachweis — kein Nebenprodukt einer Schul-Umsetzung.

### 4.2b Die drei Fixierungs-Wirkungen im Einzelnen

_Festgelegt am 2026-08-04. Die drei Wirkungen standen seit 4.1 fest, ihre **Reichweite und Dauer**
nicht — und der Code hatte sie stillschweigend beantwortet._

| Wirkung                 | Greift an                                 | Reicht wie weit                                         |
| ----------------------- | ----------------------------------------- | ------------------------------------------------------- |
| **Zauber verlangsamen** | Die Wirkzeit dehnt sich                   | Der **eine laufende** Zauber                            |
| **Schlag verzögern**    | Der Zeitleisten-Eintrag rückt nach hinten | **Nur der nächste.** Die folgenden bleiben, wo sie sind |
| **Phase hinauszögern**  | Die Phasen-Schwelle sinkt vorübergehend   | **Nur der nächste Wechsel**, und **befristet**          |

**Alle drei wirken auf genau eine Boss-Handlung.** Das ist die gemeinsame Regel der Schule: Fixierung
kauft einen einzelnen Moment, nicht einen Zustand.

#### „Hinauszögern" heißt befristet, nicht dauerhaft

Die Senkung endet, **was zuerst eintritt**:

1. die Frist läuft ab, oder
2. der nächste Phasenwechsel hat stattgefunden — dann ist sie verbraucht.

> **Warum nicht dauerhaft:** Eine Senkung, die den ganzen Kampf hält, verschiebt bei einem Boss mit
> zwei Schwellen **beide** — ein einziger Druck zu Kampfbeginn wäre die beste Spielweise, und danach
> gäbe es nichts mehr zu entscheiden. Das widerspricht „Planungswerkzeug, keine passive Wirkung"
> (4.1). Befristet muss man den Moment treffen.
>
> **Der Code hielt sie bis zum 2026-08-04 dauerhaft.** `phase_hold_percent` wurde gesetzt und nur
> beim Kampf-Reset wieder genullt — es gab keinen Ablauf. Aufgefallen ist es nicht beim Spielen,
> sondern beim Gegenprüfen von Issue #100: Die Zusicherung „die Phase kann nur steigen" ließ sich
> im Code aufheben, ohne dass einer von 26 Prüfläufen es merkte. Die Zeile war richtig — nur
> ungeprüft, weil das Ereignis, das sie abfängt, gar nicht vorkommen konnte.

#### Wann die Phase überhaupt zurückfallen könnte

Nur in **einem** Fall, und der ist der Prüffall:

```
Der Boss hat bereits gewechselt (Lebensstand unter der Schwelle, Phase 1).
DANN wird die Senkung gewirkt — der gerechnete Anteil steigt über die Schwelle.
Ohne Schutzregel fiele der Boss auf Phase 0 zurück.
```

**Eine schon gezeigte Mechanik verschwindet nicht wieder** (11.2). Die Schutzregel bleibt deshalb
bestehen, auch wenn die Frist den Fall seltener macht — und sie braucht einen Prüffall, der die
Senkung **nach** dem Wechsel wirkt. Wer sie davor wirkt, prüft nichts.

**Die Frist gilt für alle Zauber gleich** — sie gehört zur **Wirkung**, nicht zum einzelnen Zauber.

_Präzisiert am 2026-08-04. Hier stand „eine Zahl in den Daten, nicht im Code" — das war zu schnell
aus der allgemeinen Katalog-Regel abgeleitet._

> Wäre die Frist je Zauber einstellbar, hieße „hinauszögern" bei jedem etwas anderes, und der Spieler
> müsste sie pro Zauber nachschlagen. Er lernt sie stattdessen einmal. Die Unterschiede zwischen
> Fixierungs-Zaubern entstehen weiter über **Stärke** und **Abklingzeit** — dort sind sie sichtbar
> und gehören in die Daten.

**Der Wert bleibt vorläufig** bis zur Balance-Runde (Teil F.6, vor E10) — vorläufig ist die _Zahl_,
nicht ihr Ort.

### 4.3 Schul-Mischung

**Frei.** Die Leiste darf beliebig aus allen Schulen befüllt werden, keine Begrenzung.

Praktisch ergeben sich **zwei bis drei Schulen** als beste Wahl, weil eine Schul-Mechanik mehrere
Zauber braucht, um zu tragen — Zersplittern lohnt sich nicht. **Die Struktur entsteht von selbst
statt durch Verbot.**

---

## 5. Synergien

**Schulübergreifende Synergien verbinden die Mechaniken.** Beispiele:

- FIXATION verlangsamt einen Boss-Zauber → das Zeitfenster für einen großen CALCINATION-Aufbau wächst
- SUBLIMATION setzt zurück → ein zweiter starker Zauber wird möglich
- PUTREFACTION erhöht den Schaden aller anderen Schulen

| Regel                | Festlegung                                                                                                                                                                                                                                                                     |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Synergie-Fenster** | **Zwei bis drei globale Wartezeiten** (3–5 s bei 1,5 s Grundtakt). Lang genug, dass es kein Reflextest ist; kurz genug, dass man nicht beliebig aufschieben kann. **In Wartezeiten gerechnet, nicht in Sekunden** — sonst verschiebt sich das Verhältnis bei hohem Tempo       |
| **Stapel-Regel**     | **Verschiedene Bonus-Arten stapeln multiplikativ, gleiche Arten ersetzen einander.** Das Zusammenlegen aller großen Fähigkeiten zu einem gewaltigen Fenster wird damit zur Kernfertigkeit („Cooldown-Alignment"), aber derselbe Bonus lässt sich nicht stumpf mehrfach stapeln |
| **Anzeige**          | **Pulsierender Rahmen** am begünstigten Zauber, solange das Fenster offen ist                                                                                                                                                                                                  |

Die Bonus-Kategorien müssen im Hilfe-System erklärt werden (siehe 17.3).

**Anzeige der Schul-Mechaniken:** Nur die **aktiv genutzten**, direkt über der Zauberleiste. Wer
keine CALCINATION mitführt, sieht keine Hitze-Anzeige. Nie mehr als drei bis vier Anzeigen
gleichzeitig, und sie sitzen dort, wo der Blick ohnehin ist.

---

## 6. Werte-System und Formeln

### 6.1 Grundwerte

Auf jedem der zehn Ausrüstungsplätze vorhanden:

| Name        | Ersetzt     | Bedeutung in der Fiktion                                |
| ----------- | ----------- | ------------------------------------------------------- |
| **ESSENCE** | Intelligenz | Stärke der Substanz — erhöht den Schaden aller Prozesse |
| **VESSEL**  | Leben       | Belastbarkeit des Gefäßes, das die Reaktion hält        |
| **WARD**    | Rüstung     | Schutzbannung gegen Rückschlag                          |

### 6.2 Sonderwerte

Die drei Werte, die die Rotation tatsächlich beeinflussen:

| Name          | Ersetzt     | Warum der Begriff passt                           |
| ------------- | ----------- | ------------------------------------------------- |
| **CATALYSIS** | Tempo       | Katalyse beschleunigt Reaktionen — fachlich exakt |
| **PURITY**    | Kritchance  | Reinheit der Substanz                             |
| **YIELD**     | Kritschaden | Ausbeute                                          |

**CATALYSIS ist der einzige Wert, der die Rotation strukturell verändert.** Er wirkt auf
**Abklingzeiten und DoT-Tick-Abstände** — ausdrücklich **nicht** auf die globale Wartezeit.

> **Festlegung vom 2026-07-29.** Der GCD sinkt ausschließlich über den **Talentbaum**, nie über
> Ausrüstung. Grund: Kürzere Abklingzeiten sind eine Planungsaufgabe (mehr Wirkungen möglich,
> man muss sie einweben). Ein kürzerer GCD ist ein Reaktionstest (die Hand muss schneller
> werden). Ein einziger Ausrüstungswert, der beides tut, zwingt dem Spieler Fingerfertigkeit
> auf, statt sie ihn wählen zu lassen — und widerspricht dem Grundsatz, dass die Reaktionszeit
> nicht bewertet wird (21.2).
>
> **Am 2026-08-04 bestätigt**, nachdem 6.7 sie versehentlich übergangen hatte. **Wie** CATALYSIS auf
> die Zeiten wirkt, steht in 6.7; **worauf** es wirkt, steht hier.
>
> **DoT-Ticks skalieren mit** (WoW-Muster seit Patch 6.0.2): Eine DoT-lastige Ausrichtung hat
> damit denselben Skalierungswert wie jede andere. Ticks werden dichter, die **Dauer bleibt** —
> das erzeugt Breakpoints je DoT, die der Balance-Läufer messen muss.

Er wächst dauerhaft über Talente und Ausrüstung, kurzzeitig über **Overdrive** (siehe 11.4).

### 6.3 Schadensformel

```
Schaden = Grundschaden
        × ESSENCE-Faktor
        × Talent-Faktoren
        × Ausrüstungs-Sondereffekte
        × aktive Verstärkungen
        × Synergie-Faktoren
        × Widerstandsfaktor des Bosses
        × Verwundbarkeit des Bosses
        × Krit
```

> **Der achte Faktor, ergänzt am 2026-07-29.** Die **Ausrüstungs-Sondereffekte** sind Topf 4 aus
> 13.1b: legendäre Effekte und Satz-Boni, untereinander addiert, nach außen **ein** Faktor.
>
> **Position: direkt nach den Talenten.** Damit stehen die drei Faktoren, die vom Spieler kommen —
> ESSENCE, Talente, Ausrüstung — beieinander, danach die Zustands-Faktoren (Verstärkungen,
> Synergie), dann die Boss-Faktoren (Widerstand, Verwundbarkeit) und zuletzt Krit.
>
> **Warum ein eigener Faktor und keiner der bestehenden:** `essence` würde Topf 4 additiv mit
> ESSENCE machen. `talents` würde ihn additiv mit Talenten machen — und 13.1 trennt beide Systeme
> ausdrücklich („WIE die Rotation funktioniert" gegen „WIE STARK die Werte sind"). `buffs` **ist**
> Topf 3, und zwischen Töpfen wird multipliziert.
>
> Auch hier gilt: Die Position ist eine Festlegung, keine Geschmacksfrage, und heute bewegt sie
> keine Zahl — der Faktor steht bei 1,0.

**Zu Topf 2 (CATALYSIS, PURITY, YIELD): „je eigener Faktor" heißt nicht drei neue Faktoren hier.**
CATALYSIS wirkt auf Abklingzeiten und DoT-Abstände (6.2), ist also gar kein Schadensfaktor. PURITY
ist eine Wahrscheinlichkeit, kein Multiplikator. Nur YIELD landet als `crit`. Gemeint ist: **nicht in
einen gemeinsamen Multiplikator zusammengeworfen** — jeder Sonderwert behält seinen eigenen Wirkweg.

> **Der siebte Faktor, ergänzt am 2026-07-29.** Die **Verwundbarkeit des Bosses** trägt alles,
> was ihn für Schaden **aus jeder Quelle** empfänglicher macht — vor allem die Zersetzungs-Stapel
> der Putrefaktion (4.1). Sie steht **direkt neben dem Widerstand**, weil beide Aussagen über den
> Boss sind und nicht über den Spieler.
>
> **Die Position ist eine Festlegung, keine Geschmacksfrage.** Fließkomma-Multiplikation ist
> nicht assoziativ — eine andere Reihenfolge ergibt andere Bits und damit eine andere
> Zustands-Prüfsumme. Sie gehört deshalb ins Dokument und darf nicht im Code entschieden werden.
>
> **Heute ändert sie keine einzige Zahl:** Der Faktor steht bei 1,0, und 1,0 ist in IEEE754 die
> exakte Eins. Genau deshalb war Vorsicht geboten — eine Änderung, die keine Zahl bewegt, geht
> sonst unbemerkt durch.

Die Reihenfolge der Multiplikationen ist **festgelegt und für Spiel und Simulator identisch** —
Fließkomma-Multiplikation ist nicht assoziativ, und beide Seiten müssen auf denselben Wert kommen
(siehe 18.4).

### 6.4 Zahlen-Rahmen

| Größe                      | Festlegung                                                                                                                                                                                                                                    |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Schadens-Größenordnung** | ~100 Schaden auf Stufe 1, einige zehntausend auf Stufe 100. Vier- bis fünfstellige Zahlen bleiben im Kampf lesbar                                                                                                                             |
| **Wachstum**               | **Gleichmäßig ~5 % je Stufe.** Über 100 Stufen ergibt das Faktor ~130. Jede Stufe fühlt sich gleich wertvoll an, alte Inhalte werden verlässlich leichter. Der genaue Prozentsatz wird am Simulator feinjustiert                              |
| **Abklingzeiten**          | **3–180 Sekunden, fein abgestuft, NUR ganze Sekunden.** Keine festen Klassen — jeder Zauber bekommt seinen individuell gewählten Wert. Die unregelmäßigen Überlagerungen machen die Rotation interessant und schwerer zu lernen (**gewollt**) |
| **Globale Wartezeit**      | **1,5 s**, per Talent senkbar bis ca. 1,0 s                                                                                                                                                                                                   |

> **Warnung aus der Genre-Erfahrung:** WoW musste seine Zahlen zweimal zurücksetzen (2014, 2018),
> Diablo 3 landete bei Billionen. Bei 5 % je Stufe bleiben wir weit davon entfernt — aber die
> Endlos-Leiter treibt die Zahlen weiter. Die Werte-Obergrenze dort (12.4) ist deshalb nicht nur
> eine Balance-, sondern auch eine **Lesbarkeits**-Entscheidung.

### 6.5 Krit — echter Zufall

**Echte Wahrscheinlichkeit pro Zauber, pro DoT-Tick und pro Heilung.** Kein Pech-Schutz, kein
fester Startwert, keine feste Reihenfolge. PURITY und YIELD sind normale Werte aus Talenten und
Ausrüstung.

> **Bewusst akzeptierte Folge:** Auch die Leistungsmessung enthält damit Glück. Der Auftraggeber hat
> die glücksfreie Alternative (Messung allein an Zeitpunkten) ausdrücklich abgelehnt.
>
> **Konsequenz für die Umsetzung:** Das „Optimum" ist ein **Erwartungswert**, kein fester Wert.
> Die Optimums-Suche im Simulator muss über viele Durchläufe mitteln. Werte über 100 % sind möglich
> (Glücks-Durchlauf) — das muss jede Anzeige verkraften.

### 6.6 Globale Wartezeit (GCD)

**Alle Zauber lösen dieselbe Wartezeit aus.** Der gleichmäßige Grundtakt ist Voraussetzung dafür,
dass Muskelgedächtnis entsteht.

**Genau eine Ausnahme:** **Unterbrechen** löst keine globale Wartezeit aus (siehe 11.3).

**Defensiv-Zauber lösen den normalen GCD aus** — Überleben kostet Schaden und Zeit. Das ist die
Abwägung, die COAGULATION interessant macht.

### 6.7 Wie die Werte im Kampf ankommen

_Entschieden am 2026-08-04, nach Recherche zu WoW und Diablo 4. Bis dahin war die Umrechnung als
offener Punkt in Kapitel 22 geführt — sie blockierte E5 und E6._

#### Prozent stehen direkt auf dem Gegenstand

Ein Fund trägt **„+3 % Tempo"**, nicht „+120 Tempo". Was er bringt, hängt nicht von der Spielerstufe
ab.

> **Warum nicht das Punkte-Modell aus WoW:** Dort wächst zusätzlich ein Umrechnungs-Nenner mit der
> Stufe. Der Zweck — alte Funde entwerten — wird bei uns schon von der **Gegenstandsstufe** erledigt:
> Ein Stück von Stufe 20 trägt weniger Prozent als eines von Stufe 100, genau wie bei allen anderen
> Werten. Ein zweiter Mechanismus daneben wäre eine Bremse ohne eigenen Zweck.
>
> **Der Preis ist ein harter Deckel**, und der steht unten. Ohne den Nenner gibt es keine eingebaute
> Bremse.

#### Tempo (CATALYSIS) wirkt auf ZEITEN, nicht auf den Schaden

Das ist der Unterschied zu den beiden anderen Sonderwerten: CATALYSIS steht in der Schadensformel
(6.3) **gar nicht**, weder als siebter noch als achter Faktor.

> Diablo 4 formuliert es genau: _„Attack speed does not enter the damage formula, but rather it
> multiplies how often you get to use it."_ Tempo verändert nicht, wie hart ein Treffer trifft,
> sondern wie oft man handelt.

```
neue_zeit = auf_takt_aufrunden(basis_zeit × 1000 / (1000 + tempo_promille))
```

Ganzzahlig in Promille gerechnet, einmal auf den Logik-Takt gerastert. **Keine Fließkomma-Zeit** —
die Determinismus-Regel aus 18.4 bleibt unangetastet.

| Wirkort                | Untergrenze |
| ---------------------- | ----------- |
| **Alle** Abklingzeiten | ein Takt    |
| DoT-Tick-Abstand       | ein Takt    |
| Wirkzeit               | keine       |
| ~~Globale Wartezeit~~  | —           |

**Die globale Wartezeit gehört NICHT dazu.** 6.2 legt seit dem 2026-07-29 fest: Der GCD sinkt
ausschließlich über den **Talentbaum**, nie über Ausrüstung. Kürzere Abklingzeiten sind eine
Planungsaufgabe, ein kürzerer GCD ein Reaktionstest — und die Reaktionszeit wird nicht bewertet
(21.2).

**Alle Abklingzeiten skalieren mit** — anders als in WoW, wo eine Ausnahmeliste je Fähigkeit
entscheidet. Abklingzeiten sind bei uns die alleinige Schwierigkeit (2.1); eine Liste, welcher Zauber
mitskaliert, wäre eine zweite Regel zum Auswendiglernen.

#### DoTs: die Dauer bleibt, die Ticks werden mehr

```
tick_abstand = auf_takt_aufrunden(basis_abstand × 1000 / (1000 + tempo))
tick_zahl    = dauer / tick_abstand          (ganzzahlig, Rest verfällt)
```

**Kein anteiliger letzter Tick.** WoW zahlt ihn aus, wenn der Rest ≥ 0,1 s ist — bei uns wäre das
eine Kommazahl im **Schaden**, also genau dort, wo Spiel und Simulator bitgleich sein müssen.

> **Warum die Dauer fest bleibt:** Die harte Erneuerungs-Regel (7.2 — Erneuern setzt zurück, Restzeit
> ist verloren) hängt an einem stabilen Zeitfenster. Schrumpfte die Dauer mit dem Tempo, verschöbe
> sich der optimale Erneuerungszeitpunkt bei jedem Ausrüstungswechsel.

#### Die Deckel

| Wert                    | Deckel       | Woraus er folgt                                         |
| ----------------------- | ------------ | ------------------------------------------------------- |
| **CATALYSIS** (Tempo)   | **gemessen** | der Balance-Läufer — siehe unten                        |
| **PURITY** (Kritchance) | **100 %**    | jeder Treffer kritet; darüber hinaus wirkungslos        |
| **YIELD** (Kritschaden) | keiner       | wächst weiter, ohne eine Grenze zu berühren             |
| **WARD** (Minderung)    | **75 %**     | gesetzt vom Auftraggeber, 2026-08-06 — Begründung unten |

**Der WARD-Deckel ist gesetzt, nicht abgeleitet.** Ohne ihn kann eine hinreichend gute Ausrüstung
den Spieler **unverwundbar** machen — und damit verlöre nicht nur der Boss-Schaden seinen Sinn,
sondern auch die COAGULATION-Schule, die vom Überleben handelt.

> **Er ist nicht dasselbe wie der Deckel bei 100 %**, der im Kampfcode danebensteht. Der ist eine
> **Rechenregel**: Darüber würde der Spieler von jedem Treffer geheilt. Beide bleiben nebeneinander
> stehen, weil sie Verschiedenes sagen.

**Der Tempo-Deckel wird gemessen, nicht gesetzt.** Die Frage lautet: Bei welchem Wert hört Tempo auf,
sich gegen Kritchance und Kritschaden zu lohnen? Das beantwortet kein Überlegen, sondern der
Balance-Läufer.

> **Eine Ableitung stand hier und trug nicht.** Bis zum 2026-08-04 war der Deckel als „50 %, folgt aus
> `1500 / (1 + t) = 1000`" begründet. Beide Bezugspunkte waren falsch: 6.6 enthält keine Untergrenze
> (sie steht in 6.4, als **„ca. 1,0 s"** und für den **Talentbaum**), und Ausrüstungs-Tempo berührt
> die Wartezeit gar nicht (6.2).
>
> Es gäbe eine zweite Ableitung — die kürzeste Abklingzeit (3 s) erreicht bei 100 % Tempo den
> Grundtakt (1,5 s), darüber bringt sie für diesen Zauber nichts mehr. Sie ist nachprüfbar, aber sie
> beantwortet die **falsche** Frage: nicht „wo wird Tempo nutzlos", sondern „wo lohnt es sich nicht
> mehr gegen andere Werte". Das ist eine Balance-Frage, keine Rechenfrage.
>
> **Merksatz:** Eine Zahl mit Begründungs-Anschein ist schlimmer als eine offen gesetzte — sie sieht
> geprüft aus. Wer sie liest, sucht die Quelle nicht nach.

> **Folge des 100-%-Deckels bei PURITY, die im Blick bleiben muss:** Am Deckel ist Krit kein Zufall
> mehr. Kapitel 6.5 hält fest, dass der Glücksanteil **gewollt** ist — er verschwindet für den
> Spieler, der so weit investiert. Das ist ein erreichbares Ziel, auf das sich ein ganzer Aufbau
> ausrichten lässt, und zugleich der Punkt, ab dem YIELD der einzige Wert ist, der noch zählt.
> **Keine abnehmenden Erträge** federn das ab (siehe unten) — die Balance muss es tun.

#### Keine abnehmenden Erträge

Jeder Prozentpunkt zählt gleich, bis zum Deckel. Diablo staffelt Abklingzeit-Reduktion invers
multiplikativ; das ist mächtig, aber schwer zu erklären und schwer zu simulieren.

> „Jeder Punkt zählt gleich" ist **eine** Regel. „Jeder weitere zählt weniger, nach dieser Formel"
> sind drei — und der Spieler kann den Wert eines Fundes nicht mehr im Kopf ausrechnen.

#### Die Deckel sind sichtbar

**Die Werte-Übersicht zeigt zu jedem gedeckelten Wert seinen Deckel und weist Überschuss aus:**

```
CATALYSIS   52 %   ⚠ 2 % über dem Limit, wirkungslos
```

Zahl plus Warnhinweis, kein eigener Balken. **Ein Deckel, den man nicht sieht, ist eine Falle:** Wer
weiter in Tempo investiert, ohne es zu merken, verliert Ausrüstungsplätze an einen Wert, der nichts
mehr bringt.

---

## 7. Zauber

### 7.1 Zauberbalken und Wirkungsdauer — zwei getrennte Dinge

| Art                                              | Verhalten                                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Zauberbalken (Wirkzeit)**                      | **Sperrt nichts — aber jede andere Wirkung bricht ihn ab.** Länge **2,3–3,0 s** (entschieden 2026-08-06, siehe 7.1), also gut anderthalb bis zwei globale Wartezeiten. Kürzere Zauber haben **keine** Wirkzeit, sondern wirken sofort. Der Zauber wirkt **erst am Ende**; bricht er ab, ist die Wirkung **vollständig** verloren. Erzeugt eine echte Verpflichtung: „Kann ich mir das gerade leisten?" |
| **Wirkungsdauer (DoT, Verstärkung, Schwächung)** | **Blockiert nicht.** Der Zauber ist gewirkt, seine Wirkung läuft weiter, während man normal weiterdrückt                                                                                                                                                                                                                                                                                               |

#### Was einen laufenden Zauber abbricht — entschieden am 2026-08-05

**Jede Wirkung des Spielers bricht einen laufenden Cast oder eine Kanalisierung ab. Jede, ohne
Ausnahme — auch das Unterbrechen.** Das ist das WoW-Prinzip, und es ist ausdrücklich so gewollt:

> Wenn ein Spell z. B. 3 s gecastet werden muss, bis er gewirkt wird, dann wird — wenn der Spieler
> während dem Casten einen anderen Spell wirkt — der Cast **abgebrochen**. Er verliert seine
> vollständige Wirkung, da Cast-Spells immer erst wirken, wenn sie fertig gecastet sind.
> **Das gilt auch für Unterbrechen. Egal welcher Spell.**

**Der Spieler ist also nicht handlungsunfähig — er trifft eine Entscheidung, und sie kostet ihn den
laufenden Zauber.** Das ist die Verpflichtung aus dem Absatz oben, nur an anderer Stelle: nicht
„ich kann nicht", sondern „es kostet mich".

**Zwei Typen, zwei Verluste:**

| Typ                 | wann die Wirkung anfällt             | bei Abbruch                     |
| ------------------- | ------------------------------------ | ------------------------------- |
| **Cast** (Wirkzeit) | **am Ende**, vollständig             | **alles verloren**              |
| **Kanalisierung**   | **laufend**, über die Dauer verteilt | **der erreichte Anteil bleibt** |

> Beispiel des Auftraggebers: Eine Kanalisierung über 5 s für 500 Heilung, abgebrochen nach 3 s,
> hat 300 Heilung gegeben. Dasselbe gilt für Schadens- und Ressourcen-Kanalisierungen.

**Kanalisierungen sind damit ein eigener Zauber-Typ neben Sofort-Zauber, Wirkzeit und DoT.** Sie
widersprechen Design-Gesetz 1 nicht: Verboten ist das **Tastenhalten**, nicht die Kanalisierung.
Eine Kanalisierung startet mit einem Tastendruck und läuft von selbst.

**Abbruch eines Zauberbalkens: nur durch bestimmte, vorher erkennbare Boss-Angriffe.** Normale
Treffer stören nicht. Bei Abbruch geht der Zauber verloren, **die Abklingzeit läuft aber nicht an**.

> Kein Zufallsärger, sondern lernbare Mechanik: Wer den Boss kennt, fängt den Zauber gar nicht erst an.

#### Die Untergrenze von 1,5 s ist ein Grenzfall — gemessen am 2026-08-05

**„Jede Wirkung bricht ab" heißt nicht „jeder Druck".** Ein Druck, den die globale Wartezeit oder
eine Abklingzeit abweist, ist keine Wirkung und bricht deshalb nichts ab. Gemessen an einem Zauber
mit 2000 ms Wirkzeit:

```
Druck  400 ms nach Beginn   die Wartezeit weist ihn ab   bricht NICHTS ab
Druck 1600 ms nach Beginn   kommt durch                  bricht ab
```

Daraus folgt eine Grenze, die in der Zahl selbst steckt:

| Wirkzeit                   | Wer kann sie abbrechen                                                      |
| -------------------------- | --------------------------------------------------------------------------- |
| über 1,5 s                 | jeder Zauber, im Fenster nach Ablauf der Wartezeit                          |
| **genau oder unter 1,5 s** | **nur der Unterbrecher** — er ist der einzige Zauber ohne globale Wartezeit |

Bei 2000 ms Wirkzeit bleiben allen anderen Zaubern genau **500 ms** Fenster.

> **Der Grenzfall liegt auf der Untergrenze dieses Kapitels.** 1,5–2,5 s ist der Korridor; bei
> genau 1,5 s existiert die Abbruch-Entscheidung für alle Zauber außer einem nicht mehr. Das ist
> keine Fehlfunktion, sondern folgt zwingend aus der Regel — aber es ist die Zahl, an der die
> Mechanik hängt.

**Für E10 heißt das:** Ein Zauber, dessen Wirkzeit die Verpflichtung („kann ich mir das gerade
leisten?") tatsächlich erzeugen soll, braucht **deutlich mehr als eine globale Wartezeit**. Wer ihm
1,5 s gibt, gibt ihm einen Balken ohne Entscheidung dahinter.

##### Und CATALYSIS schließt das Fenster mit der Zeit ganz

Hier greifen zwei Regeln dieses Dokuments ineinander, die einzeln beide richtig sind:

|         |                                                                                                                   |
| ------- | ----------------------------------------------------------------------------------------------------------------- |
| **6.6** | Die **Wirkzeit** skaliert mit Tempo                                                                               |
| **6.2** | Die **globale Wartezeit** skaliert **nicht** mit Ausrüstungs-Tempo — sie sinkt ausschließlich über den Talentbaum |

Das Fenster ist die Differenz der beiden. Wenn nur eine Seite schrumpft, schließt es sich.
Gerechnet für einen Zauber mit 2000 ms Wirkzeit:

```
CATALYSIS   Wirkzeit   Fenster ohne Talent   mit GCD-Talent (1000 ms)
   0 %        2000            500 ms                1000 ms
  10 %        1820            320 ms                 820 ms
  20 %        1670            170 ms                 670 ms
  33 %        1510             10 ms                 510 ms
  40 %        1430              0                    430 ms
  50 %        1340              0                    340 ms      ← Deckel
```

**Ab 33,4 % CATALYSIS ist ein 2000-ms-Zauber von keinem anderen Zauber mehr abbrechbar** — nur noch
vom Unterbrecher, der als einziger keine Wartezeit auslöst. Der Wert liegt weit unter dem Deckel von
50 %, ist also im Endgame der Normalfall und nicht der Ausnahmefall.

> **Das ist kein Fehler, sondern eine Folge, die niemand entschieden hat.** Sie entsteht aus zwei
> Regeln, die getrennt voneinander festgelegt wurden — und sie kehrt sich mit dem GCD-Talent wieder
> um, weil eine kürzere Wartezeit das Fenster öffnet statt es zu schließen. Ein Spieler, der Tempo
> stapelt, verliert also eine Entscheidungsmöglichkeit; einer, der den GCD senkt, gewinnt sie zurück.

**Der Korridor dieses Kapitels kann das nicht auffangen.** Gerechnet bei 50 % CATALYSIS:

```
Basis-Wirkzeit   bei 50 % Tempo   Fenster
    2,25 s           1500 ms         0 ms
    2,30 s           1540 ms        40 ms      vier Logik-Takte
    2,50 s           1670 ms       170 ms      obere Grenze des Korridors
```

Selbst am **oberen** Ende von 1,5–2,5 s bleiben 170 ms — ein Neuntel dessen, was ohne Tempo zur
Verfügung steht. Ein längerer Zauber ist also kein Ausweg, sondern nur eine Verzögerung des
Problems.

**Damit stehen zwei Wege offen, und keiner ist entschieden:**

| Weg                                   | Was er kostet                                                                                                                                                                              |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Das GCD-Talent trägt die Mechanik** | Wer Wirkzeit-Zauber spielen will, muss in den GCD investieren. Das ist eine echte Kopplung zwischen Talentbaum und Zauberwahl — reizvoll, aber sie schränkt die freie Zusammenstellung ein |
| **Die Folge annehmen**                | Tempo-Ausrichtungen verlieren die Abbruch-Entscheidung. Wirkzeit-Zauber werden für sie zu reinen „langsamen Zaubern" ohne die Verpflichtung, die 7.1 beschreibt                            |

##### ENTSCHIEDEN am 2026-08-06: kurze Wirkzeiten entfallen, die Spanne wird 2,3–3,0 s

**Keiner der beiden Wege oben wurde gewählt.** Der Auftraggeber hat die Frage zurückgewiesen und
einen dritten genannt:

> naja dann müssen wir doch eher die spells überarbeiten?

**Das trifft, und es ist die bessere Antwort.** Beide Wege oben drehen an einer _Regel_ — an der
Kopplung zum Talentbaum oder an der Verpflichtung selbst. Der dritte dreht an den _Zahlen_, und die
gehören ohnehin noch niemandem: Es gibt bis heute keinen einzigen spielbaren Zauber mit Wirkzeit.

Gerechnet mit der echten `hastedMs`, bei 50 % CATALYSIS (dem Deckel):

| Basis-Wirkzeit | bei 50 % Tempo | Fenster ohne Talent | mit GCD-Talent |
| -------------- | -------------- | ------------------- | -------------- |
| 1,5 s          | 1000 ms        | **−500**            | 0              |
| 2,0 s          | 1340 ms        | **−160**            | +340           |
| 2,25 s         | 1500 ms        | 0                   | +500           |
| **2,3 s**      | 1540 ms        | **+40**             | +540           |
| **3,0 s**      | 2000 ms        | **+500**            | +1000          |

**Bei 3,0 s bleibt bei vollem Tempo genau so viel Fenster wie bei 2,0 s ganz ohne Tempo.** Die
obere Grenze der neuen Spanne löst das Problem also vollständig; die untere (2,3 s) hält es gerade
eben offen.

**Die Regel, die daraus folgt:**

|                     |                                                                                                 |
| ------------------- | ----------------------------------------------------------------------------------------------- |
| **Wirkzeit-Zauber** | **2,3–3,0 s** — deutlich mehr als eine globale Wartezeit, damit die Verpflichtung aus 7.1 trägt |
| **Alles darunter**  | **Sofort-Zauber.** Was nach altem Korridor 1,5–1,9 s bekommen hätte, bekommt gar keine Wirkzeit |

> **Die Wirkzeit wird damit zum Merkmal weniger, besonderer Zauber statt zu einer Abstufung.** Ein
> Balken bedeutet ab jetzt immer: „Das hier kostet dich zwei Aktionen, und es kann dir
> abgebrochen werden."
>
> **Was das kostet, und es ist bewusst angenommen:** Zwischen „sofort" und „zwei Aktionen lang"
> liegt nichts mehr. Der Katalog verliert eine Zwischenstufe — und gewinnt dafür, dass jeder
> Balken etwas bedeutet.

**Beide Regeln oben bleiben unangetastet.** 6.6 (Wirkzeit skaliert) und 6.2 (die Wartezeit nicht)
gelten unverändert; sie stehen nicht mehr im Widerspruch, weil kein Zauber mehr in den Bereich
fällt, in dem sie einander aufheben.

---

_Der Stand vor der Entscheidung:_ Bis heute war der Prüf-Zauber aus #304 (2000 ms) die einzige
Wirkzeit im Spiel, und für ihn spielte es keine Rolle — er ist für keinen Spieler erreichbar.

### 7.2 DoT-Regel: Erneuern setzt zurück

Ein DoT läuft z. B. 5 Sekunden. Wird er innerhalb dieser Zeit erneut gewirkt, läuft er wieder volle
5 Sekunden — **die verbleibende Restzeit ist verloren**.

> Der Spieler muss **so spät wie möglich** erneuern, um nichts zu verschenken, darf aber nicht zu
> spät kommen. Das ist die klassische, härtere Variante (WoW hat sie später abgeschwächt).
> **Bewusste Entscheidung des Auftraggebers** — die Präzision ist gewollt und wird als Können belohnt.
>
> **Folge für die Anzeige:** Die Restlaufzeiten der DoTs sind damit die **wichtigste Anzeige neben
> den Abklingzeiten** und müssen entsprechend klar und gut lesbar dargestellt werden.

**Anzahl gleichzeitiger DoTs: ergibt sich aus der Ausrichtung**, nicht aus einer festen Vorgabe.
Wer sich sechs Brand-Effekte zumutet, hat sich bewusst dafür entschieden und wird dafür belohnt.

### 7.3 Zauber-Datenmodell

Ein Zauber ist ein **Datensatz**, kein Code (Leitplanke 6). Felder:

| Feld                   | Bedeutung                                                                  |
| ---------------------- | -------------------------------------------------------------------------- |
| `id`, `name`, `school` | Kennung, englischer Anzeigename, Schule                                    |
| `baseDamage`           | Grundschaden                                                               |
| `cooldownS`            | Abklingzeit in **ganzen Sekunden**                                         |
| `castTimeS`            | Wirkzeit (0 = Sofort-Zauber)                                               |
| `triggersGcd`          | true außer bei Unterbrechen                                                |
| `dot`                  | optional: `{ tickDamage, tickIntervalS, durationS }`                       |
| `resourceGain`         | was die Schul-Mechanik dazubekommt (Hitze, Kontroll-Punkte, Stapel)        |
| `synergy`              | auf welche Ereignisse der Zauber reagiert und welchen Faktor er beisteuert |
| `unlockLevel`          | ab welcher Spielerstufe verfügbar                                          |

Das genaue JSON-Format steht in [`STEP1-SPEC.md`](STEP1-SPEC.md) und wird von Spiel **und**
Rust-Simulator geteilt.

### 7.4 Die drei schulunabhängigen Sonderzauber

| Zauber           | Leisten-Platz                | GCD          | Verfügbarkeit                      |
| ---------------- | ---------------------------- | ------------ | ---------------------------------- |
| **Unterbrechen** | belegt einen Platz           | **kein GCD** | eigene Abklingzeit                 |
| **Overdrive**    | **kein Platz**, eigene Taste | —            | **einmal pro Kampf**, frei zündbar |
| **Spellsteal**   | schulunabhängig              | —            | Fenster-abhängig                   |

**Overdrive** (ersetzt „Bloodlust"): kurzer, starker Tempo-Schub. **Jede Zusammenstellung hat ihn**
→ das Zünden im richtigen Moment ist eine Fähigkeit, die jeder lernen muss, und er ist der
verlässliche Angelpunkt fürs Zusammenlegen der großen Fähigkeiten.

**Spellsteal:** Der Boss verstärkt sich mit einem sichtbaren Effekt. Wer im richtigen Fenster
stiehlt, **nimmt ihm die Verstärkung UND bekommt sie selbst** — doppelte Belohnung. Belohnt Kenntnis
des Kampfverlaufs.

**Cooldown-Rücksetzer** (SUBLIMATION, kein Sonderzauber, aber verwandt): setzt den **nächsten
gewirkten** Zauber zurück. Zieht eine Planungsebene ein: „Welchen teuren Zauber setze ich damit
zurück?"

> Abklingzeiten, Fensterbreiten und GCD-Verhalten dieser vier sind **noch nicht festgelegt** —
> siehe Kapitel 22.

### 7.5 Die zwölf Prototyp-Zauber und ihre Namen

_Namen am 2026-08-04 vom Auftraggeber freigegeben. Sie werden später die **Start-Zauber des
Spiels** — keine Wegwerfarbeit, und weil sie ohnehin die einfachsten sein müssen, passen sie
genau an den Anfang._

| Schule           | Zauber                               | Rolle                                      |
| ---------------- | ------------------------------------ | ------------------------------------------ |
| **CALCINATION**  | `KINDLE` · `SMOULDER` · `PYRE`       | zwei Hitze-Erzeuger und der Entlade-Zauber |
| **FIXATION**     | `BIND` · `STALL` · `STAY` · `ANCHOR` | Kontroll-Erzeuger und Ausgeber             |
| **COAGULATION**  | `CRUST` · `SALVE` · `TEMPER`         | Schutzschicht, Heilung, Umwandlung         |
| **SUBLIMATION**  | `VAPOUR` · `REFLUX`                  | Verkürzer und Cooldown-Rücksetzer          |
| **PUTREFACTION** | `CANKER` · `FESTER`                  | Stapel-Erzeuger und Verstärker             |
| schulunabhängig  | `QUENCH`                             | Unterbrechen                               |

**Jeder Name trägt seine Mechanik** — dasselbe Prinzip wie bei den Schul-Namen selbst (Kapitel 4):

| Name       | Bedeutung        | Warum er passt                                                                                    |
| ---------- | ---------------- | ------------------------------------------------------------------------------------------------- |
| **VAPOUR** | verdampfen       | Sublimation überspringt die flüssige Phase — der Zauber überspringt Wartezeit                     |
| **REFLUX** | Rückfluss        | Echter Laborbegriff: Kondensat fließt in den Kolben zurück. Genau das tut ein Cooldown-Rücksetzer |
| **CANKER** | Fäulnisherd      | Legt den ersten Zersetzungs-Stapel an — etwas, das sich ausbreitet                                |
| **FESTER** | eitern, schwären | Wirkt **stärker, je mehr Stapel liegen**. Der Name sagt, dass es schlimmer wird                   |

> **`CANKER` und `FESTER` gehören als Paar zusammen** — Fäulnisherd und was daraus wird. Wer die
> Bedeutung einmal liest, weiß ohne Nachschlagen, welcher aufbaut und welcher davon lebt.

**Alle einsilbig oder zweisilbig.** Ein Zaubername steht auf einem Symbol von 48 Pixeln und wird
im Kampf im Augenwinkel gelesen — er muss kurz sein.

> **Zwölf Zauber bei zwölf Leisten-Plätzen bedeutet: Man kann nicht alle mitnehmen** — genau die
> Entscheidung, die der Prototyp-Prüflauf (20.3) messen soll.

> **Nachgetragen am 2026-08-04.** Die vier neuen Namen standen zunächst **nur im Plan**, und ein
> Issue verwies auf „GAME-DESIGN 4" — wo sie nicht standen. Derselbe Fall wie beim Player Home
> (16.1) am selben Tag: Eine Festlegung, die nur an einem Ort steht, den die Umsetzer nicht lesen.

---

## 8. Eingabe und Tastenbelegung

### 8.1 Grundsatz: um die Hand herum

Das Spiel verlangt sehr schnelle Reaktionen, also bleibt die linke Hand in Ruhestellung — kein
Finger greift weit.

```
Q  W  E  R          13 Tasten
A  S  D  F      ×   2 (mit Umschalt)
Y  X  C  V      =  26 Belegungen
   Leertaste
```

| Aspekt                                 | Festlegung                                                                                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Physische Position statt Buchstabe** | Tasten werden über ihre **Position** angesprochen (`KeyboardEvent.code`), nicht über den Buchstaben. Ein deutscher Spieler sieht `Y`, ein englischer `Z` — beide drücken dieselbe Taste unter demselben Finger. **Die Beschriftung auf dem Symbol wird zur Laufzeit ermittelt**                                                                                                                                                       |
| **Umschalt**                           | Frei belegbar wie jede andere Taste, keine vorgegebene Rolle                                                                                                                                                                                                                                                                                                                                                                          |
| **Leertaste**                          | Normale belegbare Taste, keine Sonderrolle                                                                                                                                                                                                                                                                                                                                                                                            |
| **Leiste frei anordenbar**             | Der Spieler bestimmt, welcher Zauber auf welchem Platz liegt. **Muskelgedächtnis entsteht nur, wenn ein Zauber immer an derselben Stelle liegt**                                                                                                                                                                                                                                                                                      |
| **Keine Doppelbelegung**               | **Ein Zauber liegt auf höchstens einem Platz** (entschieden am 2026-08-05, Issue #287). Ein Platz ist knapp — 12 bis 16 bei 60 Zaubern —, und ihn zu verdoppeln verschenkt ihn. Die Auswahl bietet einen bereits gelegten Zauber nicht erneut an; ein Spielstand mit Doppelbelegung wird beim Laden bereinigt. **Folge für die Vorschau:** Die Zählung „wie viele Zauber zahlen je Mechanik ein" (Kapitel 16) stimmt damit von selbst |
| **Rechte Hand**                        | Im Kampf unbeteiligt. Die Maus dient nur Menüs, Talentbaum und Zusammenstellung                                                                                                                                                                                                                                                                                                                                                       |

### 8.2 Fehleingabe

**Kurz rot aufblinken + dumpfer Ton.** **Kein Vormerken der Eingabe** (keine Input-Queue) — der
Spieler lernt sofort. Eine Warteschlange würde ihm Präzisionsleistung abnehmen und die Auswertung
verfälschen.

### 8.3 Was NICHT bewertet wird

**Nur die Reihenfolge wird bewertet, nicht wie präzise am Takt gedrückt wurde.** Niemand wird für
seine Reaktionszeit getadelt. (Die Eingabe-Zeitmessung existiert nur als Entwicklungs-Werkzeug,
siehe 19.1.)

---

## 9. HUD

**Grundsatz:** Keine Spielfigur, kein Kreatur-Artwork. Das Spiel _ist_ das HUD.

| Element               | Festlegung                                                                                                                                                                                                                                                                             |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Boss**              | **Großer Kreisring** in der Bildmitte = Lebensbalken und Boss-Darstellung zugleich. Es ist das **Reaktionsgefäß**; darin läuft eine Umwandlung außer Kontrolle. Identität durch Farbe, Segmentierung, Verhalten (springt/reißt bei Phasenwechsel, wird rot und vibriert bei Überdruck) |
| **Abklingzeit**       | **Wischblende + Zahl** — dunkler Schleier wandert wie ein Uhrzeiger weg, Restsekunden als Zahl                                                                                                                                                                                         |
| **Globale Wartezeit** | **Eigene Darstellung**, deutlich unterscheidbar von der individuellen Abklingzeit                                                                                                                                                                                                      |
| **Bereit-Zustand**    | Bereite Zauber leuchten, nicht bereite sind abgedunkelt. **Keine Empfehlung**, welcher Zauber der beste wäre                                                                                                                                                                           |
| **Synergie**          | Pulsierender Rahmen am begünstigten Zauber                                                                                                                                                                                                                                             |
| **DPS-Anzeige**       | Große Zahl + Verlaufskurve + gestrichelte Bestlauf-Linie. **Keine** Schadensaufschlüsselung pro Zauber während des Kampfes                                                                                                                                                             |
| **Wucht**             | Aufsteigende und verblassende Schadenszahlen, Aufblitzen bei Treffern, leichtes Bild-Ruckeln bei großen Schlägen, Splittern des Zauberbalkens beim Unterbrechen                                                                                                                        |

### 9.1 Anordnung

| Bereich                  | Ort                          |
| ------------------------ | ---------------------------- |
| Zauberleiste             | **unten links**              |
| Reaktionsgefäß-Ring      | Bildmitte                    |
| Instrumente und Anzeigen | **rechts**                   |
| Schul-Mechanik-Anzeigen  | direkt über der Zauberleiste |
| Überdruck-Timer          | oben links                   |

**Die Leiste bildet die Tastatur nach:** obere Reihe `Q W E R`, mittlere `A S D F`, untere `Y X C V`.
**Auge und Finger lernen dieselbe Karte.** Wer das Symbol oben links sieht, weiß ohne Nachdenken,
dass die Taste oben links liegt.

> Weicht bewusst von den Referenzbildern ab — die zeigen lange Reihen mittig.

**Alle 26 Belegungen dauerhaft sichtbar** (zwei komplette Blöcke: direkte und Umschalt-Ebene).
Grund: Auch die selteneren Zauber brauchen sichtbare Abklingzeiten. Kostet Bildfläche, ist aber für
die Rotationsplanung nötig.

**Nur freigeschaltete Plätze zeigen.** Am Anfang zwei Symbole statt 26 leerer Kästen.
**Auflage:** Positionen müssen von Anfang an fest vergeben werden, damit das Wachsen bestehende
Plätze nicht verschiebt — sonst leidet das Muskelgedächtnis.

**Kennzeichnung:** Auf jedem Symbol steht schlicht **der Keybind**, auch bei Umschalt-Belegungen.
Kein Farbcode, keine Sonderzeichen-Systematik.

### 9.2 Skalierung

**Feste Anordnung, gleichmäßig skaliert.** Das HUD ist ein festes Bild mit festem Seitenverhältnis,
das als Ganzes auf die Bildschirmgröße skaliert wird (wie bei einer Spielkonsole). Die Anordnung ist
auf jedem Gerät identisch, Lesbarkeit muss nur einmal geprüft werden. Auf sehr breiten Bildschirmen
bleiben Ränder frei.

### 9.3 Farbcodierung

Nach alchemistischer Tradition (Nigredo, Albedo, Citrinitas, Rubedo) — **nicht** nach WoW-Elementen:

| Schule       | Farbe           |
| ------------ | --------------- |
| CALCINATION  | rot-orange      |
| FIXATION     | weiß-blau       |
| SUBLIMATION  | gelb-gold       |
| COAGULATION  | grün-türkis     |
| PUTREFACTION | schwarz-violett |

> Schwarz-violett braucht auf dunklem Grund sorgfältige Abstimmung.

### 9.4 Animationen — vier Parameter

Jede Bewegung ist über **Dauer, Kurve, Überschwingen, Verzögerung** exakt beschrieben:

| Bewegung      | Umsetzung                             |
| ------------- | ------------------------------------- |
| Wischblende   | rotierende Maske, Winkel 0–360°       |
| Ring          | SVG-Strichlänge                       |
| Schadenszahl  | Translation + Ausblendung über 700 ms |
| Bild-Ruckeln  | 6 px über 120 ms                      |
| Synergie-Puls | Schwingung zweier Werte               |

---

## 10. Kampfablauf

| Phase                     | Verhalten                                                                                                                                                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Vorbereitung**          | **Kein Ziel vorhanden** → alle Zauber auf den Boss sind ausgegraut. Selbst-Verstärkungen erlaubt, Abklingzeiten laufen mit. **Keine Uhr.** Schlechte Vorbereitung ist die Schuld des Spielers |
| **Bereitschafts-Prüfung** | „Ready"-Knopf → **3 s Countdown** → Start. Uhr und Eingabe beginnen gleichzeitig                                                                                                              |
| **Kampf**                 | Rotation gegen den Überdruck-Timer                                                                                                                                                            |
| **Ende**                  | Sieg · Spieler-Tod · Zeitablauf (Überdruck) · Abbruch                                                                                                                                         |

**Abbrechen:** `Esc` → Menü → **„Wipe Run"**. Gilt als Ende und zeigt **dieselbe Auswertung wie ein
Sieg**, mit „Restart Run" und „Back to Menu".

**Spieler-Tod:** Versuch vorbei, **keine Strafe**, einfach neu starten.
**Zeitablauf:** ebenfalls ohne Strafe. Der Gegner ist die Stoppuhr, nicht der Verlust.

**Wipe = von vorn.** Kein Übungsmodus für einzelne Boss-Phasen — wie im echten Raid.

### 10.1 Kampfdauer

| Boss-Typ          | Dauer              |
| ----------------- | ------------------ |
| Kleinere Bosse    | kurz               |
| Zwischenbosse     | länger             |
| Instanz-Endbosse  | **bis 10 Minuten** |
| **Endgame-Bosse** | **3–5 Minuten**    |

> Die kurze Endgame-Dauer ist Absicht: Zehn Versuche sind dann 40 Minuten statt anderthalb Stunden.
> Die 10-Minuten-Kämpfe bleiben den wenigen Instanz-Endbossen vorbehalten, wo sie als Ereignis wirken.

**Spannungsbogen bei langen Kämpfen:**

- **Eskalierende Phasen** — jede Phase legt eine Mechanik obendrauf, statt sie zu tauschen
- **Sichtbare Meilensteine** — Phasenwechsel als Bildschirm-Ereignis, Ring ändert Farbe und Form
- **Endspurt-Phase** unter 20 % — Boss schlägt härter, eigene Verstärkungen wirken stärker, Musik zieht an

Keine Zwischenphase zum Durchatmen.

---

## 11. Boss-System

### 11.1 Boss-Datenmodell

| Feld                | Bedeutung                                       |
| ------------------- | ----------------------------------------------- |
| Lebenspunkte        |                                                 |
| Schadens-Ereignisse | als **feste Zeitleiste**                        |
| Phasen-Schwellen    | Lebenspunkt-Prozente, z. B. 70 %, 40 %          |
| Überdruck-Zeitpunkt |                                                 |
| Widerstände         | je Schule                                       |
| Bausteine           | 2–4 aus dem Baukasten, mit der Stufe wachsend   |
| Signatur-Regel      | **eine Regel, die es nur bei diesem Boss gibt** |

#### Was sich an eine Phase binden lässt (Issue #557)

**Ein Zeitleisten-Ereignis, und sonst nichts.** Das Feld heißt `phase` und steht am Ereignis:

```json
{ "atMs": 36000, "mechanic": "strike", "name": "Rupture", "damage": 900, "phase": 1 }
```

| Regel                                                  | Warum so                                                                                   |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| `phase` ist eine **Untergrenze**, kein Gleichheitstest | Phasen laufen nur vorwärts; ein Ereignis für eine übersprungene Phase käme nie wieder      |
| Ohne Angabe feuert das Ereignis **immer**              | Ein bestehender Boss ändert sein Verhalten nicht                                           |
| Ein übersprungenes Ereignis **verfällt**               | Aufgestaut fiele es beim Wechsel gesammelt — aus einer lernbaren Abfolge würde eine Lawine |

**Schadenswerte und Bausteine lassen sich heute NICHT binden.** Das ist eine Grenze, keine
Auslassung: Sie fällt auf, sobald jemand einen Boss baut, der sie braucht.

**Warum das hier steht:** Bis #557 war die Phase ein Zustand ohne Wirkung. Ein Boss mit zwei
Phasenschwellen verlief bitgleich zu demselben Boss mit `phases: []` — gemessen an 1000
Durchgängen. Ein „Verhaltenswechsel", an dem sich nichts ändert, ist keiner.

**Der Ablauf ist fest und lernbar.** Der Boss reagiert **nicht** auf Spielerverhalten, und es gibt
keine zufälligen Abweichungen — Voraussetzung für Planbarkeit, Simulierbarkeit und vergleichbare
Bestzeiten.

### 11.2 Die neun Mechanik-Bausteine

Abgeleitet aus 29 gelesenen WoW-Raid-Bossen. **Kernbefund: 60–70 % aller WoW-Mechaniken sind
räumlich** und damit für uns unbrauchbar.

> **Übersetzungs-Grundsatz: Zeitfenster statt Raumfenster.** Was in WoW ein Ort ist, wird bei uns
> ein Zeitpunkt. Statt „stell dich in den Kreis" heißt es „wirke diesen Zauber innerhalb von zwei
> Sekunden". Der Spieler trifft dieselben Entscheidungen, nur in der Zeit statt im Raum.

| #   | Baustein                          | Wirkung                                                                                                                                                                    |
| --- | --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | **Energieleiste als zweite Uhr**  | Boss-Energie steigt unabhängig von den Lebenspunkten und löst bei 100 eine Mechanik aus. Gibt dem Kampf einen inneren Rhythmus, der nichts mit dem Lebensbalken zu tun hat |
| 2   | **Zähler, der bei 100 tötet**     | Ein Wert steigt unfreiwillig an; bei 100 stirbt man. Zurücksetzen nur über eine begrenzt verfügbare Handlung                                                               |
| 3   | **Exponentieller Stapel**         | 1 Stapel = X, 2 = 3X, 3 = 9X. Erzwingt eine Handlung an einem exakt bestimmten Punkt. **Muss gut angekündigt sein**                                                        |
| 4   | **Wettrennen gegen einen Zauber** | Der Boss wird unverwundbar, während etwas anderes vor Ablauf zerstört werden muss. **Der beste Moment für Overdrive**                                                      |
| 5   | **Fehler heilt den Boss**         | Ein Fehler macht Fortschritt rückgängig. Härteste Form von Konsequenz. **Nur für einzelne Bosse**, sonst frustrierend                                                      |
| 6   | **Umgekehrtes Ziel**              | Statt Schaden zu machen muss die kippende Reaktion **stabilisiert** werden — **am Boss-Ring selbst**, nicht an einem zweiten Wert. Der Kampf, in dem COAGULATION glänzt    |
| 7   | **Eskalierende Wiederholung**     | Dieselbe Phase kehrt mehrfach wieder — kürzere Fenster, mehr gleichzeitige Anforderungen                                                                                   |
| 8   | **Schwächung ohne Heil-Lösung**   | Ein Effekt lässt sich nicht wegheilen, sondern nur durch eine bestimmte **Handlung** loswerden. **Braucht mehrere Lösungswege**                                            |
| 9   | **Unterbrechbarer Zauber**        | Boss beginnt einen Zauber, Balken läuft. Auf niedriger Schwierigkeit optional, auf hoher Pflicht                                                                           |

> **Baustein 6 im Einzelnen** (entschieden am 2026-08-05, Issue #247): Er sitzt am **Boss-Ring**,
> nicht an einem zweiten Wert daneben. Der Ring **ist** die Reaktion (9.1) — ein zweiter Wert wäre
> eine Dublette, und erst dadurch wird die Umkehrung zur Umkehrung.
>
> |                   | Ring auf 0                      | Zeit abgelaufen               |
> | ----------------- | ------------------------------- | ----------------------------- |
> | gewöhnlicher Boss | **Sieg**                        | Niederlage (Überdruck tötet)  |
> | Baustein 6        | Niederlage (die Reaktion kippt) | **Sieg** (sie ist eingedämmt) |
>
> **Beide Bedingungen kehren um.** Das ist kein Zuviel, sondern der Baustein: Wer das Ziel
> umkehrt, kehrt zwangsläufig um, was daran hängt. Der Bildschirmtext „REACTION CONTAINED"
> beschreibt genau das — eingedämmt, nicht zerstört.
>
> Das Datenfeld heißt `runaway`, nicht `stabilise`: Der Fachbegriff beschreibt die **Mechanik**
> (die Reaktion läuft davon), nicht die Antwort des Spielers darauf. Schaden senkt den Ring
> weiter; **Schutz und Heilung** heben ihn, um ihren eigenen Betrag aus den Zauberdaten. Die
> Schaden-in-Schutz-Umwandlung bleibt außen vor — sie würde die Bedingung aufheben, die den
> Kampf ausmacht.
>
> **BEDINGUNG AN JEDEN Boss mit `runaway` (gemessen, Issue #394):** Er braucht einen
> Stabilisierer, der **ohne eingehenden Schaden** wirkt — sonst ist die Hälfte des Weges
> wirkungslos, und niemand sieht es.
>
> Von den beiden Wegen greift nur einer bedingungslos:
>
> | Weg                   | wirkt                              | Bedingung                                      |
> | --------------------- | ---------------------------------- | ---------------------------------------------- |
> | **Schutz** (`shield`) | um seinen vollen Betrag            | nur beim **Stapeln**, nicht beim Erneuern      |
> | **Heilung** (`heal`)  | um `min(Betrag, fehlende Anzeige)` | **nur, wenn der Spieler Schaden genommen hat** |
>
> Wer einen `runaway`-Boss ohne Spielerschaden baut — leere `timeline` —, macht die Heilung dort
> zur Nullhandlung: Es fehlt nie etwas, also hebt sie nie. Bei `test-runaway` stabilisiert
> `salve` bei **jedem** Intervall exakt 0. Der Kampf bleibt gewinnbar, aber nur über `crust`,
> und ein Spieler mit der falschen Zusammenstellung findet keinen Weg und keinen Hinweis.
>
> Die Prüfung dazu steht in `tests/unit/runaway-beatable.test.ts` — sie fällt, wenn ein solcher
> Boss unschaffbar wird.

Die restlichen 6–11 Bausteine entstehen als **Varianten dieser neun** (z. B. unterbrechbarer Zauber
mit Schild davor / zwei gleichzeitig, nur einer schaffbar / unterbrechen macht ihn beim nächsten
Mal stärker). Noch auszuarbeiten — siehe Kapitel 22.

**Aufbau jedes Bosses: drei bis vier Bausteine plus eine Signatur, die sich in einem Satz erklären
lässt.**

### 11.3 Mehrere Antworten je Mechanik

**Kein Boss verlangt eine bestimmte Schule.** Ein Boss-Zauber lässt sich **unterbrechen**, ODER durch
**FIXATION verlangsamen**, ODER durch **COAGULATION aushalten**. Jede Zusammenstellung hat einen
Weg, nur unterschiedlich elegante.

### 11.4 Widerstände

| Aspekt           | Festlegung                                                                                                                                                                                                    |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stärke**       | ca. **20–30 %** weniger von der widerstandenen Schule, 20–30 % mehr von der Schwachstelle                                                                                                                     |
| **Wirkung**      | **Nur auf den Schaden, nicht auf die Mechanik.** Wer trotz Widerstand CALCINATION spielt, sammelt weiter normal Hitze — macht damit aber weniger Schaden. Keine Schule fühlt sich je kaputt an, nur schwächer |
| **Sichtbarkeit** | **Vorher in der Boss-Übersicht.** Die Zusammenstellung ist bewusste Vorbereitung, keine böse Überraschung                                                                                                     |

> **„Es muss möglich sein, nicht nötig."** Wer bei seiner Lieblingsschule bleibt, kommt durch —
> nur langsamer.

#### Der Widerstand hängt am ZAUBER, nicht am Kampf — präzisiert am 2026-08-05

**Bis heute war die Frage nicht gestellt, weil es nur eine Schule gab.** Seit die Zauber ihre eigene
Schule tragen (#270) und die Leiste frei aus allen fünf befüllt wird, sind es zwei verschiedene
Dinge — und die Umsetzung hatte die falsche gewählt:

```
src/core/combat.ts:621   boss.resistances[options.school.school]   die Schule des KAMPFES
```

**Gemessen, was das bedeutet** (Worker-3, Balance-Läufer über 150 Listen je Boss):

```
test-resistant   Widerstand 0,75   gefunden 23340,0   pyre / smoulder / kindle
test-vulnerable  Widerstand 1,25   gefunden 38900,0   pyre / smoulder / kindle

23340 / 0,75 = 31120        38900 / 1,25 = 31120       DIESELBE Liste
```

Zwei Bösse, die sich um Faktor 1,67 unterscheiden, und beide Male dieselbe Rotation. **Ein Faktor
auf allen Spielerschaden kann keine Reihenfolge ändern** — er skaliert das Ergebnis und lässt die
Wahl unberührt. Das gilt für jede Zahl, die man einsetzt.

**Es gilt die Schule des gewirkten Zaubers.** Das ist die einzige Lesart, unter der die Sätze dieses
Kapitels etwas bedeuten:

| Stelle                | Was sie voraussetzt                                                                                                                                  |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| 11.4, Zeile „Wirkung" | „Wer **trotz Widerstand** CALCINATION spielt, macht weniger Schaden" — setzt voraus, dass jemand mit einer anderen Schule **nicht** weniger macht    |
| Leitplanke 2          | „Jeder Zauber muss irgendwo die beste Wahl sein" — ein Faktor, der alle gleich trifft, trägt dazu nichts bei                                         |
| 4.2, freie Mischung   | Die Leiste wird aus allen Schulen befüllt. Ein Widerstand, der die Zusammenstellung nicht berührt, ist keine Vorbereitung, sondern eine Zahl am Ende |

**Schulunabhängige Zauber (`school: null`) trifft kein Widerstand.** QUENCH steht außerhalb der
Schulen; das ist dieselbe Sonderstellung, die ihm auch die globale Wartezeit erlässt.

> **Was das dem Spieler gibt:** Gegen einen kalzinationsfesten Boss lohnen sich PUTREFACTION und
> COAGULATION — nicht weil das Spiel es verlangt, sondern weil es sich rechnet. Genau der
> „freiwillige Zwang", den der Auftraggeber als Grund für die Widerstände genannt hat.

### 11.5 Ankündigungen — eingebaut statt per Zusatzprogramm

In WoW nutzt praktisch jeder Raid-Spieler DBM oder BigWigs. **Bei ROTA gehört das ins Spiel selbst.**
Dann haben alle dieselbe Grundlage, und die Schwierigkeit lässt sich ehrlich darauf abstimmen.

**Der entscheidende Befund aus der Recherche:** Der Balken zeigt **nicht ein Warnfenster, sondern die
volle Abklingzeit** der Boss-Fähigkeit. Eine Fähigkeit mit 45 s Abstand hat einen durchgehend
laufenden 45-s-Balken. **Das macht aus Reagieren Planen.**

**Drei Eskalationsstufen:**

| Stufe                 | Darstellung                                                                       | Wofür                                      |
| --------------------- | --------------------------------------------------------------------------------- | ------------------------------------------ |
| **1 — Balken läuft**  | Zeitbalken über die volle Abklingzeit, durchgehend sichtbar                       | Alles; die Grundinformation zum Planen     |
| **2 — Hervorhebung**  | Balken blinkt, springt an eine auffälligere Stelle, Klang in den letzten Sekunden | Fähigkeiten, die hart treffen              |
| **3 — Höchste Stufe** | Gesprochener Countdown 5-4-3-2-1, Bildschirm blitzt und ruckelt, eigener Klang    | **Ausschließlich** für tödliche Mechaniken |

> **Wenn alles gleich laut warnt, hört man nichts mehr.** Stufe 3 muss selten bleiben.

**Vier Warn-Arten:**

| Art                            | Bedeutung                                                              | Behandlung                                                                       |
| ------------------------------ | ---------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| **Schwächung auf dir**         | Trifft nur den Spieler, verlangt persönliche Reaktion                  | Eigene Farbe und eigener Klang, sofortiges Handeln                               |
| **Allgemeiner Schaden**        | Trifft ohne Zutun; man kann nur vorher heilen oder schützen            | Ruhigere Ankündigung                                                             |
| **Jetzt handeln oder sterben** | Unterbrechen sonst Wipe; Schutzschild jetzt oder Tod                   | Volle Behandlung: Countdown, Blitz, eigener Klang                                |
| **Gelegenheit**                | Schadensfenster öffnet sich, Verstärkung stehlbar, Schwäche verwundbar | Muss sich klar von den Warnungen abheben — **hier entscheidet sich der Schaden** |

**Vier Kanäle**, je nach Ereignis unterschiedlich kombiniert — _nicht alles ist bei allem angebracht_:

1. **Zeitleiste der nächsten Ereignisse** — Balken mit Symbol und Restzeit
2. **Textmeldung mit Countdown** in der Bildmitte — benennt die Fähigkeit
3. **Eigener Klang je Ereignisart** — man hört die Warnung, ohne hinzusehen. Bei einem Spiel, dessen
   Kern das Drücken ist, **der wertvollste Kanal**
4. **Hervorhebung am Reaktionsgefäß** — Ring färbt sich, pulsiert, zeigt einen aufziehenden Bogen

**Weitere Festlegungen:**

| Punkt                                                    | Festlegung                                                                                                                      |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Warnungen auf allen Schwierigkeitsstufen vollständig** | Die Härte kommt aus mehr Lebenspunkten, engeren Fenstern und zusätzlichen Mechaniken — **nicht aus vorenthaltener Information** |
| **Fähigkeiten werden benannt**                           | „Cascade Failure in 5" statt „Danger in 5". Namen sind das, woran man sich an einen Boss erinnert                               |
| **Nur die nächste Wiederholung**                         | Kein Zähler „noch 3×" — das wäre nur eine Schätzung. **Ausnahme:** begrenzte Zwischenphasen zeigen die Restzahl                 |
| **Keine Erklärungen im Kampf**                           | Die Ankündigung nennt nur den Namen. Einzelheiten stehen vorher in der Boss-Übersicht                                           |
| **Mehrere gleichzeitige Ereignisse**                     | Nach **Dringlichkeit sortiert** — das Tödliche oben und lauter, das Nebensächliche gedämpft oder kurz unterdrückt               |

### 11.6 Boss-Übersicht (Vorbild: WoW Adventure Guide)

| Element                            | Festlegung                                                                                                                              |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| **Vollständige Fähigkeiten-Liste** | Name, Wirkung, Abklingzeit, mögliche Antworten — dazu Widerstände, Phasen-Schwellen und die Signatur-Regel                              |
| **Keine Zeitangaben**              | Nur Phasen und Reihenfolge, **keine Sekunden**. Die Zeitpunkte muss man erspielen                                                       |
| **Keine Empfehlungen**             | Nur Fakten. Das Zusammenstellen ist eine Kernaufgabe                                                                                    |
| **Markierungen je Fähigkeit**      | **unterbrechbar · tödlich wenn ignoriert · Gelegenheit · stehlbar · verlangt Schutz** — dieselben Kategorien wie die Warnungen im Kampf |
| **Eigene Vorgeschichte**           | Versuche, Bestzeit je Schwierigkeitsstufe, Fortschrittsmarke, erreichte Tempo-Stufe, offene Erfolge, fehlende Beute                     |

### 11.7 Schwierigkeitsstufen

**Normal / Heroisch / Mythisch** je Boss: mehr Leben, härteres Zeitlimit, zusätzliche Mechaniken,
mehr Punkte. Mit 68 Bossen ergibt das **204 Herausforderungen**.

#### Die Schwierigkeit gehört zur INSTANZ, nicht zur Spielerstufe

_Entschieden am 2026-07-29, nachdem der Auftraggeber die Frage stellte, wie sich Normal und Mythisch
zu den Stufen 1–100 verhalten._

Instanz 1 auf Mythisch ist für **Stufe ~12** gedacht und wird dort gespielt — härter als Normal, aber
im selben Stufenbereich. Vorbild ist WoWs klassisches Muster: Dungeon, dann heroischer Dungeon, beide
auf der Stufe, für die sie gebaut sind.

**Der Widerspruch, den das auflöst:** Zwei bereits getroffene Entscheidungen ließen sich sonst nicht
beide erfüllen —

| Regel                                                                   | Folgt daraus                                                                  |
| ----------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| Der Ertrag sinkt mit dem Stufenabstand, bei ~20 Stufen praktisch nichts | Instanz 1 Mythisch muss **zeitnah** gespielt werden, sonst wirft es nichts ab |
| „Ab Höchststufe beginnt das eigentliche Spiel: mythische Schwierigkeit" | Mythisch wäre eine **Endgame**-Sache                                          |

Spielte man Instanz 1 Mythisch erst mit Stufe 100, wäre der Ertrag null — die Stufe existierte
formal, aber niemand spielte sie.

**„Mythisch ist Endgame" ist damit keine Aussage über die Spielerstufe, sondern über die Instanzen:**
Das Mythisch der sechs Endgame-Instanzen ist das Schwerste im Spiel, weil sie selbst das Schwerste
sind — nicht, weil die Schwierigkeitsstufe an eine Spielerstufe gebunden wäre.

Das passt zur bereits beschlossenen Beute-Regel: **Beute auf der Gegenstandsstufe des Bosses.** Wer
Instanz 1 auf Mythisch mit Stufe 12 spielt, bekommt Gegenstände, die zu Stufe 12 passen.

---

## 12. Fortschritt

### 12.1 Stufenaufstieg

Eine neue Stufe gibt **alles vier**:

| Belohnung                       | Wirkung                                                                                                                                           |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Neue Zauber freischalten**    | Zauber sind an Stufen gebunden → vollständig steuerbare Lernkurve                                                                                 |
| **Talentpunkte**                | Stetiger, spürbarer Fortschritt                                                                                                                   |
| **Grundwerte erhöhen**          | Aufstieg ist sofort spürbar                                                                                                                       |
| **Leisten-Plätze freischalten** | Start mit **zwei**, schrittweise bis 12–16. **Löst das „easy to learn"-Problem elegant:** Ein Anfänger jongliert zwei Abklingzeiten, kein Dutzend |

### 12.2 Einstieg — die ersten Minuten

| Aspekt                  | Festlegung                                                                                                                                                                                                |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Erster Eindruck**     | **Sofort drücken, Erklärung nebenbei.** Kein Menü, kein Vorspann — nach dem Start steht man vor dem Werktisch, ein Symbol leuchtet, eine Zeile sagt „Press Q". Die Reaktion antwortet sichtbar und hörbar |
| **Start-Ausstattung**   | **Zwei Zauber, zwei Leisten-Plätze**                                                                                                                                                                      |
| **Wachstum**            | Bis Stufe 10 drei weitere Zauber, danach etwa alle drei Stufen einer                                                                                                                                      |
| **Einführungs-Instanz** | **Drei kurze Lern-Kämpfe**: (1) erster Zauber und Abklingzeit, (2) globale Wartezeit und zweiter Zauber, (3) erste Synergie. Kein Zeitdruck, kein Boss-Schaden. **Überspringbar**                         |

### 12.3 Neue Zauber: gelenkter Zufall

Erste 2–3 Zauber in fester Reihenfolge (Einführung), danach nach jedem Boss **„Wähle 1 aus 3"** aus
gewichteten Töpfen. Frühe Bosse ziehen nur einfache Zauber. Defensiv-Zauber kommen aus **demselben
Topf**; die Gewichtung erhöht sie, wenn der Spieler noch keine hat.

### 12.4 Endgame

**Drei Ebenen:**

1. **Acht Aufstiegs-Instanzen** (32 Bosse) tragen Stufe 1–100
2. **Sechs Endgame-Instanzen zu je sechs Bossen** (36 Bosse) — mit **neuen Mechaniken, die es im
   Aufstieg gar nicht gibt**: mehrere Unterbrechungen gleichzeitig, Phasen mit umgekehrten Regeln,
   Zauber die bei falscher Reihenfolge bestrafen. Der Sprung fühlt sich wie ein neues Spiel an
3. **Endlos-Leiter** darauf aufgesetzt

**Endlos-Leiter:**

| Aspekt         | Festlegung                                                                                                                                                                                                                                                |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Zugang**     | **Als Stufe auf jedem Endgame-Boss** — jeder der 36 Bosse ist nach Mythisch auf Leiter-Stufe 1, 2, 3 … spielbar. Höchststufe wird je Boss geführt                                                                                                         |
| **Obergrenze** | **Werte gedeckelt, Stufen unbegrenzt.** Ab einer bestimmten Leiter-Stufe wachsen Gegenstandsstufen und Spielerstärke nicht mehr — ab da wird es reines Können. **Ein echter, nie erreichbarer Horizont.** Muss dem Spieler verständlich vermittelt werden |
| **Fiktion**    | **Tiefer und tiefer in den Apparat** — ältere, fremdartigere Reaktionen, die dort schon lange vor dem Spieler liefen                                                                                                                                      |

### 12.5 Talentbaum

**Fünf Schul-Bäume + globaler Baum + Zauber-Feinschliff.** Die Schul-Bäume **vertiefen die jeweilige
Mechanik**, nicht nur Werte. Der globale Baum wirkt auf CATALYSIS/PURITY/YIELD/VESSEL/GCD.
Zusätzlich ein bis zwei wählbare Verbesserungen **direkt am ausgerüsteten Zauber**.

| Aspekt                                     | Festlegung                                                                                                                                                                               |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Punkte**                                 | **~50 bei 100 Stufen.** Damit lassen sich zwei Bäume tief ausbauen und ein dritter antippen — **niemals alles**                                                                          |
| **Struktur je Schul-Baum**                 | **Gemeinsamer Stamm** → **drei Wege** (bei CALCINATION etwa: schneller aufbauen / stärker entladen / länger halten). Am Ende jedes Weges ein **Schlüsseltalent**                         |
| **Schlüsseltalente bauen die Rotation um** | z. B. „Der Entlade-Zauber verbraucht nur die Hälfte der Hitze, wirkt dafür zweimal". **Ein neues Schlüsseltalent fühlt sich an wie eine neue Schule** — genau das, was 300 Stunden trägt |
| **Ansicht**                                | **Alles auf einem Bildschirm** — eine große Fläche mit allen sechs Bäumen, frei verschiebbar und zoombar                                                                                 |
| **Umskillen**                              | **Jederzeit kostenlos** zwischen den Kämpfen. Kostet ausdrücklich KEINE Punkte — das Spiel ist ein Optimierungs-Labor                                                                    |

**Vorlagen:** Zusammenstellung **UND** Talentverteilung werden **gemeinsam** in einer Vorlage
gespeichert (z. B. „Boss 7 mythic"). Beides gehört zusammen; getrennt zu speichern wäre eine
Fehlerquelle.

### 12.6 Punkte

| Aspekt                 | Festlegung                                                                                                                                                                                                                                                                     |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Quelle**             | Restlaufzeit beim Boss-Sieg, **überproportional** umgerechnet — die letzten Sekunden sind unverhältnismäßig viel wert, damit Feinoptimierung sich lohnt. **Die Kurve ist quadratisch** (entschieden am 2026-08-05): `round(2 × Sekunden²)`, Werte in `data/player/reward.json` |
| **Wiederholen**        | Punkte für **Bestzeit-Verbesserung** (Differenz-Auszahlung) plus ein Rest-Ertrag fürs bloße Wiederholen — **20 Promille** der vollen Auszahlung (entschieden am 2026-08-05): 50 Wiederholungen wiegen einen ersten Sieg auf                                                    |
| **Abnehmender Ertrag** | Der Ertrag sinkt **mit dem Stufenabstand**: voller Ertrag solange der Boss zur eigenen Stufe passt, bei ~10 Stufen Abstand nur noch ein Bruchteil, bei ~20 praktisch nichts. Lenkt sanft zum passenden Inhalt, **ohne etwas zu verbieten**                                     |
| **Verwendung**         | **Freischaltungen** (Instanzen, einzelne Zauber, Ausrüstungsplätze) und **Gegenstände aufwerten**                                                                                                                                                                              |

> **Warum quadratisch und nicht flacher oder steiler.** Drei Kurven standen zur Wahl, gemessen
> am Verhältnis eines sehr guten Laufs (45 s Rest) zu einem mittelmäßigen (20 s Rest):
>
> | Exponent            | 5 s    | 20 s    | 50 s      | sehr gut ÷ mittelmäßig |
> | ------------------- | ------ | ------- | --------- | ---------------------- |
> | moderat 1,6         | 105    | 965     | 4.183     | 3,66×                  |
> | **quadratisch 2,0** | **50** | **800** | **5.000** | **5,06×**              |
> | kubisch 3,0         | 6      | 400     | 6.250     | 11,39×                 |
>
> **Die Begründung ist die Mitte:** Feinoptimierung lohnt deutlich, ein mittelmäßiger Lauf ist
> nicht wertlos. Bei 1,6 lohnt die Feinoptimierung zu wenig — und genau dafür ist die Kurve da.
> Bei 3,0 spielt umsonst, wer die letzten Sekunden nicht holt.

> **Warum 20 Promille für die Wiederholung.** Der Rest-Ertrag soll das Wiederholen tragen, ohne
> den ersten Sieg zu entwerten: Bei 20 Promille wiegen **fünfzig** Wiederholungen einen ersten
> Sieg auf. Wer grinden will, darf; es lohnt sich nur zunehmend weniger — genau das Muster, das
> der abnehmende Ertrag über den Stufenabstand ohnehin verfolgt.

### 12.7 Instanz-Freischaltung

Vier Kriterien **nebeneinander**:

| Kriterium                    | Einsatz                                                                                       |
| ---------------------------- | --------------------------------------------------------------------------------------------- |
| **Instanz abgeschlossen**    | Hauptpfad                                                                                     |
| **Punkte-Schwelle**          | **Ventil gegen Feststecken** — wer scheitert, kommt durch Perfektionieren alter Kämpfe weiter |
| **Schwierigkeitsstufe**      | Heroisch öffnet nach Normal, Mythisch nach Heroisch                                           |
| **Meisterschafts-Schlüssel** | Nur für optionale Geheim-Instanzen. **Nie im Hauptpfad**                                      |

Innerhalb einer freigeschalteten Instanz sind die Bosse **frei wählbar**.

### 12.8 Frust-Vermeidung

**Viele Versuche ≠ frustrierende Versuche.** Ein Raid-Trupp scheitert hundertmal und bleibt dabei,
weil **jeder Versuch etwas einbringt**. Frustrierend wird es, wenn zwanzig Versuche gleich aussehen.

| Mechanismus                     | Ausgestaltung                                                                                                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Fortschrittsmarke je Boss**   | „Best attempt: reaction down to 23 %". Jeder Versuch, der die Marke unterbietet, ist sichtbar ein Schritt weiter — auch ohne Sieg. **Getrennt je Boss UND Schwierigkeitsstufe** |
| **Kürzere Versuche im Endgame** | 3–5 Minuten                                                                                                                                                                     |
| **Konkrete Fehleranalyse**      | Die drei größten Verlustquellen mit Auswirkung in Prozent (siehe 15.2). Versuch 20 ist dann klüger als Versuch 5, nicht nur häufiger                                            |

> **Keine Belohnung bei Niederlage.** Ein gescheiterter Versuch zahlt nichts aus.
> **Folge, die im Blick bleiben muss:** Damit fehlt das Sicherheitsventil, sich über Ausrüstung an
> einem zu harten Boss vorbeizuschieben. **Punkte-Schwellen und niedrigere Schwierigkeitsstufen
> müssen deshalb verlässlich offen bleiben.**

### 12.9 Erfolge

**Boss-eigene Erfolge** (auf die jeweilige Mechanik zugeschnitten, z. B. „jeden Zauber unterbrochen")
**plus gestaffelte Tempo-Stufen** (Bronze/Silber/Gold-Zeitmarken), je Schwierigkeitsstufe erneut
auszahlbar. Alle mit Punkten.

**Erfolgs-Kriterien sind absolut formuliert** („ohne Schaden zu nehmen", „mit über 50 % Restzeit",
„ohne Heilzauber") — nicht relativ zu einem Optimum.

**Prüflauf vor Auslieferung:** Der Simulator läuft über jede Kombination aus Boss × Schwierigkeit ×
Erfolg und bestätigt, dass eine Lösung existiert. **Ohne gefundene Lösung gilt der Boss als nicht
auslieferbar.** Als automatischer Test formuliert.

---

## 13. Ausrüstung und Beute

### 13.1 Zwei Fortschritts-Systeme mit getrennten Rollen

| System         | Rolle                                                                              |
| -------------- | ---------------------------------------------------------------------------------- |
| **Talentbaum** | **WIE** die Rotation funktioniert — neue Effekte, kürzere Abklingzeiten, Synergien |
| **Ausrüstung** | **WIE STARK** die Werte sind                                                       |

### 13.1b Wie sich Werte verrechnen — vier Töpfe

_Entschieden am 2026-07-29 nach Recherche, wie Diablo IV und WoW es tatsächlich machen._

**Das Prinzip: Innerhalb eines Topfes wird addiert, zwischen den Töpfen multipliziert.**

| Topf                          | Inhalt                                        | Verrechnung                                      |
| ----------------------------- | --------------------------------------------- | ------------------------------------------------ |
| ESSENCE                       | alle Quellen des Grundwerts                   | additiv, dann **ein** Faktor                     |
| Sonderwerte                   | CATALYSIS, PURITY, YIELD                      | **je eigener** Faktor                            |
| Aktive Verstärkungen          | Overdrive, Synergien, Zersetzungs-Stapel      | additiv untereinander                            |
| **Ausrüstungs-Sondereffekte** | **legendäre Effekte UND Satz-Boni gemeinsam** | **additiv untereinander**, ein Faktor nach außen |

**Warum die Sonderwerte je einen eigenen Faktor bekommen** (WoW-Muster): Weil sie sich
multiplizieren, will man alle drei mischen statt einen zu stapeln. Das ist der eigentliche
Designwert — nicht „mehr Zahl", sondern **keine Monokultur**.

**Warum legendäre Effekte und Satz-Boni in DENSELBEN Topf gehören:** Ein Sechser-Bonus von 30 % und
ein legendärer Effekt von 20 % ergeben 50 %, nicht 56 %. Damit konkurrieren sie fair, und die
Mischung wird zur echten Abwägung statt zur Frage, was sich besser multipliziert.

> **Die Lektion aus zwei Spielen.** In Diablo IV waren Kritschaden und Verwundbarkeit ursprünglich
> eigene multiplikative Töpfe. Sie wurden „disproportional stark", jeder Build lief über sie, und
> Blizzard verschob sie in Saison 2 in den additiven Topf zurück. Diablo 3 verlor dieselbe Schlacht
> bei den Sätzen: Die Sechser-Boni wurden über Jahre hochgezogen, bis **legendäre Gegenstände ohne
> Satz-Bonus schlicht ignoriert wurden** — von Spielern wie von Entwicklern.
>
> **Was multiplikativ ist, wird gejagt — und alles andere wird liegen gelassen.** Deshalb wenige,
> klar benannte Multiplikatoren, und deshalb Sätze und Legendäre im selben Topf.

**Die Zahlen stehen ausdrücklich noch nicht fest.** Sie werden nicht festgelegt, sondern **gemessen**:
Der Balance-Läufer vergleicht den besten Satz-Aufbau gegen den besten Affix-Aufbau je Ausrichtung
(siehe 13.5, harte Balance-Anforderung).

**Verhältnis zur Schadensformel:** Die sechs Faktoren im Kampf-Kern (`essence`, `talents`, `buffs`,
`synergy`, `resistance`, `crit`) bleiben unverändert — sie laufen seit E1 und tragen alle
Prüfsummen. Die vier Töpfe beschreiben, **wie Ausrüstungs-Werte in diese Faktoren einfließen**, nicht
eine zweite Formel daneben. Ob eine Vereinheitlichung sich lohnt, entscheidet eine Messung.

#### Anzeige: additiv und multiplikativ sind unterscheidbar

Wie in Diablo, wo `[+]%` und `[x]%` am Gegenstand stehen: **Multiplikative Werte bekommen eine eigene
Schreibweise und Farbe.** Ohne diese Kennzeichnung kann ein Spieler nicht erkennen, warum ein
Gegenstand viel besser ist als ein scheinbar gleichwertiger — und müsste es außerhalb des Spiels
nachschlagen. Das ist dieselbe Linie wie bei den Boss-Ankündigungen: Was man zum Spielen wissen muss,
steht im Spiel.

### 13.2 Zehn Ausrüstungsplätze

Bewusst **keine Körperteile**, weil es keine Spielfigur gibt — arkane Artefakte:

`Focus · Catalyst · Sigil · Relic · Grimoire · Shard · Talisman · Phylactery · Anchor · Prism`

**Die Plätze sind Behälter, sonst nichts** — wie in WoW und Diablo. Ein gefundenes Stück gehört auf
seinen Platz; welche Werte es trägt, hängt an **Seltenheit und Gegenstandsstufe**, nicht am Platz.

> **Gestrichen am 2026-07-29: die platz-spezifische Wertezuordnung.**
>
> Hier stand bis dahin eine feste Zuordnung („Focus und Catalyst immer CATALYSIS, Sigil und Talisman
> immer PURITY/YIELD …") samt der Begründung „gezieltes Grinden statt blinden Sammelns".
>
> **Sie war nie mit dem Auftraggeber besprochen.** Auf seine Rückfrage hin durchsucht:
>
> ```
> $ mainrag search "Ausruestungsplaetze Fokus Katalysator Siegel platz-spezifische Sonderwerte" \
>       --source claude-conversations      →  No results
> $ mainrag search "Fokus Katalysator Siegel Reliquie Grimoire"   →  No results
> ```
>
> Keine Fundstelle. Die Regel ist beim Schreiben des Plans entstanden, mit einer Begründung, die
> plausibel klang und deshalb nie hinterfragt wurde. Zwei der zehn Plätze (Relic, Shard) bekamen nie
> eine Zuordnung, und zwei der vier genannten Kategorien („Verteidigung", „Zauber-Verstärkung")
> waren gar keine definierten Werte — beides fiel erst auf, als E6-1 die Tabelle bauen sollte.
>
> **Merksatz:** Eine Festlegung, die niemand angeordnet hat, sieht im Dokument genauso aus wie eine,
> die entschieden wurde. Wer sie später liest, kann beides nicht unterscheiden.

### 13.3 Qualitäten

Ein Gegenstand trägt **immer die drei Grundwerte** ESSENCE, VESSEL und WARD — jeder Fund verbessert
also spürbar etwas. **Dazu** kommen Affixe, deren Zahl mit der Seltenheit steigt:

| Qualität      | Affixe | Besonderheit                                                                                                |
| ------------- | ------ | ----------------------------------------------------------------------------------------------------------- |
| Common        | 1      | —                                                                                                           |
| Rare          | 2      | —                                                                                                           |
| Epic          | 3      | —                                                                                                           |
| **Legendary** | 3+     | **Echter Sondereffekt, der die Rotation verändert** (z. B. „Der Cooldown-Rücksetzer wirkt auf zwei Zauber") |

> **Präzisiert am 2026-07-29.** Die Spalte hieß „Werte-Plätze", was so gelesen werden konnte, als
> zählten die Grundwerte mit — dann hätte ein Common-Gegenstand **einen** Wert insgesamt, und ein
> Platz könnte keine drei Grundwerte tragen. Gemeint sind **Affixe**, wie in WoW und Diablo: Die
> Grundwerte kommen zusätzlich, so wie ein Diablo-Gegenstand immer Rüstung trägt.

**Werte-Berechnung: feste Formel, Qualität als Faktor.** Aus Gegenstandsstufe und Qualität errechnet
sich ein Wertebudget, das auf die Werte-Plätze verteilt wird. **Kein Gegenstand wird von Hand
balanciert** — ein neuer Gegenstand ist ein Eintrag mit Stufe, Qualität und Werte-Arten.

**Keine Werte-Streuung.** Kein „perfektes Exemplar"-Glücksspiel.

#### Woher die drei Grundwerte ihre Höhe bekommen

> **AUSGELEGT vom Orchestrator am 2026-08-05, nicht vom Auftraggeber entschieden.** Der Merksatz aus
> 13.2 gilt auch hier: Eine Festlegung, die niemand angeordnet hat, sieht im Dokument genauso aus wie
> eine, die entschieden wurde. Deshalb steht dieser Kasten davor.

Die Frage kam bei #282 auf, wo gemessen wurde, dass ESSENCE, VESSEL und WARD im Kampf gar nicht
ankommen: `BASE_STAT_IDS` steht auf beiden Seiten und **wird nirgends gelesen**. Kein vergessener
Anschluss, sondern ein nie gebauter — und beim Bauen stellte sich heraus, dass die Höhe nirgends
festgelegt ist.

**Was das Dokument bereits hergibt:**

| Frage                                  | Antwort                                           | Fundstelle  |
| -------------------------------------- | ------------------------------------------------- | ----------- |
| Hängt die Höhe am **Platz**?           | **Nein**, ausdrücklich — die Plätze sind Behälter | 13.2        |
| Hängt sie an der **Gegenstandsstufe**? | Ja                                                | 13.2, 13.3  |
| Steuert die **Qualität** sie mit?      | **Nein — sie steuert die Zahl der Affixe**        | siehe unten |

**Die dritte Zeile ist die Auslegung.** 13.2 sagt „hängt an Seltenheit und Gegenstandsstufe", 13.3
sagt „die Grundwerte kommen zusätzlich, so wie ein Diablo-Gegenstand immer Rüstung trägt". Beides
zusammen ist nicht eindeutig. Gelesen wird es so:

- **13.2 spricht über die Affix-Arten** — welche Werte ein Stück trägt. Die drei Grundwerte sind
  ohnehin immer dieselben, dort gibt es nichts zu wählen.
- **In Diablo skaliert die Rüstung mit der Gegenstandsstufe**; die Qualität gibt Affixe, nicht mehr
  Rüstung. Genau dieser Vergleich steht in 13.3.

**Die Kurve wird nicht neu erfunden.** Sie steht bereits in den Stufendaten und lässt VESSEL des
Spielers wachsen (`LEVELS.factorPerMille`, `statAtLevel`). Für Gegenstände gilt dieselbe Kurve mit
der **Gegenstandsstufe** als Eingang statt der Spielerstufe — der Unterschied ist wesentlich:

> Die Gegenstandsstufe kommt vom **Inhalt**, nicht vom Spieler (13.2: „Die Schwierigkeit gehört zur
> Instanz, nicht zur Spielerstufe"). Hinge sie an der Spielerstufe, bekäme ein Spieler auf
> Höchststufe in einer frühen Instanz plötzlich brauchbare Funde — genau die Entwertung, die der
> abnehmende Ertrag verhindern soll.

**Was ausdrücklich NICHT ausgelegt ist: die Basiswerte selbst.** Wie viel ESSENCE ein Gegenstand auf
Stufe 1 trägt, ist eine reine Balance-Zahl. Sie gehört gemessen, nicht gesetzt — der Balance-Läufer
weist Skalierungsfaktoren je Wert aus. Bis dahin steht sie als benannte, ausdrücklich unkalibrierte
Konstante an einer Stelle; siehe `docs/ENTSCHEIDUNGEN-OFFEN.md`.

### 13.4 Fallraten

| Regel                                | Wirkung                                                                                                                                                                                                             |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Obergrenze durch Inhalt**          | Frühe Instanzen lassen nur Common und Rare fallen; Epic erst ab einer bestimmten Instanz, Legendary erst im Endgame. **Verhindert, dass ein früher Glückstreffer die restlichen zwanzig Aufstiegsstufen entwertet** |
| **Steigende Raten darin**            | Innerhalb der erlaubten Qualitäten steigen die Raten mit Instanz, Schwierigkeitsstufe und Endlos-Stufe                                                                                                              |
| **Richtwerte im Endgame**            | Common 60 %, Rare 30 %, Epic 9 %, **Legendary 1 %**                                                                                                                                                                 |
| **Zwei getrennte Achsen**            | **Qualität** und **Gegenstandsstufe** laufen unabhängig. Ein episches Stück aus einer frühen Instanz ist schwächer als ein seltenes aus dem Endgame — alter Inhalt entwertet sich von selbst                        |
| **Kein Pech-Schutz, keine Garantie** | Eine ehrliche Prozentzahl, kein versteckter Zähler                                                                                                                                                                  |
| **JEDER Sieg lässt etwas fallen**    | 1000 ‰ — entschieden am 2026-08-06. Die Seltenheit liegt in der **Qualität**, nicht im **Ob**                                                                                                                       |

**Ein Sieg geht nie leer aus.** Die Begründung des Auftraggebers ist der Vergleich, der auch die
Bauart trägt: _„in wow gibt es doch auch keine bosse die nichts droppen."_ Dort fällt bei jedem
Boss etwas — selten ist, **was** es ist.

> Genau so steht es hier schon: Der Normalfall ist ein **gewöhnliches** Stück, das über die
> Materialumwandlung nicht wertlos ist; Epic und Legendary bleiben bei 9 % und 1 %. Es war
> deshalb **eine Zahl** (`dropChancePerMille` in `data/items.json`), keine zweite Mechanik.
>
> **Was das für die Beute-Inszenierung bedeutet** (13.6): Sie läuft jetzt bei jedem Sieg. Deshalb
> ist die Staffelung dort keine Zierde, sondern die Bedingung — ein gewöhnlicher Fund erscheint
> beiläufig, und nur der legendäre hält alles an.

> **Bekannte, bewusst akzeptierte Folge (der lange Schwanz):** Bei 1 % Legendary haben nach 100
> Siegen noch 37 % nichts, nach 300 noch 5 %. Die Minderheit im Schwanz erlebt ein anderes Spiel.
> **Die Materialumwandlung ist die einzige Abfederung.**

### 13.5 Sätze — nur im Endgame

- Während des Aufstiegs nur Einzelstücke
- **Drei Bonus-Stufen: zwei, vier und sechs Teile** — der Sechser-Bonus als Krönung
- **Frei kombinierbar**: Satz-Teile passen auf beliebige Plätze, gezählt wird nur die Anzahl. Damit
  lassen sich zwei Sätze mischen (vier von A und zwei von B)
- Werte-Boni auf den unteren Stufen, **echte Rotations-Effekte** auf den höheren. Jeder Satz zielt
  auf eine Schule → auch ungenutzte Zauber bekommen ihre Nische

> **Harte Balance-Anforderung:** Satz-Boni und **Affixe** müssen **etwa gleichwertig** sein. Kippt es
> in die eine Richtung, tragen alle nur Sätze; kippt es in die andere, sind die Sätze wertlos.
> Eigener Prüflauf im Simulator (bester Satz-Aufbau gegen besten Affix-Aufbau je Ausrichtung).
>
> Gegenüber steht den Sätzen also die **Affix-Auswahl** aus 13.3 — drei Affixe auf einem Epic gegen
> zwei Satz-Teile, das ist die Abwägung, die der Prüflauf messen muss.

> **Nachgezogen am 2026-08-04.** Hier stand „Satz-Boni und **platz-spezifische Sonderwerte**". Die
> platz-spezifische Wertezuordnung ist am 2026-07-29 gestrichen (13.2) — der Begriff blieb an dieser
> Stelle stehen und beschrieb damit einen Gegenspieler, den es nicht mehr gibt. Die Anforderung
> selbst ist unverändert: Sätze gegen Affixe.
>
> Gefunden hat es Worker-1, nicht ich. Wer 13.5 gelesen hätte, ohne 13.2 zu kennen, hätte den
> Prüflauf gegen eine gestrichene Regel gebaut.

#### Der Preis des Satz-Bonus ist die Wahlfreiheit

_Vom Auftraggeber entschieden am 2026-08-04, nachdem Worker-1 die Lücke beim Bau der Messung
gemeldet hat._

**Ein Satz-Teil trägt dieselben Affixe wie ein freies Stück derselben Qualität.** Es wird nichts
abgezogen. Der Preis ist ein anderer:

> Wer sechs Satz-Teile trägt, kann auf diesen sechs Plätzen **nicht die besten Einzelstücke
> tragen**. Er nimmt, was der Satz an Affixen mitbringt, statt gezielt zu wählen.

**Warum die Frage gestellt werden musste:** Ein Satz-Teil ist ein Gegenstand — es trägt Affixe
**und** zählt für den Satz. Ohne einen Preis käme der Bonus obendrauf, und „Satz tragen" wäre
immer richtig; die Anforderung „etwa gleichwertig" wäre unerfüllbar, ganz gleich wie die Zahlen
gesetzt sind.

**Was das für den Prüflauf bedeutet:** Der Satz-Aufbau darf seine Affixe **nicht frei wählen** —
sie sind durch die getragenen Teile vorgegeben. Nur der Affix-Aufbau wählt frei. Ein Vergleich,
in dem beide Seiten optimal verteilen, misst den Preis nicht und fällt zwangsläufig zugunsten
der Sätze aus.

> **Bewusst nicht gewählt:** weniger Affixe je Satz-Teil. Das wäre leichter zu balancieren und im
> Kopf nachrechenbar — ließe Satz-Teile beim Finden aber schwächer wirken als andere Stücke
> derselben Qualität, und das widerspricht dem Sechser-Bonus als Krönung (13.5).
>
> **Bekannte Folge, die der Prüflauf im Auge behalten muss:** Der Nachteil hängt am Beute-Glück.
> Wessen Satz-Teile zufällig passende Affixe tragen, zahlt kaum etwas — wessen Teile danebenliegen,
> zahlt viel. Der Vergleich misst also eine Spanne, keinen festen Abstand.

### 13.6 Beute-Handhabung

| Aspekt                            | Festlegung                                                                                                                                                                                                                                                               |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Beute-Moment gestaffelt**       | Common erscheint sofort und beiläufig, Rare mit kurzem Aufblitzen, Epic mit deutlichem Effekt — **ein Legendary hält alles an**: eigener Klang, Bild verdunkelt sich, Gegenstand erscheint groß. 99 von 100 Siegen laufen schnell durch, der eine Moment brennt sich ein |
| **Automatischer Vergleich**       | Jeder Fund zeigt sofort, ob er besser ist als das Getragene — mit ungefährer Schadensangabe aus den **Skalierungsfaktoren des Balance-Läufers**. Die Bewertung ist eine **Näherung** und kann bei ungewöhnlichen Zusammenstellungen danebenliegen                        |
| **Überflüssige Beute → Material** | Nicht behaltene Gegenstände werden **automatisch** in Aufwertungs-Material umgewandelt. Kein Inventar, kein Sortieren                                                                                                                                                    |
| **Aufwertung**                    | Material und Punkte heben die **Gegenstandsstufe** an — **höchstens bis zur Stufe, die im aktuellen Inhalt ohnehin fällt.** Grinden macht nie stärker als der Inhalt hergibt                                                                                             |
| **Aufwertung ist stückgebunden**  | Hängt am einzelnen Gegenstand, nicht am Platz. **Bekannter Nachteil:** Jeder bessere Fund entwertet die Investition — das kann das Ausprobieren bremsen. Im Auge behalten                                                                                                |
| **Gezielte Jagd**                 | Bestimmte Gegenstände fallen **nur bei einem bestimmten Boss**. Jeder Boss bekommt einen eigenen Grund, wiederholt zu werden                                                                                                                                             |

**Beute aus altem Inhalt:** ja, **aber auf der Gegenstandsstufe des Bosses**. Wer ihm entwachsen ist,
bekommt Übertroffenes. Der Grind versiegt von selbst, ohne künstliches Verbot.

---

## 14. Bestenliste

**Nur lokal**, aber so gebaut, dass eine geteilte Liste später angeschlossen werden kann (Server +
Datenbank + Nachweis über Protokoll-Nachsimulation).

**Kein Manipulationsschutz**, solange die Liste lokal ist — wer seinen eigenen Spielstand fälscht,
betrügt nur sich selbst. Die Vorbereitung für später ist getroffen: Das Kampfprotokoll erlaubt
serverseitiges Nachrechnen jeder eingereichten Zeit.

---

## 15. Auswertung nach dem Kampf

### 15.1 Ablauf

**Sieg-Inszenierung:** Die Reaktion bricht sichtbar und hörbar zusammen (Ring zerspringt, Bild
ruckelt, Klang), **danach werden die gefallenen Gegenstände einzeln aufgedeckt**. Erst dann die
Zahlen.

> **Auflage: vollständig überspringbar.** Beim hundertsten Sieg will man direkt zu den Zahlen.
> **Eine Taste muss die ganze Inszenierung abkürzen.**

Dann die **Kurzfassung**: Erfolg/Misserfolg, Restzeit, Punkte, drei Kennzahlen — mit den Knöpfen
„Retry" und „View Analysis". Details nur auf Wunsch.

**Die drei Kennzahlen:**

- **Leerlauf pro Zauber** — wie viele Sekunden lag jeder Zauber ungenutzt bereit (nach Verlust
  sortiert). **Nur Zauber, die auf der Leiste liegen** (präzisiert am 2026-08-05, Issue #306): Ein
  Zauber auf keinem Platz kann nicht ungenutzt bereitliegen — er ist nicht wirkbar, und die Zeile
  wäre eine Falschaussage. Bei 60 Zaubern und 12–16 Plätzen stünden sonst über vierzig Zeilen mit
  `0.0 s` in der Liste und verdünnten die Sortierung nach Verlust, die derselbe Satz verlangt
- **Verlorene Zeit gesamt** — Sekunden und Prozent ohne aktiven Zauber
- **Synergie-Ausbeute** — wie viel Prozent der möglichen Kombinationen genutzt wurden

### 15.2 Fehleranalyse — aus den eigenen Daten gerechnet

**Keine Referenzdaten, keine Kombinatorik, kein Simulator im Browser.** Der Verlust wird aus den
Daten des Durchlaufs berechnet — es sind Multiplikationen, alle Werte stehen bereits im
Kampfprotokoll.

| Fehlerart               | Rechnung                                                                                                                          | Ausgabe an den Spieler                                     |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| **Verpasste Wirkungen** | Ein Zauber zählt als verpasst, wenn er **bereit war** UND der Spieler **in diesem GCD-Fenster nichts gewirkt hat** × Schadenswert | „Five casts missed — 21,000 damage left on the table, 9 %" |
| **Leerlauf**            | Sekunden, in denen **kein Zauber bereit war**, ÷ GCD × Durchschnittsschaden je Aktion                                             | „6.4 s idle — four wasted actions, 7 %"                    |
| **Verpasste Synergien** | Verfügbar minus genutzt × Synergie-Bonus                                                                                          | „Seven of ten combinations used — 5 %"                     |
| **Verlorene DoT-Ticks** | Zu früh erneuert → verschenkte Restlaufzeit × Schaden je Tick                                                                     | „Twelve ticks lost — 4,800 damage, 6 %"                    |

**Diese vier teilen sich eine Rangliste und eine gemeinsame Prozentrechnung.** Sie messen alle
dasselbe: Schaden, den der Spieler **nicht ausgeteilt** hat.

#### Verpasste Unterbrechungen stehen getrennt

_Entschieden am 2026-08-04, nachdem E7-1 den Größenunterschied gemessen hatte._

| Fehlerart                     | Rechnung                                          | Ausgabe an den Spieler                       |
| ----------------------------- | ------------------------------------------------- | -------------------------------------------- |
| **Verpasste Unterbrechungen** | Anzahl × Schaden des durchgekommenen Boss-Zaubers | „Two interrupts missed — 6,000 damage taken" |

**Eigene Zeile, eigene Zahl, NICHT in derselben Prozentrechnung.** Der Grund ist keine Feinheit:

> Die vier oberen messen **nicht ausgeteilten** Schaden. Diese misst **eingesteckten**. Beides in
> einen Prozentsatz zu werfen vergleicht Unvergleichbares — und die Zahl gewinnt immer.
>
> **Gemessen an einem Testboss:** 2.100 ausgeteilt, „6.000 verloren", Anteil 74,1 %. Der Spieler
> liest „du hast 74 % liegen gelassen" — dabei hat er 6.000 Schaden **genommen** und ist gestorben.
> Das ist schlimmer als Schadensverlust, aber es ist nicht dasselbe.
>
> **Und es ist kein Randfall:** Ein Boss-Zauber (2.000) übersteigt den Spielerschaden je Wirkung
> (100) um das Zwanzigfache. Diese Quelle stünde fast immer oben — unabhängig davon, wie gut die
> Rotation war. Damit stünde der Spieler wieder vor dem Problem, das die Gelegenheits-Regel lösen
> sollte: Der wichtigste Hinweis handelte vom Unterbrecher.

Bis zum 2026-08-04 stand sie als fünfte Zeile in derselben Tabelle, mit dem Beispiel
„Two missed — 14 %". Die Rechnung selbst ändert sich **nicht** — nur ihr Platz in der Ausgabe.

> **Korrektur vom 2026-07-29 — warum „Kampfzeit ÷ Abklingzeit" nicht trägt.**
> Die frühere Rechnung nahm das Maximum je Zauber einzeln: 240 s ÷ 12 s = 20 mögliche
> Wirkungen. Die Summe dieser Maxima übersteigt aber die Zahl der Aktionen, die es im Kampf
> überhaupt gibt — die Zauber teilen sich den GCD. **Gemessen an einem Prüfdurchlauf: 193
> theoretische Maxima gegen 160 verfügbare Aktionen**, also 33 Wirkungen, die es nicht geben kann.
>
> Der Spieler bekam dadurch eine Messlatte hingehängt, die niemand erreichen kann („KINDLE 92 von
> 160, 68 verpasst" — obwohl er dafür alle anderen Zauber hätte weglassen müssen), und die größte
> Verlustquelle war systematisch zu hoch ausgewiesen.
>
> **Verworfen wurde die rechnerisch genaueste Lösung:** die Summe der Maxima auf die verfügbaren
> Aktionen zu normieren. Ein so verteiltes Budget kommt einer Aussage darüber nahe, wie man hätte
> spielen sollen — genau die Grenze aus 21.2.
>
> **Die jetzige Definition braucht keine Referenz.** Beide Bedingungen stehen im Kampfprotokoll,
> und sie kann nicht überzeichnen: Jedes leere GCD-Fenster zählt nur einmal. Wer statt Zauber A
> Zauber B gewirkt hat, hat nichts verloren — er hat gewählt, und Wählen ist das Spiel.
>
> Gefunden hat es Worker-3 beim Bau von #81, statt die Rechnung ungeprüft zu übernehmen.

> **Verlorene DoT-Ticks, ergänzt am 2026-07-29.** Die Zahl wird längst gemessen
> (`combat.lostDotTicks`, im Prüfdurchlauf 14 bei der schlechten Rotation) und fließt in die
> Zustands-Prüfsumme ein — sie fehlte nur in der Auswertung.
>
> Sie ist ein **eigenständiger** Verlust ohne Überschneidung mit dem Leerlauf: Wer einen Brand zu
> früh erneuert, hat **gehandelt**, nur falsch. Kein leeres GCD-Fenster, keine Doppelzählung.
>
> Und sie gehört gezeigt, weil das Design die harte Erneuerungs-Regel ausdrücklich als Präzision
> will, die als Können belohnt wird — dann muss ihr Verlust auch sichtbar sein. In Schritt 1 ist
> sie der **zweitgrößte** Fehler (9,9 % laut STEP1-SPEC § 4).

> **Leerlauf und verpasste Wirkungen schließen einander aus, ergänzt am 2026-08-04.**
>
> Beide Quellen zählten bisher denselben Fehler: Wer einen bereiten Zauber nicht wirkt, erzeugt ein
> leeres GCD-Fenster — das erschien einmal als „verpasste Wirkung" und einmal als „Leerlauf". Die
> Prozentzahlen addierten sich damit auf mehr als den tatsächlichen Verlust.
>
> **Die Trennung verläuft an der Frage, ob überhaupt etwas bereit war:**
>
> | Fall                                       | Zählt als             | Was der Spieler daraus lernt                |
> | ------------------------------------------ | --------------------- | ------------------------------------------- |
> | Ein Zauber war bereit, wurde nicht gewirkt | **verpasste Wirkung** | „du hättest wirken können"                  |
> | **Kein** Zauber war bereit                 | **Leerlauf**          | „du hast dich in eine Wartezeit manövriert" |
>
> Zwei verschiedene Fehler mit zwei verschiedenen Lehren — und jeder erscheint genau einmal.

#### Situative Zauber: gezählt wird die Gelegenheit, nicht die Bereitschaft

_Entschieden am 2026-08-04, nachdem der Fall beim Bau von E4-2 auffiel._

Ein **Unterbrecher** liegt fünfzehn Sekunden bereit und wird nie gewirkt — weil der Boss keinen
unterbrechbaren Zauber hat. Nach der Bereitschafts-Regel stünde er damit als **größte**
Verlustquelle ganz oben, und der Spieler bekäme als wichtigsten Hinweis „wirke QUENCH öfter",
während es nichts zu unterbrechen gab.

**Nicht die Bereitschaft zählt, sondern die ungenutzte Gelegenheit:**

| Zauber-Art       | Verlust zählt, wenn                                        |
| ---------------- | ---------------------------------------------------------- |
| Schaden          | er bereit war und das GCD-Fenster leer blieb — wie bisher  |
| **Unterbrecher** | ein unterbrechbarer Boss-Zauber lief **und durchkam**      |
| **Defensiv**     | ein Schlag traf, den Schutz oder Heilung abgefangen hätten |

Das deckt sich mit den bereits vorgesehenen Quellen: „Verpasste Unterbrechungen" zählt denselben
Fehler, nur richtig herum. Ohne diese Regel würde er **doppelt** erscheinen.

> **Die Regel gilt vorausschauend, nicht nur für den bekannten Fall.** Der naheliegende Fix wäre
> gewesen, Unterbrecher aus der Leerlauf-Liste zu nehmen. Das löst genau einen Fall — und sobald
> Schutzschilde existieren (sie entstehen gerade in E3-4), taucht derselbe Fehler bei ihnen wieder
> auf und wird als neuer Fund gemeldet.
>
> **Voraussetzung:** Das Kampfprotokoll muss die Boss-Gelegenheiten ausweisen — welcher Zauber lief,
> welcher Schlag traf. Heute tut es das nicht; das ist Teil dieser Arbeit und keine Nebensache.

**Die drei größten Verlustquellen** werden hervorgehoben. Sie können aus der Rotation stammen
**oder aus dem Umgang mit dem Boss** — bei einem Boss mit Mechaniken ist das oft der teurere Teil.

> Was damit entfällt: Das Spiel kann **nicht** sagen „die optimale Reihenfolge wäre A-B-C gewesen".
> Es sagt stattdessen „du hast diese vier Dinge liegen gelassen, zusammen 30 %" — für den Spieler
> die nützlichere Aussage, weil umsetzbar.

### 15.3 Ansichten

- **Effizienz-Tabelle** — Wirkungen gegen theoretisches Maximum, Leerlauf, Synergie-Quote,
  Schadensanteil. **Effizienz statt Schadensanteil** ist die aussagekräftige Größe: Der
  Schadensanteil ist bei jedem Durchlauf nahezu gleich und misst die Zauber, nicht das Können.
  Er bleibt als _eine_ Spalte, weil er für Talent-Entscheidungen nützlich ist
- **Vergleich mit Bestlauf** — zwei Kurven übereinander, Kennzahlen mit Differenz
- **Verlauf über alle Durchläufe** — Entwicklung von Schaden und Leerlauf über die Zeit

### 15.4 Datenhaltung

Alle Durchläufe dauerhaft im Browser-Speicher, **plus Export und Import als Datei** (Gerätewechsel
und Datensicherung ohne Server).

### 15.5 Trainingsraum

**Puppe mit einstellbaren Bedingungen** — unbegrenztes Leben, kein Zeitlimit, jederzeit abbrechbar.
Einstellbar: **Widerstände**, **ob sie Schaden macht**, **ob sie Zauber wirkt** (zum Üben des
Unterbrechens).

**Auswertung: dieselbe wie nach einem Boss-Kampf, plus laufende Anzeige** der drei Kennzahlen
**während** des Übens. Man sieht sofort, ob man sich verbessert — das beschleunigt das Lernen
erheblich.

**Wechseln der Zusammenstellung: nur zwischen Durchgängen**, nicht mitten im Üben.

---

## 16. Zusammenstellungs-Ansicht

Links die eigenen Zauber nach Schule gruppiert, rechts die Leiste.

**Live-Vorschau** zeigt beim Bestücken die entstehenden Synergien und wie viele Zauber je
Schul-Mechanik einzahlen — man erkennt sofort, ob die Hitze-Mechanik mit nur einem Zauber trägt.

**Keine Empfehlungen vom Spiel.** Das Zusammenstellen ist eine der beiden Kernaufgaben.

### 16.1 Das Player Home — der Bildschirm zwischen den Kämpfen

_Nachgetragen am 2026-08-04. Der Begriff stand vorher nur in der Überschrift von 19.2 und war
nirgends erklärt — siehe die Anmerkung am Ende dieses Abschnitts._

**Das Player Home ist ein Spieler-Bildschirm**, nicht ein Werkzeug-Bereich. Er ist der Ort, an dem
alles geschieht, was zwischen zwei Kämpfen zu entscheiden ist:

| Was dort sitzt             | Kapitel |
| -------------------------- | ------- |
| Talentpunkte verteilen     | 12.5    |
| Instanz und Boss wählen    | 11.6    |
| Die Zusammenstellung bauen | 16      |
| Ausrüstung anlegen         | 13      |
| **Die Werte-Übersicht**    | 6.7     |

Die vier **Test-Einstellungen** aus 19.2 sitzen zusätzlich dort — sie sind ein Aufsatz für den
Auftraggeber, kein Bestandteil des Bildschirms. 19.3 schneidet **sie** beim Bauen heraus, nicht
das Player Home.

> **Woran sich das entscheidet:** Der Bildschirm bleibt im Auslieferungs-Bau vollständig
> vorhanden. Verschwindet er, verliert der Spieler seinen Talentbaum und seine Ausrüstung —
> und 6.7 begründet die Werte-Übersicht ausdrücklich mit dem Spieler: „Ein Deckel, den man
> nicht sieht, ist eine Falle." Eine Falle, vor der nur der Auftraggeber gewarnt wird, ist
> keine Warnung.

> **Warum dieser Abschnitt nachgetragen werden musste.** „Player Home" kam im ganzen Dokument
> genau einmal vor: in der Überschrift „19.2 Für den Auftraggeber — sichtbar im Player Home".
> Wer nur dieses Dokument liest — und genau das sollen die Worker —, konnte den Begriff nur
> als Werkzeug-Bereich verstehen und musste aus 19.3 schließen, dass er beim Bauen verschwindet.
> Gemeldet von E7 beim Beginn von #188, mit Beleg statt Vermutung.
>
> **Die allgemeine Lehre:** Ein Begriff, der nur in einer Überschrift vorkommt, ist nicht
> definiert — er ist benannt. Wer ihn zum ersten Mal liest, leitet seine Bedeutung aus dem
> Kapitel ab, in dem er steht, und das kann das falsche sein.

---

## 17. Klang, Grafik, Hilfe

### 17.1 Klang

**Umfang V1: ~115 Sounds**, jeder der 60 Zauber einzeln vertont.

| Bereich                                                                                                                                                                                 | Anzahl |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ |
| Zauber (je einzeln)                                                                                                                                                                     | ~60    |
| Sonderzauber (Unterbrechen, Overdrive, Rücksetzer, Schild auf, Schild bricht, Heilung, Reflexion)                                                                                       | 7      |
| Kampf-Rückmeldung (Zauber bereit, Fehleingabe, Synergie verfügbar, GCD-Ende)                                                                                                            | 4      |
| Boss (Zauberbeginn, durchgekommen, Treffer, Phasenwechsel, Überdruck-Warnung, Überdruck, Reaktion kippt, besiegt)                                                                       | 8      |
| Kampfzustände (Countdown, Start, wenig Leben, Tod, Zeitablauf, **Wipe Run**)                                                                                                            | 6      |
| Nach dem Kampf (Sieg, Niederlage, Bestzeit, Erfolg, Stufenaufstieg, Punkte)                                                                                                             | 6      |
| Menü (Berühren, Drücken, Reiter, Fenster auf, Fenster zu, Talentpunkt, Talente zurück, Gegenstand angelegt, Zusammenstellung gespeichert, abgelehnt, **Freischaltung**, **Aufwertung**) | 12     |
| **Beute nach Qualität**                                                                                                                                                                 | **4**  |
| Klangteppich: 5 feste Schichten (Menü, Spannung, Endspurt, Phasenwechsel, Überdruck) plus **ein Grundklang je Instanz**                                                                 | 5 + 14 |

> **Zu den drei zuletzt benannten Klängen** (2026-07-27): Der Planungs-Plan nannte bei
> Kampfzuständen die Zahl 6 und bei Menü die Zahl 12, listete aber nur 5 bzw. 10 Einträge — ein
> Zählfehler aus dem Interview. Die drei fehlenden sind aus dem Design abgeleitet und ergänzt:
>
> | Klang             | Warum genau dieser                                                                                                                                                                                                                                |
> | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
> | **Wipe Run**      | GAME-DESIGN 12.8 verlangt einen reibungslosen Wiederholungs-Ablauf (Esc → Wipe Run → Auswertung → Restart). Bei ~10 Versuchen je Endgame-Boss ist das der **häufigste Übergang im ganzen Spiel** und der einzige Kampf-Ausgang ohne eigenen Klang |
> | **Freischaltung** | Punkte für eine Instanz, einen Zauber oder einen Ausrüstungsplatz ausgeben (12.6, 12.7)                                                                                                                                                           |
> | **Aufwertung**    | Material und Punkte in Gegenstandsstufe umsetzen (13.6) — im Endgame der Kern-Kreislauf                                                                                                                                                           |
>
> **Freischaltung und Aufwertung sind die beiden einzigen Verwendungen von Punkten** und hatten
> beide keinen Klang. „Vorlage geladen" braucht dagegen keinen eigenen — es teilt sich den Klang
> mit „Zusammenstellung gespeichert", weil es dieselbe Handlungsklasse ist.

> **Die vier Beute-Klänge sind einer der stärksten Dopamin-Momente überhaupt** — der legendäre Klang
> in Diablo ist berühmt, Spieler erkennen ihn, bevor sie den Gegenstand sehen. Hier lohnt sich Sorgfalt.

> **Korrektur der Slot-Zahl (2026-07-27):** Der Plan nannte hier **8** — eine Schätzung aus der Zeit,
> bevor der Umfang auf **14 Instanzen** festgelegt war (8 passen zu drei Instanzen plus den fünf
> festen Schichten). Der echte Bedarf ist **19**: fünf feste plus 14 Instanz-Grundklänge.
>
> **Die Trennung verläuft entlang der Abhängigkeit:**
>
> |                                                                          | Anzahl | Wann                                                                       |
> | ------------------------------------------------------------------------ | ------ | -------------------------------------------------------------------------- |
> | **Feste Schichten** — Menü, Spannung, Endspurt, Phasenwechsel, Überdruck | 5      | **jetzt**, in Epic E8. Sie hängen an Kampf-Ereignissen, nicht an Instanzen |
> | **Instanz-Grundklänge**                                                  | 14     | mit **E10**, wo Thema, Farbwelt und Klangteppich je Instanz entstehen      |
>
> Damit steigt die Gesamtzahl der Klänge von 115 auf **126**.
>
> **Offen für E10:** Ob die 14 Grundklänge eigenständige Stücke werden oder Varianten der fünf
> Prozess-Schwerpunkte (17.4 ordnet jeder Instanz einen Prozess zu). Das entscheidet sich, wenn die
> Instanz-Themen stehen — nicht vorher.

**Musik: Schichten, die auf den Kampf reagieren.** Ein Grundklang je Instanz, dazu zuschaltbare
Schichten — Endspurt, Phasenwechsel, Überdruck. Deutlich haltbarer als eine feste Schleife, die bei
hunderten Wiederholungen zur Qual wird.

| Technik               | Festlegung                                                                                                                                                                         |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Wiedergabe**        | Web Audio API — latenzfrei auf Tastendruck, `loopStart`/`loopEnd` für Sustained-Sounds                                                                                             |
| **Lautstärke**        | **Getrennt regelbar** für Musik, Effekte und Oberfläche. Viele hören nebenbei eigene Musik und würden sonst den Ton ganz abschalten — und damit die klanglichen Hinweise verlieren |
| **Tonhöhen-Streuung** | Wenige Prozent, zufällig, bei jeder Wiedergabe. Verhindert den Maschinengewehr-Effekt. Bei tonalen Klängen je Klang begrenzen                                                      |
| **Zwei Sound-Typen**  | **One-Shot** (feste Länge) und **Sustained** — letztere **dreiteilig**: Ansatz (One-Shot) → Körper (Loop) → Ausklang (One-Shot), weil der Effekt abgebrochen werden kann           |

**Produktion:** Suno, Werkzeugkette steht und ist verifiziert — SSOT [`suno-sfx.md`](suno-sfx.md).

#### Was die Klang-Produktion kostet (gemessen, 2026-07-28)

Aus Epic E8, für die Planung von E10:

|                                                               |                                                                                                            |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 56 Klänge kosteten                                            | rund **2.500 Suno-Credits** — etwa **45 je Klang**, inklusive Nachbesserungen und verworfener Erstversuche |
| Die verbleibenden **71** (57 Zauber + 14 Instanz-Grundklänge) | rund **3.200 Credits**, eher mehr                                                                          |
| Monatskontingent im Pro-Tarif                                 | **2.500**, setzt sich monatlich zurück                                                                     |

> **E10 ist dadurch nicht blockiert, aber es passt nicht in eine Abrechnungsperiode.** Entweder über
> zwei Monate verteilen oder aufladen. Bei den Instanz-Grundklängen ist mit mehr Aufwand zu rechnen:
> Loops haben eine niedrigere Trefferquote als One-Shots.
>
> **Vor jedem größeren Lauf den Stand prüfen** — `GET /api/billing/info/` liefert
> `total_credits_left`. Nicht `credits` lesen: Das ist ein anderes Feld und führte am 28.07. zu der
> Fehlmeldung, das Guthaben stehe bei null, obwohl 2.372 verfügbar waren.

**Ästhetische Abnahme macht der Auftraggeber** über ein Abhör-Soundboard; Claude kann nicht hören
und prüft nur technisch (ffprobe: Länge, Pegel, Anfangs-Stille, Clipping).

#### Kein Klang verzögert irgendetwas

_Entschieden am 2026-08-04, als Antwort auf eine Frage, die enger gestellt war._

**Alle Klänge müssen zeitgleich spielen können.** Kein Klang wartet auf einen anderen, keiner
verdrängt einen laufenden, und **keiner verzögert eine Handlung im Spiel** — weder einen Zauber noch
eine Boss-Aktion noch einen Zustandswechsel.

> Die Frage lautete, welcher von zwei gleichzeitigen Klängen weichen soll. Die Antwort war: keiner.
> Ein Klang ist eine **Meldung über** das Spiel, nie ein Teil davon. Sobald die Wiedergabe einen
> Ablauf beeinflussen kann — auch nur um einen Takt —, ist der Gleichlauf mit dem Simulator dahin,
> denn der hat keine Audio-Schicht.

Praktisch heißt das: Wiedergabe immer nebenläufig, nie abwartend; ein Klang, dessen Kanal belegt ist,
bekommt einen neuen statt zu warten; und die Zahl gleichzeitiger Klänge ist eine Frage der
Lautstärke-Mischung, nicht der Reihenfolge.

#### Die drei offenen Schaltpunkte

_Entschieden am 2026-08-04. Sie standen als Design-Fragen aus E8 und blockierten das Epic._

| Klang                         | Wann                                                                                                               |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| **Spannungs-Schicht** (Musik) | **Zwei Auslöser**: unter 20 % Boss-Leben **oder** wenn das Zeitlimit ins letzte Viertel geht — was zuerst eintritt |
| **Ende der Wartezeit**        | **Abschaltbar, standardmäßig aus** — ein Metronom als Lernhilfe, nicht als Grundzustand                            |
| **Überdruck + Zeitablauf**    | **Beide, gleichzeitig**, als ein Klangbild abgestimmt (siehe Regel oben)                                           |

> **Warum die Spannungs-Schicht zwei Auslöser hat:** Der Endspurt unter 20 % ist bereits ein
> beschlossenes Ereignis (10.2) — der Spieler lernt keinen neuen Schaltpunkt. Aber bei einem Lauf,
> der gut läuft, käme sie nie, und dann fehlte der Höhepunkt genau bei den besten Durchgängen. Der
> zweite Auslöser warnt außerdem vor dem, was den Spieler tatsächlich tötet.
>
> **Warum das Metronom standardmäßig aus ist:** Bei einem Zehn-Minuten-Kampf sind es rund 400
> Klänge. Als dauerhafter Grundzustand nutzt sich das ab; als einschaltbare Lernhilfe ist es
> wertvoll, weil der Spieler den Grundtakt über das Ohr lernt und den Blick für die Abklingzeiten
> frei behält.

### 17.2 Grafik

**Eingestandene Grenze:** Claude kann keinen eigenen visuellen Stil hervorbringen. Referenzbilder
erkennen: ja. Selbst gestalten: nein.

**Reihenfolge:**

1. **Fertige freie Assets zuerst** — der Stil kommt von einem menschlichen Gestalter
2. **KI-Generierung nur für Lücken**
3. **Selbst zeichnen: nie**

**Bau-Reihenfolge des HUDs:**

1. **Zuerst das HUD als SVG-Gerüst** — Aufteilung, Geometrie und alle Bewegungen. Sofort spielbar und prüfbar
2. **Danach die erzeugten Grafiken als Texturen und Rahmen darüberlegen**

**Lizenzdisziplin: nur CC0 und CC BY.** **„Free for personal use" ist ausgeschlossen** — kostenlos,
aber kommerziell nicht nutzbar; die häufigste Falle, weil der Knopf trotzdem „Free Download" heißt.
[`ASSETS.md`](../ASSETS.md) führt jede Datei mit Herkunft, Lizenz und Nennungspflicht.

#### Gewählter Stil: GEÄTZT (Entscheidung des Auftraggebers, 2026-07-27)

Ausgewählt aus vier echten Kandidaten-Sets, die in Issue #19 nebeneinander vorgelegt wurden.

| Baustein    | Quelle                                                                                               | Lizenz    |
| ----------- | ---------------------------------------------------------------------------------------------------- | --------- |
| **Symbole** | [game-icons.net](https://game-icons.net) — einfarbige Silhouette in der Schulfarbe auf dunklem Eisen | CC BY 3.0 |
| **Textur**  | ambientCG Metal036 (geschwärztes Eisen)                                                              | CC0       |
| **Schrift** | Cinzel (Überschriften) + EB Garamond (Fließtext)                                                     | OFL       |

**Warum GEÄTZT:**

- **Vektor** — bei jeder Bildschirmgröße scharf, kein Nachschärfen nötig. Das HUD wird als Ganzes
  skaliert (9.2), also ist das keine Feinheit, sondern Voraussetzung.
- **Die Schulfarbe steckt im Symbol selbst**, nicht nur im Rahmen. Farbcodierung nach 9.3 ohne
  Zusatzaufwand — und auf einen Blick zuzuordnen, worauf es bei Millisekunden-Entscheidungen ankommt.
- **4.181 Symbole zur Auswahl.** Für 60 Zauber (Kapitel 3) ist reichlich Reserve da; die
  alchemistischen Begriffe sind sämtlich abgedeckt.

**Die bekannte Schwäche und was daraus folgt:** Flach. Ohne Rahmen und Textur wirkt der Satz
nüchtern bis karg. **Damit wird Issue #22 (Rahmen, Panels, Ringtextur) zum tragenden Teil des
Aussehens** — nicht mehr zur Zutat. Es entscheidet, ob GEÄTZT streng oder leer wirkt.

**Folge für die Lizenz:** game-icons.net steht unter CC BY 3.0. **Der Credits-Bereich wird damit
fällig** (17.2), sobald die erste Symboldatei nach `assets/img/` wandert. Die Nennungstexte je
Urheber stehen in `ASSETS.md`. Das ist eine Lizenzauflage, keine Höflichkeit.

> Verworfen wurde unter anderem der einzige reine CC0-Weg („Skill, item and spell icons", pauliuw):
> überwiegend Gegenstands-Symbole, Rahmen fest eingemalt, uneinheitliche Machart. Der Kontaktbogen
> liegt als Beleg unter `docs/style-candidates/`.

**Zauber-Symbole: freie Gestaltung wie in WoW**, aber **mit mechanischer Ähnlichkeits-Prüfung**:
Alle Symbole werden in Spielgröße paarweise rechnerisch verglichen; zu ähnliche Paare werden neu
erzeugt. Grund: Erzeugte Symbole aus einem Durchgang tendieren zur Ähnlichkeit. Bei 16 Symbolen und
Millisekunden-Entscheidungen wäre das ein Ratespiel.

**Verifikation der Animationen:** Aufzeichnung als Video → Zerlegung in Einzelbilder → hunderte
Frames auf einmal prüfen (technische Korrektheit). Ob es sich _gut anfühlt_, beurteilt der
Auftraggeber.

### 17.3 Hilfe-System

| Baustein                           | Ausgestaltung                                                                                                                                                     |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Erklärfenster bei Berührung**    | Maus über Symbol/Wert/Begriff → vollständige Erklärung: Wirkung, Werte, Synergien, Bedeutung des Prozess-Namens. **Muss auch für Boss-Fähigkeiten funktionieren** |
| **Geführte Einführung**            | Beim ersten Auftreten einer Mechanik hält das Spiel kurz an und erklärt sie an Ort und Stelle                                                                     |
| **Klartext neben dem Fachbegriff** | Überall „COAGULATION (Survival)"                                                                                                                                  |

Ein separates Nachschlagewerk im Menü wurde **nicht** gewählt.

### 17.4 Geschichte und Archiv

**Handlungsbogen: Das Große Werk in Stufen.** Jede Instanz ist eine Stufe dieses Werks, und jede
misslingt auf eine neue Art. Im Endgame zeigt sich, dass das Werk selbst das eigentliche Problem ist.

**Ton: kreativ, spannend, geheimnisvoll — und zugleich witzig, unterhaltsam, seltsam.** Kein
durchweg düsterer Ernst.

**Erzählweise: Aufzeichnungen zum Nachlesen.** Vor jeder Instanz und nach jedem **ersten** Boss-Sieg
ein kurzer Text — danach nie wieder automatisch, aber jederzeit im Archiv nachlesbar. Bei hunderten
Wiederholungen steht die Geschichte damit nie im Weg.

**Die Aufzeichnungen lehren zugleich die Mechanik:** Der Text über Kalzination beschreibt, warum sich
Hitze aufbaut — und erklärt damit die Aufbau-Mechanik der Schule.

**Erzählstimmen: mehrere** — eigene Notizen des Spielers, gefundene Fragmente früherer Alchemisten,
nüchterne Geräte-Protokolle, und gelegentlich etwas, das offensichtlich nicht von einem Menschen
geschrieben wurde.

**Umfang: ~150 Einträge.** Eigenes Arbeitspaket.

---

## 18. Technische Architektur

### 18.1 Technik-Entscheidungen

| Thema                   | Festlegung                                                                                                                                                                                     |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Sprache/Bauwerkzeug** | **TypeScript + Vite. KEIN UI-Framework** — kein React, kein Svelte                                                                                                                             |
| **Darstellung**         | **Canvas für den Kampf, HTML für Menüs/Talentbaum/Auswertung**                                                                                                                                 |
| **Dev-Server**          | **nginx auf `localhost:8000/rota/`** — NIEMALS eigene HTTP-Server starten                                                                                                                      |
| **Browser-Tests**       | `playwright-cli -s=rota` (headless; `--headed` nur wenn zugeschaut wird). Session am Ende schließen                                                                                            |
| **Simulator**           | **Rust**, eigenständige Bibliothek unter `sim/`                                                                                                                                                |
| **Spielinhalte**        | **Dateien im Repository** (JSON/TOML), **keine Datenbank**                                                                                                                                     |
| **Speicherung**         | Browser-Speicher, laufend automatisch, **plus die letzten fünf rotierenden Sicherungsstände** und Export/Import. Bei beschädigtem Spielstand wird automatisch der letzte heile Stand angeboten |
| **Offline**             | **Vollständig offline** nach dem ersten Laden                                                                                                                                                  |
| **Auslieferung**        | Eigene Webseite. So bauen, dass eine Spieleplattform später möglich bleibt                                                                                                                     |

> Ein Spiel mit festem Logik-Takt und eigener Zeichenschleife braucht kein Oberflächen-Framework —
> die haben ihre eigene Vorstellung davon, wann neu gezeichnet wird, und stören hier eher.
> Preis: Menüs und Talentbaum werden von Hand gebaut.

### 18.2 Die vier Leistungs-Gefahren und ihre Gegenmaßnahmen

Die Spiellogik selbst ist vernachlässigbar: ~50 Zähler ergeben etwa 1.000 Rechenschritte pro Bild.
Auch wildes Hämmern auf 20 Tasten kostet praktisch nichts — **die Grenze setzt die Hand, nicht der
Prozessor.** Die echten Gefahren liegen woanders:

| Gefahr                                         | Gegenmaßnahme                                                                                                                                                         |
| ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Speicherbereinigung verursacht Ruckeln**     | **Objekte vorab anlegen und wiederverwenden**, nie im laufenden Bild etwas erzeugen. Ziel: **null Speicheranforderungen nach dem Aufwärmen**                          |
| **Layout-Neuberechnung durch HTML-Änderungen** | Canvas für alles Bewegte, **ein Zeichenvorgang pro Bild**                                                                                                             |
| **Geräteabhängiges Verhalten**                 | **Fester Logik-Takt (~10 ms)**, Darstellung getrennt und interpolierend. Auf 144 Hz gelten dieselben Regeln wie auf 60 Hz. Voraussetzung für vergleichbare Bestzeiten |
| **Datenmenge im Kampfprotokoll**               | **Vorab angelegter Ringpuffer mit gepackten Zahlen** (Zeitpunkt, Ereignisart, Zauber, Wert). ~10.000 Einträge bei einem 10-Minuten-Kampf ≈ 160 KB, keine Objekte      |

### 18.3 Ereignisgesteuertes Kampfsystem

**Feste Ereignismenge:** _Zauber gewirkt_ · _Schaden verursacht_ · _Krit_ · _DoT tickt_ · _Boss
betritt Phase_ · _Verstärkung läuft ab_.

Jeder Zauber, jedes Talent, jeder legendäre Effekt beschreibt nur, **worauf er reagiert**.

> **Empfänger in fest definierter Reihenfolge — keine Hash-Container.** Sonst weichen Spiel und
> Simulator voneinander ab.

**Ein neuer Zauber ist ein Datensatz statt neuer Programmcode.**

### 18.4 Determinismus und der Abgleich mit dem Simulator

| Prinzip                                                           | Warum                                                                                                                          |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| **Ganzzahlige Zeit statt Fließkomma-Sekunden**                    | Abklingzeit 8 s bei Tempo-Faktor 1,17 ergibt krumme Werte; Fließkomma-Timer driften über hunderte Ticks                        |
| **Gesetzte Startwerte, getrennte Zufallsströme**                  | Krit, DoT-Ticks und Heilung brauchen **je einen eigenen Strom** — sonst verschiebt eine zusätzliche Ziehung alles Nachfolgende |
| **Ergebnisse nach Startwert sortieren** vor dem Zusammenzählen    | Fließkomma-Addition ist nicht assoziativ                                                                                       |
| **Parallel zwischen Durchgängen, nie innerhalb**                  | Ein Schritt hängt vom vorherigen ab                                                                                            |
| **Keine Speicheranforderung im Schritt**, zusammenhängende Listen | Aus dem Vorbild-Projekt übernommen                                                                                             |

#### Der Beute-Zufall gehört NICHT zu den Kampf-Strömen

_Entschieden am 2026-08-04, nachdem E6-2 die Folgen beider Wege gemessen hat._

Die Kampf-Ströme (Krit, DoT-Ticks, Heilung) leben in `RandomStreams`, und ihre Ziehungszähler
gehen **je Takt** in die Zustands-Prüfsumme. Der Beute-Zufall bekommt einen **eigenen Strom
außerhalb** davon — aus demselben Kampf-Startwert abgeleitet, aber nicht in `drawCounts()`.

**Warum, und warum das kein Widerspruch zur Regel darüber ist:** Die Regel schützt davor, dass
eine zusätzliche Ziehung **alles Nachfolgende verschiebt**. Dieser Schutz greift für Ströme, die
im selben Ablauf ziehen. Beute wird **beim Sieg** gewürfelt, nicht während des Kampfes — sie kann
nichts verschieben, und nichts kann sie verschieben.

Ein vierter Strom in `RandomStreams` würde in einem Kampf über 7952 Takte **7952 Nullen** an die
Prüfsumme hängen, für einen Strom, der im Kampf nie zieht. Das macht die Prüfsumme länger, nicht
schärfer. Sie soll den **Kampf-Zustand** beschreiben; Beute ist kein Kampf-Zustand.

**Drei Auflagen, damit die Trennung nicht zur Schlamperei wird:**

1. **Dieselbe Ableitung aus dem Kampf-Startwert.** Eine aufgezeichnete Runde muss auch ihre Beute
   exakt wiederholen.
2. **Ein Wächter verhindert die Rückkehr in `drawCounts()`.** Wer den Beute-Strom später dort
   einträgt — aus dem nachvollziehbaren Reflex, alle Ströme an einer Stelle zu haben — verschiebt
   damit stillschweigend jede aufgezeichnete Prüfsumme.
3. **Die Begründung steht im Code**, nicht nur hier. Sonst liest jemand „zwei Orte für Zufall" und
   räumt auf.

> **Nicht entschieden wurde nach Aufwand.** Der andere Weg hätte 23 463 Zeilen in fünf
> Vergleichsdateien bewegt, davon zwei aus fremder Arbeit — gemessen, nicht geschätzt. Zum
> Zeitpunkt der Entscheidung hatte aber **keine** offene Verzweigung eine dieser Dateien
> angefasst; die Kollisionsgefahr war null. Die Entscheidung fiel allein daran, was die
> Prüfsumme beschreiben soll.

**Gemeinsame Prüffälle (Pflichttest bei jeder Änderung):** Eine Sammlung fester Testfälle der Form
„diese Tastendrücke zu diesen Zeitpunkten mit dieser Ausrüstung ergeben exakt diesen Schaden".
**Beide Seiten — Simulator (Rust) und Spiel (Browser) — müssen jede Datei bestehen.**

> Diese Prüffälle sind **aufgezeichnete echte Durchläufe** (siehe 19.1), nicht von Hand geschrieben.
> Ohne diesen Abgleich wäre die ganze Balance-Arbeit für ein Spiel gemacht, das es so nicht gibt.

### 18.5 Der Simulator — reines Entwicklungs-Werkzeug

**Er läuft bei der Entwicklung, nie beim Spieler.** Das laufende Spiel braucht **überhaupt keine
Simulation** — im Spiel wird nur ausgerechnet, was tatsächlich gewirkt wurde.

Vorbild: **SimulationCraft**. Übernommen:

| Konzept                                 | Bedeutung für ROTA                                                                                                                                     |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Ereignisgesteuert** statt fester Takt | Zwischen zwei Tastendrücken passiert außer DoT-Ticks nichts                                                                                            |
| **Aktions-Prioritätenliste (APL)**      | Die Rotation als geordnete Regelliste: „Wirke A, wenn Bedingung X. Sonst B, wenn Y." Beherrschbarer Suchraum, und das Ergebnis ist für Menschen lesbar |
| **Monte-Carlo**                         | Viele Durchgänge, Mittelwert und Fehlerbalken — nötig, weil Krit echter Zufall ist                                                                     |
| **Skalierungsfaktoren**                 | „Wie viel Schaden bringt ein Punkt CATALYSIS?"                                                                                                         |

**Ausgabe** (nur für Claude, Textausgabe/JSON genügt, **keine Diagramme nötig**):

- **Skalierungsfaktoren je Wert** — Grundlage für Ausrüstungs- und Talent-Balance
- **Zauber-Vergleichstabelle** — deckt tote Zauber auf, die niemand nutzt
- **Boss-Machbarkeitsprüfung** — ist jeder Boss je Schwierigkeit im Zeitlimit besiegbar, mit wie viel Reserve?

**Der Vollraum bleibt unerreichbar** (Ausrüstung × Talente × Zusammenstellung). Erschöpfend gerechnet
werden nur die Projektionen, auf denen Balance-Entscheidungen fallen. Der Rest bleibt Stichprobe.

**Die Referenz-Rotationen aus dem Balance-Läufer erscheinen NIRGENDS im Spiel** — auch nicht als
Lernhilfe. Das Herausfinden der besten Reihenfolge ist der Kern des Spiels.

### 18.6 Aktualisierungen und Spielstände

**Spielstände bleiben immer gültig:** Der Spielstand speichert nur, **WAS** besessen und erreicht
wurde — **nie berechnete Werte**. Balance-Änderungen gelten sofort für alle. Bestzeiten werden mit
der Fassung vermerkt, unter der sie erzielt wurden.

#### Die eine Ausnahme: Vorgeschichte mit Fassungsvermerk

_Entschieden am 2026-08-04, nachdem E7-2 den Widerspruch zwischen 15.3 und dieser Regel meldete._

**Der Verlauf über alle Durchläufe (15.3) darf den Schaden mitführen** — obwohl er ein berechneter
Wert ist. Bedingung: **jeder Eintrag trägt die Fassung**, unter der er entstand, wie die Bestzeit es
schon tut.

|                               | Wirkt auf das Spiel zurück               | Im Spielstand        |
| ----------------------------- | ---------------------------------------- | -------------------- |
| Ein gespeicherter Werte-Bonus | **ja** — er verändert den nächsten Kampf | verboten             |
| Ein vergangenes Ergebnis      | **nein** — es ist Vorgeschichte          | erlaubt, mit Fassung |

> **Die Regel bleibt, was sie war:** Kein Spielstand friert die Balance seines Tages **für kommende
> Kämpfe** ein. Ein Ergebnis von gestern tut das nicht — es steht in der Vergangenheit, so wie die
> Bestzeit, die ausdrücklich erlaubt ist.
>
> **Was der Fassungsvermerk leistet:** Wechselt die Fassung, zeigt der Verlauf an dieser Stelle einen
> **sichtbaren Schnitt** statt einer glatten Linie über zwei verschiedene Balance-Stände. Sonst
> erschiene ein Sprung, den der Spieler sich selbst zuschreibt — „bin ich schlechter geworden?" —
> obwohl sich die Zahlen unter ihm verändert haben.

**Die Schutzprüfung bekommt eine benannte Ausnahme, keine Umbenennung.** Ein Feld `dmgValue` zu
nennen, um an der Prüfung vorbeizukommen, wäre schlimmer als keine Prüfung: Der nächste, der sie
liest, hielte sie für wirksam. Die Ausnahme steht mit Begründung im Code und ist damit auffindbar.

---

### 18.7 Die Zauber-Reihenfolge ist eine Schnittstelle

**Der Index eines Zaubers ist an einigen Stellen eine nackte Zahl** — und dort deutet eine
Verschiebung der Liste sie still um. Gemessen, damit die Regel weiß, wovon sie handelt:

| Wo ein Zauber vorkommt                           | Form                         | Verschiebung     |
| ------------------------------------------------ | ---------------------------- | ---------------- |
| 21 von 22 Aufzeichnungen (`bindings`, `presses`) | `spellId` und `slot` — Namen | übersteht sie    |
| `cast-time.json`, Spalte `playerCasting`         | roher Index (2400 Einträge)  | **bricht**       |
| Prüffälle mit `const QUENCH = 3`                 | roher Index                  | **bricht still** |

**Der teure Fall ist der dritte**, nicht der zweite. Eine Aufzeichnung, die bricht, meldet
sich: Die Prüfsumme läuft auseinander. Ein Prüffall mit falschem Index meldet sich **nicht**
— er ist grün und misst einen anderen Zauber.

> Bei `interrupt-losses` fiel eine Verschiebung nur auf, weil an Index 3 zufällig ein
> Hitze-Zauber landete, der gar keine Unterbrechung platziert. Wäre dort ein anderer
> Unterbrecher gelandet, hätte niemand etwas gemerkt.

**Neue Zauber gehören deshalb ans Ende.** Hinten anhängen ändert keine bestehende Zahl;
dazwischen einfügen verschiebt alles danach — in Dateien, die niemand dabei ansieht.

Die Regel folgte keiner ableitbaren Ordnung und stand bis dahin nur als Kommentar in
`src/core/data.ts`, dreimal wiederholt (#201, #97, #304). Wer sie dort nicht las, konnte sie
nicht kennen: `quench` aus `common.json` steht vor `crust` aus `coagulation.json`, weil die
Liste von Hand zusammengesetzt ist.

**Eine Verschiebung ist erlaubt, wenn sie bewusst geschieht.** Sie kostet dann: Aufzeichnungen
neu erzeugen, den Wächter nachziehen, die Prüffälle prüfen. Das ist Arbeit, und sie soll es
sein — der Unterschied zu vorher ist, dass sie überhaupt sichtbar wird.

## 19. Entwicklungs-Werkzeuge

**Grundsatz:** Streng trennen zwischen Werkzeugen für **Claude als Entwickler** und Einstellungen für
**den Auftraggeber beim Spieltest**.

> **Der Auftraggeber will beim Testen das Spiel sehen, sonst nichts.**

### 19.1 Nur für Claude — unsichtbar, Bedienung über die Browser-Konsole

**Keine Oberfläche, keine Knöpfe, nichts im Bild.** Alle Werkzeuge über Konsolen-Befehle ansprechbar
→ dadurch auch automatisiert in Tests verwendbar.

| Werkzeug                        | Zweck                                                                                                                                                                                                     |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Zeitsteuerung**               | Anhalten, Einzelschritt, Zeitlupe, Zeitraffer. **Muss sauber vom Logik-Takt getrennt sein**, sonst verfälscht es Messungen                                                                                |
| **Zustand setzen**              | Boss-Leben, alle Abklingzeiten bereit, zu Phase springen, Hitze setzen, **Boss wechseln**, **Stufe setzen**. Erspart drei Minuten Spielzeit für eine Phase-3-Prüfung                                      |
| **Aufzeichnen und Abspielen**   | Runde mit Tastendrücken und Zufalls-Startwert aufzeichnen, exakt wiederholen. Grundlage für Regressionstests **und** für den Abgleich Spiel ↔ Simulator                                                   |
| **Zustands-Prüfsumme je Takt**  | Weichen Spiel und Simulator ab, sieht man **in welchem Takt**. Abschaltbar (kostet Rechenzeit)                                                                                                            |
| **Leistungs-Messwerte**         | Bildzeit, Zeit je Logik-Takt, Speicheranforderungen pro Bild, Eingabe-Verzögerung                                                                                                                         |
| **Schadens-Herleitung**         | Bei jedem Treffer nachvollziehbar: Grundschaden × welcher Faktor × welcher Faktor. Nur auf Abruf für einen einzelnen Treffer                                                                              |
| **Eingabe-Zeitmessung**         | Je Tastendruck: wie viele Millisekunden nach Ablauf des GCD. Beantwortet die Balance-Frage, wie viel ein Mensch gegenüber der perfekten Rechnung verliert. **Im fertigen Spiel bewusst nicht vorgesehen** |
| **Kampfprotokoll als Klartext** | Ringpuffer als lesbare Liste, exportierbar                                                                                                                                                                |

> **Warum der Boss-Wechsel dazugehört** (ergänzt am 2026-08-05, Issue #219): Die Liste nennt
> Beispiele, keine abschließende Aufzählung — ihr gemeinsamer Zweck ist, einen Spielzustand
> herzustellen, den zu erspielen unverhältnismäßig wäre. Beim Bau der Energieleiste fiel auf,
> dass **nur einer von sieben Bossen** überhaupt eine hat und dass er im Spiel unerreichbar ist:
> Das Spiel startet mit `FIRST_CRUCIBLE`, und weder `session.ts` noch `app.ts` rufen `bossById`.
> Zwei Abnahme-Kriterien waren damit nicht belegbar — nicht weil die Anzeige fehlte, sondern
> weil der Boss fehlte. Ein Werkzeug, mit dem sich jede Mechanik ohne Umweg ansehen lässt, ist
> genau das, was 19.1 meint.

> **Dasselbe gilt für die Stufe** (ergänzt am 2026-08-05, Issue #264): `setLevel(n)` setzt die
> Spielerstufe unmittelbar. Die Zauberleiste wächst mit ihr, und **ohne freien Platz lässt sich
> nicht zeigen, dass ein Zauber darauf ankommt** — acht Kämpfe im Browser zu spielen, um Stufe 3
> zu erreichen, wäre ein Nachweis über die Erfahrungskurve gewesen, und die ist an anderer Stelle
> belegt.
>
> **Es schreibt nur, WAS erreicht wurde** — Stufe und Erfahrung auf null. Alles andere wird daraus
> gerechnet, genau wie im Spiel (18.6: der Spielstand hält nie berechnete Werte, die auf kommende
> Kämpfe zurückwirken). Ein Werkzeug, das stattdessen Grundwerte oder Leisten-Plätze direkt
> setzte, könnte einen Zustand herstellen, den das Spiel selbst nie erzeugt — und ein Nachweis
> daran belegte nichts.
>
> Nicht zu verwechseln mit dem **Stufen-Boost** aus 19.2: Der ist für den Auftraggeber sichtbar
> im Player Home, dieser hier ist ein Konsolen-Befehl ohne Oberfläche.

### 19.2 Für den Auftraggeber — sichtbar im „Player Home"

| Einstellung           | Zweck                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------- |
| **Unsterblich**       | Eine Mechanik in Ruhe ansehen                                                                            |
| **Boss sofort töten** | Zur Auswertung oder nächsten Stufe springen                                                              |
| **Stufen-Boost**      | Spielerstufe frei einstellen — setzt Stufe, Talentpunkte und freigeschaltete Zauber                      |
| **Ausrüstung wählen** | An jedem der zehn Plätze **alle im Spiel vorhandenen Gegenstände** zur Auswahl, nicht nur die erspielten |

> Ohne Stufen-Boost und Ausrüstungs-Auswahl lässt sich das **Endgame nicht beurteilen, bevor es
> erspielt ist** — bei 24 h Aufstieg wäre jede Endgame-Prüfung sonst unmöglich.

> **Das Player Home selbst ist ein Spieler-Bildschirm (16.1), kein Werkzeug.** Herausgeschnitten
> werden von 19.3 die vier Einstellungen oben — nicht der Bildschirm, in dem sie sitzen. Ohne ihn
> hätte der Spieler keinen Talentbaum und keine Ausrüstung.

### 19.3 Trennung von der Auslieferung

**Beim Bauen komplett entfernt.** Alles aus 19.1 und 19.2 steht hinter einem Schalter, den Vite in
der Auslieferungs-Fassung auf aus setzt — **der Code wird dabei vollständig herausgeschnitten**.

Kein Byte davon landet beim Spieler: keine versehentliche Aktivierung, keine Leistungskosten, kein
Manipulations-Scheunentor. **Nachweisbar per grep über `dist/`.**

---

## 20. Umsetzungs-Reihenfolge

### 20.1 Das Kernrisiko

> **Wenn sich herausstellt, dass die optimale Rotation eine feste Prioritätenliste ist, die sich nie
> ändert, ist das Spiel nach etwa zwanzig Stunden nur noch Ausführung** — dann tragen auch die
> schönsten Belohnungen keine 300 Stunden.

Dagegen arbeiten: Synergien über Schulgrenzen, der Cooldown-Rücksetzer, Boss-Mechaniken die zur
Anpassung zwingen, Widerstände die Umbauen belohnen. **Ob es reicht, ist überprüfbar — und zwar
bevor viel Arbeit hineinfließt.**

### 20.2 Schritt 1 — spielbarer Kampf mit drei Zaubern

| Enthalten                     |                                               |
| ----------------------------- | --------------------------------------------- |
| Zauberleiste mit drei Zaubern | Abklingzeiten mit Wischblende und Zahl        |
| Globale Wartezeit             | eigene Darstellung                            |
| Ein Testboss                  | Reaktionsgefäß-Ring und Überdruck-Timer       |
| Schadenszahlen                | aufsteigend und verblassend                   |
| Kampfprotokoll                | vorab angelegter Ringpuffer                   |
| Kurzfassung nach dem Kampf    | mit den drei Kennzahlen                       |
| **Drei bis vier Kernklänge**  | je Zauber einer, plus Fehleingabe und Treffer |

Alles als **SVG-Gerüst ohne Grafiken**.

> **Warum der Klang schon in Schritt 1 dazugehört:** Das Gefühl beim Drücken hängt zu einem
> erheblichen Teil am Klang. **Ein stummer Prototyp fühlt sich immer träge an**, egal wie gut die
> Technik ist — und genau dieses Gefühl ist die Frage, die dieser Abschnitt beantworten soll.

**Die drei Zauber und der Testboss werden später die ersten des fertigen Spiels — keine
Wegwerfarbeit.** Konkrete Zahlen: [`STEP1-SPEC.md`](STEP1-SPEC.md).

**Abnahme:**

| Prüfung             | Kriterium                                                             |
| ------------------- | --------------------------------------------------------------------- |
| Spielgefühl         | **Urteil des Auftraggebers** — die Frage, die nur er beantworten kann |
| Ruckelfreiheit      | Keine Aussetzer im laufenden Kampf                                    |
| Eingabe-Verzögerung | **Die Wirkung erscheint im nächsten Bild** — nie im übernächsten      |
| Abklingzeiten       | Auf die Millisekunde korrekt                                          |
| Kampfprotokoll      | Vollständig, kein Ereignis fehlt                                      |
| Speicher            | **Keine Speicheranforderung im laufenden Bild**                       |

> Alles Messbare wird **gemessen statt behauptet**. Verfahren: [`VERIFICATION.md`](VERIFICATION.md).

### 20.3 Schritt 2–4

| Schritt | Inhalt                                                                                                                                                                                                                 |
| ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **2**   | **Simulator** als Rust-Bibliothek, abgeglichen über die gemeinsamen Prüffälle                                                                                                                                          |
| **3**   | **Prototyp-Prüflauf**: zwölf Zauber (3 CALCINATION, 3 FIXATION, 2 SUBLIMATION, 2 COAGULATION, 2 PUTREFACTION), drei Testbosse (60 s ohne Mechanik / 180 s mit Unterbrechen / 300 s mit hartem Schaden und zwei Phasen) |
| **4**   | **Inhalt in Wellen** — Instanz für Instanz, jede sofort spielbar                                                                                                                                                       |

**Hartes Abnahme-Kriterium für Schritt 3, vorab festgelegt:**

> **Eine schlechte Rotation muss messbar weniger Schaden machen als eine gute — mindestens 10 %.**
> Können muss sich lohnen; das ist der Kern.
>
> Wird das nicht erreicht, wird an den **Mechaniken** nachgebessert, bevor Inhalte gebaut werden.
> Nicht am Aussehen — am Spiel.

> **Ausdrückliche Abgrenzung:** Es wird **nicht** geprüft, ob jeder Boss eine _andere_ Rotation
> verlangt. Der Spieler soll spielen können, wie er will.

### 20.4 Wo der Bestand steht — gemessen am 2026-08-10

**Der Abstand zwischen Soll und Ist ist keine Schätzung.** Er wird gemessen, weil er zweimal aus
dem Gedächtnis falsch angegeben wurde:

```bash
for f in data/spells/*.json; do echo -n "$(basename $f .json): "; jq '.spells|length' $f; done
jq '.items|length' data/items.json
ls data/bosses/*.json | grep -v "/test" | wc -l
jq -r '[.trees[].school]|join(" ")' data/player/talents.json
```

| Bestand                     |                        Soll |                                  Ist | Quelle des Solls           |
| --------------------------- | --------------------------: | -----------------------------------: | -------------------------- |
| Zauber                      |                           — |                **54** + 2 gemeinsame | Zauber-Raster abgearbeitet |
| Talentbäume                 |          5 Schulen + global |                                **6** | 12.5                       |
| Talentwege                  |                          12 |                      **12** — gebaut | `docs/TALENT-RASTER.md`    |
| — im Simulator geprüft      |                          12 |                  **10** (#738 offen) | #738                       |
| Schlüsseltalente            |                           5 |                  **1** (CALCINATION) | #418                       |
| **Ausrüstung**              |     zehn Plätze, vier Töpfe |        **21 Gegenstände**, 10 Plätze | 13.0, 13.1b                |
| — Topf 4 (Sätze, Legendäre) |               angeschlossen | Leitung liegt, **nichts speist sie** | 13.1b                      |
| **Bosse**                   |      **68 in 14 Instanzen** |                 **9 in 2 Instanzen** | 3.0                        |
| Instanz-Zuordnung           |       Teil des Datenmodells |                            vorhanden | `instance` je Boss         |
| Instanz als Objekt          | Name, Stufen, Freischaltung |                       **fehlt ganz** | `data/instances.json`      |

**Entschieden vom Auftraggeber am 2026-08-10, nachdem ihm diese Zahlen vorlagen:**

1. **Die Ausrüstung wird vor den Bossen gebaut.** Sie wirkt auf jeden Kampf und geht in jede
   Schadensrechnung ein — ohne sie misst der Balance-Läufer einen Spieler ohne Werte. Es ist
   derselbe Grund, aus dem im Juli der Zauber-Katalog vorgezogen wurde.
2. **Bei den Bossen zuerst EINE Instanz vollständig**, samt Instanz-Datenmodell — nicht alle 68.
   Danach wird gemessen, ob die Rotation sich wirklich ändert.

Punkt 2 ist keine Einschränkung von Schritt 4, sondern seine Anwendung: „Inhalt in Wellen — Instanz
für Instanz". Der Grund steht in 20.1 und gilt für Bosse besonders scharf: **Vier Bosse, die falsch
gebaut sind, zieht man nach. Achtundsechzig nicht.**

> **Vor jeder Planung nach dieser Tabelle die Befehle oben ausführen.** Am 2026-08-09 nannte das
> Zauber-Raster 41 Zauber, am 2026-08-10 waren es 54 — beide Male hat die Messung den Fehler
> gefunden, nicht das Lesen. Eine Bestandszahl ist ein Datum, keine Wahrheit.

---

## 21. Bewusst verworfen · außerhalb des Umfangs · für später

**Vor jeder Design-Entscheidung lesen.** Jede Zeile war eine bewusste Wahl. Nichts davon ohne
ausdrückliche Rücksprache mit dem Auftraggeber wieder aufnehmen.

### 21.1 (F.1) Verworfen — Spielmechanik

| Verworfen                                                  | Stattdessen                                       | Grund                                                                 |
| ---------------------------------------------------------- | ------------------------------------------------- | --------------------------------------------------------------------- |
| **Globale Ressource (Mana)**                               | Nur Schul-Mechaniken mit eigenen Ladungen         | Abklingzeiten sind die alleinige Schwierigkeit                        |
| **Zufalls-Procs im Spieler-Kit**                           | Deterministische Synergien                        | Planbarkeit ist der Kern. _Ausnahme: Krit_                            |
| **Pech-Schutz beim Krit**                                  | Echte Wahrscheinlichkeit pro Treffer              | Bewusst mit Glücksanteil                                              |
| **Klickbare Zauberleiste**                                 | Nur Tastatur                                      | Grundsatzentscheidung                                                 |
| **Eingabe vormerken (Input-Queue)**                        | Fehleingabe blinkt rot + Ton, nichts wird gemerkt | Nimmt dem Spieler Präzisionsleistung ab und verfälscht die Auswertung |
| **Rotations-Empfehlung / Hervorhebung des besten Zaubers** | Nur „bereit"-Zustand über Helligkeit              | Würde die Hauptaufgabe lösen, bevor der Spieler anfängt               |
| **Übungsmodus für Boss-Phasen**                            | Wipe = von vorn                                   | Wie im echten Raid                                                    |
| **Prüfzeit-Wettkampfmodus im Training**                    | Nur die freie Puppe                               | —                                                                     |
| **Belohnung bei Niederlage**                               | Nur Fortschrittsmarke und Fehleranalyse           | _Folge: Punkte-Schwellen und niedrigere Stufen müssen offen bleiben_  |
| **Pech-Grenze / Garantie bei Beute**                       | Reine Prozent-Fallrate                            | Materialumwandlung federt indirekt ab                                 |
| **Werte-Streuung bei Gegenständen**                        | Feste Formel aus Stufe und Qualität               | Kein „perfektes Exemplar"-Glücksspiel                                 |
| **Qualität anheben / Werte neu würfeln**                   | Nur Gegenstandsstufe anheben                      | Würde die Fallraten entwerten                                         |
| **Aufwertung auf den Ausrüstungsplatz**                    | Stückgebunden                                     | _Bekannter Nachteil: bessere Funde entwerten die Investition_         |
| **Sätze während des Aufstiegs**                            | Sätze erst im Endgame                             | —                                                                     |
| **Boss reagiert auf Spielerverhalten**                     | Fester, lernbarer Ablauf                          | Voraussetzung für Planbarkeit und Simulierbarkeit                     |
| **Zufällige Abweichungen im Boss-Ablauf**                  | Fester Ablauf                                     | Bestzeiten müssen vergleichbar sein                                   |
| **Zwischenphase zum Durchatmen**                           | Eskalierende Phasen ohne Pause                    | Nicht gewählt                                                         |
| **Zauber-Versiegelung als Grundmechanik**                  | Drei Grundmechaniken ohne sie                     | Nicht gewählt                                                         |
| **Kanal-Zauber mit Tastenhalten**                          | Keypress = Trigger                                | Design-Gesetz 1                                                       |
| **Vollbild-exklusive Funktionen**                          | Feature-Parität                                   | Design-Gesetz 3                                                       |

### 21.2 (F.2) Verworfen — Anzeige und Hilfen

| Verworfen                                            | Grund                                                                                                                                                                                               |
| ---------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rotations-Güte in Prozent im Spiel**               | Erspart dem Spiel jede Simulation — der eigentliche Gewinn                                                                                                                                          |
| **Referenz-Rotation als Lernhilfe**                  | Das Herausfinden ist der Kern des Spiels                                                                                                                                                            |
| **Zeitangaben in der Boss-Übersicht**                | Nur Phasen und Reihenfolge; Timing muss man erspielen                                                                                                                                               |
| **Empfehlungen zur Zusammenstellung**                | Nur Fakten — die Schlussfolgerung zieht der Spieler                                                                                                                                                 |
| **Erklärungen während des Kampfes**                  | Die Einzelheiten stehen vorher in der Vorbereitung                                                                                                                                                  |
| **Sichtbares Kampfprotokoll**                        | Läuft nur intern; in WoW liest es ohnehin niemand                                                                                                                                                   |
| **Zähler „noch 3×"**                                 | Wäre nur eine Schätzung; Rhythmus soll verinnerlicht werden                                                                                                                                         |
| **Nachschlagewerk im Menü**                          | Erklärfenster, geführte Einführung und Klartext genügen                                                                                                                                             |
| **Eingabe-Zeitmessung im Spiel**                     | Niemand wird für seine Reaktionszeit getadelt (_bleibt Entwicklungs-Werkzeug_)                                                                                                                      |
| **Weniger Warnung auf höheren Schwierigkeitsstufen** | Information vorzuenthalten ist die billigste Form von Schwierigkeit                                                                                                                                 |
| **Farbsehschwächen-Unterstützung**                   | Ein Farbsatz für alle. _Milderung ohne Zusatzaufwand: Bereit-Zustand läuft über Helligkeit, Abklingzeit über Wischblende und Zahl — die spielkritische Information hängt nicht allein an der Farbe_ |
| **Einstellungen zur Reizstärke**                     | Grundeinstellung muss **dauertauglich** sein — lieber zurückhaltender als zu grell                                                                                                                  |
| **Teilen von Ergebnissen (Auswertungsbild)**         | Nicht vorgesehen                                                                                                                                                                                    |
| **Manipulationsschutz**                              | Unnötig, solange die Bestenliste lokal ist                                                                                                                                                          |

### 21.3 (F.3) Verworfen — WoW-Elemente

Wöchentliche Sperren · Ausrüstung als Zufallsbelohnung _(neben dem Talentbaum)_ · Trash-Gegner vor
dem Boss · Verbrauchsgegenstände (Fläschchen, Essen).

**Acht mechanisch reizvolle Muster aus der Boss-Recherche, bewusst nicht übernommen:**
Ausrichtungs-Wechsel (Licht/Dunkel umschalten) · zweiter Ressourcen-Kreislauf · Entscheidungen
bestimmen den Kampfverlauf · Rätsel-Phase mit Frist · begrenzte Zwischenphasen · Spieler bestimmt
Boss-Fähigkeiten · Ressource die auch dem Boss nützt · begrenzte Ressource schonen.

### 21.4 (F.4) Verworfen — Technik

| Verworfen                                            | Stattdessen                             | Grund                                                                                        |
| ---------------------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------- |
| **React / Svelte / jedes UI-Framework**              | Reines TypeScript + Vite                | Fester Logik-Takt und eigene Zeichenschleife vertragen sich schlecht mit Framework-Rendering |
| **Datenbank für Spielinhalte**                       | Dateien im Repository                   | Versionierung, kein Server, einkompilierbar                                                  |
| **Alles über HTML/CSS**                              | Canvas für den Kampf                    | Layout-Neuberechnung bei jedem Bild                                                          |
| **Logik im Bild-Takt**                               | Fester Logik-Takt, Darstellung getrennt | Sonst wären Bestzeiten geräteabhängig                                                        |
| **Objekt-Liste fürs Kampfprotokoll**                 | Vorab angelegter Ringpuffer             | Aufräum-Last verursacht Ruckeln                                                              |
| **Simulator in TypeScript**                          | Rust                                    | 10–50× schneller bei großen Läufen                                                           |
| **Zwei getrennte Regel-Umsetzungen ohne Abgleich**   | Gemeinsame Prüffälle als Pflichttest    | Sonst Balance für ein Spiel, das es nicht gibt                                               |
| **ElevenLabs für Sound**                             | Suno                                    | Kein Guthaben vorhanden                                                                      |
| **Reine API-Sound-Generierung ohne Browser**         | API-first mit Seiten-Fallback           | Suno verlangt ab dem 3. Aufruf einen Captcha-Token                                           |
| **Captcha-Token exportieren und außerhalb einlösen** | Klick in der echten Seite               | Kein Gewinn (der Browser wird ohnehin gebraucht), dazu Sperrrisiko für den bezahlten Account |
| **Selbst gestaltete Grafik**                         | Freie Assets + KI-Elemente              | Eingestandene Grenze                                                                         |
| **Assets mit „Free for personal use"**               | Nur CC0 und CC BY                       | Kommerziell nicht nutzbar                                                                    |

### 21.5 (F.5) Außerhalb des Umfangs von Version 1

| Punkt                                                  | Status                                                                                                                                                |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Zusatzgegner, Zielwechsel, Flächenschaden**          | **Version 2.** Würde eine zweite Rotationsebene einziehen und Zauber-Katalog, Talentbaum, Anzeige und Optimums-Suche erheblich vergrößern             |
| **Zeitstrahl in der Auswertung**                       | **Später nachrüsten.** Die drei Kennzahlen sagen bereits, WAS schieflief; der Zeitstrahl sagt WO                                                      |
| **Geteilte Bestenliste**                               | **Später.** Braucht Server, Datenbank, Betrugs-Schutz. Vorbereitung steht                                                                             |
| **Spieleplattform (itch.io, Steam)**                   | **Offen gehalten** — so bauen, dass ein späterer Wechsel möglich bleibt                                                                               |
| **Mobile / Touch-Bedienung**                           | **Nicht vorgesehen.** Widerspricht der Tastatur-Grundentscheidung                                                                                     |
| **Mehrsprachigkeit**                                   | **Nicht vorgesehen.** Spieltexte nur Englisch                                                                                                         |
| **Frei belegbare Tasten mit eigenem Einstellungsmenü** | Grundsatz steht (physische Position, Umschalt frei belegbar, Leiste frei anordenbar) — die **Tastenbelegungs-Oberfläche** dafür ist noch zu entwerfen |

### 21.6 (F.7) Risiken, die im Auge behalten werden müssen

| Punkt                                        | Beschreibung                                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Kernrisiko Tiefe**                         | Wenn die optimale Rotation eine feste Prioritätenliste ist, trägt nichts 300 Stunden → Prüfstein 20.3, **mindestens 10 %**                                                                                                                                                                                                                                                              |
| **288 Zeitmarken von Hand**                  | Viel Arbeit und fehleranfällig. Entlastung: Balance-Läufer liefert Vorschlagswerte (Gold = 105 %, Silber = 115 %, Bronze = 130 % der errechneten Bestzeit — **korrigiert am 2026-07-29**: Mit den früheren 110/130/160 % lag Bronze bei einer Bestzeit von 40.510 ms auf 64.816 ms und damit **über dem Zeitlimit von 60.000 ms**. Jeder Sieg erfüllte sie; sie unterschied niemanden.) |
| **Satz-Boni gegen Sonderwerte**              | Müssen etwa gleichwertig sein, sonst kippt es. Eigener Prüflauf nötig                                                                                                                                                                                                                                                                                                                   |
| **Stückgebundene Aufwertung**                | Kann das Ausprobieren bremsen                                                                                                                                                                                                                                                                                                                                                           |
| **Kein Sicherheitsventil bei Niederlage**    | Punkte-Schwellen und niedrigere Stufen müssen verlässlich offen sein                                                                                                                                                                                                                                                                                                                    |
| **Langer Schwanz bei Fallraten**             | Bei 1 % Legendary haben nach 300 Siegen noch 5 % nichts                                                                                                                                                                                                                                                                                                                                 |
| **Zahlen-Inflation durch die Endlos-Leiter** | WoW musste zweimal zurücksetzen. Die Werte-Obergrenze ist auch eine Lesbarkeits-Entscheidung                                                                                                                                                                                                                                                                                            |
| **Grafik-Stil aus KI-Einzelteilen**          | Uneinheitliche Lichtrichtung und Rahmenstärke wirken zusammengestückelt. Notausgang: ein Gestalter                                                                                                                                                                                                                                                                                      |
| **E1 blockiert den Start**                   | _(Arbeitsorganisation)_ Die ersten Tage arbeitet nur eine Code-Session, weil alle folgenden Epics auf einer **abgenommenen** Grundlage aufbauen sollen. E8 (Klang) und E9 (Grafik) füllen die Zeit — sie brauchen keinen Code                                                                                                                                                           |

---

## 22. Noch nicht ausgeplant

Diese Punkte sind **beschlossen**, aber die Ausarbeitung fehlt. Jeder braucht vor seinem Epic eine
eigene Planungsrunde mit dem Auftraggeber. **Nicht selbst entscheiden — fragen.**

| Punkt                          | Was fehlt                                                                                                                                                                              | Vor Epic                                                     |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| **Die 60 Zauber**              | Namen, Werte, Abklingzeiten, konkrete Synergie-Beziehungen, Stufen-Zuordnung                                                                                                           | E10 (Anfang: die 3 aus Schritt 1, dann die 12 des Prototyps) |
| **15–20 Mechanik-Bausteine**   | Die Varianten der neun übernommenen Muster                                                                                                                                             | E4                                                           |
| **68 Signatur-Regeln**         | Je Boss eine Regel in einem Satz                                                                                                                                                       | E10                                                          |
| **Boss-Zeitleisten**           | Je Boss: welcher Baustein wann, Phasen-Schwellen, Energieleisten-Takt                                                                                                                  | E10                                                          |
| **Talent-Inhalte**             | ~50 Punkte, 15 Zweige, Schlüsseltalente je Weg                                                                                                                                         | E5                                                           |
| **Gegenstands-Katalog**        | Konkrete Gegenstände, Sätze, legendäre Effekte, Boss-Bindungen                                                                                                                         | E6                                                           |
| **Erfahrungskurve**            | ~~Wie viel Erfahrung je Stufe~~ — **entschieden am 2026-08-05**, siehe 12.2                                                                                                            | erledigt                                                     |
| **Endlos-Leiter-Skalierung**   | Wachstum je Stufe, wo die Werte-Obergrenze liegt                                                                                                                                       | E10                                                          |
| **Erfolge**                    | Je Boss die konkreten Ziele, 288 Zeitmarken                                                                                                                                            | E10                                                          |
| **Instanz-Themen und Namen**   | 14 Instanzen mit Thema, Farbwelt, Klangteppich                                                                                                                                         | E10                                                          |
| **~150 Archiv-Texte**          | Inhalte, Stimmen-Verteilung, Reihenfolge                                                                                                                                               | E10                                                          |
| **Sound-Manifest**             | 115 Prompts mit Ziel-Längen                                                                                                                                                            | E8                                                           |
| **Grafik-Stil**                | Konkretes Asset-Set auswählen                                                                                                                                                          | E9                                                           |
| **Cooldown-Rücksetzer**        | Abklingzeit, ob GCD-frei, auf welche Zauber anwendbar                                                                                                                                  | E3                                                           |
| **Unterbrechen**               | Fensterbreite, Abklingzeit, Folgen des Verpassens                                                                                                                                      | E4                                                           |
| **Spellsteal**                 | Fensterbreite, Dauer der übernommenen Verstärkung, Abklingzeit                                                                                                                         | E4                                                           |
| **Schwierigkeitsstufen**       | Welche Mechanik kommt auf welcher Stufe dazu                                                                                                                                           | E4                                                           |
| ~~**Punkteformel**~~           | **ERLEDIGT** — quadratisch (2,0), Rest-Ertrag 20 Promille, entschieden am 2026-08-05. Steht in 12.6 und `data/player/reward.json`                                                      | —                                                            |
| **Abnehmender Ertrag**         | Konkrete Kurve über den Stufenabstand                                                                                                                                                  | E5                                                           |
| **Tastenbelegungs-Oberfläche** | Entwurf des Einstellungs-Bildschirms                                                                                                                                                   | E1 oder E5                                                   |
| **Issue-Ausarbeitung je Epic** | _(Arbeitsorganisation)_ Angelegt sind zunächst nur die Issues für E1, E8 und E9. Die übrigen werden formuliert, wenn ihr Epic an die Reihe kommt — sie jetzt zu schreiben wäre Raterei | laufend                                                      |
