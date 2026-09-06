# CPU-Drosselung: Teilumsetzung von #932 / E15.1

`balance::run_controlled` fragt vor jedem vollständigen Kampf eine gemeinsame
Laufsteuerung ab. Sie kann die Zahl gleichzeitig rechnender Stränge begrenzen,
neue Kämpfe pausieren und fortsetzen sowie den Arbeitsanteil während des Laufs
ändern. Bereits begonnene Kämpfe enden regulär. Ein freier Rechenplatz gehört
keinem bestimmten Strang; auch nach einer Reduktion auf einen Platz können die
übrigen statisch zugeteilten Bereiche vollständig abgearbeitet werden.

Die vollständige Steuerdatei enthält genau drei Felder:

```json
{ "cpu_threads": 2, "cpu_percent": 50, "paused": false }
```

`cpu_threads` darf zwischen null und der anfangs angelegten Strangzahl liegen;
null parkt alle Rechenstränge. `cpu_percent` liegt zwischen 1 und 100. Fehlende,
doppelte oder unbekannte Felder sind Fehler. Die Datei sollte durch atomare
Umbenennung einer vollständig geschriebenen neuen Datei ersetzt werden.

Eine ungültige Anfangsdatei bricht den Start ab. Eine später ungültige oder
vorübergehend unlesbare Datei lässt die letzte gültige Einstellung bestehen;
der separate Steuerstrang zählt die abgewiesenen Leseversuche und hält den
letzten Fehler zur Statusabfrage bereit. Er läuft auch bei vollständig
pausierten Rechensträngen weiter. Sein Guard stoppt und verbindet den Strang
beim Verlassen des Gültigkeitsbereichs.

`cpu_percent` ist **kooperative Taktung anhand gemessener Wandzeit**, keine harte
CPU-Quote. Nach einem Kampf mit Dauer `w` reserviert sein Rechenplatz eine Pause
von `w × (100 − cpu_percent) / cpu_percent`. Andere Stränge können diese Pause
nicht durch Übernahme desselben Platzes umgehen. Auch Betriebssystempausen
stecken in der gemessenen Dauer und können deshalb zusätzlich bremsen.
Einstellungsänderungen wecken wartende Stränge; die verbleibende Ruhezeit wird
mit dem neuen Anteil berechnet.

Die Reaktionszeit umfasst einen laufenden Kampf sowie das Leseintervall der
Steuerdatei. `max_job_time` zeigt die längste beobachtete Kampfberechnung. Eine
Obergrenze von 25 ms wird nicht behauptet. Die Steuerung empfängt weder
Kampfzustand noch Startwert, Archiv oder Kampagnenfingerabdruck. Laufgrenzen
beeinflussen ausschließlich die Ausführungsreihenfolge; bestehende Sortierung
und Aggregation bleiben zuständig für identische Ergebnisse.

`sim/tests/drosselung.rs` prüft den tatsächlichen Balance-Pfad mit 512 nicht
reduzierten, zufallsabhängigen Kämpfen: volle Strangzahl, ein Strang und während
des Laufs geänderte Grenzen, Pause/Fortsetzung sowie eine beschädigte
Zwischenfassung. Verglichen werden Einzelresultate, Zauberzahlen und die genaue
JSON-Ausgabe. Der Test meldet beide Ergebnisfingerabdrücke, tatsächlich
eingetretene Warteereignisse und die längste Arbeitseinheit. Weitere Fälle
prüfen Platzfreigabe, Arbeits-/Ruhetakt, Wiederaufnahme und Schreibfreiheit des
Dateibeobachters. Ausführung und PASS/FAIL werden im Integrationsbericht erst
nach dem Remote-Lauf eingetragen.

Das schließt #932 nicht ab. Offen bleiben das gemeinsame Speicherbudget aller
Verbraucher mit verlustfreier Behandlung von Suchzustand, druckgesteuerte
Selbstregelung, getrennte und tatsächlich eingerichtete Betriebssystemgruppen,
harte CPU-/Speicher-/I/O-Grenzen, Grafikkartensteuerung, vollständige
Betriebsprofile und die millisekundenbasierte adaptive Paketplanung. Der
Nachweis betrifft den vorhandenen lokalen CPU-Pfad; eine verteilte Abnahme ist
damit nicht erbracht.

Fehler beim Nachladen werden in der CLI sofort auf stderr gemeldet, auch wenn alle
Arbeiter pausiert sind. Derselbe unveränderte Fehler wird nicht bei jedem Poll
erneut ausgegeben; die Zähler bleiben vollständig.
