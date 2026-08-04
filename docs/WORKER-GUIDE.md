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

_Vier Fundstellen am 2026-08-04, an vier verschiedenen Stellen, in vier verschiedenen Rollen._

Die Abschnitte oben behandeln Prüffälle, die zu wenig zeigen. Das hier ist eine andere Klasse:
**Der Prüffall ist richtig, aber das Messinstrument liefert eine Zahl, die nichts mit der Frage zu
tun hat.** Er wird dann nicht rot — er wird grün und bedeutet nichts.

| Was gemessen wurde                                  | Warum es nichts belegte                                                                    |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `!!document.querySelector('[class*=rota-loot]')`    | `skipLoot()` setzt `hidden`, das Element **bleibt im DOM** — der Ausdruck ist immer `true` |
| `cargo test 2>&1 \| tail -40`                       | Die letzten 40 Zeilen sind vier Sammlungen von 27. `0 FAILED` stimmt und beweist nichts    |
| `npx tsc --noEmit; echo "Exit=$?" && git commit …`  | Das `echo` gelingt immer. Der Rückgabewert wurde **angezeigt**, nicht ausgewertet          |
| `window.__rota === undefined` als Beleg für den Bau | Die Seite lud aus einem fremden Worktree — das erwartete Ergebnis aus der falschen Quelle  |

**Die gemeinsame Frage, die alle vier beantwortet hätte:**

> Unter welcher Bedingung würde meine Messung ein **anderes** Ergebnis liefern?
>
> Fällt dir keine ein, misst du nichts.

Konkret heißt das:

- **Prüfe auf das Merkmal, das sich ändert** — nicht auf Existenz. `hidden`, nicht `querySelector`.
- **Werte Rückgabewerte aus, statt sie anzuzeigen.** `cmd && weiter` oder `set -e`, nie `echo $?`.
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
