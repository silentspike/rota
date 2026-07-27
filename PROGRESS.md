# Plan-Umsetzung Progress — ROTA Infrastruktur

## Status: IN PROGRESS

**Plan:** `/home/jan/.claude/plans/greedy-skipping-russell.md` (SSOT der Planung, 1856 Zeilen)
**Umfang dieser Umsetzung:** Teil E.5 — der Infrastruktur-Aufbau, bis der erste Worker (E1) starten kann.
Der Spielinhalt selbst gehört den Worker-Sessions und ist **nicht** Teil dieser Liste.

**Tasks gesamt:** 13
**Tasks done:** 0
**Tasks blocked:** 0

---

## Reihenfolge

Optimiert nach Dateibereichen: erst alle Dokumente (ein Schreibvorgang je Datei), dann das Repository
mit den Dokumenten im ersten Push, dann Werkzeugkette, zuletzt Übergabe.

| # | Task | Status | ACs | Commit |
|---|------|--------|-----|--------|
| 12 | `docs/GAME-DESIGN.md` schreiben (SSOT der Umsetzung) | PENDING | 0/5 | – |
| 13 | Schritt-1-Spezifikation mit konkreten Zahlen | PENDING | 0/5 | – |
| 14 | Referenzbilder ablegen, `ASSETS.md` anlegen | PENDING | 0/5 | – |
| 16 | `docs/WORKER-GUIDE.md` — Ablauf je Issue | PENDING | 0/5 | – |
| 17 | `docs/VERIFICATION.md` — Abnahme-Checkliste | PENDING | 0/5 | – |
| 1 | GitHub-Repository einrichten | PENDING | 0/6 | – |
| 2 | CI-Workflows mit `ci-pass`-Aggregat | PENDING | 0/6 | – |
| 3 | Vite-/TypeScript-Gerüst | PENDING | 0/6 | – |
| 15 | nginx-Route `localhost:8000/rota/` | PENDING | 0/5 | – |
| 19 | Worktree-Struktur (E1, E8, E9) | PENDING | 0/5 | – |
| 18 | GitHub-Issues für E1, E8, E9 (16 Stück) | PENDING | 0/5 | – |
| 20 | Startbriefing + Übergabe-Prüfung mit Test-PR | PENDING | 0/6 | – |
| 21 | Plan-Verifikation | PENDING | 0/6 | – |

**ACs gesamt: 70**

---

## Abhängigkeiten

```
12 ──> 13 ──┐
1  ─────────┼──> 18 ──┐
1  ──> 2 ───────────  ┼──> 20 ──> 21
1  ──> 3 ──> 15 ──────┤
1  ──> 3 ──> 19 ──────┤
16, 17, 14 ───────────┘
```

Sofort startbar: **12, 14, 16, 17, 1**

---

## Ausgangslage (geprüft 2026-07-27)

| Punkt | Befund |
|---|---|
| `/work/rota` | git-Repository, Branch `main`, **kein Remote** — Pull entfällt |
| Letzter Commit | `de5cdb6` — Suno-Werkzeugkette, Konto-Wächter |
| Vorhanden | `CLAUDE.md`, `.claude/`, `docs/suno-sfx.md`, `tools/` (3 Skripte) |
| Fehlt | Alles unter `src/`, `package.json`, `.github/`, `docs/GAME-DESIGN.md` |
| Enforcement-Hooks | in `/work/rota/.claude/settings.json` registriert, Permissions unangetastet |
| Auffälligkeit | `.playwright-cli/*.log` wird derzeit mitversioniert → gehört in `.gitignore` (Task 1) |

---

## Blocker

(keine)

---

## Findings

(keine)
