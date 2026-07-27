# Plan-Umsetzung Progress — ROTA Infrastruktur

## Status: IN PROGRESS

**Plan:** `/home/jan/.claude/plans/greedy-skipping-russell.md` (SSOT der Planung, 1856 Zeilen)
**Umfang dieser Umsetzung:** Teil E.5 — der Infrastruktur-Aufbau, bis der erste Worker (E1) starten kann.
Der Spielinhalt selbst gehört den Worker-Sessions und ist **nicht** Teil dieser Liste.

**Tasks gesamt:** 13
**Tasks done:** 2
**Tasks blocked:** 0

---

## Reihenfolge

Optimiert nach Dateibereichen: erst alle Dokumente (ein Schreibvorgang je Datei), dann das Repository
mit den Dokumenten im ersten Push, dann Werkzeugkette, zuletzt Übergabe.

| #   | Task                                                 | Status   | ACs | Commit    |
| --- | ---------------------------------------------------- | -------- | --- | --------- |
| 12  | `docs/GAME-DESIGN.md` schreiben (SSOT der Umsetzung) | **DONE** | 5/5 | `c1794ed` |
| 13  | Schritt-1-Spezifikation mit konkreten Zahlen         | **DONE** | 5/5 | `2d233ee` |
| 14  | Referenzbilder ablegen, `ASSETS.md` anlegen          | PENDING  | 0/5 | –         |
| 16  | `docs/WORKER-GUIDE.md` — Ablauf je Issue             | PENDING  | 0/5 | –         |
| 17  | `docs/VERIFICATION.md` — Abnahme-Checkliste          | PENDING  | 0/5 | –         |
| 1   | GitHub-Repository einrichten                         | PENDING  | 0/6 | –         |
| 2   | CI-Workflows mit `ci-pass`-Aggregat                  | PENDING  | 0/6 | –         |
| 3   | Vite-/TypeScript-Gerüst                              | PENDING  | 0/6 | –         |
| 15  | nginx-Route `localhost:8000/rota/`                   | PENDING  | 0/5 | –         |
| 19  | Worktree-Struktur (E1, E8, E9)                       | PENDING  | 0/5 | –         |
| 18  | GitHub-Issues für E1, E8, E9 (16 Stück)              | PENDING  | 0/5 | –         |
| 20  | Startbriefing + Übergabe-Prüfung mit Test-PR         | PENDING  | 0/6 | –         |
| 21  | Plan-Verifikation                                    | PENDING  | 0/6 | –         |

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

| Punkt             | Befund                                                                                |
| ----------------- | ------------------------------------------------------------------------------------- |
| `/work/rota`      | git-Repository, Branch `main`, **kein Remote** — Pull entfällt                        |
| Letzter Commit    | `de5cdb6` — Suno-Werkzeugkette, Konto-Wächter                                         |
| Vorhanden         | `CLAUDE.md`, `.claude/`, `docs/suno-sfx.md`, `tools/` (3 Skripte)                     |
| Fehlt             | Alles unter `src/`, `package.json`, `.github/`, `docs/GAME-DESIGN.md`                 |
| Enforcement-Hooks | in `/work/rota/.claude/settings.json` registriert, Permissions unangetastet           |
| Auffälligkeit     | `.playwright-cli/*.log` wird derzeit mitversioniert → gehört in `.gitignore` (Task 1) |

---

## Blocker

(keine)

---

## Findings

| #   | Task | Finding                                                                                                                                                                                            | Behoben                                                       |
| --- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| F1  | 12   | Drei Einträge aus Teil F des Plans fehlten im ersten Wurf: „Captcha-Token exportieren und außerhalb einlösen" (F.4), „E1 blockiert den Start" (F.7), „Issue-Ausarbeitung je Epic" (F.6)            | ja, `c1794ed`                                                 |
| F2  | 12   | Ein F.5-Eintrag war umbenannt („Tastenbelegungs-Oberfläche" statt „Frei belegbare Tasten mit eigenem Einstellungsmenü") — dieselben Begriffe müssen in Plan und Design auffindbar sein             | ja, `c1794ed`                                                 |
| F3  | 12   | `CLAUDE.md` behauptete, `GAME-DESIGN.md` werde erst noch erstellt — echter Widerspruch, sobald die Datei existiert                                                                                 | ja, `c1794ed`                                                 |
| F4  | 13   | Erster Wertesatz erzeugte nur **5,3 %** Unterschied statt 10 %: Der Füller hatte 3 s Abklingzeit und konnte den GCD nicht füllen → **15 s Zwangs-Leerlauf**, den kein Können vermeiden kann        | ja, `2d233ee` — Füller-Cooldown = GCD                         |
| F5  | 13   | Zweiter Versuch koppelte den PYRE-Bonus an „volle Hitze". Der Verfall zieht die Hitze ständig unter den Cap → gemessen **2 statt 8 Zündungen** in 60 s. Für einen Spieler praktisch nicht treffbar | ja, `2d233ee` — Bonus-**Schwelle** bei 15 statt Cap-Bedingung |
| F6  | 13   | GAME-DESIGN 12.2 nennt **zwei** Start-Zauber, Schritt 1 braucht **drei** (ohne Entlade-Zauber keine Schul-Mechanik)                                                                                | ja, `2d233ee` — PYRE auf `unlockLevel: 2`                     |
