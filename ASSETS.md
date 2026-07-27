# ASSETS.md — Herkunft, Lizenz, Nennungspflicht

> **Jede Datei unter `assets/`, `docs/refs/` und `docs/style-candidates/` steht hier drin.** Ohne
> Eintrag darf nichts ausgeliefert werden.
> Pflege: Wer ein Asset hinzufügt, ergänzt im selben Commit die Zeile hier.
> Prüfung: `python3 tools/assets-check.py` — meldet jede Datei ohne Eintrag und jede unerlaubte Lizenz.

## Lizenzregel — nicht verhandelbar

| Lizenz                               | Erlaubt? | Auflage                                                       |
| ------------------------------------ | -------- | ------------------------------------------------------------- |
| **CC0 / Public Domain**              | **ja**   | keine                                                         |
| **CC BY**                            | **ja**   | **Namensnennung** — muss in den Credits im Spiel stehen       |
| **OFL** (Schriften)                  | ja       | Schrift darf nicht einzeln verkauft werden; Nennung empfohlen |
| **Eigenerzeugung** (KI/eigene Datei) | ja       | Werkzeug und Datum notieren                                   |
| „Free for personal use"              | **NEIN** | —                                                             |
| „Free for non-commercial use"        | **NEIN** | —                                                             |
| unklare oder fehlende Lizenz         | **NEIN** | —                                                             |

> **Die häufigste Falle: „Free for personal use".** Der Download-Knopf heißt trotzdem „Free
> Download", die Datei ist kostenlos — aber **kommerziell nicht nutzbar**. ROTA wird als
> Produkt behandelt (siehe `CLAUDE.md`), also fällt alles darunter weg.
>
> **Im Zweifel nicht verwenden.** Ein Asset ohne belegbare Lizenz ist wertlos, weil es später
> ausgetauscht werden muss — meist genau dann, wenn keine Zeit dafür ist.

**Erlaubte Quellen** (aus `docs/GAME-DESIGN.md` 17.2):

| Quelle                                                                   | Was                                           | Lizenz                                                         |
| ------------------------------------------------------------------------ | --------------------------------------------- | -------------------------------------------------------------- |
| [game-icons.net](https://game-icons.net)                                 | 4.000+ Fantasy-Symbole als SVG, umfärbbar     | CC BY 3.0                                                      |
| [Kenney.nl](https://kenney.nl)                                           | UI-Pakete                                     | CC0                                                            |
| [OpenGameArt.org](https://opengameart.org)                               | gemischt, je Asset prüfen                     | gemischt                                                       |
| itch.io                                                                  | Dark-Fantasy-RPG-UI-Pakete                    | **nur die ausdrücklich kostenlosen und kommerziell nutzbaren** |
| [ambientCG](https://ambientcg.com) / [Poly Haven](https://polyhaven.com) | Metall- und Steintexturen                     | CC0                                                            |
| [Google Fonts](https://fonts.google.com)                                 | Cinzel, Marcellus, EB Garamond, Zahlenschrift | OFL                                                            |
| [Freesound.org](https://freesound.org)                                   | ergänzende Klänge, falls Suno versagt         | je Datei prüfen                                                |

---

## Bestand

### Referenzen (`docs/refs/`) — nicht Teil der Auslieferung

| Datei                       | Herkunft                                | Lizenz         | Nennung | Zweck                                                                    |
| --------------------------- | --------------------------------------- | -------------- | ------- | ------------------------------------------------------------------------ |
| `frame-alpha-reference.png` | Eigenerzeugung, `gpt-image`, 2026-07-27 | Eigenerzeugung | nein    | **Nachweis des Transparenz-Wegs** und Stil-Referenz für Rahmen (Epic E9) |

**`frame-alpha-reference.png` — gemessene Eigenschaften:**

| Größe       | Modus | vollständig transparent | vollständig deckend | halbtransparent |
| ----------- | ----- | ----------------------- | ------------------- | --------------- |
| 1024 × 1024 | RGBA  | 84,9 %                  | 8,2 %               | **6,84 %**      |

> **Warum der Wert 6,84 % der wichtige ist:** Halbtransparente Pixel bedeuten **weiche Kanten und
> erhaltenes Glühen**. Läge er bei 0, wäre die Transparenz hart ausgestanzt und jedes Symbol hätte
> sichtbare Treppenkanten. Der Umweg über schwarzen Hintergrund mit `mix-blend-mode: screen`
> (GAME-DESIGN 17.2) ist damit **nicht nötig** — `gpt-image` liefert brauchbares Alpha direkt.

**Zwei Nachbearbeitungs-Schritte, die Epic E9 bauen muss** (an diesem Bild gemessen):

1. **Farbreste hinter der Transparenz ausbluten lassen** — sonst dunkle Ränder in manchen Zeichenpfaden
2. **Zuschneiden und auf einheitliche Größe normalisieren** — der sichtbare Inhalt dieses Bildes misst
   **742 × 822** und sitzt bei Mittelpunkt (511, 555) statt (512, 512), ist also **weder quadratisch
   noch zentriert**. Ohne diesen Schritt sitzt jedes Symbol anders im Rahmen.

---

### Stil-Kandidaten (`docs/style-candidates/`) — Entscheidungsvorlage, nicht Teil der Auslieferung

Vier Kandidaten-Sets für Issue #19, nebeneinander unter `http://localhost:8000/rota-e9-style/`.
**Nichts davon ist bereits gewählt.** Die gewählten Dateien wandern nach der Entscheidung in
`assets/img/` bzw. `assets/fonts/` und bekommen dort ihre eigene Zeile.

Maschinenlesbare Einzelliste mit Urheber und Schul-Zuordnung: `docs/style-candidates/manifest.json`.
Die Original-Lizenztexte liegen unverändert unter `docs/style-candidates/LICENSES/`.

| Gruppe                                     | n   | Herkunft                                                                                                                                      | Lizenz         | Nennung                                                 |
| ------------------------------------------ | --- | --------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------- |
| `icons/game-icons/lorc/*.svg`              | 19  | [game-icons.net](https://game-icons.net), Urheber **Lorc**                                                                                    | **CC BY 3.0**  | **ja** — „Icons made by Lorc"                           |
| `icons/game-icons/delapouite/*.svg`        | 4   | [game-icons.net](https://game-icons.net), Urheber **Delapouite**                                                                              | **CC BY 3.0**  | **ja** — „Icons made by Delapouite"                     |
| `icons/game-icons/cathelineau/*.svg`       | 1   | [game-icons.net](https://game-icons.net), Urheber **Cathelineau**                                                                             | **CC BY 3.0**  | **ja** — „Icons made by Cathelineau"                    |
| `icons/painterly/*.png`                    | 28  | [Painterly Spell Icons 1–4](https://opengameart.org/content/painterly-spell-icons-part-1), **J. W. Bjerk** (eleazzaar)                        | **CC BY 3.0**  | **ja** — „J. W. Bjerk (eleazzaar), www.jwbjerk.com/art" |
| `ui/kenney/button_square*`                 | 3   | [Kenney UI Pack](https://kenney.nl/assets/ui-pack)                                                                                            | CC0            | nein                                                    |
| `ui/kenney/fantasy-*`                      | 3   | [Kenney Fantasy UI Borders](https://kenney.nl/assets/fantasy-ui-borders)                                                                      | CC0            | nein                                                    |
| `ui/kenney/adventure-*`                    | 2   | [Kenney UI Pack Adventure](https://kenney.nl/assets/ui-pack-adventure)                                                                        | CC0            | nein                                                    |
| `ui/kenney/rpg-*`                          | 2   | [Kenney UI Pack RPG Expansion](https://kenney.nl/assets/ui-pack-rpg-expansion)                                                                | CC0            | nein                                                    |
| `textures/Metal036_Color_512.jpg`          | 1   | [ambientCG Metal036](https://ambientcg.com/a/Metal036), auf 512 px verkleinert                                                                | CC0            | nein                                                    |
| `textures/Metal009_Color_512.jpg`          | 1   | [ambientCG Metal009](https://ambientcg.com/a/Metal009), auf 512 px verkleinert                                                                | CC0            | nein                                                    |
| `fonts/Cinzel[wght].ttf`                   | 1   | [Google Fonts — Cinzel](https://fonts.google.com/specimen/Cinzel)                                                                             | OFL            | empfohlen                                               |
| `fonts/EBGaramond[wght].ttf`               | 1   | [Google Fonts — EB Garamond](https://fonts.google.com/specimen/EB+Garamond)                                                                   | OFL            | empfohlen                                               |
| `fonts/Marcellus-Regular.ttf`              | 1   | [Google Fonts — Marcellus](https://fonts.google.com/specimen/Marcellus)                                                                       | OFL            | empfohlen                                               |
| `fonts/ChakraPetch-*.ttf`                  | 2   | [Google Fonts — Chakra Petch](https://fonts.google.com/specimen/Chakra+Petch)                                                                 | OFL            | empfohlen                                               |
| `fonts/KenneyFuture.ttf`                   | 1   | [Kenney UI Pack](https://kenney.nl/assets/ui-pack), Schrift „Kenney Future"                                                                   | CC0            | nein                                                    |
| `rejected/pauliuw-cc0-kontaktbogen.png`    | 1   | Eigenerzeugung (ImageMagick `montage`) aus [Skill, item and spell icons](https://opengameart.org/content/skill-item-and-spell-icons), pauliuw | CC0            | nein                                                    |
| `LICENSES/*.txt`                           | 10  | Original-Lizenztexte der obigen Pakete                                                                                                        | (Lizenztext)   | —                                                       |
| `index.html`, `README.md`, `manifest.json` | 3   | Eigenerzeugung, 2026-07-27                                                                                                                    | Eigenerzeugung | nein                                                    |

> **Geprüft und verworfen:** „Skill, item and spell icons" von _pauliuw_ (OpenGameArt, CC0) — der
> einzige gefundene Weg zu Zaubersymbolen **ohne** Nennungspflicht. Es sind überwiegend
> Gegenstands-Symbole mit fest eingemaltem Rahmen und uneinheitlicher Machart. Der Kontaktbogen liegt
> als Beleg unter `rejected/`.
>
> **Folge:** Für 26 Zaubersymbole gibt es in den erlaubten Quellen **keinen reinen CC0-Weg**. Beide
> tragfähigen Symbol-Quellen stehen unter CC BY 3.0. Der Credits-Bereich im Spiel wird also gebraucht,
> sobald die Wahl getroffen ist — siehe unten.

### Grafik (`assets/img/`)

_(noch leer — wird gefüllt, sobald der Stil aus `docs/style-candidates/` gewählt ist)_

| Datei | Herkunft | Lizenz | Nennung | Verwendung |
| ----- | -------- | ------ | ------- | ---------- |
| —     | —        | —      | —       | —          |

### Klang (`assets/audio/`)

_(noch leer — wird in Epic E8 gefüllt)_

Erzeugung über Suno, Werkzeugkette siehe [`docs/suno-sfx.md`](docs/suno-sfx.md).
Konto: `backupslave` (Konto-Wächter in `tools/suno_api.py` erzwingt das).

| Datei | Herkunft | Lizenz | Nennung | Verwendung |
| ----- | -------- | ------ | ------- | ---------- |
| —     | —        | —      | —       | —          |

### Schriften (`assets/fonts/`)

_(noch leer — die Auswahl steht in `docs/style-candidates/fonts/` zur Entscheidung bereit)_

| Datei | Herkunft | Lizenz | Nennung | Verwendung |
| ----- | -------- | ------ | ------- | ---------- |
| —     | —        | —      | —       | —          |

---

## Credits-Bereich im Spiel

Sobald das erste **CC-BY**-Asset dazukommt, braucht das Spiel einen Credits-Bereich. Er listet je
Asset: Titel, Urheber, Lizenz, Quelle. **Das ist eine Lizenzauflage, keine Höflichkeit** — ohne
Nennung ist die Nutzung unzulässig.

Solange ausschließlich CC0 und Eigenerzeugungen verwendet werden, ist der Bereich nicht nötig.

**Aktueller Stand: kein CC-BY-Asset im ausgelieferten Bestand (`assets/`), also noch kein
Credits-Bereich erforderlich.** Unter `docs/style-candidates/` liegen CC-BY-Dateien, die aber
ausdrücklich nur zur Entscheidung dienen und nicht ausgeliefert werden.

**Vorgemerkt und unvermeidbar:** Die Recherche zu Issue #19 hat ergeben, dass es für 26
Zaubersymbole in den erlaubten Quellen **keinen reinen CC0-Weg** gibt — beide tragfähigen Quellen
(game-icons.net und Painterly Spell Icons) stehen unter CC BY 3.0. Sobald der Stil gewählt und die
erste Symboldatei nach `assets/img/` übernommen ist, **muss** der Credits-Bereich existieren. Er
listet je Urheber: „Icons made by Lorc / Delapouite / Cathelineau" beziehungsweise „J. W. Bjerk
(eleazzaar), www.jwbjerk.com/art".

---

## Offen

| Punkt                                   | Status                                                                                                                |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Konkretes Asset-Set und Stil-Festlegung | **Wartet auf den Auftraggeber.** Vier Kandidaten liegen vor: `docs/style-candidates/` (Issue #19)                     |
| Credits-Bereich im Spiel                | **Wird gebraucht**, sobald der Stil gewählt ist — es gibt keinen reinen CC0-Weg zu 26 Zaubersymbolen (Issue #19, AC5) |
| Sound-Manifest (115 Slots)              | Epic E8                                                                                                               |

### Zum Stil: es gibt bewusst kein HUD-Referenzbild

Frühere Fassungen dieses Dokuments führten „zwei fehlende HUD-Referenzbilder" als offenen Punkt.
**Das ist erledigt — nicht weil die Bilder aufgetaucht sind, sondern weil sie nicht gebraucht werden.**

Der Grund steht in `docs/GAME-DESIGN.md` 17.2: Der Stil kommt **von einem menschlichen Gestalter über
fertige freie Assets**, nicht aus einer Bildvorlage. Genau deshalb heißt Issue E9-1 „Assets
recherchieren und Stil **festlegen**" und nicht „Stil generieren". Die Entscheidung fällt an echten
Kandidaten-Sets, die der Auftraggeber nebeneinander sieht.

Ein erzeugtes HUD-Bild wurde am 2026-07-27 zur Probe angefertigt und vom Auftraggeber **abgelehnt**:
Messing, Nieten und vergilbtes Papier — Steampunk von der Stange, wörtlich „austauschbar". Es liegt
bewusst **nicht** im Repository. Eine Richtungsvorgabe, die niemand will, ist schlechter als keine;
sie würde die Auswahl in eine bereits verworfene Ecke drängen.

**Was aus dem Versuch übernommen wurde — die Anordnung ist bestätigt:** Ring in der Bildmitte,
Zauberleiste unten links im Tastatur-Raster mit sichtbarem Umschalt-Block daneben, Instrumente
rechts, Überdruck oben links, Cast-Balken darunter. Das liest sich auf einen Blick und ist in
GAME-DESIGN 9.1 festgehalten.

**Was E9 stattdessen als Ausgangspunkt hat:** `docs/refs/frame-alpha-reference.png` — kein
Stil-Vorbild, sondern der technische Nachweis, dass `gpt-image` brauchbares Alpha liefert, und die
Messgrundlage für die beiden Nachbearbeitungs-Schritte.
