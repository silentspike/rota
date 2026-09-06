# Streich-Mutationen: Bau und Tests getrennt nachweisen (#1055)

Die Korrektur der Baufehler-Erkennung allein erneuert keine Messung. Ein
`build.rs`-Panic kann mit Exit 101 enden, ohne eines der alten Compilertextmuster
zu enthalten. `bau_gebrochen` wertet deshalb den Exit-Code zuerst aus. Nach einem
Bauabbruch wird kein Rust-Testlauf gestartet.

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

`--vergleich` liest die historischen Urteile unverändert und benennt alle alten
Felder, deren neuer Durchgang BAU ergibt. `historical_comparison.new_bau` enthält
Feldname, damaliges Urteil und heutigen Bau-Exit. `unmeasured_old_fields` und
`new_fields` halten fehlende und hinzugekommene Felder sichtbar. Eine Teilmenge,
`--nur-bau` oder `--delta` erhält niemals `full_pass: true`.

## Vollständiger Messlauf

**Noch nicht ausgeführt.** Der aktuelle Bestand umfasst 60 Felder, davon 58 aus
dem alten Bericht. Die Werkzeugtests führen die echten Mutations- und
Berichtsfunktionen in temporären Verzeichnissen aus; ausschließlich die externen
Bau-/Testprozesse sind kontrolliert ersetzt. Das prüft die Klassifikation und
den Rückbau, es ersetzt keinen Mutationsdurchgang des Spiels.

Vor der Messung wird der zu prüfende Stand vollständig committed und in einen
isolierten Checkout übernommen. Dort müssen die Node-Abhängigkeiten installiert
sein. Der durchgehend serialisierte Lauf gehört in ein eigenes Rust-Messfenster;
währenddessen darf kein anderer Remote-Cargo-Lauf starten. Der echte Arbeitsbaum
darf nicht das Mutationsziel sein.

Beispielaufruf **aus dem isolierten Workspace**; das Zielverzeichnis vorher
anlegen und den historischen Bericht dort unverändert sichern:

```sh
python3 -B tools/streich-mutation.py \
  --aus /work/tmp/rota-1055/result.md \
  --roh /work/tmp/rota-1055/failure-names.json \
  --nachweis /work/tmp/rota-1055/evidence.json \
  --vergleich /work/tmp/rota-1055/historical-report.md \
  --bau-log /work/tmp/rota-1055/build-current.log \
  --timeout 1800
```

Das Werkzeug führt nacheinander `cargo remote --no-copy-lock -- test --no-run
--all-targets` und `cargo remote --no-copy-lock -- test --all-targets
--no-fail-fast` aus. Bau und Rust-Tests laufen vollständig auf dem Buildserver;
Rust-Binaries werden niemals lokal gestartet. Beide Aufrufe verzichten auf
Artefakt-Copyback und sperren die Rückkopie von `Cargo.lock`. Die TS-Prüfung läuft
über `npx --no-install vitest run`. Die Cargo-Ausgabe liefert alle Testnamen und
Suite-Summen ohne `--nocapture`; ein abgebrochener weiterer Target-Prozess macht
auch vorherige echte rote Summen zu einem unvollständigen Lauf. Reguläre leere
Binär-Harnesses sind erlaubt, zählen aber als null ausgeführte Fälle.

Ein Timeout stoppt den Durchgang, beendet die eigene Prozessgruppe und
stellt die sechs Datendateien wieder her. Der Nachweis bleibt ausdrücklich
unvollständig. Vor einem erneuten Lauf ist der Abbruchgrund zu untersuchen.

Die Abnahme verlangt `completed` und `historical_comparison.full_pass`, alle
60 aktuellen Felder, keine ungemessenen historischen Felder und einen geprüften
Quellbezug. Erst dann wird der historische Bericht durch die neue Messung ersetzt
und jedes von ROT/gedeckt nach BAU wechselnde Feld einzeln aufgeführt. Wechselt
kein Feld, muss auch dieser Befund aus dem vollständigen Lauf stammen.

Doc-Tests sind im bestehenden `--all-targets`-Mutationsweg nicht enthalten. Der
normale vollständige Repository-Prüflauf bleibt eine eigene Pflichtprüfung.
