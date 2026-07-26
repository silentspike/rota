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

4. **Dauer-Angaben bringen fast nichts** — laut Guides soll `2 second long ...` helfen; unsere
   Messung (Abschnitt 6) zeigt: nur bis ~2 s als grober Hinweis wirksam, darüber ignoriert.
   Die Länge wird stattdessen in der Nachbearbeitung gesetzt (`tools/sfx-post.sh`).

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

## 6. Automatisierung (verifiziert 2026-07-26)

**Suno hat keine öffentliche API**, aber die interne Web-API ist direkt ansteuerbar.
Es gibt zwei Wege — **Weg A (API) ist der Produktions-Weg**, Weg B (Browser) der Fallback.

### Weg A — Interne API ohne Browser (PRIMÄR, verifiziert)

`tools/suno_api.py`, HTTP-Backend `curl_cffi` (impersoniert Chrome auf TLS-/HTTP2-Ebene,
dadurch keine Cloudflare-Reibung). Nach einmaligem Browser-Login läuft alles headless.

```bash
# Einmalig (oder wenn Cookies abgelaufen): eingeloggte Browser-Session sichern
playwright-cli -s=suno open https://suno.com/create --headed   # User loggt ein
playwright-cli -s=suno state-save /work/rota/.suno-state.json  # gitignored!

# Danach ohne Browser:
/venv/bin/python tools/suno_api.py auth-test
/venv/bin/python tools/suno_api.py gen "<prompt>" <basename> [--loop] [--outdir DIR]
/venv/bin/python tools/suno_api.py batch <manifest.json> [--outdir DIR]
```

**Manifest-Format** (für Massen-Produktion; überspringt automatisch, was schon existiert):

```json
[
  {"name": "spell-fire", "prompt": "fiery whoosh into heavy impact ...", "loop": false},
  {"name": "amb-fireworld", "prompt": "dark volcanic ambience ...", "loop": true}
]
```

**Technik:**

| Baustein | Detail |
|----------|--------|
| Auth | Clerk. Kurzlebiger JWT (~60 s) wird pro Aufruf frisch geminted: `POST auth.suno.com/v1/client/sessions/{sid}/tokens` mit `__client`-Cookie. Session-ID steckt im `clerk_active_context`-Cookie. |
| **Cookie-Scoping ist kritisch** | `__client` liegt auf `auth.suno.com`, `__session` auf `.suno.com`. Werden Cookies nur für eine Domain gesetzt → `403 client_not_found`. Alle Suno-Cookies domain-korrekt übernehmen. |
| Generate | `POST studio-api-prod.suno.com/api/generate/v2-web/` mit `Authorization: Bearer <jwt>`, `task: "sound"`, `mv: "chirp-fenix"`, Prompt in `tags` (nicht `prompt`!), `make_instrumental: true`. |
| **Loop-Schalter** | `metadata.sound_configs.user_loop: true/false` — direkt im Payload, kein UI-Umschalten nötig. |
| **Kein Captcha** | Der echte Request enthält `"token": null, "token_provider": null` — Turnstile/hCaptcha werden auf der Seite geladen, sind für die Sound-Generierung aber nicht scharf. |
| Antwort | `{"clips": [{id: ...}, {id: ...}]}` — zwei Entwürfe, sofort. |
| Download | `https://cdn1.suno.ai/{clip-id}.mp3`, ohne Auth. Polling bis HTTP 200 (Rendering dauert ~20–60 s). |

**Verifiziert (2026-07-26):** `auth-test` → HTTP 200, 10.805 Credits · `gen` One-Shot → 2× 1,80 s ·
`gen --loop` → 2× 8,2/9,4 s · `batch` mit 2 Einträgen → 4 Dateien. Alle 48 kHz Stereo MP3.

### Captcha-Verhalten (gemessen)

Suno verlangt **nicht immer** einen Verifikations-Token, aber die Vertrauensstufe sinkt bei
reinen API-Aufrufen:

1. Die ersten API-Generierungen laufen mit `"token": null` durch.
2. Ab ca. dem dritten Aufruf: `HTTP 422 token_validation_failed`, `retryable: false` — dauerhaft,
   auch nach Wartezeit.
3. **Eine** Generierung über die echte Seite mintet einen Token (`token: "1.OVrEc8Vz...",
   token_provider: 2`) und **stellt das Vertrauen für die Sitzung wieder her** — danach laufen
   direkte API-Aufrufe wieder durch (verifiziert).

Deshalb: `make_sound()` = **API-first mit automatischem Seiten-Fallback**. Der Browser wird nur
angefasst, wenn Suno tatsächlich challengt; der Fallback erzeugt den Sound trotzdem und schaltet
zugleich wieder frei. `PACING_S = 5` hält den Abstand zwischen Generierungen bewusst höflich.

> Kein Umgehen der Schutzmaßnahme: Der Token entsteht ausschließlich dort, wo Suno ihn vorsieht —
> in der echten Seite. Es wird nichts nachgebaut, gelöst oder wiederverwendet.

### Dauer-Verhalten (gemessen 2026-07-26, je 2 Varianten pro Prompt)

| Prompt-Angabe | Container A/B | **Reiner Ton** A/B |
|---|---|---|
| „0.5 seconds" | 1,80 / 1,80 s | 0,87 / 0,85 s |
| „1 second" | 1,80 / 1,80 s | 0,43 / 0,99 s |
| „2 seconds duration" | 2,72 / 7,32 s | 1,76 / 2,06 s |
| „5 seconds duration" | 1,80 / 1,80 s | 0,84 / 1,12 s |
| „10 seconds duration" | 3,80 / 1,80 s | 2,46 / 0,93 s |
| *ohne Angabe* | 2,16 / 1,80 s | 0,62 / 0,88 s |

**Fazit:**

- **Die Zeitangabe im Prompt wird NICHT eingehalten.** 5 s und 10 s erzeugten 0,8–2,5 s Ton —
  nicht besser als ganz ohne Angabe.
- Bis ~2 s wirkt die Angabe als **grober Hinweis** (2 s → 1,8/2,1 s Ton), darüber wird sie ignoriert.
- **1,80 s ist die Standard-Containerlänge** für One-Shots; der Rest ist Stille-Polsterung.
- **Bis zu 60 % jeder Datei ist Stille** — für Tastendruck-Sounds tödlich (hörbare Verzögerung).
- **Lange Sounds nur über Loop-Modus** (dort gemessen: 8,2–9,4 s).

### Nachbearbeitung ist PFLICHT — `tools/sfx-post.sh`

```bash
tools/sfx-post.sh <in-dir> <out-dir> [a|b]     # Variante a oder b uebernehmen
```

Pro Datei: Stille vorn/hinten entfernen (−50 dB) → Lautheit normalisieren (EBU R128, −16 LUFS)
→ als OGG Vorbis speichern (kleiner als MP3, im Browser latenzärmer dekodierbar).

Verifiziert: 1,80 s → 0,87 s · 2,72 s → 1,78 s · 3,80 s → 2,46 s.

### Weg B — Browser-UI als Fallback

`tools/suno-sfx.sh` — steuert die Web-UI per playwright-cli über Element-Refs aus dem
Accessibility-Snapshot (keine Maus-Koordinaten, keine Popup-Menüs), Download ebenfalls per
CDN-Direktlink. Nutzen, falls sich die API ändert oder Cookies nicht erneuerbar sind.

```bash
# Voraussetzung: Session "suno" läuft, eingeloggt, Sounds-Tab offen
tools/suno-sfx.sh "<prompt>" <basename> [outdir]
```

### Weitere Erkenntnisse aus dem Reverse-Engineering

| Erkenntnis | Detail |
|------------|--------|
| **CDN-Direktlink** | `https://cdn1.suno.ai/{song-id}.mp3` — HTTP 200 sobald fertig gerendert, ohne Auth abrufbar. Song-IDs stehen im DOM als `/song/{uuid}`-Links im Workspace. |
| **Dauer-Angabe unzuverlässig** | „1 second" im Prompt lieferte 1,8–3,4 s (One-Shots), Loops 8–9,5 s. Trimmen in der Nachbearbeitung ist PFLICHT (ffmpeg silenceremove + Ziel-Länge). |
| **2 Entwürfe pro Generierung** | Beide werden heruntergeladen (`-a.mp3`/`-b.mp3`); da Claude nicht hören kann, entscheidet der User am Soundboard — oder Variante A wird als Default genommen. |
| **Snapshot statt Koordinaten (Weg B)** | `playwright-cli snapshot` liefert Element-Refs (Textbox, Create-Button) — Bedienung ohne Maus-Koordinaten und ohne Popup-Menüs. Refs IMMER frisch greppen (Attribute wie `[active]` können dazwischenstehen). |

## 7. Quellen

- [Suno Sounds: Generate Custom Audio Samples — offizielle Doku](https://help.suno.com/en/articles/10625537)
- [Jack Righteous: How to Generate AI Sound Effects with Suno Sounds](https://jackrighteous.com/en-us/blogs/guides-using-suno-ai-music-creation/suno-sounds-ai-sound-effects-guide)
- [Jack Righteous: How to Add Sound Effects Inside Suno Songs](https://jackrighteous.com/en-us/blogs/guides-using-suno-ai-music-creation/suno-ai-sound-effects-asterisks)
- [HookGenius: Suno AI for Game Developers](https://hookgenius.app/learn/suno-for-game-developers/)
- [HookGenius: Suno Gaming Music Prompts](https://hookgenius.app/learn/suno-gaming-music-prompts/)
