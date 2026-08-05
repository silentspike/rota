# Die gute Rotation fuer den vollen Zauberkasten (Issue #224).
#
# WARUM ES SIE GIBT, UND WARUM step1-good BLEIBT
# ----------------------------------------------
# step1-good nutzt drei Zauber von fuenfzehn. Eine Neumessung mit ihr haette
# EXAKT die alten Zahlen geliefert — die vier neuen Zauber werden nie gewirkt,
# also koennen sie nichts verschieben. Die Zahlen haetten gestimmt, die Aussage
# nicht.
#
# Deshalb steht diese Liste NEBEN der alten und ersetzt sie nicht: Nur so ist
# der Unterschied den ZAUBERN zuzuschreiben und nicht einer geaenderten
# Rotation.
#
# SIE IST EIN MESSWERKZEUG, KEINE LERNHILFE
# -----------------------------------------
# Die Referenz-Rotationen erscheinen nirgends im Spiel — auch nicht als
# Lernhilfe. Was gut ist, findet der Spieler selbst heraus; das ist der Kern
# des Spiels. Diese Liste legt nichts fest, was jemand spielen muss.
#
# DIE REIHENFOLGE UND IHR GRUND
# -----------------------------
# Von oben nach unten, die erste zutreffende Regel gewinnt.
#
# DIE ERSTE FASSUNG STAND KOPF UND WAR HALB SO GUT
# ------------------------------------------------
# Sie hatte die Werkzeuge oben: VAPOUR, REFLUX, FESTER, CANKER, danach den
# Schaden. Gemessen: 2484,9 gegen 5960,0 der alten Liste.
#
# Der Grund ist eine Rechnung, die ich vorher nicht gemacht hatte:
#
#   kindle 100 Schaden | fester 60 | canker 45 | vapour 0 | reflux 0
#
# JEDER Zauber kostet dieselbe Wartezeit. Ein GCD in VAPOUR ist ein GCD, der
# nicht in KINDLE geht. Die Liste tauschte also systematisch Hunderter gegen
# Nullen.
#
# **Der GCD ist die knappe Ressource, nicht die Abklingzeit.**
#
# WANN SICH EIN WERKZEUG-ZAUBER TROTZDEM LOHNT
# --------------------------------------------
#   VAPOUR kostet  1500 ms  (ein GCD)
#   VAPOUR spart   1000 ms x (Zahl der Abklingzeiten, die GERADE laufen)
#
#     eine laufende    1000 gespart, 1500 gekostet   VERLUST
#     zwei laufende    2000 gespart                  knapper Gewinn
#     drei laufende    3000 gespart                  lohnt sich
#
# Deshalb ist VAPOUR kein Zauber, den man nach Priorität wirkt, sondern einer,
# den man wirkt, wenn genug Knappes gleichzeitig läuft. Die Bedingung nennt
# zwei lange Abklingzeiten ausdrücklich, weil die Sprache nicht zählen kann —
# `and` verknüpft, aber es gibt kein "wie viele laufen".
#
# REFLUX holt den teuersten Zauber zurück und lohnt nur, solange der auf
# Abklingzeit liegt. FESTER steht über beiden, weil er der Zauber ist, für den
# sie arbeiten.
#
# WAS DIESE LISTE NICHT IST
# -------------------------
# Das Optimum. Sie ist von Hand geschrieben, und ob eine bessere existiert,
# hat niemand gemessen — die Suche ueber Rotationen ist nicht gebaut
# (Issue #237). Jede Zahl, die mit ihr entsteht, ist eine UNTERGRENZE.
#
# Der Pruefpunkt dazu steht in `sim/tests/rotation_step3.rs`: Diese Liste muss
# step1-good auf demselben Zauberkasten messbar schlagen. Tut sie es nicht,
# ist sie schlecht — nicht die Zauber.

smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
fester
vapour
reflux
canker
