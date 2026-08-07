# AC4 (#553), die ISOLIERTE Fassung: nur die fuenf Verstaerker-Zeilen
# unterscheiden sich von step540-nutzer.apl.
#
# `diff nutzer step553-nur-verstaerker-bedingt` ergibt GENAU fuenf Zeilen.
# bind und canker stehen hier unbedingt, wie dort — sonst waeren es sieben
# Unterschiede, und der Gewinn liesse sich nicht den Verstaerkern zuschreiben.
#
# Die Bedingung an den Verstaerkern bleibt: nur wirken, wenn das eigene
# Fenster nicht schon laeuft.

flashpoint  if buff.flashpoint.remains < 1
griplock    if buff.griplock.remains < 1
virulence   if buff.virulence.remains < 1
distil      if buff.distil.remains < 1
quicksilver if buff.quicksilver.remains < 1
bind
canker
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
