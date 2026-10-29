//! Die Suche nach der besten Rotation (Issue #237).
//!
//! # Warum es sie gibt
//!
//! [`crate::best_build`] sucht den besten **Aufbau** und begründet das so:
//!
//! > „Den **besten** Aufbau suchen, statt einen anzunehmen. Ein beliebiger
//! > Aufbau als Gegenspieler misst nicht die Frage, sondern die Wahl des
//! > Aufbaus."
//!
//! Wort für Wort gilt das für die Rotation. Bis hierher wurde sie angenommen:
//! Die Balance-Messungen liefen gegen `step1-good.apl` und `step3-good.apl`,
//! beide von Hand geschrieben. Jede Zahl daraus ist deshalb eine **Untergrenze**
//! — sie sagt, was mit DIESER Liste erreichbar war, nicht was erreichbar ist.
//!
//! # Der Suchraum, und wo er beschnitten wird
//!
//! Fünfzehn Zauber in einer Prioritätenliste sind 15! = 1.307.674.368.000
//! Reihenfolgen. Erschöpfend geht nichts; die Frage ist nicht „wie schnell",
//! sondern **was genau gesucht wird**. Drei Schnitte:
//!
//! | Schnitt | Was |
//! | --- | --- |
//! | 1 | Bedingungen werden **abgeleitet**, nicht gesucht — dieses Modul |
//! | 2 | Nur die Reihenfolge wird gesucht, und zwar gierig |
//! | 3 | Gemessen wird gegen mehrere Bosse, sonst misst die Suche den Boss |
//!
//! # Warum Bedingungen abgeleitet werden und nicht gesucht
//!
//! Das Argument ist **nicht** der Aufwand, sondern die Überanpassung. Eine
//! freie Bedingungssuche über [`crate::apl::Value`] mal [`crate::apl::Compare`]
//! mal beliebige Zahlen fände Zeilen wie `if time > 17340`, die zufällig zur
//! Messung passen und sonst nichts.
//!
//! > Eine Suche, die das darf, findet verlässlich etwas — und zwar immer. Sie
//! > liefert nie ein leeres Ergebnis, und genau das macht sie wertlos: Man kann
//! > sie nicht widerlegen.
//!
//! Abgeleitet wird stattdessen aus dem, was der Zauber **selbst** mitbringt.
//! Jede erzeugte Zeile ist damit eine, die ein Mensch auch schreiben würde.

use crate::apl::{Apl, Compare, Condition, Rule, Term, Value};
use crate::balance::BalanceConfig;
use crate::data::{BuffTarget, ControlEffect, Spell, MAX_LEVEL, SPELLS, SPELL_COUNT};
use crate::time::{Millis, GCD_MS};

/// Ein Kandidat für eine Zeile der Prioritätenliste.
///
/// Er trägt seine Herkunft mit — der Bericht aus AC4 muss sagen können, wie
/// viele Zeilen aus welcher Regel entstanden sind, und ohne diese Angabe wäre
/// „elf Kandidaten" eine Zahl ohne Aussage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Der Zauber, um den es geht.
    pub spell: usize,
    /// Die Bedingung, unter der er gewirkt wird.
    pub condition: Condition,
    /// Woraus diese Zeile abgeleitet wurde.
    pub origin: Origin,
}

/// Woraus eine Kandidatenzeile entstanden ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Ohne Bedingung — die Abklingzeit reguliert den Zauber selbst.
    Always,
    /// Aus dem Brand-Effekt: nachlegen, bevor er ausläuft.
    DotRefresh,
    /// Aus der Bonus-Schwelle eines Entladers.
    SpenderThreshold,
    /// Aus der Verhängnis-Uhr des Bosses (Issue #1088).
    ///
    /// # Warum diese Art anders entsteht als alle davor
    ///
    /// Die übrigen Kandidatenzeilen leiten ihre Schwelle aus dem **Zauber** ab
    /// — `spender.bonus_threshold` steht im Zauber, `GCD_MS` ist eine
    /// Konstante der Regeln. Diese hier braucht den **Boss**: `threshold`,
    /// `amount` und `clear_amount` stehen dort, und sie unterscheiden sich je
    /// Boss.
    ///
    /// Deshalb entsteht sie nicht in [`candidates_for`], sondern erst in der
    /// Suche, wo der Boss über `BalanceConfig` bekannt ist.
    DoomClear,
    /// Aus dem Fenster einer Verstaerkung (Issue #553).
    ///
    /// # Warum diese Art gefehlt hat
    ///
    /// Bis #553 konnte der Parser keine Buff-Fenster lesen, und die Suche
    /// baut ihre Kandidatenzeilen aus derselben Werteliste. Sie konnte
    /// `kindle if buff.flashpoint.active` also gar nicht bilden — eine
    /// Verstaerkung auf eine Nebengroesse war fuer sie unsichtbar.
    ///
    /// Das ist der Grund, warum #550 unter Vorbehalt stand: Ein Teil des
    /// Befundes "drei von fuenf Schulen kommen in keiner besten Rotation vor"
    /// konnte ein Werkzeug-Befund sein statt ein Balance-Befund.
    BuffWindow,
    /// Aus dem Fenster eines laufenden Boss-Zaubers (Issue #797).
    ///
    /// # Warum diese Art gefehlt hat
    ///
    /// Dieselbe Geschichte wie bei [`Origin::BuffWindow`], eine Ebene tiefer:
    /// Bis #797 kannte die Sprache keinen laufenden Boss-Zauber, also konnte
    /// die Suche keine Zeile bilden, die auf einen antwortet. Reaktionsspiel
    /// war für sie nicht darstellbar — nicht schwach bewertet, sondern
    /// unsichtbar.
    ///
    /// Gemessen hing #472 daran: An einem Boss mit einem Schlag, den man
    /// abwenden MUSS, wählte der Balance-Läufer bei keinem Gewicht einen
    /// FIXATION-Zauber. Der Befund „die Schulen liegen 67 % auseinander" war
    /// damit zum Teil ein Werkzeug-Befund, kein Balance-Befund — genau der
    /// Vorbehalt, unter dem schon #550 stand.
    BossCastWindow,
    /// Aus dem Fenster vor dem nächsten Boss-Schlag (Issue #797).
    ///
    /// # Warum [`Origin::BossCastWindow`] dafür nicht reicht
    ///
    /// Der beschreibt den *laufenden* Zauber, und PIN wirkt auf den
    /// **nächsten**. Bis #797 stand PIN trotzdem dort — die erzeugte Zeile
    /// `pin if boss.casting` war gültig, wählbar und nutzlos: Sie feuert genau
    /// dann, wenn schon etwas läuft, und laut Zauberdaten verpufft PIN an
    /// einem ohnehin unterbrechbaren Eintrag.
    ///
    /// Das ist der teurere Fehler von beiden. Eine fehlende Zeile fällt beim
    /// Zählen auf; eine vorhandene, die nichts ausrichtet, sieht in jedem
    /// Bericht aus wie Abdeckung.
    NextStrikeWindow,
}

/// Wie früh vor einem Schlag die Suche PIN einplant.
///
/// **Gemessen, nicht gewählt** — `pin_fenster_ist_gemessen` in
/// `sim/tests/next_strike_in.rs` fährt die Reihe und hält fest, welche Werte
/// eine überlebende Rotation finden.
///
/// Die Zahl kann nicht beliebig klein sein: Die Prioritätenliste wird an
/// GCD-Grenzen ausgewertet, und ein Fenster unter einem GCD kann zwischen zwei
/// Auswertungen komplett verstreichen. Sie kann auch nicht beliebig groß sein —
/// PIN kostet drei Kontroll-Punkte und einen GCD, und `pin_window` hält bis
/// zum nächsten Eintrag, gleich welcher es ist. Wer zu früh drückt, bezahlt
/// den Zauber an einen Eintrag, der ihn nicht braucht.
pub const NEXT_STRIKE_FENSTER_MS: Millis = 3000;

impl Origin {
    /// Alle Herkunftsarten, in fester Reihenfolge.
    ///
    /// # Wozu die Liste gebraucht wird
    ///
    /// Der Bericht aus AC4 zählt die Kandidatenzeilen je Herkunft. Bis #797
    /// führte er dafür eine **eigene**, wörtlich hingeschriebene Aufzählung —
    /// und der Kommentar daneben sagte den Fehler voraus, den sie erlaubt:
    /// „Wer einen Herkunfts-Typ in `Origin` ergänzt, bekommt hier keine Zeile
    /// und merkt es nicht, weil die Zahl einfach fehlt."
    ///
    /// Genau das ist beim Bau von [`Origin::NextStrikeWindow`] eingetreten.
    /// Der Übersetzer fing das `match` in der Zählung — die Aufzählung in der
    /// Ausgabe fing er nicht, weil ein Feld-Literal keine Vollständigkeit
    /// kennt.
    ///
    /// # Was diese Liste absichert und was nicht
    ///
    /// [`Origin::ordnung`] ist ein exhaustives `match`: Wer eine Art ergänzt,
    /// **muss** ihr dort einen Platz geben, sonst übersetzt es nicht. Dass sie
    /// auch hier steht, erzwingt der Übersetzer **nicht** — dafür prüft der
    /// Bericht seine eigene Summe gegen die Gesamtzahl der Zeilen, und
    /// `jede_herkunft_steht_in_der_liste` prüft es unabhängig davon.
    pub const ALL: &'static [Origin] = &[
        Origin::Always,
        Origin::DotRefresh,
        Origin::SpenderThreshold,
        Origin::BuffWindow,
        Origin::BossCastWindow,
        Origin::NextStrikeWindow,
        Origin::DoomClear,
    ];

    /// Der Platz dieser Art in [`Origin::ALL`].
    ///
    /// **Exhaustiv und ohne Auffang-Zweig**, aus demselben Grund wie
    /// `Value::is_time_dependent`: Ein `_ => 0` würde zwei Arten auf denselben
    /// Zähler legen, und der Bericht wäre still falsch statt laut kaputt.
    #[must_use]
    pub const fn ordnung(self) -> usize {
        match self {
            Self::Always => 0,
            Self::DotRefresh => 1,
            Self::SpenderThreshold => 2,
            Self::BuffWindow => 3,
            Self::BossCastWindow => 4,
            Self::NextStrikeWindow => 5,
            Self::DoomClear => 6,
        }
    }

    /// Kurzbezeichnung für den Bericht.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Always => "unbedingt",
            Self::DotRefresh => "Brand nachlegen",
            Self::SpenderThreshold => "Entlade-Schwelle",
            Self::BuffWindow => "Verstaerkungs-Fenster",
            Self::BossCastWindow => "Boss wirkt",
            Self::NextStrikeWindow => "Schlag steht bevor",
            Self::DoomClear => "Verhaengnis senken",
        }
    }
}

impl Candidate {
    /// Die Zeile als APL-Regel, wie die Suche sie misst.
    #[must_use]
    pub fn rule(&self) -> Rule {
        Rule {
            spell: self.spell,
            condition: self.condition.clone(),
            // Die Zeilennummer entsteht erst beim Zusammensetzen der Liste;
            // ein Kandidat für sich steht in keiner Datei.
            line: 0,
        }
    }
}

/// Eine Bedingung aus genau einem Vergleich.
fn single(left: Value, compare: Compare, right: Value) -> Condition {
    Condition {
        groups: vec![vec![Term {
            left,
            compare,
            right,
        }]],
    }
}

/// Die Kandidatenzeilen eines einzelnen Zaubers.
///
/// **Die unbedingte Zeile steht immer dabei**, auch wenn der Zauber eine
/// abgeleitete Bedingung hergibt. Sonst wäre die Bedingung erzwungen und die
/// Suche hätte an dieser Stelle nichts zu wählen — sie könnte nicht
/// herausfinden, dass ein Zauber besser ohne Bedingung läuft.
///
/// Das ist kein theoretischer Fall: `SMOULDER` mit Brand kann in einer
/// Rotation, in der ohnehin nichts anderes bereitsteht, unbedingt besser sein
/// als mit Nachlege-Bedingung.
#[must_use]
pub fn candidates_for(index: usize) -> Vec<Candidate> {
    let spell: &Spell = &SPELLS[index];

    // **Prüf-Zauber gehören nicht in die Rotationssuche.** Seit Issue #304
    // steht `test-cast` im Index — `unlockLevel: 101` bei `MAX_LEVEL = 100`,
    // also für keinen Spieler jemals erreichbar.
    //
    // Er kostete nicht nur Messungen (18 Kandidatenzeilen statt 17): Fände die
    // Suche ihn gut, empföhle sie eine Rotation, die niemand spielen kann.
    // Gemessen hat er die Zahlen nicht bewegt — 5960,0 und 31120,0 blieben —
    // aber „gewinnt heute nicht" ist keine Zusicherung für morgen.
    //
    // Die Grenze steht in den Daten, nicht hier: `MAX_LEVEL` entsteht in
    // `build.rs` aus den Stufendaten.
    if spell.unlock_level > MAX_LEVEL as u16 {
        return Vec::new();
    }

    let mut out = vec![Candidate {
        spell: index,
        condition: Condition::default(),
        origin: Origin::Always,
    }];

    // Ein Brand wird nachgelegt, bevor er ausläuft — nicht früher. Der Rest
    // eines GCD ist die kleinste Spanne, in der das noch rechtzeitig gelingt:
    // Wer später nachlegt, verliert Ticks; wer früher nachlegt, verschenkt
    // Laufzeit.
    if spell.dot.is_some() {
        out.push(Candidate {
            spell: index,
            condition: single(
                Value::DotRemains(index),
                Compare::Less,
                Value::Literal(i64::from(GCD_MS)),
            ),
            origin: Origin::DotRefresh,
        });
    }

    // Ein Entlader lohnt ab seiner Bonus-Schwelle. Die Zahl steht in den
    // Daten, nicht hier — deshalb ist das eine Ableitung und keine Suche.
    if let Some(spender) = spell.spender {
        out.push(Candidate {
            spell: index,
            condition: single(
                Value::Resource,
                Compare::GreaterEqual,
                Value::Literal(i64::from(spender.bonus_threshold)),
            ),
            origin: Origin::SpenderThreshold,
        });
    }

    // **Im Fenster einer Verstaerkung** (Issue #553).
    //
    // Die Zuordnung kommt aus den DATEN, nicht aus einer Liste: Eine
    // Verstaerkung auf `Resource` lohnt bei einem Zauber, der Vorrat gibt;
    // eine auf `Stack` bei einem, der Stapel legt. Wer einen Verstaerker
    // ergaenzt, bekommt die Kandidatenzeilen ohne Zutun.
    //
    // Der Verstaerker SELBST bekommt keine — `flashpoint if
    // buff.flashpoint.remains < 1` ist gemessen wirkungslos (1200 gegen 1200),
    // weil seine Abklingzeit laenger ist als sein Fenster. Eine Zeile, die
    // nichts aendert, kostet die Suche nur Zeit.
    for (verstaerker, spell_v) in SPELLS.iter().enumerate() {
        let Some(buff) = spell_v.buff else { continue };
        if spell_v.unlock_level > MAX_LEVEL as u16 || verstaerker == index {
            continue;
        }
        let passt = match buff.target {
            BuffTarget::Resource | BuffTarget::Cap => spell.resource_gain > 0,
            BuffTarget::Stack => spell.decay_stacks > 0,
            BuffTarget::Control => spell.control.is_some(),
            BuffTarget::Tick => spell.dot.is_some(),
            BuffTarget::CdReduce => spell.cooldown_reduction_ms > 0,
            BuffTarget::Convert => spell.conversion.is_some(),
            BuffTarget::Damage => spell.base_damage > 0,
        };
        if !passt {
            continue;
        }
        out.push(Candidate {
            spell: index,
            condition: single(
                Value::BuffActive(verstaerker),
                Compare::GreaterEqual,
                Value::Literal(1),
            ),
            origin: Origin::BuffWindow,
        });
    }

    // **Zeilen, die auf einen laufenden Boss-Zauber antworten** (Issue #797).
    //
    // Ohne sie kann die Suche den neuen Wert lesen, aber nie wählen: Ein Wert,
    // für den keine Kandidatenzeile existiert, taucht in keiner Rotation auf,
    // ganz gleich wie nützlich er wäre. Genau daran hing #472 — der
    // Balance-Läufer wählte bei keinem Gewicht einen FIXATION-Zauber, weil die
    // Bedingung fehlte, unter der sie sich lohnen.
    //
    // **Nur für Zauber, die an einem Cast überhaupt etwas ausrichten.** Ein
    // `kindle if boss.casting` wäre eine gültige Zeile und schlicht sinnlos:
    // Sie verengt den Füller auf ein Fenster, ohne dort etwas zu gewinnen. Drei
    // Sorten können es:
    //
    //   - der Unterbrecher (`interrupt`) beendet den Zauber
    //   - `SlowCast` (STALL, und über `AllThree` auch CAPSTONE) dehnt ihn
    //
    // `DelayStrike`, `HoldPhase` und `ConvertToDamage` stehen NICHT hier: Sie
    // wirken auf die Zeitleiste oder den Vorrat, nicht auf einen laufenden
    // Zauber. Eine Zeile `if boss.casting` würde sie auf ein Fenster
    // einschränken, das für sie nichts bedeutet.
    //
    // **`MakeInterruptible` (PIN) stand hier und war falsch** (#797, AC2). Der
    // Zauber wirkt laut Daten auf den NÄCHSTEN Eintrag, und traf er einen
    // ohnehin unterbrechbaren, verpufft er — `pin if boss.casting` feuerte
    // also genau dann, wenn PIN nichts ausrichtet. Die Zeile war formal
    // erfüllt und sachlich leer; sie steht jetzt weiter unten mit dem Wert,
    // der zu ihr passt.
    let antwortet_auf_cast = spell.interrupt
        || spell
            .control
            .is_some_and(|c| matches!(c.effect, ControlEffect::SlowCast | ControlEffect::AllThree));
    if antwortet_auf_cast {
        out.push(Candidate {
            spell: index,
            condition: single(Value::BossCasting, Compare::GreaterEqual, Value::Literal(1)),
            origin: Origin::BossCastWindow,
        });
    }

    // **Zeilen, die einen Schlag vorbereiten, bevor er fällt** (Issue #797).
    //
    // Nur `MakeInterruptible` kommt hierher, und `AllThree` ausdrücklich
    // NICHT: CAPSTONE ruft `SlowCast`, `DelayStrike` und `HoldPhase` auf
    // (`Fight::apply_control_effect`) — unterbrechbar macht es nichts. Der
    // Doc-Kommentar am Datentyp sagt „alle drei bisherigen Wirkungen", und wer
    // daraus die drei VOR `AllThree` stehenden liest, zählt `MakeInterruptible`
    // mit. Die Liste im Code ist die Quelle, nicht die Reihenfolge im `enum`.
    if spell
        .control
        .is_some_and(|c| c.effect == ControlEffect::MakeInterruptible)
    {
        out.push(Candidate {
            spell: index,
            condition: single(
                Value::BossNextStrikeIn,
                Compare::LessEqual,
                Value::Literal(i64::from(NEXT_STRIKE_FENSTER_MS)),
            ),
            origin: Origin::NextStrikeWindow,
        });
    }

    out
}

/// Alle Kandidatenzeilen über alle Zauber, in fester Reihenfolge.
///
/// Die Reihenfolge ist die der Zauberdaten, nicht die eines Containers mit
/// Streuung — GAME-DESIGN 18.4 verlangt Determinismus, und eine Suche, deren
/// Kandidatenliste zwischen zwei Läufen die Reihenfolge wechselt, liefert bei
/// gleichwertigen Kandidaten verschiedene Ergebnisse.
#[must_use]
pub fn all_candidates() -> Vec<Candidate> {
    (0..SPELL_COUNT).flat_map(candidates_for).collect()
}

// ---------------------------------------------------------------------------
// Schnitt 2: die Suche ueber die Reihenfolge
// ---------------------------------------------------------------------------

/// Eine gemessene Rotation.
#[derive(Debug, Clone)]
pub struct Measured {
    /// Die Liste, so wie sie gemessen wurde.
    pub apl: Apl,
    /// Mittlerer Schaden ueber alle Durchgaenge.
    pub mean: f64,
    /// Standardfehler des Mittelwerts.
    pub stderr: f64,
}

impl Measured {
    /// Ob diese Rotation die andere **messbar** schlaegt.
    ///
    /// Verglichen wird ueber die Standardfehler **beider** Messungen, nicht
    /// ueber die nackten Mittelwerte — dasselbe Vorgehen wie in
    /// [`crate::best_build::Candidate::clearly_better_than`]. Zwei Rotationen,
    /// deren Fehlerbalken sich ueberlappen, sind nicht unterscheidbar, und
    /// „die bessere gewinnt" waere dann eine Muenze.
    #[must_use]
    pub fn clearly_better_than(&self, other: &Self) -> bool {
        self.mean - other.mean > self.stderr + other.stderr
    }
}

/// Setzt eine Liste aus Kandidatenzeilen zusammen und misst sie.
///
/// **`stop_on_kill` steht auf `false`**, und das ist keine Nebensache:
/// `BalanceConfig` warnt ausdruecklich davor, die beiden Stellungen zu
/// verwechseln. Mit `true` endet der Durchgang beim Sieg, der Schaden ist
/// abgeschnitten, und zwei verschieden schnelle Rotationen rechnen verschieden
/// lang — ihre Zahlen waeren dann nicht vergleichbar. Eine Suche, die so misst,
/// waehlt die Rotation, die am frühesten aufhoert.
#[must_use]
pub fn measure(config: &BalanceConfig, name: &str, chosen: &[Candidate]) -> Measured {
    let apl = Apl {
        name: name.to_owned(),
        rules: chosen
            .iter()
            .enumerate()
            .map(|(i, k)| Rule {
                line: i + 1,
                ..k.rule()
            })
            .collect(),
        overdrive: None,
    };
    let result = crate::balance::run(config, &apl);
    // **Verhinderter Schaden zaehlt mit** (Issue #472).
    //
    // Vorher stand hier `result.damage.mean` allein. Gemessen an boss-1-2:
    // Acht von elf FIXATION-Zaubern tragen `baseDamage: 0`, kosten aber eine
    // globale Wartezeit — sie waren damit in JEDER Runde der schlechteste Zug
    // und kamen in keiner der acht besten Rotationen vor. Nicht weil sie
    // schwach sind, sondern weil ihre Leistung in keiner Zahl stand, die die
    // Suche ansieht.
    //
    // Dass sie ueberhaupt etwas leisten, ist belegt: Dieselbe Rotation mit
    // `pin` vorne senkt `damage_taken` von 1720 auf 1440. Die Mechanik wirkt
    // (`fight.rs:2405` verschiebt `cast_lands_at`), nur bewertet wurde sie nie.
    //
    // **Der Bezugspunkt ist `damage_taken_baseline` aus der Konfiguration**,
    // nicht ein zweiter Lauf: Ein Vergleichslauf je Messung vervielfachte die
    // Laufzeit einer Suche, die heute schon 1778 Messungen braucht.
    let verhindert = (config.damage_taken_baseline - result.damage_taken.mean).max(0.0);
    Measured {
        apl,
        mean: result.damage.mean + config.prevented_weight * verhindert,
        stderr: result.damage.stderr,
    }
}

/// Womit die Suche aufgehoert hat — gehoert nach AC4 in den Bericht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// Keine Zeile verbesserte das Ergebnis noch messbar.
    NoImprovement,
    /// Alle Kandidaten stehen in der Liste.
    PoolExhausted,
    /// Die Obergrenze fuer die Laenge war erreicht.
    LengthLimit,
}

/// Das Ergebnis einer Suche, mit allem, was der Bericht braucht.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Die gefundene Rotation.
    pub best: Measured,
    /// Die gewaehlten Zeilen in ihrer Reihenfolge.
    pub chosen: Vec<Candidate>,
    /// Wie viele Kandidaten zur Wahl standen.
    ///
    /// **Die übergebene Zahl**, nicht die tatsächlich benutzte: Was die Suche
    /// selbst ergänzt hat, steht in [`SearchResult::ergaenzt`].
    pub pool_size: usize,
    /// Welche Zeilen die Suche selbst ergänzt hat, mit Zaubernamen (#1088).
    ///
    /// # Warum Namen und nicht nur eine Zahl
    ///
    /// Eine Zahl sagt, **dass** etwas dazukam. Ein Name sagt, **ob es das
    /// ist**, was der Aufrufer herausgenommen hat. Wer `purge` aus dem Vorrat
    /// filtert, um seine Wirkung zu messen, muss sehen können, dass die Suche
    /// ihn nicht wieder hineingelegt hat.
    ///
    /// Leer, wenn nichts ergänzt wurde — und das ist der Normalfall an einem
    /// Boss ohne die betreffende Mechanik.
    pub ergaenzt: Vec<String>,
    /// Wie viele Rotationen tatsaechlich gemessen wurden.
    pub measured: usize,
    /// Warum die Suche aufgehoert hat.
    pub stop: Stop,
    /// Wie viele PAARE ueberhaupt zur Wahl standen (Issue #558).
    ///
    /// **Ohne diese Zahl sagt „kein Unterschied" nichts.** Sie trennt die zwei
    /// Faelle, die sonst gleich aussehen: Das Paar wurde geprueft und hat
    /// verloren — oder es kam nie zur Wahl, weil die Laengengrenze vorher
    /// griff. Gemessen ist genau das passiert: Bei `max_len = 6` ist die Liste
    /// voll, bevor die Einzelzuege aufhoeren zu tragen.
    pub paare_geprueft: usize,
    /// Wie viele DREIERGRUPPEN ueberhaupt zur Wahl standen (Issue #797).
    ///
    /// Dieselbe Aufgabe wie [`SearchResult::paare_geprueft`], eine Stufe
    /// weiter. „Kein Unterschied" kann heissen: geprueft und verloren, ODER die
    /// Suche hielt an der Laengengrenze an, bevor kein Einzelzug mehr trug,
    /// ODER eine der drei Mengen in [`tripel_zuege`] war leer.
    ///
    /// **Bis #1027 kam ein vierter Fall dazu:** Die Gruppe brauchte drei freie
    /// Plaetze (`chosen.len() + 3 <= max_len`) und wurde bei weniger gar nicht
    /// gefragt. Seither darf sie Zeilen verdraengen; gezaehlt werden weiterhin
    /// die Gruppen, nicht die Arten, Platz zu schaffen.
    pub tripel_geprueft: usize,
    /// Wie viele Runden an der Laengengrenze stattfanden (#1027) — **hoechstens
    /// eine**: die eine Runde, in der nur noch Dreiergruppen gefragt werden.
    ///
    /// Die Zahl steht hier, weil die Beschraenkung sonst an keinem Ergebnis
    /// sichtbar waere: Mehr Runden an der Grenze aendern die gefundene Liste
    /// meist nicht, wohl aber die Schranke aus [`restmessungen_hoechstens`],
    /// die genau eine annimmt.
    pub grenzrunden: usize,
}

/// Die Suche: jeder Kandidat wird an **jeder** Position probiert.
///
/// # Warum nicht hinten anhaengen — gemessen, nicht vermutet
///
/// Die erste Fassung haengte jeden Gewinner unten an. Sie war gierig im
/// engeren Sinn und **nachweislich falsch**:
///
/// ```text
/// anhaengen    4000,0
/// erschoepfend 5960,0     Verfehlung um ein Drittel
/// ```
///
/// 5960,0 ist der Wert von `step1-good` — die Suche waere also deutlich
/// schlechter gewesen als die Handschrift, die sie schlagen soll.
///
/// Der Grund ist derselbe Satz, mit dem ich das Anhaengen begruendet hatte:
/// *Der Beitrag einer Position haengt fast nur von dem ab, was UEBER ihr
/// steht.* Er stimmt — und genau deshalb scheitert das Anhaengen: Weil das
/// Obere alles Darunterliegende verdeckt, ist die erste Wahl die
/// folgenreichste, und sie wird mit der schlechtesten Information getroffen,
/// naemlich mit einem einzelnen Zauber ohne Liste. Allein gemessen gewinnt
/// `KINDLE`; in einer Liste gehoert das Brand-Nachlegen darueber.
///
/// **Ein Argument gegen die Annahme war fuer ein Argument dafuer gehalten
/// worden.** Aufgefallen ist es nur, weil die Gegenprobe erschoepfend
/// dagegenhielt.
///
/// # Was stattdessen geschieht
///
/// Jeder noch offene Kandidat wird an jeder der bisherigen Positionen
/// eingefuegt, nicht nur hinten. Eine spaeter erkannte Zeile kann damit noch
/// nach oben, und die folgenreichste Wahl wird nicht mehr zuerst und blind
/// getroffen.
///
/// Der Aufwand ist n³/6 statt n²/2 — bei fuenfzehn Kandidaten rund 560
/// Messungen statt 112. Gegen 15! = 1.307.674.368.000 ist beides nichts.
#[must_use]
pub fn greedy(config: &BalanceConfig, pool: &[Candidate], max_len: usize) -> SearchResult {
    greedy_from(config, pool, max_len, &[])
}

/// Dieselbe Suche, aber von einer **vorgegebenen Liste** aus (Issue #558).
///
/// # Der Mangel, den das behebt
///
/// Gemessen mit `--max-len 12` ueber fuenf Bosse (#509): Die fuer
/// `test-metronome` gefundene Liste ist an VIER anderen Bossen besser als
/// deren jeweils eigene gefundene — um 20,9, 30,7, 22,1 und 37,3 %.
///
/// Das ist kein Zufall, sondern die Bauart. [`greedy`] bricht ab, sobald keine
/// EINZELNE weitere Zeile mehr messbar verbessert (`Stop::NoImprovement`) —
/// bei Laenge 12 geschieht das schon nach acht Zeilen. Je groesser der Raum,
/// desto frueher legt sich das Verfahren fest, und desto weiter kann der Punkt
/// vom Optimum entfernt liegen. **Bei Laenge 6 tritt der Fall gar nicht auf**,
/// weil die Laengengrenze vorher greift; der Mangel war dort unsichtbar.
///
/// # Warum ein Startpunkt und nicht ein besseres Verfahren
///
/// Ein Verfahren, das Paare findet, waere die gruendlichere Antwort und steht
/// als AC2 in #558 offen. Der Startpunkt ist die billige, die den belegten
/// Fall unmittelbar trifft: Was an einem Boss gut war, ist an einem anderen
/// oft ein besserer Ausgangspunkt als die leere Liste — und von dort sucht
/// dasselbe Verfahren weiter, statt die fremde Liste nur zu uebernehmen.
///
/// # Was er NICHT kann
///
/// Er findet nichts, was an keinem Boss gefunden wurde. Ein Paar, dessen
/// beide Haelften einzeln verlieren, bleibt unsichtbar — das ist der
/// eigentliche Gegenstand von #558 und durch diesen Nachlauf nicht erledigt.
///
/// # ⚠ Das Ergebnis kann SCHLECHTER sein als eine freie Suche
///
/// Diese Funktion gibt zurueck, was von DIESEM Startpunkt aus erreichbar ist —
/// mehr verspricht sie nicht. Ist die Startliste schon so lang wie `max_len`,
/// greift sofort [`Stop::LengthLimit`]: Es wird gar nicht gesucht, und heraus
/// kommt die fremde Liste, an der neuen Stelle gespielt.
///
/// Gemessen an `first-crucible` gegen `test-onslaught`:
///
/// ```text
///   freie Suche an test-onslaught            6553
///   von der first-crucible-Liste aus         3138
/// ```
///
/// **Wer sie ohne Besser-Pruefung benutzt, verschlechtert sein Ergebnis**, und
/// die Zahl sieht dabei plausibel aus. Der Aufrufer muss also vergleichen:
///
/// ```text
///   if versuch.best.mean > bisher { uebernehmen }
/// ```
///
/// Genau das tut der Nachlauf in `examples/rotation-search.rs`. Der Fall
/// `ohne_besser_pruefung_verschlechtert_der_nachlauf` haelt die Falle fest.
#[must_use]
pub fn greedy_from(
    config: &BalanceConfig,
    pool: &[Candidate],
    max_len: usize,
    start: &[Candidate],
) -> SearchResult {
    greedy_opts(config, pool, max_len, start, true)
}

/// Dieselbe Suche mit abschaltbarer Paar-Betrachtung (Issue #558).
///
/// # Warum der Schalter Pflicht ist und nicht Bequemlichkeit
///
/// Ein Gewinn, der mit Paar-Suche gemessen wird, koennte auch vom Startpunkt
/// des Nachlaufs stammen oder vom Rauschen. Nur wer dieselbe Stelle OHNE
/// Paare messen kann, weiss, woran er liegt. Ohne den Schalter waere die
/// einzige Gegenprobe, die Paar-Logik auszubauen — also mit einem Werkzeug zu
/// messen, das es so nicht gibt.
///
/// `mit_paaren = false` ergibt exakt die Suche von vor #558.
///
/// **Seit #797 schaltet dieses Flag beides**: Paare und Dreiergruppen. Wer die
/// zwei getrennt messen will — und für die Gegenprobe zum Tripel-Zug muss man
/// das —, nimmt [`greedy_zuege`]. Der Grund für die Kopplung hier ist der
/// Zweck des Schalters: `false` heißt „nur Einzelzüge, wie vor #558", und eine
/// Dreiergruppe ist kein Einzelzug.
#[must_use]
pub fn greedy_opts(
    config: &BalanceConfig,
    pool: &[Candidate],
    max_len: usize,
    start: &[Candidate],
    mit_paaren: bool,
) -> SearchResult {
    greedy_zuege(
        config,
        pool,
        max_len,
        start,
        Zuege {
            paare: mit_paaren,
            tripel: mit_paaren,
        },
    )
}

/// Welche zusammengesetzten Züge die Suche betrachten darf.
///
/// # Warum das ein eigener Typ ist und kein zweites `bool`
///
/// Zwei Wahrheitswerte hintereinander im Aufruf (`…, true, false)`) sind an der
/// Aufrufstelle nicht lesbar, und beim Vertauschen ändert sich das Ergebnis
/// ohne Fehlermeldung. Der Fall `paar_und_tripel_lassen_sich_getrennt_messen`
/// hängt genau daran: Er misst dieselbe Stelle einmal mit und einmal ohne
/// Dreiergruppen, und eine vertauschte Zeile hätte dort zwei gleiche Zahlen
/// erzeugt — die aussehen wie „kein Unterschied".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zuege {
    /// Paare aus [`paar_zuege`] (Issue #558).
    pub paare: bool,
    /// Dreiergruppen aus [`tripel_zuege`] (Issue #797).
    pub tripel: bool,
}

impl Zuege {
    /// Nur Einzelzüge — die Suche von vor #558.
    pub const NUR_EINZELN: Self = Self {
        paare: false,
        tripel: false,
    };
    /// Einzelzüge und Paare — die Suche zwischen #558 und #797.
    pub const MIT_PAAREN: Self = Self {
        paare: true,
        tripel: false,
    };
    /// Alles, was die Suche heute kann.
    pub const ALLE: Self = Self {
        paare: true,
        tripel: true,
    };
}

/// Ergänzt boss-abhängige Kandidatenzeilen (Issue #1088).
///
/// # Warum das hier steht und nicht in `candidates_for`
///
/// Die Schwelle kommt aus den **Bossdaten**. `candidates_for(index)` kennt nur
/// den Zauber — es müsste den Boss durchgereicht bekommen, und das fasst 32
/// aufrufende Dateien an, deren Diff zu 95 % mechanisch wäre. Hier ist der Boss
/// über `BalanceConfig` ohnehin bekannt.
///
/// # Nur für Zauber, die im übergebenen Pool stehen
///
/// **Das ist keine Sparmaßnahme, sondern der Kern.** Wer einen Zauber aus dem
/// Vorrat filtert, um seine Wirkung zu messen, darf ihn hier nicht wieder
/// hineingelegt bekommen — sonst wäre seine Gegenprobe still kaputt und sähe
/// aus wie vorher. Die Prüfung auf Anwesenheit macht diesen Fehler nicht
/// sichtbar, sondern unmöglich.
///
/// # Die Schwelle
///
/// Gesenkt wird **so spät wie möglich, aber nicht zu spät**: bei
/// `threshold - amount * (Wartezeit bis zur nächsten Gelegenheit)`. Die
/// Wartezeit ist ein GCD.
///
/// Die naheliegende Alternative `doom >= clear_amount` war in der Messung vom
/// 2026-08-22 schlechter — an zwei Bossen, bei `MAX_LEN = 6`. Sie senkt,
/// sobald nichts mehr verschenkt wird, und verbraucht die Ladung dadurch
/// früh. Bei `boss-1-2` liegen zwischen `clear_amount` (40) und
/// `threshold` (100) zehn Sekunden; wer bei 40 senkt, steht bei 90 ohne da.
/// Genau das beschreibt der Boss selbst: „wer die zweite zu früh verbraucht,
/// verliert".
///
/// # Öffentlich, damit der Prüffall den Gegenstand selbst greifen kann
///
/// #1088 fragt, ob die Suche die Bedingung **bilden** kann — nicht, ob sie sie
/// wählt. Das Wählen entscheidet die Bewertung und darf negativ ausgehen.
/// Beides in einem Kriterium zu prüfen hieße, zwei Fragen in einen Satz zu
/// packen; die gebildete Zeile muss deshalb von außen sichtbar sein.
#[must_use]
pub fn boss_kandidaten(config: &BalanceConfig, pool: &[Candidate]) -> Vec<Candidate> {
    let Some(doom) = config.boss.doom else {
        return Vec::new();
    };
    // Ein GCD Vorlauf, in `amount`-Schritten je `per_step_ms` gerechnet.
    let vorlauf = u32::try_from(
        u64::from(doom.amount) * u64::from(GCD_MS) / u64::from(doom.per_step_ms.max(1)),
    )
    .unwrap_or(0);
    // `saturating_sub` FAENGT DEN UNTERLAUF AB, UND DAS IST HIER EINE
    // ENTSCHEIDUNG, KEINE FORMALIE: Waere `vorlauf` groesser als `threshold`,
    // ergaebe die Saettigung die Schwelle 0 — also „senke immer", still und
    // ohne Meldung.
    //
    // Heute kann das nicht eintreten: Das groesste `amount` ueber alle
    // doom-Bosse ist 6, der Vorlauf damit 9 gegen ein `threshold` von 100. Ein
    // Boss mit `amount = 70` wuerde es ausloesen, und niemand saehe es.
    let schwelle = i64::from(doom.threshold.saturating_sub(vorlauf));

    (0..SPELL_COUNT)
        .filter(|&i| SPELLS[i].clears_doom)
        // AUFLAGE 1: nur, wenn der Zauber überhaupt zur Wahl steht.
        .filter(|&i| pool.iter().any(|k| k.spell == i))
        .map(|i| Candidate {
            spell: i,
            condition: single(Value::Doom, Compare::GreaterEqual, Value::Literal(schwelle)),
            origin: Origin::DoomClear,
        })
        .collect()
}

/// Der beste Dreiergruppen-Zug einer Runde (#797, Platz schaffen seit #1027).
struct TripelZug {
    /// Unterbrecher, Zeile darum, Erzeuger — Stellen in `offen`.
    glieder: [usize; 3],
    /// Die Liste mit der Gruppe.
    liste: Vec<Candidate>,
    gemessen: Measured,
}

/// Alle Arten, aus `laenge` Stellen genau `k` wegzulassen, aufsteigend.
///
/// `k = 0` ergibt genau eine Art: nichts weglassen. Dann ist der Zug derselbe
/// wie vor #1027.
fn weglassungen(laenge: usize, k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![Vec::new()];
    }
    if k > laenge {
        return Vec::new();
    }
    let mut alle = Vec::new();
    for erste in 0..laenge {
        for rest in weglassungen(laenge - erste - 1, k - 1) {
            let mut art = vec![erste];
            art.extend(rest.into_iter().map(|i| i + erste + 1));
            alle.push(art);
        }
    }
    alle
}

/// Dieselbe Suche mit getrennt schaltbaren Paaren und Dreiergruppen (#797).
///
/// # Warum die Trennung Pflicht ist
///
/// Dieselbe Begründung wie bei `mit_paaren` in #558, eine Stufe weiter: Ein
/// Gewinn, der mit Dreiergruppen gemessen wird, könnte auch von den Paaren
/// kommen. Nur wer dieselbe Stelle mit Paaren **und ohne** Dreiergruppen messen
/// kann, weiß, welche Stufe ihn gebracht hat.
///
/// Ohne diesen Schalter wäre die einzige Gegenprobe, die Tripel-Logik wieder
/// auszubauen — also mit einem Werkzeug zu messen, das man dafür verändert.
#[must_use]
pub fn greedy_zuege(
    config: &BalanceConfig,
    pool: &[Candidate],
    max_len: usize,
    start: &[Candidate],
    zuege: Zuege,
) -> SearchResult {
    let mit_paaren = zuege.paare;

    // **Boss-abhängige Zeilen** (#1088). Sie entstehen hier und nicht in
    // `candidates_for`, weil ihre Schwelle aus den Bossdaten kommt — und nur
    // für Zauber, die im übergebenen Pool stehen.
    let ergaenzung = boss_kandidaten(config, pool);
    let ergaenzt: Vec<String> = ergaenzung
        .iter()
        .map(|k| SPELLS[k.spell].id.to_string())
        .collect();
    let pool: Vec<Candidate> = pool.iter().chain(ergaenzung.iter()).cloned().collect();
    let pool: &[Candidate] = &pool;

    let mut chosen: Vec<Candidate> = start.to_vec();
    // **Was schon in der Startliste steht, steht nicht mehr zur Wahl.** Sonst
    // koennte dieselbe Zeile zweimal in der Rotation landen — die zweite waere
    // unerreichbar, und die Laengengrenze verbraucht.
    let mut offen: Vec<usize> = (0..pool.len())
        .filter(|&i| !chosen.contains(&pool[i]))
        .collect();
    let mut measured = 0;
    let mut paare_geprueft = 0;
    let mut tripel_geprueft = 0;

    // **Der Bezugspunkt für „verhindert" wird HIER bestimmt, einmal** (#814).
    //
    // Der geerbte Wert war `damage_taken_baseline: 0.0` aus dem Default, und
    // damit rechnete die Formel `(0.0 - erlitten).max(0.0)` in jedem Fall zu
    // null: eine Größe, die aussieht, als würde sie zählen, und es nie tut.
    //
    // **Die leere Rotation ist der natürliche Bezugspunkt.** Sie erleidet
    // alles, was der Boss austeilt, weil ihm niemand in den Arm fällt. Was
    // eine Rotation davon abwendet, ist genau ihre Leistung im Verhindern —
    // und die Zahl bedeutet dasselbe an jedem Boss, ohne dass jemand sie je
    // von Hand setzen muss.
    //
    // **Einmal, nicht je Messung.** Der geerbte Kommentar hat recht: Ein
    // Vergleichslauf pro Messung vervielfachte eine Suche, die schon 1778
    // Messungen braucht. Ein einziger Lauf vorweg kostet einen von 1778.
    let mut config = *config;
    if config.prevented_weight > 0.0 {
        let leer = Apl {
            name: "bezugspunkt".to_owned(),
            rules: Vec::new(),
            overdrive: None,
        };
        config.damage_taken_baseline = crate::balance::run(&config, &leer).damage_taken.mean;
    }
    let config = &config;

    // Heisst „start" und nicht mehr „leer": Seit #558 kann hier eine
    // vorgegebene Liste stehen, und ein Name, der das Gegenteil behauptet,
    // schickt den naechsten Leser auf die falsche Faehrte.
    let mut best = measure(config, "start", &chosen);
    crate::fortschritt::bestes(best.mean);
    let mut runde = 0u64;
    // Ob die eine Runde an der Längengrenze schon stattgefunden hat (#1027).
    let mut grenze_gefragt = false;
    let mut grenzrunden = 0;
    let stop = loop {
        // **An der Längengrenze bekommt die Dreiergruppe noch einen Zug**
        // (#1027): Sie darf Zeilen verdrängen (siehe unten). Unter Fassung B
        // mit dem Entscheidungsmodell aus #1027 tragen die Einzelzüge der
        // #797-AC5-Suche bis zur Grenze 10 — und dort hielt die Suche an, ohne
        // eine einzige Gruppe gefragt zu haben (`tripel_geprueft` 0, Stopp
        // `LengthLimit`). Einzelzüge und Paare bleiben an der Grenze gesperrt.
        let voll = chosen.len() >= max_len;
        if voll && !zuege.tripel {
            break Stop::LengthLimit;
        }
        if offen.is_empty() {
            break if voll {
                Stop::LengthLimit
            } else {
                Stop::PoolExhausted
            };
        }
        // **Genau eine Runde an der Grenze**: Danach steht kein Zug mehr zur
        // Wahl, der die Länge hielte, und die Obergrenze aus
        // [`restmessungen_hoechstens`] bliebe ohne diese Beschränkung ohne
        // Schranke — jeder Tausch an der Grenze hält die Länge.
        if voll {
            if grenze_gefragt {
                break Stop::LengthLimit;
            }
            grenze_gefragt = true;
            grenzrunden += 1;
        }
        let aufgehoert = if voll {
            Stop::LengthLimit
        } else {
            Stop::NoImprovement
        };
        runde += 1;
        melde_runde(runde, pool, &offen, chosen.len(), max_len, zuege);
        // (Stelle im Pool, Einfuegeposition, Messung)
        let mut bester_zug: Option<(usize, usize, Measured)> = None;
        for (pos, &kandidat) in offen.iter().enumerate().filter(|_| !voll) {
            for stelle in 0..=chosen.len() {
                let mut versuch = chosen.clone();
                versuch.insert(stelle, pool[kandidat].clone());
                let gemessen = measure(config, "versuch", &versuch);
                measured += 1;
                let besser = match &bester_zug {
                    None => true,
                    Some((_, _, bisher)) => gemessen.mean > bisher.mean,
                };
                if besser {
                    bester_zug = Some((pos, stelle, gemessen));
                }
            }
        }
        // **Der Paar-Zug** (Issue #558). Er kommt NACH den Einzelzuegen und
        // nur zum Zug, wenn kein einzelner mehr traegt — siehe
        // [`paar_zuege`] fuer die Begruendung.
        let mut bester_paar_zug: Option<(usize, usize, usize, Measured)> = None;
        if mit_paaren && chosen.len() + 2 <= max_len {
            let paare = paar_zuege(pool, &offen);
            paare_geprueft += paare.len();
            for (v_pos, n_pos) in paare {
                for stelle in 0..=chosen.len() {
                    let mut versuch = chosen.clone();
                    versuch.insert(stelle, pool[offen[v_pos]].clone());
                    versuch.insert(stelle + 1, pool[offen[n_pos]].clone());
                    let gemessen = measure(config, "paar", &versuch);
                    measured += 1;
                    let besser = match &bester_paar_zug {
                        None => true,
                        Some((_, _, _, bisher)) => gemessen.mean > bisher.mean,
                    };
                    if besser {
                        bester_paar_zug = Some((v_pos, n_pos, stelle, gemessen));
                    }
                }
            }
        }

        let einzeln_traegt = bester_zug
            .as_ref()
            .is_some_and(|(_, _, m)| m.clearly_better_than(&best));

        // **Der Dreiergruppen-Zug** (Issue #797).
        //
        // **`!einzeln_traegt` steht hier in der Bedingung, und das ist kein
        // Beiwerk.** Der erste Entwurf rechnete die Gruppen in JEDER Runde und
        // benutzte sie nur, wenn kein Einzelzug mehr trug — mit einem Kommentar
        // daneben, der „solange etwas Billigeres traegt, wird das Teurere nicht
        // gefragt" behauptete. Gefragt wurde es doch, jede Runde, und bezahlt
        // wurde es auch: drei Einfuegungen je Gruppe und Stelle.
        //
        // Der Kommentar beschrieb die BENUTZUNG und las sich wie eine Aussage
        // ueber die KOSTEN. Jetzt beschreibt er den Code.
        //
        // (Der Paar-Zug darueber rechnet weiterhin in jeder Runde. Das ist
        // Bestand aus #558, nicht Gegenstand dieses Issues — aber wer dort
        // Laufzeit sucht, findet sie hier beschrieben.)
        //
        // **Fehlt der Gruppe nur der Platz, darf sie Zeilen verdrängen** (#1027,
        // Nachbesserung der Suche unter Fassung B). Bis hierher stand in der
        // Bedingung `chosen.len() + 3 <= max_len`: Eine Liste, an der die
        // Einzelzüge bis zwei Plätze vor der Grenze getragen hatten, sah die
        // Dreiergruppe nie. Gemessen an der #797-AC5-Suche
        // (`test-lethal-strike`, Gewicht 1, Grenze 10): Unter Fassung B tragen
        // die Einzelzüge bis Länge 8, `tripel_geprueft` blieb 0, FIXATION fiel
        // weg und der Spieler starb bei 12000 (Suchwert 11627) — obwohl
        // dieselbe Liste mit der Gruppe statt `kindle` unter B 18950 zählt
        // (`docs/APL-STRICT-COMPARISON-2026-10-24.md`, „Der #797-AC5-Prüffall
        // unter Fassung B").
        //
        // Jetzt wird die Gruppe auch dann gefragt, und zwar gegen jede Liste,
        // die aus der bisherigen durch Weglassen von so vielen Zeilen entsteht,
        // wie Platz fehlt. Gefragt wird sie wie bisher nur, wo kein Einzelzug
        // mehr trägt — und an der Längengrenze, wo kein Einzelzug mehr zur Wahl
        // steht.
        let mut bester_tripel_zug: Option<TripelZug> = None;
        if !einzeln_traegt && zuege.tripel {
            let gruppen = tripel_zuege(pool, &offen);
            tripel_geprueft += gruppen.len();
            let fehlt = (chosen.len() + 3).saturating_sub(max_len);
            let grundlagen: Vec<Vec<Candidate>> = weglassungen(chosen.len(), fehlt)
                .into_iter()
                .map(|weg| {
                    chosen
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| !weg.contains(i))
                        .map(|(_, k)| k.clone())
                        .collect()
                })
                .collect();
            // Die Dreiergruppen stehen erst jetzt fest — sie gehören zu dieser
            // Runde und heben deren Untergrenze (#933). Je Gruppe so viele
            // Messungen, wie es Arten gibt, Platz zu schaffen, mal deren Stellen
            // (#1027) — ohne Verdrängen genau `chosen.len() + 1`.
            let je_gruppe: usize = grundlagen.iter().map(|basis| basis.len() + 1).sum();
            crate::fortschritt::runde_ergaenzt((gruppen.len() * je_gruppe) as u64);
            // **Die Namen folgen der Ordnung aus [`tripel_zuege`]**, nicht der
            // Reihenfolge, in der die Glieder gedacht wurden: erst der
            // Unterbrecher, dann die Zeile darum, dann der Erzeuger. Sie hiessen
            // hier einmal `p_pos, q_pos, e_pos` — nach dem Drehen der Ordnung
            // trug `p_pos` den Unterbrecher, und der naechste Leser haette das
            // Gegenteil dessen gelesen, was der Code tut.
            for (unterbrecher, zeile, erzeuger) in gruppen {
                for basis in &grundlagen {
                    for stelle in 0..=basis.len() {
                        let mut versuch = basis.clone();
                        versuch.insert(stelle, pool[offen[unterbrecher]].clone());
                        versuch.insert(stelle + 1, pool[offen[zeile]].clone());
                        versuch.insert(stelle + 2, pool[offen[erzeuger]].clone());
                        let gemessen = measure(config, "tripel", &versuch);
                        measured += 1;
                        let besser = match &bester_tripel_zug {
                            None => true,
                            Some(bisher) => gemessen.mean > bisher.gemessen.mean,
                        };
                        if besser {
                            bester_tripel_zug = Some(TripelZug {
                                glieder: [unterbrecher, zeile, erzeuger],
                                liste: versuch,
                                gemessen,
                            });
                        }
                    }
                }
            }
        }

        if !einzeln_traegt {
            // **Hier und nur hier greifen Paar und Dreiergruppe.** Solange ein
            // einzelner Zug noch traegt, ist er billiger und die Suche laeuft
            // wie bisher. Erst an der Stelle, an der `greedy` frueher aufgab,
            // wird gefragt: Gibt es eine Gruppe, deren Glieder einzeln
            // verloren haben?
            //
            // **Das Paar hat Vorrang vor der Dreiergruppe, wenn beide tragen**
            // — und zwar bei GLEICHEM Messwert. Es verbraucht eine
            // Listenposition weniger, und die Laengengrenze ist knapp: Was hier
            // an Platz draufgeht, fehlt spaeter jeder weiteren Zeile. Ist die
            // Dreiergruppe echt besser, gewinnt sie.
            let paar_wert = bester_paar_zug.as_ref().map(|(_, _, _, m)| m.mean);
            let tripel_wert = bester_tripel_zug.as_ref().map(|zug| zug.gemessen.mean);
            let tripel_gewinnt = match (paar_wert, tripel_wert) {
                (None, Some(_)) => true,
                (Some(p), Some(t)) => t > p,
                _ => false,
            };

            if tripel_gewinnt {
                let Some(TripelZug {
                    glieder: [unterbrecher, zeile, erzeuger],
                    liste,
                    gemessen,
                }) = bester_tripel_zug
                else {
                    break aufgehoert;
                };
                if !gemessen.clearly_better_than(&best) {
                    break aufgehoert;
                }
                // **Verdrängte Zeilen kehren nicht in den Vorrat zurück.** Nach
                // einem Tausch ist die Liste voll, und es bleibt nur die eine
                // Runde an der Grenze. Kehrten sie zurück, könnten dort Gruppen
                // entstehen, die es zu Rundenbeginn nicht gab — und die
                // Obergrenze aus [`restmessungen_hoechstens`] hielte nicht mehr
                // („Tripel ≤ die Zahl von jetzt").
                chosen = liste;
                // **Absteigend entfernen**, sonst verschiebt die erste
                // Entnahme die uebrigen Indizes. Bei zwei Gliedern reichte ein
                // Vergleich; bei drei wird sortiert, weil jede Handrechnung
                // hier genau der Fehler waere, den die Sortierung erspart.
                //
                // Die drei koennen nicht gleich sein: Sie stammen aus drei
                // Schleifen ueber sich gegenseitig ausschliessende Herkuenfte
                // (NextStrikeWindow, BossCastWindow, Always). Waere das eines
                // Tages nicht mehr so, entfernte die Schleife denselben Platz
                // zweimal und nähme einen fremden Kandidaten mit.
                let mut raus = [unterbrecher, zeile, erzeuger];
                raus.sort_unstable();
                debug_assert!(
                    raus[0] != raus[1] && raus[1] != raus[2],
                    "eine Dreiergruppe nennt denselben Platz zweimal: {raus:?}"
                );
                for pos in raus.iter().rev() {
                    offen.remove(*pos);
                }
                best = gemessen;
                crate::fortschritt::bestes(best.mean);
                continue;
            }

            let Some((v_pos, n_pos, stelle, gemessen)) = bester_paar_zug else {
                break aufgehoert;
            };
            if !gemessen.clearly_better_than(&best) {
                break aufgehoert;
            }
            chosen.insert(stelle, pool[offen[v_pos]].clone());
            chosen.insert(stelle + 1, pool[offen[n_pos]].clone());
            // Absteigend entfernen, sonst verschiebt die erste Entnahme den
            // zweiten Index.
            let (hoch, tief) = if v_pos > n_pos {
                (v_pos, n_pos)
            } else {
                (n_pos, v_pos)
            };
            offen.remove(hoch);
            offen.remove(tief);
            best = gemessen;
            crate::fortschritt::bestes(best.mean);
            continue;
        }

        let Some((pos, stelle, gemessen)) = bester_zug else {
            break Stop::PoolExhausted;
        };
        // **Messbar besser, nicht nur groesser.** Sonst haengt die Suche
        // Zeilen an, die nichts beitragen, und der Bericht meldet eine
        // Rotation, deren untere Haelfte Rauschen ist.
        if !gemessen.clearly_better_than(&best) {
            break Stop::NoImprovement;
        }
        chosen.insert(stelle, pool[offen[pos]].clone());
        offen.remove(pos);
        best = gemessen;
        crate::fortschritt::bestes(best.mean);
    };

    // Die Messung der gefundenen Liste zählt zur letzten Runde (#933).
    crate::fortschritt::schlussmessung();
    SearchResult {
        best: measure(config, "gefunden", &chosen),
        chosen,
        // Die ÜBERGEBENE Zahl, zurückgerechnet aus dem erweiterten Vorrat.
        //
        // `ergaenzung.len()` und nicht `ergaenzt.len()`: Beide sind heute
        // gleich lang, weil die Namensliste ein schlichtes `map` ohne
        // Entdopplung ist. Sie hängt aber an der DARSTELLUNG — wer die
        // Meldeliste später lesbarer macht und doppelte Namen zusammenfasst,
        // ändert damit still diese Zahl. Der Vorrat selbst kann das nicht.
        pool_size: pool.len() - ergaenzung.len(),
        ergaenzt,

        measured,
        stop,
        paare_geprueft,
        tripel_geprueft,
        grenzrunden,
    }
}

/// Meldet den Beginn einer Suchrunde an den Fortschritt (#933).
///
/// **Nur mit angemeldetem Zählwerk.** Ohne Anmeldung rechnet die Suche hier
/// keine Paar- und Tripelzahlen zusätzlich aus — ein Lauf ohne Anzeige zahlt
/// einen Vergleich, sonst nichts.
///
/// Sicher kostet die Runde ihre Einzel- und Paarzüge; die Dreiergruppen
/// kommen nur, wenn kein Einzelzug trägt, und melden sich dann selbst
/// ([`crate::fortschritt::runde_ergaenzt`]). Beide Grenzen tragen eins mehr:
/// die Messung der gefundenen Liste nach der letzten Runde. In den Plan der
/// Runde kommt sie erst, wenn feststeht, dass es die letzte war
/// ([`crate::fortschritt::schlussmessung`]).
fn melde_runde(
    runde: u64,
    pool: &[Candidate],
    offen: &[usize],
    gewaehlt: usize,
    max_len: usize,
    zuege: Zuege,
) {
    if crate::fortschritt::strang().is_none() {
        return;
    }
    let paare = if zuege.paare {
        paar_zuege(pool, offen).len()
    } else {
        0
    };
    let tripel = if zuege.tripel {
        tripel_zuege(pool, offen).len()
    } else {
        0
    };
    let paare_jetzt = if gewaehlt + 2 <= max_len { paare } else { 0 };
    // An der Längengrenze misst die Runde keine Einzelzüge (#1027): Sie ist
    // die eine Runde der Dreiergruppen, und deren Messungen meldet die Suche
    // selbst nach (`runde_ergaenzt`).
    let einzeln = if gewaehlt < max_len { offen.len() } else { 0 };
    let geplant = ((einzeln + paare_jetzt) * (gewaehlt + 1)) as u64;
    let hoechstens = restmessungen_hoechstens(gewaehlt, offen.len(), max_len, paare, tripel);
    crate::fortschritt::suchrunde(runde, geplant, geplant + 1, hoechstens + 1);
}

/// Wie viele Messungen die gierige Suche ab einem Rundenbeginn höchstens noch
/// kostet (#933) — diese Runde eingeschlossen, ohne die abschließende Messung
/// der gefundenen Liste.
///
/// # Warum eine Schranke und keine Zahl
///
/// Die Suche endet, sobald keine Zeile mehr messbar trägt; wann das ist, steht
/// erst nach der Messung fest. Fest steht nur, was sie **höchstens** tun kann,
/// und das folgt aus [`greedy_zuege`] selbst, Runde `j` ab jetzt:
///
/// ```text
///   Länge           gewaehlt + j  ≤  c_j  ≤  min(gewaehlt + 3j, max_len − 1)
///                   (jede Runde fügt eine bis drei Zeilen an; eine Runde
///                    beginnt nur unter der Längengrenze)
///   offen           m_j ≤ offen − j
///   Paare, Tripel   ≤ die Zahl von jetzt — sie entstehen nur aus offenen Zeilen
///   Messungen       (m_j + Paare + Tripel) · (c_j + 1)
///   Runden          höchstens min(max_len − gewaehlt, offen)
/// ```
///
/// Paare zählen nur, solange `c_j + 2 ≤ max_len` noch möglich ist. Die
/// Schranke ist grob — sie nimmt in jeder Runde die längste mögliche Liste
/// an —, aber sie hält: `tests/fortschritt.rs` prüft sie an jeder beobachteten
/// Runde einer echten Suche.
///
/// # Dreiergruppen seit #1027
///
/// Eine Dreiergruppe darf Zeilen verdrängen, wenn ihr der Platz fehlt, und an
/// der Längengrenze bekommt sie **genau eine** Runde für sich. Sie zählt
/// deshalb in jeder Runde, mit [`tripel_messungen`] je Gruppe statt
/// `c_j + 1`, und dazu kommt die Runde an der Grenze. Beide Schritte halten die
/// zwei Annahmen oben: Jede Runde unter der Grenze verlängert die Liste um
/// mindestens eine Zeile und nimmt mindestens eine aus `offen` (drei hinein,
/// höchstens zwei verdrängt), und verdrängte Zeilen kehren nicht nach `offen`
/// zurück — die Zahl der Gruppen kann also nur sinken.
#[must_use]
pub fn restmessungen_hoechstens(
    gewaehlt: usize,
    offen: usize,
    max_len: usize,
    paare: usize,
    tripel: usize,
) -> u64 {
    let an_der_grenze = tripel as u64 * tripel_messungen(max_len, max_len);
    if gewaehlt >= max_len {
        return an_der_grenze;
    }
    let runden = (max_len - gewaehlt).min(offen);
    let mut summe = 0u64;
    for j in 0..runden {
        let laenge_hoechstens = (gewaehlt + 3 * j).min(max_len - 1);
        let laenge_mindestens = gewaehlt + j;
        let mut zuege = (offen - j) as u64;
        if laenge_mindestens + 2 <= max_len {
            zuege += paare as u64;
        }
        summe += zuege * (laenge_hoechstens as u64 + 1);
        let je_gruppe = (laenge_mindestens..=laenge_hoechstens)
            .map(|laenge| tripel_messungen(laenge, max_len))
            .max()
            .unwrap_or(0);
        summe += tripel as u64 * je_gruppe;
    }
    summe + an_der_grenze
}

/// Wie viele Messungen eine Dreiergruppe an einer Liste der Länge `laenge`
/// kostet (#1027): je Art, die fehlenden Plätze freizumachen, jede Stelle der
/// verbliebenen Liste.
///
/// ```text
///   fehlt = max(0, laenge + 3 − max_len)
///   (laenge über fehlt) · (laenge − fehlt + 1)
/// ```
///
/// Ohne fehlenden Platz ist das `laenge + 1` — die Zahl von vor #1027. Dieselbe
/// Rechnung wie in [`greedy_zuege`], wo sie als `runde_ergaenzt` gemeldet wird.
#[must_use]
pub fn tripel_messungen(laenge: usize, max_len: usize) -> u64 {
    let fehlt = (laenge + 3).saturating_sub(max_len);
    if fehlt > laenge {
        return 0;
    }
    let mut auswahl = 1u64;
    for i in 0..fehlt {
        auswahl = auswahl * (laenge - i) as u64 / (i + 1) as u64;
    }
    auswahl * (laenge - fehlt + 1) as u64
}

/// Die Paare, die einzeln verlieren und zusammen tragen (Issue #558, AC2).
///
/// # Der Mangel, gemessen
///
/// Der Bericht der Rotations-Suche weist ihn selbst aus: 25 Kandidatenzeilen
/// stammen aus Verstaerkungs-Fenstern, und ueber alle gefundenen Rotationen
/// zusammen steht **keine einzige** davon in einer Liste.
///
/// Der Grund ist kein Zufall und keine Balance-Frage:
///
/// | Zeile | allein |
/// | --- | --- |
/// | `flashpoint` (der Verstaerker) | kostet einen GCD, bringt nichts |
/// | `kindle if buff.flashpoint.active` | der Buff ist nie da, also tot |
///
/// Ein gieriges Verfahren probiert jede Zeile EINZELN. Beide verlieren, also
/// nimmt es keine — und die zweite kann gar nicht gewinnen, solange die erste
/// fehlt. **Das Paar ist die kleinste Einheit, die ueberhaupt etwas wert
/// ist.**
///
/// # Woher die Paare kommen
///
/// Aus den DATEN, nicht aus einer gepflegten Liste. Eine Kandidatenzeile mit
/// [`Origin::BuffWindow`] traegt den Verstaerker in ihrer Bedingung
/// ([`Value::BuffActive`]); ihr Partner ist die unbedingte Zeile genau dieses
/// Verstaerkers. Wer einen Verstaerker ergaenzt, bekommt seine Paare ohne
/// Zutun — dieselbe Linie wie bei den Kandidatenzeilen selbst.
///
/// # Warum das billig bleibt
///
/// Nicht alle Paare aus allen Zeilen: Das waeren bei 88 Kandidaten 3828
/// Paare mal Einfuegepositionen zum Quadrat — der Grund, aus dem AC2
/// ausdruecklich auch „zu teuer" als Antwort zulaesst. Die Vorauswahl aus den
/// Daten laesst davon die Paare uebrig, die eine Beziehung HABEN.
///
/// Dazu zwei Beschraenkungen, beide begruendet:
///
/// 1. **Der Verstaerker steht direkt vor dem Nutzer.** In einer
///    Prioritaetenliste wird die erste erfuellte Zeile gewirkt; steht der
///    Verstaerker darueber, kommt er dran, sobald er bereit ist, und der
///    Nutzer greift in seinem Fenster. Das spart eine zweite freie Position
///    (n statt n²) — und die Suche kann spaeter Zeilen dazwischenschieben,
///    die Beschraenkung gilt nur fuer den Einfuege-Moment.
/// 2. **Nur wenn kein einzelner Zug mehr traegt.** Solange die gewoehnliche
///    Suche vorankommt, ist sie billiger. Das Paar wird erst dort gefragt, wo
///    `greedy` frueher `Stop::NoImprovement` gemeldet hat.
///
/// # Warum sie oeffentlich ist (#969)
///
/// Der Trockenlauf `rota-sim explain` sagt voraus, wie viele Messungen eine
/// Suche kostet, ohne sie zu fahren. Die Einzelzuege ergeben sich aus Pool und
/// Laenge; die Paar-Zuege nicht — sie haengen daran, welche Kandidaten ein
/// Verstaerkungs-Fenster tragen und ob der passende Verstaerker im Pool steht.
///
/// **Diese Zahl ist datenabhaengig, aber NICHT messabhaengig**: Die Funktion
/// liest nur `origin`, `condition` und `spell`, keinen einzigen Kampfwert. Sie
/// laesst sich also im Trockenlauf rufen, ohne AC2 zu verletzen.
///
/// Der Trockenlauf koennte sie auch nachbauen — und genau das soll er nicht.
/// Ein Nachbau antwortet plausibel aus einer zweiten Quelle und laeuft beim
/// naechsten Kriterium hier auseinander, ohne dass etwas rot wird.
#[must_use]
pub fn paar_zuege(pool: &[Candidate], offen: &[usize]) -> Vec<(usize, usize)> {
    let mut paare = Vec::new();
    for (n_pos, &nutzer) in offen.iter().enumerate() {
        if pool[nutzer].origin != Origin::BuffWindow {
            continue;
        }
        // Der Verstaerker steht in der Bedingung der Nutzer-Zeile.
        let Some(verstaerker) = buff_aus_bedingung(&pool[nutzer].condition) else {
            continue;
        };
        // Sein Partner ist die UNBEDINGTE Zeile desselben Zaubers — die, die
        // ihn ueberhaupt erst wirkt.
        for (v_pos, &kandidat) in offen.iter().enumerate() {
            if pool[kandidat].spell == verstaerker && pool[kandidat].origin == Origin::Always {
                paare.push((v_pos, n_pos));
            }
        }
    }

    // **Die zweite Beziehung: erst angreifbar machen, dann unterbrechen**
    // (Issue #797). Dieselbe Bauart wie oben, anderer Gegenstand.
    //
    // PIN gibt dem naechsten Schlag ein Fenster, in dem er ueberhaupt
    // unterbrechbar ist; QUENCH beendet ihn darin. Keine Haelfte traegt
    // allein — PIN kostet einen GCD und bringt fuer sich nichts, QUENCH
    // findet ohne PIN nie einen laufenden Zauber. Gemessen: Ohne dieses Paar
    // waehlt die Suche die PIN-Zeile bei KEINEM Gewicht auf das Verhinderte
    // (0, 1, 5, 20, 100 geprueft).
    //
    // **PIN STEHT OBEN — und diese Reihenfolge ist gemessen, nicht abgeleitet.**
    //
    // Sie war zwischendurch gedreht. Die Begruendung dafuer klang gut: Eine
    // Prioritaetenliste ist keine Sequenz, sie wird bei jeder Entscheidung neu
    // von oben gelesen; steht PIN oben, koennte er in dem Takt gewinnen, in dem
    // der Boss GERADE wirkt, und der Unterbrecher kaeme zu spaet.
    //
    // GEMESSEN AN `test-metronome` (Schleife 6000 ms, unterbrechbarer Cast UND
    // Schlag, also die Lage, in der sich die beiden Bedingungen ueberlappen):
    //
    // ```text
    //   PIN oben       2776 Leben       67 PIN-Wuerfe
    //   QUENCH oben    2768 Leben       67 PIN-Wuerfe
    // ```
    //
    // PIN oben ist besser. Die Ableitung war plausibel und falsch, und der
    // Unterschied ist klein (8 von 5000) — aber der Simulator ist
    // deterministisch (GAME-DESIGN 18.4), also ist er reproduzierbar und kein
    // Rauschen.
    //
    // **Der erste Messversuch konnte die Frage gar nicht beantworten**: Er fuhr
    // `test-lethal-strike` mit EINEM Eintrag und ohne Schleife und ergab
    // Gleichstand — dort kann die Ueberlappung nicht eintreten.
    // `die_reihenfolge_im_block_ist_gemessen` faehrt sie deshalb an den Bossen,
    // an denen sie entscheidbar ist.
    for (n_pos, &unterbrecher) in offen.iter().enumerate() {
        if pool[unterbrecher].origin != Origin::BossCastWindow
            || !SPELLS[pool[unterbrecher].spell].interrupt
        {
            continue;
        }
        for (v_pos, &kandidat) in offen.iter().enumerate() {
            if pool[kandidat].origin == Origin::NextStrikeWindow {
                paare.push((v_pos, n_pos));
            }
        }
    }

    paare
}

/// Die Dreiergruppen, die auch als Paar noch verlieren (Issue #797).
///
/// # Warum zwei hier nicht reichen
///
/// [`paar_zuege`] bildet fuer PIN das Paar „angreifbar machen, dann
/// unterbrechen". Gemessen wird es gebildet und gewinnt trotzdem nichts — und
/// der Grund steht in den Zauberdaten: **PIN kostet drei Kontroll-Punkte**, die
/// im Kampf erst erzeugt werden muessen.
///
/// ```text
/// Erzeuger   erzeugt Kontroll-Punkte      allein: ein mittelmaessiger Fueller
/// PIN        macht den Schlag angreifbar  allein: NICHT WIRKBAR
/// QUENCH     unterbricht ihn              allein: findet nie etwas Laufendes
/// ```
///
/// Keine der drei traegt allein, und **auch kein Paar daraus**: Ohne Erzeuger
/// ist PIN nicht wirkbar, ohne PIN hat QUENCH nichts zu unterbrechen, und
/// Erzeuger plus QUENCH ohne PIN ist nur ein Fueller mit einer toten Zeile.
/// Die kleinste Einheit, die etwas wert ist, hat hier drei Glieder.
///
/// # Warum es trotzdem billig bleibt
///
/// **Kein Suchraum aus drei Freiheitsgraden.** Alle drei Glieder folgen aus den
/// Daten, so wie das Paar aus #558 aus dem Verstaerker in der Bedingung folgt:
///
/// 1. Der Ausgangspunkt ist eine [`Origin::NextStrikeWindow`]-Zeile.
/// 2. Der Unterbrecher ist die [`Origin::BossCastWindow`]-Zeile eines Zaubers
///    mit `interrupt`.
/// 3. Der Erzeuger ist die unbedingte Zeile eines Zaubers mit
///    `control_gain > 0` — und nur dann, wenn der Ausgangspunkt ueberhaupt
///    Kontroll-Punkte **kostet**.
///
/// Bei den heutigen Daten sind das 1 x 1 x 3 Gruppen. Waere eine der drei
/// Mengen leer, entstuende gar keine — das ist der Unterschied zwischen einer
/// abgelesenen Beziehung und einer Sonderregel fuer einen Zauber.
///
/// # Die Reihenfolge ist nicht beliebig — und sie ist GEMESSEN
///
/// Eingefuegt wird in der zurueckgegebenen Ordnung, und eine Prioritaetenliste
/// wird bei **jeder** Entscheidung von oben gelesen. Herauskommt:
///
/// ```text
/// 1  der Unterbrecher   `quench if boss.casting`
/// 2  die Zeile darum    `pin if boss.next_strike_in <= X`
/// 3  der Erzeuger       unbedingt, fuellt die Kontroll-Punkte
/// ```
///
/// **Der Erzeuger steht unten**, weil seine unbedingte Zeile in jedem Takt
/// trifft, in dem seine Abklingzeit gefallen ist — bei `bind` jeden GCD.
/// Stuende er oben, kaeme PIN nie an die Reihe, und die Gruppe waere aus
/// demselben Grund tot, aus dem sie gebaut wurde.
///
/// **Der Unterbrecher steht ueber PIN** — so, wie es die erste Ableitung sagte:
/// Eine Prioritaetenliste ist keine Sequenz; steht PIN oben, gewinnt er in dem
/// Takt, in dem der Boss GERADE wirkt, loest die Wartezeit aus, und der
/// Unterbrecher kommt erst an ihrem Ende. Steht QUENCH oben, kostet er nichts:
/// Er loest keine Wartezeit aus, und die Liste entscheidet im naechsten Takt
/// neu — PIN ist dann immer noch dran.
///
/// Gemessen an `test-metronome` — Schleife 6000 ms, unterbrechbarer Cast UND
/// Schlag, also die Lage, in der sich beide Bedingungen ueberlappen — mit
/// `wisp` als Fueller (`die_reihenfolge_im_block_ist_gemessen`):
///
/// ```text
///   QUENCH oben    3416 Leben       67 PIN-Wuerfe
///   PIN oben       2784 Leben       67 PIN-Wuerfe
/// ```
///
/// **Bis #1027 stand PIN oben, gemessen mit 2776 gegen 2768.** Die Messung
/// hing am Fehler, nicht an der Reihenfolge: Nach QUENCH entschied der
/// Simulator erst am naechsten eingereihten Ereignis neu statt im naechsten
/// Takt, und genau das kostete die Ordnung mit QUENCH oben ihren Vorsprung.
/// An `boss-1-1`, `boss-2-3` und `boss-4-4` sind beide Ordnungen gleich.
///
/// **Der erste Messversuch konnte die Frage gar nicht beantworten.** Er fuhr
/// `test-lethal-strike`: EIN Eintrag, keine Schleife, Ergebnis Gleichstand.
/// Dort kann die Ueberlappung nicht eintreten — ein Fall, der genau den Fall
/// nicht enthaelt, fuer den er gebaut wurde.
/// `die_reihenfolge_im_block_ist_gemessen` faehrt sie deshalb an den Bossen,
/// an denen sie entscheidbar ist.
#[must_use]
pub fn tripel_zuege(pool: &[Candidate], offen: &[usize]) -> Vec<(usize, usize, usize)> {
    let mut gruppen = Vec::new();

    for (p_pos, &pin) in offen.iter().enumerate() {
        if pool[pin].origin != Origin::NextStrikeWindow {
            continue;
        }
        // **Nur wenn der Zauber ueberhaupt bezahlt werden muss.** Kostet er
        // nichts, ist der Erzeuger kein Glied der Gruppe, sondern eine
        // beliebige dritte Zeile — und die zu suchen waere der Suchraum, den
        // diese Funktion gerade vermeidet.
        let Some(kosten) = SPELLS[pool[pin].spell].control.map(|c| c.cost) else {
            continue;
        };
        if kosten == 0 {
            continue;
        }

        for (q_pos, &quench) in offen.iter().enumerate() {
            if pool[quench].origin != Origin::BossCastWindow
                || !SPELLS[pool[quench].spell].interrupt
            {
                continue;
            }
            for (e_pos, &erzeuger) in offen.iter().enumerate() {
                if pool[erzeuger].origin == Origin::Always
                    && SPELLS[pool[erzeuger].spell].control_gain > 0
                {
                    // Der Unterbrecher, dann die Zeile darum, dann der Erzeuger
                    // — gemessen, siehe Doc-Kommentar.
                    gruppen.push((q_pos, p_pos, e_pos));
                }
            }
        }
    }

    gruppen
}

/// Der Zauber, dessen Verstaerkung eine Bedingung abfragt.
///
/// Gibt `None` zurueck, sobald die Bedingung nicht die einfache Form aus
/// [`candidates_for`] hat — eine Paarbildung auf gut Glueck waere schlechter
/// als keine.
fn buff_aus_bedingung(condition: &Condition) -> Option<usize> {
    let [gruppe] = condition.groups.as_slice() else {
        return None;
    };
    let [term] = gruppe.as_slice() else {
        return None;
    };
    match term.left {
        Value::BuffActive(i) => Some(i),
        _ => None,
    }
}

/// Die erschoepfende Suche — **nur als Massstab**, nie als Verfahren.
///
/// Sie prueft jede Reihenfolge jeder Teilmenge. Das ist bei fuenfzehn
/// Kandidaten unerreichbar und bei sieben oder acht gerade noch machbar; ihr
/// einziger Zweck ist die Frage, ob [`greedy`] findet, was erschoepfend
/// auffindbar ist.
///
/// # Warum sie mit einem einzigen Durchgang misst
///
/// Der Vergleich soll die **Verfahren** trennen, nicht das Rauschen zweier
/// Messungen. Mit `iterations = 1` und festem Startwert ist jede Rotation
/// exakt reproduzierbar, und ein Unterschied im Ergebnis ist einer im
/// Verfahren. Mit vielen Durchgaengen koennte Greedy zufaellig gewinnen oder
/// verlieren, und die Gegenprobe maesse dann die Streuung statt der Suche.
#[must_use]
pub fn exhaustive(config: &BalanceConfig, pool: &[Candidate], max_len: usize) -> SearchResult {
    let mut best: Option<(Vec<usize>, Measured)> = None;
    let measured = exhaustive_besuchen(config, pool, max_len, &mut |liste, gemessen| {
        let besser = match best.as_ref() {
            None => true,
            Some((_, bisher)) => gemessen.mean > bisher.mean,
        };
        if besser {
            best = Some((liste.to_vec(), gemessen));
        }
    });

    let (liste, gemessen) = best.expect("kein einziger Kandidat — dann gibt es nichts zu suchen");
    let chosen = liste.iter().map(|&i| pool[i].clone()).collect();
    SearchResult {
        best: gemessen,
        chosen,
        pool_size: pool.len(),
        // Die erschoepfende Suche ergaenzt nichts — sie bekommt ihren Vorrat
        // vollstaendig uebergeben und prueft jede Teilmenge davon. Leer heisst
        // hier "nichts dazugetan", nicht "nicht nachgesehen".
        ergaenzt: Vec::new(),
        measured,
        stop: Stop::PoolExhausted,
        // Die erschoepfende Suche kennt keine Paare — sie prueft ohnehin jede
        // Teilmenge, also auch jedes Paar. Null heisst hier „nicht noetig",
        // nicht „nicht geprueft".
        paare_geprueft: 0,
        // Dasselbe fuer Dreiergruppen (#797): Wer jede Teilmenge prueft, prueft
        // auch jede Dreiergruppe. Die Null steht hier aus demselben Grund.
        tripel_geprueft: 0,
        // Und sie kennt keine Laengengrenze als Runde: Sie zaehlt Teilmengen auf.
        grenzrunden: 0,
    }
}

/// Der Kern von [`exhaustive`]: jede Reihenfolge jeder Teilmenge bis
/// `max_len`, jede genau einmal gemessen (Issue #946).
///
/// # Warum er offen liegt
///
/// [`exhaustive`] behält nur die beste Liste. Die Viabilitätsbefunde
/// ([`crate::viabilitaet`]) brauchen mehr: die beste Liste, **in der ein
/// bestimmter Zauber vorkommt**. Erst sie ist eine Schranke — und ohne
/// Schranke darf kein Zauber „dominiert" heißen. Ein zweiter Aufzähler daneben
/// liefe beim nächsten Umbau auseinander; beide gehen deshalb über diesen.
///
/// `besuch` bekommt jede Liste als Positionen im Vorrat, in ihrer Reihenfolge,
/// und ihre Messung aus [`measure`]. Besucht wird in Tiefensuche: eine Liste
/// vor ihren Verlängerungen, der Vorrat in seiner Reihenfolge — dieselbe
/// Reihenfolge wie vor #946, und an ihr hängt, welche von zwei gleich guten
/// Listen [`exhaustive`] meldet.
///
/// Gibt zurück, wie viele Listen gemessen wurden. Es sind genau
/// [`exhaustive_messungen`] — wer weniger zählt, hat nicht alles gesehen.
pub fn exhaustive_besuchen(
    config: &BalanceConfig,
    pool: &[Candidate],
    max_len: usize,
    besuch: &mut dyn FnMut(&[usize], Measured),
) -> usize {
    fn rekursion(
        config: &BalanceConfig,
        pool: &[Candidate],
        max_len: usize,
        aktuell: &mut Vec<usize>,
        benutzt: &mut Vec<bool>,
        measured: &mut usize,
        besuch: &mut dyn FnMut(&[usize], Measured),
    ) {
        if !aktuell.is_empty() {
            let zeilen: Vec<Candidate> = aktuell.iter().map(|&i| pool[i].clone()).collect();
            let gemessen = measure(config, "erschoepfend", &zeilen);
            *measured += 1;
            besuch(aktuell, gemessen);
        }
        if aktuell.len() >= max_len {
            return;
        }
        for i in 0..pool.len() {
            if benutzt[i] {
                continue;
            }
            benutzt[i] = true;
            aktuell.push(i);
            rekursion(config, pool, max_len, aktuell, benutzt, measured, besuch);
            aktuell.pop();
            benutzt[i] = false;
        }
    }

    let mut measured = 0;
    let mut aktuell: Vec<usize> = Vec::new();
    let mut benutzt = vec![false; pool.len()];
    rekursion(
        config,
        pool,
        max_len,
        &mut aktuell,
        &mut benutzt,
        &mut measured,
        besuch,
    );
    measured
}

/// Wie viele Listen [`exhaustive`] bei `n` Kandidaten und Länge `max_len`
/// misst: die Summe von `n! / (n - k)!` für `k = 1 … min(n, max_len)`.
///
/// Bei sieben Kandidaten und Länge 4 sind das 7 + 42 + 210 + 840 = 1099.
///
/// **Vorher ausrechenbar, und darum geht es** (Issue #946): Ein Budget für die
/// erschöpfende Suche lässt sich nur prüfen, bevor sie läuft. Läuft sie
/// erst, ist die Zeit schon ausgegeben. Sättigt statt überzulaufen — eine
/// Zahl über `u64::MAX` liegt ohnehin über jedem Budget.
#[must_use]
pub const fn exhaustive_messungen(n: usize, max_len: usize) -> u64 {
    let mut summe: u64 = 0;
    let mut glied: u64 = 1;
    let mut k = 0;
    while k < max_len && k < n {
        glied = glied.saturating_mul((n - k) as u64);
        summe = summe.saturating_add(glied);
        k += 1;
    }
    summe
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::index_of;

    /// Die Arten, Platz zu schaffen (#1027): jede Auswahl von `k` Stellen genau
    /// einmal, aufsteigend — gegen die Formel n über k gehalten, und mit `k = 0`
    /// als dem Zug von vor #1027 (nichts weglassen).
    #[test]
    fn weglassungen_zaehlt_jede_auswahl_genau_einmal() {
        assert_eq!(weglassungen(5, 0), vec![Vec::<usize>::new()]);
        assert_eq!(weglassungen(2, 3), Vec::<Vec<usize>>::new());
        assert_eq!(
            weglassungen(4, 2),
            vec![
                vec![0, 1],
                vec![0, 2],
                vec![0, 3],
                vec![1, 2],
                vec![1, 3],
                vec![2, 3]
            ]
        );
        for (n, k, erwartet) in [(10, 3, 120), (9, 2, 36), (8, 1, 8)] {
            let arten = weglassungen(n, k);
            assert_eq!(arten.len(), erwartet, "{n} über {k}");
            let einmal: std::collections::BTreeSet<_> = arten.iter().collect();
            assert_eq!(einmal.len(), erwartet, "{n} über {k}: doppelte Auswahl");
            assert!(
                arten
                    .iter()
                    .all(|art| art.windows(2).all(|p| p[0] < p[1]) && art.iter().all(|&i| i < n)),
                "{n} über {k}: Stelle doppelt oder außerhalb"
            );
        }
    }

    #[test]
    fn jeder_zauber_bekommt_mindestens_eine_zeile() {
        // Der Gegenfall, ohne den die Suche später im Leeren liefe: Ein Zauber
        // ohne Kandidatenzeile kann in keiner gefundenen Rotation vorkommen —
        // und die Suche meldete ihn als „nie gewählt", obwohl sie ihn nie zur
        // Wahl hatte.
        let mut spielbar = 0;
        for (index, spell) in SPELLS.iter().enumerate() {
            // Prüf-Zauber sind ausgenommen — sie sollen KEINE Zeile bekommen.
            if spell.unlock_level > MAX_LEVEL as u16 {
                assert!(
                    candidates_for(index).is_empty(),
                    "Der Pruef-Zauber {} steht in der Suche",
                    spell.id
                );
                continue;
            }
            spielbar += 1;
            let zeilen = candidates_for(index);
            assert!(
                !zeilen.is_empty(),
                "Zauber {index} ({}) hat keine Kandidatenzeile",
                spell.id
            );
            assert!(
                zeilen.iter().any(|k| k.origin == Origin::Always),
                "Zauber {} hat keine unbedingte Zeile — dann kann die Suche \
                 nicht herausfinden, dass er ohne Bedingung besser laeuft",
                spell.id
            );
        }
        assert!(
            spielbar > 1,
            "nur {spielbar} spielbare Zauber — dann prueft der Rest nichts"
        );
    }

    #[test]
    fn ein_brand_bekommt_eine_nachlege_zeile() {
        let index = index_of("smoulder").expect("smoulder fehlt");
        assert!(
            SPELLS[index].dot.is_some(),
            "smoulder hat keinen Brand mehr — dann prueft dieser Fall nichts"
        );
        let zeilen = candidates_for(index);
        let nachlegen = zeilen
            .iter()
            .find(|k| k.origin == Origin::DotRefresh)
            .expect("keine Nachlege-Zeile");
        assert_eq!(
            nachlegen.condition,
            single(
                Value::DotRemains(index),
                Compare::Less,
                Value::Literal(i64::from(GCD_MS))
            )
        );
    }

    #[test]
    fn ein_zauber_ohne_brand_bekommt_keine() {
        let index = index_of("kindle").expect("kindle fehlt");
        assert!(
            SPELLS[index].dot.is_none(),
            "kindle hat einen Brand bekommen"
        );
        // Der Gegenfall zur Ableitung: Sie darf nicht jedem alles geben.
        // Waere sie grosszuegig, entstuenden Zeilen wie „kindle nachlegen",
        // die nie zutreffen — und die Suche verbraeuchte Messungen an
        // Kandidaten, die nichts tun koennen.
        assert!(candidates_for(index)
            .iter()
            .all(|k| k.origin != Origin::DotRefresh));
    }

    #[test]
    fn die_entlade_schwelle_stammt_aus_den_daten_und_nicht_von_hier() {
        // Das ist der Kern von Schnitt 1: Die Zahl in der Bedingung wird nicht
        // gesucht und nicht geraten, sie steht im Datensatz. Aendert jemand
        // dort die Schwelle, aendert sich die Bedingung mit — ohne dass hier
        // eine Zeile angefasst wird.
        let mut gefunden = 0;
        for (index, spell) in SPELLS.iter().enumerate() {
            let Some(spender) = spell.spender else {
                continue;
            };
            gefunden += 1;
            let zeile = candidates_for(index)
                .into_iter()
                .find(|k| k.origin == Origin::SpenderThreshold)
                .unwrap_or_else(|| panic!("{} hat keine Schwellen-Zeile", spell.id));
            assert_eq!(
                zeile.condition,
                single(
                    Value::Resource,
                    Compare::GreaterEqual,
                    Value::Literal(i64::from(spender.bonus_threshold))
                )
            );
        }
        assert!(
            gefunden > 0,
            "Kein einziger Zauber hat einen Entlader — dann prueft dieser Fall nichts"
        );
    }

    #[test]
    fn die_gesamtliste_ist_groesser_als_die_zahl_der_zauber() {
        // Wenn beide Zahlen gleich waeren, haette kein Zauber eine abgeleitete
        // Bedingung bekommen und die Ableitung liefe leer — grün, aber ohne
        // Wirkung.
        let alle = all_candidates();
        // **Gegen die SPIELBAREN Zauber, nicht gegen alle Eintraege.**
        // `SPELL_COUNT` zaehlt auch die Pruef-Zauber ueber der Stufengrenze
        // (Issue #489) — und genau die leiten keine Bedingung ab, weil
        // `all_candidates` sie ueberspringt. Mit dem rohen Zaehler waere die
        // Pruefung mit jedem neuen Pruef-Zauber naeher an ihrem Schwellwert
        // gefallen, ohne dass an der Ableitung etwas kaputt gewesen waere.
        let spielbare = SPELLS
            .iter()
            .filter(|s| s.unlock_level <= MAX_LEVEL as u16)
            .count();
        assert!(
            alle.len() > spielbare,
            "{} Kandidaten fuer {spielbare} spielbare Zauber — keine einzige Bedingung abgeleitet",
            alle.len()
        );
    }

    #[test]
    fn die_reihenfolge_ist_bei_zwei_laeufen_dieselbe() {
        // GAME-DESIGN 18.4. Bei gleichwertigen Kandidaten entscheidet die
        // Reihenfolge, welcher gewinnt — streute sie, streute das Ergebnis.
        assert_eq!(all_candidates(), all_candidates());
    }
}
