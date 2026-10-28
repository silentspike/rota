# Prüfvertrag für Python-Werkzeuge

Stand 2026-10-28, Issues #1100, #1051 und Epic #892. Der stets laufende `lint`-Job führt
`python3 -B tools/python-check.py --tests` aus. Der Schritt übersetzt sämtliche
`tools/**/*.py` und `tests/tools/*.py` mit `py_compile`, ohne sie dabei zu
importieren oder auszuführen. Die Bytecode-Dateien entstehen außerhalb des
Arbeitsbaums. Syntaxfehler und eine leere Werkzeugtestsuite brechen den Schritt
und damit `lint` und `ci-pass` ab.

Die Einstufung unten betrifft das **Verhalten**. Auch bewusst nicht automatisch
ausgeführte Werkzeuge erhalten die Syntaxprüfung. Keine der genannten Dateien
wird als veraltet entfernt: Die manuelle Asset-Produktion und die langen
Messläufe sind weiterhin benannte Aufgaben.

Die beiden Werkzeuge der Klangerzeugung, `suno_api.py` und
`sound-manifest-to-suno.py`, standen bis 2026-10-28 in dieser Liste. Sie sind
nicht entfernt, sondern ins private Werkzeug-Repo `silentspike/rota-werkzeuge`
umgezogen (`suno/`), weil rota öffentlich werden soll. Dort prüft
`tests/test_suno_pfade.py` ihre Pfadauflösung, ohne Suno zu erreichen.

`python3 -B tools/sim-registry.py --check` prüft im selben `lint`-Job zusätzlich
die Planungsregistry und ihre erzeugten Tabellen (#895). Ihre Verhaltenstests
werden durch dieselbe `tests/tools/`-Discovery ausgeführt.

| Werkzeug                               | Verhalten in der CI | Umfang oder Grund für die Ausnahme                                                                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `tools/step1-check.py`                 | geprüft             | Bestehender Referenzlauf im `parity`-Job.                                                                                                                                                                                                                                                                                                                                                                                             |
| `tools/groessen-hebel.py`              | geprüft             | Echter `--list`-Aufruf ohne Export/Sicherung, Kopfzeilenfilter, alle Aufzeichnungsformen, Wirkgrößen und leere Grundlage. Der teure Kampfexport bleibt ein separater Messlauf.                                                                                                                                                                                                                                                        |
| `tools/streich-mutation.py`            | geprüft             | Echte Mutations- und Auswertungsfunktionen; `main()` mit kontrollierter Prozessgrenze prüft Baufehler, Dienst- und Übertragungsabweise, unterbliebenen Lauf alter Rust-Binaries, bytegleichen Rückbau mit Fingerabdrücken, Vorproben, Wertänderung gegen die Zeitregel aus `policy()` und das Ende jeder Prozessgruppe vor dem Rückbau an echten Prozessen. Dies behauptet keine ausgeführte Rust-Suite.                              |
| `tools/streich-volllauf.py`            | geprüft             | Echte Vorbedingungen mit `--trocken`: Messfenster über eine Attrappe von `rota-fenster` mit absolutem Pfad, echter Git-Stand und Plattenplatz, vollständiger Befehl. Der eigentliche Start legt einen Checkout an und läuft Stunden; er ist nur durch eine Probe belegt.                                                                                                                                                              |
| `tools/reihenfolge-einfuegen.py`       | geprüft             | Die Reihenfolge-Gegenprobe führt alle drei Einfügepositionen mit dem echten Vitest-Wächter aus.                                                                                                                                                                                                                                                                                                                                       |
| `tools/python-check.py`                | geprüft             | Gültiger, bei Ausführung absichtlich abbrechender Quelltext wird nur übersetzt; ein Syntaxfehler wird abgelehnt, ohne Bytecode im Baum.                                                                                                                                                                                                                                                                                               |
| `tools/sim-registry.py`                | geprüft             | Strukturelle Fehler, Abhängigkeiten, Phasenfolge der Voraussetzungen (#891), Anker, Abnahmekriterien und erzeugte Tabellen; eigener CI-Check und Verhaltenstests.                                                                                                                                                                                                                                                                     |
| `tools/spell-time-audit.py`            | geprüft             | Echte CLI-Inventur aller verschachtelten Zeitfelder, neues ungültiges Feld, gültige Schwellenausnahme, leere Daten und widersprüchliche Produktionsregeln; die tatsächlichen TS-/Rust-Validatoren werden zusätzlich in ihren eigenen Suiten ausgeführt.                                                                                                                                                                               |
| `tools/gegenproben-ci.py`              | geprüft             | Der CI-Aufruf führt alle drei echten Gegenproben in einer Wegwerfkopie aus und prüft danach sämtliche kopierten Eingabedateien per SHA-256.                                                                                                                                                                                                                                                                                           |
| `tools/rust-pin.py`                    | geprüft             | Eigener Schritt im `lint`-Job; Tests decken gleiche und abweichende Pins, gleitende und zweiteilige Versionen, doppelte Kopfzeile, Schema und den Präfixfall 1.98.10 ab, dazu die eingecheckten Dateien.                                                                                                                                                                                                                              |
| `tools/wirkung-baubruch-gegenprobe.py` | geprüft             | Eigener Schritt im `sim`-Job (#892) mit dem gepinnten Compiler: Wegwerfkopie, Kontrollen, Baustein ohne Einordnung, Mutation je Pass, Gegenprobe der Gegenprobe; je semantischem Typ neuer Wert ohne und mit Einordnung; Suche nach Auffangformen mit Positivkontrolle. Die Verhaltenstests ersetzen den Compiler durch ein Schein-`cargo` und prüfen Ausgangsvertrag, Client-Ausgänge, fehlende Abschlusszeile und jede Auffangform. |
| `tools/assets-check.py`                | bewusst ungeprüft   | Die vollständige Asset-/Lizenzprüfung gehört zur Asset-Abnahme; hier wird kein fachlicher Lizenzbefund aus einer Syntaxprüfung abgeleitet.                                                                                                                                                                                                                                                                                            |
| `tools/bilder-entscheidung.py`         | bewusst ungeprüft   | Schreibt die visuelle Entscheidungsvorlage aus dem lokalen Bildbestand; benötigt eine sichtbare Abnahme.                                                                                                                                                                                                                                                                                                                              |
| `tools/druecke-erzeugt.py`             | bewusst ungeprüft   | Mutiert Spieldaten und exportiert Kampfläufe; ausführbarer Langlauf im Messfenster, kein beiläufiger CI-Import.                                                                                                                                                                                                                                                                                                                       |
| `tools/frame-prepare.py`               | bewusst ungeprüft   | Verändert Bilddateien; Produktionsschritt mit Bildwerkzeugen und anschließender sichtbarer Abnahme.                                                                                                                                                                                                                                                                                                                                   |
| `tools/frame-verify.py`                | bewusst ungeprüft   | Bildbestandsprüfung mit zusätzlichen Bildwerkzeugen; die visuelle Rahmenabnahme bleibt getrennt.                                                                                                                                                                                                                                                                                                                                      |
| `tools/groessen-einordnen.py`          | bewusst ungeprüft   | Wertet konkrete Ergebnisse langer Hebelmessungen aus; deren aktueller Messbestand ist keine CI-Fixture.                                                                                                                                                                                                                                                                                                                               |
| `tools/groessen-export-probe.py`       | bewusst ungeprüft   | Mutations-/Exportlauf mit vollständigen Spielprüfungen; gehört in ein ausdrücklich gestartetes Messfenster.                                                                                                                                                                                                                                                                                                                           |
| `tools/groessen-mutation.py`           | bewusst ungeprüft   | Wiederholte Datenmutation und vollständige Spielsuite; Laufzeit und Aussage verlangen einen benannten Messlauf.                                                                                                                                                                                                                                                                                                                       |
| `tools/icon-check-page.py`             | bewusst ungeprüft   | Erzeugt ein visuelles Prüfblatt aus Bilddateien; kein automatisches Urteil über Erkennbarkeit.                                                                                                                                                                                                                                                                                                                                        |
| `tools/icon-distinct.py`               | bewusst ungeprüft   | Bildwerkzeuge und vollständiger Symbolbestand erforderlich; Ähnlichkeitsmessung bleibt Teil der Asset-Abnahme.                                                                                                                                                                                                                                                                                                                        |
| `tools/icon-normalize.py`              | bewusst ungeprüft   | Schreibt normierte Symbole; Produktionsschritt mit sichtbarer Abnahme, kein CI-Nebeneffekt.                                                                                                                                                                                                                                                                                                                                           |
| `tools/icon-verify.py`                 | bewusst ungeprüft   | Benötigt Bildwerkzeuge und die zu prüfenden Bilddateien; separate Asset-Abnahme.                                                                                                                                                                                                                                                                                                                                                      |
| `tools/sfx-distinct.py`                | bewusst ungeprüft   | Benötigt NumPy, Audiowerkzeuge und ausgewählte Klänge; separate technische und hörbare Abnahme.                                                                                                                                                                                                                                                                                                                                       |
| `tools/sfx-verify.py`                  | bewusst ungeprüft   | Benötigt Audiowerkzeuge und erzeugte Klänge; Syntaxgrün sagt nichts über Pegel oder Klangqualität.                                                                                                                                                                                                                                                                                                                                    |
| `tools/sfx_limits.py`                  | bewusst ungeprüft   | Grenzwertbibliothek der Audio-Produktion; Verhaltensprüfung gehört zu deren gesonderter Teststrecke.                                                                                                                                                                                                                                                                                                                                  |
| `tools/analyse/bauzeit.py`             | bewusst ungeprüft   | Analyse privater historischer Sitzungsprotokolle; die benötigten Protokolle gehören nicht in CI.                                                                                                                                                                                                                                                                                                                                      |

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

## Die Baubruch-Gegenprobe der Wirkungsbeschreibung

`python3 -B tools/wirkung-baubruch-gegenprobe.py --befehl cargo` läuft als
letzter Schritt im `sim`-Job (Epic #892), nicht in `lint`: Sie braucht den
gepinnten Rust-Compiler. Sie kopiert `sim/`, `data/`,
`tests/parity/recordings/` und die Workspace-Dateien in ein temporäres
Verzeichnis, fügt dort den Baustein `EffectOp::WirkungBaubruchGegenprobe` ein
und baut für jeden Schritt `cargo check --lib`:

| Schritt | erwartet                                                                               |
| ------- | -------------------------------------------------------------------------------------- |
| K0      | die unbeschädigte Kopie baut                                                           |
| M0      | ohne Einordnung bricht der Bau, mit einer E0004-Stelle in jedem registrierten Pass     |
| K1      | an allen gemeldeten Stellen eingeordnet, baut die Kopie wieder                         |
| P       | je Pass: nur er bleibt offen, und der Bau bricht nur dort                              |
| G       | mit Auffangzweig im ersten gebrochenen Pass erkennt die Auswertung ihn als ungebrochen |

Seit dem 2026-10-28 gilt dasselbe für die übrigen semantischen Typen des
Knotens. Welche das sind, liest das Werkzeug aus `pub struct EffectNode {`:
jede Aufzählung des Effekt-Moduls, die ein Feld trägt, direkt oder über eine
Struktur desselben Moduls (heute `Trigger`, `Target`, `TimingSemantics`,
`StackingSemantics`, `RandomnessSemantics`, `MonotonicDirection`,
`MonotonicityDomain`). Je Typ baut es zweimal, und vor allen Bauten sucht es:

| Schritt | erwartet                                                                                       |
| ------- | ---------------------------------------------------------------------------------------------- |
| S       | in `sim/src` ohne `#[cfg(test)]`-Module keine Auffangform über `EffectOp` oder einen der Typen |
| T0      | ein neuer Wert des Typs ohne Einordnung bricht den Bau, an mindestens einer Stelle             |
| T1      | an jeder gemeldeten Stelle eingeordnet, baut die Kopie wieder                                  |

Auffangformen sind `matches!(…)`, ein Arm `_` oder eine bloße Bindung neben
einem Arm, der den Typ nennt, `==`/`!=` neben einem Wert des Typs sowie
`if let`, `while let` und `let … else` mit einem solchen Muster. Die Suche liest
Zeichen statt Zeilen; ein umbrochener Ausdruck bleibt ihr deshalb nicht
verborgen. Ihre Positivkontrolle ist der Auffangzweig aus G und je Typ einer an
der ersten Stelle aus T0: Mit ihm muss sie genau eine Fundstelle mehr melden.
Nicht erkannt werden ein Vergleich über einen Umweg (`[Wert].contains(&x)`,
bloße Namen nach `use Typ::*`) und ein Platzhalter in einem zusammengesetzten
Muster wie `(Trigger::OnCast, _)`.

Die eingefügten Arme lauten `panic!("…")` und nicht `unreachable!("…")`: Eine
Stelle kann in einer `const fn` liegen, dort verweigert der Übersetzer jedes
formatierende Makro (E0015, gemessen im ersten Lauf über `Trigger`).

Exit 0 heißt belegt, 1 ist ein Befund, 2 heißt nicht gelaufen. Zu „nicht
gelaufen“ zählen die Client-Ausgänge 75, 70, 74, 130 und 2 sowie ein Lauf ohne
die Abschlusszeile `cargo-remote: … | exit … | …s`. Lokal läuft dasselbe mit
`--befehl remote`, also mit `cargo remote --no-copy-lock --` aus der Wurzel
der Kopie; `--protokoll VERZ` behält die Ausgabe jedes Baus. Nach dem Lauf
wird der echte Arbeitsbaum per SHA-256 gegengelesen.

Die Verhaltenstests in `tests/tools/test_wirkung_baubruch_gegenprobe.py`
ersetzen den Compiler durch ein Schein-`cargo`, das die Vollständigkeitsprüfung
an einem öffentlichen Fixture nachstellt. Sie prüfen die Entscheidungen des
Werkzeugs, nicht rustc. Den echten Compiler sieht nur der `sim`-Job.

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
