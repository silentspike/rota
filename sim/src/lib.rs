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

pub mod apl;
pub mod checksum;
pub mod data;
pub mod event;
pub mod fight;
pub mod log;
pub mod replay;
pub mod rng;
pub mod rotation;
// Issue E3-1b: das Empfaenger-Ereignis-System. Diese eine Zeile ist die
// einzige Beruehrung mit einer bestehenden Datei in `sim/src/`.
pub mod rules_event;
pub mod time;

pub use apl::{Apl, AplError};
pub use data::{Boss, Resource, Spell, BOSS, RESOURCE, SPELLS, SPELL_COUNT};
pub use fight::{Decide, Fight, FightConfig, RunResult};
pub use replay::{Binding, Press, Recording, ReplayResult};
pub use rng::{Stream, Streams};
pub use time::{Millis, GCD_MS, TICK_MS};
