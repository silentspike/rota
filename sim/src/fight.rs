//! Der Kampf — ereignisgesteuert.
//!
//! # Warum ereignisgesteuert und nicht im Takt
//!
//! Das Spiel rechnet in einem festen Logik-Takt von 10 ms (GAME-DESIGN 18.2).
//! Bei 60 Sekunden Kampf sind das 6.000 Schritte, von denen rund 5.950 nichts
//! tun. Fuer das Spiel ist das richtig: Der Takt macht das Verhalten
//! geraeteunabhaengig. Fuer den Simulator waere es Verschwendung — er soll
//! hunderttausende Durchgaenge rechnen (GAME-DESIGN 18.5).
//!
//! Dieser Simulator springt deshalb von Ereignis zu Ereignis. Ein Durchgang
//! ueber 60 Sekunden kostet damit rund 90 statt 6.000 Schritte.
//!
//! # Wie er trotzdem dasselbe rechnet
//!
//! Ein anderes Rechenverfahren darf **kein anderes Ergebnis** liefern
//! (GAME-DESIGN 18.4, gemeinsame Pruefaelle). Drei Stellen entscheiden darueber:
//!
//! **1. Die Reihenfolge innerhalb einer Millisekunde.** Die Referenz arbeitet je
//! Takt in drei Phasen: erst die passiven Wirkungen (`step_passive`: DoT-Tick,
//! dann Verfall), dann die Pruefung, ob der Boss gefallen ist, dann die
//! Entscheidung des Spielers. [`Fight::run`] arbeitet genauso — nur eben je
//! Ereigniszeitpunkt statt je Takt.
//!
//! **2. Das Takt-Raster.** Das Spiel kann nur zu Takt-Grenzen handeln. Alle
//! Zeitpunkte, die aus einer Abklingzeit entstehen, laufen deshalb durch
//! [`snap_up`]. In Schritt 1 aendert das nichts (alles ist ein Vielfaches von
//! 500 ms); ab Schritt 3 mit Tempo-Faktoren ist es der Unterschied zwischen
//! Gleichlauf und tagelanger Suche.
//!
//! **3. Wann der Sieg bemerkt wird.** Beide vorhandenen Umsetzungen — die
//! Referenz `tools/step1-check.py` und das Spiel `src/devtools/reference-run.ts` —
//! pruefen den Boss-Tod **vor** der Wirk-Phase des Takts. Ein toedlicher Zauber
//! bei 40.500 ms wird deshalb erst im Takt 40.510 ms bemerkt, und genau diese
//! Zahl steht als Sollwert in `STEP1-SPEC` § 4. Ein DoT-Tick dagegen faellt in
//! die passive Phase und wird **im selben Takt** bemerkt. Beides bildet
//! [`Fight::cast`] ab, indem es nach einem toedlichen Zauber einen Weckruf auf
//! den naechsten Takt setzt.
//!
//! Das ist keine Nachbesserung zugunsten einer Wunschzahl, sondern die Regel
//! selbst: Der Sieg gilt in dem Logik-Takt als festgestellt, in dem das Spiel
//! ihn feststellt.
//!
//! # Wo das Verfahren an seine Grenze kommt
//!
//! Die Rotation wird **nur an Ereigniszeitpunkten** befragt. Fuer eine Rotation,
//! die den Zustand abfragt — Abklingzeit bereit? Ressource ueber der Schwelle?
//! Restlaufzeit unter einem GCD? — ist das vollstaendig, denn der Zustand
//! aendert sich ausschliesslich an Ereignissen. Alle vier Rotationen aus
//! `STEP1-SPEC` § 4 sind so gebaut.
//!
//! Fuer eine Rotation, die auf die **Uhr** schaut, waere es unvollstaendig:
//! Faellt der Zeitpunkt, zu dem ihre Bedingung wahr wird, zwischen zwei
//! Ereignisse, bemerkt der Simulator ihn nicht — die Referenz mit ihrem
//! 10-ms-Takt sehr wohl.
//!
//! **Seit E2-3 ist das geloest**, aber nicht hier: [`Decide::next_wakeup`] laesst
//! die Entscheidung selbst sagen, wann sie erneut geprueft werden will, und
//! [`Fight::schedule_decision_wakeup`] plant den Weckruf ein. Eine
//! Prioritaetenliste ([`crate::apl`]) kennt ihre Bedingungen und rechnet den
//! Umschlagpunkt aus; eine Rotation als blosse Funktion kann das nicht und
//! bekommt deshalb weiterhin keine Weckrufe — fuer die vier zustandsabhaengigen
//! Pruefrotationen ist das genau richtig.
//!
//! `tests/edge_cases.rs` haelt beide Seiten fest: dass eine Funktion die Grenze
//! hat, und `tests/apl.rs`, dass eine Prioritaetenliste sie nicht mehr hat.

use crate::checksum::{mix_float, mix_int, FNV_OFFSET};
use crate::data::{Boss, Resource, Spell, BOSS, RESOURCE, SPELLS, SPELL_COUNT};
use crate::event::{EventKind, EventQueue};
use crate::factors::{compute_damage, DamageFactors, NEUTRAL};
use crate::log::{CombatLog, LogKind, NO_SPELL};
use crate::replay::{Recording, ReplayResult};
use crate::rng::{Stream, Streams};
use crate::rules_event::{RuleEvent, RuleEvents, RuleReceiver};
use crate::stacks::{PendingGain, StackMechanic};
use crate::sublimation::{shorten_cooldowns, PendingShorten, PendingSlot, SublimationMechanic};
use crate::time::{snap_up, Millis, GCD_MS, TICK_MS};

/// Kampfzustand „laeuft" — der Wert, den `src/core/combat.ts` als
/// `FIGHT_RUNNING` fuehrt.
///
/// **Er aendert sich waehrend eines Durchgangs nicht**, und das ist kein
/// Versehen: Weder `src/devtools/reference-run.ts` noch
/// `src/devtools/recording.ts` rufen `checkDefeat` auf — beide pruefen den
/// Boss-Tod selbst und schreiben den Zustand dabei nicht fort. Da die
/// Zustands-Pruefsumme dieses Feld einruehrt, muss der Simulator es genauso
/// halten; ein hier gesetzter Sieg-Zustand ergaebe ab dem Sieg-Takt eine
/// abweichende Pruefsumme, ohne dass eine Regel anders gerechnet waere.
pub const FIGHT_RUNNING: i32 = 0;

/// Eine Rotation: Welcher Zauber soll jetzt gewirkt werden?
///
/// `None` heisst bewusst warten. Neben dieser einfachen Form gibt es seit E2-3
/// die Prioritaetenliste als eigene Sprache ([`crate::apl`]); beide Wege laufen
/// ueber [`Decide`] durch dieselbe Kampfschleife.
pub type Rotation = fn(&Fight) -> Option<usize>;

/// Wer entscheidet, was gewirkt wird.
///
/// Zwei Umsetzungen: eine gewoehnliche Funktion (die vier Pruefrotationen) und
/// [`crate::apl::Apl`], die geschriebene Prioritaetenliste.
pub trait Decide {
    /// Welcher Zauber jetzt gewirkt werden soll, oder `None` fuer warten.
    fn decide(&self, fight: &Fight) -> Option<usize>;

    /// Wann die Entscheidung erneut geprueft werden will, weil eine ihrer
    /// Bedingungen **allein durch verstreichende Zeit** umschlagen kann.
    ///
    /// Der Kampf springt von Ereignis zu Ereignis. Ohne diese Auskunft wuerde
    /// eine Bedingung wie „Restlaufzeit unter einem GCD" erst beim naechsten
    /// ohnehin anstehenden Ereignis bemerkt — und das kann Sekunden spaeter
    /// sein. Das war der Vorabfund aus Issue #53; hier wird er behoben.
    ///
    /// Die Voreinstellung `None` ist fuer zustandsabhaengige Rotationen richtig:
    /// Deren Bedingungen aendern sich nur, wenn ohnehin ein Ereignis eintritt.
    fn next_wakeup(&self, _fight: &Fight) -> Option<Millis> {
        None
    }
}

impl Decide for Rotation {
    fn decide(&self, fight: &Fight) -> Option<usize> {
        self(fight)
    }
}

/// Wie viele Ereignisse gleichzeitig offen sein koennen.
///
/// Ueberschlag fuer Schritt 1: je Zauber ein Weckruf aus der Abklingzeit, einer
/// aus der globalen Wartezeit, ein DoT-Tick, ein Verfall-Schritt — zu keinem
/// Zeitpunkt mehr als eine Handvoll. 256 ist grosszuegig bemessen und kostet
/// einmalig 4 KB. Der Wert ist wichtig, weil die Schlange bei Ueberlauf
/// **nachfordern** wuerde, und das waere eine Speicheranforderung im Schritt.
const EVENT_CAPACITY: usize = 256;

/// Einstellungen eines Durchgangs.
#[derive(Debug, Clone, Copy)]
pub struct FightConfig {
    /// Startwert der Zufallsstroeme.
    ///
    /// 32 Bit, weil das Spiel 32 Bit fuehrt (`src/core/rng.ts`). Ein breiterer
    /// Startwert liesse sich hier annehmen und waere im Spiel nicht darstellbar —
    /// aufgezeichnete Runden waeren dann nicht mehr uebertragbar.
    pub seed: u32,
    /// Kampfdauer. `None` heisst: der Ueberdruck-Timer des Bosses.
    pub duration_ms: Option<Millis>,
    /// Ob der Kampf endet, sobald der Boss faellt.
    ///
    /// `false` wird fuer den Vergleich zweier Rotationen ueber die volle
    /// Kampfzeit gebraucht (`STEP1-SPEC` § 4) — sonst rechnet die bessere
    /// Rotation kuerzer und sieht dadurch schlechter aus, als sie ist.
    pub stop_on_kill: bool,
    /// Wie viele Protokoll-Eintraege aufbewahrt werden. 0 schaltet das
    /// Protokoll ab.
    pub log_capacity: usize,
    /// Die Faktoren der Schadensformel (GAME-DESIGN 6.3).
    ///
    /// In Schritt 1 durchweg neutral (`STEP1-SPEC` § 2). Der Balance-Laeufer
    /// veraendert je einen davon, um die Empfindlichkeit zu messen.
    pub factors: DamageFactors,
    /// Gegen welchen Boss gerechnet wird.
    ///
    /// Voreinstellung ist der Boss aus `data/bosses/`, der als Standard
    /// gekennzeichnet ist. Mit 68 Bossen (GAME-DESIGN 11.7) wird das der
    /// haeufigste Schalter des Balance-Laeufers.
    pub boss: &'static Boss,
}

impl Default for FightConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            duration_ms: None,
            stop_on_kill: true,
            log_capacity: 4096,
            factors: NEUTRAL,
            boss: BOSS,
        }
    }
}

/// Das Ergebnis eines Durchgangs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    /// Gesamtschaden.
    pub damage: u32,
    /// Kampfzeit des Sieges, oder `None`, wenn der Boss ueberlebt hat.
    pub killed_at_ms: Option<Millis>,
    /// Verbleibende Zeit bis zum Ueberdruck, wenn der Boss gefallen ist.
    pub remaining_ms: Millis,
    /// Wirkungen je Zauber.
    pub casts: [u32; SPELL_COUNT],
    /// Schaden je Zauber.
    pub damage_by_spell: [u32; SPELL_COUNT],
    /// Durch zu fruehes Erneuern verfallene Ticks, je Zauber.
    pub lost_dot_ticks: [u32; SPELL_COUNT],
    /// Am Deckel verpuffte Ressource.
    pub resource_overflow: u32,
    /// Zeit, in der gewartet wurde, obwohl gewirkt werden durfte.
    pub idle_ms: Millis,
    /// Ziehungen aus den Zufallsstroemen. In Schritt 1 muss das 0 sein.
    pub random_draws: u32,
}

/// Ein Kampf-Durchgang.
pub struct Fight {
    // -- Zeit --
    time: Millis,
    duration_ms: Millis,
    stop_on_kill: bool,

    // -- Boss --
    boss: &'static Boss,
    damage: u32,
    killed_at: Option<Millis>,
    /// Welche Phase gerade laeuft. 0 ist der Start.
    phase: usize,
    /// Stand der Energieleiste — der zweiten Uhr.
    energy: u32,
    /// Wie oft die Energieleiste ausgeloest hat.
    energy_triggers: u32,
    /// Wie weit die Boss-Zeitleiste abgearbeitet ist.
    ///
    /// Zeigt auf den **naechsten** faelligen Eintrag. Steht er am Ende der
    /// Liste, ist die Zeitleiste durch.
    timeline_index: usize,

    // -- Spieler --
    gcd_ready_at: Millis,
    cooldown_ready_at: [Millis; SPELL_COUNT],
    resource: u16,
    dot_expires_at: [Option<Millis>; SPELL_COUNT],
    dot_generation: [u16; SPELL_COUNT],

    // -- Zaehlwerk --
    casts: [u32; SPELL_COUNT],
    damage_by_spell: [u32; SPELL_COUNT],
    lost_dot_ticks: [u32; SPELL_COUNT],
    dot_tick_counts: [u32; SPELL_COUNT],
    resource_overflow: u32,
    idle_ms: Millis,

    // -- Regeln --
    factors: DamageFactors,

    // -- Werkzeuge --
    queue: EventQueue,
    log: CombatLog,
    rng: Streams,

    /// Was die Schul-Mechanik angesetzt, der Kampf aber noch nicht gebucht hat
    /// (Issue E3-2). Siehe [`crate::stacks`] fuer den Grund der Teilung.
    pending_gain: PendingGain,
    /// Wie oft Stapel angesetzt wurden.
    stack_gains: std::rc::Rc<std::cell::Cell<u32>>,
    /// Verkuerzung der Abklingzeiten, angesetzt und noch nicht ausgefuehrt
    /// (Issue E3-3). Siehe [`crate::sublimation`] fuer den Grund der Teilung.
    pending_shorten: PendingSlot,
    /// Wie oft die Sublimation angesetzt hat.
    sublimation_uses: std::rc::Rc<std::cell::Cell<u32>>,

    /// Die Regel-Ereignisse (GAME-DESIGN 18.3, Issue E3-1b).
    ///
    /// **Erst der Schaden, dann "Zauber gewirkt"** — Entscheidung des
    /// Auftraggebers vom 2026-07-29. Ein Empfaenger reagiert auf eine
    /// abgeschlossene Handlung und beeinflusst den naechsten Zauber, nicht den
    /// laufenden. Dieselbe Reihenfolge im Spiel (`src/core/combat.ts`).
    events: RuleEvents,
}

impl Fight {
    /// Legt einen Durchgang an. Fordert allen benoetigten Speicher **jetzt** an.
    #[must_use]
    pub fn new(config: FightConfig) -> Self {
        let mut fight = Self {
            time: 0,
            duration_ms: config.duration_ms.unwrap_or(config.boss.overpressure_ms),
            stop_on_kill: config.stop_on_kill,
            boss: config.boss,
            damage: 0,
            killed_at: None,
            phase: 0,
            energy: 0,
            energy_triggers: 0,
            timeline_index: 0,
            gcd_ready_at: 0,
            cooldown_ready_at: [0; SPELL_COUNT],
            resource: 0,
            dot_expires_at: [None; SPELL_COUNT],
            dot_generation: [0; SPELL_COUNT],
            casts: [0; SPELL_COUNT],
            damage_by_spell: [0; SPELL_COUNT],
            lost_dot_ticks: [0; SPELL_COUNT],
            dot_tick_counts: [0; SPELL_COUNT],
            resource_overflow: 0,
            idle_ms: 0,
            factors: config.factors,
            queue: EventQueue::with_capacity(EVENT_CAPACITY),
            log: CombatLog::with_capacity(config.log_capacity),
            rng: Streams::new(config.seed),
            // **Ohne Vorab-Speicher.** Der erste Versuch legte vier Plaetze je
            // Ereignis an — und `tests/allocation.rs` schlug sofort an: Der
            // Aufbau darf ohne Protokoll genau eine Anforderung kosten, die
            // Ereignis-Schlange. Vorab-Speicher fuer Empfaenger, die es noch
            // nicht gibt, kauft auch nichts: Angemeldet wird beim Aufbau, nicht
            // im Schritt (GAME-DESIGN 18.4 verlangt nur Letzteres).
            events: RuleEvents::default(),
            pending_gain: PendingGain::default(),
            stack_gains: std::rc::Rc::default(),
            pending_shorten: PendingSlot::default(),
            sublimation_uses: std::rc::Rc::default(),
        };
        fight.register_school_mechanics();
        fight
    }

    /// Die Schul-Mechanik anmelden (Issue E3-2).
    ///
    /// Eine Zeile fuer alle Schulen: Welcher Vorrat gefuellt wird und wie viel,
    /// steht in den Daten des jeweiligen Zaubers.
    fn register_school_mechanics(&mut self) {
        let mechanic = StackMechanic::new(
            std::rc::Rc::clone(&self.pending_gain),
            std::rc::Rc::clone(&self.stack_gains),
        );
        self.events
            .register("stacks:heat", RuleEvent::SpellCast, 0, Box::new(mechanic));
        let second = StackMechanic::new(
            std::rc::Rc::clone(&self.pending_gain),
            std::rc::Rc::clone(&self.stack_gains),
        );
        self.events
            .register("stacks:heat", RuleEvent::DotTick, 0, Box::new(second));

        // SUBLIMATION (Issue E3-3), Rang 10: nach der Stapel-Mechanik. Die
        // Verkuerzungen kommen aus den Zauber-Daten; wer nichts verkuerzt,
        // steht mit 0 in der Tabelle.
        let mut per_cast = [0; SPELL_COUNT];
        for (index, spell) in SPELLS.iter().enumerate() {
            per_cast[index] = spell.cooldown_reduction_ms;
        }
        self.events.register(
            "sublimation",
            RuleEvent::SpellCast,
            10,
            Box::new(SublimationMechanic::new(
                per_cast,
                std::rc::Rc::clone(&self.pending_shorten),
                std::rc::Rc::clone(&self.sublimation_uses),
            )),
        );
    }

    /// Wie oft die Sublimation angesetzt hat.
    #[must_use]
    pub fn sublimation_use_count(&self) -> u32 {
        self.sublimation_uses.get()
    }

    /// Wie oft die Schul-Mechanik Stapel angesetzt hat.
    #[must_use]
    pub fn stack_gain_count(&self) -> u32 {
        self.stack_gains.get()
    }

    /// Ein Regel-Ereignis austragen und einsammeln, was die Schul-Mechanik
    /// dabei angesetzt hat.
    ///
    /// **Die einzige Stelle, an der gebucht wird.** Der Empfaenger kann nicht
    /// selbst buchen (siehe [`crate::stacks`]); dass es hier und nur hier
    /// geschieht, haelt die Regel trotzdem an einem Ort.
    fn emit_rule(&mut self, event: RuleEvent, subject: i16, value: f64) {
        let now = self.time;
        self.events.emit(event, subject, value, now);
        let gained = self.pending_gain.replace(0);
        if gained > 0 {
            self.add_resource(gained);
        }
        let order = self.pending_shorten.replace(PendingShorten::default());
        if order.amount > 0 {
            shorten_cooldowns(&mut self.cooldown_ready_at, order, now);
        }
    }

    /// Einen Empfaenger fuer ein Regel-Ereignis anmelden (Issue E3-1b).
    ///
    /// Laeuft beim Aufbau, nicht im Schritt.
    pub fn on_rule_event(
        &mut self,
        id: &str,
        on: RuleEvent,
        order: i32,
        receiver: Box<dyn RuleReceiver>,
    ) {
        self.events.register(id, on, order, receiver);
    }

    /// Die Regel-Ereignisse dieses Durchgangs — Folge, Anzahl, Empfaenger.
    #[must_use]
    pub const fn rule_events(&self) -> &RuleEvents {
        &self.events
    }

    // ---------------------------------------------------------------- Ablauf

    /// Rechnet den Kampf mit der angegebenen Rotation durch.
    ///
    /// Ein bereits gelaufener Kampf muss vorher mit [`Fight::reset`]
    /// zurueckgesetzt werden.
    pub fn run(&mut self, rotation: Rotation) -> RunResult {
        self.run_with(&rotation)
    }

    /// Rechnet den Kampf mit einer geschriebenen Prioritaetenliste durch.
    ///
    /// Nutzt dieselbe Schleife wie [`Fight::run`] — es gibt nur **eine**
    /// Regelrechnung, nicht zwei, die auseinanderlaufen koennten.
    pub fn run_apl(&mut self, apl: &crate::apl::Apl) -> RunResult {
        self.run_with(apl)
    }

    fn run_with<D: Decide + ?Sized>(&mut self, decider: &D) -> RunResult {
        // Der Verfall laeuft ab Kampfbeginn, unabhaengig davon, ob ueberhaupt
        // Ressource da ist — so macht es die Referenz, und das ist auch richtig:
        // Der Takt haengt am Kampf, nicht am Fuellstand.
        if RESOURCE.decay_per_step_ms > 0 {
            self.queue
                .push(RESOURCE.decay_per_step_ms, EventKind::ResourceDecay);
        }
        self.schedule_first_energy_tick();
        self.schedule_next_timeline_step();
        // Die erste Entscheidung faellt bei 0.
        self.queue.push(0, EventKind::Wakeup);

        while let Some(now) = self.queue.peek_time() {
            if now >= self.duration_ms {
                break;
            }
            self.time = now;

            // Phase 1 — alle passiven Wirkungen dieses Zeitpunkts, in der
            // Rangfolge aus `event.rs`. Erst wenn keine mehr offen ist, darf
            // geprueft und entschieden werden; sonst saehe die Entscheidung
            // einen halb fortgeschriebenen Zustand.
            while self.queue.peek_time() == Some(now) {
                let event = self.queue.pop().expect("peek_time hat bestaetigt");
                self.apply(event.kind);
            }

            // Phase 2 — Sieg-Pruefung, an derselben Stelle wie in der Referenz.
            if self.killed_at.is_none() && self.damage >= self.boss.hp {
                self.killed_at = Some(now);
                self.log.push(now, LogKind::Kill, NO_SPELL, self.damage);
                if self.stop_on_kill {
                    break;
                }
            }
            self.check_phase();

            // Phase 3 — die Entscheidung des Spielers.
            if self.gcd_ready_at <= now {
                match decider.decide(self) {
                    Some(index) => self.cast(index),
                    None => {
                        // Gewartet, obwohl gewirkt werden durfte. Die Referenz
                        // zaehlt dafuer je leerem Takt 10 ms; hier ist es
                        // dieselbe Spanne am Stueck, bis das naechste Ereignis
                        // den Zustand aendern kann.
                        let next = self.queue.peek_time().unwrap_or(self.duration_ms);
                        self.idle_ms += next.min(self.duration_ms) - now;
                    }
                }
            }

            self.schedule_decision_wakeup(decider, now);
        }

        self.result()
    }

    /// Spielt eine aufgezeichnete Runde ab (GAME-DESIGN 18.4).
    ///
    /// Anders als [`Fight::run`] entscheidet hier nicht der Simulator, was
    /// gewirkt wird — die Tastendruecke stehen fest. Genau das macht den
    /// Pruefall aussagekraeftig: Geprueft wird die **Regelrechnung**, nicht die
    /// Rotationswahl.
    ///
    /// # Warum das hier Takt fuer Takt laeuft
    ///
    /// Der uebrige Simulator springt von Ereignis zu Ereignis; das geht hier
    /// nicht, denn die Aufzeichnung ist taktgebunden und die Pruefsumme wird je
    /// Takt verglichen. Beide Wege benutzen aber **dieselben** Zustandsuebergaenge
    /// ([`Fight::apply`], [`Fight::cast`]) — es gibt nur eine Regelrechnung, nicht
    /// zwei, die auseinanderlaufen koennten.
    ///
    /// Der Ablauf ist der von `replayRecording` in `src/devtools/recording.ts`:
    /// je Takt erst die passiven Wirkungen, dann die Sieg-Pruefung mit Abbruch,
    /// dann die Tastendruecke, dann die Pruefsumme.
    pub fn replay(&mut self, recording: &Recording, checksum_per_tick: bool) -> ReplayResult {
        if RESOURCE.decay_per_step_ms > 0 {
            self.queue
                .push(RESOURCE.decay_per_step_ms, EventKind::ResourceDecay);
        }
        self.schedule_first_energy_tick();
        self.schedule_next_timeline_step();

        let mut checksums = Vec::with_capacity(if checksum_per_tick {
            recording.ticks as usize
        } else {
            0
        });
        let mut sequence: u32 = 0;
        let mut accepted = 0;
        let mut rejected = 0;
        let mut press_index = 0;

        for tick in 0..recording.ticks {
            let now = tick * TICK_MS;
            self.time = now;

            // Phase 1 — passive Wirkungen dieses Takts.
            while let Some(due) = self.queue.peek_time() {
                if due > now {
                    break;
                }
                let event = self.queue.pop().expect("peek_time hat bestaetigt");
                self.apply(event.kind);
            }

            self.check_phase();

            // Phase 2 — Sieg-Pruefung. Das Spiel bricht hier ab, und die
            // Pruefsumme dieses Takts wird noch geschrieben.
            if self.killed_at.is_none() && self.damage >= self.boss.hp {
                self.killed_at = Some(now);
                self.log.push(now, LogKind::Kill, NO_SPELL, self.damage);
                let hash = self.checksum_at(now);
                sequence = sequence.wrapping_mul(31).wrapping_add(hash);
                if checksum_per_tick {
                    checksums.push(hash);
                }
                break;
            }

            // Phase 3 — die Tastendruecke dieses Takts.
            while press_index < recording.presses.len() {
                let press = recording.presses[press_index];
                if press.tick > tick {
                    break;
                }
                press_index += 1;
                match recording.spell_for_slot(press.slot) {
                    // Unbelegter Platz: Fehleingabe, nichts wird gemerkt.
                    None => rejected += 1,
                    Some(index) => {
                        if self.is_ready(index) {
                            self.cast(index);
                            accepted += 1;
                        } else {
                            // Zu frueh gedrueckt. Das Spiel merkt sich den Druck
                            // ausdruecklich **nicht** (GAME-DESIGN 21.1: keine
                            // Eingabe-Vormerkung) — er ist damit verloren.
                            rejected += 1;
                        }
                    }
                }
            }

            let hash = self.checksum_at(now);
            sequence = sequence.wrapping_mul(31).wrapping_add(hash);
            if checksum_per_tick {
                checksums.push(hash);
            }
        }

        let final_time = self
            .killed_at
            .unwrap_or_else(|| recording.ticks.saturating_sub(1) * TICK_MS);

        ReplayResult {
            damage: self.damage,
            killed_at_ms: self.killed_at,
            accepted,
            rejected,
            casts: self.casts.to_vec(),
            final_checksum: self.checksum_at(final_time),
            sequence_checksum: sequence,
            checksums,
            random_draws: self.rng.total_draws(),
        }
    }

    /// Setzt den Kampf zurueck, **ohne** Speicher freizugeben oder neu
    /// anzufordern. So kostet ein zweiter Durchgang nichts.
    pub fn reset(&mut self, seed: u32) {
        self.time = 0;
        self.damage = 0;
        self.killed_at = None;
        self.phase = 0;
        self.energy = 0;
        self.energy_triggers = 0;
        self.timeline_index = 0;
        self.gcd_ready_at = 0;
        self.cooldown_ready_at = [0; SPELL_COUNT];
        self.resource = 0;
        self.dot_expires_at = [None; SPELL_COUNT];
        self.dot_generation = [0; SPELL_COUNT];
        self.casts = [0; SPELL_COUNT];
        self.damage_by_spell = [0; SPELL_COUNT];
        self.lost_dot_ticks = [0; SPELL_COUNT];
        self.dot_tick_counts = [0; SPELL_COUNT];
        self.resource_overflow = 0;
        self.idle_ms = 0;
        self.queue.clear();
        self.log.clear();
        self.rng = Streams::new(seed);
        // Nur die Folge, nicht die Anmeldungen: Ein neuer Durchgang behaelt
        // seine Empfaenger und beginnt lediglich wieder bei null Ereignissen.
        self.events.clear_sequence();
    }

    /// Plant den ersten Anstieg der Energieleiste ein, falls der Boss eine hat.
    fn schedule_first_energy_tick(&mut self) {
        if let Some(energy) = self.boss.energy {
            if energy.per_step_ms > 0 && energy.per_step_ms < self.duration_ms {
                self.queue.push(energy.per_step_ms, EventKind::EnergyTick);
            }
        }
    }

    /// Plant den naechsten faelligen Eintrag der Boss-Zeitleiste ein.
    ///
    /// Immer nur **einen**. Alle auf einmal einzuplanen waere kuerzer, wuerde
    /// aber die Ereignis-Schlange mit Eintraegen fuellen, die ein frueher
    /// Sieg ohnehin nie erreicht — und ihre Kapazitaet ist bewusst fest
    /// ([`EVENT_CAPACITY`]).
    fn schedule_next_timeline_step(&mut self) {
        let Some(entry) = self.boss.timeline.get(self.timeline_index) else {
            return;
        };
        if entry.at_ms < self.duration_ms {
            self.queue.push(
                entry.at_ms,
                EventKind::TimelineStep {
                    index: self.timeline_index as u16,
                },
            );
        }
    }

    /// Ein Eintrag der Zeitleiste ist faellig (GAME-DESIGN 11.1).
    ///
    /// **Noch ohne Wirkung.** Die neun Mechanik-Bausteine kommen in E4-2 bis
    /// E4-6, die Zeitleisten der einzelnen Bosse in E10. Was hier schon steht,
    /// ist der Zeitpunkt: Er wird taktgenau angesteuert, protokolliert und geht
    /// ueber den Zeiger in die Zustands-Pruefsumme ein. Ein spaeter angehaengter
    /// Baustein setzt damit auf nachgewiesener Zeitrechnung auf statt auf einer
    /// erst dann erstmals gepruefen.
    fn timeline_step(&mut self, index: usize) {
        // Ein veralteter Eintrag: Der Zeiger ist bereits weiter. Kann heute
        // nicht vorkommen, weil immer nur einer offen ist — aber wenn E4-2
        // Eintraege ueberspringbar macht, faengt es das hier ab, statt den
        // Zeiger rueckwaerts laufen zu lassen.
        if index != self.timeline_index {
            return;
        }
        self.log
            .push(self.time, LogKind::BossTimeline, NO_SPELL, index as u32);
        self.timeline_index += 1;
        self.schedule_next_timeline_step();
    }

    fn apply(&mut self, kind: EventKind) {
        match kind {
            EventKind::DotTick { spell, generation } => self.dot_tick(spell as usize, generation),
            EventKind::ResourceDecay => self.decay(),
            EventKind::EnergyTick => self.energy_tick(),
            EventKind::TimelineStep { index } => self.timeline_step(index as usize),
            // Ein Weckruf traegt keine Wirkung. Er sorgt nur dafuer, dass der
            // Simulator zu diesem Zeitpunkt ueberhaupt aufwacht — weil dann eine
            // Abklingzeit abgelaufen ist oder der Sieg zu pruefen war.
            EventKind::Wakeup => {}
        }
    }

    fn dot_tick(&mut self, index: usize, generation: u16) {
        // Veraltet: Der Effekt wurde inzwischen erneuert, und `refreshMode:
        // "reset"` laesst die alte Anwendung verfallen (`STEP1-SPEC` 1.2).
        if generation != self.dot_generation[index] {
            return;
        }
        if !self.dot_active(index) {
            return;
        }
        let Some(dot) = SPELLS[index].dot else {
            return;
        };

        let tick_damage = self.deal(
            index,
            compute_damage(dot.tick_damage, &self.factors),
            LogKind::DotTick,
        );
        // Ein Tick ist **ein** Ereignis, nicht zwei. Wer allen Schaden zaehlen
        // will, meldet sich fuer beide Arten an — so bleibt die Folge eindeutig.
        self.emit_rule(RuleEvent::DotTick, index as i16, f64::from(tick_damage));
        self.dot_tick_counts[index] += 1;
        // Der Hitze-Gewinn stand bis E3-2 hier. Er haengt jetzt am Ereignis.

        // Der naechste Tick nur, wenn er noch in die Laufzeit faellt. Das `<=`
        // ist dieselbe Entscheidung wie das `>=` in [`Fight::dot_active`]:
        // 12 s Laufzeit bei 2 s Abstand ergeben **6** Ticks, der letzte genau
        // beim Ablauf (`STEP1-SPEC` 1.2).
        let next = self.time + dot.tick_interval_ms;
        if self.dot_expires_at[index].is_some_and(|expires| next <= expires) {
            self.queue.push(
                next,
                EventKind::DotTick {
                    spell: index as u8,
                    generation,
                },
            );
        }
    }

    /// Die Energieleiste steigt (GAME-DESIGN 11.2, Baustein 1).
    ///
    /// **Unabhaengig vom Schaden des Spielers** — das ist die eigentliche
    /// Aussage des Bausteins. Wer schneller zuschlaegt, verschiebt den
    /// Ausloeser nicht; damit entsteht ein zweiter Takt, gegen den geplant
    /// werden muss.
    fn energy_tick(&mut self) {
        let Some(energy) = self.boss.energy else {
            return;
        };

        self.energy = self.energy.saturating_add(energy.amount);
        if self.energy >= energy.threshold {
            self.energy_triggers += 1;
            self.energy = energy.reset_to;
            self.log.push(
                self.time,
                LogKind::BossEnergy,
                NO_SPELL,
                self.energy_triggers,
            );
        }

        let next = self.time + energy.per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::EnergyTick);
        }
    }

    /// Prueft, ob der Boss die Phase wechselt.
    ///
    /// An derselben Stelle wie die Sieg-Pruefung: nach den passiven Wirkungen,
    /// vor der Entscheidung des Spielers. Das ist kein Zufall — beide haengen
    /// am selben Wert, den Lebenspunkten, und muessen deshalb denselben
    /// Zeitpunkt sehen.
    ///
    /// **Eskalierend, nicht springend:** Die Phase kann nur steigen. Heilt der
    /// Boss spaeter einmal (GAME-DESIGN 11.2, Baustein 5), faellt sie nicht
    /// zurueck — eine schon gezeigte Mechanik verschwindet nicht wieder.
    fn check_phase(&mut self) {
        if self.boss.phases.is_empty() {
            return;
        }
        let neue = self.boss.phase_at(self.boss_hp_percent());
        if neue > self.phase {
            self.phase = neue;
            self.log
                .push(self.time, LogKind::BossPhase, NO_SPELL, neue as u32);
        }
    }

    fn decay(&mut self) {
        self.resource = self.resource.saturating_sub(RESOURCE.decay_amount);
        let next = self.time + RESOURCE.decay_per_step_ms;
        if next < self.duration_ms {
            self.queue.push(next, EventKind::ResourceDecay);
        }
    }

    /// Wirkt einen Zauber.
    ///
    /// Die Reihenfolge der Wirkungen ist die der Referenz und nicht beliebig:
    /// Ein Entlade-Zauber liest die Ressource, **bevor** ein Gewinn desselben
    /// Zaubers sie erhoehen koennte. In Schritt 1 gibt es keinen Zauber, der
    /// beides tut — die Festlegung verhindert, dass der erste solche Zauber
    /// stillschweigend anders rechnet als im Spiel.
    fn cast(&mut self, index: usize) {
        let spell: &Spell = &SPELLS[index];
        let now = self.time;

        self.casts[index] += 1;

        // **Erst der Schaden, dann die Wirkung.** Bis zu Issue E3-1b stand der
        // Cast-Eintrag hier oben und damit VOR dem Schaden — das Spiel
        // (`src/core/combat.ts`) schreibt seit E1-7 umgekehrt. Zwei
        // Erzaehlungen desselben Kampfes, die sich widersprechen, sind beim
        // Abgleich schlimmer als eine; deshalb ist der Simulator angeglichen
        // worden und nicht das Spiel (Entscheidung des Auftraggebers,
        // 2026-07-29). Verglichen wurde bis dahin nur Zahlenwerk, weshalb es
        // niemandem auffiel.
        let mut cast_damage = 0;

        if spell.base_damage > 0 {
            cast_damage += self.deal(
                index,
                compute_damage(spell.base_damage, &self.factors),
                LogKind::Damage,
            );
        }

        if let Some(spender) = spell.spender {
            let spent = self.resource;
            let mut damage = spender.damage_per_point * u32::from(spent);
            if spent >= spender.bonus_threshold {
                damage *= spender.bonus_multiplier;
            }
            self.log
                .push(now, LogKind::Spend, index as u8, u32::from(spent));
            cast_damage += self.deal(
                index,
                compute_damage(damage, &self.factors),
                LogKind::Damage,
            );
            self.resource = 0;
        }

        // **Ein Schaden-Ereignis je Wirkung, mit der Summe** — nicht eines je
        // Teilbetrag. Das Spiel rechnet Grund- und Entladeschaden zusammen und
        // meldet einmal; ein Zauber, der beides hat, ergaebe hier sonst zwei
        // Ereignisse und dort eines. In Schritt 1 gibt es keinen solchen
        // Zauber — genau deshalb ist es jetzt festzulegen und nicht dann.
        if cast_damage > 0 {
            self.emit_rule(RuleEvent::Damage, index as i16, f64::from(cast_damage));
        }
        // Hierher gehoert `RuleEvent::Crit`. Krit ruht in Schritt 1
        // (PURITY = 0, `STEP1-SPEC` 2); ein Ausloeser, der nie zutrifft, waere
        // toter Code.

        if let Some(dot) = spell.dot {
            if self.dot_active(index) {
                // Erneuern verwirft die Restzeit (GAME-DESIGN 7.2). Was dabei an
                // Ticks verloren geht, ist die Kennzahl, an der die Fehleranalyse
                // den haeufigsten Anfaengerfehler festmacht.
                let remaining = self.dot_expires_at[index].unwrap_or(now) - now;
                self.lost_dot_ticks[index] += remaining / dot.tick_interval_ms;
            }
            self.dot_generation[index] = self.dot_generation[index].wrapping_add(1);
            self.dot_expires_at[index] = Some(now + dot.duration_ms);
            let first = snap_up(now + dot.tick_interval_ms);
            self.queue.push(
                first,
                EventKind::DotTick {
                    spell: index as u8,
                    generation: self.dot_generation[index],
                },
            );
        }

        let ready_at = snap_up(now + spell.cooldown_ms);
        self.cooldown_ready_at[index] = ready_at;
        self.queue.push(ready_at, EventKind::Wakeup);

        if spell.triggers_gcd {
            self.gcd_ready_at = snap_up(now + GCD_MS);
            self.queue.push(self.gcd_ready_at, EventKind::Wakeup);
        }

        // Der Sieg wird erst in der Pruef-Phase des **naechsten** Takts bemerkt,
        // weil die Pruefung vor der Wirk-Phase steht. Ohne diesen Weckruf
        // koennte der Simulator bis zum naechsten regulaeren Ereignis
        // weiterspringen und einen zu spaeten Zeitpunkt melden.
        if self.damage >= self.boss.hp {
            self.queue.push(now + TICK_MS, EventKind::Wakeup);
        }

        // Zuletzt: Die Wirkung ist abgeschlossen — Schaden, Effekt ueber Zeit,
        // Vorrat, Abklingzeit und Wartezeit stehen.
        self.log.push(now, LogKind::Cast, index as u8, 0);
        self.emit_rule(RuleEvent::SpellCast, index as i16, f64::from(cast_damage));
    }

    /// Traegt Schaden ein und gibt den Betrag zurueck.
    fn deal(&mut self, index: usize, amount: u32, kind: LogKind) -> u32 {
        self.damage += amount;
        self.damage_by_spell[index] += amount;
        self.log.push(self.time, kind, index as u8, amount);
        amount
    }

    fn add_resource(&mut self, amount: u16) {
        let before = self.resource;
        self.resource = (before + amount).min(RESOURCE.cap);
        self.resource_overflow += u32::from(amount - (self.resource - before));
    }

    fn result(&self) -> RunResult {
        RunResult {
            damage: self.damage,
            killed_at_ms: self.killed_at,
            remaining_ms: self.killed_at.map_or(0, |t| self.duration_ms - t),
            casts: self.casts,
            damage_by_spell: self.damage_by_spell,
            lost_dot_ticks: self.lost_dot_ticks,
            resource_overflow: self.resource_overflow,
            idle_ms: self.idle_ms,
            random_draws: self.rng.total_draws(),
        }
    }

    // ------------------------------------------------- Abfragen fuer Rotationen

    /// Aktuelle Kampfzeit.
    #[must_use]
    pub fn time(&self) -> Millis {
        self.time
    }

    /// Aktueller Stand der Schul-Ressource.
    #[must_use]
    pub fn resource(&self) -> u16 {
        self.resource
    }

    /// Bisher verursachter Schaden.
    #[must_use]
    pub fn damage(&self) -> u32 {
        self.damage
    }

    /// Ob ein Zauber jetzt gewirkt werden kann: eigene Abklingzeit **und**
    /// globale Wartezeit abgelaufen.
    #[must_use]
    pub fn is_ready(&self, index: usize) -> bool {
        self.cooldown_ready_at[index] <= self.time && self.gcd_ready_at <= self.time
    }

    /// Ob ein Brand-Effekt gerade laeuft.
    ///
    /// `>=` und nicht `>`: Der letzte Tick faellt mit dem Ablauf zusammen
    /// (`STEP1-SPEC` 1.2, dort am 2026-07-28 zugunsten von 6 Ticks entschieden).
    #[must_use]
    pub fn dot_active(&self, index: usize) -> bool {
        self.dot_expires_at[index].is_some_and(|expires| expires >= self.time)
    }

    /// Restlaufzeit eines Brand-Effekts; 0, wenn er nicht laeuft.
    #[must_use]
    pub fn dot_remaining_ms(&self, index: usize) -> Millis {
        self.dot_expires_at[index].map_or(0, |expires| expires.saturating_sub(self.time))
    }

    /// Plant einen Weckruf ein, wenn die Entscheidung einen braucht.
    ///
    /// Nur, wenn er **frueher** liegt als das naechste ohnehin anstehende
    /// Ereignis — sonst fuellte sich die Schlange mit Weckrufen, die nichts
    /// bringen, und die Kapazitaet aus [`EVENT_CAPACITY`] waere schneller
    /// erreicht als noetig.
    ///
    /// Vor Ablauf der globalen Wartezeit ist eine erneute Pruefung ebenfalls
    /// zwecklos: Gewirkt werden kann ohnehin nicht.
    fn schedule_decision_wakeup<D: Decide + ?Sized>(&mut self, decider: &D, now: Millis) {
        let Some(wanted) = decider.next_wakeup(self) else {
            return;
        };
        let at = wanted.max(self.gcd_ready_at);
        if at <= now || at >= self.duration_ms {
            return;
        }
        if self.queue.peek_time().is_none_or(|next| at < next) {
            self.queue.push(at, EventKind::Wakeup);
        }
    }

    /// Restliche Abklingzeit eines Zaubers **jetzt**.
    #[must_use]
    pub fn cooldown_remaining_ms(&self, index: usize) -> Millis {
        self.cooldown_remaining_ms_at(index, self.time)
    }

    /// Ob die eigene Abklingzeit eines Zaubers abgelaufen ist.
    ///
    /// **Ohne** die globale Wartezeit — anders als [`Fight::is_ready`].
    /// Entscheidungen fallen ohnehin nur, wenn der GCD bereit ist; eine
    /// Prioritaetenliste soll deshalb nach der Abklingzeit fragen koennen, ohne
    /// die Wartezeit jedes Mal mitzudenken.
    #[must_use]
    pub fn cooldown_ready(&self, index: usize) -> bool {
        self.cooldown_ready_at[index] <= self.time
    }

    /// Verbleibende Lebenspunkte des Bosses.
    #[must_use]
    pub fn boss_hp_remaining(&self) -> u32 {
        self.boss.hp.saturating_sub(self.damage)
    }

    /// Verbleibende Lebenspunkte in Prozent, ganzzahlig abgerundet.
    ///
    /// Ganzzahlig, weil die Prioritaetenliste ganzzahlig rechnet — und weil ein
    /// Fliesskomma-Vergleich an dieser Stelle zwischen zwei Umsetzungen
    /// unterschiedlich ausfallen koennte.
    #[must_use]
    pub fn boss_hp_percent(&self) -> u32 {
        if self.boss.hp == 0 {
            return 0;
        }
        (self.boss_hp_remaining() * 100) / self.boss.hp
    }

    /// Zeit bis zum Ueberdruck.
    #[must_use]
    pub fn fight_remaining_ms(&self) -> Millis {
        self.duration_ms.saturating_sub(self.time)
    }

    /// Restliche Abklingzeit eines Zaubers zu einem beliebigen Zeitpunkt.
    ///
    /// Der Zeitpunkt wird uebergeben statt aus dem Zustand genommen, weil die
    /// Pruefsumme ihn fuer **jeden Takt** braucht — auch fuer die, an denen der
    /// ereignisgesteuerte Simulator gar nicht anhaelt.
    #[must_use]
    pub fn cooldown_remaining_ms_at(&self, index: usize, time_ms: Millis) -> Millis {
        self.cooldown_ready_at[index].saturating_sub(time_ms)
    }

    /// Restlaufzeit eines Brand-Effekts zu einem beliebigen Zeitpunkt.
    #[must_use]
    pub fn dot_remaining_ms_at(&self, index: usize, time_ms: Millis) -> Millis {
        self.dot_expires_at[index].map_or(0, |expires| expires.saturating_sub(time_ms))
    }

    /// Wie oft ein Zauber gewirkt wurde.
    #[must_use]
    pub fn cast_count(&self, index: usize) -> u32 {
        self.casts[index]
    }

    /// Wie oft ein Brand-Effekt getickt hat.
    #[must_use]
    pub fn dot_tick_count(&self, index: usize) -> u32 {
        self.dot_tick_counts[index]
    }

    /// Durch zu fruehes Erneuern verfallene Ticks.
    #[must_use]
    pub fn lost_dot_ticks(&self, index: usize) -> u32 {
        self.lost_dot_ticks[index]
    }

    /// Am Deckel verpuffte Ressource.
    #[must_use]
    pub fn resource_overflow(&self) -> u32 {
        self.resource_overflow
    }

    /// Wann die globale Wartezeit ablaeuft.
    #[must_use]
    pub fn gcd_ready_at(&self) -> Millis {
        self.gcd_ready_at
    }

    /// Zustands-Pruefsumme zu einem Zeitpunkt (GAME-DESIGN 19.1).
    ///
    /// **Die Reihenfolge der Felder ist Teil der Schnittstelle** und stammt aus
    /// `combatChecksum` in `src/core/checksum.ts`. Ein Feld dazwischenzuschieben
    /// oder zwei zu vertauschen aendert jede Summe — dann meldet der Abgleich
    /// bei jedem Takt eine Abweichung und bei keinem eine echte.
    ///
    /// Welcher Wert als ganze Zahl und welcher als Fliesskommazahl eingeht, ist
    /// ebenfalls vorgegeben: Im Spiel sind Schaden, Wartezeit und Restzeiten
    /// `number`, also Fliesskomma, auch wenn sie ganzzahlige Werte tragen.
    #[must_use]
    pub fn checksum_at(&self, time_ms: Millis) -> u32 {
        let mut h = FNV_OFFSET;
        h = mix_int(h, time_ms as i32);
        h = mix_float(h, f64::from(self.damage));
        h = mix_int(h, i32::from(self.resource));
        h = mix_int(h, self.resource_overflow as i32);
        h = mix_int(h, FIGHT_RUNNING);
        h = mix_float(h, f64::from(self.gcd_ready_at));
        for index in 0..SPELL_COUNT {
            h = mix_float(h, f64::from(self.cooldown_remaining_ms_at(index, time_ms)));
            h = mix_float(h, f64::from(self.dot_remaining_ms_at(index, time_ms)));
            h = mix_int(h, self.casts[index] as i32);
            h = mix_int(h, self.dot_tick_counts[index] as i32);
            h = mix_int(h, self.lost_dot_ticks[index] as i32);
        }
        for draws in self.rng.draw_counts() {
            h = mix_int(h, draws as i32);
        }
        // **Angehaengt, nicht eingeschoben** (Issue #96). Die bestehenden
        // Positionen bleiben dadurch unveraendert; die Reihenfolge ist ohnehin
        // willkuerlich und muss nur auf beiden Seiten gleich sein.
        //
        // Ohne diese beiden Felder waere die Pruefsumme fuer einen Boss mit
        // Phasen blind: Zwei Laeufe in verschiedenen Phasen saehen gleich aus,
        // und der Abgleich meldete Uebereinstimmung, wo keine ist.
        h = mix_int(h, self.phase as i32);
        h = mix_int(h, self.energy as i32);
        h = mix_int(h, self.timeline_index as i32);
        h
    }

    /// Welche Phase gerade laeuft. 0 ist der Start.
    #[must_use]
    pub fn phase(&self) -> usize {
        self.phase
    }

    /// Stand der Energieleiste.
    #[must_use]
    pub fn energy(&self) -> u32 {
        self.energy
    }

    /// Wie oft die Energieleiste ausgeloest hat.
    #[must_use]
    pub fn energy_triggers(&self) -> u32 {
        self.energy_triggers
    }

    /// Wie weit die Boss-Zeitleiste abgearbeitet ist.
    #[must_use]
    pub fn timeline_index(&self) -> usize {
        self.timeline_index
    }

    /// Der Boss, gegen den gerechnet wird.
    #[must_use]
    pub fn boss(&self) -> &'static Boss {
        self.boss
    }

    /// Die Schul-Mechanik.
    #[must_use]
    pub fn resource_data(&self) -> &'static Resource {
        &RESOURCE
    }

    /// Das Kampfprotokoll.
    #[must_use]
    pub fn log(&self) -> &CombatLog {
        &self.log
    }

    /// Die Zufallsstroeme — schreibend, weil Ziehen den Zustand fortschreibt.
    pub fn rng(&mut self) -> &mut Streams {
        &mut self.rng
    }

    /// Ziehungen aus einem Strom.
    #[must_use]
    pub fn draws(&self, stream: Stream) -> u32 {
        self.rng.draws(stream)
    }
}
