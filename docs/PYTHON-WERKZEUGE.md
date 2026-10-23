# Prüfvertrag für Python-Werkzeuge

Stand 2026-09-06, Issues #1100 und #1051. Der stets laufende `lint`-Job führt
`python3 -B tools/python-check.py --tests` aus. Der Schritt übersetzt sämtliche
`tools/**/*.py` und `tests/tools/*.py` mit `py_compile`, ohne sie dabei zu
importieren oder auszuführen. Die Bytecode-Dateien entstehen außerhalb des
Arbeitsbaums. Syntaxfehler und eine leere Werkzeugtestsuite brechen den Schritt
und damit `lint` und `ci-pass` ab.

Die Einstufung unten betrifft das **Verhalten**. Auch bewusst nicht automatisch
ausgeführte Werkzeuge erhalten die Syntaxprüfung. Keine der genannten Dateien
wird als veraltet entfernt: Die manuelle Asset-Produktion und die langen
Messläufe sind weiterhin benannte Aufgaben.

`python3 -B tools/sim-registry.py --check` prüft im selben `lint`-Job zusätzlich
die Planungsregistry und ihre erzeugten Tabellen (#895). Ihre Verhaltenstests
werden durch dieselbe `tests/tools/`-Discovery ausgeführt.

| Werkzeug                          | Verhalten in der CI | Umfang oder Grund für die Ausnahme                                                                                                                                                                                                                      |
| --------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tools/step1-check.py`            | geprüft             | Bestehender Referenzlauf im `parity`-Job.                                                                                                                                                                                                               |
| `tools/groessen-hebel.py`         | geprüft             | Echter `--list`-Aufruf ohne Export/Sicherung, Kopfzeilenfilter, alle Aufzeichnungsformen, Wirkgrößen und leere Grundlage. Der teure Kampfexport bleibt ein separater Messlauf.                                                                          |
| `tools/streich-mutation.py`       | geprüft             | Echte Mutations- und Auswertungsfunktionen; `main()` mit kontrollierter Prozessgrenze prüft Baufehler, unterbliebenen Lauf alter Rust-Binaries und bytegleichen Rückbau. Dies behauptet keine ausgeführte Rust-Suite.                                   |
| `tools/reihenfolge-einfuegen.py`  | geprüft             | Die Reihenfolge-Gegenprobe führt alle drei Einfügepositionen mit dem echten Vitest-Wächter aus.                                                                                                                                                         |
| `tools/python-check.py`           | geprüft             | Gültiger, bei Ausführung absichtlich abbrechender Quelltext wird nur übersetzt; ein Syntaxfehler wird abgelehnt, ohne Bytecode im Baum.                                                                                                                 |
| `tools/fenster-hook.py`           | geprüft             | Echter Hookprozess mit eigenem Schalterstand: Schalter vor jeder Zerlegung, eigene/fremde/freie/abgegebene Fenster auch bei unzerlegbarer Eingabe, fehlerhafte Zustände, unveränderte Dateien und Berechtigungen, Projektpfade mit Leerzeichen.         |
| `tools/fenster_lastpruefung.py`   | geprüft             | Über den echten Hookprozess: Heredoc-Körper, Hüllen, Hilfe-/Versionsformen, Ersetzungen, Kommentare, Laufzeitnamen, Zeitlimit; Laden neben dem Hook ohne Bytecode im Baum, Ausfall des Moduls.                                                          |
| `tools/sim-registry.py`           | geprüft             | Strukturelle Fehler, Abhängigkeiten, Anker, Abnahmekriterien und erzeugte Tabellen; eigener CI-Check und Verhaltenstests.                                                                                                                               |
| `tools/spell-time-audit.py`       | geprüft             | Echte CLI-Inventur aller verschachtelten Zeitfelder, neues ungültiges Feld, gültige Schwellenausnahme, leere Daten und widersprüchliche Produktionsregeln; die tatsächlichen TS-/Rust-Validatoren werden zusätzlich in ihren eigenen Suiten ausgeführt. |
| `tools/gegenproben-ci.py`         | geprüft             | Der CI-Aufruf führt alle drei echten Gegenproben in einer Wegwerfkopie aus und prüft danach sämtliche kopierten Eingabedateien per SHA-256.                                                                                                             |
| `tools/assets-check.py`           | bewusst ungeprüft   | Die vollständige Asset-/Lizenzprüfung gehört zur Asset-Abnahme; hier wird kein fachlicher Lizenzbefund aus einer Syntaxprüfung abgeleitet.                                                                                                              |
| `tools/bilder-entscheidung.py`    | bewusst ungeprüft   | Schreibt die visuelle Entscheidungsvorlage aus dem lokalen Bildbestand; benötigt eine sichtbare Abnahme.                                                                                                                                                |
| `tools/druecke-erzeugt.py`        | bewusst ungeprüft   | Mutiert Spieldaten und exportiert Kampfläufe; ausführbarer Langlauf im Messfenster, kein beiläufiger CI-Import.                                                                                                                                         |
| `tools/frame-prepare.py`          | bewusst ungeprüft   | Verändert Bilddateien; Produktionsschritt mit Bildwerkzeugen und anschließender sichtbarer Abnahme.                                                                                                                                                     |
| `tools/frame-verify.py`           | bewusst ungeprüft   | Bildbestandsprüfung mit zusätzlichen Bildwerkzeugen; die visuelle Rahmenabnahme bleibt getrennt.                                                                                                                                                        |
| `tools/groessen-einordnen.py`     | bewusst ungeprüft   | Wertet konkrete Ergebnisse langer Hebelmessungen aus; deren aktueller Messbestand ist keine CI-Fixture.                                                                                                                                                 |
| `tools/groessen-export-probe.py`  | bewusst ungeprüft   | Mutations-/Exportlauf mit vollständigen Spielprüfungen; gehört in ein ausdrücklich gestartetes Messfenster.                                                                                                                                             |
| `tools/groessen-mutation.py`      | bewusst ungeprüft   | Wiederholte Datenmutation und vollständige Spielsuite; Laufzeit und Aussage verlangen einen benannten Messlauf.                                                                                                                                         |
| `tools/icon-check-page.py`        | bewusst ungeprüft   | Erzeugt ein visuelles Prüfblatt aus Bilddateien; kein automatisches Urteil über Erkennbarkeit.                                                                                                                                                          |
| `tools/icon-distinct.py`          | bewusst ungeprüft   | Bildwerkzeuge und vollständiger Symbolbestand erforderlich; Ähnlichkeitsmessung bleibt Teil der Asset-Abnahme.                                                                                                                                          |
| `tools/icon-normalize.py`         | bewusst ungeprüft   | Schreibt normierte Symbole; Produktionsschritt mit sichtbarer Abnahme, kein CI-Nebeneffekt.                                                                                                                                                             |
| `tools/icon-verify.py`            | bewusst ungeprüft   | Benötigt Bildwerkzeuge und die zu prüfenden Bilddateien; separate Asset-Abnahme.                                                                                                                                                                        |
| `tools/sfx-distinct.py`           | bewusst ungeprüft   | Benötigt NumPy, Audiowerkzeuge und ausgewählte Klänge; separate technische und hörbare Abnahme.                                                                                                                                                         |
| `tools/sfx-verify.py`             | bewusst ungeprüft   | Benötigt Audiowerkzeuge und erzeugte Klänge; Syntaxgrün sagt nichts über Pegel oder Klangqualität.                                                                                                                                                      |
| `tools/sfx_limits.py`             | bewusst ungeprüft   | Grenzwertbibliothek der Audio-Produktion; Verhaltensprüfung gehört zu deren gesonderter Teststrecke.                                                                                                                                                    |
| `tools/sound-manifest-to-suno.py` | bewusst ungeprüft   | Erzeugt Produktionsaufträge für den Klangdienst; dieser Ablauf wird in der Audio-Produktion geprüft.                                                                                                                                                    |
| `tools/suno_api.py`               | bewusst ungeprüft   | Bezahlter externer Dienst und private Zugangsdaten; niemals durch CI ausführen.                                                                                                                                                                         |
| `tools/analyse/bauzeit.py`        | bewusst ungeprüft   | Analyse privater historischer Sitzungsprotokolle; die benötigten Protokolle gehören nicht in CI.                                                                                                                                                        |

## Die drei Gegenproben

`python3 -B tools/gegenproben-ci.py` führt `ac6-gegenprobe.sh`,
`guide-abschnittsnummern-gegenprobe.sh` und `reihenfolge-gegenprobe.sh` aus.
Der letzte Aufruf benötigt die bereits mit `npm ci` installierten Pakete; er
installiert selbst nichts und führt nur `spell-order.test.ts` aus.

Jede Gegenprobe verändert echte Eingaben ihrer echten Wächter. Geprüft werden
sowohl der erlaubte Zustand als auch die absichtliche Verletzung. Die
Reihenfolge-Gegenprobe ist damit für CI geeignet: Ihr einziger Spieltest ist
gezielt, ihre Änderungen bleiben in der Kopie, und die mitgeänderte
`spell-order.json` ist auch im Aufräumpfad enthalten. Ein fehlender Testbericht
ist ein Fehler und kein erfolgreicher Nachweis einer erkannten Mutation.

Beim ersten vollständigen Aufruf am 06.09. war die Reihenfolge-Gegenprobe rot:
`grün / rot / rot / rot / grün`. Ihr vermeintliches Anhängen schrieb fest nach
`test-cast.json`; weitere Prüfquellen lagen inzwischen dahinter. Die Quelle
wird jetzt über die letzte akzeptierte Kennung aus `spell-order.json` gelesen.
Die Gegenprobe prüft damit wieder den erlaubten Fall, den sie benennt.

AC6 erhält einen lokalen Git-Bezugspunkt mit denselben Ausgangsaufzeichnungen.
Der anschließende leere Commit hält Bezugspunkt und HEAD verschieden. Diese
zwei Commits entstehen ausschließlich in der Kopie. So prüft die Gegenprobe den
Wächter auch bei flachem Checkout oder bei fachlich geänderten Aufzeichnungen
im eigentlichen PR. Sie beurteilt nicht die Zulässigkeit dieser PR-Änderungen.

Nach jeder Gegenprobe müssen Inhalt und Dateimenge aller kopierten Quellen
übereinstimmen. Die zeitweiligen Dateien und der gesamte Testcheckout werden
entfernt. Der echte Arbeitsbaum wird nie zum Mutationsziel.

## Lokaler Nachweis und Grenzen

Die Werkzeugtests enthalten negative Kontrollen: ungültige Python-Syntax,
widersprüchliche Zeitraster, fehlende Messgrundlage und Bauabbruch mit noch
vorhandenen Test-Binaries. Das Ausführen dieser Kontrollen ist keine Balance-
oder Rust-Paritätsmessung. Die vorhandenen vollständigen Spiel- und
Simulatorprüfungen bleiben die Abnahme dieser Systeme.

Zusätzlich am 06.09. in getrennten Kopien nachgemessen: Ohne den Kopfzeilenfilter
wird der echte `groessen-hebel --list`-Test rot; mit der früheren
`lstrip("FAIL ×")`-Auswertung wird der Test der Fehlertestnamen rot. Jeweils
scheiterte genau der zugehörige Prüffall. Ein abgeschalteter
Abschnittsnummern-Detektor ließ die echte Gegenprobe bei Dublette und
Zeilentreue scheitern. Die Kontrollen zeigten somit auch, dass die neuen
Prüfschritte Fehler ihrer Prüfgegenstände erkennen.

Die CI-Anbindung steht im `lint`-Job und wird vom bestehenden `ci-pass`
ausgewertet. Ein lokaler Erfolg belegt die Skripte; ein grüner GitHub-Lauf am
neuen Commit muss nach dessen Veröffentlichung zusätzlich vorliegen.
