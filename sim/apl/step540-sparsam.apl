# Die aussagekraeftigste Messung dieser Reihe: EIN Verstaerker MIT Bedingung.
#
# Zu lesen gegen: step540-ohne.apl.
#
# Gemessen: 4224 gegen 4240 — also SCHLECHTER als ganz ohne ihn. Das ist der
# Befund, der traegt: Selbst ein bedingt gewirkter Verstaerker kostet mehr
# globale Wartezeit, als er einbringt.
#
# Die Verdopplung trifft eine NEBENGROESSE (Hitze), der Preis ist Schaden.
# Siehe #550 — dasselbe Muster ueber 37 Zauber hinweg.

flashpoint if heat < 5
smoulder   if dot.smoulder.remains < gcd
pyre       if heat >= 15
kindle
