($a[0]) as $A | ($b[0]) as $B
| [ $A | paths(scalars) ]
| map(. as $p | select( ($A|getpath($p)) != ($B|getpath($p)) ) | ($p | map(tostring) | join(".")))
| unique | .[0:4] | join("  ")
