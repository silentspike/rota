# Fortschritt, zwei Durchsatzzahlen und eine ehrliche Restzeit (#933 / E15.2)

`rota-sim run` und `rota-sim bench` zeigen während des Laufs, was er tut. Zwei
Ausgabewege, ein Datenstrom:

```text
rota-sim run --boss boss-1-1 --iterations 200 --max-len 6 --progress
rota-sim run --boss boss-1-1 --iterations 200 --max-len 6 --jsonl lauf.jsonl
```

`--progress` schreibt einen Block auf den Fehlerkanal — im Terminal viermal je
Sekunde überschrieben, sonst alle zehn Sekunden einer und einer am Ende.
`--jsonl` schreibt dieselben Zahlen viermal je Sekunde als JSON Lines in eine
Datei. Der Bericht auf der Standardausgabe ändert sich durch keine der beiden
Angaben. Beide lassen sich zusammen benutzen.

## Eine Quelle

Gezählt wird im Zählwerk (`sim/src/fortschritt/zaehlwerk.rs`): je Strang ein
`AtomicU64` auf einer eigenen 128-Byte-Zeile, im heißen Pfad von
`balance.rs` genau eine Erhöhung je gerechnetem Kampf. Abgeschlossene
Auswertungen zählt der Strang, der sie angestoßen hat, einmal je Lauf. Die
Zeile, der Strom, die Zeile „Kaempfe gerechnet" jedes Befehls und die
Durchsatzzahlen der Messumgebung lesen alle dasselbe Zählwerk.

`rota-sim` führte bis #933 einen eigenen Zähler. Er zählte die **angeforderten**
Kämpfe; ohne Würfel rechnet ein Lauf aber genau einen (#908). `bench` hat
deshalb bis #933 die angeforderte Zahl durch die Zeit eines einzigen Kampfes
geteilt. Es rechnet jetzt über `balance::run_vollstaendig` jeden Durchgang.

## Der Block

Auszug aus einer Suche über vier Stränge mit Würfel, 0,25 s nach dem Start:

```text
greedy · boss-1-1 · Runde 2   ███░░░░░░░░░░░░░   20 % der Runde
  Kandidaten   169 / höchstens 2 895 (mindestens 357) seit 00:00:00
  Durchsatz    676 /s   ⌀ 676 /s über 0,3 s   aufgelöste Kandidaten
     davon     ein Kampf 0 % · Stichprobe 100 % · gestaffelt 0 %   (ohne Lauf: cache, analytic, ledger, suffix, avx2)
     Kernel    135 943 /s roh   ⌀ 135 943 /s   33 996 Kämpfe für 33 800 Durchgänge
  Restzeit     noch keine: erst ab vier Messsekunden — darunter ist eine Streuung Zufall
  Systeme      localhost.localdomain 4/4 Stränge aktiv (4 Kerne erlaubt)
  Drosselung   keine (ohne --control-file)
  Engpass      keine Diagnose — die Engpasserkennung folgt mit E15.3 (#934)
  Bestes       3 753,2 seit 00:00:00 unverändert · 49 Auswertungen ohne Verbesserung · Runde 2
  Meldung      #2 · 2026-10-28T13:28:05.454384Z · progress
```

**Zwei Durchsatzzahlen:** „Durchsatz" sind aufgelöste Kandidaten je Sekunde —
die Zahl, auf die es ankommt. „Kernel" sind gerechnete Kämpfe je Sekunde. Auf
dem Weg „ein Kampf" (ohne Würfel) liegen beide gleich, bei einer Stichprobe um
den Faktor `iterations` auseinander. `aktuell` mittelt über die letzte Sekunde,
`⌀` über die letzten 30 Sekunden — nicht über den ganzen Lauf.

**Auswertungswege:** Heute gibt es drei — ein Kampf, feste Stichprobe,
gestaffelte Stichprobe. Die Wege des Plans ohne Lauf (Cache, analytisch,
Ledger, Suffix, AVX2) stehen mit Namen da, nicht als 0 %.

## Die Restzeit

| Menge                                     | Anzeige                                                 |
| ----------------------------------------- | ------------------------------------------------------- |
| bekannt (`--apl`, `--search none`, bench) | Restzeit ± eine Standardabweichung                      |
| beschränkt (gierige Suche)                | Spanne ohne Mitte; die Suche kann früher enden          |
| offen bis Konvergenz                      | **keine Restzeit**, stattdessen das Beste und seit wann |

Die Dauer kommt aus einer Ablaufsimulation (`fortschritt::ablauf`): Aufgaben
je Stufe auf die Stränge verteilt, zwischen zwei Stufen ein Sammelpunkt. Eine
Auswertung auf dem Weg „ein Kampf" ist ein Kampf auf einem Strang; vier
Stränge machen eine Kette solcher Auswertungen nicht viermal schneller. Die
Rate je Strang ist ein `RateModel` aus dem 30-Sekunden-Fenster.

Die Spanne hat eine Herleitung, und der Strom trägt jeden ihrer Terme
(`metrics.eta.derivation`):

```text
g        = 1 / (1 − p)                     p = Wiederholungswahrscheinlichkeit (lokal 0)
T        = Ablaufdauer · g
σ_rate   = σ_r · (K + (T − K) / √H)        σ_r Streuung, K kritischer Pfad, H Rechner
σ_kosten = τ · T                           τ Trend der Rate im Fenster
σ_wdh    = T · √(p / N)                    N verbleibende Kämpfe
σ        = √((σ_rate + σ_kosten)² + σ_wdh²)
```

Zum Vergleich stehen dort auch die beiden naiven Werte, die AC5 und AC6
verwerfen: Restkandidaten durch Kandidaten je Sekunde, und Restkämpfe durch
alle Stränge mal Rate.

**Gemessen am 2026-10-28 unter Fremdlast (Last 5,2 bis 5,6 auf 16 Kernen):** Mit
`bench --fights 3000000 --threads 4` schloss die Spanne in 73 von 89 Meldungen
die tatsächliche Restzeit ein. Die 16 übrigen lagen alle in den letzten vier
Sekunden, zu kurz geschätzt: Die Nacharbeit nach dem letzten Kampf (Sortieren
und Statistik von drei Millionen Durchgängen, 0,61 s) steht nicht in der
Rechnung. Bei einer Suche über vier Stränge waren es 13 von 21; die acht
übrigen lagen am Anfang und waren zu kurz, weil spätere Runden teurere Kämpfe
rechnen, als das Fenster bis dahin gesehen hatte.

## Der Strom

Eine Zeile je Meldung. Die oberste Ebene ist der Vertrag aus AC8:

```text
{ "schema_version": 1, "run_id": "…", "event": "progress",
  "timestamp_utc": "2026-10-28T13:28:09.956317Z", "elapsed_ns": 4752537760,
  "host_id": "localhost.localdomain", "host_sequence": 20, "phase": "greedy",
  "metrics": { … } }
```

`event` ist `start`, `progress` oder `end`. `elapsed_ns` misst lokale Dauern,
`timestamp_utc` ordnet über Rechner hinweg zu, `host_sequence` zählt ab 1
lückenlos — eine Lücke ist eine verlorene Meldung. Unter `metrics` stehen
`candidates` (mit `amount`: `known`, `bounded`, `open` oder `unknown`),
`fights`, `samples`, `avoided_fights`, `throughput`, `paths`, `cost`, `eta`,
`plateau`, `hosts`, `throttle` und `bottleneck`. Eine nicht endliche Zahl steht
als `null`, nie als Text.

## Was hier nicht steht

- **Keine Engpassdiagnose.** `bottleneck.diagnosis` ist `null`, bis E15.3
  (#934) sie stellt.
- **Keine offene Suche in `rota-sim`.** Heute gibt es dort keinen Lauf mit
  offener Menge; der Weg ist in der Bibliothek gebaut und geprüft
  (`Arbeitsmenge::Offen`, `tests/fortschritt.rs`).
- **Ein Rechner.** Die Ablaufsimulation rechnet mit mehreren; angezeigt wird
  heute nur der eigene, weil es noch keine Verteilung gibt (E15.5).
