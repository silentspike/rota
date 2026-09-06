# Rundungsregel für Browser und Simulator (#962)

Stand: 2026-09-06. Ausgangspunkt ist `0fd41668`.

TypeScript und Rust runden zur nächsten ganzen Zahl. Bei gleichem Abstand gewinnt
die Zahl Richtung positiver Unendlichkeit: **−1,5 wird −1**, +1,5 wird +2.
Das entspricht der bestehenden Browser-Referenz
([ECMAScript, Math.round](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.round)).
Rusts unverändertes
[`f64::round`](https://doc.rust-lang.org/std/primitive.f64.html#method.round)
liefert bei −1,5 dagegen −2. Diese Abweichung wird in `factors::round_js` korrigiert.
`roundLikeJavaScript` auf der Browser-Seite delegiert an `Math.round`.
Beide zentralen Schadensformeln verwenden den jeweiligen Helfer.

Die Eingabe ist ein bereits berechneter IEEE-754-binary64-Wert. Gerundet wird auf
ganze Einheiten derselben Größe, ohne zusätzliche Dezimalskalierung. Negative Null
bleibt erhalten: −0,5 und der kleinste negative Subnormalwert ergeben −0.
Unendlichkeiten bleiben unverändert, NaN bleibt NaN; NaN-Nutzlasten werden nicht
verglichen. Ab Betrag 2^52 sind alle darstellbaren Werte bereits ganzzahlig.

Der Rust-Helfer bildet zunächst den ganzzahligen Anteil mit `trunc`, danach den
Rest. Nur ein Rest ≥ +0,5 erhöht den Anteil; nur ein Rest < −0,5 vermindert ihn.
Der Rest ist exakt darstellbar: Er besteht aus den verbleibenden Nachkommabits des
Eingabewerts. `floor(value + 0.5)` wäre kein Ersatz: Schon die Addition kann den
unmittelbaren Nachbarn unter +0,5 auf 1 aufrunden; außerdem ginge negative Null
verloren.

## Gemeinsame Vektoren und echte Gegenprobe

Beide Tests lesen denselben Bestand:
[`sim/tests/fixtures/rounding-vectors.json`](../sim/tests/fixtures/rounding-vectors.json).
Er enthält **71 Vektoren**, darunter 33 strikt negative endliche Eingaben,
beide Nullzeichen, Subnormalwerte, ±Unendlich und NaN. Eingaben und Sollwerte
stehen als lesbare Zahlen und exakte 64-Bit-Hexmuster nebeneinander. Die Tests
dekodieren die Bitmuster; JSON-Dezimalrundung kann keinen Nachbarn verschieben.

Sieben positive und sieben negative Halbschwellen enthalten jeweils die exakte
Schwelle und beide unmittelbar benachbarten binary64-Werte. Weitere Fälle sichern
ganze Zahlen bei 2^52 und der JavaScript-Grenze 2^53. Der Sollwert wurde unabhängig
von beiden Implementierungen aus dem exakten rationalen Eingabewert n/d gebildet:
`floor((2*n + d) / (2*d))`, mit separatem Vorzeichen für ein Nullergebnis.

| Eingabe              | TypeScript (PASS) | Rust (PASS)      | Besonderheit                               |
| -------------------- | ----------------- | ---------------- | ------------------------------------------ |
| −1,5000000000000002  | −2                | −2               | unmittelbarer Nachbar unter −1,5           |
| **−1,5**             | **−1**            | **−1**           | ausdrücklicher Abnahmefall AC2             |
| −1,4999999999999998  | −1                | −1               | unmittelbarer Nachbar über −1,5            |
| −0,5                 | −0                | −0               | Vorzeichen der Null bleibt                 |
| +0,49999999999999994 | +0                | +0               | `floor(x + 0.5)` wäre hier falsch          |
| +0,5                 | +1                | +1               | exakte positive Halbschwelle               |
| +0,5000000000000001  | +1                | +1               | unmittelbarer Nachbar über +0,5            |
| 4503599627370497     | 4503599627370497  | 4503599627370497 | bereits ganzzahlig; keine Addition von 0,5 |

Der vollständige Bestand nennt alle weiteren Ein- und Ausgaben; jeder Fall wird
auf beiden Seiten gegen diese Sollbits geprüft. TypeScript zählt jeden Vektor als
eigenen Test. Rust prüft alle 71 Vektoren in einem Test und ergänzt vier weitere
Vertrags-/Integrationsfälle im Debug-Profil.

Die echte TypeScript-Gegenprobe ersetzte ausschließlich den Helferrumpf durch
`value < 0 ? -Math.round(-value) : Math.round(value)`. Damit wurden die negativen
Halbschwellen absichtlich wie in Rusts ursprünglichem `round` behandelt.
Ergebnis: **9 fehlgeschlagene und 65 bestandene Tests, Exit 1**. Nach bytegenauem
Rückbau: **74 bestandene Tests, Exit 0**. Der Fall −1,5 ist unter den Fehlern.

## Negative Zwischenwerte und Grenzen des Schadensdatentyps

Negative Zwischenwerte sind im vorhandenen Simulator **erreichbar**. Die
öffentlichen `DamageFactors` nehmen beliebige `f64` entgegen. Die echte zentrale
Formel rechnet beispielsweise bei Grundschaden 1, ESSENCE-Faktor −1,5 und
Talent-Faktor −1 zunächst −1,5, anschließend +1,5 und schließlich Schaden 2.
Die vorhandene Zusicherung prüft nur das Endprodukt und sieht den negativen
Zwischenwert nicht. Je ein Rust- und TypeScript-Test ruft diese Formel direkt auf.

Eine zweite Rust-Probe setzt nur den negativen ESSENCE-Faktor in `FightConfig`
und führt `Fight::run(rotation::good)` aus. Sie muss im Debug-Profil an der
bestehenden Zusicherung `negativer Schaden` scheitern. Das prüft den wirklichen
Kampfpfad und wird im Release-Profil ausdrücklich nicht als Test gezählt.

Für den regulären Datenpfad sind Grundschaden, Ausrüstungs-Promille, Talentwerte
und Stapelzahlen unsigned; Ausrüstungsfaktoren entstehen aus `1 + promille/1000`.
`build.rs` prüft Verstärkerfaktoren ≥ 1 und Boss-Widerstände > 0.
`Buffs::factor` multipliziert diese Werte; Synergie und Verwundbarkeit entstehen
aus positiven Prozentgrundwerten. Diese Quellen erklären die bisherige
Nichtnegativ-Annahme, beweisen sie aber nicht für beliebige öffentliche
Konfigurationen oder zukünftige Effekte. Negative Differenzen treten außerdem
in der Balance-Auswertung auf (`balance::scaling`, gepaarte Schadensdifferenzen);
sie durchlaufen die Schadensrundung nicht.

Der Rundungsvertrag ändert **keine** Wertebereiche: `compute_damage` gibt weiterhin
`u32` zurück, behält seine Debug-Zusicherung und die bestehende Sättigung beim
Cast. Negative Schadensmengen werden dadurch nicht zu einer neuen Spielmechanik.
Für alle nichtnegativen Vektoren im `u32`-Bereich wird zusätzlich die echte
Schadensfunktion gegen Sollwert und bisheriges `round` geprüft.

Die Multiplikationsreihenfolge, Rundungszeitpunkt, Daten und gespeicherten
Kampfaufzeichnungen bleiben unverändert. Andere bewusste Ganzzahldivisionen,
Abrundungen, Zeitraster und weitere `round`-Aufrufe in Kampf-/UI-Hilfsfunktionen
werden nicht umgestellt. Festkommaskalierung (#960), Überlaufbeweis (#961),
JavaScript-Grenze (#963) und SIMD/GPU-Parität (#964) werden hier nicht abgenommen.
Insbesondere ersetzt diese Regel keine noch ausstehende Skalierung am Typ.

## Ausführung und Abnahmezustand

| Prüfung                                                            | Status         | Nachweis                                                   |
| ------------------------------------------------------------------ | -------------- | ---------------------------------------------------------- |
| TypeScript: Rundungsvertrag + bestehende Formelreihenfolge/-stärke | PASS           | 92 Tests in 3 Dateien                                      |
| TypeScript: absichtlich abweichende negative Rundung               | FAIL, erwartet | 9 rot / 65 grün, Exit 1                                    |
| TypeScript: wiederhergestellter Vertrag                            | PASS           | 74 Tests, Exit 0                                           |
| Rust: Vertrag, echter Negativpfad, bestehende Faktoren/Deckel      | PASS           | 29 Tests in 4 Targets, darunter alle 71 Vektoren; Exit 0   |
| CI-Filter erfasst den gemeinsamen Vektorbestand                    | PASS           | 77 TS-Tests: Rundungsvertrag plus drei Filterprüfungen     |
| Rustfmt und gezieltes Clippy mit `-D warnings`                     | PASS           | jeweils Remote-Exit 0, keine Warnungen                     |
| Gemeinsamer vollständiger Prüflauf und alle Kampfaufzeichnungen    | BLOCKED        | Root koordiniert einmaligen Gesamtlauf und CI vor Abnahme  |
| Auf `main` gelandet und zurückgelesen                              | BLOCKED        | Integration und Abschluss erfolgen durch Root nach Prüfung |

Lokale Rohbelege: `/work/tmp/rota-962-evidence-20260906/`, insbesondere
`ts-baseline.log`, `ts-negative-mutation.log`, `ts-restored.log` und
`ts-mutation-summary.json` sowie `rust-targeted.log` und
`rust-targeted-summary.json`, `rustfmt.log` und `rust-clippy.log`.
Nicht ausgeführte Prüfungen sind kein PASS.

Gezielte Wiederholung aus der Workspace-Wurzel:

```bash
npx --no-install vitest run tests/unit/rounding-contract.test.ts tests/unit/damage-order.test.ts tests/unit/damage-strength.test.ts
cargo remote --no-copy-lock -- test -p rota-sim --test rounding_contract --test factors --test caps_tabelle --test parity_deckel --no-fail-fast
cargo remote --no-copy-lock -- clippy -p rota-sim --lib --test rounding_contract --test factors --test caps_tabelle --test parity_deckel -- -D warnings
```
