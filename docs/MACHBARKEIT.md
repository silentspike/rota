# Machbarkeitsprüfung E21.0b / #949

`sim/src/machbarkeit.rs` prüft explizite Pflichtaufträge vor ihrer Ausführung.
Arbeitsklasse, Anzahl und eingefrorene WU je Einheit sind getrennte Eingaben.
Eine DP-Rate wird nicht als Kampfrate verwendet; zuerst entstehen Zeiten je Klasse,
erst danach deren Summe. Die gemessene Host-Rate wird nicht mit seiner Kernzahl
multipliziert.

Einzelne Aufträge bleiben unteilbar: Ein Kampf auf zwei Hosts mit je 100 Kämpfen/s
braucht weiterhin mindestens 10 ms. Die ETA entsteht aus einer tatsächlichen
ganzzahligen Hostbelegung und deren längster Hostzeit. Eine proportionale Grundbelegung
wird um höchstens einen Verteilungsschritt je Host ergänzt; ein numerischer Rest bei
extrem großen Mengen wird vollständig als Block zugeteilt. Der Aufwand hängt von der
Hostzahl ab, nicht von der Zahl der Aufträge. Die Belegung ist konservativ; eine
allgemeine Optimalitätsgarantie wird nicht behauptet. Große Zahlen und Zeiten werden
bei der Schätzung nach außen gerundet, damit daraus keine zusätzliche Kapazität entsteht.

Die Prüfung verändert weder Auftragszahlen noch `CampaignRunContract`. Pflichtkosten
oberhalb des eingefrorenen WU-Budgets sind ein Fehler. Überschreitet nur die geschätzte
Pflichtzeit das Ziel, nennt der Fehler alle drei Wege: weitere passend gemessene Hosts,
mehr Zielzeit oder ausdrücklich ein Teilergebnis. Nur ein bereits explizit gesetztes
`hard_deadline` erlaubt einen Teilauftrag. Bei `ActiveRuntime` wird dafür ausdrücklich
`remaining` ausgewertet: Überschreitet die Pflichtzeit diese Restfrist, ist
`partial_requested` wahr, auch wenn die weichere Zielzeit reichen würde. Passt die
Pflichtarbeit in eine ausdrücklich längere Restfrist, bleibt der Auftrag zulässig;
die gegebenenfalls verfehlte Zielzeit wird unverändert berichtet. Null Restzeit wird
als abgelaufen abgelehnt. Zielzeit und Restfrist sind seit #914 `Duration`s; die Prüfung
rechnet in ganzen Millisekunden und lehnt eine Angabe mit Bruchteil ab, statt sie still
zu runden. Die Zielzeit stammt aus der Kampagne (`CampaignDefinition::target_wall_time`);
eine andere Zielzeit ist ein neues Profil. `WallClockUtc` wird derzeit ausdrücklich abgelehnt, weil
diese API keinen vertrauenswürdigen aktuellen UTC-Zeitwert erhält. Ein späterer Runner
muss die Frist tatsächlich durchsetzen und ein Teilergebnis entsprechend kennzeichnen;
die Vorabprüfung selbst liefert keine Ergebnisgarantie.

Optionale Arbeit verhindert die Zulassung passender Pflichtarbeit nicht. Fehlt ihre
Rate, bleibt ihre ETA unbekannt. Ungültige vorhandene Messungen werden abgelehnt:
nicht positive oder nicht endliche Raten, ungültige Streuung, zu wenige Abschnitte,
zu große Unsicherheit des Mittels, zu kurze Messdauer, ein anderer Abbruchgrund als
`Stabil` und inkompatible Profilschlüssel. Die Meldung nennt das verletzte Kriterium
mit seinem Wert, etwa
`ungültige Rate scalar_full_fight auf rota-sim: Messdauer zu kurz: 4999 ms, mindestens 5000 ms`.
Doppelte Hosts oder Raten erzeugen keine zusätzliche Kapazität.

**Taktdrift und CPU-Druck sind kein Ablehnungsgrund.** Beide werden gemeldet, und
geprüft wird nur, ob ihr Wert möglich ist: Drift endlich und nicht negativ, PSI
zwischen 0 und 100. Die Prüfung folgt damit der Kalibrierung (`sim/src/kalibrierung.rs`,
Kommentar in `kalibriere` und bei `TAKT_DRIFT_MAX`): Der Takt schwankt auf einem
modernen Regler im Leerlauf um über hundert Prozent, eine Bedingung darauf misst den
Regler und nicht die Güte, und was der Takt an der Rate bewegt, steckt schon in der
Streuung. Bis zum 29.10.2026 verwarf diese Prüfung ab `TAKT_DRIFT_MAX` = 0,10 Drift.
Auf node3 traf das eine von sechs stabilen Messungen (Tabelle unten), und die Meldung
lautete „ungültige oder doppelte Rate“, ohne das Kriterium zu nennen. Entschieden hat
der Orchestrator am 29.10.: Die Machbarkeitsprüfung folgt der Kalibrierung.

## Ausführbarer Messauftrag

```sh
# Bauen und messen auf dem vorkonfigurierten Buildserver, jeweils seriell.
# Absichtlich zu kurze Zielzeit: erwarteter Abbruch nach gültiger Kalibrierung.
cargo remote --no-copy-lock -- run --release -p rota-sim --example e21-0b-machbarkeit -- --mandatory 1000000 --target-time 1ms

# Ausreichende Zielzeit: erwartete Zulassung nach gültiger Kalibrierung.
cargo remote --no-copy-lock -- run --release -p rota-sim --example e21-0b-machbarkeit -- --mandatory 1000 --target-time 1h

# Explizite Zulassung nur eines späteren Teilauftrags.
cargo remote --no-copy-lock -- run --release -p rota-sim --example e21-0b-machbarkeit -- --mandatory 1000000 --target-time 1ms --hard-deadline

# Kontrollproben einschließlich exakt knapper Zielzeit und mehrerer Arbeitsklassen.
cargo remote --no-copy-lock -- test -p rota-sim --test machbarkeit

# Exit-Codes am gestarteten Prozess (in der CI über --all-targets).
cargo remote --no-copy-lock -- test -p rota-sim --example e21-0b-machbarkeit
```

### Exit-Codes

| Code | Zustand         | Bedeutung                                                                                        |
| ---- | --------------- | ------------------------------------------------------------------------------------------------ |
| 0    | PASS            | Die Pflichtarbeit passt, gegebenenfalls als ausdrücklicher Teilauftrag; ebenso `--help`.         |
| 1    | nicht erfüllbar | Gemessen und geprüft, die Zielzeit reicht nicht; die Meldung nennt die drei Wege.                |
| 2    | BLOCKED         | Keine Machbarkeitsaussage: instabile oder ungültige Messung, falsche Eingabe, fehlende Herkunft. |

Nur eine Überschreitung der Zielzeit (`TimeBudgetExceeded`) ist eine
Machbarkeitsaussage. Alle übrigen Fehler enden mit 2 und beginnen mit `BLOCKED:`.
Bis zum 29.10.2026 endete auch BLOCKED mit 1; ein Aufrufer konnte „passt nicht“ dann
nicht von „nicht gemessen“ trennen.

Die Prüffälle im Beispiel starten dessen Prüfprogramm als eigenen Prozess und lesen
Exit-Code und Ausgabe: je einen für 0 (auch mit Teilauftrag), für 1 mit allen drei
Wegen und dreimal 2 (instabile Kalibrierung, ungültige Rate mit Kriterium, falsche
Eingabe). Ein fester Rechner ersetzt dabei Profilschlüssel, Ausstattung und
Kalibrierung, damit der Exit-Code nicht an Last und Ausstattung des Bau-Rechners hängt.
Fest ist der Messbeleg von node3 Lauf 1 (stabil, Taktdrift 0,222755). Nicht im Prozess
läuft `main` selbst: Ein Prüfprogramm hat seinen eigenen Einstieg. Die echte
Kalibrierung belegen die node3-Läufe unten.

Das Beispiel kalibriert vor jeder Prüfung tatsächlich `boss-1-1`, `step1-good.apl`,
60 Sekunden Kampfdauer, `stop_on_kill=false`, `purity=375`, Startseed 5 und einen
Rechenstrang. Gezählt werden echte Kämpfe in Paketen zu 20 000; die deterministische
Ein-Durchgang-Abkürzung wird vorher ausgeschlossen. Die Konfiguration und der SHA256
der eigenen Binärdatei stehen im Bericht. Die Rate gehört ausschließlich zu diesem
Auftrag; sie ist keine allgemeine Host-Leistungszahl.

Ein instabiler Messlauf meldet `BLOCKED` und endet mit Exit 2. Das ist weder ein positiver
Machbarkeitsnachweis noch die geforderte Gegenprobe für eine zu kurze Zielzeit.
Das Beispiel liest keine alten Profildateien: Deren bisheriges Speicherformat enthält
nicht alle erforderlichen Messbelege, insbesondere Dauer und Abbruchgrund.

## Beweisgrenzen

Die API-Rechenproben verwenden ausdrücklich synthetische Raten. Sie prüfen die
Entscheidungen, einschließlich exakt ausreichender Zielzeit, echter Änderung durch
einen zweiten Host und unveränderter Pflichtarbeit. Sie sind keine Hardwaremessung.
Die zusätzlichen Gegenproben prüfen weniger Aufträge als Hosts, ungleich schnelle
Hosts, vollständige Aufzählung kleiner Hostbelegungen, `u64`-Auftragsmengen und
die vom Zeitziel abweichende tatsächliche Restfrist.

Das ausführbare Beispiel ist ein lokaler Vorabprüfer. Zusätzliche Hosts können an der
Bibliotheks-API übergeben werden; es baut keine Remote-Verbindung und startet keine
vollständige Kampagne. Der Aufrufer muss sicherstellen, dass Messprofil und Auftrag
denselben konkreten Arbeitsinhalt betreffen. Der Profiltyp trägt bislang keine eigene
Workload-Signatur. Netzübertragung und zukünftige Fremdlast sind nicht modelliert.

Laut Zuschnitt vom 29.10.2026 (#891) wird kein Kampagnenläufer gebaut. Entsteht doch
einer, gehört sein Anschluss an diese Vorabprüfung in dessen Abnahme.

## Ausgeführte Kontrollen am 6. September 2026

Beide Beispiele wurden seriell auf `rustbuild` über den oben genannten Remote-Weg
gefahren. SHA-256 des tatsächlich gemessenen Binärartefakts:
`7b617780400a06baa1f1274178fcef1f681ed64b3a73f732d652791c3ebe4ab3`.

| Auftrag                            | Frische Kalibrierung                                                    | Entscheidung                                               |
| ---------------------------------- | ----------------------------------------------------------------------- | ---------------------------------------------------------- |
| 1.000.000 Pflichtkämpfe, Ziel 1 ms | 58.054,941 Kämpfe/s; 15 Abschnitte; 5.239 ms; Streuung 0,016620; stabil | 17.225,063 ms geschätzt; Abbruch mit drei Auswegen, Exit 1 |
| 1.000 Pflichtkämpfe, Ziel 1 h      | 58.898,194 Kämpfe/s; 15 Abschnitte; 5.167 ms; Streuung 0,016382; stabil | 16,978 ms geschätzt; PASS, 1.000 WU unverändert, Exit 0    |

Dies belegt die beiden Vorabentscheidungen für den festgelegten skalaren Auftrag,
nicht die Laufzeit einer allgemeinen Kampagne. Die 22 API-Tests einschließlich
Restfrist, ganzer Jobs, heterogener Hostbelegung und großer Ganzzahlen bestanden
zusätzlich im vollständigen Remote-Testlauf.

## Läufe auf node3 am 29. Oktober 2026

Gemessen in CT 105 auf node3 (`rota-sim`, 16 Kerne) unter Messfenster und
`flock ~/messplatz.lock`; Programm per `cargo remote -c -- build --release` gebaut.
Während beider Reihen lief auf dem Bau-Dienst der Fremdjob `a21829f5` eines anderen
Projekts. Auftrag jeweils `--mandatory 1000000 --target-time 1ms`.

**A — vor der Behebung.** Commit `f9c29a72` (die alte Prüfung, nur mit Ausgabe von
Taktdrift und PSI), sha256 `b73e20ab1f942ab7c77a9553867c8f73caef885b607e24aa131a0b3ed7ba7536`.
Knotenlast 2,89 bis 3,68, `PSI cpu some avg10` vor jedem Lauf 0,00.

| Lauf | Rate (Kämpfe/s) | Streuung | Abschnitte | Dauer    | Grund  | Taktdrift | PSI  | Ergebnis                                  |
| ---- | --------------- | -------- | ---------- | -------- | ------ | --------- | ---- | ----------------------------------------- |
| 1    | 169.979,532     | 0,026107 | 43         | 5.122 ms | Stabil | 0,222755  | 0,00 | Exit 1, „ungültige oder doppelte Rate“    |
| 2    | 159.562,166     | 0,107770 | 39         | 5.024 ms | Stabil | 0,016878  | 0,00 | Exit 1, drei Wege, 6.267,150 ms geschätzt |
| 3    | 162.840,689     | 0,053864 | 41         | 5.110 ms | Stabil | 0,043395  | 0,00 | Exit 1, drei Wege, 6.140,971 ms geschätzt |
| 4    | 168.564,983     | 0,014631 | 42         | 5.043 ms | Stabil | 0,006397  | 0,00 | Exit 1, drei Wege, 5.932,430 ms geschätzt |
| 5    | 169.996,688     | 0,017017 | 43         | 5.118 ms | Stabil | 0,004475  | 0,00 | Exit 1, drei Wege, 5.882,468 ms geschätzt |
| 6    | 171.398,947     | 0,008529 | 43         | 5.075 ms | Stabil | 0,000136  | 0,00 | Exit 1, drei Wege, 5.834,342 ms geschätzt |

Lauf 1 erfüllt jedes übrige Kriterium: Unsicherheit des Mittels 0,026107 / √42 ≈ 0,0040,
43 von mindestens 4 Abschnitten, 5.122 von mindestens 5.000 ms, PSI im Bereich. Übrig
bleibt die Taktdrift über 0,10. Die Läufe 2 bis 6 lagen bei höchstens 0,043.

**B — nach der Behebung.** Commit `a1a59cb0`, sha256
`67fefd1e544f67e5de69bf5be5b3d82dd2e1f1ae32d3758556ff02533e5e9af4`. Knotenlast 1,69
bis 2,24, `PSI cpu some avg10` vor jedem Lauf 0,00.

| Lauf | Rate (Kämpfe/s) | Streuung | Abschnitte | Dauer    | Grund  | Taktdrift | PSI  | Ergebnis                                  |
| ---- | --------------- | -------- | ---------- | -------- | ------ | --------- | ---- | ----------------------------------------- |
| 1    | 168.874,602     | 0,018684 | 42         | 5.034 ms | Stabil | 0,079695  | 0,00 | Exit 1, drei Wege, 5.921,554 ms geschätzt |
| 2    | 172.702,840     | 0,010594 | 43         | 5.039 ms | Stabil | 0,054461  | 0,00 | Exit 1, drei Wege, 5.790,293 ms geschätzt |
| 3    | 173.286,520     | 0,020461 | 43         | 5.021 ms | Stabil | 0,026103  | 0,00 | Exit 1, drei Wege, 5.770,789 ms geschätzt |
| 4    | 173.668,401     | 0,013672 | 43         | 5.010 ms | Stabil | 0,140754  | 0,00 | Exit 1, drei Wege, 5.758,100 ms geschätzt |
| 5    | 172.883,068     | 0,015194 | 43         | 5.034 ms | Stabil | 0,047977  | 0,00 | Exit 1, drei Wege, 5.784,256 ms geschätzt |

Lauf 4 liegt mit 0,140754 über der früheren Schwelle 0,10. Vor der Behebung wäre er
als „ungültige oder doppelte Rate“ verworfen worden, jetzt endet er wie die übrigen
mit den drei Wegen.

Kontrollproben im selben Fenster, unmittelbar danach:

| Auftrag                                                 | Frische Kalibrierung                                    | Entscheidung                                        |
| ------------------------------------------------------- | ------------------------------------------------------- | --------------------------------------------------- |
| `--mandatory 1000 --target-time 1h`                     | 172.614,581 Kämpfe/s; 43 Abschnitte; Taktdrift 0,062131 | PASS, 5,793 ms geschätzt, Exit 0                    |
| `--mandatory 1000000 --target-time 7s` (knapp)          | 173.065,161 Kämpfe/s; 43 Abschnitte; Taktdrift 0,072276 | PASS, 5.778,170 ms von 7.000 ms, Exit 0             |
| `--mandatory 1000000 --target-time 1ms --hard-deadline` | 170.767,968 Kämpfe/s; 43 Abschnitte; Taktdrift 0,044208 | PASS als Teilauftrag (`Teilauftrag=true`), Exit 0   |
| `--target-time 5`                                       | keine (Eingabe vor der Messung abgelehnt)               | `BLOCKED: Zielzeit braucht eine Einheit: 5`, Exit 2 |

„Knapp“ heißt hier rund 21 % Spielraum; die exakt knappe Zielzeit prüft
`ausreichende_und_exakt_knappe_zielzeit_sind_beide_zulaessig` mit synthetischer Rate.
