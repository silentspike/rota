"""Die Laengen-Grenze der Klang-Produktion — an EINER Stelle (Issue #215).

WARUM DIESE DATEI EXISTIERT
---------------------------
Die Grenze stand zweimal im Projekt, mit verschiedenen Werten:

    suno_api.py batch     tol = 0.4    nimmt an
    sfx-verify.py         0.05         lehnt ab

`spell-bind` kam damit mit 0,229 s bei Ziel 0,50 s durch die Erzeugung, hat
Credits gekostet und ist an der Abnahme gescheitert — zwei Werkzeuge, dieselbe
Frage, zwei Antworten. Zwei Zahlen an zwei Orten sind eine Handabschrift, und
Handabschriften laufen auseinander.

DIE GRENZE IST NICHT SYMMETRISCH, UND DAS IST DER KERN
-------------------------------------------------------
`sfx-post.sh` kann **kuerzen, nicht verlaengern**:

    zu lang     die Nachbearbeitung repariert es (Beschnitt mit Ausblendung)
    zu kurz     nichts repariert es mehr

Deshalb zwei Regeln aus DERSELBEN Zahl, statt einer Zahl, die zweimal
verschieden ausgelegt wird:

    candidate_usable()  vor der Nachbearbeitung — laenger ist gut, kuerzer
                        nur im Rahmen der Grenze
    length_ok()         nach der Nachbearbeitung — die Laenge muss sitzen,
                        in beide Richtungen, denn jetzt ist geschnitten

Aendert sich LENGTH_TOLERANCE_S, aendern sich beide. Das ist der ganze Zweck.
"""

from __future__ import annotations

# Wie weit die fertige Datei vom Ziel abweichen darf.
#
# 0,05 s, weil sfx-post.sh exakt auf die Ziel-Laenge schneidet: Was danach
# weiter abweicht, ist nicht Toleranz, sondern ein Fehler in der Kette.
LENGTH_TOLERANCE_S = 0.05


def length_ok(actual_s: float, target_s: float) -> bool:
    """Haelt eine FERTIGE Datei ihre Ziel-Laenge? Beidseitig, denn sie ist geschnitten."""
    return abs(actual_s - target_s) <= LENGTH_TOLERANCE_S


def candidate_usable(actual_s: float, target_s: float) -> bool:
    """Laesst sich aus diesem ROHLING noch etwas machen?

    Nach oben offen — die Nachbearbeitung kuerzt. Nach unten nur bis an die
    Grenze, denn verlaengern kann sie nicht.
    """
    return actual_s >= target_s - LENGTH_TOLERANCE_S
