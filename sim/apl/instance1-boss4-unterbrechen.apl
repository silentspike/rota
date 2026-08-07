# Die Rotation, gegen die boss-1-4s Lebenspunkte gemessen sind (#544 AC2).
#
# Zu lesen gegen: instance1-roh.apl.
#
# ACHTUNG: Der APL-Parser kann quench NICHT bedingen (#553) — diese Datei
# drueckt ihn deshalb unbedingt und misst NICHT dasselbe wie der Prueffall
# tests/unit/instance-1-signaturen.test.ts, der `casting !== -1` prueft.
#
# Die Lebenspunkte 4900 stammen aus dem PRUEFFALL, nicht aus dieser Datei.
# Sie liegt hier, damit sichtbar ist, was der Laeufer sehen wuerde — und
# warum seine Zahl (4380) neben den beiden des Prueffalls (5368 roh brutto,
# 5196 unterbrechend) steht, ohne mit ihnen vergleichbar zu sein: drei
# verschiedene Zuschnitte.
quench
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
