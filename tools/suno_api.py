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
    python suno_api.py auth-test
    python suno_api.py gen "<prompt>" <basename> [--loop] [--outdir DIR]
"""
from __future__ import annotations

import json
import re
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


def fresh_token(s: requests.Session) -> str:
    """Kurzlebigen Clerk-JWT neu ausstellen (gueltig ~60 s)."""
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
        sys.exit(f"FEHLER Token-Refresh: HTTP {r.status_code} — {r.text[:300]}")
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


def generate(s: requests.Session, jwt: str, prompt: str, loop: bool = False) -> list[str]:
    """Sound generieren. Gibt die Clip-IDs der (zwei) Entwuerfe zurueck."""
    title = " ".join(w.capitalize() for w in prompt.split()[:12])
    payload = {
        "token": None,
        "task": "sound",
        "generation_type": "TEXT",
        "title": title,
        "tags": prompt,
        "negative_tags": "",
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
    if r.status_code not in (200, 201):
        sys.exit(f"FEHLER generate: HTTP {r.status_code} — {r.text[:500]}")
    data = r.json()
    clips = data.get("clips") or ([data] if data.get("id") else [])
    ids = [c["id"] for c in clips]
    if not ids:
        sys.exit(f"FEHLER: keine Clip-IDs in Antwort: {json.dumps(data)[:400]}")
    print(f"  generate OK — {len(ids)} Entwuerfe: {', '.join(ids)}")
    return ids


def download(ids: list[str], name: str, outdir: Path, tries: int = 40) -> list[Path]:
    outdir.mkdir(parents=True, exist_ok=True)
    dl = requests.Session(impersonate=BROWSER)
    out: list[Path] = []
    for suffix, cid in zip("ab", ids):
        target = outdir / f"{name}-{suffix}.mp3"
        for _ in range(tries):
            r = dl.get(f"{CDN}/{cid}.mp3")
            if r.status_code == 200 and len(r.content) > 5000:
                target.write_bytes(r.content)
                print(f"  {target}  ({len(r.content) // 1024} KB)")
                out.append(target)
                break
            time.sleep(6)
        else:
            print(f"  FEHLER: {cid} nach {tries} Versuchen nicht verfuegbar")
    return out


def main() -> None:
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    s = session()

    if cmd == "auth-test":
        jwt = fresh_token(s)
        r = s.get(f"{API}/billing/info/", headers=api_headers(jwt))
        print(f"Token: {jwt[:24]}...  billing/info: HTTP {r.status_code}")
        if r.status_code == 200:
            info = r.json()
            print(f"  Credits: {info.get('total_credits_left')}  Tier: {info.get('subscription_type')}")
        else:
            print(f"  {r.text[:300]}")
        return

    if cmd == "gen":
        prompt, name = sys.argv[2], sys.argv[3]
        loop = "--loop" in sys.argv
        outdir = Path("/work/rota/assets/sfx/raw")
        if "--outdir" in sys.argv:
            outdir = Path(sys.argv[sys.argv.index("--outdir") + 1])
        print(f"[1/2] generate ({'loop' if loop else 'one-shot'}): {prompt[:60]}...")
        ids = generate(s, fresh_token(s), prompt, loop)
        print("[2/2] download...")
        download(ids, name, outdir)
        print(f"FERTIG: {name}")
        return

    sys.exit(f"Unbekanntes Kommando: {cmd}\n{__doc__}")


if __name__ == "__main__":
    main()
