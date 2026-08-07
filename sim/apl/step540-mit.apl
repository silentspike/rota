# Der EINZELNE Verstaerker, ohne Bedingung ganz vorn.
#
# Zu lesen gegen: step540-ohne.apl. `diff` zeigt genau eine Zeile —
# flashpoint. Deshalb ist der Unterschied ihm allein zuzuschreiben.
#
# Gemessen: 4272 Schaden (+32). Der einzige Fall in dieser Reihe, der
# GEWINNT — und nur, weil FLASHPOINT einmal am Anfang zuendet, wenn ohnehin
# nichts Besseres bereit ist.

flashpoint
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
