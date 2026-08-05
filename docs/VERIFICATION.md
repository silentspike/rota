---
id: ROTA-VERIFICATION
status: Stable
ssot: true
refs:
  docs: [GAME-DESIGN, WORKER-GUIDE, STEP1-SPEC]
---

# Abnahme-Checkliste der Master-Session

> **Für die Master-Session, nicht für Worker.** Hier steht, wie ich die Arbeit einer Dev-Session
> prüfe — mit ausführbaren Befehlen, damit das Verfahren über 55 Issues gleich streng bleibt und
> nicht mit der Zeit abschleift.

## Der Grundsatz

> **Behauptungen des Workers sind kein Nachweis.** Sie sind der Hinweis darauf, wo ich nachmessen
> soll. Jede Acceptance Criteria wird von mir **selbst** ausgeführt, mit **meinem** Befehl und
> **meiner** Ausgabe.

Das steht so auch in der globalen `CLAUDE.md`: _Agent-Ergebnisse niemals ungeprüft weitergeben._
Der Grund ist nicht Misstrauen, sondern Arbeitsteilung — der Worker sieht sein Issue, ich sehe das
Gesamtsystem. Fehler entstehen fast immer dazwischen.

---

## 1. Ablauf einer Abnahme

| #   | Schritt                                                                   |
| --- | ------------------------------------------------------------------------- |
| 1   | Issue-Text und die zugehörigen Abschnitte in `GAME-DESIGN.md` aufschlagen |
| 2   | Prüfen, ob der PR gemerged und `ci-pass` grün war — sonst sofort zurück   |
| 3   | Spiel im aktuellen `main`-Stand starten (siehe § 2)                       |
| 4   | **Jede AC einzeln** ausführen, Befehl und Ausgabe protokollieren          |
| 5   | Zusätzlich die Standard-Messungen aus § 3, soweit das Issue sie berührt   |
| 6   | Ergebnis nach § 5 melden                                                  |

**Nicht gebündelt prüfen.** „Alle fünf ACs auf einmal durchgesehen" ist keine Abnahme. Jede AC
bekommt ihren eigenen Befehl und ihre eigene Ausgabe — auch wenn zwei sich ähneln.

**Frische Nachweise.** „Das habe ich bei Issue 3 schon geprüft" zählt nicht. Der Code hat sich
seitdem geändert.

### Eine leere Prüfliste ist nicht dasselbe wie eine laufende

_Von Worker-1 am 2026-08-05 bei #282 gemeldet, nachdem es ihm fast entgangen wäre._

```
$ gh pr checks 347
no checks reported on the 'feat/...' branch
```

Das liest sich wie **„läuft noch"**. Es hieß **„startet nie"**: Der Zweig war `CONFLICTING`, und
GitHub startet für einen Zweig mit Konflikt gar keine Läufe.

> Kein Ergebnis sieht aus wie ein unauffälliges Ergebnis. Dieselbe Gestalt wie die leere Namensfolge
> und wie ein Werkzeug, das ohne Fehlermeldung nichts tut.

**Das trifft die Warteschleife dieser Checkliste unmittelbar.** Wer auf `ci-pass` wartet und nur auf
`SUCCESS` oder `FAILURE` prüft, wartet bei einem Konflikt bis zum Abbruch — und weiß danach nicht,
warum. Deshalb gehört `mergeable` in dieselbe Abfrage:

```bash
gh pr view <n> --json mergeable,mergeStateStatus,statusCheckRollup \
  -q '.mergeable+" | "+.mergeStateStatus+" | "+([.statusCheckRollup[]|select(.name=="ci-pass")|.conclusion//"laeuft"]|join(","))'
```

| Ausgabe                            | Bedeutung                                                                          |
| ---------------------------------- | ---------------------------------------------------------------------------------- |
| `MERGEABLE \| BLOCKED \|`          | Checks laufen — warten ist richtig                                                 |
| `CONFLICTING \| DIRTY \|`          | **Es wird nie etwas kommen.** Zweig aktualisieren lassen                           |
| `MERGEABLE \| CLEAN \| SUCCESS`    | fertig                                                                             |
| `UNKNOWN \| UNKNOWN \|`            | **Zwei ganz verschiedene Lagen** — siehe unten                                     |
| `MERGEABLE \| BEHIND \| SUCCESS`   | **Fertig, aber nicht mergefähig** — `strict` verlangt den aktuellen Stand          |
| `MERGEABLE \| UNSTABLE \| SUCCESS` | **`ci-pass` grün, ein Check außerhalb davon nicht** — der PR ist trotzdem mergebar |

#### `UNSTABLE` ist Absicht, kein Loch

Am 2026-08-05 stand an PR #376 `MERGEABLE | UNSTABLE | SUCCESS`. `ci-pass` war grün, der
Gesamtzustand nicht — `pr-title` war rot, weil der Titel keinen Conventional-Commits-Präfix trug.

**Der Reflex, daraus einen Befund zu machen, ist falsch.** Der Kopf von `.github/workflows/meta.yml`
sagt, warum:

> Was hier bleibt, ist bewusst **nicht** Teil von `ci-pass`: ein falscher PR-Titel ist kosmetisch
> und soll den Code nicht blockieren, aber sichtbar sein.
>
> `actionlint` und `secret-scan` sind aus genau diesem Grund nach `ci.yml` gewandert (PR #66):
> Ein gefundenes Geheimnis oder ein syntaktisch kaputter Workflow darf **nicht** mergebar sein.

Die Trennung ist gemessen und begründet. Was daraus für die Abnahme folgt:

```bash
gh pr checks <n> | grep -v pass     # bei UNSTABLE: wer fällt?
```

**`UNSTABLE` heißt: hinsehen, dann entscheiden.** Nicht wegmergen, aber auch nicht blockieren —
die Frage ist, _welcher_ Check fällt, und die Antwort steht in einem Aufruf.

> Beinahe-Panne desselben Tages: Der Orchestrator hielt die Trennung für ein Loch im Required
> Check, maß die Jobs, die Aggregat-Struktur und die Branch Protection — und las den Kommentar
> über der Datei nicht, die er ändern wollte. **Dieselbe Bewegung wie „einen Wächter bauen, ohne
> zu suchen, ob es einen gibt": Die Antwort stand an der Stelle, an der die Arbeit anfing.**

**`UNKNOWN` heißt entweder „GitHub rechnet noch" oder „der PR ist längst gemergt".** GitHub
berechnet `mergeable` asynchron; bei einem geschlossenen PR steht dort dauerhaft nichts. Beim
Nachmessen an einem frischen PR: durchgehend `MERGEABLE | BLOCKED`, bei einem gemergten dauerhaft
`UNKNOWN | UNKNOWN`.

> Wer `UNKNOWN` sieht, fragt zuerst `gh pr view <n> --json state` — bei `MERGED` ist die Frage
> beantwortet, bei `OPEN` wartet man ein paar Sekunden und misst erneut.

**`BEHIND` ist der Zustand, der eine Warteschleife am längsten narrt.** Die Prüfungen sind grün,
der Merge scheitert trotzdem — die Branch Protection steht auf `strict`, und **jeder** fremde Merge
setzt alle offenen PRs zurück in diesen Zustand. Eine Schleife, die auf `CLEAN | SUCCESS` wartet,
läuft dann bis zum Abbruch durch.

```bash
gh pr update-branch <n>   # holt main in den Zweig, danach laufen die Prüfungen erneut
```

> Gemessen am 2026-08-05: fünfzehn Runden gewartet, `SUCCESS` stand ab der dritten. Der eigene
> Abbruch kam erst nach neun Minuten, und der Merge scheiterte danach sofort.

**Die Unterscheidung kostet nichts und spart das Warten auf etwas, das nicht kommt.**

---

## 2. Das Spiel starten und ansprechen

```bash
# Stand holen und bauen
cd /work/rota && git checkout main && git pull --ff-only && npm ci && npm run build

# Seite öffnen (nginx, NIEMALS ein eigener Server)
playwright-cli -s=rota open http://localhost:8000/rota/

# Am Ende immer
playwright-cli -s=rota close
```

Die Entwicklungs-Werkzeuge (GAME-DESIGN 19.1) sind **nur über die Browser-Konsole** ansprechbar —
keine Oberfläche, keine Knöpfe. Abfragen laufen über `playwright-cli`:

```bash
playwright-cli -s=rota eval "window.__rota.perf()"
playwright-cli -s=rota eval "window.__rota.state()"
playwright-cli -s=rota eval "window.__rota.log()"
```

> Der genaue Name des Konsolen-Objekts wird in Issue **E1-8** festgelegt. Bis dahin ist
> `window.__rota` die Annahme; weicht die Umsetzung ab, wird **dieses Dokument** angepasst, nicht
> der Code.

---

## 3. Die fünf Standard-Messungen

> **Jeder `eval`-Ausdruck mit `=>` muss eine nackte Pfeilfunktion sein.**
> `playwright-cli eval` hält jeden Ausdruck, in dem `=>` vorkommt, für eine
> Funktionsdefinition und ruft ihn auf — sonst `TypeError: result is not a function`.
>
> ```bash
> eval "window.__rota.log().filter(e=>e.type==='cast').length"        # scheitert
> eval "(() => typeof window.__rota)()"                               # scheitert AUCH
> eval "() => window.__rota.log().filter(e=>e.type==='cast').length"  # läuft
> ```
>
> Die sofort aufgerufene Fassung `(() => …)()` ist die naheliegende Rettung und
> scheitert genauso — sie enthält ebenfalls `=>`. Von der E1-Session bei #14
> gemeldet, beim Nachprüfen an der eigenen Schreibweise bestätigt.

Aus `GAME-DESIGN.md` 20.2. Sie gelten für jedes Issue, das den Kampf berührt.

### 3.1 Ruckelfreiheit

```bash
playwright-cli -s=rota eval "window.__rota.resetPerf()"
sleep 12   # 600 Bilder bei 60 Hz
playwright-cli -s=rota eval "JSON.stringify(window.__rota.perf().frameTimeSummary)"
```

**Bestanden:** `over: 0` bei mindestens 600 Bildern — kein Bild, dessen **Arbeitszeit** 16,7 ms
überschreitet.

> **`frameTimes` ist die Arbeitszeit je Bild, `frameIntervals` ist der Abstand zwischen zwei
> Bildern. Geprüft wird die Arbeitszeit.**
>
> Der Abstand misst den Bildschirm, nicht das Spiel: Bei 60 Hz liegt er naturgemäß bei ~16,67 ms und
> reißt die 16,7-ms-Marke ständig um Bruchteile. Gemessen bei der Abnahme von Issue #7: Arbeitszeit
> `max 5,3 ms, over 0` bei 794 Bildern — Abstand im selben Lauf `max 100 ms, over 456`. Wer die
> falsche Reihe nimmt, verwirft eine einwandfreie Umsetzung.
>
> **Vor jedem Leistungsbefund einmal OHNE die geprüfte Sache messen.** Ohne diese Nullmessung
> beschuldigt man das Falsche. Zwei reale Fälle:
>
> | Fall      | Was es schien                                          | Was es war                                                                                                              |
> | --------- | ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
> | Issue #10 | 25,3 ms Arbeitszeit, 178 verworfene Takte              | Zwei gleichzeitige Playwright-Schleifen hungerten die Seite aus. Sauber: 1,2 ms, 0 verworfen                            |
> | Issue #12 | 42 statt 60 Bilder je Sekunde durch die Schadenszahlen | Die Browser-Sitzung war nach ~40 Minuten Dauermessung müde. Der Vergleichslauf **ohne** Treffer zeigte dieselben 45 fps |
>
> **Das erste Fenster nach einem Zustandswechsel ist unbrauchbar.** Die Nullmessung allein genügt
> nicht — erst ein **Wiederholungsfenster im selben Zustand** ist belastbar. Bei Issue #13 gemessen,
> derselbe Kampf dreimal:
>
> | Lauf | Zustand                          | Arbeitszeit max | über 16,7 ms |
> | ---- | -------------------------------- | --------------- | ------------ |
> | A    | ohne Overlay, **erstes Fenster** | 22,9 ms         | 1            |
> | B    | mit Overlay                      | 15,3 ms         | 0            |
> | C    | ohne Overlay, **Wiederholung**   | **4,8 ms**      | 0            |
>
> A und C sind derselbe Zustand. Der Unterschied liegt allein in der Zeit seit dem letzten Wechsel —
> das HTML-Overlay, das zunächst verdächtigt wurde, war unschuldig. Dieselbe Falle wie in § 3.5
> („die erste Messung nach dem Laden ist unbrauchbar"), nur für die Bildzeit.

> **Bei langen Messreihen den Browser neu starten.** Im zweiten Fall lagen in einer frischen Sitzung
> alle vier Varianten bei 60 fps — und ohne die Nullmessung wäre ein Fehler gemeldet worden, den es
> nicht gibt.

> `frameIntervalSummary` ist trotzdem nützlich: Ein hoher `max` dort zeigt Aussetzer der Umgebung
> (Systemlast, headless-Eigenheiten) und erklärt verworfene Takte.

### 3.2 Eingabe-Verzögerung: Wirkung im nächsten Bild

```bash
playwright-cli -s=rota eval "JSON.stringify(window.__rota.perf().inputFrameDelaySummary)"
playwright-cli -s=rota eval "JSON.stringify(window.__rota.perf().inputLatencySummary)"   # Begleitwert
```

**Bestanden:** `inputFrameDelaySummary.over === 0` — **jeder** Tastendruck wirkt im nächsten Bild,
keiner erst im übernächsten. Gemessen wird von `keydown` bis zum Zeichnen des Bildes, in dem die
Wirkung sichtbar ist.

> **Nicht gegen eine Millisekunden-Grenze prüfen.** Die frühere Fassung verlangte „unter 16 ms" —
> das ist in sich widersprüchlich, weil ein Bild bei 60 Hz **16,67 ms** dauert. Die Grenze war also
> strenger als das, was sie beschreiben sollte, und damit auch bei optimaler Umsetzung unerfüllbar.
>
> Bei der Abnahme von Issue #8 gemessen, 50 Drücke: **Bild-Verzögerung min 1, max 1, mean 1, over 0**
> — jeder Druck wirkt im nächstmöglichen Bild. Im selben Lauf: **Millisekunden max 31,1, over 16 ms
> bei einem von 50**. Der Ausreißer entstand, weil ein Bildabstand länger war; die Millisekunden
> messen Bildschirm und Systemlast mit, die Bild-Verzögerung nicht.
>
> Dieselbe Falle wie bei `frameTimes` in § 3.1: Ein Kriterium, das die Umgebung misst statt das
> Spiel, verwirft einwandfreie Arbeit.

Der Millisekunden-Wert bleibt als **Begleitwert** nützlich: Ein hoher `mean` bei `frameDelay = 1`
deutet auf einen zu groben Logik-Takt hin.

### 3.3 Abklingzeiten auf die Millisekunde

```bash
playwright-cli -s=rota eval "window.__rota.replay('cooldown-precision')"
python3 tools/step1-check.py
```

**Bestanden:** Die aufgezeichnete Runde ergibt im Spiel **denselben Schadenswert** wie
`tools/step1-check.py`. Weicht er ab, sagt die Zustands-Prüfsumme, **in welchem Takt** die
Abweichung entstand (GAME-DESIGN 19.1).

Zusätzlich über 100 Zyklen prüfen, ob die Abklingzeit driftet:

```bash
playwright-cli -s=rota eval "window.__rota.measureCooldown('kindle', 100)"
```

**Bestanden:** Abweichung über 100 Zyklen unter 1 ms (ganzzahlige Zeit darf gar nicht driften).

### 3.4 Kampfprotokoll vollständig

```bash
playwright-cli -s=rota eval "window.__rota.log().length"
playwright-cli -s=rota eval "() => window.__rota.log().filter(e=>e.type==='cast').length"
```

**Bestanden:** Die Zahl der protokollierten Wirkungen stimmt mit der Zahl der Tastendrücke aus der
Aufzeichnung überein — **kein Ereignis fehlt**, auch nicht bei 10.000 Einträgen.

### 3.5 Keine Speicheranforderung im laufenden Bild

Voraussetzung: `.playwright/cli.config.json` startet Chrome mit `--js-flags=--expose-gc` und
`--enable-precise-memory-info`. Ohne beides rundet Chrome auf 100 KB, und die Messung ist wertlos.

```bash
# Messung starten, Ergebnis in eine globale Variable (eval kennt kein await)
playwright-cli -s=rota eval "() => { window.__m=null; window.__rota.measureAllocations(600).then(r=>{window.__m=r}); return 1; }"
sleep 14
playwright-cli -s=rota eval "JSON.stringify(window.__m)"
```

**Bestanden: kein Skalieren mit der Bildzahl.** Dreimal messen mit 300, 600 und 1800 Bildern — die
Werte müssen um Null schwanken und dürfen **nicht** proportional zur Bildzahl wachsen.

> **Nicht auf „exakt 0 Byte" prüfen.** Das Verfahren ist dafür zu grob. Bei der Abnahme von Issue #7
> gemessen: 300 Bilder → **−552** Byte, 600 → **−484**, 1800 → **+712**. **Negative Werte** sind der
> Beweis, dass hier Messrauschen des Aufräumers vorliegt und keine Allokation — Speicher kann nicht
> negativ angefordert werden.
>
> Eine echte Allokation von auch nur 10 Byte je Bild ergäbe bei 1800 Bildern 18 KB, monoton positiv,
> und würde beim Verdreifachen der Bildzahl mitwachsen. **Genau darauf wird geprüft.**
>
> Ein Einzelwert von „0" ist kein Nachweis, sondern ein Treffer im Rauschband.

**Die Messung verfälscht die Takt-Zählung** (gefunden bei Issue #11): Die erzwungene Aufräumung
hält die Seite über hundert Millisekunden an; danach holt die Schleife Takte nach und verwirft den
Rest. Eine einzige Speichermessung erzeugte so **10 verworfene Takte**, die wie ein Spielfehler
aussahen.

`GameLoop.discardPendingTime` trennt das jetzt: `perf().discardedForMeasurement` zählt die
Mess-Artefakte getrennt von `droppedTicks`. **Nachgemessen bei der Abnahme:** `droppedTicks` blieb
über eine Speichermessung hinweg unverändert, `discardedForMeasurement` stand bei 6.

> **Folge für § 3.1:** Wer `droppedTicks` nach einer Speichermessung liest, misst seinen Prüfstand.
> Entweder vorher zurücksetzen oder beide Zähler nebeneinander lesen. Die „7 verworfenen Takte", die
> ich bei Issue #7 als Build-Wechsel erklärt habe, waren mit hoher Wahrscheinlichkeit dasselbe.

**Zwei weitere Fallen** (beide bei der Abnahme von Issue #8 aufgetreten):

1. **Die erste Messung nach dem Laden ist unbrauchbar.** Direkt nach dem Öffnen wurden 450 KB auf
   300 Bilder gemessen — reines Aufwärmen (Klang-Dekodierung, JIT). **Vor der Messung 20 bis 25 s
   laufen lassen.** Und die Reihenfolge einmal umdrehen (1800/600/300 statt 300/600/1800): Hängt ein
   Wert an der Bildzahl, bleibt er; hängt er am Zeitpunkt, wandert er mit.
2. **Klang-Wiedergabe erzeugt Speicher je Tastendruck, nicht je Bild.** Gemessen: 156 Byte je Druck.
   Das ist der `AudioBufferSourceNode`, den die Web-Audio-Spezifikation für jede Wiedergabe verlangt
   — nicht vermeidbar und **kein Verstoss**, weil er nicht an der Bildzahl haengt. Bei einer Messung
   unter Tastatur-Beschuss diesen Anteil herausrechnen.

Das ist die härteste der fünf Messungen und der Grund für die Objekt-Pools und den Ringpuffer
(GAME-DESIGN 18.2). Systematische Allokation im Bild-Pfad führt früher oder später zu Rucklern.

---

## 4. Animationen prüfen

Claude kann nicht beurteilen, ob sich etwas _gut anfühlt_ — aber sehr wohl, ob es **technisch
korrekt** läuft: springt nichts, stimmen Timing und Endzustand.

```bash
# 1. Aufzeichnen
playwright-cli -s=rota video-start
# replayLive, NICHT replay: replay() rechnet sofort und ohne Bild durch — die
# Aufnahme zeigte sonst einen unveraenderten Bildschirm.
playwright-cli -s=rota eval "() => window.__rota.replayLive('cooldown-sweep')"
playwright-cli -s=rota video-stop /tmp/anim.webm

# 2. Zerlegen — 1280 px Breite, Qualität 6 (Begründung unten)
mkdir -p /tmp/frames && rm -f /tmp/frames/*.jpg
ffmpeg -v error -i /tmp/anim.webm -vf "fps=30,scale=1280:-2" -q:v 6 /tmp/frames/f_%04d.jpg

# 3. GRÖSSEN-BUDGET PRÜFEN — Pflicht vor dem Ansehen
du -ch /tmp/frames/*.jpg | tail -1
ls /tmp/frames/*.jpg | wc -l
```

**Grenzen:** höchstens **600 Bilder** und höchstens **32 MB Summe** je Durchgang.

**Gemessen** (2026-07-27, an einem synthetischen Testbild — dem ungünstigsten Fall, echte
HUD-Aufnahmen auf dunklem Grund komprimieren deutlich besser):

| Variante                   | je Bild   | hochgerechnet auf 600 | Budget          |
| -------------------------- | --------- | --------------------- | --------------- |
| `scale=1920:-2 -q:v 4`     | 68 KB     | 39,8 MB               | **zu groß**     |
| `scale=1920:-2 -q:v 6`     | 60 KB     | 35,2 MB               | **zu groß**     |
| `scale=1920:-2 -q:v 8`     | 55 KB     | 32,2 MB               | **zu groß**     |
| **`scale=1280:-2 -q:v 6`** | **35 KB** | **20,5 MB**           | **eingehalten** |

> **Warum nicht 1920 px:** Die naheliegende Wahl reißt das Budget. Für die Animationsprüfung geht es
> um Sprünge, Timing und Endzustand — dafür genügt 1280 px mit Reserve. Wer bei einem einzelnen
> Bild wirklich Details braucht, zieht dieses eine gezielt in voller Auflösung.

Dann **alle** Bilder in einem Durchgang mit `Read` ansehen — nicht stichprobenweise. Geprüft wird:

| Prüfpunkt                         | Was schiefgehen kann                          |
| --------------------------------- | --------------------------------------------- |
| Kein Sprung zwischen zwei Bildern | Animation setzt neu an statt zu interpolieren |
| Endzustand stimmt                 | Wischblende bleibt bei 359° statt 360° stehen |
| Timing passt zur Vorgabe          | 700 ms Ausblendung dauert tatsächlich 700 ms  |
| Kein Flackern                     | doppeltes Zeichnen pro Bild                   |

---

## 5. Ergebnis-Formen — es gibt genau zwei

### ABGENOMMEN

Alle ACs mit eigenem Befehl und eigener Ausgabe belegt. Keine Ausnahme, keine Restpunkte.

```markdown
## Issue #<n> — ABGENOMMEN

AC1: <Text> → BESTANDEN
$ <mein Befehl>
<meine Ausgabe>
...
Zusätzlich geprüft: <Standard-Messungen>
```

### OFFEN MIT BLOCKER

Mindestens eine AC nicht belegbar. Das Issue **bleibt offen**.

```markdown
## Issue #<n> — OFFEN MIT BLOCKER

Blockierende AC: <welche>
$ <mein Befehl>
<die Ausgabe, die das Gegenteil zeigt>
Grund: <was fehlt oder falsch ist>
Alle anderen ACs: <Stand>
```

**Dazwischen gibt es nichts.** Kein „größtenteils abgenommen", kein „bis auf Kleinigkeiten fertig",
kein `scope:partial`. Ein Issue mit vier von fünf ACs ist ein offenes Issue.

---

## 5a. Klang und Musik prüfen

Was klingt, kann ich nicht beurteilen (siehe § 6) — **ob** etwas klingt, sehr wohl. Der Bericht
`__rota.sound()` beantwortet drei Fragen: Läuft eine Anforderung ins Leere? Bleibt ein Eintrag des
Verzeichnisses stumm? Schalten die Musik-Schichten an den richtigen Punkten?

```bash
# Klang wacht erst mit einer Nutzergeste auf — ohne Tastendruck bleibt alles stumm
# und `kontext` steht auf "suspended".
playwright-cli -s=rota press Enter

playwright-cli -s=rota eval "() => JSON.stringify(window.__rota.sound().probleme)"      # muss [] sein
playwright-cli -s=rota eval "() => window.__rota.sound().klangFehlend"                   # muss 0 sein
playwright-cli -s=rota eval "() => window.__rota.sound().kontext"                        # "running"
playwright-cli -s=rota eval "() => JSON.stringify(window.__rota.sound().proKlang)"
playwright-cli -s=rota eval "() => JSON.stringify(window.__rota.sound().musik)"
playwright-cli -s=rota eval "() => JSON.stringify(window.__rota.events())"               # 24 Ereignisse
playwright-cli -s=rota eval "() => window.__rota.emit('boss.overpressure')"              # von Hand auslösen
```

| Kennzahl        | Bedeutung                                                              | Erwartet                |
| --------------- | ---------------------------------------------------------------------- | ----------------------- |
| `klangFehlend`  | angeforderte Klänge, die es nicht gibt                                 | **0**                   |
| `probleme`      | Beanstandungen am Verzeichnis (unbekanntes Ereignis, falscher Betreff) | **[]**                  |
| `zugeordnet`    | Einträge im Verzeichnis mit Auslöser                                   | 29                      |
| `ausgeloest`    | davon in dieser Sitzung erklungen                                      | wächst mit den Läufen   |
| `nochNicht`     | Zuordnungen ohne Auslösung, **mit Grund**                              | nur E3/E4-Gründe        |
| `ohneAusloeser` | Einträge ganz ohne Auslöser (Menü, Beute, Sonderzauber)                | 27, alles spätere Epics |

**Ein stummer Eintrag ist kein Fehler, solange der Grund benannt ist.** Acht Klänge hängen an
Ereignissen, die es in Schritt 1 nicht gibt (Boss-Zauber, Phasenwechsel, Spieler-Schaden) — sie
stehen mit Epic-Verweis unter `nochNicht`. Ein Eintrag dort ohne Grund wäre einer.

**Die Musik-Schichten prüft man über die Zeit, nicht als Momentaufnahme.** Ein Beobachter, der
`musik.hoerbar` bei jeder Änderung mitschreibt, zeigt den ganzen Verlauf:

```bash
playwright-cli -s=rota eval "() => { window.__obs=[]; window.__last=''; setInterval(()=>{ const s=__rota.state(); const h=__rota.sound().musik.hoerbar.join('+'); if(h!==window.__last){ window.__last=h; window.__obs.push({t:Math.round(s.fightElapsedMs), hp:__rota.combat().bossHpRemaining, hoerbar:h}); } }, 50); __rota.replayLive('cooldown-precision'); return 1; }"
# 45 s warten, dann:
playwright-cli -s=rota eval "() => JSON.stringify(window.__obs)"
```

Erwarteter Verlauf eines Siegeslaufs (gemessen bei E1-9):

| Kampfzeit | Boss | hörbar                  | Warum                                         |
| --------- | ---- | ----------------------- | --------------------------------------------- |
| Countdown | 4200 | `music-menu`            | Grundklang außerhalb des Kampfes              |
| 30 ms     | 4200 | `music-bed-placeholder` | `fight.start`                                 |
| 33 050 ms | 780  | `+ music-final-push`    | unter 20 % (GAME-DESIGN 10.1)                 |
| 50 040 ms | —    | `+ music-overpressure`  | 10 s Restzeit, dieselbe Schwelle wie der Ring |
| Ende      | 0    | `music-menu`            | `fight.end`, dann `menu.enter`                |

> **Klang wacht nur mit einer Nutzergeste auf.** `eval` allein genügt nicht — der Browser hält den
> Kontext an, bis eine echte Taste kam. Wer ohne Tastendruck misst, sieht `kontext: "suspended"`,
> `wiedergaben: 0` und hält eine funktionierende Anbindung für kaputt.

---

## 6. Was ich ausdrücklich NICHT beurteile

| Nicht meine Zuständigkeit                        | Wer entscheidet                                                                                              |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| **Spielgefühl** — fühlt sich das Drücken gut an? | **der Auftraggeber**                                                                                         |
| **Ästhetik** — sieht das gut aus?                | **der Auftraggeber**                                                                                         |
| **Klang** — klingt das richtig?                  | **der Auftraggeber** (ich kann nicht hören; ich prüfe nur technisch: Länge, Pegel, Anfangs-Stille, Clipping) |
| **Ob eine Design-Entscheidung klug war**         | steht in `GAME-DESIGN.md` und ist getroffen                                                                  |

> Bei diesen vier Punkten ist meine Aufgabe, **die Voraussetzungen zu schaffen** — dass der
> Auftraggeber etwas Lauffähiges vor sich hat und dass alles Messbare gemessen ist. Nicht, ein Urteil
> zu fällen, das mir nicht zusteht.

---

## 7. Häufige Fehler bei der Abnahme — an mich selbst

| Fehler                                       | Warum er passiert                                             | Gegenmittel                                                                                                                                  |
| -------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **Self-Grading**                             | „Der Code sieht korrekt aus" fühlt sich wie Prüfen an         | Kein Befehl = ungeprüft. Ausnahmslos                                                                                                         |
| **Schwellwert anpassen**                     | Die Messung liegt knapp daneben, die Grenze wirkt willkürlich | Die Grenze steht im Design. Der Code wird angepasst, nicht die Grenze                                                                        |
| **Nachlassende Strenge**                     | Bei Issue 40 wirkt vieles „offensichtlich"                    | Issue 40 bekommt exakt dasselbe Verfahren wie Issue 1                                                                                        |
| **Alte Nachweise wiederverwenden**           | Spart Zeit                                                    | Der Code hat sich geändert. Neu messen                                                                                                       |
| **`browser`-Nachweise gegen den Dev-Server** | Dort funktioniert mehr als im Auslieferungs-Bau               | Jeder `browser`-Nachweis gilt nur im **gebauten** Stand über nginx. Bei Issue #16 lieferte die Route HTTP 200 und trotzdem die falsche Seite |
| **Eine URL weitergeben, ohne sie zu öffnen** | Die Route antwortet, der Worker hat es geprüft                | Selbst aufrufen. Bei #16 habe ich genau das versäumt — der Auftraggeber bekam eine schwarze Seite                                            |
| **Dem Melde-Format des Workers glauben**     | Es sieht sauber aus und hat Befehle drin                      | Es sind **seine** Befehle. Ich führe **meine** aus                                                                                           |
| **Schaden messen, wenn der Boss stirbt**     | „Schaden" klingt nach der Frage, die man gestellt hat         | Bei `stopOnKill` ist der Gesamtschaden ungefähr die Boss-HP — **unabhängig von der Spielerstärke**. Siehe unten                              |
| **Einzelmessung nach einem Tastendruck**     | Der Befehl ist zurückgekommen, also ist er verarbeitet        | `playwright-cli press` läuft nicht im Takt der Logik. Fünfmal messen und den eingeschwungenen Wert nehmen                                    |

### `cargo test` bricht nach der ersten gefallenen Sammlung ab

_Von Worker-2 am 2026-08-05 gefunden, nachdem er eine halbe Stunde einen Widerspruch eingekreist
hatte, den es nicht gab._

Er fuhr eine Gegenprobe über die volle Suite. Seine Konstanten-Prüfung fiel **zuerst** — und die
Sammlung, die er eigentlich messen wollte, **lief danach gar nicht mehr**. Er las die grünen Zeilen
des vorigen Laufs und hielt sie für das Ergebnis.

> **„Grün" hieß hier nicht „geprüft und bestanden", sondern „nicht ausgeführt".**

Ohne einen Widerspruch daneben hätte er vier grüne Fälle als Nachweis gemeldet, **die nie gelaufen
waren**.

**Für Gegenproben heißt das:**

```bash
cargo test --test <name>          # die betroffene Sammlung EINZELN
cargo test --all --no-fail-fast   # oder alle, ohne Abbruch
```

Eine Gegenprobe über `cargo test --all` **ohne** `--no-fail-fast` liefert eine **Untergrenze**, keine
Zahl: Alles nach der ersten gefallenen Sammlung ist unbekannt. Die Aussage „genau ein Fall wird rot"
ist damit nicht belegt — sie könnte „einer und alles danach" heißen.

> `npx vitest run` verhält sich anders: Es fährt alle Dateien und meldet `3 failed | 1246 passed`.
> Dort ist die Zahl belastbar.

**Und beim Zählen der roten Fälle nicht `grep -c` auf das Muster ansetzen:** `^test .*FAILED` trifft
auch die Ergebniszeile `test result: FAILED` und zählt eine Sammlung als zusätzlichen Fall. Beim
Nachmessen dieser Regel selbst passiert — die Namen auflisten, nicht die Zeilen zählen.

### Die Kill-Falle — an einem Tag zweimal zugeschlagen

**Wer misst, ob eine Verstärkung wirkt, darf nicht den Gesamtschaden nehmen, solange der Lauf mit
dem Tod des Bosses endet.** Wer härter schlägt, überschießt beim letzten Treffer weniger; die Summe
ist dann fast genau die Boss-HP, und ein stärkerer Spieler kann sogar **weniger** messen.

```
ESSENCE 500 gegen ohne:   4230 gegen 4240     ← sieht nach "wirkt nicht" aus
```

Beide Male sah es aus wie ein Befund an der Umsetzung, beide Male war es die Messung:

|                |                                                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------------------------------- |
| **#237, AC1b** | Zwei Bösse mit identischer Überdruck-Zeit sollten verschiedene Rotationen zeigen. `test-metronome` (300 s) löste es |
| **#282, AC2**  | „ESSENCE ändert den Schaden nicht" — gemessen wurde der Überschuss beim Todesstoß                                   |

**Zwei Wege heraus, je nach Frage:**

- **Die Zeit messen** statt den Schaden: `killedAtMs`. +50 % Schaden drücken sie auf etwa zwei
  Drittel — das ist eine Aussage über die Stärke.
- **`stopOnKill: false`** setzen und über eine feste Dauer messen. Dann ist die Summe wieder eine
  Aussage über den Schaden.

`BalanceConfig` warnt wörtlich davor: **`stop_on_kill` misst, OB der Boss fällt, nicht WIE VIEL
Schaden entsteht.** Der Satz stand da und ist beide Male gelesen worden — er hilft nur, wenn man
ihn beim **Formulieren der Messung** liest, nicht beim Deuten des Ergebnisses.

<!-- Prüflauf Pfad-Filter, 2026-07-27T15:29:45Z -->

### `git merge-tree` mit drei Argumenten misst etwas anderes

Am 2026-08-05 habe ich einem Worker **vier Konflikte** gemeldet. Es war einer.

```bash
git merge-tree $(git merge-base origin/main origin/$B) origin/main origin/$B \
  | grep "changed in both"
```

Der Befehl lief, die Ausgabe war eindeutig, der Stand war frisch gefetcht — und die
Zahl trotzdem falsch. **Die Drei-Argument-Form ist der deprecated
`--trivial-merge`-Modus.** Aus der Handbuchseite:

> outputs trivial merge results **and** conflicting stages

Sie listet also, was auf **beiden Seiten berührt** wurde, ohne zu unterscheiden, ob
git es zusammenführen kann. Ein trivialer Merge kann eine beidseitig geänderte Datei
grundsätzlich nicht auflösen; die `ort`-Strategie des echten Merges kann es, solange
sich die Zeilen nicht überlappen.

**Der Befehl, der Konflikte misst:**

```bash
git merge-tree --write-tree --name-only origin/main origin/<zweig> >/dev/null
echo $?        # 0 = sauber, 1 = Konflikt (Namen stehen dann im Kopf der Ausgabe)
```

> **Warum das die erste Gestalt ist und nicht die fünfte.** Der Worker vermutete, ich
> hätte gegen einen veralteten Stand gemessen — und weil wir an diesem Tag beide
> genau diesen Fehler gemacht hatten, lag die Erklärung nahe. Sie war falsch. Es war
> **das falsche Werkzeug**: ein Befehl, dessen Ausgabe ich gelesen habe, ohne zu
> prüfen, was er misst.
>
> Eine Fehlerart, die man gerade oft gesehen hat, drängt sich als Erklärung für die
> nächste auf. Die Zuordnung gehört genauso gemessen wie die Zahl.

### Ein Prüfbefehl, der bei einem Fehler grün meldet, ist schlimmer als keiner

Beim Bauen des AC6-Prüfbefehls für #268 ist mir dasselbe **zweimal in zehn Minuten**
passiert:

|                 | Was ich meldete                          | Was tatsächlich lief                                                                                                   |
| --------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| erster Versuch  | „15 Dateien: nur Prüfsummen"             | `jq` stürzte ab (`Cannot iterate over null`), **beide** Seiten lieferten leere Ausgabe — und leer verglich sich gleich |
| zweiter Versuch | „OK — kein bestehender Wert bewegt sich" | `jq` warf bei **jeder** Datei `Path must be specified as an array`                                                     |

Ursache beim ersten Mal: Es liegen **zwei Aufzeichnungs-Formate** nebeneinander —
`fixation.json` hat `runs[]`, `cast-time.json` ist flach. Ein Filter, der `.runs`
voraussetzt, stirbt bei der Hälfte.

**Regel:** Jeder Vergleichsbefehl bricht bei einem Fehler des Unterprogramms mit
eigenem Exit-Code ab.

```bash
a=$(git show HEAD:"$p" | jq -S "$FILTER") || { echo "ABBRUCH $p"; exit 2; }
```

Dieselbe Klasse wie ein `all()` über eine leere Liste: **Der Vergleich zweier
Nichtse ist immer wahr.** Sie sieht jedes Mal anders aus und ist jedes Mal dieselbe.

### Ein Kriterium, das eine Datei nennt, wird beim SCHREIBEN geprüft

Am 2026-08-05 stand in einem Acceptance Criterion, das ich selbst formuliert hatte:

> …und dem Hinweis, dass `tools/ac6.sh` heute sichtbar macht, was sie damals verhindern sollte

```
$ ls tools/              18 Dateien, kein ac6.sh
$ find . -name "ac6*"    nichts
```

Das Werkzeug existierte — in einem **fremden, offenen Zweig**. Für den Werker, der das Kriterium
erfüllen sollte, ist das dasselbe wie „existiert nicht": Er kann es nicht belegen, und das Issue
hängt plötzlich an einem anderen.

**Warum dieser Verweis-Fehler schwerer wiegt als die drei anderen desselben Tages** (`merge-tree`
mit drei Argumenten, `playwright-cli screenshot <pfad>`, `data.ts:1362`): Die standen in Nachrichten
und in einem Issue-Text — dort liest sie jemand und stolpert.

> **Ein Kriterium wird abgehakt, nicht gelesen.** Abgehakt hätte ich es selbst, bei der Abnahme,
> weil der geforderte Abschnitt ja dastand.

Gefunden hat es der Werker, weil er das Werkzeug **suchte, bevor er darauf verwies** — obwohl drei
plausible Kandidaten in `tools/` lagen und sich einer mit gutem Willen passend hätte erzählen lassen.

**Beim Schreiben eines Issues, nicht beim Abnehmen:**

```bash
ls <jede im Kriterium genannte Datei>
git ls-tree origin/main -- <pfad>      # liegt sie auf main, oder nur in einem Zweig?
```

Ein Kriterium darf nur auf Dinge zeigen, die auf `main` stehen oder im selben PR entstehen.

### Vier eigene Fehler in einer Nacht — alle beim Messen, keiner beim Urteilen

_2026-08-06. Die Urteile waren am Ende richtig; falsch war jeweils der Weg dorthin, und in drei
von vier Fällen hätte er zu einem falschen Urteil geführt._

#### 1. Der CI-Status stand am falschen Commit

`gh pr view <n> --json mergeStateStatus,statusCheckRollup` meldete `CLEAN|SUCCESS`, der Merge
scheiterte trotzdem mit _„Required status check ci-pass is expected"_. Der Zustand gehörte zum
Commit **vor** `update-branch`; der neue Lauf hatte noch nicht begonnen.

**Der Status eines PR ist keine Aussage über seinen heutigen Kopf.** Nach jedem
`gh pr update-branch` wird gegen den Commit gemessen, nicht gegen den PR:

```bash
N=$(gh pr view <n> --json headRefOid -q .headRefOid)
gh api repos/<org>/<repo>/commits/$N/check-runs \
  -q '.check_runs[]|select(.name=="ci-pass")|.head_sha[0:8]+" "+.conclusion'
```

Die Ausgabe nennt den SHA — man sieht also, worüber sie spricht. Ein `SUCCESS` ohne SHA daneben
ist nicht prüfbar.

#### 2. `parity:export` vor dem Test gefahren

Um zu messen, ob eine falsche Platzangabe auffällt, lief erst der Export und dann der Prüflauf:
**64 grün** — der Export hatte die falsche Zahl längst zur Vergleichsgröße gemacht. In der
richtigen Reihenfolge fällt derselbe Fall sofort.

> **Wer exportiert, bevor er prüft, misst sein eigenes Ergebnis.** Die Vergleichsdatei ist kein
> Zeuge mehr, sobald man sie neu geschrieben hat.

Daraus folgt für jedes Kriterium der Form _„`parity:export` bewegt keine Datei"_: Es belegt nur
etwas, wenn **vor** jeder Änderung gemessen wurde. Sonst ist die Unbewegtheit trivial.

#### 3. Gegen einen zwei Merges alten Arbeitsbaum gemessen

In `payloads.ts` stand noch der `?? 0`-Rückfall, um den es in einem gerade abgenommenen Issue
ging — beinahe als „die Arbeit ist unvollständig" gemeldet. Der lokale Stand war zwei Merges
alt; nach `git pull` war die Stelle weg.

Die Abnahme selbst lief im Worktree und damit gegen den richtigen Stand. **Gefährlich war nicht
die Abnahme, sondern der beiläufige Blick danach** — der greift zum Hauptverzeichnis, und das
ist genau der Stand, den niemand pflegt.

```bash
git pull --ff-only && git log --oneline -1     # vor jeder Aussage über „steht da noch"
```

#### 4. Eine Prüfung gebaut, die nicht lief

Eine Typ-Probe sollte belegen, dass der Übersetzer ein fehlendes Feld meldet. Sie schwieg auf
**beiden** Ständen — gelesen als „übersetzt". Sie lag in der Repository-Wurzel, und
`tsconfig.json` hat `include: ["src", "tests", "vite.config.ts"]`. Sie wurde nie übersetzt.

> **Bei einer Prüfung, die bricht, wenn sie greift, ist Schweigen der Erfolgsfall** — und damit
> im Ausgabetext nicht vom Ausfall zu unterscheiden.

Deshalb gehört zu jeder brechenden Prüfung eine zweite, die sie absichtlich bricht. Nicht als
Formalie: Es ist der einzige Weg, das Schweigen zu deuten. Dasselbe gilt für Suchbefehle in
Kriterien — _„die Suche findet nichts"_ belegt erst dann etwas, wenn gezeigt wurde, dass sie
etwas finden **kann**.

#### Was die vier gemeinsam haben

Keiner war ein Denkfehler über die Sache. Alle vier waren Aussagen darüber, **worauf sich eine
Messung bezieht**: auf welchen Commit, auf welche Datei, auf welchen Stand, auf welchen
Übersetzungslauf. Und alle vier sahen im Ausgabetext genau so aus wie das richtige Ergebnis.

Die Gegenmaßnahme ist keine Regel, sondern eine Gewohnheit: **Der Befehl nennt seinen
Bezugspunkt mit.** Ein SHA in der Ausgabe, ein `git log -1` davor, ein absichtlich gebrochener
Fall daneben. Das kostet Sekunden und macht die Verwechslung unmöglich statt unwahrscheinlich.

### Ein Messpunkt hinter der Wirkung sieht aus wie keine Wirkung

_Gefunden von Worker-2 am 2026-08-06 an sich selbst, vom Orchestrator nachgemessen._

Die Frage war, ob ein Feld in `WornValues` die Zustands-Prüfsumme berührt. Der erste Anlauf
maß **alle 1500 ms** und meldete: `CATALYSIS` bewegt sie nicht. Das wäre als Befund
durchgegangen — und es war falsch.

`CATALYSIS` verkürzt die Abklingzeit von 1500 ms auf 1160 ms. Beide Werte liegen **unter**
dem Messabstand: Zum Messzeitpunkt stand `cooldownRemaining` in beiden Läufen längst auf 0.
Der Unterschied war nicht weg, weil es keinen gab, sondern weil hinter ihn gemessen wurde.

```
Takt 0                     gleich       die Abklingzeit läuft noch nicht
direkt nach dem Cast       verschieden  1500 gegen 1160
5000 ms nach dem Cast      gleich       beide abgelaufen
```

Alle drei Zeilen sind wahr, und zwei davon würden „keine Wirkung" belegen.

#### Zwei Werte, zwei verschiedene Messpunkte

| Wert        | wirkt sich aus                     | weil                                                                       |
| ----------- | ---------------------------------- | -------------------------------------------------------------------------- |
| `VESSEL`    | **schon bei Takt 0**               | geht in `playerHp` ein, und das steht in der Prüfsumme (`checksum.ts:140`) |
| `CATALYSIS` | **erst mit laufender Abklingzeit** | verkürzt Zeiten, und eine abgelaufene Zeit ist in beiden Fällen 0          |

**Ein Prüffall, der pauschal „nach dem Kampf" misst, verfehlt beide** — den einen, weil der
Unterschied im Anfangszustand liegt, den anderen, weil er nur währenddessen sichtbar ist.

#### Was daraus folgt

Vor jeder Messung gehört die Frage beantwortet: **Wann ist die Wirkung sichtbar, und misst mein
Messpunkt dort?** Eine grüne Zeile beantwortet sie nicht — sie ist mit „keine Wirkung", „noch
nicht" und „nicht mehr" gleichermaßen vereinbar.

Der billigste Weg dorthin ist, die **Ursache mitzumessen** statt nur das Ergebnis:

```
GCD          1500 gegen 1500     unberührt
Schaden       100 gegen  100     unberührt
Abklingzeit  1500 gegen 1160     ← der Hebel
```

Damit steht nicht nur, **dass** sich etwas unterscheidet, sondern **woran** es liegt — und man
sieht sofort, wo der Messpunkt liegen muss.
