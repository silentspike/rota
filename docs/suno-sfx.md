# Suno Sounds — SFX-Produktion für ROTA

> Recherche-Stand: 2026-07-26. SSOT für die Sound-Produktion des Spiels.
> Kontext: ElevenLabs hat aktuell kein Guthaben → alle Sounds werden mit **Suno Sounds** (eigenes Suno-Feature) erzeugt.
> Wichtigste Einschränkung: Claude kann nicht hören → präzises Prompting nach Lehrbuch + User als finale Qualitätskontrolle.

---

## 1. Was Suno Sounds ist

- **Beta-Feature** von Suno: erzeugt einzelne Soundeffekte, Instrument-Samples und Ambient-Geräusche aus Textbeschreibungen. Kein Gesang, keine Songs — reine Sounds.
- **Zugang:** Im Create-Bereich das Dropdown (Custom Mode) auf **„Sounds"** umstellen.
- Pro Generierung entstehen **zwei Varianten** zur Auswahl.

### Einstellungen

| Option | Bedeutung | Einsatz bei uns |
|--------|-----------|-----------------|
| **One Shot** | Einzelner kurzer Sound | Zauber-Impacts, Crits, Kill-Sounds, UI-Klicks, „Spell ready"-Chime |
| **Loop** | Nahtlos wiederholbarer Clip | Hintergrund-Ambiente pro Welt, Spannungs-Layer |
| **BPM** | Beats pro Minute | Nur für musikalische Loops; bei reinen Effekten weglassen |
| **Key (Tonart)** | z.B. C major, A minor | Für Welt-Ambiente: **gleiche Tonart pro Welt** = alles klingt „aus einem Guss" |

---

## 2. Die Prompt-Regeln (das Entscheidende)

1. **Kurz und direkt schlägt lang und blumig.**
   - Gut: `Wind howling strong`
   - Schlecht: `the sound of a lot of wind outside during a storm`
   - Keine Sätze, keine Poesie — Klang-Stichworte.

2. **Etabliertes Sound-Design-Vokabular verwenden.** Suno versteht die Fachbegriffe der Branche:
   `whoosh`, `impact`, `boom`, `rumble`, `crash`, `glitch`, `shimmer`, `swoosh`, `hit`, `stinger`, `riser`, `tail`, `ambiance`, `kick drum`, `bark`

3. **Wie ein Audio-Designer denken, nicht wie ein Musiker.** Die Formel:

   ```
   TEXTUR + AKTION + MATERIAL + NACHKLANG + DAUER + KONTEXT
   ```

   Referenz-Beispiel aus den Guides:
   `heavy cinematic boom with metallic tail, 3 second long, dark trailer impact`
   → Was (boom), Textur (heavy cinematic), Material (metallic tail), Dauer (3s), Kontext (dark trailer)

4. **Dauer explizit in den Prompt schreiben**, wenn die Länge wichtig ist: `2 second long ...`

5. **Mehrfach generieren, dann auslesen.** Empfohlener Workflow laut Guides: pro Sound mehrere Durchläufe, stärkste Variante nehmen, ggf. nachträglich trimmen/normalisieren. Iterieren statt am ersten Ergebnis kleben: Wenn „coffee shop ambiance" Lofi-Musik liefert → umformulieren zu „background chatter and clinking cups ambiance".

### Beispiel-Prompts aus den Guides

- `cinematic whoosh transition sound`
- `heavy metal door slam with echo`
- `digital glitch transition, sharp, futuristic, short one shot`
- `forest night ambience with insects and distant wind, seamless loop`
- `coffee shop chatter and clinking cups, warm background loop`

---

## 3. Stärken & Schwächen von Suno Sounds

**Stark (= exakt unser Bedarfsprofil):**
- Cineastische Hits/Impacts und Übergänge
- Glitch-Effekte
- Synthetische/designte Effekte
- Ambient-Hintergrundschichten
- Ausdrücklich: **Game-Audio**

**Schwach (brauchen wir nicht):**
- Realistisches Foley (Schritte auf definiertem Untergrund)
- Komplexe physische Interaktionen
- High-End-Foley-Ansprüche

---

## 4. Produktions-Workflow für ROTA

1. **Prompt schreiben** — pro benötigtem Sound ein präziser Prompt nach der Formel (Abschnitt 2).
2. **Generieren in Suno** — pro Sound **2–3 Generierungen** → 4–6 Varianten (jede Generierung liefert 2).
3. **Abhör-Seite (Soundboard)** — lokale HTML-Seite mit allen Kandidaten pro Sound-Slot; der User klickt durch und markiert: *behalten / neu generieren*. (Claude kann nicht hören — der User ist die QA.)
4. **Technische Prüfung durch Claude** (ohne Ohren möglich):
   - Länge prüfen (`ffprobe`)
   - Lautheit/Pegel normalisieren (`ffmpeg loudnorm`)
   - **Stille am Anfang wegschneiden** (kritisch! Tastendruck → Sound muss latenzfrei kommen)
   - Clipping-Check
5. **Einbindung im Spiel** über die Web Audio API: latenzfreies Abspielen auf Tastendruck, Mixing mehrerer Sounds, leichte Tonhöhen-Variation bei wiederholten Sounds (z.B. Crits), damit nichts monoton klingt.

---

## 5. Prompt-Vorlagen für unsere Sound-Slots (Entwurf)

> Alle als **One Shot**, außer anders vermerkt. Englisch prompten.

| Slot | Prompt-Entwurf |
|------|---------------|
| Feuerball-Impact | `fiery whoosh into heavy impact, crackling flame tail, 1.5 seconds, fantasy spell hit, one shot` |
| Frost-Zauber | `sharp icy crystalline shatter, cold shimmer tail, 1 second, frost magic hit` |
| Arkan/Generic Hit | `bright arcane zap impact, short magical shimmer tail, 1 second, fantasy spell` |
| Crit-Verstärker | `massive impact hit with deep sub boom and debris tail, 1.5 seconds, powerful critical strike` |
| DoT-Applikation | `dark sizzling ignition, burning crackle settling into faint loop tail, 1 second` |
| Heilung | `warm soft chime swell with gentle shimmer, 1.5 seconds, healing glow` |
| Schutzschild auf | `deep resonant energy barrier activation, metallic hum tail, 1 second` |
| Spell Reflect | `sharp metallic deflection ping with reverse whoosh, 0.5 seconds` |
| Spell ready | `short bright chime, ascending, subtle, UI notification` |
| Fehleingabe (Spell nicht bereit) | `dull muted thud click, short, UI error, subtle` |
| Boss-Kill | `heavy metallic gong with dark reverb tail, 3 seconds, victorious dark fantasy stinger` |
| Neue Bestzeit | `triumphant short fanfare stinger, bright brass and shimmer, 2 seconds` |
| Countdown-Tick | `soft wooden tick, very short, metronome click` |
| Kampfstart | `deep braam with rising tension riser, 2 seconds, battle start stinger` |
| Zeit fast abgelaufen | `tense heartbeat pulse with low rumble, 2 second loop` (Loop) |
| Welt-Ambiente (Beispiel Feuerwelt) | `dark volcanic ambience, distant lava rumble and ember crackle, seamless loop` (Loop, Tonart pro Welt fixieren) |

> Die Slot-Liste wächst mit dem Spiel-Design (pro Zauber ein eigener Klang). Namen/Anzahl folgen aus dem Spell-Katalog im Game-Design-Dokument.

---

## 6. Quellen

- [Suno Sounds: Generate Custom Audio Samples — offizielle Doku](https://help.suno.com/en/articles/10625537)
- [Jack Righteous: How to Generate AI Sound Effects with Suno Sounds](https://jackrighteous.com/en-us/blogs/guides-using-suno-ai-music-creation/suno-sounds-ai-sound-effects-guide)
- [Jack Righteous: How to Add Sound Effects Inside Suno Songs](https://jackrighteous.com/en-us/blogs/guides-using-suno-ai-music-creation/suno-ai-sound-effects-asterisks)
- [HookGenius: Suno AI for Game Developers](https://hookgenius.app/learn/suno-for-game-developers/)
- [HookGenius: Suno Gaming Music Prompts](https://hookgenius.app/learn/suno-gaming-music-prompts/)
