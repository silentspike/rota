# Das Zauber-Raster — 18 fehlende Zauber

> **Freigegeben vom Auftraggeber am 2026-08-06**, vollständig und ohne Änderung: _„wir übernehmen
> alle deine vorschläge. wir müssen sowieso ja dann das ganze tuning, balancing, usw machen."_

> ### ⚠ Bestandszahlen sind ein Datum, keine Wahrheit
>
> **Vor jeder Planung nach diesem Dokument den Ist-Stand messen:**
>
> ```bash
> for f in data/spells/*.json; do
>   echo -n "$(basename $f .json): "
>   python3 -c "import json,sys;d=json.load(open('$f'));print(len(d.get('spells',d)))"
> done
> ```
>
> **Stand 2026-08-09**, gemessen und in die Abschnitts-Überschriften eingetragen:
>
> | Schule       | vorhanden | fehlen |
> | ------------ | --------: | -----: |
> | CALCINATION  |        10 |      2 |
> | FIXATION     |        10 |      2 |
> | PUTREFACTION |         6 |      5 |
> | SUBLIMATION  |         8 |      4 |
> | COAGULATION  |         7 |      5 |
> |              |           | **18** |
>
> Am 2026-08-09 nannte dieses Dokument bei PUTREFACTION _zwei_ vorhandene Zauber — es waren
> **sechs**. Vier der zehn Vorschläge waren längst gebaut. Wer die Liste abgearbeitet hätte, hätte
> vier Zauber doppelt angelegt.
>
> **Die Vorschlagslisten selbst bleiben gültig** — Namen, Wirkung, Abklingzeit, Bauart. Nur die
> Bestandszahlen altern, weil jeder gebaute Zauber sie ändert und niemand sie nachträgt.
>
> Weitere Zahlen im folgenden Kopfabschnitt („Kandidaten aus N Zaubern") stammen aus derselben
> Analyse und sind ebenso alt. Sie tragen die Aussage über die **Rotationsfähigkeit** der Schulen,
> nicht über den Bestand — für die Planung ist der Messbefehl oben maßgeblich.

**Warum dieses Dokument jetzt entsteht.** Die Balance-Arbeit an #472 wurde gestoppt, weil sie auf
einer Grundlage stand, die sich noch bewegt:

```
CALCINATION    8 Kandidaten aus 5 Zaubern
FIXATION       4 aus 4 — aber nur bind macht Schaden
PUTREFACTION   2 aus 2
```

> CALCINATION ist die einzige Schule, die eine Drei-Zeilen-Rotation **füllen** kann. Selbst bei
> gleichem Schaden je Zauber bliebe sie vorn.

Die Entscheidung des Auftraggebers dazu:

> Sowas kann nur die Simulation errechnen. Es gibt ja immer Synergien, Talente usw. **Der Katalog
> sollte jetzt unbedingt erledigt werden, da alles davon abhängt** — die ganze Simulation, alle
> anderen Spells, die Bosse, die Items, die Skills. Sonst bauen wir was auf, berechnen etwas, was
> dann gar nicht stimmt.

## Wie dieses Dokument zu lesen ist

> ## Achtzehn dieser Zauber brauchen eine Mechanik, die es noch nicht gibt
>
> **Gemessen am 2026-08-06 von drei Sitzungen unabhängig voneinander**, nachdem dieses Raster
> geschrieben war:
>
> | Schule       | brauchen neue Mechanik |
> | ------------ | ---------------------- |
> | CALCINATION  | **6 von 7**            |
> | FIXATION     | 4 von 8                |
> | PUTREFACTION | **8 von 10**           |
>
> **Der Grund ist ein Fehler in diesem Dokument:** Es wurde als _Design_ geschrieben — welche
> Rolle in der Schul-Mechanik fehlt — **ohne zu prüfen, was das Datenmodell heute kann.** Hier
> stehen Rollen, und einige davon heißen zu Unrecht „Zauber".
>
> **Deshalb wird in drei Ebenen gebaut:** die mit heutigen Feldern baubaren zuerst, dann die
> Mechaniken als eigene Vorhaben (nach Gruppen, weil mehrere sich dieselbe Vorarbeit teilen),
> dann die restlichen Zauber als reine Daten-Einträge.
>
> Wo eine Zeile eine fehlende Mechanik verlangt, ist das **kein schwieriger Zauber, sondern ein
> eigenes Vorhaben.**

**Alle Zahlen sind Platzhalter.** Sie stehen da, damit der Balance-Läufer etwas zu messen hat —
nicht als Vorschlag. Was **entschieden** ist: welcher Zauber es gibt, welche Rolle er in der
Schul-Mechanik spielt, und warum er fehlt.

**Die Abklingzeiten folgen dem Muster der vorhandenen:** 1,5 s für Füller, 3–15 s für Wirkungen,
20–45 s für Höhepunkte. Auch das ist eine Ordnung, keine Balance.

**Zwei Namen sind ersetzt**, weil sie die Regel „ein- bis zweisilbig, passt auf ein 48-Pixel-Symbol"
brechen: `FORGE` → `FORGE`, `PYROCLASM` → `PYROCLASM`.

**Zwei Namenskollisionen sind aufgelöst:** `KEYSTONE` → `CAPSTONE` (der Begriff gehört den
Schlüsseltalenten aus #418), `RIPOSTE` → `RIPOSTE` (der Design-Plan nennt „Spell Reflect").

---

## CALCINATION — 10 vorhanden, 2 neu

**Mechanik (GAME-DESIGN 173):** Hitze sammeln und entladen. Jede Wirkung erzeugt Hitze; ein
Entlade-Zauber verbraucht die gesamte angesammelte Hitze. Der Entlade-Zauber ist Pflichtzauber
der Schule.

**Die drei Talent-Wege (12.5):** schneller aufbauen · stärker entladen · länger halten.

## Was steht

| Zauber     | CD    | Schaden    | Rolle                                    |
| ---------- | ----- | ---------- | ---------------------------------------- |
| `kindle`   | 1,5 s | 100        | Füller, +3 Hitze                         |
| `bellows`  | 1,5 s | 40         | Füller, **+8 Hitze** — der starke Aufbau |
| `smoulder` | 1,5 s | DoT        | Brand, Hitze je Tick                     |
| `searing`  | 1,5 s | DoT        | Brand, Hitze je Tick                     |
| `pyre`     | 5 s   | Hitze × 12 | **Entlader**                             |

**Die Schule hat heute fünf Zauber und ist damit die einzige, die eine Drei-Zeilen-Rotation
füllen kann** — das ist der Befund, der die Balance-Arbeit gestoppt hat.

## Vorschlag für die sieben neuen

| #     | Name         | CD    | Rolle                                                                                        | Warum er fehlt                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ----- | ------------ | ----- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | `EMBER`      | 1,5 s | Füller, +2 Hitze, **kein Schaden** — dafür verlängert er alle laufenden Brände um einen Tick | Der Weg „länger halten" hat heute keinen Zauber                                                                                                                                                                                                                                                                                                                                                                                                   |
| ~~2~~ | ~~`SCORIA`~~ | —     | **GESTRICHEN am 2026-08-10**                                                                 | Die Annahme war falsch: Den Teil-Verbrauch **gibt es bereits** — als Schlüsseltalent `split-spender` (`combat.ts:3739-3750`, `data/player/talents.json:49`), samt Doppelschlag (`const schlaege = geteilt ? 2 : 1`). Der Kommentar dort zitiert diese Raster-Zeile wörtlich. Der Zauber wäre eine zweite Umsetzung derselben Idee gewesen, bei gleichzeitiger Wirkung unklar. Entschieden vom Auftraggeber: „Zauber streichen, Talent behält es." |
| 3     | `FLASHPOINT` | 12 s  | Verdoppelt die Hitze-Erzeugung für drei Wartezeiten                                          | Der Weg „schneller aufbauen" braucht einen Höhepunkt                                                                                                                                                                                                                                                                                                                                                                                              |
| 4     | `CINDER`     | 3 s   | Brand mit **kurzer** Laufzeit, hohe Tick-Zahl                                                | Heute sind beide Brände gleich lang — keine Erneuerungs-Entscheidung                                                                                                                                                                                                                                                                                                                                                                              |
| 5     | `FORGE`      | 30 s  | Wandelt Hitze in **Tempo** statt Schaden                                                     | Verbindet die Schule mit CATALYSIS, ohne eine Zahl zu erhöhen                                                                                                                                                                                                                                                                                                                                                                                     |
| 6     | `SLAG`       | 20 s  | Hitze-Obergrenze steigt, solange er läuft                                                    | Macht „aufsparen" zu einer echten Option                                                                                                                                                                                                                                                                                                                                                                                                          |
| 7     | `PYROCLASM`  | 45 s  | Der große Entlader: verbraucht Hitze **und** alle laufenden Brände                           | Ein Höhepunkt für lange Kämpfe                                                                                                                                                                                                                                                                                                                                                                                                                    |

### Der dritte Fall derselben Klasse

`SCORIA` ist der dritte gestrichene Zauber, dessen Raster-Zeile eine **ungemessene
Annahme über den Bestand** enthielt — nicht der dritte schwierige Zauber:

| Zauber      | die Annahme                                    | gemessen                |
| ----------- | ---------------------------------------------- | ----------------------- |
| `CARRION`   | „die Stapel überleben den Phasenwechsel nicht" | sie tun es              |
| `CONTAGION` | „die Stapel wirken nicht auf DoT-Ticks"        | sie tun es              |
| `SCORIA`    | „es gibt keinen Teil-Verbrauch"                | es gibt ihn, als Talent |

Alle drei hätten einen halben Tag gekostet, wenn sie gebaut worden wären. **Wo eine
Raster-Zeile eine Aussage über den Bestand macht, gehört die Aussage gemessen, bevor der
Zauber gemessen wird.**

Bei `SCORIA` war die Annahme besonders gut versteckt: Der Quelltext des Talents
**zitiert diese Raster-Zeile wörtlich** (`combat.ts:3739`). Wer nach einem Feldnamen
suchte — `hits`, `multiStrike`, `strikeCount` — fand nichts und schloss daraus auf die
Mechanik. Sie hing an einem Schalter, nicht an einem Feld.

## Was ich dabei nicht entschieden habe

**Alle Zahlen sind Platzhalter.** Sie stehen da, damit der Balance-Läufer etwas zu messen hat —
nicht als Vorschlag. Die Abklingzeiten folgen dem Muster der vorhandenen (1,5 s für Füller, 5–15 s
für Wirkungen, 30–45 s für Höhepunkte).

**Die Namen sind englisch und ein- bis zweisilbig**, wie die vorhandenen (GAME-DESIGN: „Ein
Zaubername steht auf einem Symbol von 48 Pixeln"). `FORGE` und `PYROCLASM` brechen das
— falls Ihnen das wichtig ist, ersetze ich sie durch `FORGE` und `PYROCLASM`.

## FIXATION — 10 vorhanden, 2 neu

**Mechanik (GAME-DESIGN 174):** Kontroll-Punkte als Vorrat zum Ausgeben. Fixierungs-Zauber bauen
Punkte auf; der Spieler gibt sie gezielt aus, um Boss-Handlungen zu beeinflussen. Ein
**Planungswerkzeug**, keine passive Wirkung.

**Die drei Wirkungen kaufen einen Moment, keinen Zustand:** Zauber verlangsamen · Schlag verzögern
· Phase hinauszögern.

## Was steht

| Zauber   | CD    | Schaden | Rolle                                                |
| -------- | ----- | ------- | ---------------------------------------------------- |
| `bind`   | 1,5 s | 60      | **Der einzige Zauber der Schule, der Schaden macht** |
| `stall`  | 6 s   | —       | Ausgeber: Zauber verlangsamen                        |
| `stay`   | 8 s   | —       | Ausgeber: Schlag verzögern                           |
| `anchor` | 20 s  | —       | Ausgeber: Phase hinauszögern                         |

**Die Schieflage ist hier am deutlichsten:** Drei der vier Zauber machen keinen Schaden. Wer
FIXATION spielt, hat eine Rotation aus einem Zauber und drei Werkzeugen.

## Vorschlag für die acht neuen

| #   | Name       | CD    | Rolle                                                                               | Warum er fehlt                                           |
| --- | ---------- | ----- | ----------------------------------------------------------------------------------- | -------------------------------------------------------- |
| 1   | `TETHER`   | 1,5 s | Zweiter Füller mit Schaden, +2 Punkte                                               | Eine Schule braucht mehr als einen Schadenszauber        |
| 2   | `CLASP`    | 1,5 s | Füller, +1 Punkt, verlängert laufende Verlangsamungen                               | Der Aufbau ist heute an `bind` allein gebunden           |
| 3   | `PIN`      | 4 s   | Ausgeber: der nächste Boss-Zauber wird **unterbrechbar**, auch wenn er es nicht war | Verbindet die Schule mit `quench`                        |
| 4   | `LATCH`    | 10 s  | Ausgeber: der nächste eingehende Schlag trifft **verzögert und schwächer**          | Die Schule hat heute keine Verteidigung                  |
| 5   | `MOOR`     | 15 s  | Punkte-Obergrenze steigt, solange er läuft                                          | Wie `SLAG` bei CALCINATION: „aufsparen" wird eine Option |
| 6   | `WEDGE`    | 6 s   | Wandelt Punkte in **Schaden** um — der Notausgang, wenn nichts zu kontrollieren ist | Gegen Bosse ohne Mechanik ist die Schule sonst tot       |
| 7   | `GRIPLOCK` | 25 s  | Doppelte Punktegewinnung für vier Wartezeiten                                       | Höhepunkt für den Aufbau-Weg                             |
| 8   | `CAPSTONE` | 40 s  | Alle drei Ausgeber-Wirkungen auf einmal, verbraucht alle Punkte                     | Der große Moment der Schule                              |

## Der Befund, der hier drinsteckt

**`WEDGE` ist der wichtigste der acht.** FIXATION ist heute gegen einen Boss ohne Mechanik
wertlos — es gibt nichts zu verlangsamen. Ein Umweg von Kontrolle zu Schaden macht die Schule
überall spielbar, ohne ihren Charakter zu ändern.

Das ist keine Balance-Zahl, sondern eine Lücke in der Mechanik: **Die Schule hat keinen Plan B.**

**Der Name `KEYSTONE` kollidiert** mit dem Begriff für Schlüsseltalente (`#418`). Falls er bleiben
soll, benenne ich das andere um — sonst heißt der Zauber `CAPSTONE`.

## PUTREFACTION — 6 vorhanden, 5 neu

**Mechanik (GAME-DESIGN 177):** Zersetzungs-Stapel auf dem Boss. Jede Wirkung legt einen Stapel
an; je mehr Stapel, desto mehr Schaden nimmt der Boss **von allen Quellen** — 6 % je Stapel.
Stapel verfallen langsam, mit Obergrenze. **Allein gespielt schwach — braucht immer Partner.**

## Was steht

| Zauber   | CD  | Schaden | Rolle                                                 |
| -------- | --- | ------- | ----------------------------------------------------- |
| `canker` | 2 s | 45      | Stapel-Erzeuger, öffnet Synergie-Fenster für `fester` |
| `fester` | 9 s | 60      | Der Partner — wirkt stärker, je mehr Stapel liegen    |

**Zwei Zauber sind zu wenig für eine Drei-Zeilen-Rotation.** Das ist die andere Hälfte des
Balance-Befunds: PUTREFACTION erreicht 1964 gegen CALCINATIONs 7250, und selbst gleiche Zahlen je
Zauber würden das nicht ausgleichen.

## Vorschlag für die zehn neuen

| #      | Name           | CD    | Rolle                                                      | Warum er fehlt                                                                                                                                                                               |
| ------ | -------------- | ----- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1      | `BLIGHT`       | 1,5 s | Füller, +1 Stapel                                          | Die Schule hat keinen billigen Füller                                                                                                                                                        |
| 2      | `MIASMA`       | 1,5 s | Füller, kein Stapel, dafür Schaden nach Stapelzahl         | Belohnt den Aufbau ohne ihn zu treiben                                                                                                                                                       |
| 3      | `ROT`          | 4 s   | DoT, legt bei jedem Tick einen Stapel                      | Der Aufbau läuft heute nur über Tastendrücke                                                                                                                                                 |
| 4      | `SEPSIS`       | 8 s   | Verdoppelt die Verfallszeit aller Stapel                   | Der Weg „länger halten"                                                                                                                                                                      |
| 5      | `NECROSIS`     | 12 s  | Verbraucht alle Stapel für einen harten Schlag             | Ein Entlader wie `pyre` — heute gibt es keinen                                                                                                                                               |
| 6      | `PUTRID BLOOM` | 15 s  | Die Stapel-Obergrenze steigt, solange er läuft             | Wie `SLAG`/`MOOR` in den anderen Schulen                                                                                                                                                     |
| 7      | `CONTAGION`    | 20 s  | Stapel wirken zusätzlich auf **DoT-Ticks** anderer Schulen | Macht die Partner-Abhängigkeit zur Stärke                                                                                                                                                    |
| 8      | `VIRULENCE`    | 25 s  | Jeder Stapel zählt für vier Wartezeiten doppelt            | Der Höhepunkt vor einem Schadensfenster                                                                                                                                                      |
| 9      | `DECAY`        | 3 s   | Schwächt den Boss-Schaden nach Stapelzahl                  | Verbindet Schwächung mit Überleben                                                                                                                                                           |
| ~~10~~ | ~~`CARRION`~~  | —     | **GESTRICHEN am 2026-08-06**                               | Die Annahme war falsch: Die Stapel überleben den Phasenwechsel **bereits** — gemessen an `test-gatekeeper`, 3 vorher und 3 danach an zwei Wechseln. Der Zauber hätte keine messbare Wirkung. |

## Zwei Befunde, die beim Schreiben aufgefallen sind

**Die Schule hat keinen Entlader.** CALCINATION hat `pyre`, PUTREFACTION nichts Vergleichbares —
die Stapel verfallen einfach. `NECROSIS` schließt das, und erst damit hat die Schule einen
Rhythmus statt eines Dauerzustands.

**Und sie verliert ihren Aufbau beim Phasenwechsel.** Ob das so gewollt ist, weiß ich nicht — es
steht nirgends — **und die Frage hat sich beim Messen erledigt: Sie überleben ihn.** `CARRION` ist damit gestrichen. Die Annahme in der Tabelle oben war ungemessen, und das ist der Fehler, nicht der Zauber.

## SUBLIMATION — 8 vorhanden, 4 neu

**Mechanik (GAME-DESIGN 175):** Automatische Verkürzung aller Abklingzeiten. Sublimations-Zauber
verkürzen laufend alle Abklingzeiten um einen kleinen Betrag. Der **Cooldown-Rücksetzer** bleibt
als eigener Zauber daneben: er setzt den _nächsten gewirkten_ Zauber zurück.

## Was steht

| Zauber   | CD   | Schaden | Rolle                              |
| -------- | ---- | ------- | ---------------------------------- |
| `vapour` | 8 s  | —       | Verkürzt alle Abklingzeiten um 5 s |
| `reflux` | 45 s | —       | Der Cooldown-Rücksetzer            |

## Der Befund, der über dieser Schule steht

**Beide Zauber sind gemessen tot.** Der Balance-Läufer hat sie an zwei Bössen mit
unterschiedlicher Dauer geprüft — sie kommen in **keiner** optimalen Rotation vor. Das ist Punkt
11 der Entscheidungsliste, den Sie heute ausdrücklich **vertagt** haben:

> Die Messung lief an zwei Bössen mit 15 Zaubern. Bei 60 Zaubern und Abklingzeiten bis 180
> Sekunden laufen mehr lange Abklingzeiten gleichzeitig — genau die Bedingung, unter der die
> Verkürzung sich lohnt.

**Dieses Raster ist der Test dieser Annahme.** Wenn die Schule mit zwölf Zaubern immer noch tot
ist, trägt die Bauart nicht — und dann sind es nicht zwei tote Zauber, sondern zwölf.

## Vorschlag für die zehn neuen

| #   | Name       | CD    | Rolle                                                          | Warum er fehlt                                                                 |
| --- | ---------- | ----- | -------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| 1   | `WISP`     | 1,5 s | Füller mit Schaden, verkürzt um 0,5 s je Wirkung               | **Die Schule hat keinen Füller** — man kann sie nicht spielen, nur einstreuen  |
| 2   | `DECANT`   | 1,5 s | Füller, verkürzt den **längsten** laufenden Cooldown doppelt   | Zielt statt zu streuen — das ist der Unterschied, der die Bauart tragen könnte |
| 3   | `SUBLIME`  | 6 s   | Der nächste Zauber löst **keine** globale Wartezeit aus        | Wirkt auf die Rotation statt auf Zahlen                                        |
| 4   | `ALEMBIC`  | 10 s  | Zwei Zauber gleichzeitig bereit machen                         | Der Weg „gezielt statt breit"                                                  |
| 5   | `DISTIL`   | 15 s  | Verkürzungen wirken vier Wartezeiten lang doppelt              | Höhepunkt für den Aufbau                                                       |
| 6   | `EFFLUX`   | 20 s  | Wandelt verbleibende Abklingzeit in Schaden                    | Der Notausgang, wenn nichts zu verkürzen ist                                   |
| 7   | `CONDENSE` | 25 s  | Der nächste Zauber wirkt **zweimal**                           | Das, was ein Rücksetzer eigentlich verspricht                                  |
| 8   | `ASCEND`   | 30 s  | Alle Abklingzeiten unter 10 s sofort bereit                    | Trifft viele statt einen                                                       |
| 9   | `RETORT`   | 12 s  | Ein unterbrochener Zauber ist sofort wieder bereit             | Verbindet die Schule mit der Abbruch-Regel                                     |
| 10  | `AETHER`   | 60 s  | Für acht Wartezeiten laufen alle Abklingzeiten doppelt schnell | Der große Moment                                                               |

## Die Frage, die dieses Raster stellt

**Die Bauart „ein bisschen kürzer für alle" ist die gemessen tote.** Sechs der zehn Vorschläge
verlassen sie bewusst: `DECANT` zielt, `SUBLIME` umgeht die Wartezeit, `CONDENSE` verdoppelt,
`EFFLUX` wandelt um.

Wenn das der richtige Weg ist, ist es **stillschweigend eine Antwort auf Punkt 11** — die Schule
behielte ihren Namen und ihr Thema, aber nicht ihre zentrale Mechanik.

**Das gehört Ihnen, nicht mir.** Ich schreibe es dazu, damit es nicht als Nebeneffekt eines
Katalogs passiert.

## COAGULATION — 7 vorhanden, 5 neu

**Mechanik (GAME-DESIGN 176):** Drei Wege nebeneinander — **Schutzschicht** (nimmt Schaden auf,
stapelt, muss VOR dem Schlag stehen), **klassische Heilung** (verzeiht Fehler nachträglich),
**Schaden-in-Schutz-Umwandlung** (wandelt einen Teil des ausgeteilten Schadens laufend um).

## Was steht

| Zauber   | CD   | Rolle             | Weg |
| -------- | ---- | ----------------- | --- |
| `crust`  | 6 s  | Schutzschicht     | 1   |
| `salve`  | 12 s | Heilung           | 2   |
| `temper` | 15 s | Schaden-in-Schutz | 3   |

**Genau ein Zauber je Weg.** Wer COAGULATION spielt, hat keine Wahl innerhalb der Schule — er
nimmt alle drei oder keinen.

## Vorschlag für die neun neuen

| #   | Name            | CD    | Rolle                                                     | Weg   |
| --- | --------------- | ----- | --------------------------------------------------------- | ----- |
| 1   | `SCAB`          | 1,5 s | Füller mit Schaden, legt eine dünne Schicht an            | 1     |
| 2   | `PLATE`         | 20 s  | Große Schicht, aber erst nach zwei Wartezeiten wirksam    | 1     |
| 3   | `HARDEN`        | 10 s  | Bestehende Schichten halten doppelt so lange              | 1     |
| 4   | `POULTICE`      | 8 s   | Heilung über Zeit statt sofort                            | 2     |
| 5   | `MEND`          | 25 s  | Heilt nach verlorenem Leben — je weniger, desto mehr      | 2     |
| 6   | `SUTURE`        | 18 s  | Wandelt Schicht in Heilung um                             | 2 ↔ 1 |
| 7   | `QUICKSILVER`   | 12 s  | Umwandlungsrate steigt für vier Wartezeiten               | 3     |
| 8   | `RIPOSTE`       | 30 s  | Ein Teil des eingehenden Schadens geht zurück an den Boss | 3     |
| 9   | `CRUCIBLE SKIN` | 45 s  | Alle drei Wege gleichzeitig, kurz und stark               | alle  |

## Zwei Anmerkungen

**`SCAB` ist der wichtigste.** Die Schule hat heute **keinen Zauber mit Schaden** — wer sie
mitnimmt, gibt Leistenplätze auf, ohne etwas dafür zu bekommen außer Überleben. Das ist laut
Design gewollt („echte Abwägung Schaden gegen Sicherheit"), aber drei reine Verteidigungszauber
sind eine harte Fassung davon.

> Die Frage dahinter: **Soll COAGULATION überhaupt Schaden machen dürfen?** Wenn nein, streiche
> ich `SCAB` — dann bleibt die Schule bewusst ein reiner Kostenfaktor.

**`RIPOSTE` kollidiert namentlich mit `reflux`** aus SUBLIMATION. Falls er bleibt, heißt er
`RIPOSTE` — der Design-Plan nennt „Spell Reflect" als Beispiel für diese Schule, und das ist die
nähere Übersetzung.
