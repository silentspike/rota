# Die gute Rotation aus STEP1-SPEC § 4.
#
# Sie nutzt beide Entscheidungen, die Schritt 1 anbietet:
#
#   1. SMOULDER erst erneuern, wenn er auslaeuft — keine Sekunde frueher, sonst
#      ist die Restlaufzeit verloren UND ein GCD verschwendet
#   2. PYRE erst ab der Bonus-Schwelle zuenden, weil dort die Verdopplung liegt
#   3. sonst KINDLE als Fueller
#
# Sollwert: 4240 Schaden, Boss faellt bei 40510 ms.
#
# Dass ein Zauber bereit sein muss, steht hier bewusst nicht: Eine Regel wird
# uebersprungen, solange ihre Abklingzeit laeuft.

smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
