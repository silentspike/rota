# ROTA — Spell-Rotation-DPS-Game (Browser)

Du arbeitest im Projekt **ROTA**: ein Browser-Spiel, bei dem der Spieler ohne Spielfigur-Bewegung
gegen Bosse ("Targets") kämpft und maximalen DPS über die perfekte Spell-Rotation erzielt —
angelehnt an World-of-Warcraft-Rotationen (Global Cooldown, Cooldown-Management, Synergien).

## Rollen & Arbeitsteilung

- **Planungs-Session** (läuft in `/work`): erstellt das Game-Design-Dokument und den Implementierungsplan gemeinsam mit dem Auftraggeber.
- **Diese Session** (in `/work/rota`): setzt den Plan um. KEINE eigenmächtigen Design-Änderungen —
  bei Konflikten oder Lücken im Design: FRAGEN, nicht raten.

## SSOT-Dokumente

| Dokument              | Inhalt                                                                                                                                        |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `docs/GAME-DESIGN.md` | **Das komplette Spiel-Design — SSOT der Umsetzung.** Vor jeder Design-Entscheidung Kapitel 21 lesen (dort steht, was bereits verworfen wurde) |
| `docs/suno-sfx.md`    | Sound-Produktion mit Suno Sounds: Prompt-Regeln, Workflow, Prompt-Vorlagen                                                                    |

Bei Widerspruch zwischen Code/Kommentaren und `docs/GAME-DESIGN.md` gewinnt IMMER das Design-Dokument.

## Nicht verhandelbare Design-Gesetze (vom Auftraggeber festgelegt)

1. **Keypress = Trigger.** Ein Tastendruck löst genau eine Aktion aus. NIEMALS Tasten halten,
   niemals unterschiedlich lange Drücke. Cast-Balken und DoTs laufen nach dem Druck von selbst.
2. **Nur Keybinds.** Zauber werden ausschließlich über die Tastatur ausgelöst (einfache Tasten,
   keine Strg/Alt-Kombinationen). Die Zauberleiste ist sichtbar, aber nicht klickbar.
3. **Feature-Parität Fenster/Vollbild.** Vollbild ist rein kosmetisch — identisches Verhalten,
   identische Features in beiden Modi. Keine exklusiven Vollbild-Features.
4. **Kein Figuren-Artwork.** Keine Spielfigur, keine illustrierten Boss-Kreaturen. Bosse sind
   abstrakte "Targets" (geometrisch, mit Zustands-Feedback). Das Spiel IST das HUD — Design-Qualität
   fließt in Zauberleiste, Cooldown-Anzeigen, Zahlen, Balken, Meter.
5. **Spieltexte nur Englisch.** UI, Zaubernamen, Talente, Tutorials: English only.
   Kommunikation mit dem Auftraggeber: Deutsch.
6. **Determinismus.** Keine Zufalls-Procs im Spieler-Kit. Synergien sind 100% planbar.
   Zufall existiert nur beim Zauberbuch-Draft (gelenkter Zufall).

## Technische Rahmenbedingungen

- **Dev-Server:** nginx auf `localhost:8000/rota/` — NIEMALS eigene HTTP-Server starten.
  - **Ausgeliefert wird der gebaute Stand** (`npm run build` → `dist/`), nicht der Vite-Dev-Server.
    So prüfen Worker und Master-Session dasselbe, was auch ausgeliefert würde. Nach jeder Änderung
    also `npm run build`, dann neu laden.
  - **Dev-Werkzeuge im Browser testen:** `npx vite build --mode development` — dann ist
    `window.__rota` verfügbar. Danach wieder `npm run build`, damit der Produktionsstand ausliegt.
  - **Abhör-Soundboard:** `npm run soundboard`, dann `localhost:8000/rota/soundboard.html`.
    Die Seite entsteht **nur** im Entwicklungs-Bau (Dev-Werkzeug, GAME-DESIGN 19.3); jedes
    `npm run build` entfernt sie wieder. Ohne den Bau-Schritt liefert nginx über `try_files`
    die Spiel-Seite aus — HTTP 200, aber die falsche Seite.
  - Ein `404` auf `/favicon.ico` in der Konsole ist **erwartet und nicht projektbezogen**: Browser
    fragen die Datei automatisch auf der Server-Wurzel ab, nicht unter `/rota/`. Das eigene Symbol
    liegt unter `/rota/favicon.svg` und wird korrekt geladen.
- **Browser-Tests:** `playwright-cli -s=rota ...` (headless; `--headed` nur wenn der User zuschauen will).
  Session am Ende mit `close` beenden.
- **Sound:** Erzeugung via Suno Sounds (siehe `docs/suno-sfx.md`). ElevenLabs hat KEIN Guthaben.
  Claude kann nicht hören → Sounds technisch prüfen (ffprobe/ffmpeg: Länge, Pegel, Anfangs-Stille,
  Clipping), ästhetische Abnahme macht der User über ein Abhör-Soundboard.
- **Speicherung:** Spielstand im Browser (localStorage), laufend automatisch.
- **Verifikation:** Nach jeder Code-Änderung: im laufenden Spiel testen (playwright-cli Screenshot +
  Konsole prüfen), BEVOR an den User berichtet wird. Gameplay-Logik (DPS-Formeln, Cooldowns, GCD,
  Punkteberechnung) zusätzlich mit Unit-Tests abdecken.

## Verzeichnisstruktur (Soll)

```
/work/rota/
  CLAUDE.md            ← diese Datei
  .claude/             ← projektspezifische Claude-Konfiguration
  docs/                ← SSOT-Dokumente (Design, Sound, Pläne)
  src/                 ← Spiel-Code
  assets/              ← Sounds, Grafiken (generiert)
  tests/               ← Unit-Tests der Spiel-Logik
```
