# Lastwarnung vor Bash-Aufrufen (#1049)

Der in `.claude/settings.json` registrierte `PreToolUse`-Hook liest den geplanten
Bash-Befehl und den Messfensterstand. Er warnt bei den drei gemeldeten Fällen:
`npm run lint`, `npx prettier --write docs/WORKER-GUIDE.md` und
`jq '.casts' tests/recordings/*`. Auch sichtbare Cargo-, Test- und Werkzeugläufe,
die üblichen `env`-/`time`-Hüllen und Git-Worktree-Änderungen werden erkannt.
`git status`, das Lesen einzelner Dokumente und bloß zitierte Befehle bleiben
im selben fremden Fenster still.

Die Warnung nennt Inhaber, Zweck und Beginn. Eigene Sitzungen werden anhand
der tatsächlich gespeicherten Kennung erkannt, einschließlich `session-` mit
acht UUID-Zeichen. Nur alte Stände ohne Sitzungskennung verwenden die geprüfte
Rollenauskunft `rota-ipc wer`. Ein neuer Prozess mit derselben Rolle übernimmt
dadurch nicht stillschweigend das Fenster einer alten Sitzung. Freie und
abgegebene Fenster schweigen; fehlende oder ungültige Schalterstände werden
bei einem relevanten Befehl ausdrücklich als unbekannt gemeldet.

Der Hook gibt immer Exit 0 zurück und setzt weder `permissionDecision` noch
`decision` oder `continue`. `additionalContext` erreicht das arbeitende Modell,
`systemMessage` macht die Warnung zusätzlich sichtbar. Die bestehenden
Berechtigungen bleiben wirksam. Dieses Ausgabeformat folgt der
[offiziellen Hook-Referenz](https://code.claude.com/docs/en/hooks#pretooluse-decision-control).

## Ausgeführte Gegenproben

Der rote Commit `07178c08` enthält die noch fehlende Hook-Integration als
fehlgeschlagene Gegenprobe. Anschließend bestehen neun Verhaltenstests mit
zusätzlichen Unterfällen:

```bash
python3 -B -m unittest discover -s tests/tools -p test_fenster_hook.py -v
```

Dabei läuft das echte Hookprogramm mit echten Dateien in einem temporären
Verzeichnis. Nur die Rollenauskunft und das Vorhandensein des Fensterwerkzeugs
sind kontrollierte lokale Attrappen. Der echte Schalterstand wird nie verändert,
kein IPC-Versand aufgerufen und kein untersuchter Befehl ausgeführt. Warnen und
Schweigen werden im selben fremden Stand sowie bei freiem Stand geprüft.
Eine weitere Probe führt genau den konfigurierten Hook-Befehl aus einem
Projektpfad mit Leerzeichen aus. Die Tests laufen dauerhaft im Python-CI-Schritt.

## Gemessener Preis

Messung am 6. September 2026 auf dem gemeinsam genutzten lokalen Rechner:
je fünf Aufwärmrunden und 30 neue Prozesse, einschließlich Bash- und
Python-Start des exakt konfigurierten Befehls. Währenddessen lief keine weitere
lokale Testsuite unseres Teams; fremde Systemlast war vorhanden. Der Schalter
lag isoliert vor, mit bekannter Sitzungskennung. Es wurde kein Modell aufgerufen.

| Aufruf und Stand                   |   Median | 95. Perzentil | Minimum–Maximum |
| ---------------------------------- | -------: | ------------: | --------------: |
| `git status --short`, fremd, still | 42,50 ms |      52,49 ms |  34,33–55,89 ms |
| `npm run lint`, fremd, Warnung     | 54,44 ms |      68,73 ms |  47,04–69,65 ms |
| `npm run lint`, frei, still        | 59,35 ms |      76,96 ms | 43,00–150,16 ms |

Das sind gemessene Kosten auf diesem belasteten Host, keine zugesicherte
Obergrenze. 1.000 ruhige Aufrufe entsprächen mit dem gemessenen Median rund
42,5 Sekunden zusätzlicher Laufzeit. Die Reihenfolge frei/fremd ist wegen
wechselnder Fremdlast kein Vergleich der Rechenkosten der einzelnen Zweige.
Alte Schalterstände ohne Kennung können zusätzlich den bis zu einer Sekunde
begrenzten IPC-Leseaufruf benötigen; dieser Sonderfall ist nicht in der Tabelle.
Rohwerte und Quellhash liegen im lokalen Abnahmebeleg
`fenster-1049-cost-configured.json`.

## Geltungsbereich

Dies ist eine Warnung für sichtbare Bash-Aufrufe in Claude Code. Sie ist kein
vollständiger Shell-Interpreter und keine Lastsperre. Dynamische Befehlsnamen,
Aliase, unbekannte Skripte, Backticks und innerhalb doppelter Anführungszeichen
versteckte Befehlsersetzungen wie `echo "$(npm run lint)"` sind nicht vollständig
klassifiziert. Parserfehler werden als unbekannte Prüfung gemeldet. Außerhalb
dieses Hook-Hosts greifen weiterhin nur die direkten npm-/Werkzeugwarnungen
aus `fenster-warnung.sh`; daraus folgt keine Erfassung aller Codex-Werkzeugaufrufe.

Geprüft sind Hook-Konfiguration, echte Prozessausführung und das dokumentierte
Ein-/Ausgabeformat. Eine neue interaktive Claude-Sitzung wurde für diese Abnahme
nicht gestartet. Bereits laufende Sitzungen müssen die geänderte Projekt-
Hook-Konfiguration neu laden, bevor sie den zusätzlichen Hook ausführen.
