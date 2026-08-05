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

#### `[new branch]` beim Push ist dieselbe Meldung

_Am 2026-08-04 abends, gefunden von E7._

Er baute auf Anweisung an einer bereits gemergten Änderung weiter. Währenddessen wurde der PR
gemergt und der Zweig gelöscht. Sein `push` legte ihn **neu an**, statt ihn zu aktualisieren — und
sagte das auch:

```
 * [new branch]      fix/... -> fix/...
```

Erwartet hatte er eine Aktualisierung. Er hat nachgesehen statt weiterzumachen und fand: PR gemergt,
alter Zweig weg, sein Commit allein auf einem neuen.

> **Ein Push, der einen Zweig NEU anlegt statt ihn zu aktualisieren, ist eine Meldung über einen
> Zustand, den du nicht kennst** — genauso wie `non-fast-forward`. Beide Male sagt git etwas über
> die Gegenseite, das du nicht erwartet hast.

Der Schaden entsteht erst, wenn man darüber hinwegliest: Der Commit liegt dann auf einem Zweig, den
niemand kennt und kein PR verfolgt.

#### Vor jedem PR: steht im Diff nur, was du angefasst hast?

```bash
git diff --stat origin/main..HEAD
```

Zwei Sekunden. Was dort auftaucht und nicht von dir stammt, ist fremde Arbeit, die dein Zweig
**zurückdreht** — meist, weil sie nach deinem Zweigstart entstanden ist und du `main` nicht
nachgezogen hast.

> **Gemessen am 2026-08-04.** Ein PR, der eine einzige Testdatei ändern sollte, hätte nebenbei
> 22 Zeilen aus `docs/WORKER-GUIDE.md` gelöscht:
>
> ```
> docs/WORKER-GUIDE.md    | 22 ----------------------
> tests/unit/loop.test.ts |  9 ++++++++-
> ```
>
> **Die CI hat es nicht gefangen** — ein PR mit weniger Dokumentation ist genauso grün wie einer mit
> mehr. Gefunden hat es nur der Blick auf den Diff.

Der Fall ist derselbe wie oben, nur an einer Datei, die niemand als „Arbeit" ansieht: Grün belegt,
dass nichts kaputt ist, nicht dass etwas da ist.

#### Steht statt einer Zeilenzahl `Bin` da, ist die Prüfung für diese Datei blind

_Gemessen am 2026-08-04 an einer Quelldatei, die es nicht sein sollte._

```
src/analysis/progress-mark.ts    | Bin 6480 -> 8114 bytes
                                   ^^^
tests/unit/progress-mark.test.ts |  51 +++++++++++++++++
```

**Ein einziges NUL-Byte** in einem Template-String genügt — Git behandelt die ganze Datei als binär.
Die Ursache war ein Trennzeichen für einen zusammengesetzten Schlüssel, technisch sogar die richtige
Wahl. Nur als **echtes Byte** geschrieben statt als Escape-Sequenz.

| Was ausfällt                            |                                              |
| --------------------------------------- | -------------------------------------------- |
| `git diff`                              | zeigt den Inhalt nicht, nur die Byte-Zahl    |
| `git diff --stat`                       | **genau die Prüfung aus dem Abschnitt oben** |
| Merge-Konflikte                         | nicht auflösbar                              |
| Durchsicht im PR                        | GitHub zeigt „Binary file not shown"         |
| `git blame`, `git log -p`, `git bisect` | hören an dieser Datei auf zu arbeiten        |

> Der Bearbeiter hatte an dem Tag fünfmal „1725 Zeilen hinzu, 4 entfernt" gemeldet — für diese Datei
> stand dort nie eine Zeile. **Ein Zweig, der sie zurückdreht, hätte in seiner Prüfung genauso
> ausgesehen wie einer, der sie ergänzt.**

**Zwei Dinge folgen daraus:**

1. **Ein `Bin` im `--stat` ist kein Detail, sondern ein Halt.** Bei Quelldateien immer.
2. **Wer den Fehler behebt, fasst den Commit mit dem Vorgänger zusammen** — sonst bleibt ein binärer
   Stand in der Historie stehen, und `blame` und `bisect` stolpern weiter darüber.

Ein Prüffall wacht seitdem darüber (`tests/unit/source-is-text.test.ts`): Er lädt alle `.ts` und
`.css` aus `src/` und `docs/` und schlägt bei jedem NUL-Byte an — **einschließlich der Absicherung,
dass er überhaupt Dateien findet.** Ohne sie liefe er auch bei leerer Liste durch.

##### Nach NUL-Bytes suchen: grep taugt dafür nicht

_Zwei Sitzungen, zwei verschiedene Fehlschläge, dieselbe Ursache — am selben Tag._

| Versuch                 | Ergebnis                                           | Was wirklich geschah                                                                                               |
| ----------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `grep -qP '\x00' datei` | findet **nichts**, auch nicht die betroffene Datei | grep behandelt die Datei als binär und meldet nur „Binary file matches" — mit `-q` verschluckt                     |
| `grep -c $'\x00' datei` | meldet **210**                                     | Die Shell kann kein NUL in einer Variable halten. `$'\x00'` wird zum **leeren Muster**, und grep zählt alle Zeilen |

Der zweite Fall ist der tückischere: Die Zahl sieht wie ein Ergebnis aus. 210 Zeilen und 210
angebliche NUL-Bytes sind nicht zu unterscheiden, wenn man die Datei nicht kennt.

**Was trägt:**

```bash
# Ist die Datei Text?
grep -qI . datei && echo TEXT || echo BINAER

# Wie viele NUL-Bytes?
A=$(wc -c < datei); B=$(tr -d '\000' < datei | wc -c); echo $((A - B))
```

Beides ohne Muster-Interpretation — deshalb funktionieren sie.

### Schritt 6 — Merge nach `main`

Merge-Commit (kein Squash, kein Rebase — die Historie bleibt linear und vollständig).

#### Eine Zusage mit Bedingung braucht einen Ort

_Am 2026-08-04 lag ein abgenommener PR **acht Stunden** offen. Der Autor hätte die Arbeit fast ein
zweites Mal gebaut._

Die Abnahme war erfolgt, und die Merge-Ankündigung war an eine Bedingung geknüpft: „sobald ich den
anderen PR durch habe". Der andere PR ging durch — und die Zusage blieb am Gedächtnis dessen
hängen, der sie gegeben hatte. Der war längst beim nächsten Auftrag.

> **Wer eine Bedingung setzt, muss sie auflösen.** Eine Zusage, die nur im Kopf steht, hat keinen
> Ort, an dem jemand sie findet.

**Beide Seiten haben eine Hälfte:**

- **Wer wartet:** Ein PR, auf den du wartest, gehört auf **deine** Liste, nicht auf die des anderen.
  Prüfe beim Anfangen eines Issues, ob dazu noch ein offener PR von dir existiert.
- **Wer zusagt:** Keine Ankündigung mit „sobald…". Entweder sofort, oder mit einem Zeitpunkt, an
  dem der andere nachfragen darf.

> Am selben Tag lag ein zweiter Fall daneben: Ein PR war seit sechs Stunden gemergt, das zugehörige
> Issue offen. **Merge und Schließen sind zwei Schritte** — der Ablauf oben nennt sie getrennt, weil
> sie es sind.

### Schritt 7 — Verifikation durch die Master-Session

Du meldest dein Ergebnis im Format aus § 6. Die Master-Session prüft **selbst im laufenden Spiel**
gegen `GAME-DESIGN.md`. Deine Behauptungen zählen dabei nicht als Nachweis — sie werden nachgemessen.

#### Warten ist kein Zustand, in dem man bleibt

_Am 2026-08-04 saßen zwei Sitzungen 45 Minuten still, jede aus einem eigenen Grund. Beide hatten
Arbeit._

| Sitzung | Wartete auf                          | Warum das falsch war                                                                                                        |
| ------- | ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| A       | die Abnahme des gerade gemeldeten PR | Das **nächste** Issue war längst zugewiesen und hing inhaltlich nicht daran                                                 |
| B       | die Antwort auf eine Design-Frage    | Sie betraf **einen** Teilaspekt; der Rest des Issues war unabhängig — was sie selbst angekündigt und dann nicht getan hatte |

**Eine Abnahme ist kein Startsignal für das nächste Issue.** Sie schließt das vorige ab. Wenn im
Issue-Bestand etwas auf dich wartet, fang damit an — der Orchestrator holt dich nicht ab.

**Eine offene Frage blockiert selten alles.** Trenn beim Melden ausdrücklich, was daran hängt und was
nicht, und bau dann den unabhängigen Teil. Wer schreibt „ich arbeite derweil weiter" und es nicht
tut, hat zweimal falsch gemeldet.

> **Die Voreinstellung ist Arbeiten, nicht Warten.** Wer wirklich nichts hat, sagt das — ausdrücklich
> und mit der Frage, was als Nächstes kommt. Stille sieht von außen genauso aus wie Beschäftigung.

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

**Daraus folgt eine Regel, die sich sofort anwenden lässt: Wo du eine _Wirkung_ nachweist, prüfe
auf eine Ungleichung, nicht auf eine Zahl.**

| Formulierung                     | Fängt den Nullfall |
| -------------------------------- | ------------------ |
| `expect(mit).toBeLessThan(ohne)` | **ja**             |
| `expect(mit).toBe(1234)`         | nein               |
| `expect(mit).toBe(ohne * 0.5)`   | nein — `0 === 0`   |

> **Gemessen am 2026-08-04 in E7-4.** Zwei Läufe sollten zeigen, dass ein Widerstand den Schaden
> halbiert. Beide ergaben **null Schaden** — der Prüfaufbau startete den Countdown nicht, es wurde
> nie gewirkt. Die Ungleichung schlug fehl (`expected 0 to be less than 0`) und deckte den kaputten
> Aufbau auf. Eine Gleichheitsprüfung wäre grün gewesen und hätte nichts belegt.

Dieselbe Falle eine Ebene tiefer: Nicht die Größe ist null, sondern **die Handlung findet gar nicht
statt.** In E3-3b sollte ein Zauber zu einem festen Zeitpunkt gedrückt werden — lief dort gerade
eine Wartezeit, kam der Druck nicht durch, und der Nachweis war still weg, ohne dass der Prüffall
rot wurde. Gelöst durch ein **Zeitfenster** statt eines Zeitpunkts.

### Die Messung selbst kann kaputt sein — nicht nur der Prüffall

_Zehn Fundstellen, 2026-08-04 und 2026-08-05, in allen vier Rollen — auch beim Orchestrator._

Die Abschnitte oben behandeln Prüffälle, die zu wenig zeigen. Das hier ist eine andere Klasse:
**Der Prüffall ist richtig, aber das Messinstrument liefert eine Zahl, die nichts mit der Frage zu
tun hat.** Er wird dann nicht rot — er wird grün und bedeutet nichts.

| Was gemessen wurde                                              | Warum es nichts belegte                                                                                                                                                                    |
| --------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `!!document.querySelector('[class*=rota-loot]')`                | `skipLoot()` setzt `hidden`, das Element **bleibt im DOM** — der Ausdruck ist immer `true`                                                                                                 |
| `cargo test 2>&1 \| tail -40`                                   | Die letzten 40 Zeilen sind vier Sammlungen von 27. `0 FAILED` stimmt und beweist nichts                                                                                                    |
| `npx tsc --noEmit; echo "Exit=$?" && git commit …`              | Das `echo` gelingt immer. Der Rückgabewert wurde **angezeigt**, nicht ausgewertet                                                                                                          |
| `window.__rota === undefined` als Beleg für den Bau             | Die Seite lud aus einem fremden Worktree — das erwartete Ergebnis aus der falschen Quelle                                                                                                  |
| Ein Skript von Hand getestet, dann als Cron-Job eingetragen     | Cron bringt kein `PATH` mit `~/bin` mit. **30 Läufe, 30 Abstürze, kein einziger erfolgreicher** — und niemand sah es, weil der Job still starb                                             |
| `grep -c "muster" datei.min.js`                                 | Minifiziertes JS ist **eine** Zeile. `grep -c` zählt Zeilen: `1` heißt „kommt vor", nicht „kommt einmal vor"                                                                               |
| `grep "doom" hud.ts \| sed -n '346,400p'`                       | Die Zeilenspanne reichte über das Funktionsende hinaus — die Treffer standen im **Aufrufer**                                                                                               |
| `git grep "vulnerability" origin/main` → Treffer                | Beide Treffer waren **Kommentare** — einer verglich mit Diablo, einer erklärte eine andere Methode. Das Feld gab es nicht. Der Orchestrator meldete es einem Worker als vorhanden          |
| `SPELL_IDS = CALCINATION.value.spells.map(…)` im Klang-Prüffall | Der Fall bindet **eine** Schule. Bei 13 Zaubern prüft er 3 — VAPOUR und REFLUX kamen nicht durch, weil sie Klänge haben, sondern weil sie nie geprüft wurden                               |
| `quelle.indexOf('setDamageFactor(')` als Anker auf die Methode  | `indexOf` findet das **erste** Vorkommen. Solange niemand die Methode aufruft, ist das die Definition — beim ersten Aufrufer der Aufruf. Der Wächter ging beim ersten echten Nutzer kaputt |

> **Die drei letzten teilen eine Wurzel: Eine Prüfung, die grün meldet, weil sie den Fall gar nicht
> erreicht.** Sie ist gefährlicher als eine rote — eine rote wird untersucht, eine grüne geglaubt.
>
> Der Prüffall `crit-factors-source.test.ts` benennt die Grenze in seinem eigenen Kopf:
> „Sie prüft Text, nicht Bedeutung." Das gilt für jeden `grep` genauso, und der Orchestrator hat es
> am selben Abend selbst missachtet, an dem er es zitierte.
>
> **Die Gegenfrage, die alle drei gefangen hätte:** Nicht „ist es grün?", sondern **„wäre es rot,
> wenn der Fehler da wäre?"** Wer einen Wächter baut, baut die Gegenprobe mit: den Fehler künstlich
> herstellen und sehen, ob er anschlägt. Beim `indexOf`-Fall heißt das konkret — einen Aufruf vor
> die Definition setzen und prüfen, dass der Wächter **grün bleibt**.

> **Und die Gegenprobe selbst ist der nächste blinde Fleck.** In der Nacht zum 2026-08-05 sind
> sieben Gegenproben durchgelaufen, ohne etwas zu prüfen — dreimal beim Orchestrator, viermal
> bei den Werkern. Jede war grün, keine war verdächtig.
>
> | Was die Gegenprobe anfasste                          | Warum sie nichts belegte                                    |
> | ---------------------------------------------------- | ----------------------------------------------------------- |
> | `arm()` innerhalb des Zweigs verschoben              | Dort folgenlos — `cast()` liest das Feld nicht              |
> | Künstlicher Aufruf **hinter** die Definition gesetzt | Der Fehler wirkt nur davor                                  |
> | Eine von zwei `println!`-Zeilen entfernt             | Der Begriff stand noch in der anderen                       |
> | Syntaxfehler in `examples/`, Bau lief durch          | `cargo test --test X` baut nur diesen Test                  |
> | `cooldownReductionMs` auf 0 gesetzt                  | Der Prüffall liest die **Aufzeichnung**, nicht die Rechnung |
>
> Der Satz, der es zusammenfasst, stammt von dem Werker, dem es dreimal in einer Stunde passierte:
>
> **„Dass eine Gegenprobe durchläuft, sagt nichts darüber, ob sie das Richtige angefasst hat."**
>
> Praktisch heißt das: Bevor die Gegenprobe zählt, muss belegt sein, dass sie überhaupt ankommt —
> ein `grep` auf die eingebaute Änderung, ein absichtlicher Kompilierfehler an der richtigen
> Stelle, ein Begriff, der nur einmal vorkommt.

### Richtig bei der Entstehung heißt nicht dauerhaft richtig

_Gefunden am 2026-08-05 beim Browser-Nachweis zu #248, AC6._

Nach einem gewonnenen Kampf stand im Player Home weiterhin `0 / 200`. Der Bereich entsteht
einmal beim Start und wurde nie neu gebaut — die Zahl war richtig, **als sie entstand**.

```
1048 Prüffälle grün, während der Bildschirm eine falsche Zahl zeigte
```

Prüffälle laufen ohne DOM. Sie messen die **Rechnung** — ob das Ergebnis auch ankommt, nachdem
sich etwas geändert hat, messen sie nicht.

> Das ist derselbe Fehler wie ein **gespeicherter statt gerechneter** Wert, nur eine Ebene
> weiter außen. Im Spielstand ist eine veraltete Zahl persistent und fällt irgendwann auf. Im
> DOM ist sie flüchtig — und genau deshalb fängt sie kein Wächter.

**Die Frage bei jeder Anzeige:** Was passiert mit ihr, wenn sich der Wert dahinter ändert? Wird
sie neu gebaut, oder steht sie noch da, wo sie beim Aufbau hingeschrieben wurde?

Der Nachweis dafür braucht **zwei** Zustände im laufenden Bild: einmal vor der Änderung, einmal
danach. Ein Screenshot nach dem Aufbau belegt nur, dass der Anfangswert stimmt.

### Könnte dieser Fall überhaupt einen Unterschied zeigen?

_Die gemeinsame Frage hinter vier Fällen aus einer Nacht._

Vier Prüffälle sahen aus wie ein Beleg und waren keiner. Jedes Mal lag es an der **Quelle** der
Vergleichszahl:

| Fall                              | Woher die Zahl kam                                   | Was daran falsch war                                                          |
| --------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------------------------- |
| **Tautologie**                    | aus dem Prüflauf selbst                              | `[0,1,2].slice(0,2)` gegen `[0,1]` — die eigene Erwartung mit sich selbst     |
| **Falscher Gegenfall**            | aus einer zweiten Ursache                            | Der Vergleichslauf scheiterte an der Kampfdauer, nicht am fehlenden Feld      |
| **Fehlschlag aus falschem Grund** | aus dem Werkzeug statt aus dem Fall                  | Der Übersetzer brach ab, der Prüffall lief nie                                |
| **Fall ohne Unterschied**         | aus einer Umsetzung, die beide Wege gleich behandelt | Platz 1 fiel weg — ein nachrückender Eintrag landet dort **zufällig richtig** |

> **Die Frage vor jedem Nachweis:** Woraus kommt die Zahl, gegen die ich vergleiche? Aus einer
> Quelle, die **unabhängig** von dem ist, was ich prüfe — oder aus derselben Rechnung, demselben
> Lauf, demselben Werkzeug?
>
> **Und die schärfere Fassung**, nachdem die erste zweimal nicht gereicht hat: Nicht nur, woher
> die Zahl kommt — sondern ob der **Fall** so liegt, dass richtig und falsch sich unterscheiden
> **können**.
>
> Der vierte Fall hat keinen sichtbaren Fehler: Die Zahlen kamen aus der Umsetzung, der Prüffall
> war sauber gebaut, die Gegenprobe lief. Sie schlug nur nicht an, weil beide Umsetzungen an
> dieser Stelle dasselbe liefern. Erst als der **vordere** Platz wegfiel statt des hinteren,
> gingen Nachrücken und Behalten auseinander.

Der erste Fall ist der heimtückischste, weil er eine **Auflage formal erfüllen** kann. Die
Auflage lautete: _„Die Gegenprobe über zwei Zustände — Belegung auf Stufe 10 gegen Stufe 20,
nicht nur die Zahl."_ Der Prüffall verglich zwei Zustände. Beide stellte er selbst her.

**Eine Auflage, die die Form eines Nachweises beschreibt, sagt nichts über seine Quelle.** Wer
die Form erfüllt, glaubt fertig zu sein — und liegt richtig, solange niemand fragt, woher die
Zahlen stammen.

### Bei einer Gegenprobe zählt die Meldung, nicht der Rückgabewert

_Gefunden am 2026-08-05 in Etappe 2 von #248._

Eine Gegenprobe sollte belegen, dass ein Prüffall den Fall „steigt **mehrere** Stufen" wirklich
hält. Dafür wurde `while` durch `if` ersetzt:

```
Rückgabewert 1 — sah nach Fehlschlag aus
```

Es war ein **Übersetzungsfehler**: Das `break` stand plötzlich ohne Schleife da. Der Prüffall
lief kein einziges Mal.

> **Rückgabewert 1 heißt nicht „der Prüffall hat angeschlagen", sondern nur „irgendetwas ging
> schief"** — und beim Gegenproben sind das zwei sehr verschiedene Dinge.

Eine Gegenprobe, die aus dem falschen Grund scheitert, sieht aus wie ein Erfolg. Man hakt sie ab
und geht weiter — und der Prüffall, den man belegen wollte, ist weiterhin ungeprüft.

Die richtige Fassung lieferte die Meldung, auf die es ankam:

```
expected 2 to be greater than 3
```

**Die Regel:** Bei einer Gegenprobe wird die **Meldung** gelesen, nicht der Rückgabewert. Sie
muss den Prüffall nennen, den man treffen wollte, und die Zahl, die er beanstandet.

Die Umkehrung desselben Fehlers steht weiter unten in der Messfehler-Tabelle: `RÜCKGABEWERT 0`
durch `tail` — ein falscher **Erfolg**. Beide führen dazu, dass die Zahl gelesen wird statt
ihrer Herkunft.

### Ein Prüffall hinter einem Wächter ist grün ohne Grund

_Gefunden am 2026-08-05 bei der fünften Gegenprobe zu #248._

Ein Leser für die Stufen-Tabelle prüft beim Einlesen mehrere Zusicherungen. Daneben steht ein
Prüffall, der belegen soll, dass die Werte **gleichmäßig** wachsen. Vier Gegenproben liefen:

```
Stufe 1 nicht 1000        → Leser bricht ab, mit dem Grund
Aufstieg macht schwächer  → Leser bricht ab, nennt die Stufe
Tabelle zu kurz           → Leser bricht ab, nennt beide Zahlen
Sprung, nicht monoton     → Leser bricht ab
```

**Alle vier fing der Leser ab.** Der Verlaufs-Prüffall kam kein einziges Mal zum Zug — er war
grün, weil er nie erreicht wurde.

Erst die fünfte Probe traf ihn:

```
Sprung, aber MONOTON      → Leser lässt ihn durch, und HIER greift der Prüffall
```

> **Eine Prüfung, die hinter einer strengeren liegt, wird von den naheliegenden Gegenproben nie
> erreicht.** Sie ist dann grün, ohne je etwas gemessen zu haben — und der Bau sieht doppelt
> abgesichert aus, wo nur eine Sicherung greift.

**Die Gegenfrage:** Kommt der Fehler, den ich einbaue, überhaupt bis zu dem Prüffall, den ich
prüfen will? Wo mehrere Prüfungen hintereinander stehen, braucht jede eine Gegenprobe, die
**genau sie** trifft — und die früheren passieren lässt.

### Ein Wert, der bei der Niederlage fehlt, sagt nichts über den Sieg

_Gefunden am 2026-08-05 beim Browser-Nachweis zu #247._

Ein Boss mit umgekehrtem Ziel wurde im Browser gespielt, der Lauf endete als Niederlage. Der
Ergebnis-Bildschirm zeigte drei Anzeigen, die **sichtbar** das Gegenteil sagten — und eine
vierte, die gar nichts sagte:

```
Punkte:  —
```

Das las sich wie „kein Sieg, keine Punkte" und war die einzige Stelle, an der es wirklich weh
tut. Die Punkteformel rechnet die **Restzeit** beim Sieg; bei diesem Boss ist der Sieg das
Ablaufen der Frist, es gibt also keine Restzeit. Sie hätte still **null** geliefert — für den
bestmöglichen Lauf dieselbe Punktzahl wie für den knappsten.

> **Die drei anderen Anzeigen logen sichtbar. Die vierte schwieg, und Schweigen liest sich wie
> Unauffälligkeit.**

**Die Regel:** Ein leeres Feld ist kein geprüftes Feld. Wer einen Fehlschlag ansieht, hat den
Erfolgsfall nicht gemessen — und umgekehrt. Bei jeder Anzeige, die im geprüften Ausgang **nicht
vorkommt**, gehört die Frage dazu: _Was stünde hier im anderen Ausgang?_

Gefunden hat es der Werker selbst, nachdem ihm der fehlende vierte Punkt gemeldet worden war. Er
hat nicht die Liste übernommen, sondern gefragt, **warum** er ihn übersehen hatte.

### Die gefährlichste Reparatur ist die, die den Fehler erst erzeugt

_Gefunden am 2026-08-05 beim Bau von #247, Etappe 2._

Ein Rust-Prüffall erwartete 300 und bekam 240. Das sah aus wie eine Abweichung **zwischen den
Seiten** — genau der Fall, für den die Parität gebaut ist.

Es war eine Abweichung zwischen **zwei eigenen Prüffällen**:

```
der im Spiel      lief bis einschließlich 5000
die Wiedergabe    läuft tick < ticks, also bis 4990
```

Beide behaupteten dieselbe Zahl und maßen verschiedene Zeiten.

> **Hätte ich die Rust-Seite an das Spiel „angepasst", wäre daraus eine echte Abweichung
> geworden — an einer Stelle, die nie wieder jemand angesehen hätte.**

Das ist eine eigene Klasse. Die Abschnitte oben behandeln Prüfungen, die nichts fangen. Hier
fängt die Prüfung etwas Richtiges, und die naheliegende Reparatur macht daraus einen echten
Fehler: Die Parität hätte danach grün gemeldet, und die beiden Seiten wären wirklich
auseinander gewesen.

**Die Regel:** Zwei Prüffälle, die dieselbe Zahl behaupten, müssen dieselbe Zeit messen. Bevor
eine Seite an die andere angeglichen wird, muss belegt sein, **welche** von beiden recht hat —
und zwar aus einer dritten Quelle, nicht aus der Erwartung.

Erkennungszeichen: Der Unterschied ist klein und „sieht nach Rundung aus". Genau dann lohnt der
Blick auf die Schleifengrenzen.

### Ein Wächter, dessen Referenz im selben PR mitwächst, kann nicht anschlagen

_Gefunden am 2026-08-05, nachdem derselbe Fall dreimal verschieden erklärt worden war._

`sim/tests/parity_boss_state.rs` prüft seit Längerem, dass Spiel und Simulator dieselbe
Boss-Menge sehen — das Spiel über `import.meta.glob`, der Simulator über `build.rs`. Sein
Kommentar sagt: _„AC1 ist erst erfüllt, wenn eine neue Datendatei auf **beiden** ankommt."_

Als in #250 ein neuer Prüf-Boss dazukam, blieb er **grün**. Nicht weil er falsch gebaut war,
sondern weil die Vergleichsdatei im selben PR mitwuchs: Er verglich zwei Stände, die beide schon
den neuen Boss kannten.

> Der Wächter ist richtig gebaut, wird ausgeführt, ist grün — und kann in genau dem Moment
> nichts sagen, in dem sich ändert, was er bewacht.

Das ist gefährlicher als eine Prüfung, die den Fall nicht erreicht: Dort fehlt die Absicherung
sichtbar. Hier ist sie vorhanden, läuft und meldet Erfolg.

**Die Gegenfrage:** Wächst im selben PR auch das, wogegen geprüft wird? Dann muss die Gegenprobe
**vor** dem Mitwachsen ansetzen — die Referenz aus `main` nehmen, nicht die neu erzeugte.

### Ein Gegenfall, der aus dem falschen Grund anders ausgeht, belegt nichts

_Vom selben Tag, an derselben Arbeit._

Der Abschnitt oben behandelt Prüfdaten, die den Fall unerreichbar machen. Das hier ist die
Umkehrung — der Gegenfall geht anders aus, aber aus dem falschen Grund:

```
Gegenfall: derselbe Lauf gegen den Ersten Schmelztiegel
  4240 Lebenspunkte, 60 s Frist — KINDLE allein schafft rund 4000
  Der Lauf endete weder als Sieg noch überhaupt
```

Er sah aus wie ein Beleg („mit dem Feld gewinnt man, ohne nicht") und belegte nichts: Der zweite
Lauf scheiterte an der Kampfdauer, nicht am fehlenden Feld.

> **Die Regel:** Zwischen Fall und Gegenfall darf sich **genau eine** Größe unterscheiden. Der
> Werker hat es so gelöst: derselbe Boss-Datensatz, einmal mit `runaway`, einmal mit
> `runaway: null`. Ein Feld, sonst nichts.

### Eine Probe braucht die Liste des Erlaubten, nicht des Verbotenen

_Gefunden in der Nacht zum 2026-08-05, an derselben Probe zweimal._

Nach jedem Neuexport der Vergleichsdateien gilt die Regel: **Ändert sich mehr als die
Prüfsummen, hat sich eine Regel bewegt.** Sie ist richtig — und sie zweimal falsch anzuwenden
kostet eine Viertelstunde und, schlimmer, das Vertrauen in sie.

| Versuch                                     | Was passierte                                                                                                                         |
| ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `expected` als **Ganzes** verglichen        | Meldete „in sieben Dateien geändert" — darin stecken `finalChecksum` und `sequenceChecksum`. Ein Fehlalarm, der wie ein Befund aussah |
| Rekursiv verglichen, Prüfsummen ausgenommen | Fand `allBossIds: + test-crucible-trial`. Korrekt (ein neuer Prüf-Boss gehört in die Liste), aber kein Prüfsummen-Feld                |

> **Die Probe soll den Fall fangen, in dem sich eine Regel bewegt hat.** Genau dieser Fall — eine
> Boss-Liste wächst — sieht in der Datei aus wie ein Spielwert und ist einer. Dass er harmlos
> war, ist Glück, nicht die Leistung der Probe.

**Die Regel:** Führe die Felder, die sich ändern **dürfen** — Prüfsummen plus dokumentierte
Ausnahmen. Nicht die, die sich nicht ändern sollen. Der Unterschied ist derselbe wie zwischen
„ist es grün" und „wäre es rot".

Und der Grund, warum ein Fehlalarm teurer ist als er aussieht, stammt von dem Werker, dem er
passierte:

**„Wer das zweimal erlebt, hört auf hinzusehen."**

### Und der Bezugspunkt ist die Merge-Base, nicht HEAD

_Nachgetragen am 2026-08-05, nachdem der Abschnitt oben den Fehler falsch benannt hatte._

Die erste Fassung der Probe war **nicht blind** — sie maß gegen den falschen Stand:

```
Basis HEAD               vessel-phases.json: expected, checksums
Basis merge-base(main)   vessel-phases.json: allBossIds, expected, checksums
```

Der neue Prüf-Boss kam im **ersten** Commit des Zweigs herein und stand zum Zeitpunkt der
Messung längst in der Basis. Der Unterschied war real — nur nicht gegenüber dem Stand, gegen den
verglichen wurde.

> **Wer gegen HEAD misst, misst den letzten Schritt und nennt es die Abnahme.**

Das ist wichtig für den Abschnitt darüber: Eine Positivliste allein hätte diesen Fall **nicht**
gefangen. Mit falschem Bezugspunkt meldet auch eine vollständige Liste nichts, weil sie nichts zu
vergleichen bekommt. Es sind zwei unabhängige Regeln, und beide werden gebraucht.

Gefunden hat es der Werker selbst — an einem Befund, den der Orchestrator ihm als Lücke seiner
Probe gemeldet hatte.

### Eine Zahl ohne committeten Läufer ist eine Behauptung

_Gefunden am 2026-08-05 beim Abnehmen von #224._

Die Balance-Messung, auf der zwei Design-Entscheidungen des Auftraggebers beruhen, lässt sich
nicht wiederholen:

```
$ git log --all --oneline -S "9892"        → nur Commits von heute
$ git grep "best_affix_build("             → nur die Definition, kein Aufruf
$ git log --all --diff-filter=D            → keine gelöschte Datei
```

Der Aufruf wurde **nie committet**. Durchgangszahl, Startwert, Kampfdauer und Boss sind
unbekannt. Beim Abnehmen wurde versucht, die Durchgangszahl aus dem Fehlerbalken zurückzurechnen
— und das Ergebnis passte nur bei einer bestimmten Annahme.

> **Eine Zahl, die zufällig stimmt und nicht belegbar ist, ist keine gemessene Zahl.**
>
> Hätte der Rückschluss danebengelegen, wäre es genauso durchgegangen. Das ist der Punkt.

**Die Regel:** Jede Zahl, die in einem Issue oder Bericht landet, braucht einen committeten
Läufer mit festen Parametern — und der Läufer druckt sie in seiner Ausgabe mit, damit die Zahl
ihre Herkunft mitträgt, auch wenn jemand sie herauskopiert.

Der Werker, dem es passierte, hat die brauchbarste Einordnung geliefert: _„Dass es diesmal besser
gemacht wurde, ist kein Verdienst — es ist die Folge davon, dass das Issue einen **Bericht
verlangt** hat. Wäre er nicht in den Abnahme-Kriterien gestanden, wären wieder nur Zahlen
gemeldet worden."_ Nicht Sorgfalt hat den Unterschied gemacht, sondern ein Kriterium, das den
Läufer erzwang.

### „Bereit zum Merge" ist keine Eigenschaft, sondern ein Zeitpunkt

_Zwei Fundstellen in derselben Nacht, dieselbe Lücke von zwei Seiten._

| Was gemeldet wurde   | Warum es überholt war                                                                                                        |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| „bereit zur Abnahme" | Während die Nachricht rausging, wurde ein anderer PR gemergt — der eigene stand auf BEHIND, bevor sie ankam                  |
| „bereit zum Merge"   | Der Melder arbeitete noch an derselben Datei; der Orchestrator mergte auf diese Freigabe hin, und die letzte Änderung fehlte |

Einmal bewegte sich `main`, einmal der Melder selbst.

> **Melde den Zustand mit dem Zeitpunkt seiner Messung** — „CLEAN um 02:04" statt „bereit". Und
> sag „bereit zum Merge" erst, wenn die Datei nicht mehr offen ist: nicht wenn der letzte Lauf
> grün war, sondern wenn die Arbeit fertig ist.

Der zweite Fall fiel nur auf, weil der Push `[new branch]` meldete statt des üblichen
Commit-Bereichs — der Zweig war nach dem Merge gelöscht, und der Push legte ihn neu an, ohne PR
und ohne dass es jemand sieht. Diese Meldung steht weiter unten in der Tabelle der Messfehler,
und sie wurde diesmal gelesen statt überblättert.

### Die Schwester davon: Prüfdaten, die den Fall unerreichbar machen

_Fünf Fundstellen in der Nacht zum 2026-08-05, in drei verschiedenen Rollen, innerhalb von drei
Stunden._

Oben ging es um Prüfungen, die den Fall nicht erreichen. Hier ist die Prüfung richtig gebaut — die
**Zahlen im Datensatz** machen den geprüften Fall unmöglich. Das Kriterium ist grün und hat nie
etwas berührt.

| Gewählte Zahlen                                                                                        | Warum der Fall nie eintrat                                                                                                                                     |
| ------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Stapel-Obergrenze **5**, bei 4,5 s Abklingzeit und 12 s Haltbarkeit                                    | Höchstens **drei** Stapel sind gleichzeitig erreichbar. Das AC zur Obergrenze konnte nie anschlagen                                                            |
| Schwelle **1500** für das Wettrennen, Fenster 6 s                                                      | In sechs Sekunden passen vier Wirkungen à 100 Schaden. Der **Erfolgsfall** des Kriteriums war unerreichbar — belegt war nur der Fehlschlag                     |
| **600** Heilung bei **999 999** Lebenspunkten                                                          | Null Prozentpunkte am Balken. Der Prüffall „der Balken läuft rückwärts" maß nichts                                                                             |
| Die Rotation im Balance-Läufer kennt **3** Zauber, der Kasten hat **15**                               | Die Messung hätte exakt die alten Zahlen geliefert — und daraus wäre „die neuen Zauber verschieben nichts" geworden. Sie kann die Änderung gar nicht erreichen |
| **1000 ms** Verkürzung auf eine Abklingzeit, die zum Zeitpunkt der Wirkung noch **10 ms** Restzeit hat | Die Verkürzung wird auf 0 gekappt. Drei grüne Prüffälle und eine Takt-für-Takt-Prüfsumme belegten, dass beide Seiten **dasselbe Nichts** rechnen               |

> **Die gemeinsame Wurzel: Zahlen, die aussehen wie unabhängige Freiheitsgrade, sind es nicht.**
> Obergrenze, Abklingzeit und Haltbarkeit sind zwei Größen plus eine Folge. Schwelle, Fensterdauer
> und Schaden je Wirkung ebenso. Wer sie einzeln wählt, wählt eine davon zwangsläufig wirkungslos.
>
> **Die Gegenfrage:** Nicht „ist die Zahl plausibel?", sondern **„kann der Fall mit diesen Zahlen
> überhaupt eintreten — und der Gegenfall auch?"** Beide Ausgänge müssen erreichbar sein, sonst
> prüft das Kriterium nur einen davon.
>
> Alle drei wurden von der Messung aufgedeckt, keiner beim Lesen. Und in allen drei Fällen hat der
> Werker die Rechnung danach **in den Datensatz geschrieben** statt die Zahl stillschweigend zu
> korrigieren — damit der nächste sie nicht erneut anfassen muss.

> **Der fünfte Fall ist der schärfste, weil dort sogar die Prüfsumme zustimmte.** Sie vergleicht,
> ob beide Seiten dasselbe rechnen — nicht, ob das Gerechnete etwas bewirkt. Wer nur auf sie sieht,
> hält „identisch" für „richtig".
>
> Und alle fünf hat die **Messung** gefunden, keiner fiel beim Lesen auf. Der Werker, dem der fünfte
> passierte, hat ihn selbst so eingeordnet: _„Hätte ich den vierten Prüffall nicht gebaut, wären die
> drei grünen durchgegangen und das Issue wäre erledigt gewesen."_

**Die gemeinsame Frage, die alle sieben beantwortet hätte:**

> Unter welcher Bedingung würde meine Messung ein **anderes** Ergebnis liefern?
>
> Fällt dir keine ein, misst du nichts.

Konkret heißt das:

- **Prüfe auf das Merkmal, das sich ändert** — nicht auf Existenz. `hidden`, nicht `querySelector`.
- **Werte Rückgabewerte aus, statt sie anzuzeigen.** `cmd && weiter` oder `set -e`, nie `echo $?`.
- **Prüfe das, was laufen wird — nicht eine Näherung davon.** Ein Skript von Hand zu starten prüft
  das Skript; ob der Cron-Job läuft, prüft nur `env -i`. Wer den Eintrag anlegt und das Skript
  testet, hat die Zustellung nicht geprüft.
- **`grep -c` zählt Zeilen, nicht Treffer.** Auf minifiziertem JS, einzeiligem JSON oder jeder
  anderen langen Zeile ist die Antwort immer `0` oder `1`. Treffer zählt `grep -o … | wc -l`.
- **Eine Zeilenspanne ist eine Behauptung über den Aufbau der Datei.** `sed -n '346,400p'` kann über
  das Ende der Funktion hinausreichen — dann misst du im Nachbarn und schließt auf deinen.
- **Leite direkt am Befehl in eine Datei um** — `cargo test > lauf.txt 2>&1`, dann darin zählen.
  Nicht `| tail`, nicht `| head`. `grep -c` über den vollen Strom ist in Ordnung, weil es filtert
  statt abzuschneiden.

  > **„In einer Datei messen" allein genügt nicht.** Die Harness legt einen langen Ausgabestrom
  > selbst in eine Datei und nennt sie „full output" — war er vorher schon durch `tail -40`
  > gekürzt, steht **die Kürzung** darin. Wer dann in dieser Datei zählt, glaubt den ganzen Lauf zu
  > sehen und sieht ein Achtel. Die Umleitung muss **vor** jeder Kürzung stehen.

- **Zähle, was BESTANDEN hat — nicht nur, was fehlgeschlagen ist.** `0 FAILED` ist auch dann wahr,
  wenn vier von 27 Sammlungen gelaufen sind. Die Zahl der bestandenen Sammlungen ist die Größe, die
  eine unvollständige Messung verrät.
- **Belege mit einem Merkmal, das nur dein Stand hat** — nicht mit der Abwesenheit von etwas. Die
  Abwesenheit hat viele Ursachen, und eine davon ist, dass du am falschen Ort suchst.

### Eine Prüfung kann durch eine spätere Änderung zur Konstruktion werden

_Gefunden und selbst gemeldet von E7-2 am 2026-08-04._

Der heimtückischste Fall, weil ihn niemand verursacht: Eine Prüfung war ein echter Nachweis, und
eine **spätere, richtige** Änderung macht sie zur Tautologie. Sie bleibt grün, niemand fasst sie an,
und sie steht weiter in der Melde-Liste.

```
vorher   missedCastWindows und idleWindows wurden getrennt gerundet
         Die Prüfung "beide zusammen ergeben die Gesamtzahl" konnte FEHLSCHLAGEN
         → sie war ein Nachweis

nachher  idleWindows = gesamt − missedCastWindows
         Dieselbe Prüfung ist jetzt per Konstruktion wahr
         → sie ist eine Tautologie, die wie ein Nachweis aussieht
```

Messbar wurde es an einer Zahl: Dieselbe Verfälschung brach vorher **zehn** Prüffälle, nachher
**sechs**. Die vier Weggefallenen hatten sie über eine Rundungs-Zufälligkeit gefangen, nicht über
eine echte Eigenschaft.

> **Die Regel:** Wer eine Rechnung ändert, prüft nach, **welche seiner Zusicherungen dadurch
> unhaltbar geworden sind** — im Sinne von: nicht mehr falsifizierbar. Und schreibt es in die
> Meldung, statt die alte Formulierung stehen zu lassen.
>
> Niemand hätte es bemerkt. Genau deshalb steht es hier.

### Der Prüffall lief an einem Punkt, an dem die Abweichung gar nicht sichtbar sein kann

_Drei Fundstellen am 2026-08-04, in drei verschiedenen Worktrees, alle drei von den Findern selbst
gemeldet._

Der Prüffall ist richtig gebaut, das Messinstrument ist in Ordnung — aber die **Stelle**, an der
gemessen wird, ist so gewählt, dass der gesuchte Fehler dort keine Spur hinterlässt. Er wird grün,
egal ob der Fehler da ist oder nicht.

| Fall                                                                                       | Warum die Abweichung unsichtbar war                                                                                         |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------- |
| Wächter gegen versehentlich beschleunigte Boss-Zauber, gemessen an der **Summe** über 60 s | Der Zauber landete 1500 ms zu früh — und traf trotzdem. Am Kampfende stand dieselbe Summe. **Zwölf von zwölf blieben grün** |
| Neues Feld im Testliteral mit dem **neutralen Wert** nachgetragen                          | `1.0` bewegt keine Zahl. Der Wächter blieb grün, obwohl der Faktor in der Namensliste fehlte                                |
| Prüffall gegen einen Durchlauf mit **genau 50 %** Fortschritt                              | Dort ist die verbliebene Gesundheit ebenfalls 50. Die beiden Größen sind an diesem Punkt nicht zu unterscheiden             |

**Die gemeinsame Frage:**

> An der Stelle, an der ich messe — **gehen die beiden Größen überhaupt auseinander?**
>
> Bei 50 zu 50 nicht. Bei einem neutralen Wert nicht. Bei einer Summe über den ganzen Lauf nicht,
> wenn der Fehler nur etwas **verschiebt**.

Konkret heißt das:

- **Gegen eine Verschiebung in der Zeit hilft keine Summe.** Der Prüffall muss auf die Uhr sehen,
  nicht auf das Konto — Kampfende so legen, dass der Zauber regulär **dahinter** und fälschlich
  beschleunigt **davor** landet.
- **Ein Wächter, dem man beim Nachtragen den neutralen Wert gibt, hört auf zu wachen.** Neue Felder
  in Testdaten bekommen einen Wert, der etwas **bewegt**.
- **Wähle Zahlen, die sich unterscheiden.** 66 zu 34, nicht 50 zu 50. Und sichere ausdrücklich zu,
  dass sie es tun — sonst wandert der Fall beim nächsten Umbau still zurück.

> Alle drei fielen ihren Findern **beim Nachlesen des eigenen Prüffalls** auf, nicht durch ein
> Werkzeug. Der Compiler sagt dazu nichts, und die CI bleibt grün.

### „Lief es sauber" beantwortet nicht „war es das Richtige"

_Gefunden und selbst gemeldet von E1 am 2026-08-04, beim Nachziehen eines PR._

Der Zweig war der falsche. Der Merge von `main` lief trotzdem **konfliktfrei** durch, 34
Sammlungen grün, alles wie erwartet:

```
git checkout feat/e6-4-aufwertung     # klang richtig — war der Zweig von heute früh
git merge main                        # sauber, keine Konflikte
cargo test                            # 34 Sammlungen ok
```

Gefangen hat es erst der Versuch, das eine Merkmal zu benutzen, das **nur dieser Zweig** hat:

```
cargo run --example gear-balance
error: no example target named `gear-balance`
```

> **Ein sauberer Merge auf dem falschen Zweig sieht aus wie Erfolg.**
>
> **Die Prüfung „lief es sauber" kann die Frage „war es das Richtige" nicht beantworten. Sie sind
> unabhängig.**

Dasselbe Muster wie die `--base`-Falle in `CLAUDE.md`: **an einem Merkmal prüfen, das nur der
eigene Stand hat** — nicht an der Abwesenheit von Fehlern.

Und die zweite Hälfte gehört dazu: E1 hatte **nicht gepusht**, sondern mit `git reset --hard`
zurückgenommen. Verloren war nichts.

### Die Uhr kann genauso danebenliegen wie der Wert

_Gefunden und selbst gemeldet von E7 am 2026-08-04, beim Warten auf die CI._

Zwei Schleifen hintereinander: erst auf zehn Einträge warten, dann auf „kein pending". Dazwischen
tauschte GitHub die Liste aus — erst standen die zehn des **alten** Laufs da, dann vier neue auf
pending. Die erste Schleife stieg auf dem alten Lauf aus und meldete `0 bestanden, 0 fehlgeschlagen`.

**Die Bedingung gehört in EINEN Ausdruck**, nicht in zwei aufeinanderfolgende Wartephasen:

```
mindestens zehn bestanden UND kein pending UND kein fail
```

> Es ist dieselbe Klasse wie die Fälle oben — eine Messung, die etwas anderes misst als gemeint —
> **nur an der Uhr statt am Wert.** Zwischen zwei Abfragen kann sich das ändern, worüber die erste
> geurteilt hat.

### Ein neuer Zauber ist kein lokaler Eingriff

_Von E3 am 2026-08-04 teuer gelernt: ein Commit, fünf rote Prüffälle in fremden Dateien._

`SPELL_COUNT` ist eine feste Größe für **alle** Zustands-Listen im Kampf, und mehrere Prüffälle
führen Erwartungen darauf. Wer einen Zauber anlegt, ändert damit:

| Ort                                            | Was dort hängt                               |
| ---------------------------------------------- | -------------------------------------------- |
| `sim/build.rs`                                 | kennt die neue Datei nicht                   |
| `data.rs`, `data_source.rs`                    | Zauberzahl, Datei-Abgleich, Schritt-1-Filter |
| `report.rs` und zwei weitere                   | Erwartungslisten „toter" Zauber              |
| `step1.test.ts`, `data.test.ts`, `log.test.ts` | abgeschriebene Zahlen                        |
| `data/sound-manifest.json`                     | jeder Zauber braucht genau einen Klang       |

**Die Prüffrage bei jeder dieser Stellen:** Würde sie nach meiner Änderung noch rot werden, wenn
jemand einen Zauber vergisst? Wenn nein, hast du einen Wächter **entschärft** statt ihn nachgezogen.

> Die richtige Reparatur ist fast nie „die Zahl hochsetzen". `tote.len() == table.len() - 1` ist
> keine größere Zahl, sondern eine andere **Aussage** — und sie braucht beim nächsten Zauber keine
> Pflege mehr.

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
