# AC2 (#553): SCHARF — nur im letzten Drittel des Fensters.
#
# Die Abstufung wie bei `decay` in #526: nie / mit / scharf. `remains` misst
# in Millisekunden; ein Fenster von 3 GCDs sind 4500 ms, "unter 1600" heisst
# also die letzte Wartezeit.
#
# Damit ist belegt, dass `remains` eine ZAHL liefert und nicht nur ein Ja/Nein
# mit anderem Namen.
flashpoint
bellows  if buff.flashpoint.remains < 1600 and buff.flashpoint.active
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
