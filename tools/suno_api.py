#!/usr/bin/env python3
"""
suno_api.py — Suno-Sound-Generierung ueber die interne Web-API.

HTTP-Backend: curl_cffi (impersoniert Chrome auf TLS-/HTTP2-Fingerprint-Ebene),
damit Cloudflare die Requests ohne Turnstile-Challenge durchlaesst.

Auth: Clerk. Der kurzlebige JWT wird aus dem __client-Cookie frisch geminted
(Endpoint /v1/client/sessions/{sid}/tokens), daher braucht es nach einem
einmaligen Browser-Login keinen laufenden Browser mehr.

Cookie-Quelle: .suno-state.json (aus `playwright-cli -s=suno state-save`).

Usage:
    python suno_api.py auth-test          # Account + Credits pruefen
    python suno_api.py save-state         # Browser-Sitzung sichern (nur Suno-Cookies)
    python suno_api.py gen "<prompt>" <basename> [--loop] [--outdir DIR]
    python suno_api.py batch <manifest.json> [--outdir DIR]

Manifest-Format (JSON-Liste):
    [{"name": "spell-fire", "prompt": "fiery whoosh ...", "loop": false},
     {"name": "boss-kill", "prompt": "heavy gong ...", "target_s": 2.0, "attempts": 3}]

    target_s  = gewuenschte Laenge des HOERBAREN Anteils; wird durch Iteration angenaehert
                (Suno ignoriert Sekundenangaben im Prompt — wirksam ist das Ausklang-Vokabular).
    attempts  = max. Generierungs-Versuche (default 3)

    Wie weit ein Kandidat vom Ziel abweichen darf, steht NICHT im Auftrag,
    sondern in tools/sfx_limits.py — dieselbe Quelle, aus der sfx-verify.py
    seine Grenze zieht (Issue #215). Ein `tol` im Auftrag wird gemeldet und
    ignoriert.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

from curl_cffi import requests

from sfx_limits import LENGTH_TOLERANCE_S, candidate_usable

STATE_FILE = Path(__file__).resolve().parent.parent / ".suno-state.json"
API = "https://studio-api-prod.suno.com/api"
AUTH = "https://auth.suno.com/v1"
CDN = "https://cdn1.suno.ai"
BROWSER = "chrome124"
PW_SESSION = "suno"
SESSION_RE = re.compile(r"(session_[0-9a-zA-Z]+)")
EXPECTED_ACCOUNT = "backupslave"  # Nur dieser Suno-Account darf verwendet werden
PACING_S = 5  # Pause zwischen Generierungen — bewusst hoeflich zur API


def cookies_in(path: Path) -> list[dict]:
    """Die Suno-Cookies einer Zustandsdatei. Leer, wenn sie fehlt oder unlesbar ist."""
    if not path.exists():
        return []
    try:
        return json.loads(path.read_text()).get("cookies", [])
    except (json.JSONDecodeError, OSError):
        return []


def load_state() -> list[dict]:
    if not STATE_FILE.exists():
        sys.exit(
            f"FEHLER: {STATE_FILE} fehlt.\n"
            "Anlegen mit:  playwright-cli -s=suno state-save /work/rota/.suno-state.json"
        )
    return json.loads(STATE_FILE.read_text()).get("cookies", [])


def session() -> requests.Session:
    """Session mit korrekt domain-gescopten Suno-Cookies."""
    s = requests.Session(impersonate=BROWSER)
    for c in load_state():
        if "suno" in c["domain"]:
            s.cookies.set(c["name"], c["value"], domain=c["domain"], path=c.get("path", "/"))
    return s


def cookie_value(name: str) -> str:
    for c in load_state():
        if c["name"] == name and "suno" in c["domain"]:
            return c["value"]
    return ""


def session_id() -> str:
    m = SESSION_RE.match(cookie_value("clerk_active_context"))
    if m:
        return m.group(1)
    sys.exit("FEHLER: clerk_active_context-Cookie ohne session_id — neu einloggen und state-save.")


def missing_credentials(cookies: list[dict]) -> list[str]:
    """Was einem Zustand fehlt, um damit arbeiten zu koennen. Leere Liste = brauchbar.

    GEPRUEFT WIRD, WAS DIESES WERKZEUG SPAETER BENUTZT -- nicht etwas
    Danebenliegendes:

      __client              haelt die Clerk-Anmeldung; ohne ihn gibt es keinen JWT
      clerk_active_context  liefert die session_id fuer den Token-Endpunkt,
                            und zwar ueber DASSELBE Muster wie session_id()

    Beide ueber denselben Ausdruck zu pruefen, den der Aufruf spaeter nimmt, ist
    Absicht: Eine Pruefung, die nur "Cookie vorhanden" fragt, ginge bei einem
    leeren oder verstuemmelten Wert durch -- und genau so sah der Zustand aus,
    der am 2026-08-04 einen funktionierenden ueberschrieben hat.
    """
    werte = {c["name"]: c.get("value", "") for c in cookies if "suno" in c.get("domain", "")}
    fehlt = []
    if not werte.get("__client"):
        fehlt.append("__client")
    if not SESSION_RE.match(werte.get("clerk_active_context", "")):
        fehlt.append("clerk_active_context (mit session_id)")
    return fehlt


def sanitize_state(path: Path) -> int:
    """Alles verwerfen, was nicht zu Suno gehoert.

    playwright-cli state-save sichert den KOMPLETTEN Browser-Kontext — darin stecken auch
    Cookies von Drittanbietern, die suno.com einbindet (Stripe, Werbe-Pixel) sowie vom
    Login-Anbieter (Microsoft). Die braucht dieses Werkzeug nicht, also bleiben sie auch
    nicht im Klartext auf der Platte liegen.

    Gibt die Anzahl der entfernten Eintraege zurueck.
    """
    if not path.exists():
        return 0
    data = json.loads(path.read_text())
    cookies = data.get("cookies", [])
    keep = [c for c in cookies if "suno" in c.get("domain", "")]
    origins = data.get("origins", [])
    keep_origins = [o for o in origins if "suno" in o.get("origin", "")]
    removed = (len(cookies) - len(keep)) + (len(origins) - len(keep_origins))
    path.write_text(json.dumps({"cookies": keep, "origins": keep_origins}, indent=1))
    path.chmod(0o600)
    return removed


def refresh_state() -> bool:
    """Cookies aus dem laufenden Browser neu sichern (Clerk rotiert sie regelmaessig).

    DER VORHANDENE ZUSTAND WIRD NIE AUFS SPIEL GESETZT
    --------------------------------------------------
    Am 2026-08-04 hat diese Funktion eine funktionierende Datei durch eine
    wertlose ersetzt. Die Erfolgspruefung lautete

        ok = STATE_FILE.exists() and r.returncode == 0

    und fragte damit das Falsche: ob der Befehl durchlief und eine Datei da ist.
    `state-save` laeuft aber IMMER durch -- auch wenn im Browser niemand
    angemeldet ist. Dann liegen 23 Cookies in der Datei, `clerk_active_context`
    fehlt, und der Zugang ist tot. Es gab kein Backup; wiederhergestellt wurde
    er nur, weil zufaellig zwei alte Kopien in anderen Arbeitsbaeumen lagen.

    Unter welcher Bedingung haette die alte Pruefung ein anderes Ergebnis
    geliefert? Unter keiner. Deshalb jetzt drei Aenderungen:

      1. Gesichert wird NEBEN die Datei, nie darauf.
      2. Der neue Zustand muss die Pflicht-Cookies tragen, sonst wird er
         verworfen und der alte bleibt unangetastet.
      3. Erst unmittelbar vor dem Ersetzen entsteht die Sicherungskopie.

    Der verworfene Zustand bleibt als `.verworfen` liegen -- wer wissen will,
    warum das Auffrischen scheiterte, kann hineinsehen, statt es nachstellen zu
    muessen.
    """
    neu = STATE_FILE.parent / (STATE_FILE.name + ".neu")
    neu.unlink(missing_ok=True)
    r = subprocess.run(
        ["playwright-cli", f"-s={PW_SESSION}", "state-save", str(neu)],
        capture_output=True, text=True, timeout=60,
    )
    if r.returncode != 0 or not neu.exists():
        neu.unlink(missing_ok=True)
        print(f"  Auffrischen fehlgeschlagen (state-save Rueckgabewert {r.returncode})")
        print(f"  {STATE_FILE.name} ist UNVERAENDERT.")
        return False

    # Erst aufraeumen, dann pruefen: Beurteilt wird genau das, was geschrieben
    # wuerde -- nicht ein Zwischenstand, den es hinterher nicht mehr gibt.
    removed = sanitize_state(neu)
    fehlt = missing_credentials(cookies_in(neu))
    if fehlt:
        verworfen = STATE_FILE.parent / (STATE_FILE.name + ".verworfen")
        neu.replace(verworfen)
        print(f"  Auffrischen VERWORFEN — dem neuen Zustand fehlt: {', '.join(fehlt)}")
        print("  Im Browser ist vermutlich niemand angemeldet:")
        print(f"    playwright-cli -s={PW_SESSION} open https://suno.com/create --headed")
        print(f"  {STATE_FILE.name} ist UNVERAENDERT. Verworfener Zustand: {verworfen.name}")
        return False

    if STATE_FILE.exists():
        sicherung = STATE_FILE.parent / (STATE_FILE.name + ".bak")
        sicherung.write_bytes(STATE_FILE.read_bytes())
        sicherung.chmod(0o600)
    neu.replace(STATE_FILE)
    STATE_FILE.chmod(0o600)
    print(f"  Cookies aus dem Browser aufgefrischt ({removed} fremde Eintraege verworfen)")
    return True


def fresh_token(s: requests.Session, _retried: bool = False) -> str:
    """Kurzlebigen Clerk-JWT neu ausstellen (gueltig ~60 s).

    Bei abgelaufenen/rotierten Cookies einmalig die Sitzungsdatei aus dem laufenden
    Browser auffrischen und erneut versuchen.
    """
    r = s.post(
        f"{AUTH}/client/sessions/{session_id()}/tokens",
        params={"__clerk_api_version": "2025-04-10", "_clerk_js_version": "5.114.0"},
        headers={
            "origin": "https://suno.com",
            "referer": "https://suno.com/",
            "content-type": "application/x-www-form-urlencoded",
        },
        data="",
    )
    if r.status_code != 200:
        if not _retried and r.status_code in (401, 403):
            print(f"  Sitzung abgelaufen (HTTP {r.status_code}) — frische Cookies holen...")
            if refresh_state():
                return fresh_token(session(), _retried=True)
        sys.exit(
            f"FEHLER Token-Refresh: HTTP {r.status_code} — {r.text[:200]}\n"
            f"  Browser-Login erneuern:  playwright-cli -s={PW_SESSION} open https://suno.com/create --headed"
        )
    jwt = r.json().get("jwt")
    if not jwt:
        sys.exit(f"FEHLER: kein jwt in Antwort: {r.text[:300]}")
    return jwt


def api_headers(jwt: str) -> dict[str, str]:
    return {
        "authorization": f"Bearer {jwt}",
        "content-type": "application/json",
        "origin": "https://suno.com",
        "referer": "https://suno.com/",
        "affiliate-id": "undefined",
        "device-id": cookie_value("suno_device_id"),
        "browser-token": "{}",
    }


def check_account(s: requests.Session, jwt: str | None = None) -> str:
    """Prueft ueber die API, welcher Suno-Account aktiv ist. Bricht bei falschem Account ab.

    Rein API-basiert (GET /api/session/) — KEIN Browser noetig. Verhindert, dass
    versehentlich die Credits eines anderen Accounts verbraucht werden.
    """
    r = s.get(f"{API}/session/", headers=api_headers(jwt or fresh_token(s)))
    if r.status_code != 200:
        sys.exit(f"FEHLER Account-Pruefung: HTTP {r.status_code} — {r.text[:200]}")
    user = r.json().get("user") or {}
    name = user.get("handle") or user.get("display_name") or user.get("email") or ""
    if not name:
        sys.exit("FEHLER: Account nicht ermittelbar — Sitzung erneuern (save-state).")
    if EXPECTED_ACCOUNT.lower() not in name.lower():
        sys.exit(
            f"ABBRUCH: falscher Suno-Account aktiv — '{name}', erwartet '{EXPECTED_ACCOUNT}'.\n"
            "  Im Browser mit dem richtigen Account anmelden, dann:\n"
            "  /venv/bin/python tools/suno_api.py save-state"
        )
    return name


class ChallengeRequired(Exception):
    """Suno verlangt einen Captcha-Token (HTTP 422 token_validation_failed)."""


def generate(s: requests.Session, jwt: str, prompt: str, loop: bool = False,
             negative: str = "") -> list[str]:
    """Sound direkt per API generieren. Gibt die Clip-IDs der (zwei) Entwuerfe zurueck.

    Wirft ChallengeRequired, wenn Suno einen Captcha-Token verlangt — dann uebernimmt
    ui_generate() (echter Seiten-Klick), was zugleich das Vertrauen wiederherstellt.
    """
    title = " ".join(w.capitalize() for w in prompt.split()[:12])
    payload = {
        "token": None,
        "task": "sound",
        "generation_type": "TEXT",
        "title": title,
        "tags": prompt,
        "negative_tags": negative,
        "mv": "chirp-fenix",
        "prompt": "",
        "make_instrumental": True,
        "user_uploaded_images_b64": None,
        "metadata": {
            "web_client_pathname": "/create",
            "is_max_mode": False,
            "is_mumble": False,
            "create_mode": "custom",
            "disable_volume_normalization": False,
            "sound_configs": {"user_loop": loop},
        },
        "override_fields": [],
        "cover_clip_id": None,
        "cover_start_s": None,
        "cover_end_s": None,
        "persona_id": None,
        "artist_clip_id": None,
        "artist_start_s": None,
        "artist_end_s": None,
        "continue_clip_id": None,
        "continued_aligned_prompt": None,
        "continue_at": None,
        "transaction_uuid": str(uuid.uuid4()),
        "token_provider": None,
    }
    r = s.post(f"{API}/generate/v2-web/", headers=api_headers(jwt), json=payload)
    if r.status_code == 422 and "token_validation" in r.text:
        raise ChallengeRequired(r.text[:200])
    if r.status_code not in (200, 201):
        sys.exit(f"FEHLER generate: HTTP {r.status_code} — {r.text[:500]}")
    data = r.json()
    clips = data.get("clips") or ([data] if data.get("id") else [])
    ids = [c["id"] for c in clips]
    if not ids:
        sys.exit(f"FEHLER: keine Clip-IDs in Antwort: {json.dumps(data)[:400]}")
    print(f"  generate OK — {len(ids)} Entwuerfe: {', '.join(ids)}")
    return ids


# ---------------------------------------------------------------------------
# Generierung ueber die echte Seite (mintet den Captcha-Token selbst).
# Reine API-Generierung scheitert ab dem 3. Aufruf mit 422 token_validation_failed.
# ---------------------------------------------------------------------------

HOOK = Path(__file__).resolve().parent / "suno-hook.js"


def pw(*args: str, timeout: int = 120) -> str:
    r = subprocess.run(
        ["playwright-cli", f"-s={PW_SESSION}", *args],
        capture_output=True, text=True, timeout=timeout,
    )
    return r.stdout


def pw_eval(js: str) -> str:
    return pw("eval", js)


def latest_snapshot(seit: float) -> Path:
    """Die Seiten-Aufnahme, die playwright-cli GERADE geschrieben hat.

    ZWEI FEHLER AUF EINMAL, BEIDE STILL (Issue #204)
    ------------------------------------------------
    Vorher stand hier `Path("/work/.playwright-cli")` — fest verdrahtet.
    `playwright-cli` schreibt seine Aufnahmen aber ins ARBEITSVERZEICHNIS des
    Aufrufers. Aus einem Arbeitsbaum heraus las das Werkzeug damit die Aufnahme
    einer voellig anderen Sitzung: am 2026-08-04 eine zehn Stunden alte Seite,
    auf der es den Sounds-Reiter erwartungsgemaess nicht fand. Gemeldet wurde
    "Suno-Create-Seite mit Sounds-Tab nicht gefunden" — wahr fuer jene Datei,
    falsch fuer die Seite, die tatsaechlich offen stand.

    Der Ort allein reicht aber nicht. Wenn `pw("snapshot")` scheitert, liegt im
    richtigen Verzeichnis immer noch eine aeltere Aufnahme, und die Funktion
    haette sie genommen — derselbe Irrtum, nur naeher dran. Deshalb muss die
    Datei NACH dem Auftrag entstanden sein; sonst ist es keine Antwort auf
    diesen Aufruf, sondern ein Fund.
    """
    d = Path.cwd() / ".playwright-cli"
    frisch = [p for p in d.glob("page-*.yml") if p.stat().st_mtime >= seit]
    if not frisch:
        sys.exit(
            f"FEHLER: playwright-cli hat keine neue Seiten-Aufnahme in {d} abgelegt.\n"
            "  Laeuft die Sitzung?  playwright-cli list"
        )
    return max(frisch, key=lambda p: p.stat().st_mtime)


def ui_refs() -> tuple[str, str]:
    """Frische Element-Refs fuer Prompt-Textbox und Create-Button.

    Steht die Seite im Song-Modus, wird der Sounds-Reiter selbst geoeffnet: Nach
    einem Neuladen faellt Suno auf den Song-Modus zurueck, und ohne diesen
    Klick scheitert der Fallback an einer Seite, die eigentlich in Ordnung ist.
    """
    for attempt in range(2):
        seit = time.time()
        pw("snapshot")
        text = latest_snapshot(seit - 2).read_text()
        tb = re.search(r'textbox "Describe the sound you want".*?\[ref=([^\]]+)\]', text)
        cb = re.search(r'button "Create song".*?\[ref=([^\]]+)\]', text)
        if tb and cb:
            return tb.group(1), cb.group(1)
        if attempt == 0:
            tab = re.search(r'tab "Sounds" \[ref=([^\]]+)\]', text)
            if tab:
                print("  Sounds-Reiter war nicht offen — oeffne ihn...")
                pw("click", tab.group(1))
                time.sleep(2)

    sys.exit(
        "FEHLER: Suno-Create-Seite mit Sounds-Tab nicht gefunden.\n"
        f"  playwright-cli -s={PW_SESSION} open https://suno.com/create --headed"
    )


def workspace_ids() -> list[str]:
    """Song-UUIDs, die gerade im Workspace stehen — neueste zuerst.

    Zweiter Weg zu den Clip-IDs, falls der fetch-Hook nichts erfasst (siehe
    ui_generate). Die Links stehen als /song/{uuid} im DOM.
    """
    raw = pw_eval(
        "() => JSON.stringify(Array.from(document.querySelectorAll('a[href^=\"/song/\"]'))"
        ".map(a => a.getAttribute('href').slice(6)))"
    )
    m = re.search(r"\[.*\]", raw.replace('\\"', '"'))
    if not m:
        return []
    try:
        ids = json.loads(m.group(0))
    except json.JSONDecodeError:
        return []
    # Reihenfolge erhalten, Doppelte entfernen.
    seen: list[str] = []
    for i in ids:
        if i not in seen:
            seen.append(i)
    return seen


def ui_generate(prompt: str, loop: bool = False, timeout_s: int = 120) -> list[str]:
    """Loest die Generierung in der Seite aus und liest die Clip-UUIDs aus.

    Zwei Wege, weil der erste nicht verlaesslich ist: Der fetch-Hook sieht den
    Generate-Aufruf nur, wenn die Seite ihn ueber `window.fetch` schickt — bei
    einem Seiten-Neuaufbau (Clerk-Handshake, React-Remount) geht er verloren
    und `window.__gen` bleibt leer. Dann werden die neuen Eintraege stattdessen
    im Workspace abgelesen.
    """
    # Ohne abschliessendes Semikolon: playwright-cli uebergibt den Text als
    # Funktion an page.evaluate und lehnt "() => {...};" mit
    # "Passed function is not well-serializable" ab. Prettier haengt das
    # Semikolon bei jedem Formatieren wieder an, deshalb wird es hier
    # abgeschnitten statt in der Datei zu fehlen.
    pw_eval(HOOK.read_text().strip().rstrip(";"))
    before = pw_eval("() => String((window.__gen || []).length)")
    n_before = int(re.search(r'"?(\d+)"?', before).group(1)) if re.search(r"\d", before) else 0

    known_before = set(workspace_ids())

    tb, cb = ui_refs()
    if loop:
        set_loop_toggle(True)
    pw("fill", tb, prompt)
    pw("click", cb)

    for _ in range(timeout_s // 2):
        time.sleep(2)
        raw = pw_eval(
            "() => JSON.stringify((window.__gen || []).slice(%d).map(g => ({tags: g.tags, ids: g.ids})))"
            % n_before
        )
        m = re.search(r'\[(?:\\?"|\{).*\]', raw.replace('\\"', '"'))
        if m:
            try:
                entries = json.loads(m.group(0))
            except json.JSONDecodeError:
                entries = []
            for e in entries:
                if e.get("ids") and (not e.get("tags") or e["tags"].strip() == prompt.strip()):
                    print(f"  generate OK — {len(e['ids'])} Entwuerfe: {', '.join(e['ids'])}")
                    return e["ids"]

        # Zweiter Weg: neue Eintraege im Workspace.
        fresh = [i for i in workspace_ids() if i not in known_before]
        if len(fresh) >= 2:
            ids = fresh[:2]
            print(f"  generate OK (aus dem Workspace) — 2 Entwuerfe: {', '.join(ids)}")
            return ids
    sys.exit("FEHLER: keine Clip-IDs erfasst (Captcha sichtbar? Credits leer?) — Screenshot pruefen.")


def make_sound(s: requests.Session, prompt: str, loop: bool = False,
               negative: str = "") -> list[str]:
    """API-first mit automatischem Seiten-Fallback.

    Normalfall: direkter API-Aufruf (kein Browser noetig).
    Bei Captcha-Challenge: einmal ueber die echte Seite — erzeugt den Sound UND
    stellt das Vertrauen wieder her, sodass die folgenden Aufrufe wieder per API laufen.
    """
    try:
        return generate(s, fresh_token(s), prompt, loop, negative)
    except ChallengeRequired:
        print("  Challenge angefordert — weiche einmal auf die Seite aus...")
        return ui_generate(prompt, loop)


def set_loop_toggle(loop: bool) -> None:
    """Schaltet Type: One-Shot / Loop in den Advanced Options."""
    seit = time.time()
    pw("snapshot")
    text = latest_snapshot(seit - 2).read_text()
    label = "Loop" if loop else "One-Shot"
    m = re.search(rf'button "{label}".*?\[ref=([^\]]+)\]', text)
    if m:
        pw("click", m.group(1))
    else:
        print(f"  WARNUNG: {label}-Schalter nicht gefunden — UI-Zustand wird uebernommen")


# ---------------------------------------------------------------------------
# Ziel-Laenge: Suno ignoriert Sekundenangaben im Prompt (gemessen, siehe docs).
# Wirksam ist die Beschreibung des AUSKLANGS. Wir iterieren ueber diese Leiter,
# messen den reinen Ton-Anteil und behalten den besten Treffer.
# ---------------------------------------------------------------------------

# Gemessen: die Sekundenangabe im Prompt wirkt nicht, die KLANGBESCHREIBUNG schon.
# Kurz  = perkussiv/trocken (ping, click, snap)      -> 0,4-1,0 s
# Mittel= impact/whoosh mit kurzem Nachklang         -> 1,8-2,5 s
# Lang  = gong/boom mit Hall, riser, sustained, drone-> 6-11 s
LONGER = "with long reverb tail, slow decay, sustained resonance"
MUCH_LONGER = "long sustained resonance, very slow fade out, lingering reverb tail, drone"
SHORTER = "tight and dry, short decay"
MUCH_SHORTER = "clipped, snappy, percussive, no reverb"
# negative_tags: echter API-Hebel — hier gegen die jeweils falsche Richtung
NEG_FOR_SHORT = "long reverb, sustained, drone, ambient pad, slow fade"
NEG_FOR_LONG = "clipped, dry, abrupt cutoff"


def audible_seconds(path: Path) -> float:
    """Dauer OHNE Stille am Anfang/Ende (das, was man tatsaechlich hoert)."""
    tmp = path.with_suffix(".probe.wav")
    subprocess.run(
        ["ffmpeg", "-v", "error", "-y", "-i", str(path), "-af",
         "silenceremove=start_periods=1:start_threshold=-50dB:start_silence=0:"
         "stop_periods=-1:stop_threshold=-50dB:stop_duration=0.05", str(tmp)],
        capture_output=True, check=False,
    )
    if not tmp.exists():
        return 0.0
    r = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(tmp)],
        capture_output=True, text=True,
    )
    tmp.unlink(missing_ok=True)
    try:
        return float(r.stdout.strip())
    except ValueError:
        return 0.0


# Woerter im Basis-Prompt, die der gewuenschten Richtung zuwiderlaufen
CONFLICT_LONG = re.compile(r",?\s*\b(dark |long |deep )?(reverb|reverberant|sustained|drone|ambient|fade[- ]?out)\b[^,]*", re.I)
CONFLICT_SHORT = re.compile(r",?\s*\b(tight|dry|clipped|snappy|staccato|short decay|no reverb)\b[^,]*", re.I)


def tune_prompt(prompt: str, measured: float, target: float) -> tuple[str, str]:
    """Prompt + negative_tags fuer den naechsten Versuch. Gibt (prompt, negative) zurueck.

    Entfernt zusaetzlich widersprechende Woerter aus dem Basis-Prompt — sonst kaempft
    z.B. 'dark reverb' gegen den Kuerzungs-Hinweis an (im Test: Ziel 2,5 s -> 11 s).
    """
    if measured < target:
        hint = MUCH_LONGER if measured < target * 0.5 else LONGER
        base = CONFLICT_SHORT.sub("", prompt)
        return f"{base}, {hint}", NEG_FOR_LONG
    hint = MUCH_SHORTER if measured > target * 2 else SHORTER
    base = CONFLICT_LONG.sub("", prompt)
    return f"{base}, {hint}", NEG_FOR_SHORT


def make_sound_targeted(
    s: requests.Session, prompt: str, name: str, outdir: Path,
    target_s: float, attempts: int = 3, loop: bool = False,
) -> Path | None:
    """Generiert solange (max. `attempts`), bis ein Kandidat brauchbar ist.

    Es wird NICHT hart auf Laenge geschnitten — nur Stille entfernt, damit kein
    Ausklang abgehackt wird. Den Schnitt macht spaeter sfx-post.sh.

    BRAUCHBAR HEISST NICHT "NAHE DRAN" (Issue #215)
    ------------------------------------------------
    Vorher galt eine symmetrische Toleranz von 0,4 s: Ein Klang mit 0,229 s bei
    Ziel 0,50 s wurde angenommen, weil die Abweichung darunter lag. Die Abnahme
    fordert aber 0,05 s, und `sfx-post.sh` kann nur kuerzen — der Klang war
    endgueltig durchgefallen, die Credits waren weg, und gemeldet wurde Erfolg.

    Jetzt entscheidet `candidate_usable()` aus `sfx_limits`, dieselbe Quelle,
    aus der auch `sfx-verify.py` seine Grenze zieht.

    Gibt None zurueck, wenn kein Versuch einen brauchbaren Kandidaten geliefert
    hat. Dann entsteht auch KEINE Ausgabedatei: Was nicht abnahmefaehig ist,
    soll nicht im Ordner liegen und beim naechsten Lauf als "schon vorhanden"
    uebersprungen werden.
    """
    cand_dir = outdir / "candidates"
    all_cands: list[tuple[float, Path]] = []  # (Sekunden, Datei) ueber alle Versuche
    cur, neg = prompt, ""
    for attempt in range(1, attempts + 1):
        print(f"  Versuch {attempt}/{attempts} (Ziel {target_s:.2f}s, "
              f"brauchbar ab {target_s - LENGTH_TOLERANCE_S:.2f}s)")
        print(f"    prompt: {cur}")
        if neg:
            print(f"    negativ: {neg}")
        ids = make_sound(s, cur, loop, neg)
        files = download(s, ids, f"{name}-t{attempt}", cand_dir)
        attempt_secs: list[float] = []
        for f in files:
            sec = audible_seconds(f)
            attempt_secs.append(sec)
            all_cands.append((sec, f))
            diff = abs(sec - target_s)
            if sec >= target_s:
                marker = " (>= Ziel, kuerzbar)"
            elif candidate_usable(sec, target_s):
                marker = " (knapp darunter, im Rahmen der Grenze)"
            else:
                marker = " (ZU KURZ — nicht mehr zu retten)"
            print(f"    {f.name}: {sec:.2f}s Ton (Abweichung {diff:.2f}s){marker}")
        # Abgebrochen wird nur bei einem BRAUCHBAREN Kandidaten. "Nahe dran"
        # reicht nicht: Was unter dem Ziel liegt, kann die Nachbearbeitung
        # nicht mehr strecken.
        if any(candidate_usable(sec, target_s) for sec in attempt_secs):
            break
        if attempt < attempts and attempt_secs:
            cur, neg = tune_prompt(prompt, max(attempt_secs), target_s)
            time.sleep(PACING_S)

    if not all_cands:
        print(f"  FEHLER: kein Kandidat fuer {name}")
        return None
    # Auswahl: der kuerzeste brauchbare — er wird am wenigsten beschnitten.
    usable = sorted([c for c in all_cands if candidate_usable(c[0], target_s)])
    if not usable:
        laengster = max(all_cands)[0]
        print(f"  FEHLSCHLAG: {name} — nach {attempts} Versuchen kein Kandidat ueber "
              f"{target_s - LENGTH_TOLERANCE_S:.2f}s (bester: {laengster:.2f}s, Ziel "
              f"{target_s:.2f}s). Keine Ausgabedatei — die Abnahme wuerde sie ablehnen.")
        print(f"    Die Kandidaten liegen zum Nachhoeren in {cand_dir}")
        return None
    chosen_sec, chosen = usable[0]
    final = outdir / f"{name}-a.mp3"
    final.write_bytes(chosen.read_bytes())
    # "gekuerzt" nur, wenn wirklich gekuerzt wird. Ein Kandidat knapp UNTER dem
    # Ziel ist brauchbar, wird aber nicht beschnitten -- eine Zeile, die das
    # Gegenteil behauptet, ist genau die Sorte Meldung, die den Leser in die
    # falsche Richtung schickt.
    was = (f"wird auf {target_s:.2f}s gekuerzt" if chosen_sec > target_s
           else f"bleibt wie er ist, {target_s - chosen_sec:.2f}s unter dem Ziel")
    print(f"  GEWAEHLT: {final.name} — {chosen_sec:.2f}s Ton, {was} (Quelle: {chosen.name})")
    return final


def clip_status(s: requests.Session, jwt: str, ids: list[str]) -> dict[str, dict]:
    """Status mehrerer Clips in EINEM Aufruf. Gibt {id: clip-dict} zurueck."""
    r = s.get(f"{API}/feed/v2", params={"ids": ",".join(ids)}, headers=api_headers(jwt))
    if r.status_code != 200:
        print(f"  WARNUNG feed/v2: HTTP {r.status_code} — {r.text[:200]}")
        return {}
    return {c["id"]: c for c in r.json().get("clips", [])}


def await_complete(
    s: requests.Session, ids: list[str], tries: int = 60, delay: int = 5
) -> dict[str, str]:
    """Wartet bis alle Clips 'complete' sind. Gibt {id: audio_url} zurueck.

    Bricht pro Clip sofort ab, wenn Suno 'error' meldet (statt blind weiterzupollen).
    """
    ready: dict[str, str] = {}
    failed: set[str] = set()
    for attempt in range(tries):
        pending = [i for i in ids if i not in ready and i not in failed]
        if not pending:
            break
        clips = clip_status(s, fresh_token(s), pending)
        for cid in pending:
            c = clips.get(cid)
            if not c:
                continue
            st = c.get("status")
            if st == "complete" and c.get("audio_url"):
                ready[cid] = c["audio_url"]
                print(f"    {cid[:8]}: complete")
            elif st == "error":
                failed.add(cid)
                print(f"    {cid[:8]}: FEHLER — {c.get('error_type')}: {c.get('error_message')}")
            elif attempt % 4 == 0:
                print(f"    {cid[:8]}: {st}...")
        if len([i for i in ids if i not in ready and i not in failed]):
            time.sleep(delay)
    for cid in ids:
        if cid not in ready and cid not in failed:
            print(f"    {cid[:8]}: TIMEOUT nach {tries * delay}s")
    return ready


def download(s: requests.Session, ids: list[str], name: str, outdir: Path) -> list[Path]:
    """Wartet auf Fertigstellung (Status-API) und laedt die von Suno gelieferte audio_url."""
    outdir.mkdir(parents=True, exist_ok=True)
    urls = await_complete(s, ids)
    dl = requests.Session(impersonate=BROWSER)
    out: list[Path] = []
    for suffix, cid in zip("ab", ids):
        url = urls.get(cid)
        if not url:
            print(f"  UEBERSPRUNGEN: {cid} (nicht fertiggestellt)")
            continue
        target = outdir / f"{name}-{suffix}.mp3"
        r = dl.get(url)
        if r.status_code == 200 and len(r.content) > 5000:
            target.write_bytes(r.content)
            print(f"  {target}  ({len(r.content) // 1024} KB)")
            out.append(target)
        else:
            print(f"  FEHLER Download {url}: HTTP {r.status_code}, {len(r.content)} Bytes")
    return out


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    s = session()

    if cmd == "auth-test":
        jwt = fresh_token(s)
        acc = check_account(s, jwt)
        r = s.get(f"{API}/billing/info/", headers=api_headers(jwt))
        print(f"Account: {acc}  |  billing/info: HTTP {r.status_code}")
        if r.status_code == 200:
            info = r.json()
            print(f"  Credits: {info.get('total_credits_left')}  Tier: {info.get('subscription_type')}")
        else:
            print(f"  {r.text[:300]}")
        return

    if cmd == "save-state":
        # Rueckgabewert 1 bei Fehlschlag. Vorher meldete auch der abgelehnte
        # Fall 0 -- in einer &&-Kette liefe alles weiter, als stuende der
        # Zugang. Dieselbe Klasse wie der Fehler, den dieses Issue behebt.
        if not refresh_state():
            sys.exit(1)
        print(f"  Gesichert: {STATE_FILE} (nur Suno-Cookies, Rechte 600)")
        print(f"  Account: {check_account(session())}")  # nach Auffrischen
        return

    if cmd == "gen":
        check_account(s)
        prompt, name = sys.argv[2], sys.argv[3]
        loop = "--loop" in sys.argv
        outdir = Path("/work/rota/assets/sfx/raw")
        if "--outdir" in sys.argv:
            outdir = Path(sys.argv[sys.argv.index("--outdir") + 1])
        print(f"[1/2] generate ({'loop' if loop else 'one-shot'}): {prompt[:60]}...")
        ids = make_sound(s, prompt, loop)
        print("[2/2] download...")
        download(s, ids, name, outdir)
        print(f"FERTIG: {name}")
        return

    if cmd == "batch":
        check_account(s)
        manifest = json.loads(Path(sys.argv[2]).read_text())
        outdir = Path("/work/rota/assets/sfx/raw")
        if "--outdir" in sys.argv:
            outdir = Path(sys.argv[sys.argv.index("--outdir") + 1])
        # Eintraege MIT Ziel-Laenge laufen einzeln (Iteration noetig),
        # Eintraege OHNE Ziel-Laenge werden gebuendelt angestossen.
        jobs: list[tuple[str, list[str]]] = []
        gescheitert: list[str] = []
        for i, item in enumerate(manifest, 1):
            name, prompt = item["name"], item["prompt"]
            loop = bool(item.get("loop", False))
            if "tol" in item:
                # Nicht still uebergehen: Wer den Wert setzt, erwartet Wirkung.
                print(f"  HINWEIS: 'tol' bei {name} wird ignoriert — die Grenze steht "
                      f"seit Issue #215 in tools/sfx_limits.py (±{LENGTH_TOLERANCE_S}s)")
            if all((outdir / f"{name}-{sfx}.mp3").exists() for sfx in "ab"):
                print(f"[{i}/{len(manifest)}] {name}: schon vorhanden, ueberspringe")
                continue
            if item.get("target_s"):
                print(f"[{i}/{len(manifest)}] {name} — Ziel-Laenge {item['target_s']}s")
                if make_sound_targeted(
                    s, prompt, name, outdir,
                    target_s=float(item["target_s"]),
                    attempts=int(item.get("attempts", 3)),
                    loop=loop,
                ) is None:
                    gescheitert.append(name)
                time.sleep(PACING_S)
                continue
            print(f"[{i}/{len(manifest)}] generate {name} ({'loop' if loop else 'one-shot'})")
            jobs.append((name, make_sound(s, prompt, loop)))
            time.sleep(PACING_S)
        # Phase 2: einsammeln
        print(f"\nDownload von {len(jobs)} Jobs...")
        for name, ids in jobs:
            print(f"  {name}:")
            download(s, ids, name, outdir)
        # Ziel-Laengen ZUSAMMENFUEHREN statt ueberschreiben.
        #
        # Die Nachbearbeitung laeuft unten ueber den GANZEN Ordner, nicht nur
        # ueber die Dateien dieses Laufs. Stuende in lengths.json nur, was
        # gerade erzeugt wurde, verloeren alle frueher erzeugten Klaenge beim
        # naechsten Teil-Lauf ihre Kuerzung und lagen wieder in voller Laenge
        # vor (beobachtet: ein Boss-Klang mit Ziel 1,2 s kam als 7,3 s zurueck).
        lengths_file = outdir / "lengths.json"
        lengths: dict[str, float] = {}
        if lengths_file.exists():
            try:
                lengths = json.loads(lengths_file.read_text())
            except json.JSONDecodeError:
                lengths = {}
        lengths.update({i["name"]: float(i["target_s"]) for i in manifest if i.get("target_s")})
        for i in manifest:
            if i.get("loop"):
                lengths[i["name"]] = {"loop": True}
        if lengths:
            lengths_file.write_text(json.dumps(lengths, indent=1))
        print(f"\nFERTIG: {len(jobs) + len(lengths)} Sounds in {outdir}")
        ziel = outdir.parent / "final"
        print(f"Nachbearbeitung nach {ziel} (Stille weg + kuerzen + Lautheit + OGG):")
        # Puffer leeren, BEVOR das Unterprogramm schreibt.
        #
        # print() ist in eine Datei blockgepuffert, sfx-post.sh schreibt direkt
        # auf den Dateideskriptor. Ohne dieses flush() steht die Ausgabe der
        # Nachbearbeitung im Log GANZ OBEN, vor den Zeilen, die sie ankuendigen.
        # Ich habe daraus am 2026-08-04 geschlossen, die Nachbearbeitung sei gar
        # nicht gelaufen, und einen Fehler gemeldet, den es nicht gab.
        sys.stdout.flush()
        post = Path(__file__).resolve().parent / "sfx-post.sh"
        r = subprocess.run([str(post), str(outdir), str(ziel)], check=False)
        if r.returncode != 0:
            # Ein `batch`, das ohne fertige OGG-Dateien 0 meldet, waere genau
            # die Sorte stiller Erfolg, um die es in diesem Issue geht.
            sys.exit(f"FEHLER: Nachbearbeitung fehlgeschlagen (Rueckgabewert {r.returncode})")
        if gescheitert:
            # ZULETZT, nicht sofort: Die anderen Klaenge sind fertig und sollen
            # nachbearbeitet werden. Aber der Lauf als Ganzes ist nicht gelungen,
            # und das muss der Rueckgabewert sagen — sonst laeuft eine Kette
            # weiter, als laege alles vor (Issue #215, AC4).
            sys.exit(
                f"FEHLSCHLAG: {len(gescheitert)} von {len(manifest)} nicht erzeugt — "
                f"{', '.join(gescheitert)}.\n"
                "  Kein Kandidat hat die Ziel-Laenge erreicht. Prompt anpassen "
                "(dunkler, laengerer Ausklang) und erneut laufen lassen."
            )
        return

    sys.exit(f"Unbekanntes Kommando: {cmd}\n{__doc__}")


if __name__ == "__main__":
    main()
