#!/usr/bin/env bash
# Warnt den, der RECHNET — nicht den, der nachsieht. (Issue #1049)
#
# Am 2026-08-21 und in der Nacht zum 22.08. hat viermal jemand gerechnet,
# während ein anderer das Messfenster hielt: `npm run lint` über das ganze
# Repository, `prettier --write` auf ein 345-KB-Dokument, `jq` über alle
# Aufzeichnungen, drei Wächter-Läufe, ein `git worktree add/remove`.
#
# DER SCHALTER WAR JEDES MAL KORREKT GESETZT UND KORREKT LESBAR. Er hat nur
# niemanden erreicht, weil niemand nachgesehen hat. Alle vier Male kannte der
# Verursacher die Regel im Wortlaut und hat sie am selben Abend anderen
# gegenüber vertreten.
#
#   Ein Vorsatz hilft gegen einen Fehler, den man kommen sieht. Dieser wird
#   nicht gesehen: `npm run lint` fühlt sich nicht wie eine Messung an, also
#   fühlt es sich außerhalb der Regel an. Für die fremden Zahlen ist das
#   gleichgültig.
#
# ═══════════════════════════════════════════════════════════════════════════
#  WAS DIESE WARNUNG NICHT ERREICHT
# ═══════════════════════════════════════════════════════════════════════════
#
# Sie hängt an zwei Stellen: an `pre*`-Skripten in package.json und an den
# Läufern unter tools/. Beides ist PROJEKTCODE. Ein Hook, der jeden
# Bash-Aufruf abfängt, wäre vollständig — er bräuchte aber eine Änderung an
# `.claude/settings.json`, und das ist eine Entscheidung des Auftraggebers,
# nicht des Teams.
#
# Folglich schweigt sie bei:
#
#   npx prettier --write <datei>   geht an package.json vorbei
#   npx <irgendwas>                dito
#   jq über die Aufzeichnungen
#   git worktree add/remove, git push
#   cargo/rustc direkt aufgerufen (nicht über tools/)
#   alles, was direkt gestartet wird statt über `npm run`
#
# DER ERSTE DER VIER VORFÄLLE — `npx prettier --write` auf den Guide — FÄLLT
# ALSO DURCH. Das steht hier, weil eine Abhilfe, die vollständig aussieht,
# gefährlicher ist als gar keine: Wer glaubt, gewarnt zu werden, sieht nicht
# mehr selbst nach.
#
# ═══════════════════════════════════════════════════════════════════════════
#
# Sie BLOCKIERT NICHT (Fertig-Kriterium 5). Wer in einem fremden Fenster
# rechnen muss, darf das — er soll es nur nicht versehentlich tun. Exit ist
# immer 0.

set -u

STAND="${HOME}/.claude/rota-fenster.json"

# **Gibt es das Messfenster in dieser Umgebung überhaupt?** Wo kein
# `rota-fenster` installiert ist, teilt sich niemand eine Maschine mit
# anderen Sitzungen — dann ist Schweigen richtig und jede Meldung Rauschen.
#
# Das ist kein theoretischer Fall: Der erste Entwurf hat in der CI bei JEDEM
# Lauf angeschlagen. `/home/runner/.claude/rota-fenster.json` gibt es dort
# nicht, also lief der Zweig unten — bei `npm test`, `npm run lint`, `build`
# und `typecheck`, in jedem Durchgang jedes PRs.
#
# Damit war die erste Auflage des Issues verletzt: Eine Warnung, die immer
# kommt, ist von einer abgeschalteten nicht zu unterscheiden. Gefunden habe
# ich es nicht durch die Gegenprobe — die läuft nur auf DIESER Maschine —,
# sondern im CI-Protokoll des eigenen Pull Requests.
if ! command -v rota-fenster > /dev/null 2>&1; then
  exit 0
fi

# Hier dagegen IST das Werkzeug da, und dann ist ein fehlender Stand
# verdächtig. **Nicht als "frei" behandeln** — ein fehlender Messwert ist kein
# grüner Messwert (#1049, Kriterium 4). Anders als beim Schalter selbst wird
# hier nichts blockiert, also genügt eine Zeile.
if [ ! -f "$STAND" ]; then
  echo "  [Fenster] Der Schalterstand ist nicht lesbar ($STAND fehlt)." >&2
  echo "            Ob jemand misst, ist damit UNBEKANNT — sieh selbst nach." >&2
  exit 0
fi

# Die eigene Rolle aus dem Werkzeug, nicht aus dem Pfad geraten: `rota-ipc`
# prüft sie zusätzlich gegen den laufenden Prozess.
MEINE_ROLLE="$(rota-ipc wer 2>/dev/null | awk '/^Rolle:/ {print $2; exit}')"

python3 - "$STAND" "${MEINE_ROLLE:-}" <<'PYTHON'
import json
import sys

stand_pfad, meine_rolle = sys.argv[1], sys.argv[2]

try:
    with open(stand_pfad, encoding="utf-8") as datei:
        stand = json.load(datei)
except (OSError, ValueError) as fehler:
    print(f"  [Fenster] Schalterstand unlesbar ({fehler}) — sieh selbst nach.",
          file=sys.stderr)
    sys.exit(0)

inhaber = stand.get("inhaber")
abgabe = stand.get("abgabe")

# ---- Die drei Fälle, in denen GESCHWIEGEN wird ---------------------------
#
# Auflage 1 aus der Zuteilung, und sie ist die wichtigere Hälfte: Eine
# Warnung, die immer kommt, ist von einer abgeschalteten nicht zu
# unterscheiden. Genau daran ist am 2026-08-22 ein Zwei-Leerzeichen-Detektor
# gescheitert (1328 Treffer auf 1416 Nachrichten).
if not inhaber:
    sys.exit(0)                    # niemand hält es
if abgabe is not None:
    sys.exit(0)                    # abgegeben, also frei
if meine_rolle and inhaber == meine_rolle:
    sys.exit(0)                    # es ist mein eigenes Fenster

# ---- Und der eine, in dem gewarnt wird -----------------------------------
zweck = (stand.get("zweck") or "").strip()
if len(zweck) > 150:
    zweck = zweck[:147] + "..."

print("", file=sys.stderr)
print("  ┌─ MESSFENSTER ─────────────────────────────────────────────────",
      file=sys.stderr)
print(f"  │  {inhaber} misst gerade. Seit {stand.get('seit')}.", file=sys.stderr)
print(f"  │  Zweck: {zweck}", file=sys.stderr)
print("  │", file=sys.stderr)
print("  │  Was du gleich startest, belastet dieselbe Maschine. Seine",
      file=sys.stderr)
print("  │  Zahlen werden dadurch falsch, und er merkt es nicht.",
      file=sys.stderr)
print("  │", file=sys.stderr)
print("  │  DAS IST KEINE SPERRE. Wenn du weisst, dass du es tust, fahr",
      file=sys.stderr)
print("  │  fort — und sag es ihm. Wenn du es nicht wusstest: warte auf",
      file=sys.stderr)
print("  │  seine Abgabe.", file=sys.stderr)
print("  └───────────────────────────────────────────────────────────────",
      file=sys.stderr)
print("", file=sys.stderr)
PYTHON

exit 0
