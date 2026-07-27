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
     {"name": "boss-kill", "prompt": "heavy gong ...", "target_s": 2.0, "tol": 0.4, "attempts": 3}]

    target_s  = gewuenschte Laenge des HOERBAREN Anteils; wird durch Iteration angenaehert
                (Suno ignoriert Sekundenangaben im Prompt — wirksam ist das Ausklang-Vokabular).
    tol       = erlaubte Abweichung in Sekunden (default 0.4)
    attempts  = max. Generierungs-Versuche (default 3)
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

STATE_FILE = Path(__file__).resolve().parent.parent / ".suno-state.json"
API = "https://studio-api-prod.suno.com/api"
AUTH = "https://auth.suno.com/v1"
CDN = "https://cdn1.suno.ai"
BROWSER = "chrome124"
PW_SESSION = "suno"
EXPECTED_ACCOUNT = "backupslave"  # Nur dieser Suno-Account darf verwendet werden
PACING_S = 5  # Pause zwischen Generierungen — bewusst hoeflich zur API


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
    m = re.match(r"(session_[0-9a-zA-Z]+)", cookie_value("clerk_active_context"))
    if m:
        return m.group(1)
    sys.exit("FEHLER: clerk_active_context-Cookie ohne session_id — neu einloggen und state-save.")


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
    """Cookies aus dem laufenden Browser neu sichern (Clerk rotiert sie regelmaessig)."""
    r = subprocess.run(
        ["playwright-cli", f"-s={PW_SESSION}", "state-save", str(STATE_FILE)],
        capture_output=True, text=True, timeout=60,
    )
    ok = STATE_FILE.exists() and r.returncode == 0
    if ok:
        removed = sanitize_state(STATE_FILE)
        print(f"  Cookies aus dem Browser aufgefrischt ({removed} fremde Eintraege verworfen)")
    else:
        print("  Auffrischen fehlgeschlagen")
    return ok


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


def latest_snapshot() -> Path:
    d = Path("/work/.playwright-cli")
    return max(d.glob("page-*.yml"), key=lambda p: p.stat().st_mtime)


def ui_refs() -> tuple[str, str]:
    """Frische Element-Refs fuer Prompt-Textbox und Create-Button.

    Steht die Seite im Song-Modus, wird der Sounds-Reiter selbst geoeffnet: Nach
    einem Neuladen faellt Suno auf den Song-Modus zurueck, und ohne diesen
    Klick scheitert der Fallback an einer Seite, die eigentlich in Ordnung ist.
    """
    for attempt in range(2):
        pw("snapshot")
        text = latest_snapshot().read_text()
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
    pw("snapshot")
    text = latest_snapshot().read_text()
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
    target_s: float, tol: float = 0.4, attempts: int = 3, loop: bool = False,
) -> Path | None:
    """Generiert solange (max. `attempts`), bis der reine Ton nahe an target_s liegt.

    Behaelt den besten Kandidaten. Es wird NICHT hart auf Laenge geschnitten —
    nur Stille entfernt, damit kein Ausklang abgehackt wird.
    """
    cand_dir = outdir / "candidates"
    all_cands: list[tuple[float, Path]] = []       # (Sekunden, Datei) ueber alle Versuche
    best: tuple[float, Path, float] | None = None  # (Abweichung, Datei, gemessene Sekunden)
    cur, neg = prompt, ""
    for attempt in range(1, attempts + 1):
        print(f"  Versuch {attempt}/{attempts} (Ziel {target_s:.1f}s ±{tol})")
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
            marker = " (>= Ziel, kuerzbar)" if sec >= target_s else ""
            print(f"    {f.name}: {sec:.2f}s Ton (Abweichung {diff:.2f}s){marker}")
            if best is None or diff < best[0]:
                best = (diff, f, sec)
        # Brauchbar ist alles, was >= Ziel liegt (Kuerzen mit Ausblendung ist verlustfrei
        # moeglich) oder innerhalb der Toleranz — Verlaengern geht nicht.
        usable = [c for c in attempt_secs if c >= target_s] or []
        if (best and best[0] <= tol) or usable:
            break
        if attempt < attempts and attempt_secs:
            cur, neg = tune_prompt(prompt, max(attempt_secs), target_s)
            time.sleep(PACING_S)

    if not all_cands:
        print(f"  FEHLER: kein Kandidat fuer {name}")
        return None
    # Auswahl: kuerzester Kandidat, der noch >= Ziel liegt (wird sauber gekuerzt).
    # Gibt es keinen, der lang genug ist: der laengste vorhandene.
    long_enough = sorted([c for c in all_cands if c[0] >= target_s])
    chosen_sec, chosen = long_enough[0] if long_enough else max(all_cands)
    final = outdir / f"{name}-a.mp3"
    final.write_bytes(chosen.read_bytes())
    if long_enough:
        print(f"  GEWAEHLT: {final.name} — {chosen_sec:.2f}s Ton, wird auf {target_s:.1f}s "
              f"gekuerzt (Quelle: {chosen.name})")
    else:
        print(f"  GEWAEHLT: {final.name} — nur {chosen_sec:.2f}s Ton, Ziel {target_s:.1f}s "
              f"NICHT erreicht (Quelle: {chosen.name})")
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
        if refresh_state():
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
        for i, item in enumerate(manifest, 1):
            name, prompt = item["name"], item["prompt"]
            loop = bool(item.get("loop", False))
            if all((outdir / f"{name}-{sfx}.mp3").exists() for sfx in "ab"):
                print(f"[{i}/{len(manifest)}] {name}: schon vorhanden, ueberspringe")
                continue
            if item.get("target_s"):
                print(f"[{i}/{len(manifest)}] {name} — Ziel-Laenge {item['target_s']}s")
                make_sound_targeted(
                    s, prompt, name, outdir,
                    target_s=float(item["target_s"]),
                    tol=float(item.get("tol", 0.4)),
                    attempts=int(item.get("attempts", 3)),
                    loop=loop,
                )
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
        print("Nachbearbeitung (Stille weg + auf Ziel-Laenge kuerzen + Lautheit + OGG):")
        post = Path(__file__).resolve().parent / "sfx-post.sh"
        subprocess.run([str(post), str(outdir), str(outdir.parent / "final")], check=False)
        return

    sys.exit(f"Unbekanntes Kommando: {cmd}\n{__doc__}")


if __name__ == "__main__":
    main()
