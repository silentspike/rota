---
id: apl-strict-comparison-2026-10-24
status: Review
ssot: false
refs:
  docs: [APL-STRICT-COMPARISON-2026-09-06.md, GAME-DESIGN.md]
affects:
  [
    sim/src/apl/mod.rs,
    sim/src/fight.rs,
    sim/src/rotation_search.rs,
    sim/tests/apl_strikte_vergleiche.rs,
    sim/tests/parity_gcd_freier_wurf.rs,
    sim/tests/entscheidung_nach_gcd_freiem_wurf.rs,
    sim/tests/parity_strikter_umschlag.rs,
    sim/examples/e1027-operator-measure.rs,
    src/devtools/reference-run.ts,
    tests/parity/recordings/gcd-freier-wurf.json,
    tests/parity/recordings/strikter-umschlag.json,
  ]
---

# Strikte APL-Vergleiche: Fassung B auf dem 10-ms-Takt (#1027)

Gemessen am 24. Oktober 2026 auf `23d7c13fba470ee1d6a677bfdd64d042f05724f6`, am selben Tag zweimal
nachgebessert: nach Abnahme und Wirkungsprüfung, dann nach einer zweiten Wirkungsprüfung. **Am 28.
Oktober nach zwei Behebungen in `sim/src/fight.rs` vollständig neu gefahren** (dritte Nachbesserung,
siehe unten). Auftrag ist die Antwort `f-apl-strikte-vergleiche-2` vom 22.08.: „Erst den
Vorher-Nachher-Lauf, dann entscheiden".

> **Seit dem 28. Oktober entschieden und eingebaut** — Fassung B, das Entscheidungsmodell des
> Orchestrators und eine Nachbesserung der Suche. Was das gegenüber den Messungen unten verschiebt,
> steht im nächsten Abschnitt, [Entschieden und eingebaut](#entschieden-und-eingebaut-28-oktober).
> Die Abschnitte danach beschreiben den Stand **vor** dieser Entscheidung.

**Fassung B war eine Messung, keine Semantikänderung.** Sie lag nur als Wegwerf-Patch im Arbeitsbaum
und ist nicht eingecheckt. `sim/src/apl/mod.rs` hat im Bestand den SHA-256 `377742d1…`; dieser Zweig
ergänzt darin nur Kommentarzeilen (`77388248…`, siehe [Woher der Verzug kommt](#woher-der-verzug-kommt)).
Vor und nach jedem Lauf mit B, M, M2 oder MT steht die Datei wieder auf dem Stand dieses Zweigs, per
SHA-256 geprüft. **Eingecheckt sind dagegen seit der dritten Nachbesserung zwei Behebungen in
`fight.rs`** (beide Fassungen rechnen damit) und die daraus folgende Ordnung im Dreierblock der Suche
(`rotation_search::tripel_zuege`).

Gegenüber dem Versuch vom 06.09. ([APL-STRICT-COMPARISON-2026-09-06.md](APL-STRICT-COMPARISON-2026-09-06.md))
ändert sich zweierlei: Fassung B weckt auf dem **10-ms-Takt** statt auf der Millisekunde, und
gemessen wird über **alle 42 Bossdateien** statt über acht ausgewählte Suchen.

**Die Nachbesserung** beantwortet die Nebenfrage neu — die erste Fassung hatte aus Proben ohne
GCD-freien Zug auf alle vollen Rotationen geschlossen. Dazu kommen ein Wächter für den Weckruf von
`dot.*.remains` und drei Gegenproben im Messprogramm: die Suche mit `<=` in den Nachlegezeilen, die
Nachlegezeilen neben einem GCD-freien Zug gegen die Takt-Referenz und die Wurfreihe jeder gefundenen
Liste unter beiden Fassungen.

**Die zweite Nachbesserung** misst den Weg jeder Suche Schritt für Schritt und an der ersten Gabelung
zwischen A und B beide Listen gegen die Takt-Referenz über den ganzen Kampf (siehe
[Am Weg der Suche](#am-weg-der-suche)). Sie berichtigt die Zuordnung der ersten Nachbesserung: Die
folgenreichste Verschiebung, die der AC5-Suche, entsteht nicht neben einem GCD-freien Zug, sondern an
einer Liste ohne Füller — und dort trifft B die Takt-Referenz, A nicht. Dazu kommt die Mutation MT
(`Value::Time => None`).

**Die dritte Nachbesserung (28.10.)** behebt die beiden Nebenbefunde, die bis dahin „nicht hier
behoben" hießen, und misst sie zuerst am Spiel statt an der Takt-Referenz (siehe
[Nach einem GCD-freien Wurf](#nach-einem-gcd-freien-wurf-am-spiel-gemessen-und-behoben) und
[Leerlauf](#leerlauf-idle_ms-behoben)):

1. **Nach einem Wurf ohne Wartezeit entscheidet der Simulator jetzt im nächsten Takt neu, wie die
   Schleife des Spiels.** Bisher erst am nächsten eingereihten Ereignis.
2. **`idle_ms` zählt jeden wartenden Takt genau einmal**, auch nach einem freien Wurf.

Danach sind alle Messungen dieses Berichts neu gefahren, auf `dea96d1f` (Behebung in `6d3ff723`).
**Die Antwort auf die Frage dieses Berichts ändert sich damit erheblich:** Fassung B verschiebt nur
noch eine von 84 Suchen — die AC5-Suche — statt 15, und keinen der 2394 Bestandsläufe statt 97. Die
14 übrigen Verschiebungen und alle 97 Bestandsläufe hingen an der Entscheidung nach dem freien Wurf,
nicht am Weckruf. Wo Zahlen der ersten beiden Fassungen nicht mehr gelten, sind sie ersetzt; was sich
änderte, steht in [Was die Behebung verschob](#was-die-behebung-verschob).

## Entschieden und eingebaut (28. Oktober)

**Fassung B**, Entscheidung des Auftraggebers vom 28.10. („B, wie das Spiel"): Eine Zahl in einer
Bedingung auf einem Zeitwert greift im Simulator wie im Spiel auf dem 10-ms-Takt. `switch_point`
bekommt den Operator und legt den Weckruf auf den ersten Takt, an dem der Vergleich kippt — der Patch
aus [Was Fassung B hier heißt](#was-fassung-b-hier-heißt), eingebaut und kommentiert.

**Das Entscheidungsmodell**, Festlegung des Orchestrators vom selben Tag an #1027, in der Schleife
des Spiels (`driveRotation`) und im Simulator (`Fight::run_with`), bitgleich:

|     |                                                                                                                                                                                                               |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| (a) | nach einem Wurf wählt die Liste beim selben Zeitstempel weiter, bis sie nichts mehr wählt                                                                                                                     |
| (b) | während einer laufenden Wartezeit kommt ein Zauber ohne Wartezeit zum Zug (QUENCH), auch im Takt des Wurfs, der sie ausgelöst hat; der Simulator weckt dafür innerhalb der Wartezeit (`next_wakeup_ohne_gcd`) |
| (c) | mehr Würfe beim selben Zeitstempel als Zauber im Datensatz (70) sind ein Nullzeit-Zyklus und brechen als Datenfehler ab                                                                                       |

Das ist, was `Session.tick` einem Spieler schon immer erlaubt; `Session` selbst ist unverändert.

**Die Nachbesserung der Suche** für #797-AC5: Die Dreiergruppe (Unterbrecher, Zeile im
Schlagfenster, Erzeuger) darf Zeilen verdrängen, wenn ihr nur der Platz fehlt — auch an der
Längengrenze. Unter B allein trugen die Einzelzüge der AC5-Suche bis Länge 8, mit (a) und (b) bis 10;
beide Male wurde keine einzige Gruppe gefragt. Der Prüffall ist unverändert und grün: FIXATION
`pin`, `bind`, eine Unterbrechung, ohne FIXATION tot. Die Gewichtsreihe trennt wieder bei 1, 2, 5,
10, 20, 50 und 100 (unter B allein: 2, 5, 10, 50, 100).

### Vorher-Nachher über alle 42 Bosse

Dasselbe Messprogramm, viermal gebaut — auf der Basis `a81d35f1`, nach jedem der drei Schritte —
und je einmal gefahren; der erste und der letzte Stand zweimal, bytegleich:

| Stand                   | Bau-Job                    | Ausgabe SHA-256                                                               |
| ----------------------- | -------------------------- | ----------------------------------------------------------------------------- |
| A, Basis                | `c184e941a011937c1d52a403` | `c29fdf9d…` (×2) — **bytegleich mit der A-Ausgabe der dritten Nachbesserung** |
| B                       | `fd87709adfe73ba0fa77665c` | `134a1b10…` — **bytegleich mit der dortigen B-Ausgabe**                       |
| B + (a)(b)(c)           | `52a3b22dd3a8bc8db4f8535a` | `d816fff7…`                                                                   |
| Endstand, mit der Suche | `8b8b08a40fe7e19781bcbeb7` | `76420049…` (×2)                                                              |

Die beiden Gleichheiten oben belegen zweierlei: Was seit der dritten Nachbesserung in `main` kam,
bewegt keine dieser Zahlen, und der eingebaute Weckruf ist der gemessene Patch, nicht ein ähnlicher.

Geänderte Datensätze je Schritt, über dieselben 2833 (Feld `fp` je Paar `(kind, id)`; dieselben Mengen in jedem Stand):

| Art                              |  von | durch B | durch (a)(b)(c) | durch die Suche |
| -------------------------------- | ---: | ------: | --------------: | --------------: |
| Suche (`search`), Liste anders   |   84 |       1 |              40 |               3 |
| Suche, nur Wert oder Lauf anders |   84 |       0 |               4 |               0 |
| Suche mit `<=` (`search_le`)     |   84 |       0 |              70 |               3 |
| Bestandsläufe (`bestand`)        | 2394 |       0 |             204 |               0 |
| GCD-Proben (`gcd`)               |   32 |       2 |              12 |               0 |
| `gcdfrei`                        |    2 |       0 |               1 |               0 |
| Wurfproben (`probe`)             |   84 |      24 |               0 |               0 |
| Zustandsproben (`operator`)      |  144 |      48 |               0 |               0 |
| Raster (`raster`)                |    8 |       4 |               0 |               0 |

- **B** verschiebt genau die AC5-Suche (18.950 → 11.627) und sonst keine; alle 84 Wurfproben treffen
  danach die Takt-Referenz (vorher 60).
- **(a)(b)(c)** verschieben 44 Suchen — jede mit QUENCH oder SUBLIME in der alten oder der neuen
  Liste; der Wert steigt in 21 und fällt in 23 — und 204 Bestandsläufe, alle in den sechs Dateien mit
  einer `quench`-Zeile (`e607p-paar`, `instance1-boss4-unterbrechen`, `instance1-rein`,
  `instance1-unterbrechen` an je 42 Bossen, `e558-paar-bedingt` und `e797-mit-quench` an je 18).
  Die 51 Dateien ohne Zauber ohne Wartezeit ändern sich nicht.
- **Die Suche** verschiebt drei Suchen, alle in der Einstellung `kontrolle` und alle nach oben:
  `boss-1-3` (10.660 → 10.736), `boss-2-4` (11.220 → 11.283) und `test-lethal-strike` (11.807 →
  20.834); jede hat danach die Gruppe `quench if boss.casting >= 1`, `pin if boss.next_strike_in <=
3000`, `bind`.

Gegen das Spiel (`e1027-operator-measure spiel` auf der neuen `gcd-freier-wurf.json`): Der Bau A
entscheidet in 1 von 8 Läufen wie das Spiel, der Endstand in 8 von 8, ereignisgesteuert wie mit
Takt-Referenz, Leerlauf eingeschlossen.

Suchwerte je Boss; **fett** heißt, die gefundene Liste ist eine andere als im Stand davor:

| Boss                     | `bericht` A → B → (a)(b)(c) → Suche | `kontrolle` A → B → (a)(b)(c) → Suche     |
| ------------------------ | ----------------------------------- | ----------------------------------------- |
| `boss-1-1`               | 9001 → 9001 → **9068** → 9068       | 13877 → 13877 → **13746** → 13746         |
| `boss-1-2`               | 7544 → 7544 → 7544 → 7544           | 8459 → 8459 → 8459 → 8459                 |
| `boss-1-3`               | 9364 → 9364 → 9364 → 9364           | 10520 → 10520 → **10660** → **10736**     |
| `boss-1-4`               | 8233 → 8233 → **8130** → 8130       | 9557 → 9557 → **9824** → 9824             |
| `boss-2-1`               | 8843 → 8843 → **9068** → 9068       | 13420 → 13420 → **15974** → 15974         |
| `boss-2-2`               | 7750 → 7750 → **8451** → 8451       | 15134 → 15134 → **14282** → 14282         |
| `boss-2-3`               | 11392 → 11392 → 11392 → 11392       | 12903 → 12903 → **13552** → 13552         |
| `boss-2-4`               | 8312 → 8312 → **8169** → 8169       | 11813 → 11813 → **11220** → **11283**     |
| `boss-3-1`               | 7744 → 7744 → 7744 → 7744           | 11882 → 11882 → 11882 → 11882             |
| `boss-3-2`               | 9364 → 9364 → 9364 → 9364           | 9798 → 9798 → **9768** → 9768             |
| `boss-3-3`               | 9364 → 9364 → 9364 → 9364           | 12448 → 12448 → 12448 → 12448             |
| `boss-3-4`               | 8721 → 8721 → 8721 → 8721           | 9776 → 9776 → **9954** → 9954             |
| `boss-4-1`               | 11224 → 11224 → 11224 → 11224       | 11632 → 11632 → **11836** → 11836         |
| `boss-4-2`               | 9364 → 9364 → 9364 → 9364           | 9798 → 9798 → **9446** → 9446             |
| `boss-4-3`               | 10553 → 10553 → 10553 → 10553       | 15108 → 15108 → **15165** → 15165         |
| `boss-4-4`               | 9364 → 9364 → 9364 → 9364           | 11803 → 11803 → **11953** → 11953         |
| `boss-5-1`               | 9364 → 9364 → 9364 → 9364           | 14446 → 14446 → **14461** → 14461         |
| `boss-5-2`               | 8550 → 8550 → 8550 → 8550           | 8977 → 8977 → **9467** → 9467             |
| `boss-5-3`               | 10922 → 10922 → 10922 → 10922       | 12993 → 12993 → **12873** → 12873         |
| `boss-5-4`               | 8721 → 8721 → 8721 → 8721           | 9574 → 9574 → 9474 → 9474                 |
| `first-crucible`         | 9364 → 9364 → 9364 → 9364           | 9798 → 9798 → **9446** → 9446             |
| `orch-maxload`           | 3182 → 3182 → **1938** → 1938       | 139 → 139 → 139 → 139                     |
| `test-caster`            | 48672 → 48672 → 48672 → 48672       | 50760 → 50760 → 49915 → 49915             |
| `test-collision`         | 9364 → 9364 → 9364 → 9364           | 12948 → 12948 → 12948 → 12948             |
| `test-crucible-trial`    | 18990 → 18990 → 18990 → 18990       | 23828 → 23828 → **23239** → 23239         |
| `test-doomsayer`         | 5921 → 5921 → 5921 → 5921           | 6047 → 6047 → **6068** → 6068             |
| `test-gatekeeper`        | 18990 → 18990 → 18990 → 18990       | 19660 → 19660 → **19215** → 19215         |
| `test-lapse`             | 8793 → 8793 → **9068** → 9068       | 10172 → 10172 → **9878** → 9878           |
| `test-lapse-interrupt`   | 9364 → 9364 → 9364 → 9364           | 10222 → 10222 → **9878** → 9878           |
| `test-lethal-strike`     | 8440 → 8440 → 8440 → 8440           | 18950 → **11627** → **11807** → **20834** |
| `test-metronome`         | 48672 → 48672 → 48672 → 48672       | 51730 → 51730 → **51091** → 51091         |
| `test-onslaught`         | 9014 → 9014 → 9014 → 9014           | 14233 → 14233 → **14246** → 14246         |
| `test-phase`             | 48672 → 48672 → 48672 → 48672       | 50760 → 50760 → 49915 → 49915             |
| `test-phasenbindung`     | 48672 → 48672 → 48672 → 48672       | 50760 → 50760 → 49915 → 49915             |
| `test-racer`             | 17798 → 17798 → **17976** → 17976   | 19536 → 19536 → **19237** → 19237         |
| `test-resistant`         | 37902 → 37902 → 37902 → 37902       | 39651 → 39651 → **39004** → 39004         |
| `test-runaway`           | 3361 → 3361 → 3361 → 3361           | 3361 → 3361 → 3361 → 3361                 |
| `test-stacked`           | 7504 → 7504 → 7504 → 7504           | 8274 → 8274 → **8133** → 8133             |
| `test-survivable-strike` | 10244 → 10244 → 10244 → 10244       | 11619 → 11619 → **11680** → 11680         |
| `test-vessel`            | 14047 → 14047 → 14047 → 14047       | 15705 → 15705 → **17792** → 17792         |
| `test-vulnerable`        | 59390 → 59390 → 59390 → 59390       | 61275 → 61275 → **61750** → 61750         |
| `test-warden`            | 9364 → 9364 → 9364 → 9364           | 10278 → 10278 → **9926** → 9926           |

## Ergebnis

- **Fassung B trifft die Takt-Referenz in jeder Probe.** In 84 Wurfproben (7 Aufbauten mit 6
  verschiedenen zeitabhängigen Werten × 6 Vergleichsformen × 2 Operandenreihenfolgen) wirft Fassung B
  genau dort, wo ein an jedem Takt entscheidender Lauf wirft; Fassung A verfehlt 24 der 84 (18
  verspätet, 6 fallen ganz aus) — vor und nach der Behebung dieselben (Zustands-, Wurf- und
  Rasterproben alt gegen neu: 0 Änderungen). In den 32 GCD-Proben trifft B 32, A 30; vor der Behebung
  lagen in den 18 Proben mit GCD-freiem Zug beide in 13 daneben. In allen protokollierten Kämpfen des
  Messprogramms liegt unter A wie unter B kein Eintrag neben dem 10-ms-Raster.
- **Die Takt-Referenz trifft das Spiel.** Gegen sechs Läufe aus der Schleife des Spiels selbst
  (`gcd-freier-wurf.json`) wirft sie in allen sechs Wurf für Wurf wie das Spiel, auch mit dem `fight.rs`
  von vor der Behebung; der ereignisgesteuerte Lauf traf es vorher in einem, jetzt in allen sechs.
- **Betroffen sind unter A nicht, wie bisher angenommen, `<` und `>=`**, sondern bei fallenden
  Restzeiten `<`, sein Spiegel `T > v` und `!=`, bei `time` `>`, sein Spiegel `T < time` und `!=`.
  Nur diese Formen verpassen ihren Umschlag, in 24 der 28 Wurfproben mit ihnen; `>=` trifft in allen
  Aufbauten. `!=` war bisher nicht gemessen.
- **Gefundene Rotationen:** Unter B verschiebt sich die gefundene Liste in **1 von 84 Suchen**, der
  AC5-Suche (vor der Behebung 15). In der Einstellung des Suchberichts (`rotation-search.rs`) sind es
  **0 von 42 Bossen** (vorher 2), in der Einstellung des AC5-Prüffalls **1 von 42 (2,4 %)** (vorher
  13). Keine der 84 A-Listen zählt unter B anders (vorher 7); in der Nachbarschaft der gefundenen
  Listen verschieben sich 32 von 59.336 Bewertungen (0,05 %, vorher 9,06 %), alle in Listen ohne
  Füller. Dieselbe Suche mit `<=` in den Nachlegezeilen verschiebt sich unter B in **0 von 84**
  Fällen.
- **Der #797-AC5-Prüffall fällt unter B weiter**: 2 bestanden, 1 rot
  (`ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist`), 1 ignoriert; unter A bleibt er
  grün. Die gefundene Liste ist zeilengleich mit der „Experiment"-Liste des 1-ms-Versuchs vom 06.09.
  Dieselbe A-Liste wendet den Schlag auch unter B ab (18.950 in beiden) — die gierige Suche erreicht
  sie nur nicht mehr. **Ihr Weg trennt sich in Schritt 3, an einer Liste ohne Füller und ohne
  GCD-freien Zug:** `searing if dot.searing.remains < 1500`, `plate`, `crust` zählt unter A 4740,
  unter B 4560, und B trifft dort die Takt-Referenz, A nicht. Unter B gewinnt stattdessen `kindle` am
  Ende (4700). In der Gewichtsreihe trennt B jetzt bei 2, 5, 10, 50 und 100, **nicht mehr bei 20**.
- **Nebenfrage `GCD_MS`: nach der Behebung folgenlos, außer ohne Füller.** Die Nachlegezeilen
  `< 1500` sind die einzige Stelle, an der B die Suche verschiebt. Mit Füller wirken sie nicht mehr,
  mit und ohne GCD-freien Zug (0 von 59.128 Nachbarn mit Füller, 0 von 2394 Bestandsläufen, 18 von 18
  Proben mit freiem Zug A = B = Takt-Referenz). **Ohne Füller** wartet die Rotation am Umschlag; B
  legt einen Takt danach nach, wie die Takt-Referenz, A erst am Ablauf — an `first-crucible` verliert
  SEARING dabei einen Tick (1260 → 1170 Schaden in 30 s), und genau so trennt sich der Weg der
  AC5-Suche, der einzigen, die sich noch verschiebt.
- **Die zweite Lage der zweiten Nachbesserung gibt es nicht mehr.** Neben einem GCD-freien Wurf, nach
  dem eine Nachlegezeile umschlägt, trennten sich vorher 14 Suchen und alle 7 A-Listen, die unter B
  anders zählten. Das war der Fehler in `fight.rs`: Nach einem freien Wurf entschied der Simulator
  erst am nächsten Ereignis neu, das Spiel im nächsten Takt. **Gemessen am Spiel und behoben** (siehe
  [Nach einem GCD-freien Wurf](#nach-einem-gcd-freien-wurf-am-spiel-gemessen-und-behoben)); A und B
  entscheiden dort jetzt gleich.
- **`idle_ms` behoben:** Spiel und Simulator zählen den Leerlauf gleich (6 von 6 Läufen der
  Aufzeichnung). Die Takt-Referenz zu `smoulder if dot.smoulder.remains < 1500` meldet 25.500 statt
  1.846.410 ms in 30 s.
- **Handgeschriebener Bestand:** Unter B ändert sich keiner der 2394 Läufe (vorher 97, alle in den 6
  Dateien mit `quench`).
- **Die Behebung verschiebt A selbst** — und B ebenso: 37 der 84 Suchen und 199 der 2394
  Bestandsläufe ändern sich unter A, jede Suche mit einem GCD-freien Zug in der alten oder der neuen
  Liste, jeder Bestandslauf in einer der 6 Dateien mit `quench` (siehe
  [Was die Behebung verschob](#was-die-behebung-verschob)). Für die Wahl zwischen A und B ist das
  kein Preis einer Fassung; es ist der Unterschied zwischen dem alten und dem behobenen Simulator.

Was das für die Entscheidung heißt, steht unter [Für die Entscheidung](#für-die-entscheidung).

## Was Fassung B hier heißt

`switch_point` bekommt den Vergleichsoperator und legt den Weckruf auf den ersten 10-ms-Takt, an dem
der Vergleich tatsächlich kippt. Bei vertauschten Seiten wird die Form gespiegelt (`T < v` ist
`v > T`). Fallende Restzeiten kippen bei `<` und `>=` mit dem Zielwert `T - 1`, bei `>` und `<=` mit
`T`, bei `==` und `!=` an beiden; die steigende Uhr entsprechend bei `T` bzw. `T + 1`. Der Zeitpunkt
wird auf den nächsten Takt aufgerundet, weil das Spiel seinen Zustand nur an Takt-Grenzen prüft
(`sim/src/time.rs`, `TICK_MS = 10`).

Unverändert wie am 06.09.: Wahrheitswerte (`cooldown.*.ready`, `dot.*.active`), Verstärkungen,
Vergleiche zweier zeitabhängiger Seiten und die GCD-Sperre in `fight.rs`. Fassung B ändert `fight.rs`
nicht; seit der dritten Nachbesserung rechnen **beide** Fassungen mit dem behobenen `fight.rs`
(Entscheidung einen Takt nach einem freien Wurf, Leerlauf bis zur nächsten Entscheidung).

Der Patch (Datei-SHA-256 `d95aada2…`, erzeugt aus dem Bestand, reproduziert B bytegenau mit dem
SHA-256 `3680b9c5…`). Im Text unten hat prettier die Leerzeichen der leeren Kontextzeilen entfernt;
`git apply` ergibt aus diesem Text trotzdem bytegenau `3680b9c5…` (geprüft):

<details>
<summary>Fassung B, 126 Zeilen</summary>

```diff
--- a/sim/src/apl/mod.rs
+++ b/sim/src/apl/mod.rs
@@ -121,7 +121,7 @@

 use crate::data::{Spell, MAX_LEVEL, RESOURCE, SPELLS};
 use crate::fight::Fight;
-use crate::time::{Millis, GCD_MS};
+use crate::time::{Millis, GCD_MS, TICK_MS};

 /// Eine Zahl, wie die Sprache sie kennt: ganzzahlig, mit Vorzeichen.
 ///
@@ -382,6 +382,18 @@
 }

 impl Compare {
+    /// FASSUNG B (#1027, Wegwerf-Patch): die Form bei vertauschten Seiten.
+    const fn reversed(self) -> Self {
+        match self {
+            Compare::Less => Compare::Greater,
+            Compare::LessEqual => Compare::GreaterEqual,
+            Compare::Greater => Compare::Less,
+            Compare::GreaterEqual => Compare::LessEqual,
+            Compare::Equal => Compare::Equal,
+            Compare::NotEqual => Compare::NotEqual,
+        }
+    }
+
     #[must_use]
     const fn apply(self, left: Number, right: Number) -> bool {
         match self {
@@ -515,7 +527,10 @@

         for rule in &self.rules {
             for term in rule.condition.terms() {
-                for (value, other) in [(term.left, term.right), (term.right, term.left)] {
+                for (value, other, compare) in [
+                    (term.left, term.right, term.compare),
+                    (term.right, term.left, term.compare.reversed()),
+                ] {
                     if !value.is_time_dependent() {
                         continue;
                     }
@@ -529,7 +544,7 @@
                     if other.is_time_dependent() {
                         continue;
                     }
-                    if let Some(at) = switch_point(value, other.read(fight), fight) {
+                    if let Some(at) = switch_point(value, other.read(fight), compare, fight) {
                         if at > now {
                             earliest = Some(earliest.map_or(at, |best| best.min(at)));
                         }
@@ -567,16 +582,43 @@
 /// Wert an der Uhr?" statt an der Frage „wann schlägt er um?". Die zweite ist
 /// die, deren Antwort gebraucht wird. Ohne Auffang-Zweig muss jeder neue Wert
 /// hier beantwortet werden — auch mit `None`, dann aber sichtbar und begründet.
-fn switch_point(value: Value, threshold: Number, fight: &Fight) -> Option<Millis> {
+fn switch_point(value: Value, threshold: Number, compare: Compare, fight: &Fight) -> Option<Millis> {
     let now = fight.time();
-    let at = |remaining_now: Millis, target: Number| -> Option<Millis> {
-        // Ein Restwert sinkt je Millisekunde um eins: Er erreicht `target` in
-        // `remaining_now - target` Millisekunden.
-        let delta = Number::from(remaining_now) - target;
-        if delta <= 0 {
-            return None;
+    // FASSUNG B (#1027, Wegwerf-Patch, nie einchecken). Der Weckruf liegt auf
+    // dem ersten Takt, an dem der Vergleich kippt: Das Spiel prueft seinen
+    // Zustand nur an Takt-Grenzen (time::TICK_MS).
+    let auf_takt = |t: Millis| -> Option<Millis> {
+        let rest = t % TICK_MS;
+        if rest == 0 {
+            Some(t)
+        } else {
+            t.checked_add(TICK_MS - rest)
         }
-        u32::try_from(delta).ok().map(|d| now.saturating_add(d))
+    };
+    // Fallender Wert v(now + d) = remaining_now - d. Er kippt, wenn v den
+    // Zielwert erreicht: < und >= bei threshold - 1, > und <= bei threshold,
+    // == und != bei beiden. Restwerte enden bei 0; ein negativer Zielwert wird
+    // nie erreicht und darf keinen Weckruf stellen.
+    let at = |remaining_now: Millis, threshold: Number| -> Option<Millis> {
+        let ziele: [Option<Number>; 2] = match compare {
+            Compare::Less | Compare::GreaterEqual => [threshold.checked_sub(1), None],
+            Compare::Greater | Compare::LessEqual => [Some(threshold), None],
+            Compare::Equal | Compare::NotEqual => [Some(threshold), threshold.checked_sub(1)],
+        };
+        ziele
+            .into_iter()
+            .flatten()
+            .filter_map(|ziel| {
+                if ziel < 0 {
+                    return None;
+                }
+                let delta = Number::from(remaining_now).checked_sub(ziel)?;
+                if delta <= 0 {
+                    return None;
+                }
+                auf_takt(now.checked_add(u32::try_from(delta).ok()?)?)
+            })
+            .min()
     };

     match value {
@@ -591,8 +633,20 @@
             (remains > 0).then(|| now.saturating_add(remains))
         }
         Value::Time => {
-            let target = u32::try_from(threshold).ok()?;
-            (target > now).then_some(target)
+            // Steigende Uhr: < und >= kippen bei threshold, > und <= bei
+            // threshold + 1, == und != bei beiden.
+            let ziele: [Option<Number>; 2] = match compare {
+                Compare::Less | Compare::GreaterEqual => [Some(threshold), None],
+                Compare::Greater | Compare::LessEqual => [threshold.checked_add(1), None],
+                Compare::Equal | Compare::NotEqual => [Some(threshold), threshold.checked_add(1)],
+            };
+            ziele
+                .into_iter()
+                .flatten()
+                .filter_map(|ziel| u32::try_from(ziel).ok())
+                .filter_map(auf_takt)
+                .filter(|&ziel| ziel > now)
+                .min()
         }
         Value::FightRemains => at(fight.fight_remaining_ms(), threshold),
         // Der Wert, für den diese Funktion exhaustiv gemacht wurde (#797). Er
```

</details>

Für die offene Frage aus `apl_strikte_vergleiche.rs` lief zusätzlich die **Mutation M**: der Versuch
vom 22.08., `Value::BossNextStrikeIn => None` (Patch-SHA-256 `49f3e394…`, ergibt `7e77a7af…`). Für den
Wächter des zweiten Werts lief die **Mutation M2**: `Value::DotRemains(_) => None`. Sie entsteht aus
dem Bestand, indem die Zeile `Value::DotRemains(i) => at(fight.dot_remaining_ms(i), threshold),`
ersetzt und davor die Kommentarzeile `// MUTATION M2 (#1027, Wegwerf): kein Weckruf fuer dot.*.remains.`
eingefügt wird; das ergibt `6d6f3cce…`. Die reine Ersetzung ohne Kommentarzeile ergäbe `2c6b0b81…` und
ist nicht gefahren. Für die `time`-Tabelle lief die **Mutation MT**: Die vier Zeilen des Zweigs
`Value::Time => { … }` werden durch die eine Zeile `Value::Time => None,` ersetzt, ohne Kommentarzeile
(ergibt `7b75ef22…`).

## Aufbau und Befehle

Das Messprogramm `sim/examples/e1027-operator-measure.rs` (SHA-256 `30c4bce0…`; in der ersten Fassung
`802bbea3…`) ist für alle drei Fassungen dieselbe Datei. Gebaut wurde remote, gefahren lokal nach
`-c`; alle Aufrufe aus der Worktree-Wurzel:

```bash
cargo remote -c -- build --release --example e1027-operator-measure   # je Fassung
M=./target/release/examples/e1027-operator-measure                    # nach /work/tmp gesichert
$M > a.jsonl            # Bestand; dasselbe mit dem B- und dem M-Bau
$M kreuz a.jsonl b.jsonl > kreuz-a.jsonl      # mit dem A-Bau; kreuz-b.jsonl mit dem B-Bau
$M vergleich a.jsonl b.jsonl kreuz-a.jsonl kreuz-b.jsonl > vergleich-ab.json
$M vergleich a.jsonl m.jsonl > vergleich-am.json
$M vergleich a.jsonl a2.jsonl > vergleich-aa.json   # ebenso b.jsonl gegen b2.jsonl
# zweite Nachbesserung
$M pfad a.jsonl > pfad-a.jsonl          # mit dem A-Bau; pfad b.jsonl > pfad-b.jsonl mit dem B-Bau
$M pfadkreuz pfad-a.jsonl pfad-b.jsonl > pfadkreuz-a.jsonl   # mit dem A-Bau; -b mit dem B-Bau
$M pfadvergleich pfad-a.jsonl pfad-b.jsonl pfadkreuz-a.jsonl pfadkreuz-b.jsonl > pfadvergleich.json
$M nachbarn a.jsonl kreuz-a.jsonl kreuz-b.jsonl > nachbarn.json
$M proben > proben-a.jsonl              # mit dem A-Bau; proben-mt.jsonl mit dem MT-Bau
```

| Fassung | `mod.rs` SHA-256 | Bau-Job                    | Binary SHA-256 | Ausgabe SHA-256 | Wiederholung |
| ------- | ---------------- | -------------------------- | -------------- | --------------- | ------------ |
| A       | `77388248…`      | `1bfbf0cf64f23e80456b614f` | `f31a5412…`    | `f5875952…`     | bytegleich   |
| B       | `3680b9c5…`      | `43308c02eef28b1769388715` | `df51c2ee…`    | `b81df7dc…`     | bytegleich   |
| M       | `7e77a7af…`      | `0147db42ab9ba6f85447cead` | `675ae7ef…`    | `16d73862…`     | —            |

A ist der Stand dieses Zweigs. B und M entstehen aus dem Bestand `377742d1…`, B mit dem Patch oben, M
mit dem Patch `49f3e394…`; nach jedem Bau steht `mod.rs` wieder auf `77388248…`.

Alle fünf Messläufe (A, A wiederholt, B, B wiederholt, M): Exit 0, keine Fehlerausgabe, je 2833
Datensätze — 1 Metadatensatz, 144 Zustandsproben, 84 Wurfproben, 8 Rasterproben, 32 GCD-Proben, 2
Proben `gcdfrei`, 2394 Bestandsläufe, 84 Suchen und 84 Suchen `search_le`. Beide Kreuzläufe: Exit 0,
je 84 Datensätze. Die Bosszahl liest das Programm zweimal und verlangt Gleichheit: `BOSS_COUNT` = 42
und 42 Dateien in `data/bosses/`. A gegen A und B gegen B ergeben in allen 2833 Datensätzen 0
Änderungen. Der Vergleich bricht ab, wenn sich Fallmengen unterscheiden oder die Kreuzbewertung die
eigene Suche nicht bitgleich trifft. Er zählt außerdem, ob sich ein Wahrheitswert ändert (0 Fälle)
und ob die Takt-Referenz unter A und B verschieden ausfällt (0 Fälle, auch in den neuen Proben).

**Die erste Messung steht unverändert darin.** Die 2731 Datensätze der ersten Fassung (Ausgabe
`8f473dfe…`) kommen in der neuen A-Ausgabe gleich heraus, in derselben Reihenfolge, wenn man die neuen
Felder `idle_ms_reference`, `gcd_free_lines` und `strict_time_terms` weglässt — mit dem kommentierten
`mod.rs` und dem erweiterten Programm. Der A-Bau vor `cargo-remote-fmt` (Job
`a81f35ab1d047a6e0766e8a0`) liefert dieselbe Ausgabe wie der danach. `vergleich` ergibt für die
Tabellen der ersten Fassung dieselben Felder wie damals; sie gelten weiter.

**Die zweite Nachbesserung** erweitert das Programm um die Unterbefehle `pfad`, `pfadkreuz`,
`pfadvergleich`, `nachbarn` und `proben` (SHA-256 jetzt `dff9f823…`). Drei Bauten, alle aus dieser
Datei:

| Fassung | `mod.rs` SHA-256 | Bau-Job                    | Binary SHA-256 |
| ------- | ---------------- | -------------------------- | -------------- |
| A       | `77388248…`      | `0e651f6a6a6c483482618cff` | `f06e0871…`    |
| B       | `3680b9c5…`      | `d88780732e52e41abbb4c1c6` | `a35e3037…`    |
| MT      | `7b75ef22…`      | `c1195f1d1cc0352561045957` | `6c282803…`    |

**Die Erweiterung ändert keine bisherige Zahl.** Mit den neuen Bauten sind die Messung unter A
(`f5875952…`) und unter B (`b81df7dc…`) und beide Kreuzläufe (`6474ba12…`, `b176c8b9…`) bytegleich mit
denen der ersten Nachbesserung; `vergleich` über deren Eingaben ergibt bytegleich `cfa8cc45…`. Der
Weg (`pfad`) hat je Bau 672 Datensätze (42 Suchen mit Grenze 1 bis 6, 42 mit Grenze 1 bis 10) und
endet in allen 84 Suchen bitgleich bei Liste und Suchwert der Messung; sonst bräche er ab. Wiederholt
ist er bytegleich, unter A (`65958f05…`) wie unter B (`f97b51a1…`), auch mit einem zweiten Bau.

**Die dritte Nachbesserung** baut dasselbe Programm (jetzt mit dem Unterbefehl `spiel`, SHA-256
`d09c9aa2…`, Stand `dea96d1f`) gegen das behobene `fight.rs`:

```bash
$M spiel tests/parity/recordings/gcd-freier-wurf.json > spiel-a.jsonl   # je Bau
bash /work/tmp/w1r-1027/skripte/kette.sh   # alle Läufe oben in dieser Reihenfolge, je mit Exit-Code
```

| Bau                                             | `mod.rs`    | `fight.rs`  | Bau-Job                    | Binary SHA-256 | Ausgabe SHA-256         |
| ----------------------------------------------- | ----------- | ----------- | -------------------------- | -------------- | ----------------------- |
| A                                               | `77388248…` | `c6a5670c…` | `251c4d728133a98bb7f276c3` | `1c4b0ae1…`    | `c29fdf9d…` (×3 gleich) |
| B                                               | `3680b9c5…` | `c6a5670c…` | `6518b96f5946e790d7908f51` | `1bf800af…`    | `134a1b10…` (×3 gleich) |
| A0: altes `fight.rs` und `rotation_search.rs`   | `77388248…` | `57a25599…` | `81c410c361a2032e75fc4732` | `7aec5055…`    | `f5875952…` = alte A    |
| A1: behobenes `fight.rs`, alte Ordnung im Block | `77388248…` | `c6a5670c…` | `c0da09cebca7b2df03caaabf` | `897893f0…`    | —                       |

„×3 gleich": A und B je zweimal mit dem ersten Bau vor `cargo-remote-fmt` (Jobs
`793c6f44a84d1631acad8274`, `e5f36f09b7e45af7fa676554`) und einmal mit dem formatierten Bau oben,
alle bytegleich. Alle Läufe Exit 0 und ohne Fehlerausgabe, je 2833 Datensätze; A gegen A und B gegen
B je 0 Änderungen. Kreuzläufe je 84, Weg je 672 Datensätze.

**`idle_ms` steht in keinem Fingerabdruck**, damit die Kennzahlen über die Behebung hinweg dieselbe
Frage beantworten. Gezählt wird er seit der Behebung richtig (siehe [Leerlauf](#leerlauf-idle_ms-behoben)).

## Zustandsproben: Wahrheitswert und Weckruf

An vier Werten, die nicht an der Kampfdauer hängen, je einen Takt vor, auf und nach dem Umschlag
(144 Proben): Der **Wahrheitswert ändert sich in keiner Probe**, der Weckruf in 48. Das Muster ist
an allen vier Werten dasselbe, hier an `dot.smoulder.remains` (Umschlag 10500, Schwelle 1500); die
gespiegelten Formen (`1500 > v` wie `v < 1500`) verhalten sich wie ihr Gegenstück:

| Zeitpunkt | Form                  | A: Weckruf | B: Weckruf |
| --------- | --------------------- | ---------- | ---------- |
| 10490     | `<`, `>=`             | 10500      | 10510      |
| 10490     | `<=`, `>`, `==`, `!=` | 10500      | 10500      |
| 10500     | `<`, `>=`, `==`, `!=` | keiner     | 10510      |
| 10500     | `<=`, `>`             | keiner     | keiner     |
| 10510     | alle sechs            | keiner     | keiner     |

**Am Umschlag stellt A für keine Form einen Weckruf** — auch nicht für `<` und `!=`, die dort falsch
sind und einen Takt später wahr werden. Das ist der fehlende zweite Weckruf, direkt gemessen.

## Würfe aller sechs Formen gegen die Takt-Referenz

Die Messlatte ist kein hergeleiteter Zeitpunkt, sondern ein zweiter Lauf: dieselbe Liste plus eine
angehängte Zeile, die nie greift und für jeden Takt des Fensters `time >= t and time < 0` trägt.
Sie weckt an jedem Takt und wirft nie; `>=` auf der steigenden Uhr kippt in A und B bei `t`, die
Referenz hängt also an keiner Fassung (gemessen: unter A und B dieselben Würfe).

| Wert                     | Boss                     | Umschlag | A trifft | B trifft | verspätet unter A                 |
| ------------------------ | ------------------------ | -------: | -------: | -------: | --------------------------------- |
| `boss.next_strike_in`    | `test-survivable-strike` |     9000 |   8 / 12 |  12 / 12 | `<`, `3000 >`, `!=` ×2 (10000)    |
| `boss.next_strike_in`    | `boss-1-2`               |     7000 |  10 / 12 |  12 / 12 | `<`, `3000 >` (9000)              |
| `dot.smoulder.remains`   | `first-crucible`         |    10500 |   8 / 12 |  12 / 12 | `<`, `1500 >`, `!=` ×2 (12000)    |
| `boss.cast_remains`      | `boss-1-2`               |    27500 |  10 / 12 |  12 / 12 | `<`, `500 >`: wirft nie           |
| `cooldown.purge.remains` | `test-survivable-strike` |     6000 |   8 / 12 |  12 / 12 | `<`, `1500 >`, `!=` ×2: wirft nie |
| `fight.remains`          | `test-survivable-strike` |    57000 |   8 / 12 |  12 / 12 | `<`, `3000 >`, `!=` ×2 (58000)    |
| `time`                   | `test-survivable-strike` |     9000 |   8 / 12 |  12 / 12 | `>`, `9000 <`, `!=` ×2 (10000)    |
| **Summe**                |                          |          |  60 / 84 |  84 / 84 | 24                                |

Unter B liegen alle 24 Würfe, die A verfehlt, genau auf dem ersten Referenzwurf (9010, 7010, 10510,
27510, 6010, 57010, 9010). Bei `boss-1-2` trifft `!=` auch unter A (beide 9000). „Wirft nie" heißt:
Das Fenster schließt sich, bevor ein fremdes Ereignis weckt — der Cast landet bei 28000, `purge`
wird bei 7500 wieder gewirkt.

## Gefundene Rotationen je Boss

Zwei Sucheinstellungen, beide aus dem Bestand übernommen, je Boss eine gierige Suche
(`rotation_search::greedy`, alle Kandidaten, `stop_on_kill: false`, Startwert 0, ein Thread):

- `bericht` wie `config()` in `sim/examples/rotation-search.rs`: 50 Durchgänge, Länge 6, Gewicht 0 —
  ohne dessen bossübergreifenden Nachlauf
- `kontrolle` wie `konfiguration()` in `sim/tests/ac5_kontrolle_lohnt_sich.rs`: Gewicht 1 auf
  Verhindertes, Länge 10

**Eine Rotation gilt als verschoben, wenn die gefundene Liste eine andere ist.** In allen 84 Suchen
fallen zwischen A und B Liste, Suchwert und Nachlauf-Fingerabdruck zusammen: Wo sich eines ändert,
ändern sich alle drei.

| Einstellung | Bosse | Liste verschoben | Anteil | A-Liste unter B anders | Nachbarn verschoben | vor der Behebung: verschoben / A-Liste / Nachbarn |
| ----------- | ----: | ---------------: | -----: | ---------------------: | ------------------- | ------------------------------------------------- |
| `bericht`   |    42 |                0 |  0,0 % |                      0 | 24 / 25790 (0,09 %) | 2 / 2 / 1490 von 25790                            |
| `kontrolle` |    42 |                1 |  2,4 % |                      0 | 8 / 33546 (0,02 %)  | 13 / 5 / 3869 von 33373                           |
| zusammen    |    84 |                1 |  1,2 % |                      0 | 32 / 59336 (0,05 %) | 15 / 7 / 5359 von 59163                           |

**Die einzige verschobene Suche ist die AC5-Suche** (`kontrolle/test-lethal-strike`, 18.950 → 11.627,
siehe [AC5](#der-797-ac5-prüffall-unter-fassung-b)). Keine der 84 unter A gefundenen Listen zählt unter B
anders, auch die AC5-Liste nicht (18.950 in beiden Fassungen). Die 14 übrigen Verschiebungen und alle
7 A-Listen, die vorher unter B anders zählten, hingen an der Entscheidung nach dem freien Wurf; seit sie
behoben ist, entscheiden A und B dort gleich. Die Nachbarschaften der A-Listen sind andere als vorher
(59.336 statt 59.163 Bewertungen), weil die Behebung die A-Listen selbst verschiebt (siehe
[Was die Behebung verschob](#was-die-behebung-verschob)); welche Nachbarn sich unter B noch bewegen,
steht unter [Am Weg der Suche](#am-weg-der-suche).

- **„A-Liste unter B anders"** bewertet die unter A gefundene Liste unter B — dieselbe Rotation,
  andere Semantik. Das trennt „die Zahl einer bestehenden Rotation verschiebt sich" von „die Suche
  wählt eine andere".
- **„Nachbarn"** sind genau die Listen, die der nächste Suchschritt ansähe: jede Zeile der A-Liste
  einmal entfernt, jeder übrige Kandidat an jeder Stelle eingefügt. Sie zeigen, wie empfindlich die
  Bewertung in der Nähe des Gefundenen ist.

Je Boss, A und B beide mit dem behobenen `fight.rs` (Werte sind Suchwerte; „gleich" heißt dieselbe
Liste mit demselben Wert):

| Boss                     | `bericht` A → B | `kontrolle` A → B   | A-Liste unter B (`bericht` / `kontrolle`) | Nachbarn verschoben `bericht` | Nachbarn verschoben `kontrolle` | Bestand verschoben |
| ------------------------ | --------------- | ------------------- | ----------------------------------------- | ----------------------------: | ------------------------------: | -----------------: |
| `boss-1-1`               | 9.001 (gleich)  | 13.877 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 923 (0,0 %) |             0 / 57 |
| `boss-1-2`               | 7.544 (gleich)  | 8.459 (gleich)      | gleich / gleich                           |               1 / 622 (0,2 %) |                 0 / 934 (0,0 %) |             0 / 57 |
| `boss-1-3`               | 9.364 (gleich)  | 10.520 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `boss-1-4`               | 8.233 (gleich)  | 9.557 (gleich)      | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 695 (0,0 %) |             0 / 57 |
| `boss-2-1`               | 8.843 (gleich)  | 13.420 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `boss-2-2`               | 7.750 (gleich)  | 15.134 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 923 (0,0 %) |             0 / 57 |
| `boss-2-3`               | 11.392 (gleich) | 12.903 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 1 / 615 (0,2 %) |             0 / 57 |
| `boss-2-4`               | 8.312 (gleich)  | 11.813 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `boss-3-1`               | 7.744 (gleich)  | 11.882 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 1 / 923 (0,1 %) |             0 / 57 |
| `boss-3-2`               | 9.364 (gleich)  | 9.798 (gleich)      | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `boss-3-3`               | 9.364 (gleich)  | 12.448 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 1 / 923 (0,1 %) |             0 / 57 |
| `boss-3-4`               | 8.721 (gleich)  | 9.776 (gleich)      | gleich / gleich                           |               1 / 622 (0,2 %) |                 1 / 782 (0,1 %) |             0 / 57 |
| `boss-4-1`               | 11.224 (gleich) | 11.632 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 695 (0,0 %) |             0 / 57 |
| `boss-4-2`               | 9.364 (gleich)  | 9.798 (gleich)      | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `boss-4-3`               | 10.553 (gleich) | 15.108 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 923 (0,0 %) |             0 / 57 |
| `boss-4-4`               | 9.364 (gleich)  | 11.803 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `boss-5-1`               | 9.364 (gleich)  | 14.446 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 1 / 923 (0,1 %) |             0 / 57 |
| `boss-5-2`               | 8.550 (gleich)  | 8.977 (gleich)      | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 695 (0,0 %) |             0 / 57 |
| `boss-5-3`               | 10.922 (gleich) | 12.993 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 695 (0,0 %) |             0 / 57 |
| `boss-5-4`               | 8.721 (gleich)  | 9.574 (gleich)      | gleich / gleich                           |               1 / 622 (0,2 %) |                 0 / 859 (0,0 %) |             0 / 57 |
| `first-crucible`         | 9.364 (gleich)  | 9.798 (gleich)      | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `orch-maxload`           | 3.182 (gleich)  | 139 (gleich)        | gleich / gleich                           |               0 / 622 (0,0 %) |                 0 / 187 (0,0 %) |             0 / 57 |
| `test-caster`            | 48.672 (gleich) | 50.760 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `test-collision`         | 9.364 (gleich)  | 12.948 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 1 / 923 (0,1 %) |             0 / 57 |
| `test-crucible-trial`    | 18.990 (gleich) | 23.828 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 695 (0,0 %) |             0 / 57 |
| `test-doomsayer`         | 5.921 (gleich)  | 6.047 (gleich)      | gleich / gleich                           |               1 / 622 (0,2 %) |                 0 / 703 (0,0 %) |             0 / 57 |
| `test-gatekeeper`        | 18.990 (gleich) | 19.660 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 695 (0,0 %) |             0 / 57 |
| `test-lapse`             | 8.793 (gleich)  | 10.172 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `test-lapse-interrupt`   | 9.364 (gleich)  | 10.222 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `test-lethal-strike`     | 8.440 (gleich)  | **18.950 → 11.627** | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 923 (0,0 %) |             0 / 57 |
| `test-metronome`         | 48.672 (gleich) | 51.730 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 923 (0,0 %) |             0 / 57 |
| `test-onslaught`         | 9.014 (gleich)  | 14.233 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 1 / 773 (0,1 %) |             0 / 57 |
| `test-phase`             | 48.672 (gleich) | 50.760 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `test-phasenbindung`     | 48.672 (gleich) | 50.760 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `test-racer`             | 17.798 (gleich) | 19.536 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `test-resistant`         | 37.902 (gleich) | 39.651 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `test-runaway`           | 3.361 (gleich)  | 3.361 (gleich)      | gleich / gleich                           |               1 / 533 (0,2 %) |                 1 / 533 (0,2 %) |             0 / 57 |
| `test-stacked`           | 7.504 (gleich)  | 8.274 (gleich)      | gleich / gleich                           |               1 / 622 (0,2 %) |                 0 / 859 (0,0 %) |             0 / 57 |
| `test-survivable-strike` | 10.244 (gleich) | 11.619 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 773 (0,0 %) |             0 / 57 |
| `test-vessel`            | 14.047 (gleich) | 15.705 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 849 (0,0 %) |             0 / 57 |
| `test-vulnerable`        | 59.390 (gleich) | 61.275 (gleich)     | gleich / gleich                           |               1 / 615 (0,2 %) |                 0 / 923 (0,0 %) |             0 / 57 |
| `test-warden`            | 9.364 (gleich)  | 10.278 (gleich)     | gleich / gleich                           |               0 / 615 (0,0 %) |                 0 / 849 (0,0 %) |             0 / 57 |

## Der #797-AC5-Prüffall unter Fassung B

```bash
cargo remote --no-copy-lock -- test --test ac5_kontrolle_lohnt_sich -- --nocapture
```

| Fassung (behobenes `fight.rs`) | Job                        | bestanden | rot | ignoriert | roter Fall                                                      |
| ------------------------------ | -------------------------- | --------: | --: | --------: | --------------------------------------------------------------- |
| B (im vollen Lauf)             | `6e65eb0f3b0648b3fc804d1f` |         2 |   1 |         1 | `ac5_die_suche_waehlt_fixation_wenn_der_schlag_unabwendbar_ist` |
| A (im vollen Lauf)             | `0ba75aa5b754548be77725da` |         3 |   0 |         1 | —                                                               |

Meldung: „Gewicht 1: keine FIXATION-Zeile am tödlichen Boss" — wie vor der Behebung (damals Job
`ba1de25f480302bf0dd08d8a`). **AC5 bleibt unter A grün**, auch mit der neuen Ordnung im Dreierblock.

Unter B findet die Suche bei `test-lethal-strike` die Liste `searing if dot.searing.remains < 1500`,
`aether`, `plate`, `crucible-skin`, `crust`, `riposte`, `efflux`, `kindle` — **Zeile für Zeile
dieselbe wie im 1-ms-Versuch und wie vor der Behebung**, mit demselben Suchwert 11.627 und dem
Spielertod bei 12.000 ms. Unter A stehen zusätzlich `quench`, `pin` und `bind` darin, dafür fehlt
`kindle` (10 statt 8 Zeilen), Suchwert 18.950 wie vorher; geändert hat sich nur die Reihenfolge im
Dreierblock (`quench` jetzt über `pin`, siehe [Was die Behebung verschob](#was-die-behebung-verschob)).

**Die FIXATION-Rotation wird unter B nicht schlechter.** Die A-Liste erzielt unter B unverändert
18.950 (Kreuzbewertung) und überlebt. Die gierige Suche kommt unter B nur nie an die Stelle, an der
sie eine Dreiergruppe prüfen dürfte: Einzelzüge tragen bis Länge 8, bei Grenze 10 ist dann kein
Platz für drei Zeilen. Genau daran brechen unter B auch
`ueber_greedy_wendet_die_suche_den_toedlichen_schlag_ab` (`greedy_traegt_die_gruppen.rs`) und
`der_schalter_schaltet`, `erst_mit_dreiergruppen_findet_die_suche_die_zeile` (`tripel_suche.rs`;
Meldung: keine einzige Dreiergruppe geprüft, Stopp `NoImprovement`, Länge 8). Alle drei fahren
dieselbe Suche wie der AC5-Fall: `test-lethal-strike`, Gewicht 1, Länge 10, alle Kandidaten, sonst
die Voreinstellung mit `stop_on_kill: false`, über `greedy` oder `greedy_zuege` mit `Zuege::ALLE`
(gelesen in `konfiguration`, `ueberlebt_mit_greedy` und `config` der drei Dateien).

**Wo sich der Weg trennt** (Datensätze `pfad` und `pfadkreuz`): Bis Schritt 2 wählen beide Fassungen
`plate` und `crust` (4200). In Schritt 3 setzt A `searing if dot.searing.remains < 1500` an die
Spitze, B hängt `kindle` an:

| Liste in Schritt 3                                                | unter A | unter B | Takt-Referenz | Würfe = Referenz |
| ----------------------------------------------------------------- | ------: | ------: | ------------: | ---------------- |
| A-Wahl: `searing if dot.searing.remains < 1500`, `plate`, `crust` |    4740 |    4560 |          4560 | A nein, B ja     |
| B-Wahl: `plate`, `crust`, `kindle`                                |    4700 |    4700 |          4700 | A ja, B ja       |

Die A-Wahl hat keinen Füller und keinen GCD-freien Zug. Nach `crust` bei 3000 wartet die Rotation;
die Restzeit von SEARING (gewirkt bei 0, 6000 ms Laufzeit) steht beim GCD-Ende 4500 auf 1500 (aus
Wurfreihe und Zauberdaten). B und die Takt-Referenz legen bei 4510 nach, A erst beim Ablauf bei 6000
— über den ganzen Kampf (Tod bei 12.000) sind die Würfe von B die der Referenz. **Hier bewertet A
falsch, nicht B:** A zählt die Liste 180 höher als die Referenz. Mit 4740 gewinnt sie unter A gegen
die B-Wahl (4700); mit dem Referenzwert 4560 läge sie darunter. Ob A ohne diese Fehlbewertung einen
anderen Weg zur Dreiergruppe fände, ist nicht gemessen.

Unter A hat die Liste in Schritt 7 schon 11.627 bei sieben Zeilen; bei den Grenzen 8 und 9 hält die
Suche dort an (`NoImprovement`), erst die Grenze 10 lässt die Dreiergruppe `quench`, `pin`, `bind` zu
(18.950). Unter B steht `kindle` seit Schritt 3 in der Liste; dieselben 11.627 kosten dort acht
Zeilen, und bei Grenze 10 ist keine Dreiergruppe mehr geprüft (`triples` 0). Der Weg unter A ist
Schritt für Schritt derselbe wie vor der Behebung, bis auf die Ordnung der Dreiergruppe im letzten
Schritt (`pfad`, alt gegen neu).

Die Gewichtsreihe des Prüffalls (`--include-ignored`, mit `die_gewichtsreihe_zeigt_ein_fenster`):

| Gewicht                            | 0   | 1      | 2      | 5      | 10     | 20     | 50     | 100    |
| ---------------------------------- | --- | ------ | ------ | ------ | ------ | ------ | ------ | ------ |
| A (Job `08ffa0301daba612c3babbba`) | —   | trennt | trennt | trennt | trennt | trennt | trennt | trennt |
| B (Job `17d9e899568bb937110b835c`) | —   | —      | trennt | trennt | trennt | **—**  | trennt | trennt |
| B vor der Behebung (`73cb489b…`)   | —   | —      | trennt | trennt | trennt | trennt | trennt | trennt |

„Trennt" heißt `pin` und `bind` am tödlichen Boss, keine FIXATION am Zwilling. Unter B verliert das
Fenster seinen unteren Rand 1 wie vorher, **und mit der Behebung zusätzlich das Gewicht 20**: Dort
findet die Suche unter B keine FIXATION-Zeile mehr. Unter A trennen weiter alle Gewichte ab 1.

## Die Nebenfrage zu `GCD_MS`

Die Suche erzeugt für jeden der vier Brände die Zeile `dot.<brand>.remains < 1500` (`GCD_MS`). Sie
ist die einzige Kandidatenart mit einem strikten Vergleich auf einem zeitabhängigen Wert
(`candidates_for` und `boss_kandidaten` in `sim/src/rotation_search.rs`): Die übrigen vergleichen mit
`>=` auf Werten, die nicht an der Uhr hängen (Ressource, `boss.casting`, `doom`) oder in keiner Fassung
einen Umschlag haben (`buff.*.active`), und mit `<=` auf `boss.next_strike_in`. Gemessen in drei
Schritten: an `first-crucible` gegen die Takt-Referenz, an den gefundenen Listen und an der Suche
selbst.

### An `first-crucible` gegen die Takt-Referenz

Jede Nachlegezeile über 30 s, mit und ohne Füller, gegen die Takt-Referenz über die ganze Zeit
(Datensätze `gcd`):

| Brand / Liste                   | A: Nachlegen bei           | A: Schaden, Ticks | B: Nachlegen bei           | B: Schaden, Ticks | B = Takt-Referenz |
| ------------------------------- | -------------------------- | ----------------- | -------------------------- | ----------------- | ----------------- |
| `cinder` `<= 1500` allein       | 3000, 6000, 9000, 12000 …  | 870, 29           | 3000, 6000, 9000, 12000 …  | 870, 29           | ja                |
| `cinder` `<= 1500` + `kindle`   | 3000, 6000, 9000, 12000 …  | 1870, 29          | 3000, 6000, 9000, 12000 …  | 1870, 29          | ja                |
| `cinder` `< 1500` allein        | 3000, 6000, 9000, 12000 …  | 870, 29           | 3000, 6000, 9000, 12000 …  | 870, 29           | ja                |
| `cinder` `< 1500` + `kindle`    | 3000, 6000, 9000, 12000 …  | 1870, 29          | 3000, 6000, 9000, 12000 …  | 1870, 29          | ja                |
| `rot` `<= 1500` allein          | 4000, 8000, 12000, 16000 … | 917, 29           | 4000, 8000, 12000, 16000 … | 917, 29           | ja                |
| `rot` `<= 1500` + `kindle`      | 4500, 9000, 13500, 18000 … | 2439, 26          | 4500, 9000, 13500, 18000 … | 2439, 26          | ja                |
| `rot` `< 1500` allein           | 4000, 8000, 12000, 16000 … | 917, 29           | 4000, 8000, 12000, 16000 … | 917, 29           | ja                |
| `rot` `< 1500` + `kindle`       | 4500, 9000, 13500, 18000 … | 2439, 26          | 4500, 9000, 13500, 18000 … | 2439, 26          | ja                |
| `searing` `<= 1500` allein      | 4500, 9000, 13500, 18000 … | 1170, 13          | 4500, 9000, 13500, 18000 … | 1170, 13          | ja                |
| `searing` `<= 1500` + `kindle`  | 4500, 9000, 13500, 18000 … | 2470, 13          | 4500, 9000, 13500, 18000 … | 2470, 13          | ja                |
| `searing` `< 1500` allein       | 6000, 12000, 18000, 24000  | 1260, 14          | 4510, 9020, 13530, 18040 … | 1170, 13          | ja                |
| `searing` `< 1500` + `kindle`   | 6000, 12000, 18000, 24000  | 2760, 14          | 6000, 12000, 18000, 24000  | 2760, 14          | ja                |
| `smoulder` `<= 1500` allein     | 10500, 21000               | 560, 14           | 10500, 21000               | 560, 14           | ja                |
| `smoulder` `<= 1500` + `kindle` | 10500, 21000               | 2260, 14          | 10500, 21000               | 2260, 14          | ja                |
| `smoulder` `< 1500` allein      | 12000, 24000               | 560, 14           | 10510, 21020               | 560, 14           | ja                |
| `smoulder` `< 1500` + `kindle`  | 12000, 24000               | 2260, 14          | 12000, 24000               | 2260, 14          | ja                |

Diese 16 Zeilen haben keinen GCD-freien Zug; die Behebung ändert an ihren Würfen und Werten nichts
(alt gegen neu: dieselben Fingerabdrücke, nur der Leerlauf ist neu gezählt). Unter A liegen genau zwei
neben der Referenz: die strikten Zeilen ohne Füller bei SEARING und SMOULDER.

Dieselben Zeilen mit einem GCD-freien Zug vor dem Füller — `quench` löst keine Wartezeit aus,
`sublime` befreit den Wurf danach —, dazu der freie Zug vor dem Füller ganz ohne Brand (Datensätze
`gcdfrei`). **Nach der Behebung treffen A und B die Takt-Referenz in allen 18 Zeilen**, Würfe wie
Schaden; vorher lagen beide in 13 daneben:

| Liste                                       | A = B = Takt-Referenz: Nachlegen bei | Schaden | vor der Behebung (A = B): Nachlegen bei   | Schaden vorher |
| ------------------------------------------- | ------------------------------------ | ------: | ----------------------------------------- | -------------: |
| `cinder` `< 1500` + `quench` + `kindle`     | 3010, 6010, 9020 …                   |    1870 | 3500, 6500, 10000 …                       |           1710 |
| `cinder` `<= 1500` + `quench` + `kindle`    | 3010, 6010, 9020 …                   |    1870 | 3500, 6500, 10000 …                       |           1710 |
| `cinder` `< 1500` + `sublime` + `kindle`    | 3000, 6010, 9010 …                   |    1870 | 3000, 7000, 10000 …                       |           1670 |
| `cinder` `<= 1500` + `sublime` + `kindle`   | 3000, 6010, 9010 …                   |    1870 | 3000, 7000, 10000 …                       |           1670 |
| `rot` `< 1500` + `quench` + `kindle`        | 4510, 9020, 13520 …                  |    2458 | 5000, 10000, 15000 …                      |           2258 |
| `rot` `<= 1500` + `quench` + `kindle`       | 4510, 9020, 13520 …                  |    2458 | 5000, 10000, 15000 …                      |           2258 |
| `rot` `< 1500` + `sublime` + `kindle`       | 4000, 8500, 12500 …                  |    2158 | unverändert                               |           2158 |
| `rot` `<= 1500` + `sublime` + `kindle`      | 4000, 8500, 12500 …                  |    2158 | unverändert                               |           2158 |
| `searing` `< 1500` + `quench` + `kindle`    | 4510, 9020, 13530 …                  |    2470 | 5000, 10000, 15000 …                      |           2280 |
| `searing` `<= 1500` + `quench` + `kindle`   | 4510, 9020, 13520 …                  |    2470 | 5000, 10000, 15000 …                      |           2280 |
| `searing` `< 1500` + `sublime` + `kindle`   | 6000, 12000, 18000 …                 |    2260 | unverändert                               |           2260 |
| `searing` `<= 1500` + `sublime` + `kindle`  | 4500, 9000, 13510 …                  |    2170 | 4500, 9000, 14500 …                       |           2080 |
| `smoulder` `< 1500` + `quench` + `kindle`   | 10520, 21040                         |    2260 | 11000, 22500                              |           1920 |
| `smoulder` `<= 1500` + `quench` + `kindle`  | 10520, 21040                         |    2260 | 11000, 22500                              |           1920 |
| `smoulder` `< 1500` + `sublime` + `kindle`  | 12000, 24000                         |    1760 | unverändert                               |           1760 |
| `smoulder` `<= 1500` + `sublime` + `kindle` | 10500, 21000                         |    1860 | gleich, aber Füller und freier Zug anders |           1860 |
| `quench` + `kindle`, ohne Brand             | Füller ab 10                         |    2000 | Füller ab 2000                            |           1400 |
| `sublime` + `kindle`, ohne Brand            | Füller ab 1500                       |    1500 | unverändert                               |           1500 |

Die Würfe der Takt-Referenz sind in allen 34 Proben vor und nach der Behebung dieselben: Sie entschied
schon vorher einen Takt nach jedem freien Wurf, so wie das Spiel (siehe
[Die Takt-Referenz am Spiel gemessen](#die-takt-referenz-am-spiel-gemessen)). Geändert hat sich der
ereignisgesteuerte Lauf, und zwar genau in den 13 Proben, in denen er vorher neben der Referenz lag.

### An den gefundenen Listen

**Keine der 84 A-Listen zählt unter B anders** (`kreuz`: Wurfreihe und Wert unter A und B gleich in 84
von 84). Vor der Behebung waren es 7; in allen 7 begann die Abweichung damit, dass ein GCD-freier
Wurf auf ein GCD-Ende fiel, an dem eine Nachlegezeile genau auf 1500 stand — B warf einen Takt
danach weiter, A erst am nächsten fremden Ereignis, 400 bis 1000 ms später. Genau diese Lage hat die
Behebung beseitigt: Beide entscheiden dort jetzt einen Takt später, wie das Spiel
(`quench-auf-der-schwelle` in der Aufzeichnung). 37 der 84 A-Listen haben jetzt einen GCD-freien Zug
(vorher 19).

Die zwei Gegenproben an jeder der 84 A-Listen (`kreuz`) sind damit gegenstandslos, aber gefahren: mit
`<=` statt `<` in den Nachlegezeilen und ohne ihre GCD-freien Züge zählt jede A-Liste unter A und B
gleich, 84 von 84.

### An der Suche selbst

Dieselben 84 Suchen mit `<=` statt `<` in den vier Nachlegekandidaten, sonst unverändert (Datensätze
`search_le`): **Unter B verschiebt sich keine** (0 von 84, wie vor der Behebung). Die eine verbliebene
Verschiebung kommt also aus einer Nachlegezeile; keine andere Kandidatenart ändert unter B ihr
Ergebnis. In der AC5-Suche steht `quench` nur in der Dreiergruppe der A-Liste, die erst bei Grenze 10
dazukommt — getrennt hat sich der Weg in Schritt 3, an einer Liste ohne freien Zug (nächster
Abschnitt).

Nebenbei, ohne Fassungsfrage: Mit `<=` findet die Suche unter A in 65 der 84 Fälle eine schlechter
bewertete Liste als mit `<`, in 15 eine besser bewertete, in 4 dieselbe Zahl (vor der Behebung 73,
9 und 2).

### Am Weg der Suche

Der Weg jeder Suche ist `greedy` selbst, mit wachsender Längengrenze von 1 bis zur Grenze der
Einstellung (`pfad`): Solange in jeder Runde ein Einzelzug trägt, ist das der Stand der vollen Suche
nach so vielen Runden. An der ersten Stelle, an der A und B verschiedene Listen haben, stehen zwei
Listen aus derselben Nachbarschaft — die A-Wahl und die B-Wahl. Beide sind unter beiden Fassungen
bewertet und mit angehängter Takt-Referenz über den ganzen Kampf gefahren (`pfadkreuz`), bewertet
durch `measure` wie in der Suche. Unter der eigenen Fassung trifft die Bewertung den Weg bitgleich,
und die Referenz fällt unter A und B gleich aus; beides prüft `pfadvergleich`, statt es anzunehmen.

**Die Wege trennen sich in genau einer Suche, der AC5-Suche** — in den übrigen 83 an keiner Stelle
(`pfadvergleich`: 1 Gabelung, 1 verschobenes Endergebnis; vor der Behebung 15). An der Gabelung, in
Schritt 3, sind beide Schritte Einzelzüge, und die Zahlen sind dieselben wie vorher:

| Suche                          | Schritt | Wert ändert sich | GCD-freier Zug | Füller | A → B (Takt-Referenz) | trifft die Referenz | Referenz wertet höher |
| ------------------------------ | ------: | ---------------- | -------------- | ------ | --------------------- | ------------------- | --------------------- |
| `kontrolle/test-lethal-strike` |       3 | A-Wahl           | —              | —      | 4740 → 4560 (4560)    | B                   | B-Wahl                |

Wert unter A → unter B, in Klammern gegen die Takt-Referenz. „Trifft die Referenz" heißt: über den
ganzen Kampf dieselben Würfe wie die Liste mit angehängter Takt-Referenz. Die B-Wahl `plate`, `crust`,
`kindle` zählt unter A, B und Referenz gleich (4700).

„Füller" heißt eine unbedingte Zeile eines Zaubers, der spätestens nach einem GCD wieder bereit ist
(KINDLE, BELLOWS, SCAB, CLASP …; aus den Zauberdaten, `ist_fueller`). Steht einer in der Liste, wird
an jedem GCD-Ende gewirkt, und ein Weckruf zwischen zwei GCD-Enden fällt in einen GCD.

**Die 14 Gabelungen neben einem GCD-freien Zug sind verschwunden.** Vor der Behebung gingen dort die
Wurfreihen von A und B direkt nach einem GCD-freien Wurf auseinander (B 10 ms danach, A 400 bis 1500
ms danach), und an 12 der 17 geänderten Listen lagen beide neben der Referenz. Das war der Fehler in
`fight.rs`, nicht der Weckruf: Mit der Behebung entscheiden A und B dort gleich, einen Takt nach dem
freien Wurf.

Dieselbe Einteilung über alle Nachbarn der 84 A-Listen, mit ihren Werten aus `kreuz` (`nachbarn`):

| Nachbarn der 84 A-Listen         | Bewertungen | unter B anders | vor der Behebung |
| -------------------------------- | ----------: | -------------: | ---------------: |
| GCD-freier Zug, mit Füller       |      29.654 |              0 |  5322 von 15.944 |
| GCD-freier Zug, ohne Füller      |          23 |              9 |         9 von 20 |
| kein GCD-freier Zug, ohne Füller |         185 |             23 |       28 von 193 |
| kein GCD-freier Zug, mit Füller  |      29.474 |              0 |     0 von 43.006 |
| zusammen                         |      59.336 |             32 |  5359 von 59.163 |

**Mit Füller ändert B keine einzige der 59.128 Bewertungen**, ob mit oder ohne GCD-freien Zug. Alle
32 Änderungen liegen in Listen ohne Füller — dieselbe Lage wie die AC5-Gabelung. Die 84 A-Listen
selbst: 37 mit GCD-freiem Zug und Füller, 46 nur mit Füller, eine mit keinem von beiden; keine zählt
unter B anders.

### Antwort

**Nach der Behebung: folgenlos, außer ohne Füller.** Die Nachlegezeilen `< 1500` sind die einzige
Stelle, an der B die Suche verschiebt, und das nur noch in einer Lage:

- **Mit Füller folgenlos — mit und ohne GCD-freien Zug.** Die Rotation entscheidet an GCD-Enden.
  SMOULDER (12000 ms) und SEARING (6000 ms) sind Vielfache von 1500: Ihre Restzeit steht an einem
  GCD-Ende genau auf 1500, wo `< 1500` falsch ist. Dort wird der Füller gewirkt, und der Weckruf von B
  einen Takt später fällt in dessen GCD — A und B legen beide erst am Ablauf nach, mit identischen
  Würfen und Schaden, wie die Takt-Referenz. Fällt ein GCD-freier Wurf genau auf diese Stelle,
  entscheiden seit der Behebung A und B einen Takt danach neu und sehen dort beide `< 1500` als wahr.
  Gemessen: 0 von 2394 Bestandsläufen, 0 von 59.128 Nachbarn mit Füller, 18 von 18 GCD-Proben mit
  freiem Zug A = B = Referenz.
- **Ohne Füller** wartet die Rotation; B legt dann einen Takt nach dem Umschlag nach (10510, 4510),
  A erst am Ablauf. Weil Erneuern die Restzeit verwirft (`refreshMode: reset`), kostet das bei
  SEARING einen Tick. CINDER und ROT ändern sich nicht. **In dieser Lage trennt sich der Weg der
  AC5-Suche**, und B ist dort die richtige Bewertung (siehe [Am Weg der Suche](#am-weg-der-suche)).
  Das ist der eigentliche Weckruf-Befund dieses Issues; er ist von der Behebung unberührt.
- **Vor der Behebung kam eine dritte Lage hinzu:** neben einem GCD-freien Wurf, der genau auf die
  Schwelle fiel. Dort weckte B einen Takt später, A erst am nächsten fremden Ereignis — so trennten
  sich damals 14 der 15 Suchen. Diese Lage gibt es nicht mehr; sie hing am fehlenden Weckruf nach dem
  freien Wurf, nicht am Vergleich.

Aus der ersten Tabelle, unabhängig von der Fassung: **Mit Füller ist die strikte Form bei SEARING die
bessere** — `< 1500` legt am Ablauf nach und behält alle 14 Ticks (2760 Schaden), `<= 1500` legt
einen GCD früher nach und verwirft einen (2470). Bei SMOULDER, CINDER und ROT ergeben beide Formen
dasselbe.

## Nach einem GCD-freien Wurf: am Spiel gemessen und behoben

Bis zur zweiten Nachbesserung stand hier ein Befund „ohne Fassung", gemessen an der Takt-Referenz:
Nach einem Wurf ohne Wartezeit — QUENCH, oder der Wurf, den SUBLIME befreit — entschied der Simulator
erst am nächsten eingereihten Ereignis neu, die Takt-Referenz einen Takt später. Die Takt-Referenz ist
ein **Nachbau**; gemessen ist der Befund jetzt am Spiel.

### Was das Spiel tut

Die Schleife hinter `runReference` steht jetzt als eigene Funktion `driveRotation` in
`src/devtools/reference-run.ts`; `runReference` ruft sie unverändert auf (`step1.test.ts` grün). Sie
entscheidet **je Takt einmal und nur bei bereiter Wartezeit**: `if (combat.gcd.isReady(timeMs))` →
Liste fragen → wirken, dann `timeMs += TICK_MS`. Nach einem freien Wurf bleibt die Wartezeit bereit,
also entscheidet die Liste im **nächsten** Takt — nicht im selben, nicht erst am nächsten Ereignis.

Belegt mit einer neuen Aufzeichnung aus genau dieser Schleife, `tests/parity/recordings/gcd-freier-wurf.json`
(Exportweg `npm run parity:export`, Frische-Fall in `freshness.test.ts`): sechs Listen an
`first-crucible`, je 30 s. Die Liste steht als Text in der Aufzeichnung und als Funktion daneben in
`tests/parity/payloads.ts` (`GCD_FREI_LAEUFE`). `sim/tests/parity_gcd_freier_wurf.rs` spielt die
Drücke ab (Regeln) **und** fährt den Text durch `Fight::run_apl` (Entscheidung):

| Lauf                      | Liste                                                           | Spiel                           | Simulator vor der Behebung | Schaden Spiel / vorher |
| ------------------------- | --------------------------------------------------------------- | ------------------------------- | -------------------------- | ---------------------: |
| `quench-fueller`          | `quench`, `kindle`                                              | KINDLE bei 10                   | KINDLE bei 2000            |            2000 / 1400 |
| `quench-nachlegen-strikt` | `quench`, `smoulder if dot.smoulder.remains < 1500`, `kindle`   | SMOULDER bei 10                 | SMOULDER bei 2000          |            2260 / 1680 |
| `quench-auf-der-schwelle` | `quench if time >= 10500`, `smoulder if … < 1500`, `kindle`     | QUENCH 10500, SMOULDER 10510    | SMOULDER bei 12000         |            2260 / 1860 |
| `quench-ohne-fueller`     | `quench`, `smoulder if dot.smoulder.remains <= 1500`            | SMOULDER bei 10                 | SMOULDER bei 2000          |              560 / 520 |
| `sublime-befreit`         | `sublime`, `kindle`, `smoulder if dot.smoulder.remains <= 1500` | KINDLE 1500 frei, SMOULDER 1510 | SMOULDER bei 2000          |            2060 / 1740 |
| `warten-auf-die-uhr`      | `kindle if time >= 5000` (Gegenprobe ohne freien Wurf)          | KINDLE ab 5000                  | gleich                     |            1700 / 1700 |

`quench-auf-der-schwelle` ist die Lage der 14 Gabelungen aus der zweiten Nachbesserung, genau
getroffen: QUENCH fällt auf das GCD-Ende 10500, an dem SMOULDER (gewirkt bei 0) auf 1500 steht; `< 1500`
ist dort falsch und einen Takt später wahr, ohne dass dazwischen ein Ereignis liegt. Das prüft
`quench_faellt_genau_auf_die_schwelle_der_nachlegezeile` aus der Aufzeichnung selbst. In
`quench-ohne-fueller` wird die Bedingung nach den freien Würfen bei 10000 und 20000 erst 510 bzw.
1010 ms später wahr (Nachlegen bei 10510 und 21010).

**In den bestehenden Aufzeichnungen** folgt auf einen QUENCH-Druck 24-mal ein Druck im nächsten Takt
(in dreizehn Aufzeichnungen, darunter `instance1-boss-1.json`, `timeline-loop.json`,
`pin-befreiung.json`), einmal einer im **selben** Takt (`player-damage.json`, Takt 300: QUENCH, dann
KINDLE; erzeugt von `interruptRun` in `payloads.ts`), 25-mal ein späterer (Wartezeit nicht bereit
oder nichts zu tun) — gezählt über alle Drücke mit Bindung, ohne die neue Aufzeichnung. Die abspielende Seite kann diese Fälle nicht unterscheiden: `Fight::replay` wirkt
jeden Druck zu seinem Takt und entscheidet nichts — deshalb war keine Aufzeichnung je rot.

### Was das Spiel dem Spieler erlaubt

Eine Frage, die hier nicht entschieden wird. Das Spiel selbst lässt **früher** handeln als die Schleife. `Session.tick` (`src/core/session.ts`)
arbeitet alle Drücke ab, die demselben Takt zugeordnet sind („Alles abarbeiten, was diesem Takt oder
einem früheren zugewiesen war"). Nach QUENCH kann ein Spieler also im **selben** Takt einen
Schadenszauber wirken; `player-damage.json` belegt das an Takt 300 (QUENCH und KINDLE, beide wirksam:
3 QUENCH- und 20 KINDLE-Würfe bei 23 Drücken). Die Schleife entscheidet dagegen einen Takt später, 10
ms nach dem freien Wurf. Dazu kommt ein zweiter Unterschied derselben Art: Die Schleife — und ebenso
der Simulator — entscheidet nur bei bereiter Wartezeit und wirkt QUENCH deshalb nie während eines
laufenden GCD, was das Spiel erlaubt (`player-damage.json`: QUENCH bei Takt 1000 und 1700, 100 bzw.
50 Takte nach einem KINDLE, also mitten in dessen Wartezeit). **Ob eine Prioritätenliste so früh handeln soll wie ein Spieler, ist eine Design-Frage;
der Simulator folgt hier der Schleife.**

### Die Behebung

`Fight::run_with` (`sim/src/fight.rs`) reiht nach einem Wurf, nach dem die Wartezeit bereit bleibt,
einen Weckruf im nächsten Takt ein (`snap_up(now + TICK_MS)`). Danach sind alle sechs Läufe der
Aufzeichnung bitgleich mit dem Spiel — Würfe, Schaden, Wirkungen je Zauber und Leerlauf:

| Prüffall                                                                        | vor der Behebung (`8382b950`)                                                              | danach (`6d3ff723`)        |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------- |
| `parity_gcd_freier_wurf.rs`, Job                                                | `6a6ec646d743eddb3ae94dc6`                                                                 | `d522f482116c563c088f528d` |
| fünf Läufe mit freiem Wurf (`*_entscheidet_wie_im_spiel`)                       | rot, je „Wurf N weicht ab"                                                                 | grün                       |
| `warten_auf_die_uhr_entscheidet_wie_im_spiel`                                   | rot: Leerlauf 6000 statt 5000                                                              | grün                       |
| Abspielen, Trägerprüfung, Schwelle, Vollständigkeit (4)                         | grün                                                                                       | grün                       |
| `entscheidung_nach_gcd_freiem_wurf.rs` (4 Einheitenfälle, von Hand hergeleitet) | 0 bestanden, 4 rot (2000 statt 10, 2000 statt 1510, 6000 statt 5000, 21000 statt 29940 ms) | 4 bestanden                |

Je Behebung eine Gegenprobe über die volle Suite (205 Suiten): Ohne den Weckruf im nächsten Takt
(Mutation MW) sind 9 Fälle rot — die fünf Läufe mit freiem Wurf, drei Einheitenfälle und
`die_reihenfolge_im_block_ist_gemessen` (Job `01c72ffbd216daab1bc10eba`); ohne die verlegte
Leerlauf-Zählung (Mutation MI) 3 — `warten_auf_die_uhr…`, `quench_ohne_fueller…` (27920 statt 25440
ms) und `leerlauf_zaehlt_jeden_wartenden_takt_genau_einmal` (Job `fc3eae50d1f1533d3ded0ff2`).

### Die Takt-Referenz am Spiel gemessen

`e1027-operator-measure spiel tests/parity/recordings/gcd-freier-wurf.json` fährt dieselben sechs
Listen ereignisgesteuert und mit Takt-Referenz und hält beide gegen die Aufzeichnung:

| Bau                                          | Ereignis = Spiel (Würfe) | Takt-Referenz = Spiel (Würfe, Schaden) |       Leerlauf der Takt-Referenz = Spiel |
| -------------------------------------------- | -----------------------: | -------------------------------------: | ---------------------------------------: |
| vor der Behebung (`fight.rs` von `145e3765`) |                  1 von 6 |                                6 von 6 | 3 von 6 (sonst 127.080 bis 1.684.340 ms) |
| A nach der Behebung                          |                  6 von 6 |                                6 von 6 |                                  6 von 6 |
| B nach der Behebung                          |                  6 von 6 |                                6 von 6 |                                  6 von 6 |

**Die Takt-Referenz traf das Spiel also schon vorher, in jedem der sechs Läufe Wurf für Wurf** — die
Aussagen der ersten beiden Nachbesserungen über Würfe und Werte gegen die Referenz stehen damit auf dem
Spiel, nicht nur auf einem Nachbau. Ihr Leerlauf war vorher unbrauchbar (siehe unten). Ein Nachbau,
der berichtigt werden müsste, ist sie nicht.

## Leerlauf (`idle_ms`): behoben

**Wer `idle_ms` liest** (Suche über `idle_ms`, `idleMs`, `idle` in `sim/`, `src/`, `tests/`, `tools/`
und den Aufzeichnungen):

| Seite           | Leser                                                                                                                    | Art                                                             |
| --------------- | ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| Simulator       | `RunResult::idle_ms`, geschrieben nur in `Fight::run_with`                                                               | Spanne bis zur nächsten Entscheidung                            |
| Simulator       | `tests/edge_cases.rs` (60.000 und 0), `apl.rs` (0), `reference.rs` (0, zweimal), `determinism.rs` (Bytes zweier Läufe)   | Zusicherungen, alle grün vor und nach der Behebung              |
| Simulator       | `examples/apl.rs`, `examples/step1.rs` (Ausgabe), `e1027-operator-measure.rs` (Datensätze, außerhalb des Fingerabdrucks) | Anzeige                                                         |
| Suche/Bewertung | keiner: `objective.rs` nennt den Wert nur im Kommentar, `BalanceResult` verwirft ihn                                     | —                                                               |
| Spiel           | `driveRotation`/`runReference` (`idleMs`, je Takt 10 ms), `step1.test.ts` (0), `tools/step1-check.py` (je Takt)          | Vergleich mit dem Simulator in Schritt 1                        |
| Spiel           | `Session` (`idleTicks`, Auswertung, Anzeige) und `misplays.json` (`idleTicks`, nur `> 0` geprüft)                        | eigener Zähler des Spiels, nicht gegen den Simulator verglichen |
| Prüfsummen      | weder `src/core/checksum.ts` noch `sim/src/checksum.rs`                                                                  | —                                                               |
| neu             | `gcd-freier-wurf.json` (`idleMs` der Schleife) gegen `RunResult::idle_ms` in `parity_gcd_freier_wurf.rs`                 | Vergleich Spiel/Simulator                                       |

**Die Fehler.** `fight.rs` zählte beim Warten die Spanne bis zum nächsten **bereits eingereihten**
Ereignis und reihte den Weckruf der Entscheidung erst danach ein. Lag der Weckruf davor, entschied der
Kampf dort erneut, wartete wieder und zählte bis zum selben Ereignis noch einmal — mehrfach. Nach einem
freien Wurf entschied bis zum nächsten Ereignis niemand, also zählte auch niemand. Beides ist behoben:
Gezählt wird nach `schedule_decision_wakeup`, bis zur tatsächlich nächsten Entscheidung, und nach dem
freien Wurf entscheidet die Liste im nächsten Takt wieder.

**Der Beleg.** Spiel und Simulator rechnen jetzt gleich: alle sechs Läufe der Aufzeichnung mit
demselben Leerlauf (0, 0, 0, 25.440, 2980, 5000 ms), ereignisgesteuert wie mit Takt-Referenz. Die
Takt-Referenz zu `smoulder if dot.smoulder.remains < 1500` meldet jetzt 25.500 ms statt 1.846.410,
`searing if dot.searing.remains < 1500` ereignisgesteuert 22.500 statt 30.000 ms — bei fünf GCDs genau
30.000 − 5 × 1500. Wo Lauf und Takt-Referenz dieselben Würfe haben, haben sie auch denselben
Leerlauf: unter A in 30 von 30 GCD-Proben und 2 von 2 Proben `gcdfrei`, unter B in 32 von 32 und 2
von 2. (Die zwei GCD-Proben, deren Würfe unter A neben der Referenz liegen, sind die strikten
Nachlegezeilen ohne Füller — dort legt A später nach, siehe [Die Nebenfrage](#die-nebenfrage-zu-gcd_ms).)

## Was die Behebung verschob

Alle Messungen sind nach der Behebung neu gefahren; die alten stammen aus der zweiten Nachbesserung
(`145e3765`, Ausgaben unter A `f5875952…`, unter B `b81df7dc…`). **Der Bezugspunkt ist geprüft, nicht
erinnert:** Das neue Messprogramm gegen das alte `fight.rs` und `rotation_search.rs` gebaut (Job
`81c410c361a2032e75fc4732`) ergibt bytegleich die alte A-Ausgabe `f5875952…`.

**A gegen B, vorher und nachher:**

| Kennzahl                                          | vor der Behebung         | nach der Behebung      |
| ------------------------------------------------- | ------------------------ | ---------------------- |
| verschobene Suchen                                | 15 von 84                | **1 von 84** (AC5)     |
| davon `bericht` / `kontrolle`                     | 2 / 13 von je 42         | 0 / 1                  |
| A-Listen, die unter B anders zählen               | 7                        | 0                      |
| Nachbarn, die unter B anders zählen               | 5359 von 59.163 (9,06 %) | 32 von 59.336 (0,05 %) |
| Gabelungen der Suchwege                           | 15                       | 1 (AC5, Schritt 3)     |
| Bestandsläufe, die sich unter B ändern            | 97 von 2394              | 0 von 2394             |
| Wurfproben, die A verfehlt                        | 24 von 84                | 24 von 84 (dieselben)  |
| GCD- und `gcdfrei`-Proben = Takt-Referenz (A / B) | 19 / 21 von 34           | 32 / 34 von 34         |
| AC5 unter B                                       | rot                      | rot (derselbe Fall)    |
| Gewichtsreihe: B trennt bei                       | 2, 5, 10, 20, 50, 100    | 2, 5, 10, 50, 100      |
| Prüffälle, die unter B brechen                    | 10                       | 10 (dieselben Namen)   |
| Leerlauf der Takt-Referenz, `smoulder/strikt`     | 1.846.410 ms             | 25.500 ms              |

**Was die Behebung an A selbst verschiebt** (und ebenso an B, siehe die Abschnitte oben), getrennt
nach ihren zwei Teilen. Dafür lief ein Zwischenbau: behobenes `fight.rs` mit der alten Ordnung im
Dreierblock (Job `c0da09cebca7b2df03caaabf`):

| Unter A                         | alt → behobenes `fight.rs` | → neue Ordnung im Dreierblock |  alt → neu zusammen |
| ------------------------------- | -------------------------: | ----------------------------: | ------------------: |
| Suchen mit anderem Ergebnis     |                  37 von 84 |                      2 von 84 |           37 von 84 |
| Suchen `search_le`              |                  71 von 84 |                      3 von 84 |           71 von 84 |
| Bestandsläufe                   |               199 von 2394 |                    0 von 2394 |        199 von 2394 |
| GCD- / `gcdfrei`-Proben (Würfe) |        12 von 32 / 1 von 2 |                             0 | 12 von 32 / 1 von 2 |

Alt gegen neu zusammen: In 36 der 37 Suchen ist die Liste eine andere, in `kontrolle/test-phase` nur
ihr Wert (50.390 → 50.760, derselbe von SUBLIME befreite Wurf entscheidet jetzt einen Takt danach
neu). Der Wert steigt in 24, sinkt in 12 und bleibt in einer gleich (AC5, nur die Ordnung im Block).
Jede der 37 hat in der alten oder der neuen Liste einen GCD-freien Zug. 37 der 84 A-Listen enthalten
jetzt einen, vorher 19.

**Die neue Ordnung im Dreierblock** (`rotation_search::tripel_zuege`: QUENCH über PIN) folgt aus
`die_reihenfolge_im_block_ist_gemessen`, der die bessere Ordnung verlangt und mit der Behebung brach:
An `test-metronome` QUENCH oben 3416 gegen PIN oben 2784 Leben (an `boss-1-1`, `boss-2-3`,
`boss-4-4` gleich). Die alte Messung, nach der PIN oben stand (2776 gegen 2768), kommt ohne den Weckruf
nach dem freien Wurf mit dem heutigen Fall genau so wieder heraus (Mutation MW). Sie verschiebt zwei
Suchen: die AC5-Suche nur in der Ordnung der Gruppe (18.950 bleibt), und `bericht/orch-maxload`, die
mit behobenem `fight.rs` und alter Ordnung **gar keine Liste** fand (Wert 0, `NoImprovement`) und mit
der neuen 3182 findet (vor der Behebung 696).

## Handgeschriebener Bestand

Jede der 57 Dateien aus `sim/apl/` gegen jeden der 42 Bosse, ein Kampf je Paar (Startwert 0, volle
Zeit): **Unter B ändert sich keiner der 2394 Läufe** (`vergleich`, Art `bestand`: 0 von 2394).

Nach dem Aufbau der Dateien, aus dem Parser gelesen statt aus dem Text gesucht (`vergleich`, Feld
`bestand_by_structure`; „strikt" heißt ein Vergleich, der erst nach dem Erreichen seiner Schwelle wahr
wird):

| Aufbau der Datei                           | Dateien | Läufe | unter B verschoben | vor der Behebung |
| ------------------------------------------ | ------: | ----: | -----------------: | ---------------: |
| GCD-freier Zug und strikte Zeitbedingung   |       6 |   252 |                  0 |               97 |
| strikte Zeitbedingung, kein GCD-freier Zug |      42 |  1764 |                  0 |                0 |
| keins von beiden                           |       9 |   378 |                  0 |                0 |

**Vor der Behebung waren es 97 Läufe**, genau in den sechs Dateien mit einer `quench`-Zeile
(`e607p-paar.apl` an 42 Bossen, `e558-paar-bedingt.apl` an 15, die übrigen vier an je 10). Alle 97
hingen an der Entscheidung nach dem freien Wurf: Seit der Simulator danach im nächsten Takt neu
entscheidet, tun das A und B gleich, und die strikte Nachlegezeile wirkt in keiner Datei mehr anders.
Keine Bestandsdatei hat eine Liste ohne Füller mit strikter Nachlegezeile — die einzige Lage, in der B
noch etwas verschiebt (siehe [Die Nebenfrage](#die-nebenfrage-zu-gcd_ms)).

**Die Behebung selbst verschiebt den Bestand**, in beiden Fassungen und nur in denselben sechs
Dateien: unter A 199 von 2394 Läufen (`e607p-paar`, `instance1-boss4-unterbrechen`,
`instance1-rein` und `instance1-unterbrechen` an je 42 Bossen, `e797-mit-quench` an 16,
`e558-paar-bedingt` an 15; Schaden in 197, Siegzeitpunkt in 110, Spielertod in 3), unter B 140
(die drei `instance1`-Dateien an je 42 Bossen, `e797-mit-quench` an 14; `vergleich` der alten gegen
die neue Messung je Fassung). Die 51 Dateien ohne GCD-freien Zug ändern sich in keiner Fassung. Die
Zahl im Kopf von `instance1-boss4-unterbrechen.apl` („seine Zahl (4380)") stammt aus der Zeit vor der
Behebung und ist hier nicht nachgemessen; derselbe Lauf an `boss-1-4` macht in diesem Messprogramm
jetzt 5040 statt 4204 Schaden und legt den Boss bei 57.110 ms.

## Woher der Verzug kommt

Die offene Frage aus `sim/tests/apl_strikte_vergleiche.rs` („Woher dieses Raster kommt, ist offen")
ist beantwortet. Die Aufbauten der dortigen Prüffälle, ohne deren `time < grenze`, unter A, M und B:

| Boss                     | Zeile                                   | `purge` dahinter (gewirkt bei) |     A |     M |    B | Takt-Referenz |
| ------------------------ | --------------------------------------- | ------------------------------ | ----: | ----: | ---: | ------------: |
| `test-survivable-strike` | `kindle if boss.next_strike_in <= 3000` | ja (0, 7500)                   |  9000 |  9000 | 9000 |          9000 |
| `test-survivable-strike` | `kindle if boss.next_strike_in < 3000`  | ja (0, 7500)                   | 10000 | 10000 | 9010 |          9010 |
| `test-survivable-strike` | `kindle if boss.next_strike_in <= 3000` | nein                           |  9000 | 10000 | 9000 |          9000 |
| `test-survivable-strike` | `kindle if boss.next_strike_in < 3000`  | nein                           | 10000 | 10000 | 9010 |          9010 |
| `boss-1-2`               | `kindle if boss.next_strike_in <= 3000` | ja (0)                         |  7000 |  7000 | 7000 |          7000 |
| `boss-1-2`               | `kindle if boss.next_strike_in < 3000`  | ja (0)                         |  7500 |  7500 | 7010 |          7010 |
| `boss-1-2`               | `kindle if boss.next_strike_in <= 3000` | nein                           |  7000 |  7000 | 7000 |          7000 |
| `boss-1-2`               | `kindle if boss.next_strike_in < 3000`  | nein                           |  8000 |  8000 | 7010 |          7010 |

- **Die erste Erklärung war richtig.** Am Umschlag ist `<` falsch, und A stellt keinen zweiten
  Weckruf (siehe Zustandsproben); B legt `<` auf 9010 bzw. 7010.
- **Die Mutation vom 22.08. konnte das nicht zeigen.** In ihren Aufbauten weckt am Umschlag ohnehin
  etwas: `purge` wird bei 7500 gewirkt, sein GCD endet bei 9000; bei `boss-1-2` tickt das Verhängnis
  (`perStepMs` 1000) bei 7000. Ohne `purge` fällt `<= 3000` unter M von 9000 auf 10000.
- **Das Raster ist das nächste schon eingereihte Ereignis**: der Hitzeverfall alle 2000 ms
  (`decayPerStepMs`, 10000), das Ende der `purge`-Abklingzeit (7500), der Verhängnis-Takt (8000).

**Der Befund steht jetzt auch an `switch_point`** (Issue-AC4: „als Kommentar an `at`"): Der Weckruf
liegt dort, wo der Wert die Schwelle erreicht, nicht dort, wo der Vergleich kippt; fallend `<` (auch
`T > v`) und `!=`, auf der Uhr `>` (auch `T < time`) und `!=` greifen erst am nächsten fremden
Ereignis. Der Satz im Modul-Doc von `sim/src/apl/mod.rs`, die Grenze aus #53 sei „nicht nur
beschrieben, sondern weg", ist auf die Formen eingeschränkt, die am Erreichen kippen. Nur
Kommentarzeilen: Alle 25 geänderten Zeilen beginnen mit `//`
(`git diff -U0 23d7c13f -- sim/src/apl/mod.rs`), und die erste Messung kommt damit unverändert
heraus (siehe [Aufbau und Befehle](#aufbau-und-befehle)).

## Prüffälle

Neu in `sim/tests/apl_strikte_vergleiche.rs`: alle sechs Formen in beiden Reihenfolgen an
`boss.next_strike_in`, `dot.smoulder.remains` und `time` gegen die Takt-Referenz, die Trägerprüfung
der Referenz (an einem Fall mit zwei zeitabhängigen Seiten, den keine Fassung weckt) und drei Wächter
für den Weckruf ohne fremdes Ereignis: `der_weckruf_traegt_ohne_fremdes_ereignis` für
`boss.next_strike_in`, mit der Nachbesserung `der_weckruf_von_dot_remains_traegt_ohne_fremdes_ereignis`
(`smoulder if dot.smoulder.remains <= 1500` an `first-crucible` ohne `purge`, Wurf bei 10500) und,
nachgetragen nach der Prüfung, `der_weckruf_von_cooldown_remains_traegt_ohne_fremdes_ereignis`
(`purge if time == 0`, `kindle if cooldown.purge.remains <= 1000` an `test-survivable-strike`, Wurf
bei 6500, unter MC bei 7500, siehe [Was nicht gemessen ist](#was-nicht-gemessen-ist)). Die
Tabellen halten Fassung A fest. **Alle drei Formtabellen messen nur den Takt nach dem Umschlag**,
nicht den Weckruf: An jedem der drei Umschläge endet der GCD von `purge` (9000 bei
`boss.next_strike_in` und `time`, 10500 bei `dot.smoulder.remains`), entschieden wird dort ohnehin.
Gemessen ist das je Wert mit einer Mutation, die seinen Weckruf abschaltet: Unter M bleibt die
Tabelle an `boss.next_strike_in` grün, unter M2 die an `dot.smoulder.remains` (in der ganzen Suite
fällt allein der zweite Wächter). Unter MT bleibt die Ereignisspalte der `time`-Tabelle in allen
zwölf Zeilen gleich (`proben` unter A gegen MT, dieselben Aufbauten); rot wird sie dort nur über ihre
Referenzspalte, weil die Takt-Referenz selbst an `time >= t` hängt. **Den Weckruf von `time` hält
die Trägerprüfung der Referenz fest** (`die_takt_referenz_weckt_und_wirft_nie`: unter MT `Some(8000)`
statt `Some(6010)`), dazu `zeitabhaengige_bedingungen_wecken_den_simulator` in `apl.rs`
(`Some(4000)` statt `Some(3000)`) und
`ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster` in `apl_boss_cast.rs`.

```bash
cargo remote --no-copy-lock -- test --test apl_strikte_vergleiche -- --nocapture
cargo remote --no-copy-lock -- test --no-fail-fast        # volle Suite unter B, M, M2 und MT
cargo remote --no-copy-lock -- test                       # volle Suite unter A, wie die CI
```

| Lauf | Stand                     | `apl_strikte_vergleiche.rs`          | volle Suite                                                                                   |
| ---- | ------------------------- | ------------------------------------ | --------------------------------------------------------------------------------------------- |
| A    | behoben (`6d3ff723`)      | 11 bestanden, 0 rot (im vollen Lauf) | 205 Suiten: 1289 bestanden, 0 rot, 7 ignoriert, 0 gefiltert (Job `0ba75aa5b754548be77725da`)  |
| B    | behoben (`6d3ff723`)      | 5 bestanden, 6 rot (im vollen Lauf)  | 205 Suiten: 1279 bestanden, 10 rot, 7 ignoriert, 0 gefiltert (Job `6e65eb0f3b0648b3fc804d1f`) |
| A    | vorher (`145e3765`)       | 11 bestanden, 0 rot (im vollen Lauf) | 203 Suiten: 1275 bestanden, 0 rot, 7 ignoriert, 0 gefiltert (Job `db43d22e…`)                 |
| B    | vorher (erste Nachbess.)  | 4 bestanden, 6 rot (im vollen Lauf)  | 203 Suiten: 1264 bestanden, 10 rot, 7 ignoriert, 0 gefiltert (Job `0632095f518f7e48913848b3`) |
| M    | vorher (erste Nachbess.)  | 9 bestanden, 1 rot (im vollen Lauf)  | 203 Suiten: 1270 bestanden, 4 rot, 7 ignoriert, 0 gefiltert (Job `a49ae2def4c32739529e227b`)  |
| M2   | vorher (erste Nachbess.)  | 9 bestanden, 1 rot (im vollen Lauf)  | 203 Suiten: 1273 bestanden, 1 rot, 7 ignoriert, 0 gefiltert (Job `f77d5681be9de04ce854b9c4`)  |
| MT   | vorher (zweite Nachbess.) | 6 bestanden, 4 rot (im vollen Lauf)  | 203 Suiten: 1268 bestanden, 6 rot, 7 ignoriert, 0 gefiltert (Job `f86db90453c3856c54413287`)  |

**Unter B brechen nach der Behebung dieselben 10 Fälle wie vorher**, Name für Name. Die zwei Suiten
mehr sind `parity_gcd_freier_wurf.rs` und `entscheidung_nach_gcd_freiem_wurf.rs` (14 Fälle), grün
unter A und B. M, M2 und MT sind nach der Behebung nicht nachgefahren: Ihre Mutationen schalten
Weckrufe aus `switch_point` ab, und die Fälle, die sie fangen, haben keinen GCD-freien Zug; die
Zuordnung „welcher Fall fängt welchen Weckruf" ist deshalb nicht neu gemessen, nur übernommen.

Die vier CI-gleichen Läufe auf dem Endstand stehen im PR-Text; die der zweiten Nachbesserung waren:
`cargo-remote-fmt --check` Exit 0 (Job `2f10f8f04eee02ad4652c84f`),
`cargo remote --no-copy-lock -- clippy --all-targets -- -D warnings` Exit 0 (Job
`ee92b300c4d31a16da3b33c9`), der volle Lauf und `cargo remote --no-copy-lock -- test --all-targets
--features diagnostics` mit 239 Läufen: 1309 bestanden, 0 rot, 7 ignoriert (Job
`c6b344027dcaf826d61d46dc`). `Cargo.lock` und `.cargo-remote.toml` blieben nach jedem Lauf
unverändert.

- **Rot unter B** (10): in `apl_strikte_vergleiche.rs` die drei neuen Formtabellen und die drei
  bestehenden Verzugsfälle, dazu die vier Suchfälle aus dem AC5-Abschnitt. Jede neue Tabelle bricht
  an ihrer ersten verspäteten Zeile, mit (Ereignis, Referenz) = (9010, 9010) bzw. (10510, 10510).
- **Grün unter B** bleiben beide Wächter, die Trägerprüfung der Referenz und die
  Messmethoden-Prüfung; der nachgetragene dritte ebenso (eigener Lauf von `apl_strikte_vergleiche.rs`
  unter B: 5 bestanden, dieselben 6 rot, Job `6a46960c61f157f9c10aab4a`).
- **Rot unter M** (4): der Wächter für `boss.next_strike_in` (10000 statt 9000) und drei bestehende
  Fälle in `next_strike_in.rs`. Die vier älteren Fälle in `apl_strikte_vergleiche.rs` bleiben unter M
  grün — wie am 22.08. notiert.
- **Rot unter M2** (1): allein der Wächter für `dot.*.remains`, `Some(12000)` statt `Some(10500)`.
  Alle zwölf Zeilen der Formtabelle an `dot.smoulder.remains` bleiben grün.
- **Rot unter MT** (6): in `apl_strikte_vergleiche.rs` die Trägerprüfung der Referenz und die drei
  Formtabellen, jede an ihrer ersten verspäteten Zeile und dort in der Referenzspalte (etwa
  `9000 < time`: (10000, 10000) statt (10000, 9010)); dazu `zeitabhaengige_bedingungen_wecken_den_simulator`
  (`apl.rs`) und `ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster`
  (`apl_boss_cast.rs`). Die beiden Wächter bleiben grün, der nachgetragene dritte ebenso (eigener Lauf
  von `apl_strikte_vergleiche.rs` unter MT: 7 bestanden, dieselben 4 rot, Job
  `4527a27ac1d763d1cc18a350`).

## Für die Entscheidung

Die Wahl zwischen A und B trifft dieser Bericht nicht; sie bleibt beim Auftraggeber. Gemessen ist,
was sie nach der Behebung in `fight.rs` kostet und bringt:

- **Der Preis von Fassung B, gemessen:** 0 von 42 Bossen in der Einstellung des Suchberichts, 1 von
  42 bei Gewicht 1 auf Verhindertes (die AC5-Suche); 0 von 2394 Bestandsläufen; 7 bestehende
  Prüffälle (3 davon in `apl_strikte_vergleiche.rs`, die den Ist-Zustand ausdrücklich festhält, und
  die 4 AC5-Suchfälle) und die 3 neuen Formtabellen — dieselben 10 Fälle wie vor der Behebung. Vorher
  waren es 2 und 13 Bosse und 97 Bestandsläufe; dieser Teil des Preises gehörte dem Fehler in
  `fight.rs`, nicht der Fassung.
- **Woher der Preis kommt:** allein aus der Nachlegezeile `< 1500` in einer Liste **ohne Füller** —
  der Lage, in der die Rotation am Umschlag wartet. Mit `<=` verschiebt B keine der 84 Suchen; mit
  Füller ändert B keine der 59.128 Nachbar-Bewertungen. Die AC5-Suche trennt sich genau dort, in
  Schritt 3, und ihre 4 brechenden Prüffälle hängen an diesem einen Weg.
- **Was B richtig macht:** Jede Form wirft dort, wo die Takt-Referenz wirft (84 von 84 Wurfproben,
  32 von 32 GCD-Proben, 2 von 2 Proben `gcdfrei`), und kein Wurf liegt neben dem Takt. Die
  Takt-Referenz trifft ihrerseits das Spiel (6 von 6 Läufen aus dessen eigener Schleife). An der einen
  Gabelung trifft B die Referenz über den ganzen Kampf, A nicht.
- **Was B nicht ändert:** Mit Füller legt die Nachlegezeile `< 1500` weiter erst am Ablauf nach — das
  ist die Strenge des Vergleichs am GCD-Ende, kein Weckruf-Fehler, und bei SEARING gemessen die
  bessere Wahl.
- **Was B bei AC5 kostet, ohne falsch zu sein:** Die AC5-Suche verliert FIXATION bei Gewicht 1, obwohl
  die FIXATION-Rotation unter B gleich viel wert ist (18.950). Ihr Weg trennt sich, weil B die Liste
  `searing if dot.searing.remains < 1500`, `plate`, `crust` so bewertet wie die Takt-Referenz (4560)
  und A sie überbewertet (4740). **Der Weg, auf dem AC5 unter A besteht, führt über diese
  Fehlbewertung.** Unter B trennen die Gewichte 2, 5, 10, 50 und 100, nicht 1 und — seit der Behebung
  — nicht 20; ob die Suche gegen die Takt-Referenz bei Gewicht 1 trennte oder A ohne die Fehlbewertung
  einen anderen Weg fände, ist nicht gemessen.
- **Was keine Fassung entscheidet:** Ob eine Prioritätenliste nach einem freien Wurf im selben Takt
  handeln dürfte und QUENCH während einer laufenden Wartezeit wirken, wie es das Spiel einem Spieler
  erlaubt (siehe [Was das Spiel dem Spieler erlaubt](#was-das-spiel-dem-spieler-erlaubt)).

## Nebenbefunde

- **Behoben (dritte Nachbesserung):** `idle_ms` zählte Leerlauf mehrfach und nach einem GCD-freien
  Wurf gar nicht; nach einem GCD-freien Wurf entschied der Simulator erst am nächsten Ereignis neu.
  Beides steht jetzt unter [Nach einem GCD-freien Wurf](#nach-einem-gcd-freien-wurf-am-spiel-gemessen-und-behoben)
  und [Leerlauf](#leerlauf-idle_ms-behoben), mit Messung am Spiel.
- **Die Schleife des Spiels handelt später, als das Spiel einem Spieler erlaubt** — einen Takt nach
  einem freien Wurf statt im selben, und QUENCH nie während einer laufenden Wartezeit. Eine
  Design-Frage, hier nicht entschieden (siehe [Was das Spiel dem Spieler erlaubt](#was-das-spiel-dem-spieler-erlaubt)).
- **Ein Wert in `sim/apl/instance1-boss4-unterbrechen.apl` ist vermutlich veraltet.** Der Kopf nennt
  „seine Zahl (4380)"; gemessen vor der Behebung. Der Lauf dieser Datei an `boss-1-4` macht im
  Messprogramm jetzt 5040 statt 4204 Schaden. Mit welchem Läufer die 4380 entstanden, sagt die Datei
  nicht; nachgemessen ist sie nicht. Keine der sechs Dateien mit `quench` wird von einem Prüffall
  gelesen (Suche nach ihren Namen in `sim/`, `src/`, `tests/`, `tools/`: nur dieser Bericht).
- **Sechs der 84 Wurfproben fallen unter A ganz aus**, statt sich zu verspäten: `<` in beiden
  Reihenfolgen an `boss.cast_remains` (`< 500`, wie bekannt) und an `cooldown.purge.remains`
  (`< 1500`), dazu `!=` in beiden Reihenfolgen an `cooldown.purge.remains`. Die Fenster schließen sich
  vor dem nächsten fremden Ereignis; die übrigen 18 der 24 verfehlten Würfe kommen verspätet.
- **`cargo-remote-fmt --check` stürzt bei Abweichungen im Client ab**, sobald eine Diff-Zeile für
  ihn gültiges JSON ohne Objekt ist: `AttributeError` in `render` (`client.py`). Der Exit-Code 1 ist
  dann nicht von „Format weicht ab" zu unterscheiden. Mit sauberem Format trat er nicht auf.
- **Die Spielseite hat keine zweite APL-Auswertung**: Suche nach `next_wakeup`, `nextWakeup` und
  `switchPoint` in `src/` und `tests/` ohne Treffer; `src/devtools/reference-run.ts` nutzt
  Rotationsfunktionen. Fassung B betrifft also nur den Simulator.

## Was nicht gemessen ist

- **Keine Laufzeit- oder Leistungsaussage.** Gebaut wurde auf dem Bau-Dienst, gefahren auf diesem
  Rechner unter Fremdlast; Zeiten stehen bewusst nicht in den Datensätzen.
- **Suchen nur in zwei Einstellungen**, ohne Ausrüstung, ohne Talente, ohne den bossübergreifenden
  Nachlauf aus `rotation-search.rs` und ohne Paar-/Gruppenvariation. Die Balance-Zahlen anderer
  Läufer (`rotation-balance.rs`, Machbarkeit) sind nicht nachgefahren.
- **Bestand und Proben je ein Kampf mit Startwert 0.** Ohne Ausrüstung würfelt nichts (`balance::run`
  reduziert dann auf einen Durchgang); mit Krit-Werten ist es nicht gemessen.
- **Fassung B ist eine mögliche Umsetzung**, nicht die einzige. Wahrheitswerte wie `dot.*.active`
  behandelt sie wie der Bestand — ob dort ein eigener Verzug steckt, ist nicht gemessen. Vergleiche
  zweier zeitabhängiger Seiten bleiben in beiden Fassungen ungeweckt (Trägerprüfung der Referenz,
  unter A und B grün: Referenz bei 6010, der ereignisgesteuerte Lauf später).
- **Die Takt-Referenz über ganze Kämpfe** ist nur an den beiden Listen der einen Gabelung gefahren
  (vor der Behebung an 30 Listen an 15 Gabelungen), nicht an den gefundenen Endlisten. Welche Liste
  eine Suche gegen die Referenz wählen würde, ist nicht gemessen; `reference_prefers` vergleicht nur
  die zwei Listen einer Gabelung.
- **Der Weg der Suche** ist `greedy` mit wachsender Grenze. Er gibt die volle Suche genau wieder,
  solange in jeder Runde ein Einzelzug trägt (`extends`); an der einen Gabelung tragen beide. Ob A an
  der AC5-Gabelung ohne die Fehlbewertung einen anderen Weg zur Dreiergruppe fände, ist nicht
  gemessen. Wiederholt (Determinismus) sind in der dritten Nachbesserung die Messung unter A und B
  (bytegleich), nicht `pfad` und `pfadkreuz`.
- **Einzeln nachgezeichnet** ist die eine Gabelung; die Änderungen, die die Behebung an A selbst macht
  (37 Suchen, 199 Bestandsläufe), sind nur gezählt und nach GCD-freiem Zug eingeordnet, nicht einzeln
  verfolgt. „Auf 1500" in `quench-auf-der-schwelle` ist aus der Aufzeichnung und den Zauberdaten
  gerechnet (Prüffall), der Kampf meldet nicht, ob ein Wurf befreit war.
- **Die Aufzeichnung des Spiels** hat sechs Listen an einem Boss ohne Zeitleiste
  (`first-crucible`), je 30 s, ohne Ausrüstung. Boss-Zauber, `boss.casting` und Kanalisierungen nach
  einem freien Wurf sind dort nicht vertreten; dass der Simulator dort ebenso einen Takt später
  entscheidet, folgt aus derselben Zeile in `fight.rs`, ist aber nicht gegen das Spiel gemessen.
- **Ob eine Liste im selben Takt handeln sollte**, wie es das Spiel einem Spieler erlaubt, ist nicht
  gemessen und nicht entschieden; der Simulator folgt der Schleife (`driveRotation`).
- **Mutationen der übrigen Weckrufe** sind gefahren, zwei davon aber nur bei der Prüfung. Gemessen
  bei der Prüfung, je volle Suite: MF (`Value::FightRemains => None`) 1 rot,
  `next_strike_in.rs::ohne_schlag_ist_der_wert_die_restzeit` (Job `01bf5b71582cd257827eddb5`); MBC
  (`Value::BossCastRemains => None`) 1 rot,
  `apl_boss_cast.rs::ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster` (Job
  `65108d803b9af965ab41cc03`); MC (`Value::CooldownRemains(_) => None`) vor dem Nachtrag des dritten
  Wächters 0 rot (Job `173a78f8e3b380cf9bdaff78`). Nach dem Nachtrag hier gemessen, MC als Ersatz
  dieser einen Zeile im Stand dieses Zweigs (ergibt `30986183…`): 203 Suiten, 1274 bestanden, 1 rot,
  7 ignoriert, 0 gefiltert (Job `b89459ded628f8f9faa2753b`); rot ist allein
  `der_weckruf_von_cooldown_remains_traegt_ohne_fremdes_ereignis`, `Some(7500)` statt `Some(6500)`.
  MF und MBC sind hier nicht nachgefahren. `Time` ist mit MT gefahren (siehe [Prüffälle](#prüffälle)).
  **Nach der Behebung in `fight.rs` ist keine dieser Mutationen nachgefahren** (M, M2, MT, MC, MF,
  MBC); neu gefahren sind nur MW und MI, die Gegenproben der Behebung selbst.
- **Die Messungen laufen auf dem Zweig über der Basis `23d7c13f`**, nicht über dem inzwischen
  weitergelaufenen `main` (`b087f63f`). Auf einer lokalen, nicht eingecheckten Zusammenführung mit
  `b087f63f` sind nur die Prüffälle gefahren: 210 Suiten, 1347 bestanden, 0 rot, 7 ignoriert (Job
  `93268dc1e959ae87204ee720`), die Vergleichsdateien unverändert nach dem Export, `tests/parity` 154
  bestanden. Die A/B-Messung selbst ist dort nicht gefahren.

## Rohdaten

Messausgaben, Vergleiche und Binaries der Nachbesserung liegen unter `/work/tmp/w1-1027-nb/mess/` und
`/work/tmp/w1-1027-nb/bin/`, die Fassungen von `mod.rs` unter `/work/tmp/w1-1027-nb/fassungen/`, die
Patches und die erste Messung unter `/work/tmp/w1-1027-patches/` und `/work/tmp/w1-1027-mess/` —
**nicht dauerhaft**; die Befehle oben erzeugen sie neu. Kreuzläufe: `kreuz-a.jsonl` `6474ba12…`,
`kreuz-b.jsonl` `b176c8b9…`. Vergleich A gegen B: `vergleich-ab.json`, SHA-256 `cfa8cc45…` (enthält
die Dateipfade der beiden Eingaben).

Die zweite Nachbesserung unter `/work/tmp/w1-1027-nb2/` (`mess/`, `bin2/`, `fassungen/`), ebenso nicht
dauerhaft: `pfad-a.jsonl` `65958f05…`, `pfad-b.jsonl` `f97b51a1…`, `pfadkreuz-a.jsonl` `69706f7d…`,
`pfadkreuz-b.jsonl` `48cecdd0…` (wiederholt jeweils bytegleich), `nachbarn.json` `eb991b54…`
(enthält die Dateipfade seiner Eingaben), `proben-a.jsonl` `039b172e…`, `proben-mt.jsonl`
`41a4f0b5…`; `pfadvergleich.json` `10c34918…` enthält die Dateipfade seiner Eingaben.

Die dritte Nachbesserung unter `/work/tmp/w1r-1027/` (`mess/`, `bin/`, `skripte/`), ebenso nicht
dauerhaft; `skripte/kette.sh` fährt die Befehle oben in Reihenfolge: `a.jsonl` `c29fdf9d…`,
`b.jsonl` `134a1b10…`, `a1.jsonl` (Zwischenbau), `kreuz-a.jsonl` `0f6b1d48…`, `kreuz-b.jsonl`
`e80f497c…`, `pfad-a.jsonl` `219e64c8…`, `pfad-b.jsonl` `77794732…`, `pfadkreuz-a.jsonl` `76f44f86…`,
`pfadkreuz-b.jsonl` `67c1d46a…`, `spiel-a.jsonl` = `spiel-b.jsonl` `e84fd271…`, `spiel-a0.jsonl`
`60367ace…`; `vergleich-ab.json` `fbde2a0d…`, `pfadvergleich.json` `15ef89b3…`, `nachbarn.json`
`0b693fb8…` (die drei mit den Dateipfaden ihrer Eingaben). Alte gegen neue Messung:
`vergleich-alt-neu-a.json`, `vergleich-alt-neu-b.json`, Zwischenbau: `vergleich-alt-a1.json`,
`vergleich-a1-a.json`.
