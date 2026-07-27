# Startbriefing — Epic E9: Grafik-Assets

> **Diesen Text in die Grafik-Session kopieren.** Läuft parallel zu E1 und E8, wartet auf nichts.

## Dein Auftrag

Du baust **Epic E9 — Grafik-Assets**: freie Assets auswählen, Zauber-Symbole erzeugen, Rahmen und
Texturen.

**Die wichtigste Einschränkung vorweg:** Claude kann keinen eigenen visuellen Stil hervorbringen.
Referenzbilder erkennen — ja. Selbst gestalten — nein. Deshalb gilt strikt diese Reihenfolge:

1. **Fertige freie Assets zuerst** — der Stil kommt von einem menschlichen Gestalter
2. **KI-Erzeugung nur für Lücken**
3. **Selbst zeichnen: nie**

## Zuerst lesen

| #   | Datei                              | Warum                                             |
| --- | ---------------------------------- | ------------------------------------------------- |
| 1   | `CLAUDE.md`                        | Rahmenbedingungen                                 |
| 2   | `docs/GAME-DESIGN.md` 9.3 und 17.2 | Farbcodierung, Grafik-Produktion, Bau-Reihenfolge |
| 3   | `ASSETS.md`                        | Die Lizenzregel und der Bestand                   |
| 4   | `docs/WORKER-GUIDE.md`             | Der verbindliche Ablauf je Issue                  |

## Dein Arbeitsplatz

```bash
cd /work/rota-worktrees/e9
git fetch origin && git switch wt-e9 && git reset --hard origin/main
```

**Dein Dateibereich:** `assets/img/`, `assets/fonts/`, `ASSETS.md`.

## Deine vier Issues

| #       | Issue                                                               |
| ------- | ------------------------------------------------------------------- |
| **#19** | E9-1: Freie Assets recherchieren, Stil festlegen, ASSETS.md pflegen |
| **#20** | E9-2: Symbol-Erzeugung mit Alphakanal, Ausbluten, Zuschneiden       |
| **#21** | E9-3: Ähnlichkeits-Prüfung aller Symbole                            |
| **#22** | E9-4: Rahmen, Panels und Ringtextur                                 |

## Es gibt bewusst kein HUD-Referenzbild

**Das ist keine Lücke, sondern eine Entscheidung.** Du fängst bei #19 mit einem leeren Blatt an, und
das ist so gewollt.

Ein erzeugtes HUD-Bild wurde am 2026-07-27 zur Probe angefertigt und vom Auftraggeber **abgelehnt**
— Messing, Nieten, vergilbtes Papier, wörtlich „austauschbar". Es liegt bewusst nicht im
Repository. **Frag nicht danach und erzeuge kein neues als Ausgangspunkt.** Eine Richtungsvorgabe,
die niemand will, ist schlechter als keine.

Der Stil kommt **von einem menschlichen Gestalter über fertige freie Assets** (GAME-DESIGN 17.2).
Deshalb heißt dein erstes Issue „Assets recherchieren und Stil **festlegen**", nicht „Stil
generieren". Deine Aufgabe bei #19: echte Kandidaten-Sets zusammenstellen und dem Auftraggeber
**nebeneinander** vorlegen — er entscheidet an dem, was er sieht.

**Was schon feststeht und was du nicht neu erfinden musst:**

|               |                                                                                                                                                                                                                                                                  |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Anordnung** | Ring in der Bildmitte, Zauberleiste unten links im Tastatur-Raster mit sichtbarem Umschalt-Block, Instrumente rechts, Überdruck oben links, Cast-Balken darunter (GAME-DESIGN 9.1) — geprüft und bestätigt                                                       |
| **Farben**    | Kalzination rot-orange, Fixierung weiß-blau, Sublimation gelb-gold, Koagulation grün-türkis, Putrefaktion schwarz-violett (9.3)                                                                                                                                  |
| **Technik**   | `docs/refs/frame-alpha-reference.png` — **kein Stil-Vorbild**, sondern der Nachweis, dass `gpt-image` brauchbares Alpha liefert (84,9 % transparent, 8,2 % deckend, 6,84 % halbtransparent), und die Messgrundlage für die zwei Nachbearbeitungs-Schritte in #20 |

**Drei Fallen, die der Probelauf gezeigt hat** — sie kosten dich sonst eine Runde:

1. **Symbole werden zu Klischees** (Feuerball, Schneeflocke, Totenkopf) und dadurch untereinander
   verwechselbar. Genau dagegen gibt es #21.
2. **Wischblende und GCD-Ring fehlen leicht** — das sind die zwei wichtigsten Anzeigen im ganzen HUD.
3. **Erfundene Werte schleichen sich ein.** Es gibt ESSENCE, VESSEL, WARD, CATALYSIS, PURITY, YIELD.
   Sonst nichts.

## Die Lizenzregel

**Nur CC0 und CC BY.** „Free for personal use" ist ausgeschlossen — kostenlos, aber kommerziell nicht
nutzbar, und der Download-Knopf heißt trotzdem „Free Download". Im Zweifel nicht verwenden.

## Wie du meldest

`docs/WORKER-GUIDE.md` § 6.
