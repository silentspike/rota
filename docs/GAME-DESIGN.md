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

| Thema              | Festlegung                                                                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **Spieltiefe**     | **Nur Abklingzeiten.** Keine globale Ressource, kein Mana. Die Schwierigkeit entsteht aus der Menge gleichzeitig zu verwaltender Cooldowns |
| **Synergien**      | **Feste, planbare Kombinationen** — 100 % deterministisch. Zauber A verstärkt Zauber B, immer                                              |
| **Eingabe**        | **Nur Tastatur.** Nur einfache Tasten und Umschalt-Kombinationen (kein Strg/Alt)                                                           |
| **Zauber-Typen**   | Sofort-Zauber · Zauber mit Wirkzeit (ab Mittelspiel) · DoTs. **Keine** Kanal-Zauber mit Tastenhalten                                       |
| **Umfang V1**      | **68 Bosse in 14 Instanzen** — 8 Aufstiegs-Instanzen à 4 Bosse (32) plus 6 Endgame-Instanzen à 6 Bosse (36)                                |
| **Höchststufe**    | **100**                                                                                                                                    |
| **Talentpunkte**   | **~50** (etwa jede zweite Stufe einer)                                                                                                     |
| **Zauber-Bestand** | **60** (12 je Schule), davon **12–16 gleichzeitig auf der Leiste**                                                                         |

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
| **PUTREFACTION** | **Zersetzungs-Stapel auf dem Boss.** Jede Wirkung legt einen Stapel an; je mehr Stapel, desto mehr Schaden nimmt der Boss **von allen Quellen**. Stapel verfallen langsam, mit Obergrenze. Allein gespielt schwach — braucht immer Partner                                                                                                          |

> **Warum fünf Systeme statt 120 Einzelregeln:** Man lernt fünf Mechaniken; jede Schule fühlt sich
> völlig anders an; und **ein neuer Zauber fügt sich automatisch ein**, weil er auf eine bestehende
> Mechanik einzahlt. Genau das Erweiterbarkeits-Ziel.
>
> **Risiko:** Diese fünf Mechaniken tragen das ganze Spiel. Sie müssen sehr sorgfältig entworfen
> werden und sind der erste Prüfstein im Prototyp (siehe 20.3).

### 4.2 Ressourcen

**Keine globale Ressource.** Kein Mana. Aber einzelne Schulen arbeiten mit **Ladungen**
(Hitze-Stapel, Kontroll-Punkte, Zersetzungs-Stapel) — das gibt ihnen Tiefe und ein eigenes
Spielgefühl, ohne die Grundentscheidung „Abklingzeiten sind die alleinige Schwierigkeit" anzutasten.

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

**CATALYSIS ist der einzige Wert, der die Rotation strukturell verändert** (kürzere globale
Wartezeit und kürzere Abklingzeiten). Er wächst dauerhaft über Talente und kurzzeitig über
**Overdrive** (siehe 11.4).

### 6.3 Schadensformel

```
Schaden = Grundschaden
        × ESSENCE-Faktor
        × Talent-Faktoren
        × aktive Verstärkungen
        × Synergie-Faktoren
        × Widerstandsfaktor des Bosses
        × Krit
```

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

---

## 7. Zauber

### 7.1 Zauberbalken und Wirkungsdauer — zwei getrennte Dinge

| Art                                              | Verhalten                                                                                                                                                                                                           |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Zauberbalken (Wirkzeit)**                      | **Blockiert.** Während der Balken läuft, kann nichts anderes gewirkt werden. Länge **1,5–2,5 s**, also ein bis knapp zwei globale Wartezeiten. Erzeugt eine echte Verpflichtung: „Kann ich mir das gerade leisten?" |
| **Wirkungsdauer (DoT, Verstärkung, Schwächung)** | **Blockiert nicht.** Der Zauber ist gewirkt, seine Wirkung läuft weiter, während man normal weiterdrückt                                                                                                            |

**Abbruch eines Zauberbalkens: nur durch bestimmte, vorher erkennbare Boss-Angriffe.** Normale
Treffer stören nicht. Bei Abbruch geht der Zauber verloren, **die Abklingzeit läuft aber nicht an**.

> Kein Zufallsärger, sondern lernbare Mechanik: Wer den Boss kennt, fängt den Zauber gar nicht erst an.

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

| Aspekt                                 | Festlegung                                                                                                                                                                                                                                                                      |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Physische Position statt Buchstabe** | Tasten werden über ihre **Position** angesprochen (`KeyboardEvent.code`), nicht über den Buchstaben. Ein deutscher Spieler sieht `Y`, ein englischer `Z` — beide drücken dieselbe Taste unter demselben Finger. **Die Beschriftung auf dem Symbol wird zur Laufzeit ermittelt** |
| **Umschalt**                           | Frei belegbar wie jede andere Taste, keine vorgegebene Rolle                                                                                                                                                                                                                    |
| **Leertaste**                          | Normale belegbare Taste, keine Sonderrolle                                                                                                                                                                                                                                      |
| **Leiste frei anordenbar**             | Der Spieler bestimmt, welcher Zauber auf welchem Platz liegt. **Muskelgedächtnis entsteht nur, wenn ein Zauber immer an derselben Stelle liegt**                                                                                                                                |
| **Rechte Hand**                        | Im Kampf unbeteiligt. Die Maus dient nur Menüs, Talentbaum und Zusammenstellung                                                                                                                                                                                                 |

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
| 6   | **Umgekehrtes Ziel**              | Statt Schaden zu machen muss eine kippende Reaktion **stabilisiert** werden. Der Kampf, in dem COAGULATION glänzt                                                          |
| 7   | **Eskalierende Wiederholung**     | Dieselbe Phase kehrt mehrfach wieder — kürzere Fenster, mehr gleichzeitige Anforderungen                                                                                   |
| 8   | **Schwächung ohne Heil-Lösung**   | Ein Effekt lässt sich nicht wegheilen, sondern nur durch eine bestimmte **Handlung** loswerden. **Braucht mehrere Lösungswege**                                            |
| 9   | **Unterbrechbarer Zauber**        | Boss beginnt einen Zauber, Balken läuft. Auf niedriger Schwierigkeit optional, auf hoher Pflicht                                                                           |

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

| Aspekt                 | Festlegung                                                                                                                                                                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Quelle**             | Restlaufzeit beim Boss-Sieg, **überproportional** umgerechnet — die letzten Sekunden sind unverhältnismäßig viel wert, damit Feinoptimierung sich lohnt                                                                                    |
| **Wiederholen**        | Punkte für **Bestzeit-Verbesserung** (Differenz-Auszahlung) plus ein Rest-Ertrag fürs bloße Wiederholen                                                                                                                                    |
| **Abnehmender Ertrag** | Der Ertrag sinkt **mit dem Stufenabstand**: voller Ertrag solange der Boss zur eigenen Stufe passt, bei ~10 Stufen Abstand nur noch ein Bruchteil, bei ~20 praktisch nichts. Lenkt sanft zum passenden Inhalt, **ohne etwas zu verbieten** |
| **Verwendung**         | **Freischaltungen** (Instanzen, einzelne Zauber, Ausrüstungsplätze) und **Gegenstände aufwerten**                                                                                                                                          |

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

### 13.2 Zehn Ausrüstungsplätze

Bewusst **keine Körperteile**, weil es keine Spielfigur gibt — arkane Artefakte:

`Focus · Catalyst · Sigil · Relic · Grimoire · Shard · Talisman · Phylactery · Anchor · Prism`

**Platz-spezifische Sonderwerte, feste Zuordnung:** z. B. Focus und Catalyst immer CATALYSIS, Sigil
und Talisman immer PURITY/YIELD, Anchor und Phylactery immer Verteidigung, Grimoire und Prism immer
Zauber-Verstärkung.

> **Gezieltes Grinden statt blinden Sammelns** — jeder der zehn Plätze bekommt einen erkennbaren
> Charakter.

**Grundwerte (ESSENCE, VESSEL, WARD) auf jedem Platz** — jeder Fund verbessert also spürbar etwas,
auch wenn die Sonderwerte nicht passen.

### 13.3 Qualitäten

| Qualität      | Werte-Plätze | Besonderheit                                                                                                |
| ------------- | ------------ | ----------------------------------------------------------------------------------------------------------- |
| Common        | 1            | —                                                                                                           |
| Rare          | 2            | —                                                                                                           |
| Epic          | 3            | —                                                                                                           |
| **Legendary** | 3+           | **Echter Sondereffekt, der die Rotation verändert** (z. B. „Der Cooldown-Rücksetzer wirkt auf zwei Zauber") |

**Werte-Berechnung: feste Formel, Qualität als Faktor.** Aus Gegenstandsstufe und Qualität errechnet
sich ein Wertebudget, das auf die Werte-Plätze verteilt wird. **Kein Gegenstand wird von Hand
balanciert** — ein neuer Gegenstand ist ein Eintrag mit Stufe, Qualität und Werte-Arten.

**Keine Werte-Streuung.** Kein „perfektes Exemplar"-Glücksspiel.

### 13.4 Fallraten

| Regel                                | Wirkung                                                                                                                                                                                                             |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Obergrenze durch Inhalt**          | Frühe Instanzen lassen nur Common und Rare fallen; Epic erst ab einer bestimmten Instanz, Legendary erst im Endgame. **Verhindert, dass ein früher Glückstreffer die restlichen zwanzig Aufstiegsstufen entwertet** |
| **Steigende Raten darin**            | Innerhalb der erlaubten Qualitäten steigen die Raten mit Instanz, Schwierigkeitsstufe und Endlos-Stufe                                                                                                              |
| **Richtwerte im Endgame**            | Common 60 %, Rare 30 %, Epic 9 %, **Legendary 1 %**                                                                                                                                                                 |
| **Zwei getrennte Achsen**            | **Qualität** und **Gegenstandsstufe** laufen unabhängig. Ein episches Stück aus einer frühen Instanz ist schwächer als ein seltenes aus dem Endgame — alter Inhalt entwertet sich von selbst                        |
| **Kein Pech-Schutz, keine Garantie** | Eine ehrliche Prozentzahl, kein versteckter Zähler                                                                                                                                                                  |

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

> **Harte Balance-Anforderung:** Satz-Boni und platz-spezifische Sonderwerte müssen **etwa
> gleichwertig** sein. Kippt es in die eine Richtung, tragen alle nur Sätze; kippt es in die andere,
> sind die Sätze wertlos. Eigener Prüflauf im Simulator (bester Satz-Aufbau gegen besten
> Affix-Aufbau je Ausrichtung).

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

- **Leerlauf pro Zauber** — wie viele Sekunden lag jeder Zauber ungenutzt bereit (nach Verlust sortiert)
- **Verlorene Zeit gesamt** — Sekunden und Prozent ohne aktiven Zauber
- **Synergie-Ausbeute** — wie viel Prozent der möglichen Kombinationen genutzt wurden

### 15.2 Fehleranalyse — aus den eigenen Daten gerechnet

**Keine Referenzdaten, keine Kombinatorik, kein Simulator im Browser.** Der Verlust wird aus den
Daten des Durchlaufs berechnet — es sind Multiplikationen, alle Werte stehen bereits im
Kampfprotokoll.

| Fehlerart                     | Rechnung                                                                                               | Ausgabe an den Spieler                                     |
| ----------------------------- | ------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| **Verpasste Wirkungen**       | Ein Zauber mit 12 s Abklingzeit passt 20× in einen 240-s-Kampf; gewirkt 15 → 5 verpasst × Schadenswert | „Five casts missed — 21,000 damage left on the table, 9 %" |
| **Leerlauf**                  | Sekunden ohne Handlung ÷ GCD = verschenkte Aktionen × Durchschnittsschaden je Aktion                   | „6.4 s idle — four wasted actions, 7 %"                    |
| **Verpasste Synergien**       | Verfügbar minus genutzt × Synergie-Bonus                                                               | „Seven of ten combinations used — 5 %"                     |
| **Verpasste Unterbrechungen** | Anzahl × Schaden des durchgekommenen Boss-Zaubers                                                      | „Two missed — 14 %"                                        |

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
| Klangteppich (Loop je Instanz plus Menü und Spannungs-Schicht)                                                                                                                          | 8      |

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
**Ästhetische Abnahme macht der Auftraggeber** über ein Abhör-Soundboard; Claude kann nicht hören
und prüft nur technisch (ffprobe: Länge, Pegel, Anfangs-Stille, Clipping).

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

---

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
| **Zustand setzen**              | Boss-Leben, alle Abklingzeiten bereit, zu Phase springen, Hitze setzen. Erspart drei Minuten Spielzeit für eine Phase-3-Prüfung                                                                           |
| **Aufzeichnen und Abspielen**   | Runde mit Tastendrücken und Zufalls-Startwert aufzeichnen, exakt wiederholen. Grundlage für Regressionstests **und** für den Abgleich Spiel ↔ Simulator                                                   |
| **Zustands-Prüfsumme je Takt**  | Weichen Spiel und Simulator ab, sieht man **in welchem Takt**. Abschaltbar (kostet Rechenzeit)                                                                                                            |
| **Leistungs-Messwerte**         | Bildzeit, Zeit je Logik-Takt, Speicheranforderungen pro Bild, Eingabe-Verzögerung                                                                                                                         |
| **Schadens-Herleitung**         | Bei jedem Treffer nachvollziehbar: Grundschaden × welcher Faktor × welcher Faktor. Nur auf Abruf für einen einzelnen Treffer                                                                              |
| **Eingabe-Zeitmessung**         | Je Tastendruck: wie viele Millisekunden nach Ablauf des GCD. Beantwortet die Balance-Frage, wie viel ein Mensch gegenüber der perfekten Rechnung verliert. **Im fertigen Spiel bewusst nicht vorgesehen** |
| **Kampfprotokoll als Klartext** | Ringpuffer als lesbare Liste, exportierbar                                                                                                                                                                |

### 19.2 Für den Auftraggeber — sichtbar im „Player Home"

| Einstellung           | Zweck                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------- |
| **Unsterblich**       | Eine Mechanik in Ruhe ansehen                                                                            |
| **Boss sofort töten** | Zur Auswertung oder nächsten Stufe springen                                                              |
| **Stufen-Boost**      | Spielerstufe frei einstellen — setzt Stufe, Talentpunkte und freigeschaltete Zauber                      |
| **Ausrüstung wählen** | An jedem der zehn Plätze **alle im Spiel vorhandenen Gegenstände** zur Auswahl, nicht nur die erspielten |

> Ohne Stufen-Boost und Ausrüstungs-Auswahl lässt sich das **Endgame nicht beurteilen, bevor es
> erspielt ist** — bei 24 h Aufstieg wäre jede Endgame-Prüfung sonst unmöglich.

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
| Eingabe-Verzögerung | **Unter einem Bild (< 16 ms)**                                        |
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

| Punkt                                        | Beschreibung                                                                                                                                                                                                                  |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Kernrisiko Tiefe**                         | Wenn die optimale Rotation eine feste Prioritätenliste ist, trägt nichts 300 Stunden → Prüfstein 20.3, **mindestens 10 %**                                                                                                    |
| **288 Zeitmarken von Hand**                  | Viel Arbeit und fehleranfällig. Entlastung: Balance-Läufer liefert Vorschlagswerte (Gold ≈ 110 %, Silber ≈ 130 %, Bronze ≈ 160 % der errechneten Bestzeit)                                                                    |
| **Satz-Boni gegen Sonderwerte**              | Müssen etwa gleichwertig sein, sonst kippt es. Eigener Prüflauf nötig                                                                                                                                                         |
| **Stückgebundene Aufwertung**                | Kann das Ausprobieren bremsen                                                                                                                                                                                                 |
| **Kein Sicherheitsventil bei Niederlage**    | Punkte-Schwellen und niedrigere Stufen müssen verlässlich offen sein                                                                                                                                                          |
| **Langer Schwanz bei Fallraten**             | Bei 1 % Legendary haben nach 300 Siegen noch 5 % nichts                                                                                                                                                                       |
| **Zahlen-Inflation durch die Endlos-Leiter** | WoW musste zweimal zurücksetzen. Die Werte-Obergrenze ist auch eine Lesbarkeits-Entscheidung                                                                                                                                  |
| **Grafik-Stil aus KI-Einzelteilen**          | Uneinheitliche Lichtrichtung und Rahmenstärke wirken zusammengestückelt. Notausgang: ein Gestalter                                                                                                                            |
| **E1 blockiert den Start**                   | _(Arbeitsorganisation)_ Die ersten Tage arbeitet nur eine Code-Session, weil alle folgenden Epics auf einer **abgenommenen** Grundlage aufbauen sollen. E8 (Klang) und E9 (Grafik) füllen die Zeit — sie brauchen keinen Code |

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
| **Erfahrungskurve**            | Wie viel Erfahrung je Stufe, woher sie kommt                                                                                                                                           | E5                                                           |
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
| **Punkteformel**               | Konkrete überproportionale Kurve                                                                                                                                                       | E5                                                           |
| **Abnehmender Ertrag**         | Konkrete Kurve über den Stufenabstand                                                                                                                                                  | E5                                                           |
| **Tastenbelegungs-Oberfläche** | Entwurf des Einstellungs-Bildschirms                                                                                                                                                   | E1 oder E5                                                   |
| **Issue-Ausarbeitung je Epic** | _(Arbeitsorganisation)_ Angelegt sind zunächst nur die Issues für E1, E8 und E9. Die übrigen werden formuliert, wenn ihr Epic an die Reihe kommt — sie jetzt zu schreiben wäre Raterei | laufend                                                      |
