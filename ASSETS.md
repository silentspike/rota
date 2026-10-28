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

**Stil GEÄTZT** — Entscheidung des Auftraggebers vom 2026-07-27, festgehalten in
`docs/GAME-DESIGN.md` 17.2.

| Datei                                                 | n   | Herkunft                                                                       | Lizenz         | Nennung                                    | Verwendung                                                           |
| ----------------------------------------------------- | --- | ------------------------------------------------------------------------------ | -------------- | ------------------------------------------ | -------------------------------------------------------------------- |
| `assets/img/icons/lorc/*.svg`                         | 18  | [game-icons.net](https://game-icons.net), Urheber **Lorc**                     | **CC BY 3.0**  | **ja** — „Icons made by Lorc"              | Zauber-Symbole, einfarbig, werden in der Schulfarbe eingefärbt (9.3) |
| `assets/img/icons/delapouite/*.svg`                   | 5   | [game-icons.net](https://game-icons.net), Urheber **Delapouite**               | **CC BY 3.0**  | **ja** — „Icons made by Delapouite"        | wie oben                                                             |
| `assets/img/icons/cathelineau/*.svg`                  | 1   | [game-icons.net](https://game-icons.net), Urheber **Cathelineau**              | **CC BY 3.0**  | **ja** — „Icons made by Cathelineau"       | wie oben                                                             |
| `assets/img/textures/metal036-blackened-iron-512.jpg` | 1   | [ambientCG Metal036](https://ambientcg.com/a/Metal036), auf 512 px verkleinert | CC0            | nein (Nennung trotzdem im Credits-Bereich) | Untergrund „geschwärztes Eisen" für Tafeln und Rahmen                |
| `assets/img/frames/slot-frame.png`                    | 1   | Eigenerzeugung, `gpt-image`, 2026-07-28                                        | Eigenerzeugung | nein                                       | Rahmen um die Zauber-Symbole (Slot 104 px)                           |
| `assets/img/frames/ring-mount.png`                    | 1   | Eigenerzeugung, `gpt-image`, 2026-07-28                                        | Eigenerzeugung | nein                                       | Fassung des Reaktionsgefäßes — zwei schmale Reifen, Kanal frei       |
| `assets/img/LICENSES/game-icons.txt`                  | 1   | Original-Lizenztext aus dem game-icons.net-Archiv                              | (Lizenztext)   | —                                          | Nachweis, mit der vollständigen Urheberliste                         |

**Die Symbole sind bearbeitet** (Issue #20): zugeschnitten, exakt zentriert und im optischen Gewicht
angeglichen. **Zwei wurden in Issue #21 ausgetauscht**, weil sie in Spielgröße verwechselbar waren:
`spiral-bloom` wich `test-tubes`, `caged-ball` wich `clamp`. Die Zeichnung selbst ist unangetastet — jedes SVG trägt den Originalpfad und davor nur
eine Transformation. Wiederholbar über `python3 tools/icon-normalize.py`, geprüft mit
`python3 tools/icon-verify.py`. CC BY verlangt, Bearbeitungen kenntlich zu machen; der
Credits-Bereich zeigt sie an. Die unveränderten Originale liegen als Beleg unter
`docs/style-candidates/icons/game-icons/`.

> **Die 24 Symbole sind der Startbestand, nicht die Endauswahl.** Version 1 hat **60 Zauber**
> (GAME-DESIGN 3); game-icons.net stellt 4.181 Symbole bereit, die Reserve ist also reichlich. Welches
> Symbol zu welchem Zauber gehört, entscheidet sich erst mit dem Zauber-Katalog (Epic E10). Die
> Ähnlichkeits-Prüfung aus Issue #21 läuft über den jeweils aktuellen Bestand.

### Klang (`assets/audio/`)

**Alle 56 Dateien sind Eigenerzeugung mit Suno**, erzeugt am 27. und 28. Juli 2026 über das Konto
`backupslave` im **Pro Plan**. Die Werkzeugkette liegt seit 2026-10-28 im privaten Werkzeug-Repo
`silentspike/rota-werkzeuge` unter `suno/` (Doku `suno/suno-sfx.md`, Aufruf `suno/README.md`);
`suno/suno_api.py` erzwingt das Konto über einen Wächter, der vor jeder Generierung prüft, welcher
Account angemeldet ist.

| Muster                        | n   | Herkunft                               | Lizenz / Rechtelage                 | Nennung | Verwendung                                                                                                                                     |
| ----------------------------- | --- | -------------------------------------- | ----------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `assets/audio/spell-*.ogg`    | 3   | Suno Pro, `backupslave`, 2026-07-27    | Eigenerzeugung, kommerziell nutzbar | nein    | KINDLE, SMOULDER, PYRE — die drei Zauber aus Schritt 1                                                                                         |
| `assets/audio/special-*.ogg`  | 7   | Suno Pro, `backupslave`, 2026-07-27    | Eigenerzeugung, kommerziell nutzbar | nein    | Unterbrechen, Overdrive, Rücksetzer, Schild auf/bricht, Heilung, Reflexion                                                                     |
| `assets/audio/feedback-*.ogg` | 4   | Suno Pro, `backupslave`, 2026-07-27    | Eigenerzeugung, kommerziell nutzbar | nein    | Zauber bereit, Fehleingabe, Synergie verfügbar, GCD-Ende                                                                                       |
| `assets/audio/boss-*.ogg`     | 8   | Suno Pro, `backupslave`, 2026-07-27    | Eigenerzeugung, kommerziell nutzbar | nein    | Zauberbeginn, durchgekommen, Treffer, Phasenwechsel, Überdruck, Reaktion kippt, besiegt                                                        |
| `assets/audio/state-*.ogg`    | 6   | Suno Pro, `backupslave`, 2026-07-27/28 | Eigenerzeugung, kommerziell nutzbar | nein    | Countdown, Start, wenig Leben, Tod, Zeitablauf, Wipe Run                                                                                       |
| `assets/audio/post-*.ogg`     | 6   | Suno Pro, `backupslave`, 2026-07-27    | Eigenerzeugung, kommerziell nutzbar | nein    | Sieg, Niederlage, Bestzeit, Erfolg, Stufenaufstieg, Punkte                                                                                     |
| `assets/audio/menu-*.ogg`     | 12  | Suno Pro, `backupslave`, 2026-07-28    | Eigenerzeugung, kommerziell nutzbar | nein    | Berühren, Drücken, Reiter, Fenster auf/zu, Talentpunkt, Talente zurück, Gegenstand angelegt, gespeichert, abgelehnt, Freischaltung, Aufwertung |
| `assets/audio/loot-*.ogg`     | 4   | Suno Pro, `backupslave`, 2026-07-27    | Eigenerzeugung, kommerziell nutzbar | nein    | Beute nach Qualität: Common, Rare, Epic, Legendary                                                                                             |
| `assets/audio/music-*.ogg`    | 6   | Suno Pro, `backupslave`, 2026-07-28    | Eigenerzeugung, kommerziell nutzbar | nein    | Fünf feste Klangteppich-Schichten plus ein vorläufiger Grundklang                                                                              |

**Nachbearbeitet** über `tools/sfx-post.sh`: Stille entfernt, auf die Ziel-Länge gekürzt mit weicher
Ausblendung, Pegel angeglichen, als OGG Vorbis abgelegt. Die Rohdateien und der jeweils zweite
Suno-Entwurf liegen lokal unter `assets/sfx/` und sind bewusst **nicht** versioniert (siehe
`.gitignore`). Technisch geprüft mit `python3 tools/sfx-verify.py`.

#### Die Rechtelage — belegt, nicht behauptet

Bei Suno hängt die kommerzielle Nutzbarkeit am **Kontotyp**, nicht am einzelnen Klang. Drei Belege:

| #   | Beleg                               | Fundstelle                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | **Welcher Tarif tatsächlich läuft** | `GET studio-api-prod.suno.com/api/billing/info/` meldet für `backupslave`: `plan.plan_key = "pro"`, `plan.name = "Pro Plan"`, und in `plan.features` ausdrücklich **„Commercial use rights for songs made while subscribed"**. Abgefragt am 2026-07-28, Abo-Anker `2026-07-26`                                                                                                                               |
| 2   | **Die Nutzungsbedingungen**         | [suno.com/terms](https://suno.com/terms), Abschnitt **„Intellectual Property Rights > Content"**: _„if you are a user who has subscribed to the Pro or Premier paid tier of the Service, Suno hereby assigns to you all of its right, title and interest in and to any Output owned by Suno and generated from Submissions made by you through the Service during the term of your paid-tier subscription."_ |
| 3   | **Die Hilfeseite dazu**             | [help.suno.com, Artikel 2746945](https://help.suno.com/en/articles/2746945): _„If you make songs while subscribed to the Pro or Premier plan, you own the songs. Further, you are granted a commercial use license to monetize those songs."_ Für den kostenlosen Tarif gilt das Gegenteil: _„Suno is the owner of the songs. You are allowed to use the songs for non-commerical purposes."_                |

**Was daraus folgt — und was nicht:**

| Punkt                            | Stand                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| -------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Kommerzielle Nutzung in ROTA     | **zulässig** — alle 56 Dateien wurden am 27./28. Juli 2026 erzeugt, also innerhalb der laufenden Pro-Laufzeit (Anker 2026-07-26)                                                                                                                                                                                                                                                                                                                                                 |
| Nennungspflicht                  | **keine.** Die Nennungsauflage der Bedingungen betrifft ausdrücklich nur den kostenlosen Tarif                                                                                                                                                                                                                                                                                                                                                                                   |
| Urheberrecht an den Ausgaben     | **offen, und das ist kein Versäumnis.** Die Bedingungen sagen selbst: _„due to the nature of machine learning, Suno makes no representation or warranty to you that any copyright will vest in any Output."_ Wir dürfen die Klänge nutzen und verkaufen; ob sie urheberrechtlich schutzfähig sind, ist damit **nicht** zugesichert — bei KI-Ausgaben in vielen Rechtsordnungen zweifelhaft. Praktische Folge: Dritte lassen sich nicht sicher von einer Nachnutzung ausschließen |
| Was passiert, wenn das Abo endet | Die Zuweisung gilt für Ausgaben, die **während** der Laufzeit erzeugt wurden. Bereits erzeugte Klänge bleiben also nutzbar; **neue** Klänge nach einem Wechsel in den kostenlosen Tarif wären es nicht. Wer nach einer Abo-Pause weitere Klänge erzeugt, prüft den Tarif vorher                                                                                                                                                                                                  |

> **Warum das hier steht und nicht nur im Kopf:** Ein Klang ohne belegte Rechtelage ist so wertlos
> wie eine Grafik, deren Lizenz nur den privaten Gebrauch erlaubt — nur fällt es später auf, wenn
> keine Zeit mehr ist. Der Tarif
> lässt sich jederzeit gegen die Wirklichkeit prüfen: `suno/suno_api.py auth-test` aus dem
> privaten Werkzeug-Repo `silentspike/rota-werkzeuge`, aufgerufen aus der Wurzel eines
> rota-Checkouts, zeigt Konto und Tarif-Stand an.

### Schriften (`assets/fonts/`)

**Stil GEÄTZT** — Cinzel für Überschriften, EB Garamond für Fließtext (GAME-DESIGN 17.2).
Beide als variable Schrift, also eine Datei je Familie über den gesamten Strichstärkenbereich.

| Datei                                      | Herkunft                                                                    | Lizenz       | Nennung                         | Verwendung                              |
| ------------------------------------------ | --------------------------------------------------------------------------- | ------------ | ------------------------------- | --------------------------------------- |
| `assets/fonts/Cinzel-Variable.ttf`         | [Google Fonts — Cinzel](https://fonts.google.com/specimen/Cinzel), 400–900  | OFL 1.1      | empfohlen — „Natanael Gama"     | Überschriften, Beschriftungen, Keybinds |
| `assets/fonts/EBGaramond-Variable.ttf`     | [Google Fonts — EB Garamond](https://fonts.google.com/specimen/EB+Garamond) | OFL 1.1      | empfohlen — „Duffner und Pardo" | Fließtext, Zahlen in den Instrumenten   |
| `assets/fonts/LICENSES/Cinzel-OFL.txt`     | Original-Lizenztext                                                         | (Lizenztext) | —                               | Nachweis                                |
| `assets/fonts/LICENSES/EBGaramond-OFL.txt` | Original-Lizenztext                                                         | (Lizenztext) | —                               | Nachweis                                |

---

## Credits-Bereich im Spiel

Sobald das erste **CC-BY**-Asset dazukommt, braucht das Spiel einen Credits-Bereich. Er listet je
Asset: Titel, Urheber, Lizenz, Quelle. **Das ist eine Lizenzauflage, keine Höflichkeit** — ohne
Nennung ist die Nutzung unzulässig.

**Aktueller Stand: der Bereich existiert und ist im laufenden Spiel erreichbar.** Mit der
Stil-Entscheidung GEÄTZT sind 24 CC-BY-Symbole nach `assets/img/` gewandert; damit ist die Nennung
Pflicht geworden.

| Was        | Wo                                                                                                                                                |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Inhalte    | `data/credits.json` — die eine Wahrheit für das, was angezeigt wird                                                                               |
| Anzeige    | `src/credits/credits.ts` und `src/credits/credits.css`, HTML über der Zeichenfläche                                                               |
| Erreichbar | Knopf **CREDITS** unten rechts; `Escape` schließt wieder                                                                                          |
| Prüfung    | `python3 tools/assets-check.py` — meldet jede Datei unter `assets/img/` oder `assets/fonts/` ohne Nennung, und jeden Eintrag, der ins Leere zeigt |
| Unit-Tests | `tests/unit/credits.test.ts` — prüft den geforderten Wortlaut je Urheber                                                                          |

Genannt werden derzeit: **„Icons made by Lorc"**, **„Icons made by Delapouite"**,
**„Icons made by Cathelineau"** (alle CC BY 3.0, Pflicht) sowie ambientCG, Natanael Gama und
Duffner/Pardo (CC0 beziehungsweise OFL, freiwillig).

> **Der Ort des Knopfes ist vorläufig.** game-icons.net verlangt lediglich, dass die Nennung „from a
> menu" erreichbar ist. Sobald es ein Hauptmenü gibt, gehört der Eintrag dorthin und der Knopf am
> Bildrand verschwindet. Bis dahin muss er bleiben — ohne ihn wäre jede Auslieferung unzulässig.

---

## Offen

| Punkt                                   | Status                                                                                                                                                                                                                                                                                                |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Konkretes Asset-Set und Stil-Festlegung | **Erledigt** — GEÄTZT, Entscheidung vom 2026-07-27, festgehalten in `docs/GAME-DESIGN.md` 17.2                                                                                                                                                                                                        |
| Credits-Bereich im Spiel                | **Erledigt** — `data/credits.json`, Knopf unten rechts, mechanisch geprüft                                                                                                                                                                                                                            |
| Symbol-Zuordnung zu den 60 Zaubern      | Epic E10 (Zauber-Katalog). Der Bestand von 24 Symbolen ist ein Startbestand. **Beim Ausbau auf 60 wächst die Ähnlichkeits-Prüfung von 276 auf 1.770 Paare** — `tools/icon-distinct.py` ist dafür gebaut, in dieser Größenordnung aber ungetestet. Vor dem Ausbau einen Lauf mit Zeitmessung einplanen |
| Rahmen, Panels, Ringtextur              | Issue #22 — **tragender Teil des Aussehens**, nicht Zutat, weil GEÄTZT für sich flach ist                                                                                                                                                                                                             |
| `assets/sfx/` ohne Einträge             | **Erledigt** — der Ordner ist seit Issue #18 in der `.gitignore` (248 Dateien, 21 MB). Dort liegen Suno-Rohdateien und der jeweils zweite Entwurf; lokal bleiben sie, damit sich ohne neue Credits umstellen lässt                                                                                    |
| Klang-Bestand in diesem Dokument        | **Erledigt** — Issue #43, alle 56 Dateien geführt, Rechtelage mit drei Fundstellen belegt                                                                                                                                                                                                             |
| Sound-Manifest                          | **Erledigt** — `data/sound-manifest.json`, 56 beschriebene Klänge plus Platzhalter für die 71 aus E10                                                                                                                                                                                                 |

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
