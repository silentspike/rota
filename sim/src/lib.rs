//! # ROTA-Simulator
//!
//! Dieselben Kampfregeln wie das Spiel — ereignisgesteuert, deterministisch und
//! tausendfach schneller.
//!
//! ## Wofuer er da ist, und wofuer nicht
//!
//! **Er laeuft bei der Entwicklung, nie beim Spieler** (GAME-DESIGN 18.5). Das
//! Spiel braucht ueberhaupt keine Simulation — es rechnet nur aus, was
//! tatsaechlich gewirkt wurde. Dieser Simulator ist ein Werkzeug fuer die
//! Balance: Schadensverhaeltnisse, Zauberstaerke, Talente, Ausruestung.
//!
//! Vorbild ist SimulationCraft. Uebernommen sind vier Konzepte: ereignisgesteuert
//! statt fester Takt, die Aktions-Prioritaetenliste, Monte-Carlo ueber viele
//! Durchgaenge und Skalierungsfaktoren je Wert.
//!
//! ## Die dritte Umsetzung derselben Regeln
//!
//! Es gibt drei: die Python-Referenz `tools/step1-check.py`, das Spiel in
//! TypeScript, und diesen Simulator. Alle drei muessen auf dieselben Zahlen
//! kommen — dafuer gibt es die gemeinsamen Pruefaelle aus GAME-DESIGN 18.4.
//!
//! > Weicht eine ab, ist das ein starker Hinweis — auf einen Fehler hier
//! > **oder** in einer der anderen beiden. Dann wird die Abweichung gemeldet,
//! > nicht weggerechnet.
//!
//! ## Beispiel
//!
//! ```
//! use rota_sim::{rotation, Fight, FightConfig};
//!
//! let mut fight = Fight::new(FightConfig::default());
//! let result = fight.run(rotation::good);
//!
//! assert_eq!(result.damage, 4240);
//! assert_eq!(result.killed_at_ms, Some(40_510));
//! // In Schritt 1 ist PURITY = 0, also gibt es keinen Krit — und damit auch
//! // keine einzige Ziehung aus den Zufallsstroemen.
//! assert_eq!(result.random_draws, 0);
//! ```
//!
//! ## Die sieben Regeln, an denen der Determinismus haengt
//!
//! Aus GAME-DESIGN 18.4. Wo sie in diesem Kasten umgesetzt sind:
//!
//! | Regel | Wo |
//! | --- | --- |
//! | Ganzzahlige Zeit in Millisekunden | [`time`] — es gibt keinen Fliesskomma-Zeitwert |
//! | Getrennte Zufallsstroeme je Quelle | [`rng`] |
//! | Feste Ereignis-Reihenfolge, keine Hash-Container | [`event`], [`data`] |
//! | Parallel zwischen Durchgaengen, nie innerhalb | ein [`Fight`] ist in sich sequentiell |
//! | Ergebnisse sortiert zusammenzaehlen | E2-4, der Balance-Laeufer |
//! | Keine Speicheranforderung im Schritt | [`Fight::new`] fordert alles vorab an |
//! | Daten einkompiliert | `build.rs` erzeugt [`data`] aus `data/*.json` |

#![warn(missing_docs)]

// Issue #967: Durchgaenge ergaenzbar speichern statt fertig.
pub mod akkumulator;
// Issue #908: der kritische Treffer analytisch — NEBEN dem Kampf, nicht darin.
// Issue #920: angeschlossen an Durchgangswahl und Rotationssuche, je Aufbau × Boss.
pub mod analytisch;
pub mod apl;
// Issue #915: Austauschbarkeit von Treffern — abgeleitet, nicht angenommen.
pub mod austausch;
pub mod balance;
// Issue #118: die Suche nach dem besten Aufbau.
pub mod best_build;
// Issue E3-5: Verstaerkungen und Synergie-Fenster.
pub mod buffs;
// Issue #959: die Werte-Deckel je Wirkgroesse, als Tabelle.
pub mod caps;
pub mod checksum;
// Issue E3-3c: der Cooldown-Ruecksetzer als Auswahlmodus.
pub mod cooldown_reset;
pub mod data;
// Issue #896: Zaehler fuer den Diagnosepfad — Schlangenlaenge, `expire_buffs`.
pub mod diagnostics;
// Issue #940: Parameterdomaenen — der Bereich, in dem ein Optimierer suchen
// darf, samt Monotonie-Zertifikat MIT Geltungsbereich.
pub mod domaene;
// Issue #908: wie viele Durchgaenge genuegen — abgeleitet, nicht `purity == 0`.
pub mod durchgaenge;
// Issue #911: die typisierte Wirkungsbeschreibung und ihre erzeugten Analysen.
pub mod effects;
// Issue #934: die Engpasserkennung — eine Diagnose, keine Zahlenliste.
pub mod engpass;
pub mod event;
pub mod factors;
pub mod fight;
pub mod fortschritt;
pub mod gear;
// Issue #118: die Balance-Messung der Ausruestung.
pub mod gear_balance;
// Issue #177: Tempo wirkt auf Zeiten.
pub mod haste;
// Issue #912: was ein Rechner meldet — und was er wirklich leistet.
pub mod hostcaps;
// Issue #932: kooperative Laufzeitsteuerung zwischen vollständigen Kämpfen.
pub mod drosselung;
// Issue #912: was dieser Rechner je Arbeitsart leistet, gemessen.
pub mod kalibrierung;
/// Arbeitsbudget einer Kampagne, hardwareunabhaengig (#914).
pub mod kampagne;
pub mod log;
pub mod machbarkeit;
// Issue #939: der Zielvertrag — harte Bedingungen vor Rangfolge.
pub mod objective;
pub mod replay;
pub mod report;
pub mod rng;
pub mod rotation;
// Issue #981: Wertungsmenge und Instanzgruppen — welche Bosse zaehlen.
pub mod scoring;
// Issue #237: die Suche nach der besten Rotation.
pub mod rotation_search;
// Issue E3-1b: das Empfaenger-Ereignis-System. Diese eine Zeile ist die
// einzige Beruehrung mit einer bestehenden Datei in `sim/src/`.
pub mod rules_event;
pub mod sets;
// Issue #178: die fuenf Quellen der Sonderwerte.
pub mod sources;
// Issue #913: Ergebnis und Bewertung getrennt speichern.
pub mod speicher;
// Issue E3-2: die Stapel-Mechanik der Schulen, als Ereignis-Empfaenger.
pub mod stacks;
// Issue #915: gestaffelte Stichprobe ohne feste Obergrenze, jederzeit gueltig.
pub mod stichprobe;
// Issue E3-3: die Verkuerzung der Abklingzeiten (SUBLIMATION).
pub mod sublimation;
pub mod time;
// Issue #915: exakte Krit-Verteilung und zaehlerbasierter Zufall — NEBEN dem
// Kampf, weil drei Paritaets-Aufzeichnungen wirklich wuerfeln.
pub mod verteilung;

pub use apl::{Apl, AplError};
pub use balance::{BalanceConfig, BalanceResult, Sample, Statistics};
pub use data::{Boss, Resource, Spell, BOSS, RESOURCE, SPELLS, SPELL_COUNT};
pub use factors::{compute_damage, DamageFactors, NEUTRAL};
pub use fight::{Decide, Fight, FightConfig, RunResult};
pub use haste::{hasted_ms, tick_count};
pub use replay::{Binding, Press, Recording, ReplayResult};
pub use report::{
    achievement_run, feasibility, is_shippable, meaningless, spell_table, time_tiers,
};
pub use rng::{Stream, Streams};
pub use time::{Millis, GCD_MS, TICK_MS};
