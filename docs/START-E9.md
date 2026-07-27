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

| #   | Datei                              | Warum                                                         |
| --- | ---------------------------------- | ------------------------------------------------------------- |
| 1   | `CLAUDE.md`                        | Rahmenbedingungen                                             |
| 2   | `docs/GAME-DESIGN.md` 9.3 und 17.2 | Farbcodierung, Grafik-Produktion, Bau-Reihenfolge             |
| 3   | `ASSETS.md`                        | Die Lizenzregel — und der offene Punkt zu den Referenzbildern |
| 4   | `docs/WORKER-GUIDE.md`             | Der verbindliche Ablauf je Issue                              |

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

## Offener Punkt, den du kennen musst

**Die zwei HUD-Referenzbilder fehlen.** Sie sind in `GAME-DESIGN.md` 17.2 beschrieben
(Reaktionsgefäß, Messring, „CASCADE FAILURE"-Zauberbalken, Pergament-Ausgabefeld, Apparatur), aber
auf dem System nicht auffindbar — eine systemweite Suche blieb ergebnislos. Vermutlich lagen sie nur
als Anhang im Gesprächsverlauf.

**Folge für dich:** #19 (Stil festlegen) braucht sie. Frag den Auftraggeber danach, bevor du dort
anfängst. Die übrigen drei Issues kannst du unabhängig davon bearbeiten.

Was vorhanden ist: `docs/refs/frame-alpha-reference.png` — der Nachweis, dass `gpt-image` brauchbares
Alpha liefert (84,9 % transparent, 8,2 % deckend, 6,84 % halbtransparent). Daran sind auch die zwei
Nachbearbeitungs-Schritte gemessen, die #20 bauen muss.

## Die Lizenzregel

**Nur CC0 und CC BY.** „Free for personal use" ist ausgeschlossen — kostenlos, aber kommerziell nicht
nutzbar, und der Download-Knopf heißt trotzdem „Free Download". Im Zweifel nicht verwenden.

## Wie du meldest

`docs/WORKER-GUIDE.md` § 6.
