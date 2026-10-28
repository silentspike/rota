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

Der Bericht einer Suche nennt zwei Zahlen, die verschiedene Dinge zählen:
„Messungen" sind die Züge, die die Suche gegeneinander gehalten hat
(`SearchResult::measured`); „Auswertungen" sind alle abgeschlossenen
Auswertungen aus dem Zählwerk, also zusätzlich die Startliste und die
gefundene Liste. Die Anzeige und der Strom zeigen die Auswertungen.

`rota-sim` führte bis #933 einen eigenen Zähler. Er zählte die **angeforderten**
Kämpfe; ohne Würfel rechnet ein Lauf aber genau einen (#908). `bench` hat
deshalb bis #933 die angeforderte Zahl durch die Zeit eines einzigen Kampfes
geteilt. Es rechnet jetzt über `balance::run_vollstaendig` jeden Durchgang.

## Der Block

Auszug aus einer Suche über vier Stränge mit Würfel, 0,25 s nach dem Start
(`rota-sim run --boss boss-1-1 --iterations 200 --max-len 6 --threads 4
--purity 500 --progress`, node3, Bau mit `--features diagnostics`,
2026-10-28):

```text
greedy · boss-1-1 · Runde 3   █████░░░░░░░░░░░   31 % der Runde
  Kandidaten   468 / höchstens 2 649 (mindestens 708) seit 00:00:00
  Durchsatz    1 871 /s   ⌀ 1 871 /s über 0,3 s   aufgelöste Kandidaten
     davon     ein Kampf 0 % · Stichprobe 100 % · gestaffelt 0 %   (ohne Lauf: cache, analytic, ledger, suffix, avx2)
     Kernel    374 551 /s roh   ⌀ 374 551 /s   93 689 Kämpfe für 93 600 Durchgänge
  Restzeit     noch keine: erst ab vier Messsekunden — darunter ist eine Streuung Zufall
  Systeme      rota-sim 4/4 Stränge aktiv (6 Kerne erlaubt)
  Drosselung   keine (ohne --control-file)
  Engpass      noch keine Diagnose — erst ab 4 Messsekunden, das Fenster hat 0,25
     Messung   Zähler zen5 (83 % / 84 % der Zeit gezählt) · ohne Grundlage
  Bestes       5 185,2 seit 00:00:00 unverändert · 112 Auswertungen ohne Verbesserung · Runde 3
  Meldung      #2 · 2026-10-28T18:12:04.485481Z · progress
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

## Der Engpass (#934 / E15.3)

Die Zeile „Engpass" beantwortet die Frage neben dem Durchsatz: **wo lohnt der
nächste Umbau?** Sie gibt einen Satz aus — Diagnose, Beobachtung mit Zahlen,
nächster Schritt —, keine Zahlenliste. Die Regeltabelle ist eine reine
Funktion (`sim/src/engpass/regeln.rs`) und rechnet über dasselbe
30-Sekunden-Fenster wie die gleitende Rate, unter vier Messsekunden gar nicht.

Die letzte Meldung eines `bench` gegen seine Grundlage, während auf den
Geschwistersträngen je 4 MiB Puffer kreisten (`examples/engpass-last.rs`,
node3, 2026-10-28):

```text
  Engpass      Zustand zu groß für die zweite Ebene: Fehlgriffe der zweiten Ebene je Kampf 29,33 statt 1,24 (+2274 %), davon 8,0 % auch an der dritten vorbei (3,29 statt 1,05), Speicher-Wartezeit +506 %, Zyklen je Kampf +97 % — nächster Schritt: Suchzustand weiter verdichten (E11.4)
     Messung   Zähler zen5 (50 % / 50 % der Zeit gezählt) · Grundlage passt
```

Die Zeile darunter sagt, woher der Satz kommt: welche Zähler mit welchem
Anteil der Zeit gezählt haben — die beiden Gruppen teilen sich die Hardware —,
oder warum es keine gibt; und ob eine Grundlage passt.

| Beobachtung                                                            | Diagnose                             | nächster Schritt                            |
| ---------------------------------------------------------------------- | ------------------------------------ | ------------------------------------------- |
| Zyklen je Kampf ≥ +30 %, Befehlsdurchsatz ≤ −25 %, Fehlsprünge ≥ +40 % | verzweigungsgebunden                 | Kernel-Spezialisierung, profilgeführter Bau |
| Zyklen ≥ +30 %, Speicher-Wartezeit ≥ +50 %, L2 ≥ +50 %, dritte niedrig | Zustand zu groß für die zweite Ebene | Suchzustand weiter verdichten               |
| Stränge warten über 15 % ihrer Zeit                                    | Pakete zu groß oder zu wenige        | Paketlänge senken                           |
| Netzwartezeit über 10 % der Paketzeit                                  | Pakete zu klein                      | Paketgröße erhöhen                          |
| Trefferquote eines Wegs unter seiner eigenen Schwelle                  | dieser Weg lohnt nicht               | Weg abschalten                              |
| `expire_buffs` trifft über 20 % der Aufrufe                            | die Kernel-Annahme gilt nicht mehr   | neu messen, Endausbau erreicht              |
| Ereignisschlange über 80 % voll                                        | Kapazität wird knapp                 | vor dem Überlauf handeln                    |
| nur ohne Zähler: CPU-Zeit je Kampf ≥ +30 %                             | langsamer als die Grundlage          | auf einem Rechner mit Zählern nachmessen    |

**Relativ** sind nur die Zählerwerte aus AC2, und zwar gegen eine Grundlage
**desselben Rechners, derselben Kernklasse und desselben Lastprofils**. Ohne
passende Grundlage bleiben die beiden Zählerzeilen ungeprüft — eine feste Zahl
gibt es für sie nicht. Die übrigen Zeilen sind Anteile der eigenen Zeit oder
Kapazität. „Dritte niedrig" heißt: Von den zusätzlichen Fehlgriffen der
zweiten Ebene geht weniger als die Hälfte auch an der dritten vorbei. Die
beiden Zeilen, die Zeit messen — Stränge warten, CPU-Zeit je Kampf —, prüfen
nur, solange der Höchststand von PSI cpu `some avg10` im Fenster bei höchstens
5 % (`hostcaps::PSI_CPU_MAX`) und der von PSI memory bei höchstens 10 % liegt
(`PSI_IO_MEM_MAX`); darüber misst die Uhr die Maschine, nicht den Lauf. PSI
zählt auch die eigenen Stränge: Wer so viele Arbeitsstränge startet, wie er
Kerne darf, lässt Leit- und Anzeigestrang warten und erzeugt den Druck selbst.
Einen gedrosselten Takt zeigt PSI nicht; die Zähler je Kampf sehen ihn nicht,
die CPU-Zeit je Kampf schon.

**Eine Grundlage** schreibt ein Lauf, der genauso aufgerufen wird wie der
spätere; ein Lauf unter Fremdlast oder mit weniger als vier Messsekunden
schreibt keine:

```text
rota-sim bench --boss boss-1-1 --fights 3000000 --threads 4 --write-baseline g.txt
rota-sim bench --boss boss-1-1 --fights 3000000 --threads 4 --baseline g.txt --progress
```

Das Lastprofil im Schlüssel sind Befehl, Boss, Rotation samt Fingerabdruck
ihres Textes, die Mengenangaben, Stränge und der Bau (`release`/`debug`, mit
oder ohne `diagnostics`). Erhebung über die acht Lastprofile, Speicherung je
Kombination und Erneuerung bei geändertem Kernel sind E15.3b (#972).

**Die Leistungszähler** öffnet `rota-sim` über `perf_event_open`, ohne Kiste,
mit `inherit`: Jeder später gestartete Strang zählt mit. Welche Kodierung was
bedeutet, sagt ein Adapter je Kernklasse — Zen 3 (Laptop) und Zen 5 (node3);
Sandy Bridge, Alder Lake P und Gracemont E ruhen. Gibt es keine Zähler —
gesperrt, ruhende oder unbekannte Klasse —, fällt die Diagnose auf
Zeitmessung und Druckwerte zurück und nennt den Grund, statt auszufallen.
Schlangenbelegung und Trefferquote der Ablaufprüfung gibt es nur in einem Bau
mit `--features diagnostics`; sonst stehen sie als nicht erhoben da.

Im Strom steht unter `metrics.bottleneck`:

| Feld                    | Inhalt                                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------------- |
| `state`                 | `diagnosis`, `none` (geprüft, nichts gefunden), `pending` (nichts prüfbar), `not_collected` |
| `diagnosis`, `sentence` | die erste Diagnose und der ganze Satz                                                       |
| `findings`              | je Befund `rule`, `diagnosis`, `observation`, `next_step`                                   |
| `checked`, `unchecked`  | welche Zeilen geprüft wurden, und welche nicht, mit `reason`                                |
| `key`, `baseline`       | Schlüssel des Laufs; Schlüssel der Grundlage und ob er passt                                |
| `counters`              | ob es Zähler gibt, Kernklasse, Prozessor, Hinweis des Adapters oder Grund und Rückfall      |
| `measured`              | alle Messwerte aus AC1 über das Fenster, je Strang Arbeit und Warten                        |

Die künstlichen Lagen, an denen der Wächter geprüft ist, lassen sich
nachfahren: `examples/engpass-last.rs` belastet die Geschwisterstränge der
Kerne, auf denen `rota-sim` rechnet; `examples/engpass-pakete.rs` fährt viele
kleine Auswertungen mit weniger Paketen als Strängen.

## Was hier nicht steht

- **Keine offene Suche in `rota-sim`.** Heute gibt es dort keinen Lauf mit
  offener Menge; der Weg ist in der Bibliothek gebaut und geprüft
  (`Arbeitsmenge::Offen`, `tests/fortschritt.rs`).
- **Ein Rechner.** Die Ablaufsimulation rechnet mit mehreren; angezeigt wird
  heute nur der eigene, weil es noch keine Verteilung gibt (E15.5).
