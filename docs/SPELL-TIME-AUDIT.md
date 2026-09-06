# Zeitfeld-Abnahme für #1104

`python3 -B tools/spell-time-audit.py` zählt jedes verschachtelte `…Ms`-Feld in
`data/spells/*.json`. Neue Feldnamen brauchen keinen Eintrag in einer Liste.
Das Werkzeug liest das Raster und die ausdrückliche Schwellenausnahme aus beiden
Produktionsquellen. Widerspruch, unbekannte Ausnahmen, fehlende Daten und ungültige
Zeitwerte ergeben Exit 1. `--json` liefert die vollständige Inventur.

Am 6. September 2026 gezählt:

| Umfang                                       | Unterschiedliche Feldnamen | Vorkommen |
| -------------------------------------------- | -------------------------: | --------: |
| Auf dem 10-ms-Raster, 11 Zauberdateien       |                         12 |       180 |
| Ausdrücklich ungerasterte Vergleichsschwelle |                          1 |         1 |
| Gesamt                                       |                         13 |       181 |

Die Rasterfelder heißen `amountMs`, `castTimeMs`, `channelMs`, `channelTickMs`,
`cooldownMs`, `cooldownReductionMs`, `decayPerStepMs`, `durationMs`,
`extendDecayMs`, `extendShieldMs`, `overMs` und `tickIntervalMs`.
`cooldownReductionBelowMs` ist die einzige Ausnahme. Die Anzahl gilt für die
gelesenen Zauberdaten, nicht für alle im Code definierten Zeitgrößen oder Bossdaten.

## Ausgeführte Gegenprobe an der Produktionsprüfung

Die Inventur ersetzt keine Ausführung der Validatoren. Deshalb wurde zusätzlich
eine isolierte Kopie des Commit `e2d100a4fbd0f8de89cd7c9806626705b1684e75`
mit dem echten TypeScript-Datenparser geprüft:

```bash
vitest run tests/unit/spell-time-fields.test.ts --maxWorkers=1
```

| Zustand in der isolierten Kopie                              | Ausgeführte Tests              | Exit |
| ------------------------------------------------------------ | ------------------------------ | ---: |
| Unveränderte Daten                                           | 14 bestanden                   |    0 |
| Neues `futureEffect.settlingMs: 12001` in `coagulation.json` | 13 bestanden, 1 fehlgeschlagen |    1 |
| Bytegenau wiederhergestellte Datei                           | 14 bestanden                   |    0 |

Der tatsächlich fehlgeschlagene Fall ist
`/data/spells/coagulation.json: accepts original data and detects every off-grid duration`.
Er meldet `school.futureEffect.settlingMs: settlingMs 12001 liegt nicht auf dem 10-ms-Raster.`
Die neue Größe existiert in keiner gepflegten Feldliste. Damit ist der automatische
Schutz vor einem neu eingeführten ungerasterten Zeitfeld ausgeführt, nicht nur aus
Quelltext abgeleitet. Die Originaldatei im Arbeitszweig wurde dabei nie verändert.
Rohprotokolle und Rückbau-Hash liegen unter
`/work/tmp/rota-1104-evidence-20260906/`.

`tests/unit/spell-time-fields.test.ts` ändert außerdem jedes bestehende
Rasterfeld einzeln auf `12001`; es prüft die unveränderten Daten sowie die erlaubte
Schwellenausnahme. Auf Rust-Seite ruft `sim/build.rs` vor der Übersetzung dieselbe
Funktion aus `sim/time_fields.rs` auf, die `sim/tests/time_fields.rs` ausführt.
Alle drei Rust-Prüffälle bestanden im vollständigen Remote-Lauf. Der neue
unbekannte Feldname und beide `extend…Ms`-Felder werden dort mit `12001` abgelehnt,
mit `12000` angenommen. Das ist eine ausgeführte Validatorprüfung; ein zusätzlicher
erneuter Rust-Bau mit mutierter Datendatei wird hier nicht behauptet.

Die fünf CLI-Verhaltenstests in `tests/tools/test_spell_time_audit.py` laufen über
die bestehende Python-Discovery im `lint`-Job und damit im Sammelcheck `ci-pass`.
Sie prüfen echte Unterprozesse einschließlich Exit 1, Zählungen, neuer verschachtelter
Felder und Widersprüchen zwischen den beiden Produktionsverträgen.

## Entscheidungen an den Lesestellen

| Feld                              | TypeScript / Rust                                              | Entscheidung                                                                                                                                                                          |
| --------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `extendShieldMs`, `extendDecayMs` | `parseSpell` / `emit_putrefaction` in `sim/build.rs`           | Beide benötigen das Raster. Fehlend bedeutet ausdrücklich keine Verlängerung; Rust verwendet deshalb weiterhin 0. Vorhandene falsche Typen werden vom rekursiven Validator abgelehnt. |
| `channelMs`                       | `parseSpell` / `channel_ms` in `sim/build.rs`                  | Die Dauer muss zum Kanalintervall passen; zusätzlich gilt das gemeinsame Zeitraster.                                                                                                  |
| `channelTickMs`                   | `parseSpell` / `channel_tick` in `sim/build.rs`                | Raster erforderlich, weil nach dem ersten Kanaltermin weitere Termine durch Addition entstehen.                                                                                       |
| `cooldownReductionMs`             | `parseSpell` / `cd_reduction` in `sim/build.rs`                | Raster erforderlich, weil der Betrag direkt von einer Bereitschaftszeit abgezogen wird.                                                                                               |
| `cooldownReductionBelowMs`        | `parseSpell` / `cooldown_reduction_below_ms` in `sim/build.rs` | Sichere nichtnegative Ganzzahl, ohne Rasterpflicht: eine Auswahlbedingung, kein geplanter Termin.                                                                                     |

Die vier ergänzend verlangten Felder wurden einzeln an ihren Lesestellen geprüft.
Die damaligen Korrekturen aus PR #1106 stehen bereits im Bestand; diese Abnahme
ergänzt die automatische Inventur und den nachgewiesenen neuen Fehlerfall.
