# AC3 (Issue #525): DIE KONTROLLE — ein wirkungsloser Zauber auf dem Platz.
#
# Die zwei Dateien daneben unterscheiden sich um ZWEI Dinge: um HARDEN und um
# einen GCD je Wirkung. Ein Unterschied im Ergebnis kann von beidem kommen —
# beim ersten Anlauf kam er von der GCD-Verdraengung.
#
# WEDGE traegt baseDamage 0, keinen Schild und keine Verlaengerung. Er kostet
# einen GCD und tut sonst nichts.
#
# # Diese Kontrolle ist NICHT perfekt, und das gehoert hierher
#
# WEDGEs Abklingzeit ist 6 s, HARDENs sind 10 s. Er wird deshalb oefter
# geworfen und verdraengt anders. Gemessen an test-metronome:
#
#     mit harden    crust 4800, harden 2900
#     mit wedge     crust 5000, wedge  5000
#
# Ein Versuch, ihn ueber `wedge if cooldown.harden.ready` an HARDENs Takt zu
# binden, geht NICHT: In dieser Datei wird HARDEN nie geworfen, ist also immer
# bereit — die Bedingung ist dauerhaft wahr und WEDGE folgt weiter seinem
# eigenen Cooldown. Gemessen, nicht vermutet.
#
# Die Aussage traegt trotzdem, weil sie GEGEN die Verzerrung laeuft: HARDEN
# verdraengt MEHR CRUST (4800 gegen 5000) und faengt trotzdem MEHR ab. Sein
# Beitrag ist damit groesser als die Rohdifferenz, nicht kleiner.
wedge  if cooldown.wedge.ready
crust  if cooldown.crust.ready
kindle
