# Startbriefing — Epic E8: Klang

> **Diesen Text in die Klang-Session kopieren.** Läuft parallel zu E1 und E9, wartet auf nichts.

## Dein Auftrag

Du baust **Epic E8 — Klang**: die Web-Audio-Einbindung und alle rund 115 Klänge.

**Warum du sofort startest, obwohl der Kampf noch nicht steht:** Das Gefühl beim Drücken hängt zu
einem erheblichen Teil am Klang. Ein stummer Prototyp fühlt sich immer träge an, egal wie gut die
Technik ist — und genau dieses Gefühl ist die Frage, die Schritt 1 beantworten soll. Die drei bis
vier Kernklänge müssen fertig sein, wenn E1 seinen Kampf abgibt.

## Zuerst lesen

| #   | Datei                      | Warum                                                              |
| --- | -------------------------- | ------------------------------------------------------------------ |
| 1   | `CLAUDE.md`                | Rahmenbedingungen, besonders zum Klang                             |
| 2   | `docs/GAME-DESIGN.md` 17.1 | Die Aufstellung aller 115 Klänge und die Musik-Schichten           |
| 3   | `docs/suno-sfx.md`         | Die fertige Werkzeugkette — Prompt-Regeln, Längen, Nachbearbeitung |
| 4   | `docs/WORKER-GUIDE.md`     | Der verbindliche Ablauf je Issue                                   |

## Dein Arbeitsplatz

```bash
cd /work/rota-worktrees/e8
git fetch origin && git switch wt-e8 && git reset --hard origin/main
```

**Dein Dateibereich:** `src/audio/`, `assets/audio/`, `data/sound-manifest.json`.

## Deine vier Issues

| #       | Issue                                                                  |
| ------- | ---------------------------------------------------------------------- |
| **#15** | E8-1: Web-Audio-Einbindung, Pool, Tonhöhen-Streuung, Lautstärke-Regler |
| **#16** | E8-2: Sound-Manifest (115 Slots) und Erzeugung Gruppe 1 (Zauber)       |
| **#17** | E8-3: Klang-Gruppe 2 — Boss, Zustände, Menü, Beute                     |
| **#18** | E8-4: Musik-Schichten je Instanz                                       |

**Vorziehen:** Die fünf Klänge aus Schritt 1 (KINDLE, SMOULDER, PYRE, Fehleingabe, Treffer) sind
Teil von #16, werden aber am dringendsten gebraucht. Mach sie zuerst.

## Zwei Dinge, die nicht verhandelbar sind

1. **Nur das Suno-Konto `backupslave`.** `tools/suno_api.py` erzwingt das über einen Konto-Wächter.
2. **Die ästhetische Abnahme macht der Auftraggeber.** Claude kann nicht hören. Du prüfst technisch
   mit `ffprobe`: Länge, Pegel, Anfangs-Stille, Clipping. Für das Urteil baust du ein
   Abhör-Soundboard.

## Wie du meldest

`docs/WORKER-GUIDE.md` § 6.
