# Rotation MIT Reinigung — fuer #544 AC3.
#
# Zu lesen gegen: instance1-roh.apl. `diff` zeigt genau eine Zeile: quench.
#
# quench traegt keinen Schaden. Der Platz kostet also, und die Frage von AC3
# ist, ob es Bosse gibt, bei denen er sich trotzdem lohnt.
quench
smoulder if dot.smoulder.remains < gcd
pyre     if heat >= 15
kindle
