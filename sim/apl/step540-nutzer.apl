# DER BEZUGSPUNKT FUER step540-alle.apl — nicht fuer step540-ohne.apl.
#
# Diese Datei ist der Grund, warum es fuenf Dateien gibt statt drei. Sie
# enthaelt bind und canker, aber KEINEN Verstaerker.
#
# Ohne sie schiene der Verlust von `alle` bei -72 % zu liegen. Tatsaechlich
# kosten bind und canker allein 1840 von 4240 — sie sind keine Hilfszauber,
# sondern schwaechere SCHADENSZAUBER (60 und 45 gegen KINDLEs 100). Sie
# kosten, weil sie weniger bringen, nicht weil sie nichts tun.
#
# Sie muessen in `alle` hinein, damit Kontroll-Punkte und Zersetzungs-Stapel
# ueberhaupt entstehen — GRIPLOCK und VIRULENCE haetten sonst nichts zu
# verstaerken.
#
# Gemessen: 2400 Schaden.

bind
canker
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
