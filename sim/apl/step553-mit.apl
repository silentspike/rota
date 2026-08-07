# AC2 (#553): die Bedingung greift, wenn das Fenster offen ist.
#
# FLASHPOINT verdoppelt den Ressourcen-Gewinn (target resource, 3 GCDs).
# BELLOWS im offenen Fenster bringt also 16 statt 8 — genau dafuer gibt es
# die Bedingung, und ohne sie war sie nicht formulierbar.
flashpoint
bellows  if buff.flashpoint.active
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
