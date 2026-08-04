---
id: ROTA-WORKER-GUIDE
status: Stable
ssot: true
refs:
  docs: [GAME-DESIGN, STEP1-SPEC, VERIFICATION]
---

# Worker-Guide — verbindlicher Ablauf für Dev-Sessions

> **Für jede Session, die ein Epic umsetzt.** Was hier steht, ist nicht optional.
> Die Master-Session prüft dagegen, und ein Issue, das den Ablauf nicht eingehalten hat, wird
> zurückgegeben — unabhängig davon, wie gut der Code ist.

## Das Wichtigste in fünf Sätzen

1. Du setzt **ein Epic** um, in **einem Worktree**, mit **einem** Feature-Branch zur Zeit.
2. **`docs/GAME-DESIGN.md` ist die Wahrheit.** Bei Widerspruch zum Code gewinnt das Dokument.
3. Bei einer Lücke im Design: **fragen, nicht raten.**
4. Jede Acceptance Criteria wird mit **Befehl UND Ausgabe** belegt. „Code gelesen" zählt nicht.
5. Ein Issue ist **100 % fertig oder offen mit Blocker.** Nichts dazwischen.

---

## 1. Bevor du das erste Mal anfängst

Lies in dieser Reihenfolge:

| #   | Dokument              | Warum                                                                                    |
| --- | --------------------- | ---------------------------------------------------------------------------------------- |
| 1   | `CLAUDE.md` (Projekt) | Die sechs Design-Gesetze und die technischen Rahmenbedingungen                           |
| 2   | `docs/GAME-DESIGN.md` | Das komplette Design. **Kapitel 21 besonders** — dort steht, was bereits verworfen wurde |
| 3   | `docs/STEP1-SPEC.md`  | Nur wenn du an E1 arbeitest: die konkreten Zahlen                                        |
| 4   | dieses Dokument       | Der Ablauf                                                                               |

> **Kapitel 21 von GAME-DESIGN.md ist Pflichtlektüre.** Jede Zeile dort war eine bewusste Entscheidung
> des Auftraggebers. Wenn dir beim Bauen eine „bessere Idee" kommt, steht sie mit hoher
> Wahrscheinlichkeit schon dort — mitsamt der Begründung, warum sie verworfen wurde.

---

## 2. Struktur

- **Ein Epic = ein Worktree = eine Session.** Die Epics sind so geschnitten, dass sie voneinander
  unabhängig sind. Keine Session wartet auf eine andere.
- **Ein Issue = ein Feature-Branch** von `main`.
- **Immer nur EIN Feature-Branch je Worktree aktiv.** Kein paralleles Arbeiten an zwei Issues.

### Worktree-Zuordnung

| Epic                        | Worktree                  | Ruhe-Branch | Arbeitet vor allem in                    |
| --------------------------- | ------------------------- | ----------- | ---------------------------------------- |
| **E1** — Kampf-Kern und HUD | `/work/rota-worktrees/e1` | `wt-e1`     | `src/core/`, `src/render/`, `src/input/` |
| **E8** — Klang              | `/work/rota-worktrees/e8` | `wt-e8`     | `src/audio/`, `assets/audio/`            |
| **E9** — Grafik-Assets      | `/work/rota-worktrees/e9` | `wt-e9`     | `assets/img/`, `ASSETS.md`               |

Weitere Worktrees entstehen, wenn ihr Epic an die Reihe kommt.

Die Worktrees liegen bewusst **außerhalb** von `/work/rota` — lägen sie darin, zöge Vite sie als
Quellen in den Build.

> **Halte dich an deinen Dateibereich.** Die Aufteilung existiert, damit fünf Sessions gleichzeitig
> arbeiten können, ohne sich über Merge-Konflikte zu blockieren. Wenn du etwas außerhalb deines
> Bereichs ändern musst, ist das ein Hinweis darauf, dass der Schnitt nicht stimmt — dann melden,
> nicht einfach machen.

---

## 3. Der Ablauf je Issue — acht Schritte, verbindlich in dieser Reihenfolge

### Schritt 1 — Worktree auf den Stand von `main` bringen

```bash
cd /work/rota-worktrees/<dein-epic>
git fetch origin
git switch wt-<dein-epic>          # der Ruhe-Branch des Worktrees
git reset --hard origin/main
```

> **Warum nicht `git checkout main`:** `main` ist bereits im Hauptverzeichnis `/work/rota`
> ausgecheckt, und git lässt denselben Branch nicht zweimal auschecken. Deshalb hat jeder Worktree
> einen eigenen Ruhe-Branch (`wt-e1`, `wt-e8`, `wt-e9`), der per `reset --hard` auf den aktuellen
> `origin/main` gesetzt wird. Auf diesem Branch wird **nie gearbeitet** — er ist nur der
> Ausgangspunkt.

> **Nicht überspringen.** `main` kann weitergewandert sein, weil eine andere Session inzwischen einen
> PR gemerged hat. Wer auf einem alten Stand entwickelt, baut Konflikte ein, die niemand braucht.

### Schritt 2 — Feature-Branch anlegen

```bash
git checkout -b <typ>/<epic>-<nummer>-<kurzbeschreibung>
# Beispiel: feat/e1-3-spell-model
```

### Schritt 3 — Umsetzen und committen

- Kleine, nachvollziehbare Commits. Conventional Commits (`feat:`, `fix:`, `docs:`, `chore:`).
- **Nach jeder Code-Änderung an `.ts`/`.css`: Build oder Test als nächster Schritt.** Kein Bericht
  an irgendwen zwischen Änderung und Verifikation.
- Keine Stubs, keine Mocks, keine `TODO: später`. Was im Issue steht, wird fertig gebaut.

### Schritt 4 — Pull Request öffnen

```bash
git push -u origin <branch>
gh pr create --fill
```

Der PR-Text nennt: was gebaut wurde, welches Issue er schließt, und die AC-Nachweise (siehe § 5).

### Schritt 5 — Alle GitHub-Actions müssen grün sein

```bash
gh pr checks --watch
```

**`ci-pass` ist der einzige Required Check** — er wartet auf alle anderen Jobs. Ist er grün, ist alles
grün. Ist er rot, **den Code reparieren, nicht den Test.**

> **Kein Anpassen von Schwellwerten, Tests oder Konfiguration, damit etwas durchgeht.** Die einzige
> Ausnahme ist, wenn das Issue ausdrücklich sagt, dass ein Schwellwert falsch gesetzt war.

#### Grün belegt, dass nichts kaputt ist — nicht, dass etwas da ist

_Gemessen am 2026-08-04, beinahe mit Datenverlust._

Ein Zweig, dem die eigentliche Arbeit fehlt, ist **genauso grün** wie einer, der sie enthält. Die CI
prüft, ob der vorhandene Code funktioniert, nicht ob der Code vorhanden ist.

> Beim Mergen von #115 wurde der Push abgelehnt (`non-fast-forward`). Der lokale Zweig hatte die
> eigene Arbeit **nicht mehr** — ein zwischenzeitliches `git checkout -B` hatte ihn auf `main`
> zurückgesetzt. Der Remote war die einzige vollständige Kopie.
>
> Ein `git push --force` hätte sie gelöscht, und der danach gemergte PR wäre grün und leer gewesen.

**`non-fast-forward` ist kein Ärgernis, das man wegdrückt, sondern eine Meldung über einen Zustand,
den man nicht kennt.** Bevor du forcierst — was ohnehin die Ausnahme sein sollte —, sieh nach, was
auf beiden Seiten steht:

```bash
git log --oneline origin/<zweig>..HEAD        # was hat der lokale Zweig, was der Remote nicht hat?
git log --oneline HEAD..origin/<zweig>        # und umgekehrt — die wichtigere Richtung
git show origin/<zweig>:<eine deiner Dateien> # liegt deine Arbeit dort noch?
```

Dasselbe gilt für die Abnahme: Ein grüner PR ist eine Voraussetzung, kein Nachweis. Der Nachweis ist,
dass die Prüffälle die neue Mechanik **wirklich berühren** — bei E3-4 war die Parität grün, während
die Schutzschicht auf der Spielseite gar nicht eingebaut war, weil in allen bestehenden Fällen der
Schutz null ist.

### Schritt 6 — Merge nach `main`

Merge-Commit (kein Squash, kein Rebase — die Historie bleibt linear und vollständig).

### Schritt 7 — Verifikation durch die Master-Session

Du meldest dein Ergebnis im Format aus § 6. Die Master-Session prüft **selbst im laufenden Spiel**
gegen `GAME-DESIGN.md`. Deine Behauptungen zählen dabei nicht als Nachweis — sie werden nachgemessen.

### Schritt 8 — Erst jetzt: Issue schließen, Branch löschen

```bash
gh issue close <nummer>
git branch -d <branch>
git push origin --delete <branch>
```

> **Die Reihenfolge ist zwingend.** Kein Schließen vor der Verifikation, kein Löschen vor dem Merge.

---

## 4. Definition of Done

Ein Issue ist fertig, wenn **alle fünf** Punkte zutreffen:

1. **Jede AC einzeln geprüft** — keine übersprungen, keine gebündelt
2. **Für jede AC: ausgeführter Befehl UND tatsächliche Ausgabe** dokumentiert
3. **Im laufenden System getestet**, nicht nur in Unit-Tests
4. **Alle Actions grün**, PR gemerged
5. **Von der Master-Session abgenommen**

Fehlt einer davon, bleibt das Issue offen.

```
KEIN "größtenteils fertig"
KEIN "die wichtigsten ACs sind erfüllt"
KEIN "der Rest ist trivial"
```

Ein Issue, das an einer AC scheitert, wird **als offen mit Blocker gemeldet** — mit der Angabe,
welche AC und warum. Das ist ein völlig normaler Ausgang und besser als eine geschönte Meldung.

---

## 5. Was als Nachweis gilt — und was nicht

| Gilt als Nachweis                    | Gilt NICHT                        |
| ------------------------------------ | --------------------------------- |
| Befehl ausgeführt, Ausgabe gezeigt   | „Code gelesen, sieht korrekt aus" |
| Screenshot aus dem laufenden Spiel   | Zeilennummern zitiert             |
| Messung (Latenz, Bildzeit, Speicher) | „Struktur ist vorhanden"          |
| Testlauf mit konkretem Ergebnis      | „Pattern implementiert"           |
| Konsolen-Ausgabe der Dev-Werkzeuge   | Quelltext-Review ohne Ausführung  |

**Grundzustand jeder AC ist UNGEPRÜFT.** Kein Befehl = ungeprüft. Ungeprüft ist nicht bestanden.

### Die Messlatte muss den Fall enthalten, in dem sich etwas NICHT ändern darf

_Formuliert von E3-3b am 2026-08-04, als Verallgemeinerung mehrerer Funde._

Ein Prüffall, der nur zeigt, dass die neue Mechanik **etwas tut**, belegt selten das Richtige. Was
er zeigen muss, ist der Unterschied — und dazu gehört die Seite, die **gleich bleiben soll**:

| Nachweis                                      | Ohne die Gegenseite hätte auch bestanden …                                                  |
| --------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Drei Verteidigungs-Wege wirken verschieden    | … dass alle drei wirkungslos sind — es fehlte der Lauf **ohne** Verteidigung                |
| Ein Schlag lässt sich verzögern               | … dass die **ganze** Zeitleiste rückt — es fehlte der zweite Schlag, der stehen bleibt      |
| Eine Phase lässt sich hinauszögern            | … dass sie danach **zurückfällt** — es fehlte der Lauf, in dem die Wirkung ausläuft         |
| Ein Unterbrecher ohne Gelegenheit zählt nicht | … dass Unterbrecher **grundsätzlich** ignoriert werden — es fehlte der Fall mit Gelegenheit |
| Ausrüstung verändert den Schaden              | … dass ein anderer Faktor die Arbeit tut — es fehlte der neutrale Lauf                      |

**Die Frage vor jedem Prüffall lautet deshalb nicht nur „was soll sich ändern", sondern auch: was
darf sich dabei NICHT ändern — und steht das im selben Lauf?**

Ein verwandter Fall: Wenn die neue Größe im Prüflauf überall **null** ist, kann er nichts zeigen.
Bei E3-4 war die Parität grün, während die Schutzschicht auf einer Seite fehlte — in allen
bestehenden Fällen war der Schutz null.

### Formulierungen, die ohne Nachweis nicht vorkommen dürfen

```
"funktioniert"            "production ready"        "erfolgreich implementiert"
"alle Tests bestanden"    "verifiziert"             "erledigt" / "fertig"
"läuft einwandfrei"       "integriert"              "100 % done"
```

Erlaubt und erwünscht ist stattdessen die ehrliche Einschätzung:

```
"Unit-Tests grün, Integration ungetestet (75 % Zuversicht)"
"Isoliert geprüft, im Gesamtsystem noch nicht (60 %)"
```

---

## 6. Melde-Format

Das schickst du am Ende jedes Issues an den Orchestrator:

```bash
rota-ipc senden Orchestrator - <<'ENDE'
<deine Meldung>
ENDE
```

**Nicht mehr in den Chat kopieren.** Der Auftraggeber war früher der Transportweg; seit dem
2026-07-28 schreiben die Sitzungen einander direkt. Einzelheiten in [`IPC.md`](IPC.md).

**Jede Nachricht, die du bekommst, bestätigst du sofort** — mit dem verstandenen Vorhaben in
einem Satz, nicht nur „erhalten":

```bash
rota-ipc antworten <Nummer> "Beginne mit Issue #58, erwarte etwa zwei Stunden."
```

So zeigt sich ein Missverständnis sofort statt Stunden später. Dein Ergebnis kommt danach als
**neue** Nachricht.

Klemmt der Versand: **melde das, weiche nicht aus.** Keine GitHub-Kommentare, kein anderer Weg.

```markdown
## Issue #<nummer> — <Titel> [FERTIG | OFFEN MIT BLOCKER]

### Was gebaut wurde

- <Stichpunkte, 2–5 Zeilen>
- Geänderte Dateien: <Liste>
- Commits: <hashes>

### AC-Nachweise

AC1: <AC-Text> → BESTANDEN
$ <exakter Befehl>
<tatsächliche Ausgabe, gekürzt auf das Aussagekräftige>

AC2: <AC-Text> → BESTANDEN
$ <exakter Befehl>
<Ausgabe>

AC3: <AC-Text> → NICHT BESTANDEN
$ <Befehl>
<Ausgabe>
Grund: <warum> Blockiert: <ja/nein, was hängt daran>

### Was NICHT geprüft wurde

- <Punkt> — Grund: <warum nicht>

### Zuversicht

<Prozent> — <ein Satz Begründung>
```

> **Der Abschnitt „Was NICHT geprüft wurde" darf nicht leer bleiben, nur weil es unangenehm aussieht.**
> Eine ehrliche Lücke ist wertvoll; eine verschwiegene Lücke kostet die Master-Session eine Stunde
> Suche und dich das Vertrauen.

---

## 7. Bei Lücken im Design

`GAME-DESIGN.md` Kapitel 22 listet auf, was **bewusst noch nicht ausgeplant** ist. Wenn du auf einen
dieser Punkte stößt:

1. **Nicht selbst entscheiden.** Auch nicht „vorläufig".
2. Melden: welcher Punkt, was genau fehlt, welche Möglichkeiten du siehst.
3. Wenn möglich: alles andere im Issue fertigstellen und nur diesen Teil offenlassen.

Dasselbe gilt für Widersprüche zwischen Dokumenten. **Ein Widerspruch ist ein Fund, kein Hindernis** —
melden, dann wird er in der SSOT aufgelöst.

---

## 8. Technische Rahmenbedingungen (aus `CLAUDE.md`)

| Punkt                 | Regel                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Dev-Server**        | nginx auf `localhost:8000/rota/`. **Niemals eigene HTTP-Server starten**                                                        |
| **Browser-Tests**     | `playwright-cli -s=rota …`, Session am Ende mit `close` beenden. `--headed` nur, wenn zugeschaut werden soll                    |
| **Kein UI-Framework** | Reines TypeScript. Kein React, kein Svelte                                                                                      |
| **Spieltexte**        | Nur Englisch. Kommunikation auf Deutsch                                                                                         |
| **Umlaute**           | In Doku, Kommentaren und Commit-Messages echte Umlaute (ä ö ü ß), nicht ae/oe/ue/ss. ASCII nur in Bezeichnern, Dateinamen, URLs |
| **Keine Emojis**      | Weder im Code noch in der Oberfläche noch in Commits                                                                            |

---

## 9. Wie die Kommunikation läuft

```
    Worker-Session                Auftraggeber              Master-Session
         │                             │                          │
         │  Melde-Format (§ 6)         │                          │
         ├────────────────────────────►│  kopiert                 │
         │                             ├─────────────────────────►│
         │                             │                          │ prüft selbst
         │                             │                          │ im laufenden
         │                             │   ABGENOMMEN oder        │ System
         │                             │◄─────────────────────────┤
         │  Rückmeldung                │   OFFEN MIT BLOCKER      │
         │◄────────────────────────────┤                          │
```

Der Auftraggeber kopiert die Ausgaben zwischen den Sessions hin und her. **Fasse dich deshalb
kompakt genug, dass die Meldung in einen Kopiervorgang passt** — aber niemals auf Kosten der
Nachweise. Build-Ausgaben, Testergebnisse und Messwerte sind nie „zu ausführlich".
