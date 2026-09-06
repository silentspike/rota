# Produktions-Phasenbindungen (#1057)

`boss-1-2` bindet den Schlag bei 40 s an Phase 1; `boss-3-4` den Schlag bei 18 s. Ohne erreichte Phase verfällt das Ereignis. HP und globale Kampfregeln bleiben unverändert.

Die Prüfung spielt echte Produktionsbosse mit Level 1, ohne Talente, legendäre oder Satz-Effekte. Die Ausrüstung besteht aus zehn gemischten Einzelstücken normaler Beute: beim ersten Boss aus dem tatsächlich vorhandenen Prolog (Beutestufe 1), beim zweiten aus Instanz 2. Das ist ein ausgerüsteter Fortschrittsstand, keine Behauptung, dass ein neuer Spieler bereits alle zehn Teile besitzt. Nackter Einstieg wird gesondert geprüft und gewinnt gegen `boss-1-2` nicht.

| Fall     | Gute Rotation, Seeds 0–31 | Schaden bei 20 s gegenüber schlechter Rotation | Zusätzlicher erlittener Schaden durch Phasenbindung |
| -------- | ------------------------- | ---------------------------------------------- | --------------------------------------------------- |
| boss-1-2 | 32 Siege, 47,22–55,51 s   | +64,9 bis +90,3 %                              | 245 je Lauf                                         |
| boss-3-4 | 32 Siege, 28,51–31,66 s   | +57,2 bis +71,4 %                              | 228 je Lauf                                         |

`tests/unit/phase-bindings-production.test.ts` prüft Wirkung, Überleben, mindestens eine Sekunde Reserve und den Rotationsabstand. Die negative Kontrolle entfernt nur die Bindung und verlangt dann identischen Schaden mit und ohne Phasen.

`tests/parity/phase-bindings-production.ts` exportiert vier Aufzeichnungen: beide Bosse mit und ohne Phasen, bei identischen Eingaben und Ausrüstung. `sim/tests/parity_production_phases.rs` vergleicht Schaden, erlittenen Schaden, Sieg/Tod, angenommene/abgelehnte Eingaben, Zauberhäufigkeiten und alle Taktprüfsummen. Die bestehende nackte Aufzeichnung `instance3-boss-4.json` ändert ihre HP-/Prüfsummenspur ebenfalls: der Schlag bei 18 s entfällt dort, weil die Phase noch nicht erreicht ist. Ihre Eingaben und Schadenssumme bleiben gleich.

Das belegt die zwei Bindungen. Die Gesamtkalibrierung aller Instanzen und Bossstufen (#791) bleibt separat offen.
