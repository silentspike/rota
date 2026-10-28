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
# npm-pre*-Skripte und Projektläufer rufen diese direkte Warnung auf.
# Zusätzlich ist tools/fenster-hook.py als PreToolUse/Bash-Hook registriert
# (#1049). Er erfasst auch die gemeldeten direkten npx-prettier- und
# jq-recordings-Aufrufe. Dokumentation, gemessene Kosten und Analysegrenzen:
# docs/FENSTER-HOOK.md.
#
# In Shells ohne diesen Hook bleibt die direkte Warnung auf ihre Aufrufer
# beschränkt. Keine der beiden Formen blockiert einen Befehl oder erteilt
# eine Ausführungsberechtigung.

set -u

# **Übersteuerbar, damit die Gegenprobe den echten Stand nicht anfassen muss**
# (#1081). Der Pfad stand hier fest verdrahtet, und die Gegenprobe hat den
# echten Eintrag deshalb viermal je Lauf durch einen erfundenen ersetzt und
# hinterher zurückgeschrieben.
#
# Das schützt gegen das Liegenbleiben — nicht gegen das ABLESEN währenddessen.
# Wer in einem dieser vier Momente `rota-fenster status` gefahren hätte, hätte
# ein fremdes Fenster gesehen. Dazu: bei SIGKILL läuft kein `trap`, und die
# Sicherungsdatei lag unter `/work/tmp`, das sich alle Sitzungen teilen.
#
# Mit der Übersteuerung entfallen Sicherung, `trap` und alle drei Ausfälle
# ERSATZLOS. Das ist der Unterschied zu einer Absicherung, die in drei Fällen
# versagt: Hier gibt es nichts mehr, was versagen könnte.
STAND="${ROTA_FENSTER_STAND:-${HOME}/.claude/rota-fenster.json}"

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
ort = stand.get("ort")

# Der Ort des Fensters (rota-fenster --ort, #1049). Ein Schalter von vor dem
# 28.10. kennt keinen und gilt als Laptop. Ein unbekannter Ort ist kein
# "frei": ein fehlender Messwert ist kein gruener Messwert (Kriterium 4).
if ort not in (None, "laptop", "node3"):
    print(f"  [Fenster] Schalterstand unlesbar (unbekannter Ort {ort!r}) — sieh selbst nach.",
          file=sys.stderr)
    sys.exit(0)

# ---- Die vier Fälle, in denen GESCHWIEGEN wird ---------------------------
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
if ort == "node3":
    sys.exit(0)                    # npm-Laeufe sind lokale Last; node3 stoert nur der Bau-Dienst

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
