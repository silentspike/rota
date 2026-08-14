---
id: paar-blindheit-558
status: Review
ssot: false
refs:
  docs: [GAME-DESIGN.md, berichte/pruefstein-509.md]
---

# Die Rotations-Suche ist paar-blind — und das war nicht der Grund

**Issue #558.** Gemessen am 2026-08-14 von Worker-3.

Der wichtigste Befund dieses Berichts ist **negativ**, und deshalb steht er
hier: Die Paar-Blindheit ist real, sie ist behoben — und sie war nicht die
Ursache dessen, was man ihr zugeschrieben hatte. Ein negativer Befund, den
niemand findet, wird in sechs Wochen ein zweites Mal erarbeitet.

## Die Blindheit, präzise

Die Kandidatenzeile gibt es seit #553:

```text
kindle if buff.flashpoint.active
```

Sie ist nicht der Mangel. Der Mangel ist, dass der Verstärker **selbst** nie
in die Liste kommt:

| Zeile                              | allein                          |
| ---------------------------------- | ------------------------------- |
| `flashpoint`                       | kostet einen GCD, bringt nichts |
| `kindle if buff.flashpoint.active` | der Buff ist nie da, also tot   |

Ein gieriges Verfahren bewertet jede Zeile einzeln, beide verlieren, also
nimmt es keine — und die zweite **kann** nicht gewinnen, solange die erste
fehlt. Das Paar ist die kleinste Einheit, die überhaupt etwas wert ist.

## Das Verfahren (AC2)

`paar_zuege` bildet die Paare aus den **Daten**: Eine BuffWindow-Zeile trägt
den Verstärker in ihrer Bedingung, ihr Partner ist dessen unbedingte Zeile.
Wer einen Verstärker ergänzt, bekommt die Paare ohne Zutun.

**Warum es billig bleibt** — die Antwort auf „oder begründe, warum es zu teuer
ist": Alle Paare aus allen Zeilen wären bei 88 Kandidaten 3828 Stück mal
Einfügepositionen zum Quadrat. Die Vorauswahl aus den Daten lässt die übrig,
die eine Beziehung haben. Dazu zwei Beschränkungen: Der Verstärker steht
direkt vor dem Nutzer (eine Position statt zwei), und das Paar wird erst
gefragt, wenn kein einzelner Zug mehr trägt — dort, wo `greedy` vorher
aufgab.

`greedy_opts(..., mit_paaren: bool)` schaltet es ab. Ohne diesen Schalter wäre
jede Aussage über den Gewinn unbelegt.

## Der Befund: die Paare finden nichts

```text
225 Paare geprüft, Abbruch NoImprovement, kein einziges gewinnt
```

Von Hand nachgemessen, alle sieben Verstärker an zwei Kampflängen — keiner
gewinnt:

| Verstärker  | `first-crucible` (60 s) | `test-metronome` (300 s) |
| ----------- | ----------------------- | ------------------------ |
| ohne        | 9068                    | 46150                    |
| slag        | 8276                    | 44767                    |
| flashpoint  | 7976                    | 43249                    |
| quicksilver | 8236                    | 41462                    |
| griplock    | 8200                    | 44306                    |
| distil      | 7993                    | 41850                    |
| aether      | 8510                    | 45303                    |
| virulence   | 8442                    | 45734                    |

**Das ist ein Balance-Befund, kein Werkzeug-Befund.** Ein Verstärker kostet
einen GCD und spielt ihn nicht ein.

### Und das Paar wird neutralisiert, nicht abgelehnt

Erzwingt man das Paar als Startpunkt und lässt die Suche die Umgebung bauen,
liefern alle sieben Verstärker an `first-crucible` **exakt dieselbe Zahl** wie
die freie Suche: 9742. Identische Zahlen über verschiedene Eingaben sind nie
ein Ergebnis, sondern ein Hinweis. Die ausgeschriebene Liste zeigt worauf:

```text
  ... sechs Zeilen ...
  kindle
  flashpoint                          <- ans ENDE geschoben
  kindle if buff.flashpoint.active
```

Die Suche kann ein erzwungenes Paar nicht ablehnen, aber sie kann es dorthin
schieben, wo das unbedingte `kindle` ohnehin alles abfängt. Das entwertet den
Befund nicht, es schärft ihn: Jede Position wurde geprüft, und keine ist
besser als „ganz unten".

## Der eigentliche Hebel: der Startpunkt

Der Prüfstein war Worker-1s Messung an Gegenstandsstufen: Stufe 7 liefert
weniger Durchsatz als Stufe 1, obwohl sie mehr Budget hat. Die Paar-Suche
ändert daran **nichts** — bei 200 geprüften Paaren und Abbruch wegen
fehlender Verbesserung, nicht wegen voller Liste.

Der Kreuzvergleich über die Stufen (dasselbe Werkzeug wie in #509, andere
Achse) zeigt, woran es wirklich liegt:

```text
                 Stufe 1    Stufe 7   Stufe 13
  von Stufe 1      13655      15593      19131
  von Stufe 7      10290      12671       4478
  von Stufe 13      7178      15593      19131
```

Die bei Stufe 1 gefundene Liste holt bei Stufe 7 **exakt 15593** — den
Sollwert, den eine feste Rotation dort erreicht. Und die Zeile `von Stufe 7`
ist der Beleg für das lokale Optimum: Dieselbe Liste holt bei Stufe 13 nur
4478 statt 19131. Sie ist nicht schlecht gefunden, sie ist in eine Sackgasse
gelaufen, die nur bei ihrer eigenen Stufe trägt.

Der Nachlauf aus #558 — Startpunkte tauschen — löst es:

```text
  Runde 1  Stufe 7   12671 -> 16474 über Stufe 1
  Runde 2: Fixpunkt

  Stufe    nachher   Sollwert
    1       13655       -      unverändert
    7       16474    15593     +5,7 % über dem Soll
   13       19131       -      unverändert
```

**16474 liegt über der festen Liste**, weil der Nachlauf von ihr aus
weitersucht statt sie nur zu übernehmen.

### Die Falle darin, gemessen an meinem eigenen Baustein

Ich hatte behauptet, der Nachlauf sei nie schlechter als die eigene Suche.
Der Prüffall dazu wurde rot:

```text
der Nachlauf ist schlechter als die eigene Suche: 3138.0 gegen 6553.0
```

Die Behauptung war falsch, nicht der Code. `greedy_from` gibt zurück, was von
**diesem** Startpunkt aus erreichbar ist — mehr verspricht es nicht. Ist die
Startliste schon so lang wie `max_len`, greift sofort `Stop::LengthLimit`: Es
wird gar nicht gesucht, und heraus kommt die fremde Liste, an der neuen Stelle
gespielt. Taugt sie dort nichts, taugt das Ergebnis nichts.

**Der wesentliche Teil des Nachlaufs ist deshalb die Besser-Prüfung**, und die
steht im Läufer, nicht in der Bibliothek. Wer `greedy_from` ohne sie benutzt,
verschlechtert sein Ergebnis, und die Zahl sieht dabei plausibel aus. Der Fall
`ohne_besser_pruefung_verschlechtert_der_nachlauf` hält die Falle fest.

## Vier Fehlschlüsse unterwegs — alle an identischen Zahlen erkannt

Keiner davon fiel durch einen roten Test auf. Alle vier verrieten sich durch
**zwei gleiche Zahlen, wo verschiedene stehen mussten**.

1. **Die Fenster-Zeile stand über dem unbedingten `kindle`** und konnte
   nichts verdrängen — zweimal exakt 8621. Der Läufer bricht jetzt laut ab,
   wenn zwei Listen sich in einer Zeile unterscheiden und dasselbe liefern.
2. **Alle sieben Verstärker in derselben CALCINATION-Basis gemessen**, obwohl
   `virulence` Stapel verstärkt, `griplock` Kontrolle, `quicksilver`
   Umwandlung. Vier von sieben konnten dort gar nicht wirken.
3. **Das erzwungene Paar wurde ans Ende geschoben** — siehe oben.
4. **Sechs Ausrüstungs-Verteilungen probiert, keine traf.** Was fehlte, war
   nicht die Verteilung, sondern die Zahl der Plätze: Worker-1 verteilt das
   Budget für jeden der zehn Ausrüstungsplätze und summiert. Ich hatte es
   einmal verteilt und lag um eine Größenordnung daneben.

Dazu ein Werkzeug-Fehler, der jeden treffen kann: Eine neue `pub fn` wurde vom
Beispiel-Läufer nicht gefunden, obwohl die Quelle mit identischer md5 auf dem
Build-Server lag. rsync überträgt die mtime, cargo bindet gegen die alte
rlib. `find sim/src sim/examples -name '*.rs' -exec touch {} +` behebt es —
ohne eine Zeile Code zu ändern.

## Eine widerlegte Vermutung, festgehalten

Nach der AC1-Messung hatte ich gemeldet: _„Wenn eine feste Liste 18,7 % mehr
holt als die Suche bei vollem Raum und mit Paaren, steht in dieser Liste eine
Zeile, die `all_candidates` gar nicht bilden kann."_

**Das war falsch.** Die Liste besteht aus vorhandenen Kandidaten; sie wird nur
nicht gefunden, weil die Suche bei Stufe 7 anders startet. Die Vermutung ging
als Vorschlag raus, nicht als Befund — das war der Unterschied zwischen einer
Notiz und einem Issue, das jemand bearbeitet hätte.

## Was offen bleibt

**Die Paar-Suche ist gebaut und findet an den heutigen Daten nichts.** Ob sie
je etwas findet, hängt daran, ob ein Verstärker seinen GCD einspielt — eine
Balance-Frage, keine Werkzeug-Frage.

**Der Startpunkt-Tausch ist nur im Prüfstein auf die Stufen-Achse angewandt.**
Der produktive Läufer für Gegenstandsstufen ist `sim/examples/stufen-durchsatz.rs`
und gehört Worker-1; solange er die je Stufe frei gefundene Rotation nimmt,
trägt jede seiner Zahlen die Startposition der Suche mit.

## Die Messung wiederholen

    cargo remote -c --manifest-path sim/Cargo.toml -- run --release \
      --example pruefstein-558

Der Läufer bricht laut ab, wenn die Budget-Reihe nicht mehr zu Worker-1s
Messung passt — dann misst er eine andere Größe als sie, und jeder Vergleich
darunter wäre wertlos.
