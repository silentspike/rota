# Stil-Kandidaten — Entscheidungsvorlage zu Issue #19

> **Das ist keine Auslieferung.** Nichts in diesem Ordner landet im Spiel-Build. Er existiert, damit
> der Auftraggeber vier echte freie Asset-Sets **nebeneinander** sieht und den Stil an dem festlegt,
> was er sieht — nicht an einer Beschreibung (GAME-DESIGN 17.2).

## Ansehen

```
http://localhost:8000/rota-e9-style/
```

Am besten bei mindestens **1900 px** Fensterbreite — dann stehen alle vier Karten nebeneinander.
Darunter scrollt die Reihe waagerecht; sie bricht bewusst nicht um, weil ein Vergleich untereinander
keiner mehr wäre.

Die Seite lädt ausschließlich aus diesem Ordner. Kein fremder Server, keine Web-Schriften.

## Was hier liegt

| Ordner          | Inhalt                                                                |
| --------------- | --------------------------------------------------------------------- |
| `icons/`        | Zaubersymbole beider tragfähiger Quellen                              |
| `ui/`           | Rahmen und Tafeln von Kenney                                          |
| `fonts/`        | Die vier Schriften, jeweils in der Fassung, die die Seite lädt        |
| `textures/`     | Zwei CC0-Metalltexturen, auf 512 px verkleinert                       |
| `LICENSES/`     | Die Original-Lizenztexte aller Quellen, unverändert aus den Paketen   |
| `rejected/`     | Kontaktbogen des geprüften und verworfenen CC0-Symbolsatzes           |
| `manifest.json` | Maschinenlesbare Liste aller Kandidatendateien mit Urheber und Schule |

Herkunft, Lizenz und Nennungspflicht jeder einzelnen Datei stehen in
[`../../ASSETS.md`](../../ASSETS.md), Abschnitt „Stil-Kandidaten".

## Wie es weitergeht

1. Der Auftraggeber wählt einen Kandidaten — oder benennt, was er aus mehreren zusammensetzen will.
2. Die gewählten Dateien wandern nach `assets/img/` bzw. `assets/fonts/` und werden in `ASSETS.md`
   in den Bestand eingetragen.
3. Enthält die Wahl ein CC-BY-Asset — und das tun **alle vier**, weil es für 26 Zaubersymbole keinen
   reinen CC0-Weg gibt —, braucht das Spiel einen Credits-Bereich. Das ist eine Lizenzauflage.
4. Dieser Ordner bleibt als Beleg der Entscheidung liegen.

## Was hier bewusst nicht steht

- **Kein HUD-Entwurf.** Die Anordnung steht in GAME-DESIGN 9.1 fest und wird hier nicht nachgebaut.
- **Keine erfundenen Zaubernamen.** Die 60 Zauber sind noch nicht ausgeplant (GAME-DESIGN 22). Die
  Symbole tragen deshalb ihre Asset-Namen, nicht Spiel-Namen.
- **Keine erfundenen Werte.** Es kommen nur ESSENCE, VESSEL, WARD, CATALYSIS, PURITY und YIELD vor
  (GAME-DESIGN 6.1 und 6.2).
- **Keine erzeugte Bildvorlage.** Der Stil kommt von einem menschlichen Gestalter.
