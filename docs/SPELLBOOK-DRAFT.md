# Zauberbuch-Draft

Der Draft setzt GAME-DESIGN 12.3 um: Nach einem Sieg über einen spielbaren Boss
wählt der Spieler einen bisher unbekannten Zauber aus bis zu drei Angeboten.
Bei weniger verbleibenden Zaubern wird das Angebot kleiner; ein vollständiger
verfügbarer Katalog erzeugt kein leeres Auswahlfenster.

Die zwei festen Startzauber sind Kindle und Smoulder. Die schulunabhängigen
Werkzeuge Quench und Purge bleiben unabhängig vom Draft verfügbar, wie in
`data/spells/common.json` begründet. Die vorhandenen `unlockLevel`-Werte bestimmen,
welche Zauber angeboten werden können; Prüfzauber über Stufe 100 sind ausgeschlossen.
Die drei geführten Lernkämpfe sind damit noch nicht umgesetzt.
Trainings- und Prüfziele erzeugen Auswertungen, aber keine Beute, Erfahrung,
Punkte, Boss-Freischaltungen oder Draft-Angebote.

**Benannte Implementierungsentscheidung:** Solange kein COAGULATION-Zauber
erworben wurde, hat jeder solche Kandidat Gewicht 2 und jeder andere Gewicht 1.
Danach gilt für alle Gewicht 1. Das Design verlangt eine stärkere Gewichtung
fehlender Defensive, gibt aber kein Zahlenverhältnis vor. Das Verhältnis 2:1 ist
der gewählte, testbare Ausgangswert und keine bereits abgenommene Balancezahl.
Der Draft nutzt einen eigenen RNG-Strom, getrennt von Kampf und Beute.

Das Angebot steht nach dem Kampf in der Auswertung und im Player Home. Ein
gleichzeitig freigeschalteter Archivtext behält seinen bestehenden Dialog; nach
dessen Schließen steht die Auswahl in der Auswertung. Vor dem nächsten Versuch
muss gewählt werden. Die englische Erklärung sagt, dass das Angebot gespeichert
bleibt. Ein Neuladen zeigt das Player Home mit exakt derselben Reihenfolge;
ein Neustart würfelt kein neues Angebot. Die Auswahl belegt keinen Zauberplatz
gegen eine gespeicherte Belegung des Spielers.

Save-Schema 5 speichert erworbene Kennungen und das offene Angebot. Alte Saves
ohne Zauberbuch behalten sämtliche auf ihrer Stufe bereits verfügbaren Zauber
und alle realen Zauber ihrer gespeicherten Belegung. Es werden keine vorhandenen
Gegenstände, Talente, Erfahrungspunkte oder Kampfergebnisse entfernt. Beschädigte
Angebotsstrukturen verwenden die vorhandene Save-Rotation; entfernte Katalogeinträge
werden aus offenen Angeboten herausgenommen, damit sie keinen dauerhaften
Startblocker bilden. Erworbene Kennungen bleiben dabei gespeichert.

Das Entwicklungswerkzeug zum Setzen der Spielerstufe schaltet weiterhin alle
Zauber dieser Stufe für Kit-Prüfungen frei. Der normale Spieler-Aufstieg tut das
nicht: Dort bestimmt der erworbene Besitz die Zusammenstellung.
