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
//! Dasselbe gilt für die übrigen semantischen Typen des Knotens: Jede Stelle,
//! die einen von ihnen auswertet, tut es erschöpfend — kein `matches!`, kein
//! `==` gegen einen Wert, kein Auffangzweig. Beides prüft
//! `tools/wirkung-baubruch-gegenprobe.py` im sim-Job mit dem echten Übersetzer
//! (Epic #892).
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

impl TimingSemantics {
    /// Ob das Zeitverhalten nicht angegeben ist — eine Bedingung von
    /// [`EffectNode::hat_luecke`].
    ///
    /// Ein `match` ohne Auffangzweig, wie bei [`EffectOp`] (Epic #892): Ein
    /// neues Zeitverhalten muss hier entscheiden, ob es eine Lücke ist, statt
    /// still als angegeben durchzugehen. Dasselbe gilt für `ist_unbekannt` an
    /// jedem semantischen Typ des Knotens; `tools/wirkung-baubruch-gegenprobe.py`
    /// prüft es je Typ mit dem echten Übersetzer.
    pub const fn ist_unbekannt(self) -> bool {
        match self {
            TimingSemantics::Immediate | TimingSemantics::Snapshot | TimingSemantics::Live => false,
            TimingSemantics::Unknown => true,
        }
    }
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

impl StackingSemantics {
    /// Ob das Stapelverhalten nicht angegeben ist — eine Bedingung von
    /// [`EffectNode::hat_luecke`]. Ohne Auffangzweig, siehe
    /// [`TimingSemantics::ist_unbekannt`].
    pub const fn ist_unbekannt(self) -> bool {
        match self {
            StackingSemantics::None
            | StackingSemantics::Refresh
            | StackingSemantics::Extend
            | StackingSemantics::Additive
            | StackingSemantics::Capped(_)
            | StackingSemantics::Replace => false,
            StackingSemantics::Unknown => true,
        }
    }
}

/// Welche Zufallsquelle ein Effekt anfasst.
///
/// Die Frage „welche Zufallsquellen sind ueberhaupt aktiv?" wird hieraus
/// beantwortet — und nicht aus einer gepflegten Liste.
///
/// # Warum hier ein [`Stream`] steht und keine Zeichenkette
///
/// Bis Issue #966 trug die Variante einen freien `&'static str`. Der driftete
/// sofort: `build.rs` schrieb `"dot-tick"`, waehrend der Strom in
/// [`crate::rng`] `"dotTick"` heisst. **Die Zeichenkette zeigte auf nichts**,
/// und keine Pruefung konnte das melden — genau die stille Sorte Fehler, gegen
/// die dieses Modul gebaut ist.
///
/// Mit der echten Aufzaehlung ist ein Tippfehler ein Uebersetzungsfehler, und
/// ein neu hinzugefuegter Strom faellt in [`RandomSourceMask`] auf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomnessSemantics {
    /// Kein Zufall beteiligt.
    Deterministic,
    /// Zieht einen Krit-Wurf.
    CritRoll,
    /// Zieht aus einem benannten Strom des Simulators.
    Stream(crate::rng::Stream),
    /// Nicht angegeben.
    Unknown,
}

impl RandomnessSemantics {
    /// Ob **sicher** kein Zufall im Spiel ist.
    ///
    /// [`RandomnessSemantics::Unknown`] antwortet hier `false`: Unbekannt ist
    /// nicht deterministisch.
    ///
    /// Ein `match` ohne Auffangzweig, wie bei [`EffectOp`] (Epic #892): Eine
    /// neue Quelle muss hier entscheiden, ob sie würfelt, statt still als
    /// „würfelt“ durchzugehen.
    pub const fn ist_deterministisch(self) -> bool {
        match self {
            RandomnessSemantics::Deterministic => true,
            RandomnessSemantics::CritRoll
            | RandomnessSemantics::Stream(_)
            | RandomnessSemantics::Unknown => false,
        }
    }

    /// Ob die Quelle nicht angegeben ist — eine Bedingung von [`EffectNode::hat_luecke`].
    pub const fn ist_unbekannt(self) -> bool {
        match self {
            RandomnessSemantics::Deterministic
            | RandomnessSemantics::CritRoll
            | RandomnessSemantics::Stream(_) => false,
            RandomnessSemantics::Unknown => true,
        }
    }

    /// Welche Stroeme dieser Effekt anfassen kann.
    ///
    /// Der Beitrag eines einzelnen Knotens zur [`RandomSourceMask`].
    pub fn quellen(self) -> RandomSourceMask {
        match self {
            RandomnessSemantics::Deterministic => RandomSourceMask::KEINE,
            RandomnessSemantics::CritRoll => RandomSourceMask::einer(crate::rng::Stream::Crit),
            RandomnessSemantics::Stream(s) => RandomSourceMask::einer(s),
            // Unbekannt heisst **alle**, nicht keine: Wer nichts angibt, darf
            // nicht schneller sein als wer etwas angibt.
            RandomnessSemantics::Unknown => RandomSourceMask::ALLE,
        }
    }
}

/// Welche Zufallsquellen ueber eine Menge von Effekten hinweg aktiv sind.
///
/// # Wofuer das gebraucht wird (Issue #966, AC3)
///
/// Ein Lauf, in dem **keine** Zufallsquelle aktiv ist, ist zwischen zwei
/// Durchgaengen identisch — er darf auf **einen** Durchgang reduziert werden.
/// Diese Maske ist die Grundlage dafuer.
///
/// # Was sie NICHT sagt
///
/// Sie nennt die Quellen, die die **Beschreibung** kennt. Ob eine davon im
/// konkreten Lauf tatsaechlich zieht, haengt zusaetzlich an der Konfiguration
/// — bei PURITY auf null gibt es keinen Krit, obwohl Knoten `CritRoll`
/// tragen. **Leere Maske ist deshalb hinreichend fuer „ein Durchgang genuegt",
/// nicht notwendig.** Wer das verwechselt, reduziert einen Lauf, der streut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RandomSourceMask(pub u8);

impl RandomSourceMask {
    /// Keine Quelle aktiv.
    pub const KEINE: RandomSourceMask = RandomSourceMask(0);

    /// Alle Stroeme — der konservative Wert bei fehlender Angabe.
    ///
    /// Abgeleitet aus [`crate::rng::STREAM_COUNT`] und **nicht** als Zahl
    /// hingeschrieben: Ein neuer Strom weitet den Wert von selbst.
    pub const ALLE: RandomSourceMask =
        RandomSourceMask(((1u16 << crate::rng::STREAM_COUNT) - 1) as u8);

    /// Genau ein Strom.
    pub const fn einer(strom: crate::rng::Stream) -> RandomSourceMask {
        RandomSourceMask(1 << (strom as u8))
    }

    /// Vereinigung.
    pub const fn union(self, other: RandomSourceMask) -> RandomSourceMask {
        RandomSourceMask(self.0 | other.0)
    }

    /// Schnitt.
    ///
    /// Gebraucht fuer die zweite Ableitungsstufe in [`crate::durchgaenge`]:
    /// Eine Quelle ist im Lauf nur aktiv, wenn die Beschreibung sie nennt
    /// **und** die Konfiguration sie ziehen laesst. Die Vereinigung wuerde dort
    /// nie reduzieren.
    pub const fn schnitt(self, other: RandomSourceMask) -> RandomSourceMask {
        RandomSourceMask(self.0 & other.0)
    }

    /// Ob dieser Strom enthalten ist.
    pub const fn enthaelt(self, strom: crate::rng::Stream) -> bool {
        (self.0 & (1 << (strom as u8))) != 0
    }

    /// Ob gar keine Quelle aktiv ist.
    pub const fn ist_leer(self) -> bool {
        self.0 == 0
    }

    /// Wie viele Quellen aktiv sind.
    pub const fn anzahl(self) -> u32 {
        self.0.count_ones()
    }

    /// Ob ein Lauf auf **einen** Durchgang reduziert werden darf.
    ///
    /// Nur bei leerer Maske. Siehe die Einschraenkung am Typ: Die Antwort ist
    /// hinreichend, nicht notwendig — ein Lauf mit gesetzter Maske kann
    /// trotzdem deterministisch sein, wenn die Konfiguration die Quelle
    /// stilllegt. Diese Funktion behauptet darueber nichts.
    pub const fn ein_durchgang_genuegt(self) -> bool {
        self.ist_leer()
    }

    /// Die Quellen einer ganzen Wirkungsbeschreibung.
    ///
    /// **Erzeugt, nicht gepflegt** — der Kern von AC3: Die Maske entsteht aus
    /// den Knoten, nicht aus einer Liste daneben.
    pub fn aus_knoten(knoten: &[EffectNode]) -> RandomSourceMask {
        knoten.iter().fold(RandomSourceMask::KEINE, |acc, n| {
            acc.union(n.randomness.quellen())
        })
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

impl MonotonicDirection {
    /// Ob keine Richtung angegeben ist. Ohne Auffangzweig, siehe
    /// [`TimingSemantics::ist_unbekannt`]: Eine neue Richtung muss hier
    /// entscheiden, ob sie etwas aussagt, bevor ein Hinweis mit ihr trägt.
    pub const fn ist_unbekannt(self) -> bool {
        match self {
            MonotonicDirection::Increasing
            | MonotonicDirection::Decreasing
            | MonotonicDirection::NonMonotonic => false,
            MonotonicDirection::Unknown => true,
        }
    }
}

/// Worauf sich ein Monotonie-Hinweis bezieht.
///
/// **Ohne diesen Bezug ist der Hinweis wertlos.** Grundschaden ist monoton
/// bezueglich erwartetem Schaden an einem gewoehnlichen Boss bei stabilem
/// Trace — und **nicht** bezueglich der drei Zielgroessen darunter.
///
/// Die drei Gegenbeispiele stehen als eigene Varianten und nicht als Kommentar,
/// weil sie sonst niemand pruefen koennte (Issue #966, AC4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotonicityDomain {
    /// Erwarteter Schaden, gewoehnlicher Boss, stabiler Trace.
    ///
    /// Der **einzige** Bezug, fuer den ein Hinweis aus der
    /// Wirkungsbeschreibung ueberhaupt sinnvoll ist.
    ExpectedDamageStableTrace,
    /// Sieg oder Niederlage an einem Boss mit **umgekehrter** Bedingung.
    ///
    /// Wo ein Ring auf null die Niederlage ist, kann mehr Schaden schlechter
    /// sein. Der Zusammenhang kehrt sich um, statt schwaecher zu werden.
    VictoryOutcome,
    /// Zeit bis zum Sieg **ueber einen Phasenwechsel hinweg**.
    ///
    /// Mehr Schaden kann eine Phase frueher ausloesen und den Kampf dadurch
    /// verlaengern. Die Zielgroesse ist stueckweise monoton und im Ganzen
    /// nicht.
    TimeToVictoryAcrossPhase,
    /// Ein Ziel mit **Lebenspunkt-Praedikat**.
    ///
    /// Wo eine Mechanik an einer Lebenspunkt-Schwelle haengt („unter 30 %"),
    /// entscheidet nicht die Hoehe des Schadens, sondern auf welcher Seite der
    /// Schwelle man landet.
    HpPredicate,
    /// **Zaubervielfalt** — wie viele Zauber sich noch lohnen (Issue #940).
    ///
    /// Der Bezug, an dem sich zeigt, warum „monoton" ohne Bezugsgroesse
    /// gefaehrlich ist. Gemessen an `catalysis`:
    ///
    /// ```text
    ///  1 000 Promille  ->  10 von 58 Zaubern lohnend
    /// 36 000 Promille  ->   4
    /// ```
    ///
    /// Bezueglich [`Self::ExpectedDamageStableTrace`] ist der Parameter monoton
    /// **steigend**. Bezueglich dieses Bezugs ist er streng **fallend** — und
    /// die zweite Groesse ist die, die das Spiel interessiert.
    ///
    /// Ein Zertifikat „catalysis ist monoton" waere damit nicht falsch, sondern
    /// **gefaehrlich**: „monoton" ohne Bezug liest sich wie „unbedenklich", und
    /// hier ist es das Gegenteil.
    ///
    /// # Warum die Variante hier steht und nicht in `domaene.rs`
    ///
    /// Eine zweite Aufzaehlung derselben Bezugsgroessen wuerde unabhaengig
    /// driften. Dann zertifiziert das Zertifikat gegen eine Kopie, waehrend der
    /// Kampf gegen das Original rechnet.
    SpellDiversity,
    /// Kein Bezug angegeben.
    Unspecified,
}

impl MonotonicityDomain {
    /// Ob kein Bezug angegeben ist. Ohne Auffangzweig, siehe
    /// [`TimingSemantics::ist_unbekannt`]: Ein neuer Bezug muss hier
    /// entscheiden, ob er einer ist, bevor ein Hinweis für ihn trägt.
    pub const fn ist_unbestimmt(self) -> bool {
        match self {
            MonotonicityDomain::ExpectedDamageStableTrace
            | MonotonicityDomain::VictoryOutcome
            | MonotonicityDomain::TimeToVictoryAcrossPhase
            | MonotonicityDomain::HpPredicate
            | MonotonicityDomain::SpellDiversity => false,
            MonotonicityDomain::Unspecified => true,
        }
    }
}

/// Woraus ein Monotonie-**Zertifikat** entsteht.
///
/// # Warum es das als eigenen Typ gibt
///
/// Der Hinweis am Knoten ist **eine** von vier Zutaten. Ohne die anderen drei
/// gibt es kein Zertifikat — und wer den Hinweis dafuer haelt, baut eine
/// Optimierung auf eine Vermutung. Diese Struktur macht die fehlenden Zutaten
/// **benennbar**, statt sie in einem Kommentar zu erwaehnen.
///
/// Aus der Wirkungsbeschreibung allein ist [`ZertifikatsBedarf::vollstaendig`]
/// **nie** erfuellt. Das ist keine Einschraenkung dieser Umsetzung, sondern die
/// Aussage von Issue #966: Der Hinweis ist kein Zertifikat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZertifikatsBedarf {
    /// Die Wirkungsbeschreibung — dieses Modul.
    pub wirkungsbeschreibung: bool,
    /// Die Semantik des konkreten Bosses.
    pub bosssemantik: bool,
    /// Die Zielfunktion, ueber die optimiert wird.
    pub zielfunktion: bool,
    /// Die Trace-Region, in der die Aussage gelten soll.
    pub trace_region: bool,
}

impl ZertifikatsBedarf {
    /// Was die Wirkungsbeschreibung **allein** beisteuert.
    pub const AUS_HINWEIS_ALLEIN: ZertifikatsBedarf = ZertifikatsBedarf {
        wirkungsbeschreibung: true,
        bosssemantik: false,
        zielfunktion: false,
        trace_region: false,
    };

    /// Alle vier Zutaten liegen vor.
    pub const VOLLSTAENDIG: ZertifikatsBedarf = ZertifikatsBedarf {
        wirkungsbeschreibung: true,
        bosssemantik: true,
        zielfunktion: true,
        trace_region: true,
    };

    /// Ob alle vier Zutaten vorliegen.
    pub const fn vollstaendig(self) -> bool {
        self.wirkungsbeschreibung && self.bosssemantik && self.zielfunktion && self.trace_region
    }

    /// Welche Zutaten fehlen — fuer die Meldung.
    pub fn fehlend(self) -> Vec<&'static str> {
        let mut f = Vec::new();
        if !self.wirkungsbeschreibung {
            f.push("Wirkungsbeschreibung");
        }
        if !self.bosssemantik {
            f.push("Bosssemantik");
        }
        if !self.zielfunktion {
            f.push("Zielfunktion");
        }
        if !self.trace_region {
            f.push("Trace-Region");
        }
        f
    }
}

/// Warum ein Monotonie-Zertifikat verweigert wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZertifikatVerweigert {
    /// Der Hinweis nennt keine Richtung.
    KeinHinweis,
    /// Der Hinweis gilt fuer einen anderen Bezug als den gefragten.
    FalscherBezug {
        /// Wofuer der Hinweis gilt.
        hinweis: MonotonicityDomain,
        /// Wonach gefragt wurde.
        gefragt: MonotonicityDomain,
    },
    /// Der Hinweis sagt „monoton", aber es fehlen Zutaten.
    ///
    /// **Der wichtigste Fall.** Er ist der Grund, warum diese Funktion
    /// existiert: Ein zutreffender Hinweis reicht nicht.
    ZutatenFehlen,
    /// Der Effekt verzweigt am Kontrollfluss.
    KontrollflussVerzweigt,
}

/// Ob ein Monotonie-Zertifikat ausgestellt werden darf.
///
/// # Diese Funktion sagt fast immer nein — das ist ihr Zweck
///
/// Sie ist die Stelle, an der aus einem **Hinweis** kein **Zertifikat** wird.
/// Mit [`ZertifikatsBedarf::AUS_HINWEIS_ALLEIN`] verweigert sie **immer**,
/// auch wenn der Hinweis „monoton" sagt und der Bezug passt.
pub fn zertifikat(
    hinweis: LocalMonotonicityHint,
    gefragt: MonotonicityDomain,
    bedarf: ZertifikatsBedarf,
    knoten: &EffectNode,
) -> Result<(), ZertifikatVerweigert> {
    if hinweis.direction.ist_unbekannt() {
        return Err(ZertifikatVerweigert::KeinHinweis);
    }
    if !hinweis.traegt_fuer(gefragt) {
        return Err(ZertifikatVerweigert::FalscherBezug {
            hinweis: hinweis.domain,
            gefragt,
        });
    }
    if knoten.control_flow_effect.verzweigt() {
        return Err(ZertifikatVerweigert::KontrollflussVerzweigt);
    }
    if !bedarf.vollstaendig() {
        return Err(ZertifikatVerweigert::ZutatenFehlen);
    }
    Ok(())
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
        if self.direction.ist_unbekannt() {
            return false;
        }
        if self.domain.ist_unbestimmt() {
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

    /// Ob gar keine Grenze bekannt ist — [`Bounds::UNKNOWN`].
    ///
    /// Die Bedingung des konservativen Rückfalls in `kernel-choice` und
    /// `action-bound` beim Wirken (DW3, Epic #892). Ein halb offener Bereich ist
    /// eine Teilangabe und fällt dort nicht zurück.
    pub const fn ist_unbekannt(self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    /// Die Huelle zweier Bereiche.
    ///
    /// **Eine unbekannte Grenze steckt an**: Wer `None` mit `Some` vereinigt,
    /// bekommt `None`. Andersherum waere die Huelle enger als einer ihrer
    /// Teile, und eine erfundene Schranke ist schlimmer als keine.
    pub fn huelle(self, other: Bounds) -> Bounds {
        Bounds {
            min: match (self.min, other.min) {
                (Some(a), Some(b)) => Some(a.min(b)),
                _ => None,
            },
            max: match (self.max, other.max) {
                (Some(a), Some(b)) => Some(a.max(b)),
                _ => None,
            },
        }
    }
}

/// Was der numerische Vertrag aus der Wirkungsbeschreibung bekommt.
///
/// # Wofuer (Issue #966, AC5)
///
/// Die Wertebereiche der Knoten sind die **Eingabe** fuer den numerischen
/// Vertrag: Aus ihnen folgt, in welchem Bereich sich Zwischenergebnisse
/// bewegen und ob eine Festkomma- oder Ganzzahl-Darstellung traegt.
///
/// Dieses Modul stellt sie bereit; den Vertrag selbst schliesst es nicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumerischerVertrag {
    /// Die Huelle aller bekannten Bereiche.
    pub huelle: Bounds,
    /// Wie viele Knoten eine **enge** Schranke tragen.
    pub mit_enger_schranke: usize,
    /// Wie viele Knoten gar keine Schranke tragen.
    ///
    /// Steht getrennt, weil die Huelle allein das verschweigt: Ein einziger
    /// Knoten ohne Obergrenze macht die Huelle offen, und dann sieht sie
    /// genauso aus wie bei hundert solchen Knoten.
    pub ohne_schranke: usize,
    /// Wie viele Knoten insgesamt eingegangen sind.
    pub knoten: usize,
}

impl NumerischerVertrag {
    /// Die Bereiche einer ganzen Wirkungsbeschreibung.
    pub fn aus_knoten(knoten: &[EffectNode]) -> NumerischerVertrag {
        let mut huelle: Option<Bounds> = None;
        let mut mit_enger_schranke = 0;
        let mut ohne_schranke = 0;
        for n in knoten {
            if n.numeric_bounds.ist_eng() {
                mit_enger_schranke += 1;
            }
            if n.numeric_bounds.ist_unbekannt() {
                ohne_schranke += 1;
            }
            huelle = Some(match huelle {
                None => n.numeric_bounds,
                Some(h) => h.huelle(n.numeric_bounds),
            });
        }
        NumerischerVertrag {
            huelle: huelle.unwrap_or(Bounds::UNKNOWN),
            mit_enger_schranke,
            ohne_schranke,
            knoten: knoten.len(),
        }
    }

    /// Ob der Vertrag auf einer **geschlossenen** Huelle steht.
    ///
    /// Nur dann laesst sich eine Darstellung waehlen, ohne zu raten.
    pub const fn traegt(&self) -> bool {
        self.huelle.ist_eng()
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

impl Trigger {
    /// Ob der Auslöser nicht angegeben ist — eine Bedingung von
    /// [`EffectNode::hat_luecke`]. Ohne Auffangzweig, siehe
    /// [`TimingSemantics::ist_unbekannt`].
    pub const fn ist_unbekannt(self) -> bool {
        match self {
            Trigger::OnCast
            | Trigger::OnTick
            | Trigger::OnExpire
            | Trigger::OnTimeline
            | Trigger::OnPhaseChange => false,
            Trigger::Unknown => true,
        }
    }
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

impl Target {
    /// Ob das Ziel nicht angegeben ist — eine Bedingung von
    /// [`EffectNode::hat_luecke`]. Ohne Auffangzweig, siehe
    /// [`TimingSemantics::ist_unbekannt`].
    pub const fn ist_unbekannt(self) -> bool {
        match self {
            Target::Boss | Target::Player | Target::AllCooldowns | Target::OneCooldown => false,
            Target::Unknown => true,
        }
    }
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
        self.trigger.ist_unbekannt()
            || self.target.ist_unbekannt()
            || self.reads.is_unknown()
            || self.writes.is_unknown()
            || self.timing.ist_unbekannt()
            || self.stacking.ist_unbekannt()
            || self.randomness.ist_unbekannt()
            || self.control_flow_effect.unknown
    }

    /// Ob dies eine Bossregel **ohne** deklarierte Semantik ist.
    ///
    /// Das ist das Schlupfloch aus AC4, benannt und pruefbar gemacht: Eine
    /// Signatur, die nichts ueber sich sagt, darf nicht besser wegkommen als
    /// ein unbekannter Baustein.
    ///
    /// Ein `match` ohne Auffangzweig (Epic #892): Ein neuer Baustein muss hier
    /// entscheiden, ob er eine Bossregel ist.
    pub fn signature_ohne_semantik(&self) -> bool {
        match self.op {
            EffectOp::CustomSignature(_) => self.hat_luecke(),
            EffectOp::DealDamage
            | EffectOp::ApplyDot
            | EffectOp::ApplyBuff
            | EffectOp::ModifyResource
            | EffectOp::ReduceCooldown
            | EffectOp::ResetCooldown
            | EffectOp::FreeNextGcd
            | EffectOp::Heal
            | EffectOp::Shield
            | EffectOp::ConvertDamage
            | EffectOp::ChangeBossState
            | EffectOp::ChangePhase => false,
        }
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
