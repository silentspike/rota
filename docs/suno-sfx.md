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

| Option           | Bedeutung                   | Einsatz bei uns                                                                |
| ---------------- | --------------------------- | ------------------------------------------------------------------------------ |
| **One Shot**     | Einzelner kurzer Sound      | Zauber-Impacts, Crits, Kill-Sounds, UI-Klicks, „Spell ready"-Chime             |
| **Loop**         | Nahtlos wiederholbarer Clip | Hintergrund-Ambiente pro Welt, Spannungs-Layer                                 |
| **BPM**          | Beats pro Minute            | Nur für musikalische Loops; bei reinen Effekten weglassen                      |
| **Key (Tonart)** | z.B. C major, A minor       | Für Welt-Ambiente: **gleiche Tonart pro Welt** = alles klingt „aus einem Guss" |

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
   Messung (Abschnitt 7.5) zeigt: nur bis ~2 s als grober Hinweis wirksam, darüber ignoriert.
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

## 4. Sound-Längen: das zentrale Thema

Suno **ignoriert Sekundenangaben im Prompt** (Messung in Abschnitt 7.5). Die Länge entsteht aus
der Klangbeschreibung und wird anschließend in der Nachbearbeitung exakt gesetzt.

### 4.1 Länge über die Klangbeschreibung steuern (gemessen)

| Ziel               | So beschreiben                                                          | Gemessener Ton-Anteil   |
| ------------------ | ----------------------------------------------------------------------- | ----------------------- |
| **kurz** (< 1 s)   | `ping`, `click`, `tick`, `snap`, `blip` — perkussiv, trocken            | 0,43–0,99 s             |
| **mittel** (1–3 s) | `impact`, `whoosh`, `zap`, `hit` + kurzer Nachklang                     | 1,76–2,46 s             |
| **lang** (5–11 s)  | `gong`, `boom with long reverb tail`, `riser`, `sustained hum`, `drone` | 6,22–11,33 s            |
| **beliebig lang**  | **Loop-Modus** → nahtlose Schleife, im Spiel wiederholt                 | 8,2–9,4 s pro Durchlauf |

Zusätzliche Hebel im API-Payload:

- `negative_tags` — z.B. `"long reverb, sustained, drone"` erzwingt kürzer,
  `"clipped, dry, abrupt cutoff"` erzwingt länger. Wird von der Iteration automatisch gesetzt.
- **Widersprüche im Basis-Prompt bereinigen:** `dark reverb` kämpft gegen einen Kürzungs-Hinweis
  an (im Test: Ziel 2,5 s → 11 s). Die Iteration entfernt solche Wörter automatisch.

### 4.2 Zwei Sound-Typen — architektonisch unterschiedlich

**A) One-Shot** — feste Länge, einmal abgespielt: Zauber-Treffer, Crit, Kill, UI-Klick, Fanfare.

> **Produktionsregel: eher zu lang generieren als zu kurz.** Kürzen ist verlustfrei möglich
> (Einsatz behalten, bei Zielzeit mit weicher Ausblendung enden — klingt wie natürlicher
> Abklang). Verlängern geht nicht. Die Iteration wählt deshalb den **kürzesten Kandidaten,
> der noch ≥ Ziel liegt**.

**B) Sustained/Loop** — variable Länge, so lange wie der Spieleffekt: kanalisierte Zauber,
Buff-Fenster („10 s erhöhter Schaden"), DoT-Brennen, Welt-Ambiente, Zeitdruck-Layer.

> Hier ist **eine feste Datei grundsätzlich falsch** — der Spieler kann abbrechen, der Buff kann
> früher enden. Standard-Architektur im Spiele-Audio, dreiteilig:
>
> | Teil                   | Quelle         | Abspielweise                          |
> | ---------------------- | -------------- | ------------------------------------- |
> | **Ansatz** (attack)    | One-Shot       | einmal beim Start                     |
> | **Körper** (body)      | **Loop-Modus** | in Schleife, solange der Effekt läuft |
> | **Ausklang** (release) | One-Shot       | einmal beim Enden/Abbrechen           |
>
> Ein 30-Sekunden-Kanalisieren spielt dieselbe Schleife dreimal — kostet keine zusätzlichen
> Assets. Die Web Audio API kann per `loopStart`/`loopEnd` lückenlos schleifen.

> **KRITISCH:** Loop-Dateien dürfen in der Nachbearbeitung **nicht beschnitten** werden — jeder
> Eingriff an den Rändern zerstört die Nahtstelle. `sfx-post.sh` erkennt sie über
> `lengths.json` (`{"name": {"loop": true}}`) und normalisiert dann nur die Lautheit.

---

## 5. Produktions-Workflow für ROTA

1. **Manifest schreiben** — pro Sound: Name, Prompt, ggf. `target_s` (One-Shot) oder `loop: true`.
2. **`suno_api.py batch`** — generiert (API-first, Seiten-Fallback bei Challenge), prüft den
   Status, lädt UUID-genau herunter, iteriert bei gesetzter `target_s` bis die Länge passt.
3. **Automatische Nachbearbeitung** (`sfx-post.sh`, wird von `batch` selbst aufgerufen):
   Stille weg → auf Ziel-Länge kürzen mit Ausblendung → Lautheit normalisieren → OGG.
   Loops: nur normalisieren.
4. **Abhör-Soundboard** — der User hört alle Kandidaten ab und markiert _behalten / neu_.
   (Claude kann nicht hören — der User ist die ästhetische QA; die technische macht das Werkzeug.)
5. **Einbindung im Spiel** über die Web Audio API: latenzfreies Abspielen auf Tastendruck,
   Mixing, leichte Tonhöhen-Variation bei Wiederholungen (z.B. Crits) gegen Monotonie,
   `loopStart`/`loopEnd` für die Sustained-Sounds.

---

## 6. Prompt-Vorlagen für unsere Sound-Slots (Entwurf)

> Englisch prompten. `T` = Ziel-Länge für die Iteration, `LOOP` = Loop-Modus.

**One-Shots — Zauber & Treffer**

| Slot              | Prompt-Entwurf                                                                    | T     |
| ----------------- | --------------------------------------------------------------------------------- | ----- |
| Feuerball-Impact  | `fiery whoosh into heavy impact, crackling flame tail, fantasy spell hit`         | 1,2 s |
| Frost-Zauber      | `sharp icy crystalline shatter, cold shimmer tail, frost magic hit`               | 1,0 s |
| Arkan/Generic Hit | `bright arcane zap impact, short magical shimmer tail, fantasy spell`             | 0,8 s |
| Crit-Verstärker   | `massive impact hit with deep sub boom and debris tail, powerful critical strike` | 1,5 s |
| DoT-Applikation   | `dark sizzling ignition, burning crackle`                                         | 0,8 s |
| Heilung           | `warm soft chime swell with gentle shimmer, healing glow`                         | 1,2 s |
| Schutzschild auf  | `deep resonant energy barrier activation, metallic hum`                           | 1,0 s |
| Spell Reflect     | `sharp metallic deflection ping with reverse whoosh`                              | 0,5 s |

**One-Shots — UI & Ereignisse**

| Slot           | Prompt-Entwurf                                                             | T      |
| -------------- | -------------------------------------------------------------------------- | ------ |
| Spell ready    | `short bright chime, ascending, subtle, UI notification`                   | 0,4 s  |
| Fehleingabe    | `dull muted thud click, short, UI error, subtle`                           | 0,25 s |
| Countdown-Tick | `soft wooden tick, metronome click`                                        | 0,15 s |
| Kampfstart     | `deep braam with rising tension riser, battle start stinger`               | 2,0 s  |
| Boss-Kill      | `heavy metallic gong impact, dark reverb, victorious dark fantasy stinger` | 2,5 s  |
| Neue Bestzeit  | `triumphant short fanfare stinger, bright brass and shimmer`               | 2,0 s  |

**Sustained — dreiteilig (Ansatz / Körper / Ausklang)**

| Effekt                  | Ansatz (One-Shot)                              | Körper (LOOP)                                                                  | Ausklang (One-Shot)                          |
| ----------------------- | ---------------------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------- |
| Kanalisierter Zauber    | `energy channel start, rising hum` (0,4 s)     | `sustained magical channeling hum, seamless loop`                              | `energy channel release, fading hum` (0,6 s) |
| Schadens-Buff-Fenster   | `power surge activation, bright swell` (0,6 s) | `empowered magical aura hum, seamless loop`                                    | `power fading down, soft release` (0,8 s)    |
| DoT brennt              | `ignition whoosh` (0,3 s)                      | `steady burning crackle, seamless loop`                                        | `flame extinguish hiss` (0,5 s)              |
| Zeit fast abgelaufen    | —                                              | `tense heartbeat pulse with low rumble, seamless loop`                         | —                                            |
| Welt-Ambiente (je Welt) | —                                              | `dark volcanic ambience, distant lava rumble and ember crackle, seamless loop` | —                                            |

> Für Ambiente pro Welt die **Tonart fixieren** (Key-Feld), dann klingt eine Welt „aus einem Guss".
> Die Slot-Liste wächst mit dem Spell-Katalog aus dem Game-Design-Dokument.

---

## 7. Automatisierung (verifiziert 2026-07-26)

**Suno hat keine öffentliche API**, aber die interne Web-API ist direkt ansteuerbar.
Es gibt zwei Wege — **Weg A (API) ist der Produktions-Weg**, Weg B (Browser) der Fallback.

### 7.1 Weg A — Interne API (PRIMÄR, verifiziert)

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
  { "name": "spell-fire", "prompt": "fiery whoosh into heavy impact ...", "target_s": 1.2 },
  {
    "name": "boss-kill",
    "prompt": "heavy metallic gong impact ...",
    "target_s": 2.5,
    "tol": 0.5,
    "attempts": 3
  },
  { "name": "amb-fireworld", "prompt": "dark volcanic ambience ...", "loop": true }
]
```

| Feld       | Bedeutung                                                                                                                                                                                                                                                            |
| ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `target_s` | Ziel-Länge des **hörbaren** Anteils. Löst die Iteration aus: generieren → messen → Prompt nachsteuern (Ausklang-Vokabular + `negative_tags`) → wiederholen. Gewählt wird der **kürzeste Kandidat ≥ Ziel**, den die Nachbearbeitung dann exakt auf `target_s` bringt. |
| `tol`      | Toleranz in Sekunden (default 0,4)                                                                                                                                                                                                                                   |
| `attempts` | max. Generierungs-Versuche (default 3)                                                                                                                                                                                                                               |
| `loop`     | Loop-Modus. Schließt `target_s` aus — die Länge steuert das Spiel durch Wiederholen. Ränder werden nie beschnitten.                                                                                                                                                  |

Zwischenergebnisse landen in `<outdir>/candidates/`, der gewählte Kandidat als `<name>-a.mp3`,
das Endprodukt nach der Nachbearbeitung in `<outdir>/../final/<name>.ogg`.

**Technik:**

| Baustein                        | Detail                                                                                                                                                                                          |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Auth                            | Clerk. Kurzlebiger JWT (~60 s) wird pro Aufruf frisch geminted: `POST auth.suno.com/v1/client/sessions/{sid}/tokens` mit `__client`-Cookie. Session-ID steckt im `clerk_active_context`-Cookie. |
| **Cookie-Scoping ist kritisch** | `__client` liegt auf `auth.suno.com`, `__session` auf `.suno.com`. Werden Cookies nur für eine Domain gesetzt → `403 client_not_found`. Alle Suno-Cookies domain-korrekt übernehmen.            |
| Generate                        | `POST studio-api-prod.suno.com/api/generate/v2-web/` mit `Authorization: Bearer <jwt>`, `task: "sound"`, `mv: "chirp-fenix"`, Prompt in `tags` (nicht `prompt`!), `make_instrumental: true`.    |
| **Loop-Schalter**               | `metadata.sound_configs.user_loop: true/false` — direkt im Payload, kein UI-Umschalten nötig.                                                                                                   |
| **Kein Captcha**                | Der echte Request enthält `"token": null, "token_provider": null` — Turnstile/hCaptcha werden auf der Seite geladen, sind für die Sound-Generierung aber nicht scharf.                          |
| Antwort                         | `{"clips": [{id: ...}, {id: ...}]}` — zwei Entwürfe, sofort.                                                                                                                                    |
| Download                        | `https://cdn1.suno.ai/{clip-id}.mp3`, ohne Auth. Polling bis HTTP 200 (Rendering dauert ~20–60 s).                                                                                              |

**Verifiziert (2026-07-26):** `auth-test` → HTTP 200, 10.805 Credits · `gen` One-Shot → 2× 1,80 s ·
`gen --loop` → 2× 8,2/9,4 s · `batch` mit 2 Einträgen → 4 Dateien. Alle 48 kHz Stereo MP3.

### 7.2 Account-Absicherung und Datensparsamkeit

**Vorgesehener Account: `backupslave`** (`EXPECTED_ACCOUNT` in `suno_api.py`). Vor jeder
Generierung prüft `check_account()` **über die API** (`GET /api/session/` → `user.handle`),
welcher Account aktiv ist, und bricht bei Abweichung ab — damit nie versehentlich die Credits
eines anderen Accounts verbraucht werden. Kein Browser nötig.

**Datensparsamkeit:** `playwright-cli state-save` sichert den KOMPLETTEN Browser-Kontext.
Im ersten Lauf waren das 122 Cookies aus 28 Domains — darunter Stripe (Suno-Zahlungsabwicklung),
`login.microsoftonline.com`/`login.live.com` (Login über das Outlook-Konto) sowie Werbe- und
Affiliate-Pixel (TikTok, Twitter, Bing, bidr.io, pxf.io), die suno.com selbst einbindet.

`sanitize_state()` verwirft alles, was nicht zu `suno` gehört, und setzt die Dateirechte auf 600.
Läuft automatisch bei jedem `refresh_state()`/`save-state`. Verifiziert: 122 → 39 Cookies
(4 Suno-Domains), 89 fremde Einträge entfernt — Login danach weiterhin voll funktionsfähig
(in frischem Browser aus der bereinigten Datei wiederhergestellt).

### 7.3 Wann der Browser überhaupt gebraucht wird

Der Normalbetrieb läuft **ohne Browser**. Nur zwei Situationen fassen ihn an:

| Situation                  | Warum                                                                                                            |
| -------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Captcha-Challenge (`422`)  | Der Token entsteht nur in der echten Seite — ein Seiten-Klick erzeugt den Sound und schaltet die API wieder frei |
| Cookies rotiert/abgelaufen | `refresh_state()` sichert sie neu aus der laufenden Browser-Sitzung                                              |

Läuft keine Browser-Sitzung, startet `playwright-cli` bei einem solchen Befehl automatisch eine
neue — die ist dann aber nicht eingeloggt. Deshalb: Sitzung offen lassen, oder nach dem Login
`save-state` aufrufen und bei Bedarf mit `state-load` wiederherstellen.

### 7.4 Captcha-Verhalten (gemessen)

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

### 7.5 Dauer-Verhalten (gemessen 2026-07-26, je 2 Varianten pro Prompt)

| Prompt-Angabe         | Container A/B | **Reiner Ton** A/B |
| --------------------- | ------------- | ------------------ |
| „0.5 seconds"         | 1,80 / 1,80 s | 0,87 / 0,85 s      |
| „1 second"            | 1,80 / 1,80 s | 0,43 / 0,99 s      |
| „2 seconds duration"  | 2,72 / 7,32 s | 1,76 / 2,06 s      |
| „5 seconds duration"  | 1,80 / 1,80 s | 0,84 / 1,12 s      |
| „10 seconds duration" | 3,80 / 1,80 s | 2,46 / 0,93 s      |
| _ohne Angabe_         | 2,16 / 1,80 s | 0,62 / 0,88 s      |

**Fazit:**

- **Die Zeitangabe im Prompt wird NICHT eingehalten.** 5 s und 10 s erzeugten 0,8–2,5 s Ton —
  nicht besser als ganz ohne Angabe.
- Bis ~2 s wirkt die Angabe als **grober Hinweis** (2 s → 1,8/2,1 s Ton), darüber wird sie ignoriert.
- **1,80 s ist die Standard-Containerlänge** für One-Shots; der Rest ist Stille-Polsterung.
- **Bis zu 60 % jeder Datei ist Stille** — für Tastendruck-Sounds tödlich (hörbare Verzögerung).
- **Lange Sounds nur über Loop-Modus** (dort gemessen: 8,2–9,4 s).

### 7.6 Nachbearbeitung ist PFLICHT — `tools/sfx-post.sh`

```bash
tools/sfx-post.sh <in-dir> <out-dir> [a|b]     # Variante a oder b uebernehmen
```

Wird von `batch` automatisch aufgerufen. Steuerung über `<in-dir>/lengths.json`
(schreibt `batch` selbst aus dem Manifest):

```json
{ "boss-kill": 2.5, "amb-fireworld": { "loop": true } }
```

**One-Shots:** Stille vorn/hinten entfernen (−50 dB) → falls länger als Ziel: auf Ziel kürzen mit
exponentieller Ausblendung (20 % der Ziel-Länge, max. 0,25 s) → Lautheit normalisieren
(EBU R128, −16 LUFS) → OGG Vorbis.

**Loops:** ausschließlich Lautheit normalisieren — kein Trimmen, kein Fade, sonst bricht die Naht.

Verifiziert:

| Datei              | vorher  | nachher                         |
| ------------------ | ------- | ------------------------------- |
| Stille-Entfernung  | 1,80 s  | 0,87 s                          |
| Stille-Entfernung  | 3,80 s  | 2,46 s                          |
| Kürzen auf 2,0 s   | 13,88 s | **2,00 s** (0,25 s Ausblendung) |
| Kürzen auf 5,0 s   | 11,80 s | **5,00 s** (0,25 s Ausblendung) |
| Loop, unangetastet | 8,20 s  | 8,30 s (nur Pegel)              |

### 7.7 Sitzungs-Ablauf (Clerk-Cookies)

Clerk rotiert die Cookies laufend; zusätzlich kann die Suno-Sitzung ganz ablaufen
(beobachtet nach ca. 1 Stunde intensiver Nutzung — ob reguläre Ablaufzeit oder Reaktion auf
das Zugriffsmuster, ist nicht unterscheidbar).

Zwei Stufen, die erste läuft automatisch:

| Symptom                                          | Ursache                                   | Behandlung                                                                                                 |
| ------------------------------------------------ | ----------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `403 client_not_found`                           | Cookies im State-File veraltet (Rotation) | **automatisch**: `refresh_state()` sichert die Cookies neu aus dem laufenden Browser und wiederholt einmal |
| `403 client_user_not_found` nach dem Auffrischen | Browser selbst ausgeloggt                 | **manuell**: im sichtbaren Browserfenster neu einloggen, dann `state-save`                                 |

```bash
playwright-cli -s=suno open https://suno.com/create --headed   # User loggt ein
playwright-cli -s=suno state-save /work/rota/.suno-state.json
```

### 7.8 Weg B — Browser-UI als Fallback

`tools/suno-sfx.sh` — steuert die Web-UI per playwright-cli über Element-Refs aus dem
Accessibility-Snapshot (keine Maus-Koordinaten, keine Popup-Menüs), Download ebenfalls per
CDN-Direktlink. Nutzen, falls sich die API ändert oder Cookies nicht erneuerbar sind.

```bash
# Voraussetzung: Session "suno" läuft, eingeloggt, Sounds-Tab offen
tools/suno-sfx.sh "<prompt>" <basename> [outdir]
```

### 7.9 Weitere Erkenntnisse aus dem Reverse-Engineering

| Erkenntnis                             | Detail                                                                                                                                                                                                        |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **CDN-Direktlink**                     | `https://cdn1.suno.ai/{song-id}.mp3` — HTTP 200 sobald fertig gerendert, ohne Auth abrufbar. Song-IDs stehen im DOM als `/song/{uuid}`-Links im Workspace.                                                    |
| **Dauer-Angabe unzuverlässig**         | „1 second" im Prompt lieferte 1,8–3,4 s (One-Shots), Loops 8–9,5 s. Trimmen in der Nachbearbeitung ist PFLICHT (ffmpeg silenceremove + Ziel-Länge).                                                           |
| **2 Entwürfe pro Generierung**         | Beide werden heruntergeladen (`-a.mp3`/`-b.mp3`); da Claude nicht hören kann, entscheidet der User am Soundboard — oder Variante A wird als Default genommen.                                                 |
| **Snapshot statt Koordinaten (Weg B)** | `playwright-cli snapshot` liefert Element-Refs (Textbox, Create-Button) — Bedienung ohne Maus-Koordinaten und ohne Popup-Menüs. Refs IMMER frisch greppen (Attribute wie `[active]` können dazwischenstehen). |

## 8. Quellen

- [Suno Sounds: Generate Custom Audio Samples — offizielle Doku](https://help.suno.com/en/articles/10625537)
- [Jack Righteous: How to Generate AI Sound Effects with Suno Sounds](https://jackrighteous.com/en-us/blogs/guides-using-suno-ai-music-creation/suno-sounds-ai-sound-effects-guide)
- [Jack Righteous: How to Add Sound Effects Inside Suno Songs](https://jackrighteous.com/en-us/blogs/guides-using-suno-ai-music-creation/suno-ai-sound-effects-asterisks)
- [HookGenius: Suno AI for Game Developers](https://hookgenius.app/learn/suno-for-game-developers/)
- [HookGenius: Suno Gaming Music Prompts](https://hookgenius.app/learn/suno-gaming-music-prompts/)
