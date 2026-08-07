# Rotation MIT Unterbrecher — fuer #544 AC2 an boss-1-4.
#
# Zu lesen gegen: instance1-roh.apl. `diff` zeigt genau eine Zeile: quench.
#
# Der Unterbrecher kann heute nicht bedingt werden (#553) — er steht deshalb
# ganz vorn und wird gewirkt, sobald er bereit ist. Was hier gemessen wird,
# ist also die OBERGRENZE dessen, was Unterbrechen kostet, nicht die
# geschickte Nutzung.
quench
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
