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

use crate::data::{Boss, Resource, Spell, BOSS, RESOURCE, SPELLS, SPELL_COUNT};
use crate::event::{EventKind, EventQueue};
use crate::log::{CombatLog, LogKind, NO_SPELL};
use crate::rng::{Stream, Streams};
use crate::time::{snap_up, Millis, GCD_MS, TICK_MS};

/// Eine Rotation: Welcher Zauber soll jetzt gewirkt werden?
///
/// `None` heisst bewusst warten. Die Prioritaetenliste als Sprache kommt in
/// E2-3; bis dahin ist eine Rotation eine gewoehnliche Funktion.
pub type Rotation = fn(&Fight) -> Option<usize>;

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
    pub seed: u64,
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
}

impl Default for FightConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            duration_ms: None,
            stop_on_kill: true,
            log_capacity: 4096,
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
    damage: u32,
    killed_at: Option<Millis>,

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
    resource_overflow: u32,
    idle_ms: Millis,

    // -- Werkzeuge --
    queue: EventQueue,
    log: CombatLog,
    rng: Streams,
}

impl Fight {
    /// Legt einen Durchgang an. Fordert allen benoetigten Speicher **jetzt** an.
    #[must_use]
    pub fn new(config: FightConfig) -> Self {
        Self {
            time: 0,
            duration_ms: config.duration_ms.unwrap_or(BOSS.overpressure_ms),
            stop_on_kill: config.stop_on_kill,
            damage: 0,
            killed_at: None,
            gcd_ready_at: 0,
            cooldown_ready_at: [0; SPELL_COUNT],
            resource: 0,
            dot_expires_at: [None; SPELL_COUNT],
            dot_generation: [0; SPELL_COUNT],
            casts: [0; SPELL_COUNT],
            damage_by_spell: [0; SPELL_COUNT],
            lost_dot_ticks: [0; SPELL_COUNT],
            resource_overflow: 0,
            idle_ms: 0,
            queue: EventQueue::with_capacity(EVENT_CAPACITY),
            log: CombatLog::with_capacity(config.log_capacity),
            rng: Streams::new(config.seed),
        }
    }

    // ---------------------------------------------------------------- Ablauf

    /// Rechnet den Kampf mit der angegebenen Rotation durch.
    ///
    /// Ein bereits gelaufener Kampf muss vorher mit [`Fight::reset`]
    /// zurueckgesetzt werden.
    pub fn run(&mut self, rotation: Rotation) -> RunResult {
        // Der Verfall laeuft ab Kampfbeginn, unabhaengig davon, ob ueberhaupt
        // Ressource da ist — so macht es die Referenz, und das ist auch richtig:
        // Der Takt haengt am Kampf, nicht am Fuellstand.
        if RESOURCE.decay_per_step_ms > 0 {
            self.queue
                .push(RESOURCE.decay_per_step_ms, EventKind::ResourceDecay);
        }
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
            if self.killed_at.is_none() && self.damage >= BOSS.hp {
                self.killed_at = Some(now);
                self.log.push(now, LogKind::Kill, NO_SPELL, self.damage);
                if self.stop_on_kill {
                    break;
                }
            }

            // Phase 3 — die Entscheidung des Spielers.
            if self.gcd_ready_at <= now {
                match rotation(self) {
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
        }

        self.result()
    }

    /// Setzt den Kampf zurueck, **ohne** Speicher freizugeben oder neu
    /// anzufordern. So kostet ein zweiter Durchgang nichts.
    pub fn reset(&mut self, seed: u64) {
        self.time = 0;
        self.damage = 0;
        self.killed_at = None;
        self.gcd_ready_at = 0;
        self.cooldown_ready_at = [0; SPELL_COUNT];
        self.resource = 0;
        self.dot_expires_at = [None; SPELL_COUNT];
        self.dot_generation = [0; SPELL_COUNT];
        self.casts = [0; SPELL_COUNT];
        self.damage_by_spell = [0; SPELL_COUNT];
        self.lost_dot_ticks = [0; SPELL_COUNT];
        self.resource_overflow = 0;
        self.idle_ms = 0;
        self.queue.clear();
        self.log.clear();
        self.rng = Streams::new(seed);
    }

    fn apply(&mut self, kind: EventKind) {
        match kind {
            EventKind::DotTick { spell, generation } => self.dot_tick(spell as usize, generation),
            EventKind::ResourceDecay => self.decay(),
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

        self.deal(index, dot.tick_damage, LogKind::DotTick);
        self.add_resource(dot.resource_gain_per_tick);

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
        self.log.push(now, LogKind::Cast, index as u8, 0);

        if spell.base_damage > 0 {
            self.deal(index, spell.base_damage, LogKind::Damage);
        }

        if let Some(spender) = spell.spender {
            let spent = self.resource;
            let mut damage = spender.damage_per_point * u32::from(spent);
            if spent >= spender.bonus_threshold {
                damage *= spender.bonus_multiplier;
            }
            self.log
                .push(now, LogKind::Spend, index as u8, u32::from(spent));
            self.deal(index, damage, LogKind::Damage);
            self.resource = 0;
        }

        if spell.resource_gain > 0 {
            self.add_resource(spell.resource_gain);
        }

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
        if self.damage >= BOSS.hp {
            self.queue.push(now + TICK_MS, EventKind::Wakeup);
        }
    }

    fn deal(&mut self, index: usize, amount: u32, kind: LogKind) {
        self.damage += amount;
        self.damage_by_spell[index] += amount;
        self.log.push(self.time, kind, index as u8, amount);
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

    /// Der Boss, gegen den gerechnet wird.
    #[must_use]
    pub fn boss(&self) -> &'static Boss {
        &BOSS
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
