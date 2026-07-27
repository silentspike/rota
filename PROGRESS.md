# Plan-Umsetzung Progress — ROTA Infrastruktur

## Status: ABGESCHLOSSEN MIT EINEM BLOCKER

**Plan:** `/home/jan/.claude/plans/greedy-skipping-russell.md` (SSOT der Planung)
**Umfang:** Teil E.5 — der Infrastruktur-Aufbau, bis der erste Worker (E1) starten kann.
Der Spielinhalt selbst gehört den Worker-Sessions und ist **nicht** Teil dieser Liste.

**Tasks gesamt:** 13 · **fertig:** 12 · **blockiert:** 1

> **Der erste Worker kann starten.** Der eine Blocker (fehlende HUD-Referenzbilder) hält nur ein
> einzelnes Issue in Epic E9 auf, nicht den Start.

---

## Ergebnis

| #   | Task                                         | Status        | ACs | Commit / Nachweis |
| --- | -------------------------------------------- | ------------- | --- | ----------------- |
| 12  | `docs/GAME-DESIGN.md` (SSOT der Umsetzung)   | **FERTIG**    | 5/5 | `c1794ed`         |
| 13  | Schritt-1-Spezifikation mit konkreten Zahlen | **FERTIG**    | 5/5 | `2d233ee`         |
| 14  | Referenzbilder ablegen, `ASSETS.md`          | **BLOCKIERT** | 2/5 | `d9973c9`         |
| 16  | `docs/WORKER-GUIDE.md`                       | **FERTIG**    | 5/5 | `06ada5c`         |
| 17  | `docs/VERIFICATION.md`                       | **FERTIG**    | 5/5 | `6816f47`         |
| 1   | GitHub-Repository einrichten                 | **FERTIG**    | 6/6 | `d5acc13`         |
| 2   | CI-Workflows mit `ci-pass`-Aggregat          | **FERTIG**    | 6/6 | `4ca2d97`         |
| 3   | Vite-/TypeScript-Gerüst                      | **FERTIG**    | 6/6 | `38cd58f`         |
| 15  | nginx-Route `localhost:8000/rota/`           | **FERTIG**    | 5/5 | nginx-Konfig      |
| 19  | Worktree-Struktur (e1, e8, e9)               | **FERTIG**    | 5/5 | `git worktree`    |
| 18  | GitHub-Issues für E1, E8, E9                 | **FERTIG**    | 5/5 | Issues #7–#22     |
| 20  | Startbriefings + Übergabe-Prüfung            | **FERTIG**    | 6/6 | `016efc6`         |
| 21  | Plan-Verifikation                            | **FERTIG**    | 6/6 | dieser Eintrag    |

**ACs gesamt: 70 · bestanden: 67 · blockiert: 3**

---

## Was jetzt bereitsteht

|                   |                                                                         |
| ----------------- | ----------------------------------------------------------------------- |
| Repository        | `silentspike/rota`, privat, 20 Labels, Vorlagen, CODEOWNERS, dependabot |
| Branch Protection | `ci-pass` als **einziger** Required Check, strict, `enforce_admins`     |
| CI                | PR-Gate in **29 Sekunden** (Ziel war unter 3 Minuten)                   |
| Gerüst            | Vite 6 + TypeScript ohne Framework, Vitest, ESLint 10, Prettier         |
| Dev-Server        | `http://localhost:8000/rota/` (gebauter Stand)                          |
| Worktrees         | `/work/rota-worktrees/{e1,e8,e9}` mit Ruhe-Branches                     |
| Issues            | 16 für E1 (8), E8 (4), E9 (4), ACs in Evidence-Form                     |
| Briefings         | `docs/START-E1.md`, `START-E8.md`, `START-E9.md`                        |

---

## Blocker

| #      | Was fehlt                                                                                                                    | Wirkung                                                                                                                       | Was der Auftraggeber tun muss                                                   |
| ------ | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| **B1** | Die zwei **HUD-Referenzbilder** (Reaktionsgefäß, Messring, „CASCADE FAILURE"-Zauberbalken, Pergament-Ausgabefeld, Apparatur) | Blockiert **nur Issue #19** (E9-1, Stil festlegen). Das HUD-Gerüst entsteht laut GAME-DESIGN 17.2 ohnehin zuerst ohne Vorlage | Die zwei Bilder nach `/work/rota/docs/refs/` legen und in `ASSETS.md` eintragen |

Eine systemweite Suche über `/home/jan`, `/work`, `/data` und `/tmp` — alle Bilddateien der letzten
Tage plus Namenssuche nach `rota`, `hud`, `alch`, `crucible`, `cascade` — blieb ergebnislos.
Wahrscheinlichste Erklärung: Sie lagen als Anhang im Gesprächsverlauf und wurden nie als Datei
gespeichert.

---

## Findings — alle behoben

| #       | Task  | Finding                                                                                                                                                                                                                    | Behoben                               |
| ------- | ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| F1      | 12    | Drei Einträge aus Teil F des Plans fehlten: „Captcha-Token exportieren" (F.4), „E1 blockiert den Start" (F.7), „Issue-Ausarbeitung je Epic" (F.6)                                                                          | `c1794ed`                             |
| F2      | 12    | Ein F.5-Eintrag war umbenannt — dieselben Begriffe müssen in Plan und Design auffindbar sein                                                                                                                               | `c1794ed`                             |
| F3      | 12    | `CLAUDE.md` behauptete, `GAME-DESIGN.md` werde erst noch erstellt                                                                                                                                                          | `c1794ed`                             |
| F4      | 13    | Erster Wertesatz erzeugte nur **5,3 %** statt 10 %: Der Füller hatte 3 s Abklingzeit und konnte den Takt nicht füllen → **15 s Zwangs-Leerlauf**, den kein Können vermeiden kann                                           | `2d233ee` — Füller-Cooldown = GCD     |
| F5      | 13    | Zweiter Versuch koppelte den PYRE-Bonus an „volle Hitze". Der Verfall zieht die Hitze ständig unter den Cap → **2 statt 8 Zündungen** je Minute, für einen Spieler nicht treffbar                                          | `2d233ee` — Bonus-**Schwelle** bei 15 |
| F6      | 13    | GAME-DESIGN 12.2 nennt **zwei** Start-Zauber, Schritt 1 braucht **drei**                                                                                                                                                   | `2d233ee` — PYRE auf `unlockLevel: 2` |
| F7      | 2     | `gitleaks-action` verlangt für Organisationen eine kostenpflichtige Lizenz                                                                                                                                                 | `a59c348` — Binary direkt             |
| F8      | 2     | Unicode-Prüfung durchsuchte `node_modules` und schlug an TypeScripts Bibliotheksdateien fehl                                                                                                                               | `cea4f94`                             |
| F9      | 2     | `parity` prüfte auf den Ordner statt auf Testdateien; vitest bricht ohne Tests ab                                                                                                                                          | `cea4f94`                             |
| F10     | 2     | `changes`-Job scheiterte bei Dependabot-PRs an „no merge base"                                                                                                                                                             | `4ca2d97` — `fetch-depth: 0`          |
| **F11** | **2** | **GRAVIEREND: `ci-pass` wurde grün, wenn `changes` ausfiel.** Alle anderen Jobs wurden übersprungen, und „skipped" galt als in Ordnung — der Required Check hätte gemeldet, dass alles stimmt, obwohl nichts geprüft wurde | `4ca2d97`                             |
| F12     | 3     | npm meldete 5 high-severity Schwachstellen über `eslint → minimatch/brace-expansion`                                                                                                                                       | eslint 10.8.0, jetzt 0                |
| F13     | 19    | Der dokumentierte Ablauf `git checkout main` wäre beim ersten Worker sofort fehlgeschlagen: `main` ist bereits in `/work/rota` ausgecheckt                                                                                 | Ruhe-Branches `wt-e1` usw.            |
| F14     | 21    | Worktrees `e8` und `e9` standen bei der Übergabe auf einem älteren Stand                                                                                                                                                   | auf `origin/main` gesetzt             |

---

## Offener Betrieb — nicht Teil dieses Aufbaus

Drei **Dependabot-PRs** sind offen (#1, #2, #4). Zwei heben Actions auf Major-Versionen an
(`checkout` 5 → 7, `setup-node` 5 → 7). Vor dem Merge prüfen, ob die CI damit grün bleibt.

---

## Plan-Verifikation (Task 21)

**Live-Test aus einem frischen Klon** — genau so, wie ein Worker startet:

```
git clone → 104 Dateien
npm ci    → 0 vulnerabilities
npm run build → dist/ mit 3 Dateien
npm test  → 2/2 grün
npm run typecheck → EXIT=0
npm run lint → EXIT=0
python3 tools/step1-check.py → 48,2 %, Kriterium erfüllt
```

**Seite über nginx:** HTTP 200, Titel „ROTA", keine Konsolenfehler außer dem erwarteten
`/favicon.ico`-404 auf der Server-Wurzel (Browser-Automatismus, in `CLAUDE.md` dokumentiert).

**E.5-Tabelle des Plans:** alle 12 Aufgaben einzeln geprüft, jede mit eigenem Befehl und eigener
Ausgabe. 11 vollständig, 1 mit Blocker B1.
