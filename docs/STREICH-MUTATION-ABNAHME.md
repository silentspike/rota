# Streich-Mutationen: Bau und Tests getrennt nachweisen (#1055)

Die Korrektur der Baufehler-Erkennung allein erneuert keine Messung. Ein
`build.rs`-Panic kann mit Exit 101 enden, ohne eines der alten Compilertextmuster
zu enthalten. `bau_gebrochen` wertet deshalb den Exit-Code zuerst aus. Nach einem
Bauabbruch wird kein Rust-Testlauf gestartet.

Ein Exit-Code des Bau-Dienstes ist dagegen **kein** Bauabbruch: 75, 70, 74, 130
und 2 kommen vom v2-Client selbst, 12 und 23 von rsync und 255 von ssh, über die
er die Quellen hochlädt und die er unverändert durchreicht. In keinem dieser
Fälle lief ein Compiler. Der Durchgang endet dann mit NICHT GELAUFEN (Exit 2),
das Feld steht als ABBRUCH im Nachweis, und die Daten sind zurückgestellt.

Jeder neue Durchgang schreibt neben dem Bericht einen JSON-Nachweis und ein
Protokollverzeichnis. Für **jedes** Feld stehen dort tatsächliche Mutationen,
Bau-Exit, Rust- und TS-Status, Zahl ausgeführter Fälle, getrennte Fehlernamen und
der bytegleiche Rückbau. Die Nullprobe wird ebenso erfasst. Dateihashes der sechs
Eingaben, Werkzeughash, Git-HEAD und der Quellbaumstatus benennen den Bezugspunkt.
Ein abgebrochener Durchgang behält `completed: false` und `full_pass: false`.

BAU bezeichnet einen Bauabbruch; die Rust-Suite bleibt aus. Die TS-Suite kann
trotzdem laufen und wird separat erfasst. ROT setzt ausgeführte fehlgeschlagene
Fälle voraus. Fehlende Ergebniszeilen gestarteter Suites, Prozessabbrüche und null
ausgeführte Fälle ergeben FEHLER, auch wenn die andere Sprache echte rote Tests
meldet. Ein solcher Lauf kann keine vollständige Abnahme liefern.

## Drei Vorproben

Eine leere BAU-Spalte hieß bisher entweder „kein Feld bricht den Bau" oder „die
Erkennung greift nicht" — aus der Ergebnisdatei war das nicht zu trennen.
`--vorproben` fährt deshalb vor dem ersten Feld drei positive Kontrollen über
denselben gesicherten Weg wie jedes Feld:

| Vorprobe | Mutation                                          | bestanden nur mit                                 |
| -------- | ------------------------------------------------- | ------------------------------------------------- |
| Null     | keine                                             | Bau, Rust- und TS-Suite grün                      |
| BAU      | Pflichtfeld `id` im ersten Zauber von common.json | gebrochenem Bau und `build.rs`-Panic im Protokoll |
| ROT      | `baseDamage` +1 am ersten Zauber mit Grundschaden | ROT mit mindestens einem Rust- und einem TS-Namen |

Die BAU-Vorprobe hält zusätzlich fest, ob die alte Textregel („could not
compile", „error[E") den Abbruch gefunden hätte (`old_rule_detects`). Liefert
eine Vorprobe nicht ihren Ausgang, endet der Durchgang vor dem ersten Feld mit
Exit 3.

`--vergleich` liest die historischen Urteile unverändert und benennt alle alten
Felder, deren neuer Durchgang BAU ergibt. `historical_comparison.new_bau` enthält
Feldname, damaliges Urteil und heutigen Bau-Exit. `unmeasured_old_fields` und
`new_fields` halten fehlende und hinzugekommene Felder sichtbar. Eine Teilmenge,
`--nur-bau` oder `--delta` erhält niemals `full_pass: true`.
`historical_comparison.full_pass` verlangt außerdem bestandene Vorproben
(`probes_passed`).

## Vollständiger Messlauf

**Noch nicht ausgeführt.** Der aktuelle Bestand umfasst 60 Felder, davon 58 aus
dem alten Bericht. Die Werkzeugtests führen die echten Mutations- und
Berichtsfunktionen in temporären Verzeichnissen aus; ausschließlich die externen
Bau-/Testprozesse sind kontrolliert ersetzt. Das prüft die Klassifikation und
den Rückbau, es ersetzt keinen Mutationsdurchgang des Spiels.

Vor der Messung wird der zu prüfende Stand vollständig committed und in einen
isolierten Checkout übernommen. Dort müssen die Node-Abhängigkeiten installiert
sein. Der Durchgang gehört in ein eigenes Rust-Messfenster;
währenddessen darf kein anderer Remote-Cargo-Lauf starten. Der echte Arbeitsbaum
darf nicht das Mutationsziel sein.

Gestartet wird er mit **einem** Befehl, nachdem das Messfenster der Rolle
zugeteilt ist (`rota-fenster geben <Rolle> "#1055 Volllauf"`):

```sh
python3 tools/streich-volllauf.py --rolle <Rolle> --stand <Commit>   # erst --trocken
```

Das Werkzeug prüft die Vorbedingungen selbst und startet ohne eine davon nichts
(Exit 2): Fenster der Rolle zugeteilt und nicht abgegeben, Stand ist ein Commit,
Ausgabeverzeichnis neu, 1500 MB frei auf `/work`, `package-lock.json` gleich dem
von `/work/rota`. Dann legt es den Checkout per `git worktree add --detach` unter
`/work/rota-worktrees/<Rolle>-1055-volllauf` an — dort ordnet `rota-fenster` die
Prozesse einer Rolle zu; unter `/work/tmp` wären sie unsichtbar, und die erste
Probe wurde so mitten im Lauf von einer Fensterzuteilung überholt —, übernimmt
`node_modules` per Reflink, sichert den historischen Bericht des Standes
als Vergleich und startet `streich-mutation.py --vorproben` mit allen
Nachweisdateien abgekoppelt (`setsid nohup`). Das Protokoll endet mit `rc=N`.
`--probe FELD` nimmt denselben Weg für wenige Felder und verlangt dafür ein
**freies** Fenster.

**Dauer, hochgerechnet aus der Probe vom 2026-10-28** (Stand b549c554, zwei
Felder, unter Fremdlast — das Lastmittel stand kurz vor dem Start bei 14 bis 18,
gemessen wurde also nicht im Fenster): Nullprobe 6,9 min (Bau 70 s, Rust 132 s,
TS 211 s), BAU-Vorprobe 10 min (die TS-Suite brauchte bei gebrochenem Datensatz
594 s), ROT-Vorprobe 6,5 min, die zwei Felder 2,8 und 3,9 min. Für 60 Felder
ergibt das rund 3,2 bis 4,3 Stunden. Eine Stufe, die hängt, läuft bis zum
Timeout von 1800 s; dann endet der Durchgang.

Der Checkout ist für den Bau-Dienst ein neuer Arbeitsbereich. Nach dem Lauf wird
er mit `git worktree remove` entfernt; der Bereich auf dem Server gehört danach
aufgeräumt (der der Probe belegte 4921 MB).

Das Werkzeug führt nacheinander `cargo remote --no-copy-lock -- test --no-run
--all-targets` und `cargo remote --no-copy-lock -- test --all-targets
--no-fail-fast` aus. Bau und Rust-Tests laufen vollständig auf dem Buildserver;
Rust-Binaries werden niemals lokal gestartet. Beide Aufrufe verzichten auf
Artefakt-Copyback und sperren die Rückkopie von `Cargo.lock`. Die TS-Prüfung läuft
über `npx --no-install vitest run --testTimeout 30000`. Die einzelnen Tests
erhalten 30 Sekunden statt Vitests regulären fünf Sekunden: Dateisystem- und
AST-Prüfungen überschritten unter Hostlast bereits in der Nullprobe die kurze
Vorgabe. Assertions und Testumfang bleiben unverändert. Die Cargo-Ausgabe liefert alle Testnamen und
Suite-Summen ohne `--nocapture`; ein abgebrochener weiterer Target-Prozess macht
auch vorherige echte rote Summen zu einem unvollständigen Lauf. Reguläre leere
Binär-Harnesses sind erlaubt, zählen aber als null ausgeführte Fälle.

Je Mutation läuft die vollständige lokale TS-Suite parallel zum Rust-Zweig.
Remote-Bau und Remote-Tests bleiben strikt nacheinander; beide Zweige lesen
denselben unveränderten Mutationsstand. Erst nach Abschluss beider Zweige werden
die Daten zurückgesetzt. Beginn und Dauer jeder Stufe stehen im JSON-Nachweis.
Die Nullprobe bleibt seriell.

Ein Timeout stoppt den Durchgang, beendet die eigenen Prozessgruppen und
stellt die sechs Datendateien wieder her. Der Nachweis bleibt ausdrücklich
unvollständig. Vor einem erneuten Lauf ist der Abbruchgrund zu untersuchen.
Bei Timeout, SIGTERM oder einem Fehler eines parallelen Zweigs werden beide
eigenen Prozessgruppen beendet und der TS-Thread vor dem Rückbau abgewartet.
„Beendet" heißt: Kein Mitglied der Gruppe läuft mehr, ein Zombie zählt als
beendet, längstens fünf Sekunden nach SIGKILL. Auf das eigene Kind zu warten
reicht nicht — das ist die Hülle, und unter dash (dem `/bin/sh` der CI) sowie
unter npx und cargo remote arbeiten deren Kinder. Ein Enkel stand in 50 von 50
Läufen noch auf `R`, als die alte Fassung zurückkehrte.
Das Ende eines lokalen SSH-Prozesses belegt noch nicht das Ende von Remote-Cargo:
Nach einem solchen Abbruch muss vor einem Neustart zusätzlich der Zustand auf
dem Buildserver geprüft werden. Der abgebrochene Durchgang startet kein weiteres
Feld.

Die Abnahme verlangt `completed` und `historical_comparison.full_pass` (und damit
bestandene Vorproben), alle 60 aktuellen Felder, keine ungemessenen historischen
Felder und einen geprüften Quellbezug. Erst dann wird der historische Bericht durch die neue Messung ersetzt
und jedes von ROT/gedeckt nach BAU wechselnde Feld einzeln aufgeführt. Wechselt
kein Feld, muss auch dieser Befund aus dem vollständigen Lauf stammen.

Doc-Tests sind im bestehenden `--all-targets`-Mutationsweg nicht enthalten. Der
normale vollständige Repository-Prüflauf bleibt eine eigene Pflichtprüfung.

## Wertänderung (`--delta`, #1087)

Die Streichung fragt „ist das Feld Pflicht", die Wertänderung „ist der Wert
gedeckt". Die Zeitregel liest `--delta` über `policy()` (tools/spell-time-audit.py),
nicht selbst, und ordnet jedes Feld ein: `raster` (jedes `…Ms`-Feld außer den
Schwellen), `schwelle` (heute `cooldownReductionBelowMs`), `ganzzahl`, `bruchzahl`,
`keine-zahl`. Ein Schritt, der die Deckung nicht messen kann, startet nicht
(Exit 2): nicht ganzzahlig an einem Ganzzahlfeld, neben dem 10-ms-Takt an einem
Rasterfeld, Schritt 0 oder ein Feld ohne Zahl. `--art` und mehrfaches `--feld`
wählen aus.

Je Feld und Datei stehen im Nachweis vier SHA-256-Abdrücke: vorher, mutiert, vor
dem Rückbau, nachher. Weicht „vor dem Rückbau" von „mutiert" ab, hat während des
Laufs jemand geschrieben, und das Feld heißt FEHLER. Weicht „nachher" von
„vorher" ab, endet der Durchgang.

Vor jedem Feld steht im Protokoll die Frage „Liegt eine Entscheidung genau auf
dem Wert?" mit der Antwort aus `--entscheidung FELD=ja:FUNDSTELLE` (oder `nein`),
im Nachweis als `decision_on_value`. Nur mit „ja" zählt ein GRÜN als ungedeckt;
sonst heißt es GRUEN? und steht unter OHNE AUSSAGE.
