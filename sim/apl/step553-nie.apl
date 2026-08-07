# AC2 (#553): die Bedingung greift NIE — der untere Rand der Abstufung.
#
# `buff.flashpoint.remains > 99999` ist nie wahr. KINDLE faellt damit auf die
# unbedingte Zeile zurueck, und der Lauf muss dieselbe Zahl liefern wie
# step540-mit.apl. Tut er das nicht, misst die Bedingung etwas anderes als
# ihren eigenen Zustand.
flashpoint
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle   if buff.flashpoint.remains > 99999
kindle
