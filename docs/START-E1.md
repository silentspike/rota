# Startbriefing — Epic E1: Kampf-Kern und HUD

> **Diesen Text in die erste Dev-Session kopieren.**
> Es gibt zwei weitere Briefings für die Sessions, die parallel starten: `START-E8.md` (Klang)
> und `START-E9.md` (Grafik).

---

## Dein Auftrag

Du baust **Epic E1 — Kampf-Kern und HUD**: den spielbaren Kampf mit drei Zaubern gegen einen
Testboss. Am Ende steht ein Prototyp, der zwei Fragen beantwortet:

1. **Fühlt sich das Drücken gut an?** — beurteilt der Auftraggeber
2. **Lohnt sich Können messbar?** — die Rechnung sagt: ja, um 48 %. Das muss das Spiel bestätigen.

Du arbeitest **allein** an diesem Epic. E8 (Klang) und E9 (Grafik) laufen parallel, brauchen aber
keinen Code von dir.

---

## Zuerst lesen — in dieser Reihenfolge

| #   | Datei                  | Warum                                                                                  |
| --- | ---------------------- | -------------------------------------------------------------------------------------- |
| 1   | `CLAUDE.md`            | Die sechs Design-Gesetze und die technischen Rahmenbedingungen                         |
| 2   | `docs/GAME-DESIGN.md`  | Das komplette Design. **Kapitel 21 besonders** — dort steht, was schon verworfen wurde |
| 3   | `docs/STEP1-SPEC.md`   | Deine Zahlen. Jede davon ist gerechnet, nicht geschätzt                                |
| 4   | `docs/WORKER-GUIDE.md` | Der verbindliche Ablauf je Issue                                                       |

> Wenn dir beim Bauen eine bessere Idee kommt: **Kapitel 21 zuerst.** Mit hoher Wahrscheinlichkeit
> steht sie schon dort, mitsamt der Begründung, warum sie verworfen wurde.

---

## Dein Arbeitsplatz

```bash
cd /work/rota-worktrees/e1
git fetch origin
git switch wt-e1
git reset --hard origin/main
```

`wt-e1` ist der **Ruhe-Branch** — auf ihm wird nie gearbeitet. Für jedes Issue legst du davon einen
Feature-Branch an. (`main` lässt sich nicht auschecken, weil er schon in `/work/rota` liegt.)

**Dein Dateibereich:** `src/core/`, `src/render/`, `src/input/`, `src/devtools/`, `tests/unit/`.
Musst du darüber hinaus, ist das ein Hinweis darauf, dass der Schnitt nicht stimmt — dann melden,
nicht einfach machen.

---

## Die acht Issues, in dieser Reihenfolge

| #       | Issue                                             | Warum diese Stelle in der Reihenfolge                                                       |
| ------- | ------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| **#7**  | E1-1: Logik-Takt und Zeichenschleife trennen      | Das Zeitgerüst trägt alles Weitere                                                          |
| **#8**  | E1-2: Eingabe über physische Tastenpositionen     | Ohne Eingabe ist nichts prüfbar                                                             |
| **#9**  | E1-3: Zauber-Datenmodell, Abklingzeiten, GCD      | Ab hier gibt es Schaden — und die erste Zahl, die gegen `tools/step1-check.py` stimmen muss |
| **#10** | E1-4: Zauberleiste mit Wischblende                | Jetzt sieht man, was man drückt                                                             |
| **#11** | E1-5: Reaktionsgefäß, Überdruck-Timer, Kampfstart | Der erste vollständige Kampf                                                                |
| **#12** | E1-6: Schadenszahlen und Trefferwucht             | Das Gefühl, dass Treffer wirken                                                             |
| **#13** | E1-7: Kampfprotokoll und Kurzfassung              | Ohne Protokoll keine Auswertung                                                             |
| **#14** | E1-8: Dev-Werkzeuge, Aufzeichnen, Prüfsumme       | Zuletzt, weil es alles Vorherige messbar macht                                              |

> **#9 ist der erste harte Prüfstein.** Ab dort muss eine aufgezeichnete Runde denselben Schaden
> ergeben wie `python3 tools/step1-check.py`. Weicht sie ab, stimmt eine Regel nicht — und zwar
> bevor darauf aufgebaut wird.

---

## Was schon steht

|              |                                                                                    |
| ------------ | ---------------------------------------------------------------------------------- |
| Repository   | `silentspike/rota`, privat, `main` geschützt                                       |
| CI           | `ci-pass` ist der **einzige** Required Check. PR-Gate läuft in etwa 30 Sekunden    |
| Gerüst       | Vite 6 + TypeScript, streng konfiguriert, Vitest, ESLint, Prettier                 |
| Dev-Server   | nginx auf `http://localhost:8000/rota/` — liefert den **gebauten** Stand aus       |
| Daten        | `data/spells/calcination.json` und `data/bosses/first-crucible.json` liegen bereit |
| Referenz     | `tools/step1-check.py` — die Rechnung, gegen die dein Spiel stimmen muss           |
| Dev-Schalter | `__ROTA_DEVTOOLS__`, im Auslieferungs-Bundle nachweislich entfernt                 |

**Nach jeder Änderung:** `npm run build`, dann die Seite neu laden. Für die Dev-Werkzeuge im Browser:
`npx vite build --mode development`, danach wieder `npm run build`.

---

## Woran die Master-Session dich messen wird

Aus `docs/GAME-DESIGN.md` 20.2. Die Verfahren stehen in `docs/VERIFICATION.md` — **lies sie, bevor du
anfängst**, dann baust du von vornherein das ein, was gemessen wird.

| Prüfung             | Kriterium                                                  |
| ------------------- | ---------------------------------------------------------- |
| Ruckelfreiheit      | kein Bild über 16,7 ms in 600 aufeinanderfolgenden Bildern |
| Eingabe-Verzögerung | unter 16 ms                                                |
| Abklingzeiten       | auf die Millisekunde korrekt, kein Drift über 100 Zyklen   |
| Kampfprotokoll      | vollständig, kein Ereignis fehlt                           |
| Speicher            | **keine Speicheranforderung im laufenden Bild**            |
| Gleichlauf          | aufgezeichnete Runde = `tools/step1-check.py`              |

`docs/VERIFICATION.md` nimmt an, dass die Werkzeuge unter `window.__rota` erreichbar sind. Weicht
deine Umsetzung ab, **melde es** — dann wird das Dokument angepasst, nicht dein Code.

---

## Wie du meldest

Format in `docs/WORKER-GUIDE.md` § 6. Kurz: was gebaut, welche Befehle mit welcher **tatsächlichen**
Ausgabe, was **nicht** geprüft wurde und warum, Zuversicht in Prozent.

Der Abschnitt „Was NICHT geprüft wurde" bleibt nicht leer, nur weil es unangenehm aussieht. Eine
offengelegte Lücke ist wertvoll; eine verschwiegene kostet die Master-Session eine Stunde Suche.

---

## Die drei Sätze, die am häufigsten vergessen werden

1. **„Code gelesen" ist kein Nachweis.** Kein Befehl = ungeprüft. Ungeprüft ist nicht bestanden.
2. **Bei einer Design-Lücke: fragen, nicht raten.** `GAME-DESIGN.md` Kapitel 22 listet auf, was
   bewusst noch offen ist.
3. **Ein Issue ist 100 % fertig oder offen mit Blocker.** Dazwischen gibt es nichts.
