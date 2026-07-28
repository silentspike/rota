# Die schlechte Rotation aus STEP1-SPEC § 4 — plausibel falsch, nicht
# absichtlich dumm. So spielt jemand, der die beiden Regeln noch nicht
# verstanden hat:
#
#   1. erneuert den Brand-Effekt, sobald die halbe Laufzeit vorbei ist —
#      verschenkt damit Restzeit und einen GCD
#   2. zuendet PYRE, sobald es bereit ist — nimmt den Bonus nie mit
#   3. sonst ebenfalls KINDLE
#
# Sollwert: 4044 Schaden, der Boss ueberlebt mit 96,3 %.
#
# Die 6s sind die halbe Laufzeit von SMOULDER (12 s).

smoulder if dot.smoulder.remains < 6s
pyre     if heat > 0
kindle
