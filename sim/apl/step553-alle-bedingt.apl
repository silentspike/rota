# AC4 (#553): DIESELBEN FUENF VERSTAERKER, aber MIT BEDINGUNG.
#
# Zu lesen gegen: step540-nutzer.apl (2400), NICHT gegen step540-ohne.apl.
# `diff nutzer step553-alle-bedingt` ergibt genau die fuenf Verstaerker-Zeilen
# und die Bedingungen an den Erzeugern — Worker-3s Aufteilung, uebernommen.
#
# Die Wiederholung der AC6-Messung aus #540. Dort standen die fuenf ganz vorn
# und ohne Bedingung: hoechste Prioritaet, gewirkt sobald bereit. Das mass die
# naive Nutzung, nicht den Zauber.
#
# ZWEI ARTEN VON BEDINGUNG, und beide brauchen buff-Fenster:
#
#   Der Verstaerker selbst nur, wenn sein Fenster NICHT laeuft — sonst wirft
#   er sich auf sich selbst und verschenkt die Restzeit.
#
#   Der Erzeuger nur, wenn das Fenster OFFEN ist — das ist der ganze Zweck
#   einer Verstaerkung auf eine Nebengroesse.

flashpoint  if buff.flashpoint.remains < 1
griplock    if buff.griplock.remains < 1
virulence   if buff.virulence.remains < 1
distil      if buff.distil.remains < 1
quicksilver if buff.quicksilver.remains < 1
bind        if buff.griplock.active
canker      if buff.virulence.active
smoulder    if dot.smoulder.remains < gcd
pyre        if heat >= 15
kindle
