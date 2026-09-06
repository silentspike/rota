# Werkzeug-Abnahme: #1051, #1054 und #1100

Stand: 6. September 2026. Die folgenden Messungen prüfen den integrierten
Werkzeugstand. Eine nachträgliche Negativkontrolle ist kein historischer Beleg
dafür, dass ein roter Commit vor dem ursprünglichen Fix veröffentlicht wurde.
GitHub-Abnahme erfordert zusätzlich die tatsächlichen CI-Ergebnisse des
veröffentlichten Commits.

## #1051: Drei dauerhaft ausgeführte Gegenproben

Der ungefilterte `lint`-Job führt `python3 -B tools/gegenproben-ci.py` aus;
ein Fehler beendet den Schritt und lässt `ci-pass` scheitern. Der Wrapper führt
die echten Skripte `ac6-gegenprobe.sh`,
`guide-abschnittsnummern-gegenprobe.sh` und `reihenfolge-gegenprobe.sh` in einer
temporären Kopie aus. Die Reihenfolge-Gegenprobe ist für CI geeignet: Sie
benötigt die bereits installierten npm-Abhängigkeiten, führt einen gezielten
Vitest-Test aus und stellt die Zauberquellen sowie `spell-order.json` wieder
her. Ihre Einordnung steht auch im Skriptkopf.

Vier zusätzliche, automatisch entdeckte Tests in
`tests/tools/test_guard_probes.py` prüfen die äußere Fehlerkette:

| Prüffall                                                          | Beobachtung                                                   |
| ----------------------------------------------------------------- | ------------------------------------------------------------- |
| Unveränderter echter Abschnittsnummern-Wächter                    | Positive und negative Fälle bestehen; Wrapper Exit 0.         |
| Tatsächlichen Meldetext `Doppelt vergebene` ändern                | Gegenprobe meldet den Schnittstellenbruch; Wrapper Exit 1.    |
| Gegenprobe beendet sich mit Exit 7 und unveränderten Dateien      | Wrapper Exit 1; Fehlercode 7 bleibt in der Diagnose sichtbar. |
| Gegenprobe verändert `package.json`, beendet sich aber mit Exit 0 | Wrapper Exit 1; die veränderte Datei wird genannt.            |

Der letzte Prüffall war vor der Ergänzung des Fingerabdrucks **rot**:
`AssertionError: 0 == 0`. Drei Tests bestanden, einer scheiterte. Der Wrapper
prüfte nur die Unterverzeichnisse und übersah die drei ebenfalls kopierten
Dateien `package.json`, `tsconfig.json` und `vite.config.ts`. Nach ihrer
Aufnahme bestanden alle vier Tests. Nach jedem realen Gegenprobenlauf werden
Dateimenge und SHA-256-Inhalte der kopierten Quellen verglichen. Die vier
Wrapper-Tests vergleichen zusätzlich ihre ursprünglichen Eingaben vor und
nach dem kompletten Unterprozess, auch auf dem Fehlerpfad.

Damit sind positive und negative Kontrollen heute reproduzierbar. Die
behauptete zeitliche Reihenfolge „rot eingecheckt vor dem Einhängen“ aus dem
Issue wird durch diese nachträgliche Messung nicht behauptet.

## #1054: Echte Rust-Ausgabe und Vergleich des alten Runners

Der alte Runner wurde unverändert aus dem Elterncommit des Werkzeugfixes
`2559b201` gelesen. Zum Vergleich diente der aktuelle Runner. Auf dem
freigegebenen Buildserver wurde ein separates Cargo-Projekt mit fünf echten
Testfällen dreimal ausgeführt: Ein Fall ist immer grün; vier weitere schlagen
abhängig von `ROTA_REPORT_FAILURES` absichtlich fehl. Keine ROTA-Daten wurden
mutiert. Es gab weder einen lokalen Rust-Testlauf noch Artefakt- oder
Lockfile-Copyback.

Aus der isolierten Cargo-Projektwurzel, sequenziell für `N=0`, `N=1`, `N=4`:

```bash
cargo remote --no-copy-lock -b "RUST_BACKTRACE=0 ROTA_REPORT_FAILURES=$N" -- test --lib -- --test-threads=1
```

Die Projektwurzel enthielt eine eigene `[workspace]`-Deklaration und die
Remote-Konfiguration. Der erste Versuch ohne diese Projektkonfiguration
endete vor dem Serverlauf mit Exit 4 und „No remote build server was defined“;
er wird nicht als Testlauf gezählt. Die drei anschließenden Läufe verwendeten
Rust 1.98.1 und meldeten:

| Absichtlich rote Fälle | Echte libtest-Ergebniszeile                          | Cargo-Exit |
| ---------------------- | ---------------------------------------------------- | ---------- |
| 0                      | `test result: ok. 5 passed; 0 failed; 0 ignored`     | 0          |
| 1                      | `test result: FAILED. 4 passed; 1 failed; 0 ignored` | 101        |
| 4                      | `test result: FAILED. 1 passed; 4 failed; 0 ignored` | 101        |

Die unveränderten libtest-Ausgaben wurden anschließend durch beide vollständigen
Shell-Runner wiedergegeben. Nur diese Textwiedergabe lief lokal; sie ist kein
zweiter Rust-Testlauf. Bei identischen Eingaben ergab sich:

| Rote Fälle | Alte Vorgabeausgabe unter den Namen | Neue Vorgabeausgabe        | Gesamtausgabe bytegleich  |
| ---------- | ----------------------------------- | -------------------------- | ------------------------- |
| 0          | Keine Namen                         | Keine Namen                | Ja                        |
| 1          | Ein Testname und die Ergebniszeile  | Genau ein Testname         | Nein, erwartete Korrektur |
| 4          | Erste drei echte Testnamen          | Erste drei echte Testnamen | Ja                        |

**Der ursprüngliche Issue-Befund ist bei drei oder mehr Fehlern zu weit
gefasst.** Libtest druckt die Ergebniszeile nach den Testnamen. In der
gemessenen Suite mit vier Fehlern erreichte sie `head -3` schon vorher nicht.
Der echte Fehler zeigt sich bei weniger als drei Fehlern; dieser Fall ist
jetzt korrigiert und durch `test_summary_is_not_a_failed_test_name` geschützt.
`test_default_prints_three_real_names` bewahrt die Ausgabe von drei Namen;
`test_all_names_prints_every_failure` prüft die vollständige Schalterausgabe.

Die bisherigen Vorher-/Nachher-Logs enthielten bereits einen roten und einen
grünen Lauf dieser Shell-Fixtures. Die echte Rust-Suite wurde am 6. September
zusätzlich nachgeholt. Es wird kein historischer Rust-Lauf daraus abgeleitet.

**Schnittstellenmitteilung für aktuelle Werkzeugbenutzer:** Ohne
`--alle-namen` zeigt `tests-lokal.sh` höchstens drei echte fehlgeschlagene
Testnamen. Die Suite-Ergebniszeile zählt nicht mehr als Name. Mit
`--alle-namen` bleibt die vollständige Namensliste verfügbar. Eine Suite ohne
Ergebniszeile lässt den Runner scheitern, auch wenn daneben erfolgreiche
Suites existieren. `streich-mutation.py` wertet inzwischen die vollständige
Remote-Cargo-Ausgabe direkt aus; sein Namensparser wird separat geprüft.
Diese Mitteilung wird mit dem PR veröffentlicht. Inaktive historische
Worker-Sitzungen werden nicht künstlich angeschrieben.

## #1100: Syntax, echte Werkzeugpfade und Pfadfilter

Die vollständige Verhaltenseinstufung steht in
[PYTHON-WERKZEUGE.md](PYTHON-WERKZEUGE.md). Der Katalogtest vergleicht die
Einträge mit sämtlichen tatsächlich vorhandenen Python-Werkzeugen. Alle
Dateien unter `tools/**/*.py` und die Werkzeugtests erhalten eine
`py_compile`-Prüfung im ungefilterten `lint`-Job. Bewusst nicht ausgeführte
Asset- und Dienstwerkzeuge erhalten damit ebenfalls die Syntaxprüfung.

Der echte `groessen-hebel.py --list`-Aufruf wird ohne Export und Sicherungen
geprüft. `streich-mutation.py` durchläuft seine echten Auswahl-, Mutations-,
Klassifikations- und Rückbaupfade mit kontrollierter externer Prozessgrenze.
Das ersetzt keine vollständige Spiel- oder Simulatorabnahme.

Die beiden geforderten negativen Verhaltenskontrollen wurden in getrennten
Kopien nochmals ausgeführt:

| Mutation                                                      | Ausgeführter Prüffall in `test_measurement_tools.py`             | Ergebnis                                                                |
| ------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Kopfzeilenfilter in `offene_paare` entfernen                  | `test_hebel_real_cli_reads_headers_without_exporting_or_backups` | Exit 1; unerlaubtes Paar `['#', 'SCHWEIGT']` in der echten CLI-Ausgabe. |
| Fehlertestnamen mit dem früheren `lstrip("FAIL ×")` auswerten | `test_failure_names_keep_prefixes_and_separate_files_from_cases` | Exit 1; `AC2` wird zu `C2`, der Namensanfang `FAIL` geht verloren.      |

Die Syntax-Gegenkontrolle übergibt dem tatsächlichen Prüfkommando zunächst
gültigen, bei Ausführung absichtlich abbrechenden Quelltext: Exit 0, also
keine Ausführung. Mit `def broken(:` ist das Ergebnis ungleich null und
enthält `SyntaxError`. Eine leere Testsuite wird ebenfalls abgelehnt. Diese
Kontrollen sind Bestandteil des automatisch ausgeführten Werkzeugtestsatzes.

`bash tools/ci-pfadfilter.sh` liest die Filter aus dem aktuellen Workflow.
Zusätzlich zu den immer laufenden Jobs `changes`, `lint`, `actionlint`,
`secret-scan` und `ci-pass` gilt:

| Beispielpfad                                                                           | Zusätzliche Jobs                                                    |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `tools/groessen-hebel.py`, `tools/python-check.py`, `tests/tools/test_guard_probes.py` | `typecheck`, `unit`, `build`, `parity`                              |
| `docs/PYTHON-WERKZEUGE.md`                                                             | Keine; `lint` einschließlich Python und Gegenproben läuft trotzdem. |
| `docs/groessen-abdeckung-messung.txt`                                                  | `typecheck`, `unit`, `build`, `parity`                              |
| `.github/workflows/ci.yml`                                                             | `typecheck`, `unit`, `build`, `parity`, `sim`                       |
| `.github/workflows/nightly.yml`                                                        | `typecheck`, `unit`, `build`, `parity`                              |

Es gibt innerhalb eines ausgelösten CI-Workflows keinen Pfad, der alle Jobs
überspringt. Ein Werkzeugpfad startet weiterhin nicht automatisch den
Rust-Job; er wird jetzt aber direkt durch die Python-Stufe geprüft. Ein Push
auf einen Arbeitszweig ohne PR löst diesen Workflow nicht aus: Er reagiert auf
Pushes nach `main` und die dokumentierten PR-Ereignisse mit Ziel `main`.

## Aktuelle Remote-Regel und alter CI-Wächter

Die aktuelle Anweisung des Auftraggebers verlangt Rust-Bau, Rust-Tests,
Clippy und Formatierung auf dem konfigurierten Buildserver. Der frühere
CI-Schritt „Kein Lauf auf dem Bauserver“ verbot dagegen ausnahmslos remote
ausgeführte Tests. Dieser widersprechende Schritt und sein Skript
`tools/remote-nur-bauen.sh` wurden entfernt. Es gibt weder eine versteckte
Ausnahmeliste noch einen Ersatz, der pauschal Erfolg meldet.

Die fachliche Unterscheidung bleibt bestehen: Ein funktionaler Rust-Test darf
auf dem Buildserver laufen. Ein dort gemessener Durchsatz ist eine Messung
dieses Servers und belegt keine Leistung auf dem Zielrechner. Ein
Host-Leistungsnachweis muss weiterhin Rechner, Last und Ausführungsort nennen.

## Lokale Belege

Die vollständigen Cargo-Ausgaben, Befehle und SHA-256-Werte liegen lokal unter
`/tmp/rota-1054-rust-fixture-20260906/evidence/`. Die alten/neuen Runner-Ausgaben,
echten libtest-Auszüge und negativen Python-Läufe liegen unter
`/tmp/rota-tooling-acceptance-20260906/`. Diese temporären Pfade sind lokale
Belege; die dauerhaften Regressionstests stehen im Repository. Es werden
keine privaten Sitzungsdaten oder Zugangsdaten veröffentlicht.
