# Ein Kanal braucht einen positiven Tick-Abstand

Bei der Vorbereitung des vollständigen Mutationsdurchgangs zu #1055 fiel ein
weiterer Fehler auf: Wird `channelTickMs` aus DECOCTION entfernt oder auf `0`
gesetzt, meldete `parseSchool` zwar ein Problem, gab den ungültigen Zauber aber
weiter in `value.spells` zurück. Daraus konnten App und Tests einen `Combat`
erzeugen. Dessen Kanalschleife verschob den nächsten Termin um null und konnte
dadurch endlos laufen.

`parseSpell` meldet jetzt ausdrücklich, dass eine Kanalisierung
`channelTickMs > 0` benötigt, und nimmt diesen ungültigen Zauber nicht in die
geladene Schule auf. Die Rust-Validierung in `sim/time_fields.rs` weist denselben
fehlenden oder nullwertigen Abstand zurück. `build.rs` ruft diesen Validator
bereits vor der Erzeugung der Zaubertabelle für sämtliche Schul- und
Prüfzauberdateien auf. Dass null auf dem 10-ms-Raster liegt, reicht für einen
wiederkehrenden Kanal-Termin nicht aus.

Die Spielwerte bleiben unverändert. Nichtkanalisierende Zauber dürfen beide
Felder weiterhin weglassen; der echte DECOCTION-Kanal mit positivem Abstand läuft
weiter. Der Simulator schützte seine Termin-Schleife bereits mit `schritt > 0`,
hätte einen fehlenden Abstand jedoch als Kanal ohne Teilwirkungen übernommen.

Die neue TypeScript-Regression lädt den tatsächlichen DECOCTION-Datensatz mit
fehlendem beziehungsweise nullwertigem Tick-Abstand. Beide Fälle waren vor der
Korrektur rot, die beiden Gegenproben grün. Nach der Korrektur bestanden alle
**72 Fälle** aus `channeltick-validation.test.ts`, `channel.test.ts` und
`data.test.ts`; `tsc --noEmit` endete mit Exit 0. Die Regression prüft außerdem,
dass der abgelehnte Zauber nicht gewirkt werden kann und ein passiver Kampfschritt
zurückkehrt. Die positive Gegenprobe misst tatsächlich ausgeteilten Kanalschaden.

Zwei neue Rust-Integrationstests prüfen den von `build.rs` verwendeten Validator:
fehlender/nullwertiger Abstand wird abgelehnt; gültiger Kanal und Zauber ohne
Kanal bleiben zulässig. **Die Remote-Ausführung dieser Rust-Tests und der
vollständige Repository-Prüflauf stehen noch aus.** Diese gezielte Regression
ersetzt auch nicht den vollständigen Durchgang über alle 60 Mutationsfelder.

Die lokalen Vorher-/Nachher-Protokolle liegen unter
`/work/tmp/rota-1055-20260906-full/channeltick-before.log`,
`channeltick-after.log` und `channeltick-typecheck.log`.
