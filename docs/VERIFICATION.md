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

Aus `GAME-DESIGN.md` 20.2. Sie gelten für jedes Issue, das den Kampf berührt.

### 3.1 Ruckelfreiheit

```bash
playwright-cli -s=rota eval "window.__rota.perf().frameTimes.slice(-600)" > /tmp/frames.json
python3 -c "
import json; f=json.load(open('/tmp/frames.json'))
over=[x for x in f if x>16.7]
print(f'Bilder: {len(f)}  ueber 16,7 ms: {len(over)} ({100*len(over)/len(f):.2f} %)')
print(f'max {max(f):.1f} ms   p99 {sorted(f)[int(len(f)*0.99)]:.1f} ms')
"
```

**Bestanden:** kein Bild über 16,7 ms in 600 aufeinanderfolgenden Bildern (10 s bei 60 Hz).

### 3.2 Eingabe-Verzögerung unter 16 ms

```bash
playwright-cli -s=rota eval "window.__rota.perf().inputLatency"
```

**Bestanden:** Maximum unter 16 ms. Gemessen wird von `keydown` bis zum Zeichnen des Bildes, in dem
die Wirkung sichtbar ist — **nicht** bis zur Verarbeitung im Logik-Takt.

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
playwright-cli -s=rota eval "window.__rota.log().filter(e=>e.type==='cast').length"
```

**Bestanden:** Die Zahl der protokollierten Wirkungen stimmt mit der Zahl der Tastendrücke aus der
Aufzeichnung überein — **kein Ereignis fehlt**, auch nicht bei 10.000 Einträgen.

### 3.5 Keine Speicheranforderung im laufenden Bild

```bash
playwright-cli -s=rota eval "window.__rota.perf().allocationsPerFrame"
```

**Bestanden:** 0 nach dem Aufwärmen. Das ist die härteste der fünf Messungen und der Grund für die
Objekt-Pools und den Ringpuffer (GAME-DESIGN 18.2). Ein Wert über 0 bedeutet, dass irgendwo im
Bild-Pfad ein Objekt erzeugt wird — das führt früher oder später zu Rucklern.

---

## 4. Animationen prüfen

Claude kann nicht beurteilen, ob sich etwas _gut anfühlt_ — aber sehr wohl, ob es **technisch
korrekt** läuft: springt nichts, stimmen Timing und Endzustand.

```bash
# 1. Aufzeichnen
playwright-cli -s=rota video-start
playwright-cli -s=rota eval "window.__rota.replay('cooldown-sweep')"
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

| Fehler                                   | Warum er passiert                                             | Gegenmittel                                                           |
| ---------------------------------------- | ------------------------------------------------------------- | --------------------------------------------------------------------- |
| **Self-Grading**                         | „Der Code sieht korrekt aus" fühlt sich wie Prüfen an         | Kein Befehl = ungeprüft. Ausnahmslos                                  |
| **Schwellwert anpassen**                 | Die Messung liegt knapp daneben, die Grenze wirkt willkürlich | Die Grenze steht im Design. Der Code wird angepasst, nicht die Grenze |
| **Nachlassende Strenge**                 | Bei Issue 40 wirkt vieles „offensichtlich"                    | Issue 40 bekommt exakt dasselbe Verfahren wie Issue 1                 |
| **Alte Nachweise wiederverwenden**       | Spart Zeit                                                    | Der Code hat sich geändert. Neu messen                                |
| **Dem Melde-Format des Workers glauben** | Es sieht sauber aus und hat Befehle drin                      | Es sind **seine** Befehle. Ich führe **meine** aus                    |

<!-- Prüflauf Pfad-Filter, 2026-07-27T15:29:45Z -->
