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
nicht positive oder nicht endliche Raten, unzureichende Messdauer/Stichprobenzahl,
instabile Messung, ungültige Streuung, Taktdrift und inkompatible Profilschlüssel.
Doppelte Hosts oder Raten erzeugen keine zusätzliche Kapazität.

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
```

Das Beispiel kalibriert vor jeder Prüfung tatsächlich `boss-1-1`, `step1-good.apl`,
60 Sekunden Kampfdauer, `stop_on_kill=false`, `purity=375`, Startseed 5 und einen
Rechenstrang. Gezählt werden echte Kämpfe in Paketen zu 20 000; die deterministische
Ein-Durchgang-Abkürzung wird vorher ausgeschlossen. Die Konfiguration und der SHA256
der eigenen Binärdatei stehen im Bericht. Die Rate gehört ausschließlich zu diesem
Auftrag; sie ist keine allgemeine Host-Leistungszahl.

Ein instabiler Messlauf meldet `BLOCKED`. Das ist weder ein positiver
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

Der echte Anschluss an den vollständigen Kampagnenstart und damit die vollständige
Abnahme von #949 bleiben offen, solange dieser Runner fehlt. Erfolgreiche API-Tests
und Beispielaufrufe allein schließen das Issue nicht.

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
