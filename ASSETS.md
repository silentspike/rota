# ASSETS.md — Herkunft, Lizenz, Nennungspflicht

> **Jede Datei unter `assets/` und `docs/refs/` steht hier drin.** Ohne Eintrag darf nichts
> ausgeliefert werden.
> Pflege: Wer ein Asset hinzufügt, ergänzt im selben Commit die Zeile hier.

## Lizenzregel — nicht verhandelbar

| Lizenz | Erlaubt? | Auflage |
|---|---|---|
| **CC0 / Public Domain** | **ja** | keine |
| **CC BY** | **ja** | **Namensnennung** — muss in den Credits im Spiel stehen |
| **OFL** (Schriften) | ja | Schrift darf nicht einzeln verkauft werden; Nennung empfohlen |
| **Eigenerzeugung** (KI/eigene Datei) | ja | Werkzeug und Datum notieren |
| „Free for personal use" | **NEIN** | — |
| „Free for non-commercial use" | **NEIN** | — |
| unklare oder fehlende Lizenz | **NEIN** | — |

> **Die häufigste Falle: „Free for personal use".** Der Download-Knopf heißt trotzdem „Free
> Download", die Datei ist kostenlos — aber **kommerziell nicht nutzbar**. ROTA wird als
> Produkt behandelt (siehe `CLAUDE.md`), also fällt alles darunter weg.
>
> **Im Zweifel nicht verwenden.** Ein Asset ohne belegbare Lizenz ist wertlos, weil es später
> ausgetauscht werden muss — meist genau dann, wenn keine Zeit dafür ist.

**Erlaubte Quellen** (aus `docs/GAME-DESIGN.md` 17.2):

| Quelle | Was | Lizenz |
|---|---|---|
| [game-icons.net](https://game-icons.net) | 4.000+ Fantasy-Symbole als SVG, umfärbbar | CC BY 3.0 |
| [Kenney.nl](https://kenney.nl) | UI-Pakete | CC0 |
| [OpenGameArt.org](https://opengameart.org) | gemischt, je Asset prüfen | gemischt |
| itch.io | Dark-Fantasy-RPG-UI-Pakete | **nur die ausdrücklich kostenlosen und kommerziell nutzbaren** |
| [ambientCG](https://ambientcg.com) / [Poly Haven](https://polyhaven.com) | Metall- und Steintexturen | CC0 |
| [Google Fonts](https://fonts.google.com) | Cinzel, Marcellus, EB Garamond, Zahlenschrift | OFL |
| [Freesound.org](https://freesound.org) | ergänzende Klänge, falls Suno versagt | je Datei prüfen |

---

## Bestand

### Referenzen (`docs/refs/`) — nicht Teil der Auslieferung

| Datei | Herkunft | Lizenz | Nennung | Zweck |
|---|---|---|---|---|
| `frame-alpha-reference.png` | Eigenerzeugung, `gpt-image`, 2026-07-27 | Eigenerzeugung | nein | **Nachweis des Transparenz-Wegs** und Stil-Referenz für Rahmen (Epic E9) |

**`frame-alpha-reference.png` — gemessene Eigenschaften:**

| Größe | Modus | vollständig transparent | vollständig deckend | halbtransparent |
|---|---|---|---|---|
| 1024 × 1024 | RGBA | 84,9 % | 8,2 % | **6,84 %** |

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

### Grafik (`assets/img/`)

*(noch leer — wird in Epic E9 gefüllt)*

| Datei | Herkunft | Lizenz | Nennung | Verwendung |
|---|---|---|---|---|
| — | — | — | — | — |

### Klang (`assets/audio/`)

*(noch leer — wird in Epic E8 gefüllt)*

Erzeugung über Suno, Werkzeugkette siehe [`docs/suno-sfx.md`](docs/suno-sfx.md).
Konto: `backupslave` (Konto-Wächter in `tools/suno_api.py` erzwingt das).

| Datei | Herkunft | Lizenz | Nennung | Verwendung |
|---|---|---|---|---|
| — | — | — | — | — |

### Schriften (`assets/fonts/`)

*(noch leer — Auswahl in Epic E9)*

| Datei | Herkunft | Lizenz | Nennung | Verwendung |
|---|---|---|---|---|
| — | — | — | — | — |

---

## Credits-Bereich im Spiel

Sobald das erste **CC-BY**-Asset dazukommt, braucht das Spiel einen Credits-Bereich. Er listet je
Asset: Titel, Urheber, Lizenz, Quelle. **Das ist eine Lizenzauflage, keine Höflichkeit** — ohne
Nennung ist die Nutzung unzulässig.

Solange ausschließlich CC0 und Eigenerzeugungen verwendet werden, ist der Bereich nicht nötig.
**Aktueller Stand: kein CC-BY-Asset im Bestand, kein Credits-Bereich erforderlich.**

---

## Offen

| Punkt | Status |
|---|---|
| **Die zwei HUD-Referenzbilder** (Reaktionsgefäß, Messring, „CASCADE FAILURE"-Zauberbalken, Pergament-Ausgabefeld, Apparatur) | **Auf dem System nicht auffindbar** — siehe unten |
| Konkretes Asset-Set und Stil-Festlegung | Epic E9 |
| Sound-Manifest (115 Slots) | Epic E8 |

### Zu den fehlenden HUD-Referenzbildern

`docs/GAME-DESIGN.md` 17.2 und der Planungs-Plan beschreiben **zwei vom Auftraggeber erzeugte
Referenzbilder** in alchemistischer Fassung. Eine systemweite Suche über `/home/jan`, `/work`,
`/data` und `/tmp` (alle Bilddateien der letzten Tage, dazu Namenssuche nach `rota`, `hud`, `alch`,
`crucible`, `cascade`) hat sie **nicht gefunden**.

Wahrscheinlichste Erklärung: Sie lagen als Bild-Anhang im Gesprächsverlauf der Planungs-Session und
wurden nie als Datei gespeichert.

**Folge:** Epic E9 kann ohne sie den Stil nicht festlegen, und E1 baut das HUD-Gerüst ohne visuelle
Vorlage. Beides ist nicht blockierend — das SVG-Gerüst entsteht ohnehin zuerst und unabhängig
(GAME-DESIGN 17.2, Bau-Reihenfolge) —, aber die Bilder werden **vor dem Übergang zu Schritt 2 von
E9 gebraucht**.

**Was der Auftraggeber tun muss:** die zwei Bilder nach `/work/rota/docs/refs/` legen und hier
eintragen.
