//! Die typisierte Wirkungsbeschreibung — was ein Effekt **tut**, nicht wie er heisst.
//!
//! # Wofuer es das gibt (Issue #911)
//!
//! An acht Stellen wird entschieden, was eine Zusammenstellung von Zaubern tut:
//! ob der kritische Treffer analytisch statt gewuerfelt behandelt werden darf,
//! welche Obergrenze fuer die Zahl moeglicher Aktionen gilt, welcher
//! spezialisierte Kernel passt, ob ein zwischengespeicherter Rechenweg
//! wiederverwendet werden darf, welche Ergebnisse ungueltig werden, welche
//! Ereigniskanaele der Kampf braucht, welche Zufallsquellen aktiv sind, und ob
//! eine Wirkung monoton ist.
//!
//! Werden diese Fragen aus **gepflegten Listen** beantwortet, driften die
//! Listen — und der Fehler ist still: Eine Pruefung gibt weiter „nein" zurueck,
//! ein Rechenweg wird faelschlich wiederverwendet, die Invalidierung uebersieht
//! Ergebnisse. Alles sieht plausibel aus.
//!
//! Dieser Fehler ist in der Planung zweimal vorgefuehrt worden: „sieben
//! Zusatz-Systeme in den Bossdaten" — nachgezaehlt waren es **elf**; „der
//! Abklingzeit-Ruecksetzer kommt erst noch" — tatsaechlich tragen **neun von 58
//! Zaubern** bereits solche Effekte. Beide Male stand die falsche Aussage
//! monatelang unbemerkt.
//!
//! # Was hier zugesichert wird — und was nicht
//!
//! **Nicht** zugesichert ist, dass die Beschreibung vollstaendig ist.
//! Zugesichert ist, dass eine Luecke den **Bau bricht** statt still
//! durchzulaufen: [`EffectOp`] wird in jeder registrierten Analyse mit einem
//! vollstaendigen `match` **ohne Auffangzweig** behandelt. Ein neuer Baustein
//! laesst deshalb jede Analyse nicht uebersetzen, bis er ueberall eingeordnet
//! ist.
//!
//! Ebenfalls **nicht** zugesichert ist Geschwindigkeit. Dieses Modul macht
//! andere Optimierungen korrekt, nicht schnell.
//!
//! # Der konservative Fall
//!
//! Fehlt eine Angabe, gilt: alle Abhaengigkeiten annehmen, kein Ledger, keine
//! analytische Marginalisierung, kein Dominanz-Pruning, keine enge Schranke,
//! allgemeiner Rueckfall-Kernel. **Unbekannt muss langsam sein, nie falsch.**
//! Siehe [`ConservativeFallback`].

pub mod passes;
pub mod registry;

use core::fmt;

/// Die Effekt-Bausteine.
///
/// # Diese Aufzaehlung ist die Zusicherung
///
/// Sie ist **abschliessend** und wird in jeder registrierten Analyse mit einem
/// `match` ohne Auffangzweig behandelt. Genau daran haengt die Aussage des
/// ganzen Moduls: Wer hier eine Variante ergaenzt, bekommt so lange keinen
/// uebersetzbaren Simulator, bis er sie in **jeder** Analyse eingeordnet hat.
///
/// **Kein Auffangzweig, auch nicht vorlaeufig.** Genau er hebelt die Zusicherung
/// aus, und er sieht in jedem Test von heute richtig aus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectOp {
    /// Schaden am Ziel.
    DealDamage,
    /// Einen Brand-Effekt auflegen.
    ApplyDot,
    /// Eine Verstaerkung auflegen.
    ApplyBuff,
    /// Die Ressource veraendern — Gewinn wie Verbrauch.
    ModifyResource,
    /// Laufende Abklingzeiten verkuerzen.
    ReduceCooldown,
    /// Eine Abklingzeit auf null setzen.
    ResetCooldown,
    /// Den naechsten Wurf von der globalen Wartezeit befreien.
    FreeNextGcd,
    /// Lebenspunkte wiederherstellen.
    Heal,
    /// Eine Schutzschicht aufbauen.
    Shield,
    /// Schaden in etwas anderes umwandeln.
    ConvertDamage,
    /// Einen Bosszustand aendern, der keine eigene Variante hat.
    ChangeBossState,
    /// Die Bossphase wechseln.
    ChangePhase,
    /// Eine einmalige Bossregel.
    ///
    /// # Warum das kein Schlupfloch sein darf
    ///
    /// Wenn jede ungewoehnliche Mechanik hier durchlaeuft, bricht der Bau
    /// **nie** mehr — und die 68 geplanten Bosse tragen per Design je eine
    /// einmalige Regel. Deshalb muss auch eine Signatur ihre Semantik
    /// deklarieren; tut sie es nicht, greift [`ConservativeFallback`].
    /// Gepruefte Stelle: [`EffectNode::signature_ohne_semantik`].
    CustomSignature(SignatureId),
}

impl EffectOp {
    /// Ein sprechender Name fuer Meldungen.
    ///
    /// Bewusst **kein** `match` mit Auffangzweig: auch diese Abbildung ist
    /// vollstaendig, damit ein neuer Baustein auch hier auffaellt.
    pub fn name(self) -> &'static str {
        match self {
            EffectOp::DealDamage => "DealDamage",
            EffectOp::ApplyDot => "ApplyDot",
            EffectOp::ApplyBuff => "ApplyBuff",
            EffectOp::ModifyResource => "ModifyResource",
            EffectOp::ReduceCooldown => "ReduceCooldown",
            EffectOp::ResetCooldown => "ResetCooldown",
            EffectOp::FreeNextGcd => "FreeNextGcd",
            EffectOp::Heal => "Heal",
            EffectOp::Shield => "Shield",
            EffectOp::ConvertDamage => "ConvertDamage",
            EffectOp::ChangeBossState => "ChangeBossState",
            EffectOp::ChangePhase => "ChangePhase",
            EffectOp::CustomSignature(id) => id.0,
        }
    }
}

/// Der Name einer einmaligen Bossregel.
///
/// Traegt eine Zeichenkette statt einer Zahl, weil die Regeln aus den
/// Bossdateien stammen und in Fehlermeldungen wiedererkennbar sein muessen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignatureId(pub &'static str);

impl fmt::Display for SignatureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

// ------------------------------------------------------------------ Zustaende

/// Welche Zustandsbereiche ein Effekt beruehrt.
///
/// Eine Bitmaske und keine Liste: Die Frage „ueberschneiden sich zwei Effekte?"
/// ist damit ein `&`, und die Antwort haengt nicht an einer Reihenfolge.
///
/// **`UNKNOWN` ist nicht leer, sondern alles.** Wer nichts angibt, muss alle
/// Abhaengigkeiten annehmen — sonst waere die fehlende Angabe schneller als die
/// vorhandene, und das ist genau der Fehler, den dieses Modul verhindert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StateMask(pub u32);

impl StateMask {
    /// Nichts.
    pub const EMPTY: StateMask = StateMask(0);
    /// Die Ressource des Spielers.
    pub const RESOURCE: StateMask = StateMask(1 << 0);
    /// Abklingzeiten der Zauber.
    pub const COOLDOWNS: StateMask = StateMask(1 << 1);
    /// Die globale Wartezeit.
    pub const GCD: StateMask = StateMask(1 << 2);
    /// Laufende Verstaerkungen.
    pub const BUFFS: StateMask = StateMask(1 << 3);
    /// Laufende Brand-Effekte.
    pub const DOTS: StateMask = StateMask(1 << 4);
    /// Lebenspunkte des Bosses.
    pub const BOSS_HP: StateMask = StateMask(1 << 5);
    /// Lebenspunkte des Spielers.
    pub const PLAYER_HP: StateMask = StateMask(1 << 6);
    /// Die Schutzschicht.
    pub const SHIELD: StateMask = StateMask(1 << 7);
    /// Zersetzungs-Stapel.
    pub const DECAY_STACKS: StateMask = StateMask(1 << 8);
    /// Die Bossphase.
    pub const PHASE: StateMask = StateMask(1 << 9);
    /// Bosseigene Zusatz-Systeme (Energie, Verhaengnis, Eskalation und die
    /// uebrigen aus `data/bosses/*.json`).
    pub const BOSS_SYSTEMS: StateMask = StateMask(1 << 10);
    /// Die Zeitleiste des Bosses.
    pub const TIMELINE: StateMask = StateMask(1 << 11);
    /// Der Zustand der Zufallsstroeme.
    pub const RNG: StateMask = StateMask(1 << 12);

    /// Alles — der konservative Wert bei fehlender Angabe.
    pub const UNKNOWN: StateMask = StateMask(u32::MAX);

    /// Vereinigung zweier Masken.
    pub const fn union(self, other: StateMask) -> StateMask {
        StateMask(self.0 | other.0)
    }

    /// Ob sich zwei Masken beruehren.
    pub const fn intersects(self, other: StateMask) -> bool {
        (self.0 & other.0) != 0
    }

    /// Ob die Maske leer ist.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Ob die Maske der konservative Rueckfall ist.
    pub const fn is_unknown(self) -> bool {
        self.0 == u32::MAX
    }
}

// -------------------------------------------------------------- Zeitverhalten

/// Wann ein Effekt seine Eingangswerte liest.
///
/// Der Unterschied entscheidet, ob ein zwischengespeicherter Rechenweg
/// wiederverwendet werden darf: Eine Momentaufnahme haengt nur am Zeitpunkt des
/// Auflegens, ein lebender Wert an allem, was danach passiert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingSemantics {
    /// Sofort und abgeschlossen.
    Immediate,
    /// Beim Auflegen festgehalten und danach unveraenderlich.
    Snapshot,
    /// Bei jedem Tick neu gelesen.
    Live,
    /// Nicht angegeben — konservativ wie [`TimingSemantics::Live`] behandeln.
    Unknown,
}

/// Wie sich mehrfaches Auflegen verhaelt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackingSemantics {
    /// Kann nicht mehrfach liegen.
    None,
    /// Setzt die Restlaufzeit neu.
    Refresh,
    /// Verlaengert die Restlaufzeit.
    Extend,
    /// Stapelt ohne Grenze.
    Additive,
    /// Stapelt bis zu einer Grenze.
    Capped(u32),
    /// Ersetzt den vorherigen Zustand.
    Replace,
    /// Nicht angegeben.
    Unknown,
}

/// Welche Zufallsquelle ein Effekt anfasst.
///
/// Die Frage „welche Zufallsquellen sind ueberhaupt aktiv?" wird hieraus
/// beantwortet — und nicht aus einer gepflegten Liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomnessSemantics {
    /// Kein Zufall beteiligt.
    Deterministic,
    /// Zieht einen Krit-Wurf.
    CritRoll,
    /// Zieht aus einem anderen benannten Strom.
    Stream(&'static str),
    /// Nicht angegeben.
    Unknown,
}

impl RandomnessSemantics {
    /// Ob **sicher** kein Zufall im Spiel ist.
    ///
    /// [`RandomnessSemantics::Unknown`] antwortet hier `false`: Unbekannt ist
    /// nicht deterministisch.
    pub fn ist_deterministisch(self) -> bool {
        matches!(self, RandomnessSemantics::Deterministic)
    }
}

// ---------------------------------------------------------------- Monotonie

/// In welche Richtung eine Wirkung wirkt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotonicDirection {
    /// Mehr Wert ergibt ein besseres Ergebnis.
    Increasing,
    /// Mehr Wert ergibt ein schlechteres Ergebnis.
    Decreasing,
    /// Nachweislich nicht monoton.
    NonMonotonic,
    /// Nicht angegeben.
    Unknown,
}

/// Worauf sich ein Monotonie-Hinweis bezieht.
///
/// **Ohne diesen Bezug ist der Hinweis wertlos.** Grundschaden ist monoton
/// bezueglich erwartetem Schaden an einem gewoehnlichen Boss bei stabilem
/// Trace — und **nicht** bezueglich Sieg an einem Boss, bei dem ein Ring auf
/// null die Niederlage ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotonicityDomain {
    /// Erwarteter Schaden, gewoehnlicher Boss, stabiler Trace.
    ExpectedDamageStableTrace,
    /// Sieg oder Niederlage als Zielgroesse.
    VictoryOutcome,
    /// Kein Bezug angegeben.
    Unspecified,
}

/// Ein Monotonie-**Hinweis** — kein Zertifikat.
///
/// # Warum ein einzelnes Feld zu vertrauensselig waere
///
/// Ein `bool` am Knoten wuerde behaupten, die Wirkung sei „monoton", ohne zu
/// sagen wobei. Das Zertifikat entsteht erst aus Wirkungsbeschreibung,
/// Bosssemantik, Zielfunktion und Trace-Region **zusammen** — hier entsteht nur
/// der Hinweis, aus dem es spaeter gebaut werden kann.
///
/// [`LocalMonotonicityHint::traegt_fuer`] ist die Stelle, an der das durchgesetzt
/// wird: Fuer [`MonotonicityDomain::VictoryOutcome`] traegt ein Hinweis, der
/// fuer den stabilen Trace gilt, ausdruecklich **nicht**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalMonotonicityHint {
    /// Die vermutete Richtung.
    pub direction: MonotonicDirection,
    /// Der Bezug, ohne den die Richtung nichts aussagt.
    pub domain: MonotonicityDomain,
}

impl LocalMonotonicityHint {
    /// Der konservative Wert: keine Aussage.
    pub const UNKNOWN: LocalMonotonicityHint = LocalMonotonicityHint {
        direction: MonotonicDirection::Unknown,
        domain: MonotonicityDomain::Unspecified,
    };

    /// Ob dieser Hinweis fuer die gefragte Zielgroesse ueberhaupt etwas sagt.
    ///
    /// Er traegt **nur** fuer genau den Bezug, fuer den er angegeben wurde. Ein
    /// Hinweis fuer den stabilen Trace sagt ueber den Sieg nichts — das ist der
    /// Kern von AC6 und der Grund, warum der Bezug im Typ steht.
    pub fn traegt_fuer(self, gefragt: MonotonicityDomain) -> bool {
        if matches!(self.direction, MonotonicDirection::Unknown) {
            return false;
        }
        if matches!(self.domain, MonotonicityDomain::Unspecified) {
            return false;
        }
        self.domain == gefragt
    }
}

/// Grenzen eines Zahlenwerts.
///
/// `None` heisst **unbekannt**, nicht „unbegrenzt in diese Richtung" — der
/// Unterschied ist wichtig, weil eine fehlende Schranke sonst als weite
/// Schranke durchginge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Untere Grenze, falls bekannt.
    pub min: Option<i64>,
    /// Obere Grenze, falls bekannt.
    pub max: Option<i64>,
}

impl Bounds {
    /// Gar nichts bekannt.
    pub const UNKNOWN: Bounds = Bounds {
        min: None,
        max: None,
    };

    /// Ein geschlossenes Intervall.
    pub const fn exact(min: i64, max: i64) -> Bounds {
        Bounds {
            min: Some(min),
            max: Some(max),
        }
    }

    /// Ob beide Grenzen bekannt sind — nur dann ist die Schranke eng.
    pub const fn ist_eng(self) -> bool {
        self.min.is_some() && self.max.is_some()
    }
}

/// Ob und wie ein Effekt den Kontrollfluss des Kampfes beruehrt.
///
/// Aus dieser Angabe folgt, ob ueber den Effekt hinweg analytisch
/// marginalisiert werden darf: Wo eine Verzweigung am Schaden, an den
/// Lebenspunkten oder am Sieg haengt, ist der Erwartungswert nicht mehr der
/// Wert des Erwartungswerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlFlowEffect {
    /// Eine Verzweigung haengt am ausgeteilten Schaden.
    pub branches_on_damage: bool,
    /// Eine Verzweigung haengt an Lebenspunkten.
    pub branches_on_hp: bool,
    /// Eine Verzweigung haengt daran, ob der Kampf gewonnen ist.
    pub branches_on_victory: bool,
    /// Nicht angegeben — konservativ wie „alle drei" behandeln.
    pub unknown: bool,
}

impl ControlFlowEffect {
    /// Beruehrt den Kontrollfluss nicht.
    pub const NONE: ControlFlowEffect = ControlFlowEffect {
        branches_on_damage: false,
        branches_on_hp: false,
        branches_on_victory: false,
        unknown: false,
    };

    /// Keine Angabe — der konservative Wert.
    pub const UNKNOWN: ControlFlowEffect = ControlFlowEffect {
        branches_on_damage: true,
        branches_on_hp: true,
        branches_on_victory: true,
        unknown: true,
    };

    /// Ob irgendeine Verzweigung besteht.
    pub const fn verzweigt(self) -> bool {
        self.branches_on_damage || self.branches_on_hp || self.branches_on_victory || self.unknown
    }
}

// ------------------------------------------------------------ Ausloeser, Ziel

/// Wodurch ein Effekt ausgeloest wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// Beim Wirken des Zaubers.
    OnCast,
    /// Bei jedem Tick eines Kanals oder Brands.
    OnTick,
    /// Beim Auslaufen.
    OnExpire,
    /// Durch einen Eintrag der Boss-Zeitleiste.
    OnTimeline,
    /// Beim Phasenwechsel.
    OnPhaseChange,
    /// Nicht angegeben.
    Unknown,
}

/// Worauf ein Effekt wirkt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Der Boss.
    Boss,
    /// Der Spieler.
    Player,
    /// Alle laufenden Abklingzeiten.
    AllCooldowns,
    /// Eine einzelne Abklingzeit.
    OneCooldown,
    /// Nicht angegeben.
    Unknown,
}

// -------------------------------------------------------------------- Knoten

/// Ein Effekt mit seiner Semantik.
///
/// # Warum die Felder und nicht nur [`EffectOp`]
///
/// Aus einem blossen Aufzaehlungswert wie „verursacht Schaden" laesst sich
/// weder Monotonie noch Kontrollflusswirkung noch Vektorisierbarkeit ableiten.
/// Erst die Felder daneben tragen die Semantik — der Aufzaehlungswert benennt
/// sie nur.
///
/// # Erzeugt, nicht gepflegt
///
/// Die Knoten entstehen in `build.rs` aus `data/*.json`. Von Hand gepflegte
/// Knoten waeren genau die Liste, gegen die dieses Modul gebaut ist.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectNode {
    /// Welcher Baustein.
    pub op: EffectOp,
    /// Wodurch ausgeloest.
    pub trigger: Trigger,
    /// Worauf gerichtet.
    pub target: Target,
    /// Welche Zustaende gelesen werden.
    pub reads: StateMask,
    /// Welche Zustaende geschrieben werden.
    pub writes: StateMask,
    /// Wann die Eingangswerte gelesen werden.
    pub timing: TimingSemantics,
    /// Wie sich mehrfaches Auflegen verhaelt.
    pub stacking: StackingSemantics,
    /// Welche Zufallsquelle beteiligt ist.
    pub randomness: RandomnessSemantics,
    /// Der Monotonie-**Hinweis** — kein Zertifikat.
    pub monotonicity: LocalMonotonicityHint,
    /// Wertebereich, falls bekannt.
    pub numeric_bounds: Bounds,
    /// Wirkung auf den Kontrollfluss.
    pub control_flow_effect: ControlFlowEffect,
    /// Woher dieser Knoten stammt — Zauber- oder Boss-Kennung.
    ///
    /// Steht in jeder Meldung einer fehlgeschlagenen Analyse. Ohne die Herkunft
    /// waere ein Befund nicht auffindbar.
    pub origin: &'static str,
}

impl EffectNode {
    /// Ein Knoten, der **nichts** ueber sich aussagt.
    ///
    /// Jedes Feld steht auf seinem konservativen Wert. Wer einen Knoten so
    /// erzeugt, bekommt ueberall den Rueckfall — das ist beabsichtigt und die
    /// Grundlage von AC5.
    pub const fn unbekannt(op: EffectOp, origin: &'static str) -> EffectNode {
        EffectNode {
            op,
            trigger: Trigger::Unknown,
            target: Target::Unknown,
            reads: StateMask::UNKNOWN,
            writes: StateMask::UNKNOWN,
            timing: TimingSemantics::Unknown,
            stacking: StackingSemantics::Unknown,
            randomness: RandomnessSemantics::Unknown,
            monotonicity: LocalMonotonicityHint::UNKNOWN,
            numeric_bounds: Bounds::UNKNOWN,
            control_flow_effect: ControlFlowEffect::UNKNOWN,
            origin,
        }
    }

    /// Ob diesem Knoten eine Angabe fehlt.
    ///
    /// Nicht „ob er falsch ist" — nur, ob er an irgendeiner Stelle schweigt.
    /// Schweigen loest den Rueckfall aus.
    pub fn hat_luecke(&self) -> bool {
        matches!(self.trigger, Trigger::Unknown)
            || matches!(self.target, Target::Unknown)
            || self.reads.is_unknown()
            || self.writes.is_unknown()
            || matches!(self.timing, TimingSemantics::Unknown)
            || matches!(self.stacking, StackingSemantics::Unknown)
            || matches!(self.randomness, RandomnessSemantics::Unknown)
            || self.control_flow_effect.unknown
    }

    /// Ob dies eine Bossregel **ohne** deklarierte Semantik ist.
    ///
    /// Das ist das Schlupfloch aus AC4, benannt und pruefbar gemacht: Eine
    /// Signatur, die nichts ueber sich sagt, darf nicht besser wegkommen als
    /// ein unbekannter Baustein.
    pub fn signature_ohne_semantik(&self) -> bool {
        matches!(self.op, EffectOp::CustomSignature(_)) && self.hat_luecke()
    }
}

/// Die sechs konservativen Zustaende aus AC5.
///
/// Sie stehen zusammen in einer Struktur und nicht verstreut, weil sie
/// **gemeinsam** gelten muessen: Wer eine Angabe vermissen laesst, bekommt alle
/// sechs, nicht fuenf davon.
///
/// > Unbekannt muss langsam sein, nie falsch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConservativeFallback {
    /// Alle Abhaengigkeiten annehmen.
    pub alle_abhaengigkeiten: bool,
    /// Kein Ledger fuehren.
    pub kein_ledger: bool,
    /// Keine analytische Marginalisierung.
    pub keine_marginalisierung: bool,
    /// Kein Dominanz-Pruning.
    pub kein_dominanz_pruning: bool,
    /// Keine enge Schranke.
    pub keine_enge_schranke: bool,
    /// Allgemeiner Rueckfall-Kernel.
    pub rueckfall_kernel: bool,
}

impl ConservativeFallback {
    /// Alle sechs Zustaende gesetzt — der Fall bei fehlender Angabe.
    pub const ALLE: ConservativeFallback = ConservativeFallback {
        alle_abhaengigkeiten: true,
        kein_ledger: true,
        keine_marginalisierung: true,
        kein_dominanz_pruning: true,
        keine_enge_schranke: true,
        rueckfall_kernel: true,
    };

    /// Kein Rueckfall noetig.
    pub const KEINER: ConservativeFallback = ConservativeFallback {
        alle_abhaengigkeiten: false,
        kein_ledger: false,
        keine_marginalisierung: false,
        kein_dominanz_pruning: false,
        keine_enge_schranke: false,
        rueckfall_kernel: false,
    };

    /// Wie viele der sechs Zustaende gesetzt sind.
    ///
    /// Steht in Meldungen, damit ein halber Rueckfall auffaellt. Ein Rueckfall,
    /// der nur teilweise greift, ist gefaehrlicher als gar keiner: Er sieht aus
    /// wie Vorsicht und ist keine.
    pub const fn anzahl(self) -> u8 {
        self.alle_abhaengigkeiten as u8
            + self.kein_ledger as u8
            + self.keine_marginalisierung as u8
            + self.kein_dominanz_pruning as u8
            + self.keine_enge_schranke as u8
            + self.rueckfall_kernel as u8
    }

    /// Ob **alle** sechs greifen.
    pub const fn vollstaendig(self) -> bool {
        self.anzahl() == 6
    }
}
