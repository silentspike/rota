//! Der verdichtete Kampfzustand als Prüfpunkt, und der Suchkampf (Issue #917,
//! AC0 und AC5).
//!
//! # Zwei Zustände, nicht einer
//!
//! ```text
//! SearchFight   nur was das Ziel bestimmt: Summen, die aktiven Zauber,
//!               Ereignisse, Abklingzeiten, Verstärkungen, Ressource, HP,
//!               Bosszeiger                                  -> Millionen Kandidaten
//!
//! AuditFight    zusätzlich Schaden und Würfe je Zauber, verfallene Ticks,
//!               Leerlauf, volles Protokoll, Paritäts-Prüfsummen
//!                                                           -> Elites, Fehlerfälle,
//!                                                              Stichproben
//! ```
//!
//! Ohne die Trennung trüge jeder Suchkampf die Diagnosefelder mit, die nur die
//! Abnahme braucht — und jeder Prüfpunkt auch. Der [`CompactFightState`] ist der
//! Zustand des Suchkampfs: Er trägt die Diagnosefelder nicht, und die Felder je
//! Zauber nur für die Zauber, die in diesem Kampf überhaupt etwas tragen.
//!
//! # Warum die Einordnung der Übersetzer erzwingt
//!
//! Rust hat keine Reflexion; eine Sicherung, die ihre Felder in einer gepflegten
//! Liste führt, vergisst das nächste neue Feld — und ein vergessenes Feld wäre
//! hier ein Suffix-Lauf, der still von einem anderen Zustand ausgeht. Deshalb
//! steht jedes Feld von [`Fight`] genau einmal im Aufruf von `kampfzustand!`, in
//! einer von sieben Gruppen, und die Sicherung **zerlegt `Fight` vollständig**:
//! Ein Feld, das in keiner Gruppe steht, übersetzt nicht.
//!
//! | Gruppe | gesichert | warum |
//! | --- | --- | --- |
//! | `skalare` | ja, als Wert | der Suchzustand |
//! | `zauber` | ja, nur für aktive Zauber | die Felder je Zauber, verdichtet über [`ActiveSpells`] |
//! | `verstaerkungen` | ja | die acht Verstärkungs-Speicher |
//! | `werkzeuge` | ja, eigens | Schlange, Zufall, geteilte Zellen |
//! | `pruefzustand` | **nein** | das Zählwerk der Prüfung; kein Suchkampf führt es |
//! | `konstant` | **nein** | aus der Konfiguration, im Kampf unverändert; bildet die Kennung |
//! | `art` | **nein** | Such- oder Prüfkampf |
//!
//! Dass die Felder in `konstant` sich im Kampf nicht ändern, ist gemessen: Keine
//! Zuweisung `self.<feld> =` außerhalb von `Fight::new` (Suche über `fight.rs`
//! am 2026-10-29, null Treffer). Dass `pruefzustand` keine Rechnung trägt, prüft
//! `tests/such_und_pruefkampf.rs`: Such- und Prüfkampf liefern für dieselbe
//! Eingabe dieselbe Zustandsspur.

use super::{Decide, Fight, FightConfig, Nullzeit, NULLZEIT_BUDGET};
use crate::apl::Apl;
use crate::bitmenge::{ActiveSpells, InlineVec, LEISTE_INLINE};
use crate::buffs::Buffs;
use crate::cooldown_reset::CooldownReset;
use crate::data::SPELL_COUNT;
use crate::event::Schlangenstand;
use crate::factors::DamageFactors;
use crate::rng::Streams;
use crate::sublimation::PendingShorten;
use crate::time::Millis;

use super::DECAY_STACK_CAPACITY;

/// Baut aus der Einordnung aller Felder von [`Fight`] die Sicherung.
///
/// Erzeugt die Hilfstypen `Skalare` und `ZauberWerte` und je Gruppe die
/// Abschreib- und Rückschreib-Funktionen. Die vollständige Zerlegung in
/// `felder_eingeordnet` ist der Wächter aus dem Modulkopf.
macro_rules! kampfzustand {
    (
        skalare { $( $s:ident : $st:ty ),* $(,)? }
        zauber { $( $z:ident : $zt:ty = $zi:expr ),* $(,)? }
        verstaerkungen { $( $v:ident ),* $(,)? }
        werkzeuge { $( $w:ident ),* $(,)? }
        pruefzustand { $( $p:ident ),* $(,)? }
        konstant { $( $k:ident ),* $(,)? }
        art { $( $a:ident ),* $(,)? }
    ) => {
        /// Die einfachen Felder des Suchzustands, als Wert.
        #[derive(Debug, Clone, Copy)]
        struct Skalare {
            $( $s: $st, )*
        }

        /// Was je aktivem Zauber gesichert wird.
        #[derive(Debug, Clone, Copy, PartialEq)]
        struct ZauberWerte {
            $( $z: $zt, )*
        }

        impl Default for ZauberWerte {
            /// Der Stand eines Zaubers, der nichts trägt — derselbe wie in
            /// [`Fight::new`].
            fn default() -> Self {
                Self { $( $z: $zi, )* }
            }
        }

        /// Wie viele Verstärkungs-Speicher der Kampf führt.
        const VERSTAERKUNGEN: usize = [$( stringify!($v) ),*].len();

        impl Fight {
            /// **Der Wächter:** zerlegt `Fight` ohne `..`. Ein Feld, das in
            /// keiner Gruppe von `kampfzustand!` steht, übersetzt nicht.
            fn felder_eingeordnet(&self) {
                let Self {
                    $( $s: _, )*
                    $( $z: _, )*
                    $( $v: _, )*
                    $( $w: _, )*
                    $( $p: _, )*
                    $( $k: _, )*
                    $( $a: _, )*
                } = self;
            }

            fn skalare(&self) -> Skalare {
                Skalare { $( $s: self.$s, )* }
            }

            fn skalare_laden(&mut self, s: &Skalare) {
                $( self.$s = s.$s; )*
            }

            fn zauber_werte(&self, index: usize) -> ZauberWerte {
                ZauberWerte { $( $z: self.$z[index], )* }
            }

            /// Ob irgendein Feld dieses Zaubers vom Anfangsstand abweicht.
            fn zauber_traegt(&self, index: usize, anfang: &ZauberWerte) -> bool {
                [$( self.$z[index] != anfang.$z ),*].iter().any(|&weicht| weicht)
            }

            fn zauber_setzen(&mut self, index: usize, werte: &ZauberWerte) {
                $( self.$z[index] = werte.$z; )*
            }

            fn zauber_leeren(&mut self) {
                let anfang = ZauberWerte::default();
                $( self.$z = [anfang.$z; SPELL_COUNT]; )*
            }

            fn verstaerkungen(&self) -> [Buffs; VERSTAERKUNGEN] {
                [$( self.$v.clone(), )*]
            }

            fn verstaerkungen_sichern(&self, ziel: &mut [Buffs; VERSTAERKUNGEN]) {
                let [$( $v, )*] = ziel;
                $( $v.clone_from(&self.$v); )*
            }

            fn verstaerkungen_laden(&mut self, gesichert: &[Buffs; VERSTAERKUNGEN]) {
                let [$( $v, )*] = gesichert;
                $( self.$v.clone_from($v); )*
            }

            /// Eine Kennung der Konfiguration, aus allen Feldern der Gruppe
            /// `konstant` — ein Prüfpunkt darf nur in einen Kampf mit
            /// derselben geladen werden.
            ///
            /// Einmal beim Aufbau gerechnet ([`Fight::new`]) und im Feld
            /// `kennung` abgelegt: Die Felder ändern sich im Kampf nicht, und
            /// eine Kennung je Sicherung kostete mehr als die Sicherung selbst.
            pub(super) fn konstant_kennung(&self) -> u64 {
                let mut kennung = FNV_OFFSET_64;
                $( kennung = self.$k.einruehren(kennung); )*
                kennung
            }
        }
    };
}

kampfzustand! {
    skalare {
        time: Millis,
        damage: u32,
        player_damage: u32,
        killed_at: Option<Millis>,
        phase: usize,
        energy: u32,
        energy_triggers: u32,
        doom: u32,
        doom_charges: u32,
        next_doom_charge_at: Option<Millis>,
        stacks: u32,
        race_ends_at: Option<Millis>,
        race_done: bool,
        race_progress: u32,
        race_won: bool,
        damage_lost_to_race: u32,
        boss_healed: u32,
        stabilised: u32,
        afflicted: bool,
        afflictions_cleared: u32,
        timeline_index: usize,
        casting: Option<usize>,
        player_casting: Option<usize>,
        player_cast_ends_at: Millis,
        player_cast_generation: u16,
        casts_interrupted: u32,
        casts_cancelled: u32,
        interrupts_missed: u32,
        interrupts_landed: u32,
        player_hp: u32,
        died_at: Option<Millis>,
        damage_taken: u32,
        shield: u32,
        shield_expires_at: Option<Millis>,
        shield_stacks: u32,
        shield_absorbed: u32,
        healed: u32,
        heal_over_time_amount: u32,
        heal_over_time_steps: u32,
        heal_over_time_interval_ms: Millis,
        heal_over_time_done: u32,
        heal_over_time_credited: u32,
        heal_over_time_generation: u16,
        conversion_until: Option<Millis>,
        decay_ward_until: Option<Millis>,
        decay_ward_permille: u32,
        conversion_percent: u32,
        converted: u32,
        reflect_until: Option<Millis>,
        reflect_percent: u32,
        reflected: u32,
        reflect_spell: usize,
        control: u32,
        pending_control_damage: u32,
        decay_expiry: [Millis; DECAY_STACK_CAPACITY],
        decay_cap_bonus: usize,
        decay_cap_until: Option<Millis>,
        decay_count: usize,
        synergy_used: u32,
        synergy_available: u32,
        control_spent: u32,
        next_strike_delay: Millis,
        pin_window: Millis,
        phase_hold_percent: u32,
        phase_hold_until: Option<Millis>,
        cast_lands_at: Millis,
        control_failed: u32,
        gcd_ready_at: Millis,
        next_cast_free_gcd: bool,
        next_cast_doubles: bool,
        retort_armed: bool,
        reset_mode: CooldownReset,
        resource: u16,
        resource_overflow: u32,
        factors: DamageFactors,
        shield_last_amount: u32,
        shield_refund_used: bool,
        free_control_spend_armed: bool,
        free_control_spends: u32,
        nullzeit: Nullzeit,
        entscheidung_gcd_bereit: bool,
        ereignisse: u32,
    }
    zauber {
        synergy_window_ends: Millis = 0,
        cooldown_ready_at: Millis = 0,
        dot_expires_at: Option<Millis> = None,
        dot_generation: u16 = 0,
        decay_rest: u32 = 0,
        dot_tick_counts: u32 = 0,
        decay_echo_until: Millis = 0,
    }
    verstaerkungen {
        buffs,
        control_buffs,
        resource_buffs,
        tick_buffs,
        stack_buffs,
        cd_reduce_buffs,
        convert_buffs,
        cap_buffs,
    }
    werkzeuge {
        queue,
        rng,
        pending_gain,
        stack_gains,
        pending_shorten,
        sublimation_uses,
        // Die Empfänger sind beim Aufbau angemeldet und ändern sich nicht; die
        // Folgen-Prüfsumme darin ist eine Paritäts-Prüfsumme und gehört dem
        // Prüfkampf. Gesichert wird hier deshalb nichts davon.
        events,
        // Die Kennung der Konfiguration, beim Aufbau gerechnet.
        kennung,
    }
    pruefzustand {
        casts,
        damage_by_spell,
        lost_dot_ticks,
        idle_ms,
        log,
    }
    konstant {
        duration_ms,
        stop_on_kill,
        boss,
        resistance_by_spell,
        vessel_max,
        talent_resource_factor,
        talent_dot_duration_factor,
        talent_control_gain_factor,
        talent_decay_effect_factor,
        talent_control_cost_factor,
        talent_control_effect_factor,
        talent_decay_stack_permille,
        talents_base,
        talent_permille,
        talent_spender_damage_permille,
        talent_build_damage_permille,
        talent_decay_cap_bonus,
        talent_decay_ward_permille,
        talent_convert_permille,
        talent_decay_duration_factor,
        talent_shield_amount_factor,
        talent_shield_duration_factor,
        talent_shield_stacks_bonus,
        talent_cooldown_reduction_factor,
        talent_cooldown_charges_bonus,
        talent_cooldown_threshold_permille,
        worn,
        split_spender,
        auto_reset,
        partial_control,
        shield_refund,
        decay_echo,
        ashen_regimen,
        whole_work,
        slow_rot,
        sealed_vessel,
        long_grip,
        perpetual_draft,
    }
    art {
        // Such- oder Prüfkampf. Nicht gesichert und nicht in der Kennung:
        // Ein Prüfpunkt ist derselbe, gleich in welcher Art Kampf er entstand.
        fuehrt_pruefung,
    }
}

/// FNV-1a über 64 Bit — für die Kennung und die Zustandsspur.
const FNV_OFFSET_64: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Wie ein unveränderliches Feld in die Kennung der Konfiguration eingeht.
trait Kennwert {
    fn einruehren(&self, h: u64) -> u64;
}

impl Kennwert for u32 {
    fn einruehren(&self, h: u64) -> u64 {
        fnv(h, &self.to_le_bytes())
    }
}

impl Kennwert for usize {
    fn einruehren(&self, h: u64) -> u64 {
        fnv(h, &(*self as u64).to_le_bytes())
    }
}

impl Kennwert for bool {
    fn einruehren(&self, h: u64) -> u64 {
        fnv(h, &[u8::from(*self)])
    }
}

impl Kennwert for f64 {
    fn einruehren(&self, h: u64) -> u64 {
        fnv(h, &self.to_bits().to_le_bytes())
    }
}

impl<const N: usize> Kennwert for [f64; N] {
    fn einruehren(&self, h: u64) -> u64 {
        self.iter().fold(h, |h, wert| wert.einruehren(h))
    }
}

impl Kennwert for &'static crate::data::Boss {
    fn einruehren(&self, h: u64) -> u64 {
        fnv(h, self.id.as_bytes())
    }
}

impl Kennwert for crate::gear::WornValues {
    // Feld für Feld und vollständig zerlegt: Ein neues Feld übersetzt hier
    // nicht, bis es eingeht. Nicht über `Debug` — das forderte beim Aufbau
    // Speicher an, und `tests/allocation.rs` zählt jede Anforderung.
    fn einruehren(&self, h: u64) -> u64 {
        let Self {
            catalysis,
            purity,
            yield_bonus,
            essence,
            vessel,
            ward,
            gear_effects,
        } = *self;
        [
            catalysis,
            purity,
            yield_bonus,
            essence,
            vessel,
            ward,
            gear_effects,
        ]
        .iter()
        .fold(h, |h, wert| wert.einruehren(h))
    }
}

/// Die Werte der geteilten Zellen zwischen Kampf und Schul-Mechanik.
///
/// Die Zellen selbst teilt der Kampf mit seinen Empfängern; gesichert wird ihr
/// Inhalt, nicht der Zeiger — sonst schriebe ein geladener Kampf in die Zellen
/// dessen, aus dem gesichert wurde.
#[derive(Debug, Clone, Copy)]
struct Zellen {
    pending_gain: u16,
    stack_gains: u32,
    pending_shorten: PendingShorten,
    sublimation_uses: u32,
}

/// Der verdichtete Zustand eines Suchkampfs — der Inhalt eines Prüfpunkts
/// (Issue #917, AC0 und AC5).
///
/// Er trägt, was das Ziel bestimmt, und nichts vom Zählwerk der Prüfung. Die
/// Felder je Zauber stehen nur für die Zauber, die etwas tragen
/// ([`CompactFightState::aktive_zauber`]); alle übrigen stehen beim Laden auf
/// ihrem Anfangsstand.
///
/// Geladen wird er über [`SearchFight::fortsetzen`], und zwar nur in einen Kampf
/// mit derselben Konfiguration — die Kennung darin wird beim Laden verglichen.
#[derive(Debug, Clone)]
pub struct CompactFightState {
    kennung: u64,
    skalare: Skalare,
    aktive: ActiveSpells,
    zauber: InlineVec<ZauberWerte, LEISTE_INLINE>,
    verstaerkungen: [Buffs; VERSTAERKUNGEN],
    schlange: Schlangenstand,
    rng: Streams,
    zellen: Zellen,
}

impl CompactFightState {
    /// Die Kampfzeit dieses Zustands.
    #[must_use]
    pub const fn time(&self) -> Millis {
        self.skalare.time
    }

    /// Der bis hierher angerichtete Schaden.
    #[must_use]
    pub const fn damage(&self) -> u32 {
        self.skalare.damage
    }

    /// Der bis hierher eingesteckte Schaden.
    #[must_use]
    pub const fn damage_taken(&self) -> u32 {
        self.skalare.damage_taken
    }

    /// Wie viele Ereignisse bis hierher abgearbeitet waren.
    #[must_use]
    pub const fn ereignisse(&self) -> u32 {
        self.skalare.ereignisse
    }

    /// Die Zauber, deren Felder dieser Zustand trägt.
    #[must_use]
    pub const fn aktive_zauber(&self) -> &ActiveSpells {
        &self.aktive
    }

    /// Wie viele Ereignisse offen waren.
    #[must_use]
    pub fn offene_ereignisse(&self) -> usize {
        self.schlange.len()
    }

    /// Eine Prüfsumme über den **ganzen** Zustand — die Zustandsspur aus AC5.
    ///
    /// Gerechnet über die `Debug`-Darstellung, also über jedes Feld, das der
    /// Zustand trägt, auch über das von morgen. Langsam (eine Zeichenkette je
    /// Aufruf) und deshalb für Prüfungen, nicht für die Suche.
    #[must_use]
    pub fn spur_hash(&self) -> u64 {
        fnv(FNV_OFFSET_64, format!("{self:?}").as_bytes())
    }
}

impl Fight {
    /// Der Suchzustand dieses Kampfes, verdichtet auf die aktiven Zauber
    /// (#917).
    #[must_use]
    pub fn such_zustand(&self) -> CompactFightState {
        let mut zustand = CompactFightState {
            kennung: self.kennung,
            skalare: self.skalare(),
            aktive: ActiveSpells::new(),
            zauber: InlineVec::new(),
            verstaerkungen: self.verstaerkungen(),
            schlange: Schlangenstand::default(),
            rng: self.rng.clone(),
            zellen: self.zellen(),
        };
        self.sichern(&mut zustand);
        zustand
    }

    /// Sichert den Suchzustand in `ziel` und behält dessen Speicher.
    pub fn sichern(&self, ziel: &mut CompactFightState) {
        self.felder_eingeordnet();
        ziel.kennung = self.kennung;
        ziel.skalare = self.skalare();
        ziel.aktive.clear();
        ziel.zauber.clear();
        let anfang = ZauberWerte::default();
        for index in 0..SPELL_COUNT {
            if self.zauber_traegt(index, &anfang) {
                ziel.aktive.push(index);
                ziel.zauber.push(self.zauber_werte(index));
            }
        }
        self.verstaerkungen_sichern(&mut ziel.verstaerkungen);
        self.queue.sichern(&mut ziel.schlange);
        ziel.rng.clone_from(&self.rng);
        ziel.zellen = self.zellen();
    }

    fn zellen(&self) -> Zellen {
        Zellen {
            pending_gain: self.pending_gain.get(),
            stack_gains: self.stack_gains.get(),
            pending_shorten: self.pending_shorten.get(),
            sublimation_uses: self.sublimation_uses.get(),
        }
    }

    /// Lädt einen gesicherten Suchzustand.
    ///
    /// Das Zählwerk der Prüfung bleibt, wie es ist — in einem Suchkampf steht
    /// es auf null, weil er es nie fortschreibt.
    ///
    /// # Panics
    ///
    /// Wenn der Zustand aus einem Kampf mit anderer Konfiguration stammt.
    fn laden(&mut self, stand: &CompactFightState) {
        assert_eq!(
            stand.kennung, self.kennung,
            "Prüfpunkt aus einem Kampf mit anderer Konfiguration"
        );
        self.skalare_laden(&stand.skalare);
        self.zauber_leeren();
        for (lokal, global) in stand.aktive.iter().enumerate() {
            let werte = stand
                .zauber
                .get(lokal)
                .expect("je aktivem Zauber ein Eintrag");
            self.zauber_setzen(global, &werte);
        }
        self.verstaerkungen_laden(&stand.verstaerkungen);
        self.queue.laden(&stand.schlange);
        self.rng.clone_from(&stand.rng);
        self.pending_gain.set(stand.zellen.pending_gain);
        self.stack_gains.set(stand.zellen.stack_gains);
        self.pending_shorten.set(stand.zellen.pending_shorten);
        self.sublimation_uses.set(stand.zellen.sublimation_uses);
    }

    /// Was das Ziel dieses Kampfes bestimmt — gültig für Such- und Prüfkampf.
    #[must_use]
    pub fn zielwerte(&self) -> Zielwerte {
        Zielwerte {
            damage: self.damage,
            killed_at_ms: self.killed_at,
            remaining_ms: self.killed_at.map_or(0, |t| self.duration_ms - t),
            damage_taken: self.damage_taken,
            absorbed: self.shield_absorbed,
            healed: self.healed,
            boss_healed: self.boss_healed,
            died_at_ms: self.died_at,
            random_draws: self.rng.total_draws(),
        }
    }
}

/// Was das Ziel eines Kampfes bestimmt (#917, AC5).
///
/// Das Ergebnis eines [`SearchFight`]. Es trägt bewusst nichts vom Zählwerk
/// der Prüfung: Ein Suchkampf führt `casts` und `damage_by_spell` nicht, und
/// ein Ergebnis, das sie trotzdem trüge, meldete Nullen, die wie Messwerte
/// aussehen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zielwerte {
    /// Gesamtschaden.
    pub damage: u32,
    /// Kampfzeit des Sieges, oder `None`.
    pub killed_at_ms: Option<Millis>,
    /// Verbleibende Zeit bis zum Überdruck, wenn der Boss gefallen ist.
    pub remaining_ms: Millis,
    /// Eingesteckter Schaden, nach der Schutzschicht.
    pub damage_taken: u32,
    /// Von der Schutzschicht abgefangen.
    pub absorbed: u32,
    /// Zurückgeholtes Leben.
    pub healed: u32,
    /// Vom Boss zurückgedrehter Fortschritt.
    pub boss_healed: u32,
    /// Kampfzeit des Spielertods, oder `None`.
    pub died_at_ms: Option<Millis>,
    /// Ziehungen aus den Zufallsströmen.
    pub random_draws: u32,
}

/// Ein Suchkampf (#917, AC5): rechnet, was das Ziel bestimmt, und sonst nichts.
///
/// Derselbe Kampf wie [`Fight`] — **dieselbe** Regelrechnung, keine zweite —,
/// nur ohne Protokoll und ohne das Zählwerk der Prüfung. Sein Ergebnis sind
/// [`Zielwerte`], nicht [`super::RunResult`]: Was er nicht führt, kann er nicht
/// melden.
///
/// Er ist der Kampf, in den die inkrementelle Auswertung einen Prüfpunkt lädt
/// ([`SearchFight::fortsetzen`]).
#[derive(Debug)]
pub struct SearchFight {
    kampf: Fight,
}

impl SearchFight {
    /// Legt einen Suchkampf an. Das Protokoll ist abgeschaltet, gleich was
    /// `config` sagt.
    #[must_use]
    pub fn new(config: FightConfig) -> Self {
        let mut kampf = Fight::new(FightConfig {
            log_capacity: 0,
            ..config
        });
        kampf.fuehrt_pruefung = false;
        Self { kampf }
    }

    /// Setzt den Kampf für einen neuen Durchgang zurück.
    pub fn reset(&mut self, seed: u32) {
        self.kampf.reset(seed);
    }

    /// Rechnet den Kampf mit einer Prioritätenliste durch.
    pub fn run_apl(&mut self, apl: &Apl) -> Zielwerte {
        self.run_mit(apl)
    }

    /// Rechnet den Kampf mit einer beliebigen Entscheidung durch — etwa einer,
    /// die nebenher Prüfpunkte aufzeichnet.
    pub fn run_mit<D: Decide + ?Sized>(&mut self, decider: &D) -> Zielwerte {
        self.kampf.run_with(decider, NULLZEIT_BUDGET);
        self.kampf.zielwerte()
    }

    /// Lädt einen Prüfpunkt und rechnet von dort an zu Ende — der Suffix-Lauf.
    ///
    /// Der Prüfpunkt liegt vor einer Entscheidung; die Schleife beginnt dort und
    /// läuft danach wie jede andere (`Fight::kampfschleife`).
    ///
    /// # Panics
    ///
    /// Wenn der Prüfpunkt aus einem Kampf mit anderer Konfiguration stammt.
    pub fn fortsetzen<D: Decide + ?Sized>(
        &mut self,
        stand: &CompactFightState,
        decider: &D,
    ) -> Zielwerte {
        self.kampf.laden(stand);
        self.kampf.kampfschleife(decider, true);
        self.kampf.zielwerte()
    }

    /// Der Kampf selbst, zum Lesen.
    #[must_use]
    pub const fn kampf(&self) -> &Fight {
        &self.kampf
    }
}
