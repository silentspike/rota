# APL-Kontrolle: Abnahme von #797 am 2026-09-06

Ausgangsstand `e2d100a4fbd0f8de89cd7c9806626705b1684e75`, isolierte Kopie unter
`/work/tmp/rota-797-acceptance-20260906`. Ausschließlich Remote-Cargo,
keine parallelen Aufrufe, kein Copy-back und keine Änderung am Hauptarbeitsbaum.

| Lauf                                                       | Tests                           | Suites | Exit |
| ---------------------------------------------------------- | ------------------------------- | ------ | ---- |
| Bestand                                                    | 422 bestanden                   | 98     | 0    |
| `self.pin_window = control.amount` → `self.pin_window = 0` | 420 bestanden, 2 fehlgeschlagen | 98     | 101  |
| `Value::BossCasting` liest immer `Number::from(0_u8)`      | 2 bestanden, 3 fehlgeschlagen   | 1      | 101  |
| Bytegenau wiederhergestellter Bestand                      | 422 bestanden                   | 98     | 0    |

Alle vier Läufe haben 0 ignorierte und 0 weggefilterte Tests. Cargo.lock blieb bei
jedem Lauf bytegleich; beide mutierten Quelltexte wurden ebenfalls bytegleich
wiederhergestellt.

AC4a: Die 98 Targets umfassen Gegenstücke sämtlicher **91 heutiger**
Aufzeichnungsdateien (historischer Issue-Stand: 86). Unter der PIN-Mutation bricht
nur `parity_pin`: `beide_seiten_rechnen_dasselbe` und
`die_pruefsumme_stimmt_in_jedem_takt`. Beide prüfen `pin-befreiung.json`.
Damit sind gefahren / gebrochen: **91 / 1**, namentlich `pin-befreiung.json`.
Alle anderen Aufzeichnungs-Gegenstücke bleiben grün. Die vollständige
Datei-zu-ausgeführtem-Target-Zuordnung steht im [versionierten Messnachweis](evidence/797-2026-09-06.json).

AC4b: Bei ausgeschaltetem APL-Sprachwert brechen in `apl_boss_cast`:

- `ac1_boss_casting_schraenkt_auf_das_cast_fenster_ein`
- `ac1_beide_werte_treffen_dasselbe_fenster_wie_die_zeitangabe`
- `ac1_die_engere_schwelle_haengt_am_weckruf_und_trifft_dasselbe_fenster`

Die Trägerprüfung und der Nullzustand der Restzeit bleiben grün. Das ist die
Rust-Sprachwertprüfung, keine Aussage über eine zweite APL-Implementierung.

Reproduzierbare vollständige Aufzeichnungsprüfung:

```bash
cargo remote --no-copy-lock -- test -p rota-sim --test 'parity*' \
  --test buffs --test cooldown_reset --test gear --test log_kinds \
  --test rules_event --test buff_combat --test apl_boss_cast --no-fail-fast
```

Für die isolierte zweite Mutation:

```bash
cargo remote --no-copy-lock -- test -p rota-sim --test apl_boss_cast --no-fail-fast
```

AC1/AC2 sind im Parser, Wertleser und Kandidatenerzeuger vorhanden und bereits
seit main abgenommen. AC3 wird durch die ausgeführten gemeinsamen Aufzeichnungen
geprüft. AC5 ist inzwischen aktiv: die drei nicht ignorierten Tests in
`sim/tests/ac5_kontrolle_lohnt_sich.rs` prüfen erlebten tödlichen Schlag,
FIXATION-Wahl mit tatsächlicher Unterbrechung samt Tod nach Entfernung nur dieser
Zeilen, sowie den harmlosen Zwilling ohne FIXATION. Diese drei bestanden im
vollständigen Remote-Lauf des Integrationsstands. Die Recherche findet keinen
weiteren offenen fachlichen AC; Abschluss setzt Integration und CI auf main voraus.

Rohlogs: `baseline.log`, `pin-window-zero.log`, `boss-casting-zero.log`,
`restored.log`; exakte Commands, Exit-Codes, Namen und SHA-256 je Log:
[versionierter Messnachweis](evidence/797-2026-09-06.json). Die vollständigen Rohlogs liegen unter
`/work/rota-recovery-20260906/completion/797/`.
