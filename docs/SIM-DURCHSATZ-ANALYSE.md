---
id: SIM-DURCHSATZ-ANALYSE
status: Review
ssot: true
refs:
  docs: [SIM-ARCHITEKTUR-PLAN.md]
affects: [Orchestrator, Worker-1, Worker-2, Worker-3]
---

# Was den Durchsatz der Sim-Umsetzung bremst

Auswertung der drei Worker-Sitzungsprotokolle, gemessen am 2026-08-19.

## Methode und Ankerpunkt

**Anker: 2026-08-18T11:33:20Z** — die erste Sim-Zuteilung an alle drei Worker
(IPC-Nachrichten 8035, 8036, 8044). Alles davor gehört nicht zur Sim-Umsetzung.

Ausgewertet wurden die vollständigen JSONL-Protokolle, streamend geparst:

```
Worker-1  /…/-work-rota-worktrees-e1/bccf9b2f….jsonl   147,5 MB   53 023 Zeilen, 5 470 nach Anker
Worker-2  /…/-work-rota-worktrees-e2/42ab9b94….jsonl   181,7 MB   70 527 Zeilen, 5 670 nach Anker
Worker-3  /…/-work-rota-worktrees-e9/253e6fbb….jsonl   261,8 MB   80 039 Zeilen, 5 239 nach Anker
```

Gemessen wurde: Werkzeugaufrufe je Typ, Dauer je Aufruf (Differenz `tool_use` → `tool_result`),
Zeitlücken, Fehlerklassen, Datei-Zugriffe, IPC-Volumen, Token-Verbrauch. Die Skripte liegen in
`/work/tmp/orch-analyse/`.

## Die Bilanz

| Größe                                                      | Wert                             |
| ---------------------------------------------------------- | -------------------------------- |
| Kalenderzeit                                               | 21,4 h                           |
| verfügbare Worker-Stunden                                  | 64,2 h (3 × 21,4)                |
| **messbarer Leerlauf** (Fertigmeldung → nächste Zuteilung) | **33,9 h = 53 %**                |
| Werkzeugzeit gesamt (alle drei)                            | 9,62 h = 15 %                    |
| davon Bau (remote)                                         | 5,18 h = 54 % der Werkzeugzeit   |
| abgeschlossene Issues                                      | 11                               |
| Werkzeugaufrufe                                            | 2 564, davon Bash 1 913 (75 %)   |
| Werkzeug-Fehlerquote                                       | 0,8–1,0 %                        |
| Output-Tokens                                              | 6,1 Mio (2,17 / 2,16 / 1,83 Mio) |

**Die Fehlerquote von 0,8 % ist der wichtigste Kontext für alles Folgende.** Die Worker machen
kaum Werkzeugfehler. Was den Durchsatz bremst, ist nicht Ungenauigkeit — es ist Wartezeit und
Bauzeit.

## Bremse 1: Der Leerlauf — 33,9 h, und er gehört dem Orchestrator

**Das größte Einzelproblem, und es liegt nicht bei den Workern.**

```
Worker-3   fertig 18.08 16:23  ->  Zuteilung 19.08 04:34  =  12,19 h
Worker-2   fertig 18.08 18:54  ->  Zuteilung 19.08 04:34  =   9,67 h
Worker-2   fertig 18.08 15:53  ->  Zuteilung 18.08 18:19  =   2,44 h
Worker-1   fertig 19.08 06:27  ->  Zuteilung 19.08 08:44  =   2,29 h
```

Zwei Blöcke — 12,19 h und 9,67 h — machen 22 der 33,9 h aus. Beide liegen in derselben Nacht,
und beide haben dieselbe Ursache: **Der Orchestrator hat nicht zugeteilt.** Die Worker haben
fertiggemeldet und gewartet.

**Die Struktur, die es verstärkt.** 12,8 h der Kalenderzeit waren **alle drei gleichzeitig**
still (66 % aller Stille ist gemeinsam). Ein Worker, der seinen Turn beendet, läuft erst weiter,
wenn eine Nachricht kommt — und die Lücken beginnen im 20-Minuten-Raster (:00, :20, :40), also
im Takt der Systemnachrichten:

```
Minute :00-09   73 Lücken
Minute :20-29   77 Lücken
Minute :40-49   53 Lücken
```

Die längsten Lücken (38–40 min) haben alle dasselbe Muster: davor ein `stop_hook_summary`
(121× bei Worker-3), danach eine Erinnerungs-Nachricht. **Der Worker wird geweckt, aber nicht
beschäftigt.**

Dazu kommt `WIEDERHOLUNG_MINUTEN = 60` im Watcher: Dieselbe Stillstandsmeldung wird nur
stündlich wiederholt. Ein Worker, dessen Stillstand einmal gemeldet wurde, fällt für eine Stunde
aus der Aufsicht.

### Die Ursache hinter der Ursache — nachgetragen auf Hinweis des Auftraggebers

Der Abschnitt oben sagt „der Orchestrator hat nicht zugeteilt". Das ist richtig und zu
oberflächlich. **Warum** er nicht zugeteilt hat, ist messbar:

```
18.08 21:25   AskUserQuestion gestellt
18.08 21:26   Antwort da — Wartezeit 0,4 min
18.08 21:26   die Orchestrator-Sitzung wird still
19.08 04:32   erste Aktivität danach       426,5 min = 7,1 h
```

**Die 7,1 h Stille des Orchestrators erklären die beiden Worker-Blöcke (12,19 h und 9,67 h)
fast vollständig.** Die Worker waren nicht langsam — es war niemand da, der zuteilt.

**Und was dabei nicht die Ursache ist:** Die reine Wartezeit auf `/ask`-Antworten beträgt über
den ganzen Zeitraum **1,13 h** — sechs Aufrufe, 0,0 bis 44,5 min. Die Antworten kommen schnell.

Das Problem ist, dass ein `/ask` den **Turn beendet**, und ein beendeter Turn ohne eingehende
Nachricht ist Stillstand — genau derselbe Mechanismus, der bei den Workern die
38-Minuten-Lücken erzeugt (`stop_hook_summary`, 121×). Der Unterschied: Beim Worker kostet es
ihn allein, beim Orchestrator kostet es alle drei.

**Zwei Folgerungen:**

1. **Nach einem `/ask` weiterarbeiten, nicht stoppen.** Vor der Frage prüfen, was ohne die
   Antwort weitergeht — und das im selben Turn tun.
2. **`/ask` nur, wo der Plan wirklich schweigt.** Am 2026-08-19 wurden vier Entscheidungen ohne
   Rückfrage getroffen, alle aus dem Plan belegbar: #908 AC4 (GAME-DESIGN 18.4 plus
   `fight.rs:5221`), #908 AC5 (Plan), #909 AC1 (Plan Zeile 73), #915 AC4 (vier gemessene
   Aufzeichnungen). Keine davon brauchte eine Frage.

> Die Prüfung ist immer dieselbe: **Steht es im Plan oder in GAME-DESIGN?** Dann lesen statt
> fragen. Und vor jedem `/ask` gehört in den Text, _wo_ nachgesehen wurde und dass die Stelle
> schweigt — wer das nicht schreiben kann, hat nicht nachgesehen.
>
> `/ask` bleibt richtig für: Änderungen **am** Design, neue Issues (untersagt), Erweiterungen
> über den Auftrag hinaus, und Kosten, die sich nicht abschätzen lassen.

### Vorschlag 1a — Zuteilungsvorrat statt Zuteilung auf Anfrage

**Wo:** Verfahren des Orchestrators, dazu `docs/WORKER-GUIDE.md`.

Jeder Worker bekommt **mit der Zuteilung schon das nächste Issue genannt** — nicht als Auftrag,
sondern als Vorrat: „Nach #908: nimm #915, sofern main es noch trägt." Er misst dann selbst, ob
es gilt (das tut er ohnehin), und fängt an, statt zu warten.

**Warum:** Die Zuteilung kostet mich 30 Sekunden, das Warten kostet den Worker Stunden. Der
Vorrat verschiebt die Latenz von der teuren auf die billige Seite. Und er ist billiger als
jede Aufsicht: Ein Worker mit Vorrat stoppt nicht, also muss ihn niemand wecken.

**Risiko und Grenze:** Ein Vorrat kann veralten (das Issue wird von jemand anderem erledigt, die
Voraussetzung fällt weg). Deshalb _„sofern main es noch trägt"_ — die Bestandsmessung, die
Regel 5b ohnehin verlangt, entscheidet.

### Vorschlag 1b — Die Fertigmeldung erzeugt eine Aufgabe beim Orchestrator

**Wo:** `rota-ipc`, beim Empfang einer Nachricht mit `[FERTIG`.

Eine Fertigmeldung ist der einzige Nachrichtentyp, der **ohne Antwort einen Worker stilllegt**.
Sie sollte im Postfach des Orchestrators anders aussehen als eine Standmeldung — ein eigener
Zähler, den `rota-ipc offen` gesondert ausweist: „2 Worker warten auf Zuteilung."

**Warum:** Ich habe heute 11 überfällige Quittungen abgearbeitet und dabei nicht gesehen, dass
zwei Worker seit Stunden ohne Arbeit waren. Die Information war da, aber nicht unterscheidbar
von 40 anderen Nachrichten — dieselbe Klasse wie B.1.6c im Plan.

### Vorschlag 1c — Watcher: Stillstand ohne Zuteilung ist ein anderer Fall

**Wo:** `~/bin/rota-ipc-watcher`, `WIEDERHOLUNG_MINUTEN`.

Der Watcher unterscheidet heute nicht zwischen „Worker rechnet nicht, hat aber ein Issue" und
„Worker rechnet nicht und hat keins". Der zweite Fall ist der teure und sollte **nicht** von der
60-Minuten-Sperre gedeckt sein.

**Warum:** Ein Worker mit Issue, der zehn Minuten nichts tut, denkt vielleicht nach. Ein Worker
ohne Issue, der zehn Minuten nichts tut, wartet mit Sicherheit.

## Bremse 2: Die Bauzeit — 5,18 h, und sie ist zur Hälfte vermeidbar

Der Bau ist mit 54 % der größte Posten der Werkzeugzeit. Der Vergleich zwischen den Workern
zeigt, dass das keine Naturkonstante ist:

|                                                      | Läufe |         Summe | **Median** |   Max |
| ---------------------------------------------------- | ----: | ------------: | ---------: | ----: |
| Worker-1, `cargo remote … test --no-run` ohne Filter |    48 | **114,4 min** | **58,3 s** | 601 s |
| Worker-2, dieselbe Form                              |    53 |      47,9 min | **10,1 s** | 261 s |
| Worker-3, dieselbe Form                              |    65 |      43,6 min |  **9,6 s** | 239 s |

**Dieselbe Befehlsform, Faktor 6 im Median.** Und der Grund liegt daneben:

|          |    `--test X` (eine Suite) |    `--all-targets` |
| -------- | -------------------------: | -----------------: |
| Worker-1 |                **7 Läufe** | 56 Läufe, 55,9 min |
| Worker-2 | **36 Läufe**, Median 4,0 s |            8 Läufe |
| Worker-3 |                   13 Läufe |           15 Läufe |

Worker-2 baut **gezielt**, Worker-1 **breit**. Zwei Läufe von Worker-1 endeten im 600-Sekunden-Timeout
— 20 Minuten, die nichts geliefert haben.

### NACHGEMESSEN am 2026-08-19 — die Erklärung oben trägt nur zur Hälfte

Der Absatz darüber war eine **Vermutung mit einer Zahl daneben**, und das sieht aus wie ein
Befund. Task #14 verlangte, sie zu prüfen: Wenn die Wahl der Befehlsform die Ursache ist,
muss der Unterschied **innerhalb derselben Form** verschwinden. Alle 495 `cargo remote`-Aufrufe,
nach Form klassifiziert, Median in Sekunden:

| Form                   |        Worker-1 |        Worker-2 |       Worker-3 |          Spanne |
| ---------------------- | --------------: | --------------: | -------------: | --------------: |
| `--all-targets`        |     25,3 (130×) |      10,4 (40×) |     10,4 (56×) |      Faktor 2,4 |
| `build`                | **155,4** (39×) |       7,4 (31×) |      5,8 (29×) | **Faktor 26,9** |
| gezielt `--test`       |        4,5 (7×) |       4,0 (36×) |     19,1 (13×) |      Faktor 4,8 |
| nur bauen (`--no-run`) |        0,8 (2×) | **184,8** (15×) |     37,7 (35×) |      Faktor 245 |
| **Gesamt**             | **17,0** (198×) |  **6,5** (145×) | **8,9** (152×) |      Faktor 2,6 |

**Drei Dinge stimmen nicht mit der ersten Auswertung:**

1. **Der Gesamtfaktor ist 2,6, nicht 6.** Die 58,3 s gegen 10,1 s stammen aus einer engeren
   Klassifikation und sind mit dieser Methode nicht reproduzierbar. Eine Zahl, die man nicht
   wiederfindet, ist keine Messung — auch wenn sie von einem selbst stammt.
2. **Der Unterschied verschwindet nicht innerhalb der Form.** Bei `--all-targets` bleibt
   Faktor 2,4, bei `build` sogar 26,9. Die Formwahl erklärt einen Teil, nicht die Sache.
3. **Der größte Ausreißer gehört Worker-2**, nicht Worker-1: 184,8 s Median bei `--no-run`.
   Die erste Auswertung hatte ihn als den Schnellen geführt.

**Was nicht die Ursache ist**, ebenfalls gemessen: die Größe des Worktrees, die `cargo remote`
synct. e1 ist mit 12 GB der **kleinste** und trotzdem der langsamste; e2 hat 16 GB, e9 15 GB,
und ohne `target/` liegen alle drei bei 180–203 MB und 6 400–6 900 Dateien.

**Die Ursache bleibt offen.** Naheliegend, aber ungeprüft: Worker-1 hat in #915 drei neue
Module angelegt, und jedes neue Modul löst eine Neuübersetzung der Crate aus — das träfe
`build` und nicht `--test`. Prüfbar wäre es nur mit den Workern zusammen, an ihrem
tatsächlichen Änderungsmuster.

**Brauchbar ist trotzdem etwas**, und zwar innerhalb eines Workers statt zwischen zweien:

```
Worker-1   --all-targets 25,3 s   gegen   gezielt --test 4,5 s   Faktor 5,6
Worker-2   --all-targets 10,4 s   gegen   gezielt --test 4,0 s   Faktor 2,6
Worker-3   --all-targets 10,4 s   gegen   gezielt --test 19,1 s  Faktor 0,5 — umgekehrt
```

Wären zwei Drittel von Worker-1s 130 breiten Läufen gezielt gewesen, hätte er **30 Minuten**
gespart. Das ist die Grundlage von Maßnahme #13 — und die dritte Zeile ist der Grund, warum
sie als Voreinstellung und nicht als Pflicht in den Guide kommt.

Dazu das Verhältnis über alle drei: **598 Bau- und Testaufrufe zu 404 Code-Änderungen = 1,48
Bauläufe je Änderung.** Nach fast jeder Änderung wird gebaut.

### Vorschlag 2a — `--test <suite>` als Voreinstellung, `--all-targets` nur vor der Fertigmeldung

**Wo:** `docs/WORKER-GUIDE.md`, Abschnitt Bauen; und `docs/SIM-ARCHITEKTUR-PLAN.md` B.1.6.

**Warum:** Median 4 s gegen 58 s. Wer an einer Suite arbeitet, braucht die anderen 200 Ziele
nicht. Der vollständige Lauf gehört an genau eine Stelle — vor die Fertigmeldung, wo das Issue
ihn ohnehin verlangt („Vor der Fertigmeldung läuft genau ein Gesamtlauf").

**Was dabei nicht verlorengehen darf:** Der gezielte Bau übersieht Fehler in anderen Zielen.
Deshalb bleibt der Gesamtlauf Pflicht — nur eben einmal statt fünfzigmal.

### Vorschlag 2b — Mehrere Edits sammeln, dann einmal bauen

**Wo:** Arbeitsweise, `docs/WORKER-GUIDE.md`.

Bei 1,48 Bauläufen je Code-Änderung ist der Bau kein Prüfschritt mehr, sondern ein Reflex. Drei
zusammenhängende Änderungen und ein Bau kosten ein Drittel.

**Grenze, und sie ist echt:** Bei Rust ist ein früher Compile-Fehler billiger als ein spät
gefundener. Der Vorschlag gilt für **zusammenhängende** Änderungen an derselben Stelle, nicht
für unabhängige.

### Vorschlag 2c — Warum Worker-1 langsamer baut, messen statt annehmen

**Offene Frage, ausdrücklich nicht beantwortet.** Ich habe den Unterschied gemessen, nicht seine
Ursache. Kandidaten: Änderungen an `lib.rs` invalidieren die ganze Crate (Worker-1 hat sie
angefasst), mehr berührte Dateien je Bau (8 in #908), oder rsync-Volumen.

**Was ich vorschlage:** Worker-1 misst es einmal selbst — derselbe Befehl zweimal
hintereinander, einmal nach einer `lib.rs`-Änderung und einmal nach einer Änderung in einer
Testdatei. Zwei Zahlen, und die Ursache ist entweder gefunden oder ausgeschlossen.

## Bremse 3: Verwaltung — 18 bis 25 % der Werkzeug-Turns

|          | Code-Edits | IPC-Turns | Buchhaltung (`.md`) | Verwaltungsanteil |
| -------- | ---------: | --------: | ------------------: | ----------------: |
| Worker-1 |        145 |       126 |                  40 |            18,2 % |
| Worker-2 |        111 |       153 |                  57 |        **24,7 %** |
| Worker-3 |        148 |       129 |                  40 |            20,6 % |

Auffällig sind die Übergabe-Dateien: **17× bearbeitet** (Worker-2), **23×** (Worker-3). Dazu
`MEMORY.md` 11×.

Das IPC-Volumen selbst ist erheblich, aber nicht der Hauptposten: 88–96 Sendungen à ~2 000
Zeichen, zusammen 8,5 % der Output-Tokens.

### Vorschlag 3a — Die Übergabe wird an der Kontextgrenze geschrieben, nicht laufend

**Wo:** `docs/WORKER-GUIDE.md`, Abschnitt Übergabe.

Eine Übergabe soll den Zustand retten, wenn eine Sitzung endet. Sie 23× fortzuschreiben heißt,
sie 22× umsonst zu schreiben — der Zweck tritt nur beim letzten Mal ein.

**Warum das trotzdem heikel ist:** Wer sie erst an der Grenze schreibt, schreibt sie
vielleicht nie (Kontext plötzlich zu Ende). Der Kompromiss: eine Übergabe **je
abgeschlossenem Issue**, nicht je Erkenntnis. Das sind 3–4 statt 23.

### Vorschlag 3b — Quittungen kürzen, Befunde nicht

**Wo:** `docs/IPC.md`, Abschnitt Bestätigen.

Die Regel verlangt „das verstandene Vorhaben in einem Satz". Tatsächlich sind die Quittungen
2 000 Zeichen lang und wiederholen den Inhalt der Gegenseite. Das ist bei einem **Befund**
richtig — dort ist die Wiederholung die Prüfung, ob es verstanden wurde — und bei einer
**Zuteilung** oder **Standmeldung** Verschwendung.

**Was ich nicht vorschlage:** die inhaltlichen Meldungen zu kürzen. Die haben heute vier
Vorlagen-Fehler und einen Bestandsfehler gefunden, den niemand gesucht hat. Das ist der
Gegenwert.

## Bremse 4: Kleinigkeiten mit messbarem Volumen

**`cd` in 86 % der Bash-Aufrufe.** Worker-1: 558× `cd /work/rota-worktrees/e1`, Worker-2 286×,
Worker-3 181×. **Gemessen: das Arbeitsverzeichnis persistiert zwischen zwei Bash-Aufrufen** —
zwei aufeinanderfolgende `pwd` liefern denselben Pfad ohne `cd`. Die Zeile ist reiner Text,
kostet also Tokens und keine Sekunden, aber sie steht 1 025× in den Protokollen.

**`Read` wird praktisch nicht benutzt:** 35 Aufrufe gegen 1 913 Bash. Worker-3 hat es **einmal**
verwendet, Worker-2 dreimal. Gelesen wird über `grep`/`sed`/`cat`.

Das ist **kein** Fehler: Bei 5 000-Zeilen-Dateien ist ein `grep -n` gezielter als ein
Vollzugriff, und die Worker haben null Dateien mehrfach gelesen — sie arbeiten also nicht
ineffizient, sondern anders als die Werkzeugempfehlung vorsieht. Erwähnt, weil es die
Bash-Dominanz erklärt und ich es sonst als Befund missverstanden hätte.

## Was NICHT bremst — messbar

Damit die Vorschläge nicht an der falschen Stelle ansetzen:

| Vermutung                              | Messung                                                                              |
| -------------------------------------- | ------------------------------------------------------------------------------------ |
| Die Worker machen viele Werkzeugfehler | **0,8–1,0 %.** 22 Fehler auf 2 564 Aufrufe                                           |
| Sie lesen dieselben Dateien mehrfach   | **0 Dateien** bei Worker-2 und Worker-3                                              |
| Sie wiederholen Befehle                | außer `cd` und Bau-Aufrufen kaum; die „Wiederholungen" sind Bauläufe nach Änderungen |
| Kontextverluste kosten Zeit            | **1 Compaction** bei Worker-3 im ganzen Zeitraum                                     |
| Die Testläufe sind der Engpass         | 1,21 h gegen 5,18 h Bau                                                              |

## Rechnung: was die Vorschläge wert wären

**Konservativ, mit benannten Annahmen:**

```
Leerlauf heute                                33,9 h
davon in zwei Nacht-Blöcken                   21,9 h
  -> mit Zuteilungsvorrat vermeidbar          ~18 h    (Annahme: der Vorrat trägt
                                                        vier von fünf Fällen)

Bauzeit heute                                  5,18 h
  Worker-1 auf den Median der anderen           -2,1 h  (114 min -> 20 min bei
                                                         gezieltem Bau)
  1,48 -> 1,0 Bauläufe je Änderung              -1,0 h

Verwaltung: Übergabe 23x -> 4x                  -0,4 h  (geschätzt, nicht gemessen)
                                             ────────
                                              ~21,5 h
```

**21,5 h von 64 verfügbaren Worker-Stunden** — bei 11 Issues in diesem Zeitraum entspricht das
grob **vier bis fünf weiteren Issues**, ohne dass jemand schneller arbeiten müsste.

> **Was diese Rechnung nicht ist:** eine Zusage. Die 18 h beim Leerlauf hängen an einer Annahme
> über die Trefferquote des Vorrats, und die 0,4 h bei der Übergabe sind geschätzt und nicht
> gemessen. Belastbar sind die **gemessenen** Ausgangswerte: 33,9 h Leerlauf, 5,18 h Bau, Faktor
> 6 im Bau-Median zwischen zwei Workern.

## Rangfolge der Umsetzung

1. **Zuteilungsvorrat** (Vorschlag 1a) — größter Hebel, kostet nichts, sofort machbar
2. **Fertigmeldungen im Postfach unterscheidbar** (1b) — verhindert die Wiederholung
3. **`--test <suite>` als Voreinstellung** (2a) — zweitgrößter Hebel, ein Absatz im Guide
4. **Watcher: Stillstand ohne Zuteilung** (1c) — Aufsicht dort schärfen, wo es teuer ist
5. **Edits sammeln** (2b) — Arbeitsweise, wirkt langsam
6. **Übergabe je Issue statt je Erkenntnis** (3a)
7. Worker-1s Bauzeit-Ursache messen (2c) — offene Frage, keine Maßnahme
