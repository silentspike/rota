# Referenz-Spielweise der Fortschritts-Simulation (#922, Fassung A).
#
# Die eingefuehrte gute Rotation mit derselben QUENCH/PURGE-Antwort wie die
# Fortschrittskaempfe aus #1057 (tests/parity/phase-bindings-production.ts,
# playPhaseBoss, Rotation "good"):
#
#   QUENCH, sobald der Boss einen Zauber wirkt
#   PURGE, wenn die Verhaengnis-Uhr eine Senkung lohnt und Ladungen da sind
#   SMOULDER erneuern, wenn er auslaeuft
#   PYRE ab der Bonus-Schwelle
#   sonst KINDLE
#
# Die Schwelle fuer PURGE steht dort als `clearAmount` des Bosses (40 bei
# boss-1-2 und boss-5-4, 45 bei boss-3-4). Eine Liste kennt den Boss nicht;
# hier steht die kleinere der beiden. Bei boss-3-4 senkt sie damit fuenf
# Punkte frueher als die Vorlage.
#
# Sie ist eine WAHL, keine Ableitung: "schafft ein durchschnittlicher Spieler
# das?" haengt daran, was man unter durchschnittlich versteht. Wer eine andere
# messen will, gibt sie mit --apl an.

quench   if boss.casting >= 1
purge    if doom_charges > 0 and doom >= 40
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
