//! Der kritische Treffer analytisch — **neben** dem Kampf (Issue #908, AC4),
//! und angeschlossen an Durchgangswahl und Rotationssuche (Issue #920).
//!
//! # Warum neben und nicht im Kampf
//!
//! Der Kampf würfelt weiter. Er muss es: GAME-DESIGN 18.4 setzt die
//! Ziehungszähler **je Takt** in die Zustands-Prüfsumme, und `fight.rs` tut das
//! auch (`mix_named_int(h, "randomDraws", …)`). Ein Kampf, der statt zu ziehen
//! den Erwartungswert führt, hätte andere Zähler, eine andere Prüfsumme und
//! bräche jede Paritäts-Aufzeichnung. AC4 sagt dasselbe kürzer: „nur dort, **wo
//! er den Kontrollfluss nicht berührt**".
//!
//! Was hier ersetzt wird, ist deshalb nicht der Kampf, sondern das **Mitteln
//! über viele Durchgänge**, mit dem eine Balance-Zahl entsteht.
//!
//! # Die Rechnung, und warum sie im Modell exakt ist
//!
//! Nicht den Erwartungswert in den Krit-Faktor setzen — das ist der
//! naheliegende und falsche Weg, weil [`crate::compute_damage`] **je Treffer**
//! rundet und Runden nicht mit Mitteln vertauscht
//! (`sim/tests/analytischer_krit_grenzen.rs` misst es: 11 von 18 Kombinationen
//! laufen auseinander).
//!
//! Stattdessen wird über die **Ausgänge** marginalisiert. Für einen einzelnen
//! Treffer mit Krit-Wahrscheinlichkeit `p`:
//!
//! ```text
//! E[Treffer] = p · runden(basis · krit) + (1 − p) · runden(basis)
//! ```
//!
//! Über einen ganzen Kampf mit vielen Treffern greift die **Linearität des
//! Erwartungswerts** — und die gilt unabhängig davon, wie die einzelnen Würfe
//! zusammenhängen:
//!
//! ```text
//! E[Σ Treffer] = Σ E[Treffer] = p · (alle kriten) + (1 − p) · (keiner kritet)
//! ```
//!
//! Damit genügen **zwei** deterministische Läufe statt fünfzig bis zweihundert
//! gewürfelter.
//!
//! # Die Voraussetzung, ohne die das nicht gilt
//!
//! Die Trefferfolge muss in allen Ausgängen dieselbe sein. Sobald am
//! Krit-Ergebnis eine **Verzweigung** hängt — eine Lebenspunkt-Phase, ein
//! Abbruch beim Sieg, eine Zeile `if boss.hp_pct < 20` —, laufen die Ausgänge
//! auseinander, und die Summe der Erwartungswerte ist nicht mehr der
//! Erwartungswert der Summe.
//!
//! # Die Prüfung ist zweistellig, und sie gilt je Aufbau × Boss (#920)
//!
//! Bis #920 fragte dieses Modul nur die Knoten, die selbst würfeln
//! ([`krit_beruehrt_den_kontrollfluss`]). Das sah die **Bossseite** nicht: Ein
//! Phasenknoten würfelt nicht, er **liest** den Fortschritt am Boss — und der
//! hängt am Krit. Deshalb jetzt in drei Teilen:
//!
//! | Teil | Frage | Antwort steht in |
//! | --- | --- | --- |
//! | Maske (AC2) | Welche Zustände verändert der Ausgang eines Wurfs, und welcher Leser verzweigt daran? | [`Abhaengigkeitsmaske`] |
//! | Zweite Stelle (AC3) | Ist die **gefragte** Größe in den Treffern linear? | [`Ergebnisgroesse::ist_linear`], [`kann_ausintegrieren`] |
//! | Aufteilung (AC4) | Welche Quelle wird gezogen, welche ausintegriert, welche ruht? | [`StochastischeAufteilung`] |
//!
//! Die Maske ist **abgeleitet**: Sie entsteht aus den Knoten der Zauber, die
//! die Rotation nennt, und aus den Knoten des Bosses, dazu aus den Bedingungen
//! der Rotation und den Abbrüchen des Laufs. Eine Bossliste gibt es nicht.
//! Ein neuer Effekt-Baustein bricht den Bau in `ausgang_veraendert`, ein
//! neuer Wert der Rotationssprache in [`crate::apl::Value::zustand`], ein neuer
//! Protokolleintrag in [`randabgleich`] — jeweils ein `match` ohne
//! Auffangzweig.
//!
//! # Und die Ableitung wird nachgeprüft, nicht geglaubt
//!
//! Eine Voraussetzung ist Code, und Code hat Fehler (Auflage 2). Bevor ein
//! Erwartungswert herausgeht, vergleicht [`randabgleich`] die beiden Randläufe
//! Eintrag für Eintrag: dieselben Würfe zur selben Zeit, dieselben Schläge des
//! Bosses, dieselbe erlittene Menge, und jeder Treffer des Krit-Laufs passt zu
//! seinem Gegenstück. Weichen sie ab, gibt es **keinen** Wert, und der Aufrufer
//! fällt auf die Stichprobe zurück. Gezählt wird das in [`verweigerungen`].

use std::cell::Cell;
use std::sync::OnceLock;

use crate::apl::Apl;
use crate::austausch::kann_durch_krit_ausgeloest_werden;
use crate::balance::BalanceConfig;
use crate::data::{Boss, BOSSES, EFFECT_NODES, SPELLS};
use crate::effects::passes::AnalyticCritPass;
use crate::effects::registry::{EffectAnalysisPass, Verdict};
use crate::effects::{EffectNode, EffectOp, RandomSourceMask, StateMask};
use crate::fight::Fight;
use crate::gear::WornValues;
use crate::log::{Entry, LogKind};
use crate::rng::Stream;

/// Hängt am Ergebnis eines **würfelnden Knotens** eine Verzweigung?
///
/// **Die erste Stufe, nicht die ganze Prüfung.** Gefragt wird nur bei Knoten,
/// die selbst würfeln — ein Phasenknoten des Bosses, der den Fortschritt liest
/// und daran verzweigt, bleibt hier unsichtbar. Die volle Ableitung je Aufbau ×
/// Boss ist [`Abhaengigkeitsmaske`]; sie ruft dieselbe Analyse für jeden
/// würfelnden Knoten auf und geht von dort weiter.
#[must_use]
pub fn krit_beruehrt_den_kontrollfluss(knoten: &[EffectNode]) -> bool {
    knoten
        .iter()
        .filter(|n| !n.randomness.ist_deterministisch())
        .any(|n| !matches!(AnalyticCritPass.analyze(n).verdict, Verdict::Permits))
}

// ------------------------------------------------- die gefragte Größe (AC3)

/// Wonach ein Lauf fragt — die **zweite Stelle** der Prüfung (#920, AC3).
///
/// „Der Zufall ändert die Aktionsfolge nicht" sagt nicht, dass die gefragte
/// Größe im Zufall linear ist. Für den erwarteten Gesamtschaden bei festem
/// Trace trägt die Gewichtung [`gewichtet`]; für eine Wahrscheinlichkeit, ein
/// Quantil oder eine Schwellenfrage nicht — `P(Σ Treffer ≥ Lebenspunkte)` ist
/// keine gewichtete Summe zweier Randläufe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ergebnisgroesse {
    /// Der erwartete Gesamtschaden bei festem Trace — die Größe der Suche.
    ErwarteterSchaden,
    /// Die Wahrscheinlichkeit, den Boss zu legen — die Größe der gestaffelten
    /// Stichprobe ([`crate::balance::run_gestaffelt`]).
    Siegwahrscheinlichkeit,
    /// Ein Quantil des Schadens, etwa p10 oder p90.
    Schadensquantil,
    /// Die Wahrscheinlichkeit, den Kampf zu überleben.
    Ueberlebenswahrscheinlichkeit,
    /// Die Wahrscheinlichkeit, eine Phase zu erreichen.
    PhaseErreicht,
    /// „Fällt der Boss vor Zeitpunkt X?"
    SiegVorZeitpunkt,
}

impl Ergebnisgroesse {
    /// Alle Größen in fester Reihenfolge — für Prüffälle und Berichte.
    pub const ALLE: [Self; 6] = [
        Self::ErwarteterSchaden,
        Self::Siegwahrscheinlichkeit,
        Self::Schadensquantil,
        Self::Ueberlebenswahrscheinlichkeit,
        Self::PhaseErreicht,
        Self::SiegVorZeitpunkt,
    ];

    /// Ist die Größe ein Erwartungswert, der in **jedem einzelnen Treffer**
    /// linear ist?
    ///
    /// Ohne Auffangzweig: Eine neue Größe muss hier entscheiden. Ein
    /// `_ => true` machte jede neue Wahrscheinlichkeit still analytisch.
    #[must_use]
    pub const fn ist_linear(self) -> bool {
        match self {
            Self::ErwarteterSchaden => true,
            Self::Siegwahrscheinlichkeit
            | Self::Schadensquantil
            | Self::Ueberlebenswahrscheinlichkeit
            | Self::PhaseErreicht
            | Self::SiegVorZeitpunkt => false,
        }
    }
}

// ------------------------------------------------- die Maske (AC2)

/// Wo der Ausgang eines Wurfs auf eine Verzweigung trifft.
///
/// Die Fundstelle steht im Befund, damit ein „berührt" auffindbar ist: Ein
/// bloßes Ja hieße bei 21 Bossen, 21-mal von vorn zu suchen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fundstelle {
    /// Ein Knoten der Wirkungsbeschreibung — mit seiner Herkunft, also
    /// Zauber-Kennung oder `boss/system`.
    Knoten(&'static str),
    /// Eine Bedingung der Rotation.
    Zeile {
        /// Die Zeilennummer in der Liste.
        zeile: usize,
        /// Der Zauber der Zeile.
        zauber: &'static str,
    },
    /// Der Lauf endet mit dem Sieg (`stop_on_kill`). Er liest den
    /// Fortschritt am Boss in der Sieg-Prüfung von `Fight::run_with`.
    AbbruchBeimSieg,
    /// Der Lauf endet mit dem Tod des Spielers — **immer**, auch ohne
    /// `stop_on_kill` (GAME-DESIGN A.3). Er liest die Lebenspunkte.
    AbbruchBeimTod,
    /// Die Frage, ob ein Zauber bereit ist: Abklingzeit, Wartezeit, Ressource
    /// (`Fight::is_ready`). Sie entscheidet, welche Zeile greift.
    Bereitschaft,
    /// Der Boss hat in der Beschreibung keinen einzigen Knoten. Ohne
    /// Beschreibung ist nichts ausgeschlossen — **unbekannt muss langsam sein,
    /// nie falsch**.
    BossOhneBeschreibung(&'static str),
}

/// Was die Maske über **eine** Zufallsquelle herausgefunden hat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quellbefund {
    /// Die Quelle.
    pub quelle: Stream,
    /// Ob sie in diesem Lauf zieht: Ein Knoten des Aufbaus oder des Bosses
    /// nennt sie, **und** die Konfiguration legt sie nicht still
    /// ([`crate::durchgaenge::zieht`]).
    pub zieht: bool,
    /// Welche Zustände der **Ausgang** ihrer Würfe verändert — nach der
    /// Fortpflanzung über alle Knoten, die einen veränderten Zustand lesen und
    /// selbst nicht verzweigen.
    pub veraendert: StateMask,
    /// Jeder Leser eines veränderten Zustands, der verzweigt. Leer heißt: Der
    /// Ausgang dieser Quelle ändert die Aktionsfolge nicht.
    pub verzweigt_an: Vec<Fundstelle>,
}

impl Quellbefund {
    /// Ändert der Ausgang dieser Quelle die Aktionsfolge?
    #[must_use]
    pub fn beruehrt_den_kontrollfluss(&self) -> bool {
        !self.verzweigt_an.is_empty()
    }
}

/// Die Abhängigkeitsmaske eines Aufbaus gegen einen Boss (#920, AC2).
///
/// # Abgeleitet, nicht gepflegt
///
/// ```text
/// FALSCH — altert still        if boss_id != BOSS_1_2 && boss_id != BOSS_3_4 { … }
/// RICHTIG — abgeleitet         if !maske.quelle(Stream::Crit).beruehrt_den_kontrollfluss() { … }
/// ```
///
/// Die Maske entsteht in drei Stufen, je Quelle aus [`Stream::ALL`]:
///
/// 1. **Die würfelnden Knoten selbst.** Jeder geht durch dieselbe
///    registrierte Analyse wie bisher, [`AnalyticCritPass`]; verbietet sie,
///    hängt die Verzweigung am Knoten selbst. Was sein Ausgang verändert, sagt
///    `ausgang_veraendert`.
/// 2. **Die Fortpflanzung.** Ein Knoten, der einen veränderten Zustand liest,
///    ist ein Leser: Verzweigt er, ist die Quelle berührt; sonst wird, was er
///    schreibt, ebenfalls verändert. Bis nichts mehr dazukommt.
/// 3. **Die Leser außerhalb der Beschreibung**: die Bedingungen der Rotation
///    ([`crate::apl::Value::zustand`]), die Abbrüche des Laufs, die
///    Bereitschaftsfrage.
///
/// # Was hier bewusst NICHT als Verzweigung zählt
///
/// **Der Weckruf nach dem Sieg** (`Fight::cast`: `if self.damage >=
/// self.boss.hp`). Er liest den Fortschritt und setzt einen
/// Entscheidungspunkt — ohne `stop_on_kill` endet der Lauf dort aber nicht.
/// Nach dem Entscheidungsmodell aus #1027 entscheidet ein solcher Punkt nichts
/// Neues: Die Rotation hat beim selben Zustand schon „warten" gesagt, und was
/// sich danach ändert, weckt selbst. Was die Rotation am Fortschritt ablesen
/// könnte, fängt Stufe 3. **Geglaubt wird das nicht**: [`randabgleich`]
/// vergleicht die Randläufe, bevor ein Wert herausgeht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Abhaengigkeitsmaske {
    /// Je Quelle ein Befund, in der Reihenfolge von [`Stream::ALL`].
    befunde: Vec<Quellbefund>,
}

impl Abhaengigkeitsmaske {
    /// Die Maske für diese Rotation gegen den Boss der Konfiguration.
    #[must_use]
    pub fn ableiten(config: &BalanceConfig, apl: &Apl) -> Self {
        // ROTER STAND (#920): die Prüfung von main. Alle würfelnden Knoten der
        // Beschreibung, keine Bossseite, keine Rotation — nur `stop_on_kill`.
        let _ = apl;
        let knoten: Vec<&EffectNode> = EFFECT_NODES.iter().collect();
        let leser = if config.stop_on_kill {
            vec![(Fundstelle::AbbruchBeimSieg, StateMask::BOSS_HP)]
        } else {
            Vec::new()
        };
        Self::aus_teilen(&knoten, &leser, &config.worn)
    }

    /// Die Maske aus gegebenen Teilen — für Prüffälle mit gebauten Knoten.
    ///
    /// `leser` sind die Leser außerhalb der Beschreibung; jeder verzweigt.
    #[must_use]
    pub fn aus_teilen(
        knoten: &[&EffectNode],
        leser: &[(Fundstelle, StateMask)],
        worn: &WornValues,
    ) -> Self {
        Self {
            befunde: Stream::ALL
                .into_iter()
                .map(|quelle| befund_fuer(quelle, knoten, leser, worn))
                .collect(),
        }
    }

    /// Der Befund für eine Quelle.
    ///
    /// # Panics
    ///
    /// Nie: Die Maske trägt jede Quelle aus [`Stream::ALL`].
    #[must_use]
    pub fn quelle(&self, quelle: Stream) -> &Quellbefund {
        self.befunde
            .iter()
            .find(|b| b.quelle == quelle)
            .expect("die Maske trägt jede Quelle aus Stream::ALL")
    }

    /// Alle Befunde, in der Reihenfolge von [`Stream::ALL`].
    #[must_use]
    pub fn befunde(&self) -> &[Quellbefund] {
        &self.befunde
    }
}

/// Die drei Stufen der Maske für eine Quelle.
fn befund_fuer(
    quelle: Stream,
    knoten: &[&EffectNode],
    leser: &[(Fundstelle, StateMask)],
    worn: &WornValues,
) -> Quellbefund {
    let mut verzweigt_an = Vec::new();
    let mut veraendert = StateMask::EMPTY;
    let mut beschrieben = false;

    // Stufe 1 — die würfelnden Knoten.
    for n in knoten {
        if !n.randomness.quellen().enthaelt(quelle) {
            continue;
        }
        beschrieben = true;
        if AnalyticCritPass.analyze(n).langsam() {
            verzweigt_an.push(Fundstelle::Knoten(n.origin));
        }
        veraendert = veraendert.union(ohne_stromzustand(ausgang_veraendert(n)));
    }
    let zieht = beschrieben && crate::durchgaenge::zieht(quelle, worn);
    if !zieht {
        return Quellbefund {
            quelle,
            zieht,
            veraendert: StateMask::EMPTY,
            verzweigt_an: Vec::new(),
        };
    }

    // ROTER STAND (#920): keine Fortpflanzung über lesende Knoten.

    // Stufe 3 — die Leser außerhalb der Beschreibung.
    for (stelle, liest) in leser {
        if liest.intersects(veraendert) {
            verzweigt_an.push(*stelle);
        }
    }

    Quellbefund {
        quelle,
        zieht,
        veraendert,
        verzweigt_an,
    }
}

/// Ein Zustand ohne den Stand der Zufallsströme.
///
/// Ein Wurf schreibt `RNG`, aber was danach im Strom steht, hängt an der
/// **Zahl** der Züge, nicht an ihrem Ausgang: `RandomStream::next_u32` zählt
/// den Zustand um eine feste Zahl weiter, ob der Treffer kritet oder nicht. Die
/// Zahl der Züge legt der Trace fest. Bliebe `RNG` stehen, gälte jeder
/// Schadensknoten — er liest `RNG` — als Leser des Krit, und über seine übrigen
/// Schreibziele (Brand-Laufzeit, Stapel) jede Zeile der Rotation als berührt.
fn ohne_stromzustand(zustand: StateMask) -> StateMask {
    StateMask(zustand.0 & !StateMask::RNG.0)
}

/// Was der **Ausgang** eines Wurfs dieses Knotens verändert — nicht, was der
/// Knoten insgesamt schreibt.
///
/// Der Unterschied trägt die ganze Maske: Ein Brand schreibt Laufzeit, Stapel
/// und Lebenspunkte, aber ob sein Tick kritet, ändert nur den **Betrag**, der
/// am Boss ankommt. Stünde hier `writes`, wäre jede Rotation mit einer
/// `dot.<zauber>.remains`-Zeile berührt, und der analytische Weg griffe fast
/// nie.
///
/// `StateMask::BOSS_HP` steht dabei für den ausgeteilten Betrag: Jede Stelle,
/// die ihn liest — Sieg-Prüfung, Phasen, der Ring eines Rennens, die
/// Umwandlung in Schutz —, liest ihn über den Fortschritt am Boss.
///
/// # Der Auffangzweig fehlt mit Absicht (AC2)
///
/// Ein neuer Baustein bricht hier den Bau, bis jemand entschieden hat, was sein
/// Wurf verändert. `tools/wirkung-baubruch-gegenprobe.py` führt diese Stelle
/// unter „außerhalb der Analysen".
fn ausgang_veraendert(knoten: &EffectNode) -> StateMask {
    // Zuerst die Lücke, wie in jeder Analyse: Unbekannt verändert alles.
    if knoten.hat_luecke() {
        return StateMask::UNKNOWN;
    }
    match knoten.op {
        // Der Krit ändert den Betrag, sonst nichts — gemessen in
        // `Fight::cast` und `Fight::dot_tick`: Der Wurf wählt nur den Faktor
        // in `factors_for`, und der Betrag geht durch `deal`.
        EffectOp::DealDamage | EffectOp::ApplyDot => StateMask::BOSS_HP,
        // Ein gewürfelter Betrag auf der Seite des Spielers landet in dem, was
        // der Knoten schreibt — Lebenspunkte oder Schutz.
        EffectOp::Heal | EffectOp::Shield | EffectOp::ConvertDamage => knoten.writes,
        // Ein Wurf, der nicht nur einen Betrag festlegt, verändert, WAS folgt.
        // Heute würfelt keiner dieser Bausteine; täte es einer, wäre der
        // Krit hier nirgends mehr ausintegrierbar — langsam, nicht falsch.
        EffectOp::ApplyBuff
        | EffectOp::ModifyResource
        | EffectOp::ReduceCooldown
        | EffectOp::ResetCooldown
        | EffectOp::FreeNextGcd
        | EffectOp::ChangeBossState
        | EffectOp::ChangePhase
        | EffectOp::CustomSignature(_) => StateMask::UNKNOWN,
    }
}

/// Die Leser außerhalb der Wirkungsbeschreibung — Lauf und Rotation.
fn leser_des_laufs(config: &BalanceConfig, apl: &Apl) -> Vec<(Fundstelle, StateMask)> {
    let mut leser = vec![
        (Fundstelle::AbbruchBeimTod, StateMask::PLAYER_HP),
        (
            Fundstelle::Bereitschaft,
            StateMask::COOLDOWNS
                .union(StateMask::GCD)
                .union(StateMask::RESOURCE),
        ),
    ];
    if config.stop_on_kill {
        leser.push((Fundstelle::AbbruchBeimSieg, StateMask::BOSS_HP));
    }
    for regel in &apl.rules {
        let liest = regel
            .condition
            .groups
            .iter()
            .flatten()
            .fold(StateMask::EMPTY, |m, t| {
                m.union(t.left.zustand()).union(t.right.zustand())
            });
        if !liest.is_empty() {
            leser.push((
                Fundstelle::Zeile {
                    zeile: regel.line,
                    zauber: SPELLS[regel.spell].id,
                },
                liest,
            ));
        }
    }
    leser
}

/// Die Knoten je Zauber, einmal aus [`EFFECT_NODES`] gelesen.
///
/// Die Herkunft eines Zauberknotens ist die Kennung des Zaubers (`build.rs`,
/// `knoten_fuer_zauber`). Einmal aufgebaut statt je Messung gefiltert: Die
/// Suche fragt die Maske je Liste, und das sind über zweitausend je Boss.
fn knoten_je_zauber() -> &'static [Vec<&'static EffectNode>] {
    static TABELLE: OnceLock<Vec<Vec<&'static EffectNode>>> = OnceLock::new();
    TABELLE.get_or_init(|| {
        SPELLS
            .iter()
            .map(|zauber| {
                EFFECT_NODES
                    .iter()
                    .filter(|n| n.origin == zauber.id)
                    .collect()
            })
            .collect()
    })
}

/// Die Knoten eines Bosses — Herkunft `<boss>/<system>` (`build.rs`,
/// `knoten_fuer_boss`). `None`, wenn die Beschreibung den Boss nicht kennt.
fn knoten_des_bosses(boss: &Boss) -> Option<&'static [&'static EffectNode]> {
    static TABELLE: OnceLock<Vec<(&'static str, Vec<&'static EffectNode>)>> = OnceLock::new();
    let tabelle = TABELLE.get_or_init(|| {
        BOSSES
            .iter()
            .map(|b| {
                let knoten = EFFECT_NODES
                    .iter()
                    .filter(|n| {
                        n.origin
                            .strip_prefix(b.id)
                            .is_some_and(|rest| rest.starts_with('/'))
                    })
                    .collect();
                (b.id, knoten)
            })
            .collect()
    });
    tabelle
        .iter()
        .find(|(id, _)| *id == boss.id)
        .map(|(_, knoten)| knoten.as_slice())
        .filter(|knoten| !knoten.is_empty())
}

/// **Einstellig:** Ändert der Ausgang dieser Quelle die Aktionsfolge?
///
/// Die erste Stelle allein. Für eine Verteilungsgröße ist sie **nicht** die
/// Antwort — dafür steht [`kann_ausintegrieren`].
#[must_use]
pub fn beruehrt_den_kontrollfluss(quelle: Stream, maske: &Abhaengigkeitsmaske) -> bool {
    maske.quelle(quelle).beruehrt_den_kontrollfluss()
}

/// **Zweistellig** (#920, AC3): Darf diese Quelle für diese Größe
/// ausintegriert werden?
///
/// Erste Stelle: Der Ausgang ändert die Aktionsfolge nicht. Zweite Stelle: Die
/// gefragte Größe ist in den Treffern linear. Beide müssen gelten — die
/// Siegwahrscheinlichkeit an einem Boss ohne Phasen besteht die erste und
/// scheitert an der zweiten.
#[must_use]
pub fn kann_ausintegrieren(
    quelle: Stream,
    gefragt: Ergebnisgroesse,
    maske: &Abhaengigkeitsmaske,
) -> bool {
    // ROTER STAND (#920): einstellig — die gefragte Größe zählt nicht.
    let _ = gefragt;
    !beruehrt_den_kontrollfluss(quelle, maske)
}

// ------------------------------------------------- die Aufteilung (AC4)

/// Warum eine Quelle in ihrer Gruppe steht — der Gültigkeitsnachweis je Quelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Begruendung {
    /// Sie zieht in diesem Lauf nicht: Keiner der Knoten nennt sie, oder die
    /// Konfiguration legt sie still (PURITY 0 oder 1000).
    Ruht,
    /// Ihr Ausgang erreicht diese verzweigenden Leser.
    Kontrollfluss(Vec<Fundstelle>),
    /// Der Kontrollfluss bleibt unberührt, aber die gefragte Größe ist in den
    /// Treffern nicht linear.
    NichtLinear(Ergebnisgroesse),
    /// Ihr Ausgang verändert nur diese Zustände, und kein Leser verzweigt
    /// daran.
    Ausintegriert(StateMask),
}

/// Die Aufteilung der Zufallsquellen eines Laufs (#920, AC4).
///
/// **Eine Antwort je Quelle, nicht ein globales Ja oder Nein.** Jede Quelle aus
/// [`Stream::ALL`] steht in genau einer der drei Gruppen, und
/// [`Self::gueltigkeitsnachweis`] sagt je Quelle, warum.
///
/// ```text
/// Krit ohne Phase und ohne Abbruch   ->  ausintegriert, BEDINGT auf den Trace
/// Krit an einem Boss mit Phasen      ->  gezogen, Fundstelle boss-1-2/phases
/// Heilung, die nirgends zieht        ->  ruhend
/// ```
///
/// Das ist bedingtes Monte-Carlo: Der Erwartungswert bleibt exakt, die
/// Varianz der ausintegrierten Quellen fällt weg.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StochastischeAufteilung {
    /// Ändern den Kontrollfluss, oder die Größe ist in ihnen nicht linear.
    pub gezogen: RandomSourceMask,
    /// Ändern nur das Ergebnis.
    pub ausintegriert: RandomSourceMask,
    /// Ziehen in diesem Lauf gar nicht.
    pub ruhend: RandomSourceMask,
    /// Wonach gefragt wurde.
    pub gefragt: Ergebnisgroesse,
    /// Je Quelle die Begründung, in der Reihenfolge von [`Stream::ALL`].
    pub gueltigkeitsnachweis: Vec<(Stream, Begruendung)>,
}

impl StochastischeAufteilung {
    /// Die Aufteilung für diese Rotation gegen den Boss der Konfiguration.
    #[must_use]
    pub fn fuer(config: &BalanceConfig, apl: &Apl, gefragt: Ergebnisgroesse) -> Self {
        Self::aus_maske(&Abhaengigkeitsmaske::ableiten(config, apl), gefragt)
    }

    /// Die Aufteilung aus einer fertigen Maske.
    #[must_use]
    pub fn aus_maske(maske: &Abhaengigkeitsmaske, gefragt: Ergebnisgroesse) -> Self {
        let mut aufteilung = Self {
            gezogen: RandomSourceMask::KEINE,
            ausintegriert: RandomSourceMask::KEINE,
            ruhend: RandomSourceMask::KEINE,
            gefragt,
            gueltigkeitsnachweis: Vec::with_capacity(Stream::ALL.len()),
        };
        // ROTER STAND (#920): ein globales Ja oder Nein für alle Quellen.
        let ja = maske
            .befunde()
            .iter()
            .all(|b| kann_ausintegrieren(b.quelle, gefragt, maske));
        for befund in maske.befunde() {
            let begruendung = if ja {
                aufteilung.ausintegriert = RandomSourceMask::ALLE;
                Begruendung::Ausintegriert(befund.veraendert)
            } else {
                aufteilung.gezogen = RandomSourceMask::ALLE;
                Begruendung::Kontrollfluss(befund.verzweigt_an.clone())
            };
            aufteilung
                .gueltigkeitsnachweis
                .push((befund.quelle, begruendung));
        }
        aufteilung
    }

    /// Trägt der analytische Weg diesen Lauf?
    ///
    /// Nur, wenn **keine** Quelle gezogen werden muss und mindestens eine
    /// ausintegriert wird. Eine gemischte Aufteilung — eine Quelle gezogen,
    /// eine ausintegriert — ist bedingtes Monte-Carlo im vollen Sinn; die
    /// beiden Randläufe unten können sie nicht rechnen, weil PURITY beide
    /// Krit-Ströme zugleich stellt. Sie bleibt bei der Stichprobe.
    #[must_use]
    pub fn analytisch(&self) -> bool {
        self.gezogen.ist_leer() && !self.ausintegriert.ist_leer()
    }
}

// ------------------------------------------------- die Rechnung

/// Ein Randlauf: alle Würfe auf einer Seite, also ohne einen einzigen Zug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rand {
    /// Der ausgeteilte Schaden.
    pub schaden: u32,
    /// Der erlittene Schaden.
    pub erlitten: u32,
    /// Das Protokoll des Laufs.
    pub eintraege: Vec<Entry>,
}

/// Warum es keinen analytischen Wert gibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verweigerung {
    /// Die Aufteilung zieht eine Quelle.
    Gezogen,
    /// Ein Randlauf schrieb mehr Einträge, als das Protokoll hält.
    ProtokollZuKurz {
        /// Wie viele geschrieben wurden.
        geschrieben: u64,
    },
    /// Die Randläufe weichen ab — die Ableitung hat etwas übersehen.
    Abweichung {
        /// Der erste abweichende Eintrag, gezählt ohne die Sieg-Einträge.
        eintrag: usize,
        /// Was abweicht.
        grund: &'static str,
    },
}

impl std::fmt::Display for Verweigerung {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gezogen => write!(f, "die Aufteilung zieht eine Quelle"),
            Self::ProtokollZuKurz { geschrieben } => write!(
                f,
                "ein Randlauf schrieb {geschrieben} Einträge, das Protokoll hält {PROTOKOLL}"
            ),
            Self::Abweichung { eintrag, grund } => {
                write!(f, "die Randläufe weichen bei Eintrag {eintrag} ab: {grund}")
            }
        }
    }
}

/// Der analytische Erwartungswert eines Laufs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Erwartung {
    /// Der erwartete ausgeteilte Schaden.
    pub schaden: f64,
    /// Der erlittene Schaden — in beiden Randläufen derselbe, also auch der
    /// Erwartungswert.
    pub erlitten: f64,
}

/// Wie viele Einträge das Protokoll eines Randlaufs hält.
///
/// Ein Kampf über 60 Sekunden schreibt einige hundert; die Prüfbosse mit 300
/// Sekunden um zweitausend. Läuft es über, gibt es keinen Wert
/// ([`Verweigerung::ProtokollZuKurz`]) — langsam, nicht falsch.
pub const PROTOKOLL: usize = 1 << 13;

/// Die beiden Randläufe: keiner kritet, jeder kritet.
///
/// `purity = 1000` heißt `crit_chance() == 1.0`, und
/// [`crate::rng::RandomStream::chance`] antwortet dann `true` **ohne zu
/// ziehen**. Beide Läufe sind damit deterministisch — und `crit_factor()` hängt
/// nicht an PURITY, sondern an YIELD, bleibt also gleich.
///
/// Gebaut wird der Kampf aus derselben Konfiguration wie jeder Balance-Lauf
/// (`balance::kampf_konfiguration`), nur mit Protokoll.
///
/// # Errors
///
/// [`Verweigerung::ProtokollZuKurz`], wenn ein Lauf mehr schreibt als
/// [`PROTOKOLL`].
///
/// # Panics
///
/// Wenn ein Randlauf gewürfelt hat — dann stimmt `zieht_bei` nicht mehr mit
/// [`crate::rng::RandomStream::chance`] überein.
pub fn raender(config: &BalanceConfig, apl: &Apl) -> Result<(Rand, Rand), Verweigerung> {
    Ok((randlauf(config, apl, 0)?, randlauf(config, apl, 1000)?))
}

fn randlauf(config: &BalanceConfig, apl: &Apl, purity: u32) -> Result<Rand, Verweigerung> {
    let mut worn = config.worn;
    worn.purity = purity;
    let rand_config = BalanceConfig { worn, ..*config };
    let mut fight = Fight::new(crate::fight::FightConfig {
        log_capacity: PROTOKOLL,
        ..crate::balance::kampf_konfiguration(&rand_config, config.base_seed)
    });
    fight.reset(config.base_seed);
    let result = fight.run_apl(apl);
    if let Some(s) = crate::fortschritt::strang() {
        s.kampf();
    }
    assert_eq!(
        result.random_draws, 0,
        "ein Randlauf mit PURITY {purity} hat {}-mal gezogen",
        result.random_draws
    );
    if fight.log().overflowed() {
        return Err(Verweigerung::ProtokollZuKurz {
            geschrieben: fight.log().written(),
        });
    }
    Ok(Rand {
        schaden: result.damage,
        erlitten: fight.damage_taken(),
        eintraege: fight.log().entries(),
    })
}

/// Prüft, dass die beiden Randläufe **denselben Kampf** gefochten haben —
/// bis auf die Beträge der Treffer.
///
/// Verglichen wird Eintrag für Eintrag, ohne die Sieg-Einträge (der Sieg ist
/// das, was der Krit verschieben darf). Zeitpunkt, Art und Zauber müssen
/// übereinstimmen; ein Treffer muss zu seinem Gegenstück passen
/// (`mit ≈ ohne · krit_faktor`, bis auf die Rundung je Teilbetrag); jeder
/// andere Eintrag muss gleich sein.
///
/// **Das ist eine Nachprüfung, kein Beweis.** Zwei Ränder, die übereinstimmen,
/// belegen nicht, dass jeder der `2ⁿ` Ausgänge dieselbe Trefferfolge hat. Sie
/// fangen aber genau die Fälle, die eine Ableitung übersieht — eine Phase, die
/// früher fällt, einen Schlag, der später kommt, eine Verstärkung, die an
/// einem verschobenen Entscheidungspunkt abläuft.
///
/// # Errors
///
/// [`Verweigerung::Abweichung`] mit dem ersten abweichenden Eintrag.
pub fn randabgleich(ohne: &Rand, mit: &Rand, krit_faktor: f64) -> Result<(), Verweigerung> {
    let abweichung = |eintrag, grund| Err(Verweigerung::Abweichung { eintrag, grund });
    if ohne.erlitten != mit.erlitten {
        return abweichung(0, "der erlittene Schaden ist verschieden");
    }
    let ohne_sieg = |r: &Rand| -> Vec<Entry> {
        r.eintraege
            .iter()
            .copied()
            .filter(|e| !ist_sieg(e.kind))
            .collect()
    };
    let (a, b) = (ohne_sieg(ohne), ohne_sieg(mit));
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        if x.time != y.time {
            return abweichung(i, "der Zeitpunkt ist verschieden");
        }
        if x.kind != y.kind {
            return abweichung(i, "die Art ist verschieden");
        }
        if x.spell != y.spell {
            return abweichung(i, "der Zauber ist verschieden");
        }
        if traegt_den_betrag(x.kind) {
            // Ein Wurf kann aus zwei gerundeten Teilen bestehen (CONDENSE
            // verdoppelt); je Teil weicht die Rundung um höchstens einen
            // halben Punkt vor und nach dem Faktor ab.
            let toleranz = 2.0 * krit_faktor + 1.0;
            if (f64::from(y.value) - f64::from(x.value) * krit_faktor).abs() > toleranz {
                return abweichung(i, "der Betrag passt nicht zum Krit-Faktor");
            }
        } else if x.value != y.value {
            return abweichung(i, "der Wert ist verschieden");
        }
    }
    if a.len() != b.len() {
        return abweichung(
            a.len().min(b.len()),
            "die Zahl der Einträge ist verschieden",
        );
    }
    Ok(())
}

/// Ist das der Sieg-Eintrag — der einzige, den der Krit verschieben darf?
const fn ist_sieg(art: LogKind) -> bool {
    match art {
        LogKind::Kill => true,
        LogKind::Cast
        | LogKind::Damage
        | LogKind::DotTick
        | LogKind::Spend
        | LogKind::BossPhase
        | LogKind::BuffExpire
        | LogKind::BossEnergy
        | LogKind::BossTimeline
        | LogKind::PlayerDamage
        | LogKind::PlayerDeath
        | LogKind::InterruptLanded
        | LogKind::ShieldAbsorb
        | LogKind::Healed
        | LogKind::ConvertedToShield
        | LogKind::ControlSpent => false,
    }
}

/// Trägt dieser Eintrag einen Betrag, den der Krit skaliert?
///
/// Ohne Auffangzweig: Ein neuer Eintrag muss hier sagen, ob sein Wert am Krit
/// hängt. Ein `_ => false` verlangte von einem neuen Schadenseintrag
/// Gleichheit — der Abgleich verweigerte dann immer, ohne dass es auffiele.
const fn traegt_den_betrag(art: LogKind) -> bool {
    match art {
        LogKind::Damage | LogKind::DotTick => true,
        // Die Umwandlung skaliert mit dem Treffer, darf es hier aber nicht:
        // Wer umwandelt, ändert den Schutz — und den liest der Boss beim
        // nächsten Schlag. Eine Abweichung ist dort ein Befund.
        LogKind::ConvertedToShield
        | LogKind::Cast
        | LogKind::Spend
        | LogKind::Kill
        | LogKind::BossPhase
        | LogKind::BuffExpire
        | LogKind::BossEnergy
        | LogKind::BossTimeline
        | LogKind::PlayerDamage
        | LogKind::PlayerDeath
        | LogKind::InterruptLanded
        | LogKind::ShieldAbsorb
        | LogKind::Healed
        | LogKind::ControlSpent => false,
    }
}

/// Die Gewichtung selbst — **eine** Stelle, zwei Verwendungen.
///
/// Steht hier und nicht inline, damit der Nachweis in den Prüffällen dieselbe
/// Rechnung prüft, die im Betrieb läuft. Ein Nachbau im Testmodul bliebe grün,
/// wenn jemand diese Zeile ändert — er antwortete plausibel aus einer zweiten
/// Quelle.
#[must_use]
pub fn gewichtet(p: f64, mit_krit: f64, ohne_krit: f64) -> f64 {
    p * mit_krit + (1.0 - p) * ohne_krit
}

/// Der analytische Erwartungswert eines Laufs (#920).
///
/// Modellanalytisch, als binary64-Punktwert — kein numerisches Zertifikat nach
/// #965 und keine Monte-Carlo-Schätzung.
///
/// # Errors
///
/// [`Verweigerung`], wenn die Aufteilung den Weg nicht trägt oder die
/// Randläufe abweichen. Dann gibt es **keinen** Wert, auch keinen ungefähren.
pub fn erwartung(
    config: &BalanceConfig,
    apl: &Apl,
    aufteilung: &StochastischeAufteilung,
) -> Result<Erwartung, Verweigerung> {
    // Nicht `analytisch()`: Auch ein Lauf, in dem nichts zieht, hat einen
    // Erwartungswert, und die Randläufe treffen ihn — bei PURITY 0 der ohne,
    // bei 1000 der mit Krit. Die Durchgangswahl fragt `analytisch()`, weil sie
    // dort ohnehin einen Kampf wählt.
    if !aufteilung.gezogen.ist_leer() {
        return Err(Verweigerung::Gezogen);
    }
    let (ohne, mit) = raender(config, apl)?;
    randabgleich(&ohne, &mit, config.worn.crit_factor())?;
    let p = config.worn.crit_chance();
    Ok(Erwartung {
        schaden: gewichtet(p, f64::from(mit.schaden), f64::from(ohne.schaden)),
        erlitten: f64::from(ohne.erlitten),
    })
}

/// Modellanalytischer Erwartungswert des Schadens, als binary64-Punktwert.
///
/// Das ist kein numerisch exaktes Ergebnis und kein Fehlerzertifikat nach
/// #965: Diese Funktion kombiniert zwei Randläufe; sie ist keine aus
/// `Verteilung` abgeleitete Statistik. Die dortigen `*_grenze`-Methoden
/// zertifizieren ausschließlich das feste Trefferfolgenmodell.
///
/// Gibt `None` zurück, wenn die Aufteilung für den erwarteten Schaden eine
/// Quelle zieht — an einem Boss mit Lebenspunkt-Phasen, mit Abbruch beim Sieg
/// (gemessen an `step1-good`: keiner kritet 4240, alle kriten 4230, **weniger**,
/// weil der Kampf eher endet), mit einer Zeile, die den Fortschritt liest —
/// oder wenn die Randläufe abweichen.
#[must_use]
pub fn erwarteter_schaden(config: &BalanceConfig, apl: &Apl) -> Option<f64> {
    let aufteilung = StochastischeAufteilung::fuer(config, apl, Ergebnisgroesse::ErwarteterSchaden);
    erwartung(config, apl, &aufteilung).ok().map(|e| e.schaden)
}

// ------------------------------------------------- der Schalter

thread_local! {
    /// Ob die Durchgangswahl dieses Strangs ausintegrieren darf.
    static AUSINTEGRATION: Cell<bool> = const { Cell::new(true) };
    /// Wie oft der Randabgleich dieses Strangs einen Wert verweigert hat.
    static VERWEIGERT: Cell<u64> = const { Cell::new(0) };
}

/// Darf die Durchgangswahl in diesem Strang den analytischen Weg nehmen?
///
/// Voreingestellt ja. Abgeschaltet nur in [`ohne_ausintegration`] — für den
/// Bezugslauf derselben Suche über die Stichprobe.
#[must_use]
pub fn ausintegration_erlaubt() -> bool {
    AUSINTEGRATION.with(Cell::get)
}

/// Rechnet `f` ohne analytischen Weg, in **diesem** Strang.
///
/// # Warum strang-lokal und nicht in `BalanceConfig`
///
/// Der Weg ändert nicht, WAS gerechnet wird — der erwartete Schaden bleibt der
/// erwartete Schaden —, sondern WIE. `BalanceConfig` ist Teil der Identität
/// jedes gespeicherten Ergebnisses (`speicher::zerlege`); ein Schalter dort
/// gäbe zwei gleichen Fragen verschiedene Schlüssel. Derselbe Schnitt wie bei
/// [`crate::drosselung`] und [`crate::fortschritt`].
///
/// Die Wahl fällt im Strang, der die Messung anstößt, nicht in den Helfern
/// eines parallelen Laufs — wer in `f` selbst Stränge startet, die messen,
/// schaltet sie damit nicht ab.
pub fn ohne_ausintegration<T>(f: impl FnOnce() -> T) -> T {
    struct Zurueck(bool);
    impl Drop for Zurueck {
        fn drop(&mut self) {
            AUSINTEGRATION.with(|a| a.set(self.0));
        }
    }
    let _zurueck = Zurueck(AUSINTEGRATION.with(|a| a.replace(false)));
    f()
}

/// Wie oft der Randabgleich in diesem Strang einen Wert verweigert hat.
///
/// Jede Verweigerung ist ein Lauf, für den die Maske „unberührt" sagte und die
/// Ränder etwas anderes zeigten. Der Aufrufer fällt dann auf die Stichprobe
/// zurück; die Zahl gehört in jeden Bericht, der den analytischen Weg nutzt.
#[must_use]
pub fn verweigerungen() -> u64 {
    VERWEIGERT.with(Cell::get)
}

/// Eine Verweigerung zählen.
pub(crate) fn verweigerung_zaehlen() {
    VERWEIGERT.with(|v| v.set(v.get() + 1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apl::{parse, AplName};
    use crate::balance::run;
    use crate::{compute_damage, DamageFactors, NEUTRAL};

    /// Der Schaden eines Laufs, in dem **kein** Treffer kritet — über
    /// [`run`], also auf dem Weg jedes Balance-Laufs, nicht über
    /// [`raender`]. Die Randläufe dort müssen dieselbe Zahl treffen.
    fn ohne_krit(config: &BalanceConfig, apl: &Apl) -> f64 {
        let mut worn = config.worn;
        worn.purity = 0;
        run(&BalanceConfig { worn, ..*config }, apl).damage.mean
    }

    /// Der Schaden eines Laufs, in dem **jeder** Treffer kritet.
    fn mit_krit(config: &BalanceConfig, apl: &Apl) -> f64 {
        let mut worn = config.worn;
        worn.purity = 1000;
        run(&BalanceConfig { worn, ..*config }, apl).damage.mean
    }

    /// Schadensfaktoren mit einem einzigen gesetzten Krit-Wert.
    ///
    /// Heißt bewusst nicht `mit_krit` — so hieße sie wie die Modulfunktion, die
    /// einen ganzen Lauf fährt.
    fn krit_faktor(crit: f64) -> DamageFactors {
        DamageFactors { crit, ..NEUTRAL }
    }

    const GOOD: &str = include_str!("../apl/step1-good.apl");

    fn gute_liste() -> Apl {
        parse(AplName("step1-good"), GOOD).expect("die eingebaute Liste ist gültig")
    }

    fn konfig(purity: u32) -> BalanceConfig {
        let mut worn = crate::gear::WornValues::none();
        worn.purity = purity;
        BalanceConfig {
            iterations: 200,
            base_seed: 17,
            threads: 1,
            worn,
            // **Ohne das trägt die Rechnung nicht** — und das ist gemessen, nicht
            // vorsichtshalber gesetzt: Mit `stop_on_kill` ergab der Lauf mit Krit
            // 4230 gegen 4240 ohne, weil der frühere Sieg den Schaden abschneidet.
            stop_on_kill: false,
            ..BalanceConfig::default()
        }
    }

    /// Die Konfiguration, die die Voraussetzung **verletzt** — und erkannt wird.
    #[test]
    fn mit_abbruch_beim_sieg_gibt_es_keinen_erwartungswert() {
        let apl = gute_liste();
        let mit_abbruch = BalanceConfig {
            stop_on_kill: true,
            ..konfig(500)
        };
        assert!(
            erwarteter_schaden(&mit_abbruch, &apl).is_none(),
            "mit Abbruch beim Sieg hängt die Kampfdauer am Krit — dann darf hier \
             kein Wert herauskommen, auch kein ungefährer"
        );
        assert!(
            erwarteter_schaden(&konfig(500), &apl).is_some(),
            "Trägerzusicherung: ohne Abbruch liefert dieselbe Konfiguration einen \
             Wert — sonst prüfte der Fall nur, dass die Funktion immer None sagt"
        );
    }

    /// Die Voraussetzung wird **gefragt**, nicht angenommen.
    #[test]
    fn die_voraussetzung_wird_aus_der_beschreibung_abgeleitet() {
        // Heute trägt kein würfelnder Knoten eine Verzweigung — sonst wäre der
        // analytische Weg gar nicht anwendbar und alle Fälle unten sinnlos.
        assert!(
            !krit_beruehrt_den_kontrollfluss(EFFECT_NODES),
            "Trägerzusicherung: die Voraussetzung gilt im heutigen Bestand"
        );

        // Und die Gegenprobe: Ein konstruierter Knoten, der würfelt UND
        // verzweigt, muss sie kippen. Ohne ihn wüsste niemand, ob die Funktion
        // überhaupt „nein" sagen kann.
        let mut verzweigend = EFFECT_NODES
            .iter()
            .find(|n| !n.randomness.ist_deterministisch())
            .copied()
            .expect("es gibt würfelnde Knoten");
        verzweigend.op = crate::effects::EffectOp::ApplyBuff;
        assert!(
            krit_beruehrt_den_kontrollfluss(&[verzweigend]),
            "ein würfelnder Knoten, der Aktionen verändert, MUSS die \
             Voraussetzung kippen"
        );
    }

    /// **ERLÄUTERUNG, KEIN WÄCHTER — bitte nicht mitzählen.**
    ///
    /// Dieser Fall prüft die Formel gegen ihre eigenen Bestandteile:
    /// `e == 0,5·b + 0,5·a`, wobei `e` genau so gebildet wird. Er **kann nicht
    /// scheitern**, solange die Gewichtung symmetrisch bleibt — in der
    /// Mutationsprobe mit vertauschten Gewichten blieb er grün, während die
    /// beiden anderen rot wurden.
    ///
    /// Er steht hier, weil er die Rechnung dokumentiert und die Randbedingung
    /// festhält (der Erwartungswert liegt echt zwischen den beiden Läufen).
    /// Zugesichert wird die Richtigkeit woanders: von
    /// [`Self::die_vollstaendige_aufzaehlung_stimmt_bitgleich_mit_der_formel`]
    /// und vom Vergleich gegen gewürfelte Läufe.
    #[test]
    fn der_erwartungswert_liegt_exakt_auf_der_gewichtung() {
        let apl = gute_liste();
        let config = konfig(500);

        let a = ohne_krit(&config, &apl);
        let b = mit_krit(&config, &apl);
        let e = erwarteter_schaden(&config, &apl).expect("die Voraussetzung gilt");

        assert!(
            b > a,
            "Trägerzusicherung: mit Krit muss mehr Schaden entstehen ({b} gegen {a}) \
             — sonst unterscheiden sich die beiden Randläufe gar nicht"
        );
        assert!(
            e > a && e < b,
            "der Erwartungswert muss zwischen den Rändern liegen: {a} < {e} < {b}"
        );
        assert_eq!(
            e.to_bits(),
            (0.5 * b + 0.5 * a).to_bits(),
            "bei halber Chance ist es genau die Mitte"
        );
    }

    /// **Der strenge Nachweis: vollständige Aufzählung statt Stichprobe.**
    ///
    /// Bei `k` Krit-Würfen gibt es 2^k Ausgänge. Jeden einzeln rechnen, mit
    /// seiner Wahrscheinlichkeit gewichten, aufsummieren — das **ist** der
    /// gewürfelte Erwartungswert, nicht seine Schätzung. Dagegen ist
    /// Bitgleichheit eine sinnvolle Forderung; gegen eine Stichprobe wäre sie
    /// es nicht.
    ///
    /// Warum das nötig ist, obwohl der Monte-Carlo-Vergleich weiter unten
    /// grün ist: Eine Schranke von „vier Standardfehlern" ließe einen
    /// Rechenfehler von einem Promille unentdeckt, und sie wird **still
    /// schwächer**, sobald jemand die Durchgangszahl senkt. Eine Zusicherung,
    /// deren Wirksamkeit an einer Zahl hängt, die niemand mehr ansieht, ist
    /// keine.
    #[test]
    fn die_vollstaendige_aufzaehlung_stimmt_bitgleich_mit_der_formel() {
        // Realistische Basisschäden, ungerade Werte dabei: Bei glatten Zahlen
        // fiele die Rundung nicht an, und genau an ihr hängt die Sache.
        let basen: [u32; 8] = [7, 13, 40, 41, 99, 100, 137, 250];
        let p: f64 = 0.25;
        let krit: f64 = 1.5;

        let k = basen.len();
        // **Die Voraussetzungen stehen als Zusicherung im Fall, nicht im
        // Kommentar** — er soll rot werden, wenn jemand sie ihm wegzieht.
        assert!(
            k >= 2,
            "mit weniger als zwei Würfen gäbe es keinen Ausgang, in dem MEHRERE \
             Treffer kriten — und genau dort unterscheiden sich Marginalisieren \
             und Erwartungswert-Einsetzen"
        );
        assert!(
            k <= 16,
            "2^{k} Ausgänge wären nicht mehr vertretbar zu rechnen"
        );

        let ohne: Vec<f64> = basen
            .iter()
            .map(|b| f64::from(compute_damage(*b, &NEUTRAL)))
            .collect();
        let mit: Vec<f64> = basen
            .iter()
            .map(|b| f64::from(compute_damage(*b, &krit_faktor(krit))))
            .collect();

        assert!(
            ohne.iter().zip(&mit).any(|(a, b)| a != b),
            "Trägerzusicherung: der Krit ändert an mindestens einem Treffer etwas"
        );

        // Alle 2^k Ausgänge, jeder mit seiner Wahrscheinlichkeit.
        let mut aufgezaehlt = 0.0f64;
        let mut wahrscheinlichkeit_gesamt = 0.0f64;
        let mut ausgaenge_mit_mehreren_krits = 0;
        for maske in 0u32..(1u32 << k) {
            let mut schaden = 0.0f64;
            let mut kritend = 0u32;
            for (i, (a, b)) in ohne.iter().zip(&mit).enumerate() {
                if maske & (1 << i) == 0 {
                    schaden += a;
                } else {
                    schaden += b;
                    kritend += 1;
                }
            }
            if kritend >= 2 {
                ausgaenge_mit_mehreren_krits += 1;
            }
            let w = p.powi(kritend as i32) * (1.0 - p).powi((k as u32 - kritend) as i32);
            wahrscheinlichkeit_gesamt += w;
            aufgezaehlt += w * schaden;
        }

        assert!(
            ausgaenge_mit_mehreren_krits > 0,
            "Trägerzusicherung: es gibt Ausgänge, in denen MEHRERE Treffer kriten \
             ({ausgaenge_mit_mehreren_krits}) — sonst prüfte der Fall den \
             Sonderfall, in dem beide Rechenwege ohnehin übereinstimmen"
        );

        // **Dieselbe Funktion, die auch im Betrieb rechnet** — kein Nachbau.
        // Sonst bliebe dieser Fall grün, wenn jemand die Gewichtung ändert.
        let formel = gewichtet(p, mit.iter().sum::<f64>(), ohne.iter().sum::<f64>());

        println!(
            "aufgezählt {aufgezaehlt:.12}, Formel {formel:.12}, \
             Differenz {:.3e}, Wahrscheinlichkeitssumme {wahrscheinlichkeit_gesamt:.12}",
            (aufgezaehlt - formel).abs()
        );

        assert_eq!(
            aufgezaehlt.to_bits(),
            formel.to_bits(),
            "die vollständige Aufzählung ({aufgezaehlt}) und die Formel ({formel}) \
             weichen ab — die Gewichtung trifft den gewürfelten Erwartungswert nicht"
        );
    }

    /// **Der einzige unabhängige Nachweis: gegen wirklich gewürfelte Läufe.**
    ///
    /// Die Fälle darüber prüfen die Formel gegen ihre eigenen Bestandteile —
    /// `e == 0.5·b + 0.5·a` kann gar nicht scheitern, solange `e` so gebildet
    /// wird. Erst der Vergleich mit Monte-Carlo kann zeigen, dass die Gewichtung
    /// die *richtige* ist.
    ///
    /// **Bitgleich ist dieser Vergleich nicht, und das ist keine Schwäche der
    /// Rechnung.** Der analytische Wert ist der exakte Erwartungswert; ein
    /// Monte-Carlo-Lauf ist eine Stichprobe daraus und streut um ihn. Bitgleich
    /// wären sie erst bei unendlich vielen Durchgängen. Geprüft wird deshalb,
    /// dass die Abweichung im Rahmen des **Standardfehlers** bleibt — der Größe,
    /// die der Läufer selbst mitliefert.
    #[test]
    fn der_analytische_wert_trifft_das_gewuerfelte_mittel_im_fehlerrahmen() {
        let apl = gute_liste();
        // **Nicht 500.** Bei halber Chance ist die Gewichtung symmetrisch, und
        // ein vertauschtes `p`/`1−p` käme auf denselben Wert — der Fall könnte
        // den naheliegendsten Fehler nicht fangen. Mit 250 ist er unsymmetrisch.
        let config = BalanceConfig {
            iterations: 2000,
            ..konfig(250)
        };

        let gewuerfelt = run(&config, &apl).damage;
        let analytisch = erwarteter_schaden(&config, &apl).expect("die Voraussetzung gilt");

        assert!(
            gewuerfelt.stddev > 0.0,
            "Trägerzusicherung: die gewürfelten Durchgänge STREUEN wirklich \
             (stddev {}) — ohne Streuung verglichen wir zwei deterministische \
             Läufe und der Fall wäre wertlos",
            gewuerfelt.stddev
        );

        let abstand = (gewuerfelt.mean - analytisch).abs();
        assert!(
            abstand < 4.0 * gewuerfelt.stderr,
            "der analytische Wert liegt {abstand:.2} neben dem gewürfelten Mittel \
             ({} gegen {analytisch}), erlaubt sind vier Standardfehler ({:.2}) — \
             das ist keine Streuung mehr, sondern ein Fehler in der Gewichtung",
            gewuerfelt.mean,
            4.0 * gewuerfelt.stderr
        );

        println!(
            "gewürfelt {:.2} ± {:.2} (stderr), analytisch {analytisch:.2}, \
             Abstand {abstand:.2}",
            gewuerfelt.mean, gewuerfelt.stderr
        );
    }

    /// An den Rändern muss der analytische Wert die Randläufe **treffen**.
    #[test]
    fn bei_null_und_voll_trifft_er_die_raender_bitgleich() {
        let apl = gute_liste();

        let ohne = konfig(0);
        let e_ohne = erwarteter_schaden(&ohne, &apl).expect("gilt");
        assert_eq!(
            e_ohne.to_bits(),
            ohne_krit(&ohne, &apl).to_bits(),
            "bei p = 0 ist der Erwartungswert der Lauf ohne Krit"
        );

        let voll = konfig(1000);
        let e_voll = erwarteter_schaden(&voll, &apl).expect("gilt");
        assert_eq!(
            e_voll.to_bits(),
            mit_krit(&voll, &apl).to_bits(),
            "bei p = 1 ist er der Lauf mit Krit"
        );
    }
}
