# Lastwarnung vor Bash-Aufrufen (#1049)

Der in `.claude/settings.json` registrierte `PreToolUse`-Hook warnt die
rechnende Sitzung, bevor sie die gemeinsame Maschine belastet, während eine
andere Sitzung das Messfenster hält. Er warnt bei den gemeldeten Fällen
`npm run lint`, `npx prettier --write docs/WORKER-GUIDE.md` und
`jq '.casts' tests/parity/recordings/*` sowie bei sichtbaren Cargo-, Test- und
Werkzeugläufen, Git-Pushes und Worktree-Änderungen. `git status`, das Lesen
einzelner Dokumente und bloß zitierte Befehle bleiben im selben fremden Fenster
still.

## Ablauf: zuerst der Schalter

1. **Kein Fensterwerkzeug, freies, abgegebenes oder eigenes Fenster:** Der Hook
   endet, bevor er den Befehl ansieht. Es erscheint keine Meldung, auch nicht
   für Eingaben, die er nicht zerlegen könnte (Punkt 2 des Issues). Eigene
   Sitzungen erkennt er an der gespeicherten Kennung, einschließlich `session-`
   mit acht UUID-Zeichen. Nur alte Stände ohne Kennung verwenden die geprüfte
   Rollenauskunft `rota-ipc wer`; sie wird erst eingeholt, wenn der Befehl sonst
   warnen würde, weil sie einen Prozessstart kostet. Ein neuer Prozess mit
   derselben Rolle übernimmt dadurch nicht stillschweigend das Fenster einer
   alten Sitzung.
2. **Fremd gehaltenes Fenster:** Erst jetzt lädt der Hook die Zerlegung
   `tools/fenster_lastpruefung.py`. Ein belastender Befehl erhält die Warnung mit
   Inhaber, Zweck und Beginn. Ist die Last nicht entscheidbar, nennt dieselbe
   Meldung Inhaber, Zweck und Beginn und dazu „Lastprüfung UNBEKANNT". Ein
   ruhiger Befehl bleibt still.
3. **Unlesbarer oder ungültiger Schalter:** Bei belastenden oder nicht
   entscheidbaren Befehlen meldet der Hook „Belegung UNBEKANNT"; ruhige Befehle
   bleiben still.

Der Hook gibt immer Exit 0 zurück und setzt weder `permissionDecision` noch
`decision` oder `continue`. `additionalContext` erreicht das arbeitende Modell,
`systemMessage` macht die Warnung zusätzlich sichtbar. Die bestehenden
Berechtigungen bleiben wirksam. Dieses Ausgabeformat folgt der
[offiziellen Hook-Referenz](https://code.claude.com/docs/en/hooks#pretooluse-decision-control).

## Was als Last zählt

- **Werkzeuge:** `npm`, `npx`, `pnpm`, `yarn`, `cargo` einschließlich
  `cargo remote`, `cargo-remote-fmt`, `rustc`, `vitest`, `prettier` und
  `rota-sim`; Skripte unter `tools/`, direkt oder über `bash`, `sh`, `zsh`,
  `python3`, `node`, `source` und Prozess-Ersetzung wie
  `bash <(git show origin/main:tools/…)`; lokal gebaute Rust-Programme unter
  `target/{debug,release}/{deps,examples}/`; `git push` und
  `git worktree add/remove`; `jq`, wenn der Befehl `recordings/` nennt.
- **Hüllen werden ausgepackt:** `env`, `sudo`, `command`, `builtin`, `exec`,
  `time`, `timeout`, `nice`, `nohup`, `setsid`, `stdbuf`, `ionice`, `taskset`,
  `xargs` und `find -exec`. Ebenso zählen Befehle in `if`- und
  `while`-Bedingungen, in `{ …; }`, in Funktionsrümpfen, in `$( )` und
  Backquotes, auch innerhalb doppelter Anführungszeichen, und in Zeilen nach
  einem `#`-Kommentar.
- **Hilfe und Version bleiben still**, auch mit Unterbefehl:
  `cargo remote --help`, `cargo test --help`, `cargo help build`,
  `npx prettier --help`, `npm run lint --help`. Nicht ausgenommen sind Schalter
  nach `--`, weil sie an das gestartete Programm gehen (`cargo test -- --help`
  baut zuerst), `-h` bei Cargo Remote v2, weil es dort `--sync-hidden` bedeutet,
  und Schalter nach einem Skriptnamen bei `pnpm` und `yarn`.
- **Heredoc-Körper sind Daten:** Commit-Texte über `git commit -F -`, PR-Texte
  über `gh pr create --body-file -`, `rota-ipc senden … - <<'ENDE'` und
  `cat <<'EOF'`. Ausnahmen sind `bash`, `sh` und `zsh`, die den Körper als Code
  lesen, sowie ein ungequoteter Begrenzer, bei dem die Shell `$( )` und
  Backquotes im Körper ausführt. Python- und Node-Code als Heredoc, `-c` oder
  `-e` zählt nur, wenn er sichtbar etwas startet (`subprocess`, `os.system`,
  `exec`, `spec_from_file_location` und Ähnliches) und das Gestartete ein
  `tools/`-Pfad, ein als Zeichenkette genanntes Lastwerkzeug oder ein
  `target/`-Programm ist. Das Lesen oder Ändern einer Datei unter `tools/` ist
  kein Werkzeuglauf.
- **Nicht entscheidbar** sind Text, den Bash selbst ablehnen würde (offene
  Anführungszeichen oder Backquotes), Code, der per Pipe in einen Interpreter
  fließt, und Programme, deren Name erst zur Laufzeit feststeht (`"$B"`). Im
  untersuchten Korpus waren das überwiegend lokal gestartete Rust-Testprogramme.
  Gemeldet wird das nur bei fremd gehaltenem Fenster.

## Ausgeführte Gegenproben

Der rote Commit `07178c08` enthält die ursprünglich fehlende Hook-Integration.
Der rote Commit `98383e91` belegt die Befunde B1 bis B4 am unveränderten Stand:
Meldungen bei freiem, abgegebenem und eigenem Fenster, Heredoc-Texte, Hüllen,
Hilfe-Formen, versteckte Befehle und eine Nachricht, deren Zerlegung den
3-Sekunden-Timeout überschritt. Er zeigte 61 rote Unterfälle, alle als
fehlgeschlagene Zusicherung. Danach bestehen 21 Verhaltenstests mit
Unterfällen:

```bash
python3 -B -m unittest discover -s tests/tools -p test_fenster_hook.py -v
```

Dabei läuft das echte Hookprogramm mit echten Dateien in einem temporären
Verzeichnis. Nur die Rollenauskunft und das Vorhandensein des Fensterwerkzeugs
sind kontrollierte lokale Attrappen. Der echte Schalterstand wird nie verändert,
kein IPC-Versand aufgerufen und kein untersuchter Befehl ausgeführt. Warnen und
Schweigen werden im selben fremden Stand sowie bei freiem, abgegebenem und
eigenem Stand geprüft. Zwei Proben führen genau den konfigurierten Hook-Befehl
aus einem Projektpfad mit Leerzeichen aus; die zweite zeigt, dass die Zerlegung
neben dem Hook geladen wird, kein `__pycache__` entsteht und ein fehlendes Modul
als „UNBEKANNT" gemeldet wird statt mit einem Absturz. Die Tests laufen
dauerhaft im Python-CI-Schritt.

Dass die Fälle scheitern können, belegen 14 Mutationen in einer Wegwerfkopie.
Jede schaltet eine Regel ab: Schalter zuerst, Heredoc-Körper, startender
Inline-Code, Hüllen, `sudo`, `find -exec`, `source`, Hilfe mit Unterbefehl,
`-h` unter Cargo Remote v2, Laufzeitnamen, Pipe in einen Interpreter, Ersetzung
in doppelten Anführungszeichen, Kommentarende und Inhaber in der
UNBEKANNT-Meldung. Jede Mutante ließ sich übersetzen und färbte 1 bis 14 Fälle
der zuständigen Tests rot, jeweils als fehlgeschlagene Zusicherung.

## Korpusmessung

Grundlage sind alle 33.268 Bash-Aufrufe aus den Transkripten früherer
Worker-Sitzungen (`~/.claude/projects/-work-rota-worktrees-*/*.jsonl`, sieben
Dateien, letzte Änderung am 22. August 2026; ein doppelt gespeicherter Aufruf
zählt wie in der Ausgangsmessung zweimal). Gezählt wurde die Rückgabe von
`load_command`: wahr, falsch oder nicht entscheidbar. Das Messskript und die
Rohbefehle liegen privat unter `/work/tmp` und sind nicht eingecheckt.

| Klasse            | vorher (`8aa7e644`) | nachher |
| ----------------- | ------------------: | ------: |
| still             |              21.589 |  22.883 |
| warnt             |               8.837 |  10.272 |
| unbekannt         |               2.842 |     113 |
| Absturz           |                   0 |       0 |
| längste Zerlegung |     15,8 bis 17,8 s |  5,0 ms |

Vorher brauchte die Zerlegung bei 2 Befehlen länger als den konfigurierten
Hook-Timeout von 3 Sekunden und bei 27 länger als eine Sekunde (zweiter von
zwei Läufen). Dort hätte Claude Code den Hook abgebrochen, und die Warnung wäre
ausgeblieben.

| Wechsel           | Anzahl | Ursachen                                                                                                                                     |
| ----------------- | -----: | -------------------------------------------------------------------------------------------------------------------------------------------- |
| still → warnt     |    909 | 814 hinter Hüllen (`timeout` 655, `nohup` 76, `setsid` 69, `taskset` 9, `time` 5), 81 Inline-Code, 10 direkt, 4 in `$( )`                    |
| unbekannt → warnt |  1.253 | 1.099 direkte Lastbefehle, die die alte Zerlegung nicht trennen konnte, 110 hinter Hüllen, 35 Inline-Code, 9 in `$( )`                       |
| warnt → still     |    726 | 671 Heredoc-Körper (`rota-ipc` 566, `git` 44, `gh` 43, `cat` 13, `python3` 5), 41 Datenwörter oder Muster, 13 Hilfe/Version, 1 ohne Lastwort |
| unbekannt → still |  1.561 | 1.249 ohne jedes Lastwort, 164 Heredoc-Körper, 148 Lastwörter nur als Daten, Argument oder Muster                                            |
| still → unbekannt |     84 | 81 Programmname erst zur Laufzeit bekannt, 3 offene Backquotes, an denen Bash selbst scheitert                                               |
| warnt → unbekannt |      1 | Python-Heredoc, dessen Körper eine Zeile mit dem Begrenzer enthält; der Rest ist unbalanciertes Quoting, an dem Bash scheitert               |

Die Zerlegung der stillen Befehle wurde unabhängig gegengeprüft: bashlex 0.18,
eine Python-Portierung des Bash-Parsers, ermittelte für alle 22.883 nachher
stillen Befehle die einfachen Befehle, und jeder wurde mit denselben Regeln je
Einzelbefehl bewertet. Gequotete Heredoc-Körper wurden dafür entfernt, weil
bashlex gequotete Begrenzer nicht kennt und Bash sie wörtlich liest. Für die
22.545 von bashlex zerlegbaren Befehle fand diese Gegenprobe keinen
Lastbefehl. Die Regeln selbst prüft sie nicht; deren Wirkung zeigen die
Ursachen der Wechsel oben. Die 46 neu stillen Befehle, die bashlex nicht
zerlegen konnte, wurden einzeln gelesen. Sie enthalten Lastwörter nur in
`echo`-Texten, `grep`- und `case`-Mustern, Schleifenwörtern, Python-Code ohne
Start und Gegenproben-Skripten außerhalb von `tools/`.

Die Korpusprüfung hat die Regel für Python- und Node-Code geschärft. 48
Befehle enthalten Code, der über `subprocess` oder `exec` ein namentlich
genanntes Lastwerkzeug oder ein `target/`-Programm startet, etwa
`npx vitest run`, `npm run parity:export` oder `cargo remote -c -- test`. Mit
der Regel „nur `tools/` zählt" blieben 42 davon still, gemessen am Entwurf vor
der Schärfung; jetzt warnen alle 48. 89 Code-Körper luden oder starteten ein
Programm unter `tools/`. 105 nannten einen `tools/`-Pfad nur, um Dateien zu
lesen, zu prüfen oder zu ändern; das zählt nicht als Werkzeuglauf.

## Gemessener Preis

Messung am 23. Oktober 2026 am Stand `a4227c3a` auf dem gemeinsam genutzten
lokalen Rechner (16 Threads, Last 8,4 bis 11,2 durch fremde Programme; das
Messfenster war frei). Je Fall und Fassung liefen fünf Aufwärmrunden und 30
neue Prozesse des exakt konfigurierten Befehls über `/bin/bash -c`,
einschließlich Bash- und Python-Start, mit isoliertem Schalter und bekannter
Sitzungskennung. Alte und neue Fassung liefen in jeder Runde abwechselnd und
über alle Fälle im Rundlauf, damit beide dieselbe Fremdlast sahen. Jeder Lauf
prüfte, dass der beabsichtigte Pfad durchlaufen wurde; es gab keine
Abweichung. Es wurde kein Modell aufgerufen. P95 ist der Rangwert.

| Aufruf und Stand                   | neu Median |   neu P95 | vorher Median |  vorher P95 |
| ---------------------------------- | ---------: | --------: | ------------: | ----------: |
| `npm run lint`, frei, still        |   50,89 ms | 112,62 ms |      81,98 ms |   145,99 ms |
| `npm run lint`, fremd, Warnung     |   65,73 ms | 117,23 ms |      78,41 ms |   146,09 ms |
| `git status --short`, fremd, still |   60,49 ms | 128,93 ms |      58,37 ms |   118,12 ms |
| Nachricht mit 200 Zeilen, fremd    |   66,02 ms | 127,59 ms |   3.711,38 ms | 4.145,12 ms |

Die alte Fassung lief beim letzten Fall nur fünfmal und meldete dort
„UNBEKANNT". Eine unveränderte Wiederholung ergab für die neue Fassung Mediane
von 51,51, 64,50 und 60,13 ms in den ersten drei Fällen, für die alte 66,33,
72,90 und 52,19 ms. Die Mediane derselben Fassung wichen zwischen den beiden
Läufen um 0,4 bis 16 ms voneinander ab. In beiden Läufen war die neue Fassung
beim freien Fenster und bei der Warnung günstiger und beim stillen Befehl im
fremden Fenster 2 bis 8 ms teurer, weil dort die Zerlegung übersetzt wird.
1.000 Aufrufe bei freiem Fenster entsprächen mit dem ersten Median rund 51
Sekunden zusätzlicher Laufzeit.

Das sind gemessene Kosten auf diesem belasteten Host, keine zugesicherte
Obergrenze. Alte Schalterstände ohne Kennung können zusätzlich den bis zu einer
Sekunde begrenzten IPC-Leseaufruf benötigen; dieser Sonderfall ist nicht in der
Tabelle. Rohwerte und Quellhashes liegen in den lokalen Abnahmebelegen
`fenster-1049-cost-v2.json` und `fenster-1049-cost-v2-wiederholung.json`.

## Geltungsbereich

Dies ist eine Warnung für sichtbare Bash-Aufrufe in Claude Code. Sie ist kein
vollständiger Shell-Interpreter und keine Lastsperre. Aliase, Funktionen aus
früheren Aufrufen, Skripte außerhalb von `tools/` und Programme, die
Python- oder Node-Code aus berechneten Listen startet, bleiben unerkannt. Die
`jq`-Regel braucht `recordings/` mit Schrägstrich, `cd tests/parity/recordings`
gefolgt von `jq` erkennt sie nicht. `watch`, `parallel`, `flock` und `chrt`
werden nicht ausgepackt; im Korpus kamen sie nicht als Befehl vor. Außerhalb
dieses Hook-Hosts greifen weiterhin nur die direkten npm-/Werkzeugwarnungen aus
`fenster-warnung.sh`; daraus folgt keine Erfassung aller Codex-Werkzeugaufrufe.

Geprüft sind Hook-Konfiguration, echte Prozessausführung und das dokumentierte
Ein-/Ausgabeformat. Eine neue interaktive Claude-Sitzung wurde für diese Abnahme
nicht gestartet. Bereits laufende Sitzungen müssen die geänderte Projekt-
Hook-Konfiguration neu laden, bevor sie den zusätzlichen Hook ausführen.
