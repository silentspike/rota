# Strikte APL-Vergleiche: isolierte Vorher-/Nachher-Messung (#1027)

Gemessen am 6. September 2026. Ausgangscommit `8fbcddee9fddc920e1d4daca07b61e6cdbfadeda`; beide Fassungen enthalten dieselbe Messdatei. Nur die experimentelle Fassung verändert `sim/src/apl/mod.rs`. **Die Produktionssemantik wurde nicht geändert.**

## Ergebnis

Der Patch reicht den Vergleichsoperator an die Weckrufberechnung weiter und unterscheidet steigende/fallende Werte sowie vertauschte Operanden. Von 72 Zustandsproben ändern sich 24 angeforderte Weckzeiten; die Wahrheit der Bedingungen bleibt identisch. Alle sechs Operatoren wurden an DoT-Restzeit und Boss-Wirkzeit vor, auf und nach der Schwelle mit beiden Operandenreihenfolgen gemessen.

Zwei der zwölf tatsächlichen Cast-Proben ändern sich:

| Bedingung                                     | Vorher                                 | Experiment                                   |
| --------------------------------------------- | -------------------------------------- | -------------------------------------------- |
| `dot.smoulder.remains < 1500`                 | Kindle erst bei 12.000 ms; 340 Schaden | Kindle bei 10.501 und 12.010 ms; 440 Schaden |
| `boss.cast_remains < 500` während des Wirkens | Kein Kindle-Wurf; 0 Schaden            | Kindle bei 27.501 ms; 100 Schaden            |

## Tatsächlich gefundene Rotationen

Es wurden acht Greedy-Suchen mit unverändertem Suchumfang und jeweiligem skalarem Nachlauf gefahren. Dies ist eine repräsentative Auswahl aus bestehenden Suchtests, kein vollständiger Suiten- oder Balancebeweis. Sieben Suchen liefern dieselbe APL, dieselben Werte und dasselbe Kampfprotokoll.

| Suchfall                     | Wert vorher | Wert experimentell | Änderung                         |
| ---------------------------- | ----------: | -----------------: | -------------------------------- |
| `suite/brand`                |        5960 |               5960 | keine                            |
| `suite/tools`                |        5960 |               5960 | keine                            |
| `suite/eight`                |        5960 |               5960 | keine                            |
| `all/first-crucible`         |        9364 |               9364 | keine                            |
| `all/test-metronome`         |       48672 |              48672 | keine                            |
| `all/boss-1-2`               |        7544 |               7544 | keine                            |
| `ac5/test-lethal-strike`     |       18950 |              11627 | APL, Ablauf und Ausgang geändert |
| `ac5/test-survivable-strike` |       11619 |              11619 | keine                            |

Beim tödlichen Kontrollboss zählt der Suchwert auch verhinderten Schaden. Vorher werden `pin`, `quench` und `bind` gefunden: kein Tod und kein erlittener Schaden. Im Experiment fehlen diese drei Zeilen; der skalare Nachlauf meldet Bosskill **und Spielertod bei 12.000 ms**, 10.481 erlittenen Schaden. Der ausgehende Schaden steigt von 4.230 auf 7.388, während der Suchwert um 7.323 sinkt. Das ist eine materielle Veränderung des Suchverlaufs, keine folgenlose Zeitkorrektur.

Vorher:

```text
searing if dot.searing.remains < 1500
pin if boss.next_strike_in <= 3000
quench if boss.casting >= 1
bind
aether
plate
crucible-skin
crust
riposte
efflux
```

Experiment:

```text
searing if dot.searing.remains < 1500
aether
plate
crucible-skin
crust
riposte
efflux
kindle
```

## Nachweis und Entscheidung

Beide Release-Bauten und Messungen liefen seriell über `cargo remote --no-copy-lock -- run --release -p rota-sim --example e1027-operator-measure`, jeweils aus einem eigenen archivierten Workspace unter `/work/tmp/rota-1027-compare/`. Alle drei Läufe einschließlich Wiederholung endeten mit Exit 0 und genau 93 JSON-Zeilen: ein Metadatensatz, 72 Operatorzustände, zwölf Cast-Proben, acht Suchen. Die Wiederholung des Experiments war in allen 92 Messdatensätzen identisch; nur die gemessene Ausführungsdauer wurde beim Vergleich ausgenommen.

Die vollständigen Rohdaten, der Vergleich, Quellpatch, Messprogramm, Manifest und Remote-Protokolle sind lokal unter `/work/rota-recovery-20260906/1027/` gesichert. Der Vergleich enthält je Suchfall APL- und Ergebnis-SHA-256 sowie alle geänderten Zahlen und Castfolgen.

Die verlangte Vorher-/Nachher-Evidenz liegt damit für diesen Umfang vor. **#1027 bleibt offen:** Die Entscheidung über die Sprachsemantik sowie der Umgang mit dem veränderten Suchergebnis stehen aus. Das Experiment verwendet 1-ms-Grenzen; die Vereinbarkeit mit dem 10-ms-Spielraster ist nicht abgenommen. Die Entscheidung wird weder durch grüne bestehende Tests noch durch die sieben unveränderten Suchen ersetzt.
