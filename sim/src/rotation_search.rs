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
use crate::data::{Spell, SPELLS, SPELL_COUNT};
use crate::time::GCD_MS;

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
}

impl Origin {
    /// Kurzbezeichnung für den Bericht.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Always => "unbedingt",
            Self::DotRefresh => "Brand nachlegen",
            Self::SpenderThreshold => "Entlade-Schwelle",
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
    };
    let result = crate::balance::run(config, &apl);
    Measured {
        apl,
        mean: result.damage.mean,
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
    pub pool_size: usize,
    /// Wie viele Rotationen tatsaechlich gemessen wurden.
    pub measured: usize,
    /// Warum die Suche aufgehoert hat.
    pub stop: Stop,
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
    let mut chosen: Vec<Candidate> = Vec::new();
    let mut offen: Vec<usize> = (0..pool.len()).collect();
    let mut measured = 0;
    let mut best = measure(config, "leer", &chosen);
    let stop = loop {
        if chosen.len() >= max_len {
            break Stop::LengthLimit;
        }
        if offen.is_empty() {
            break Stop::PoolExhausted;
        }
        // (Stelle im Pool, Einfuegeposition, Messung)
        let mut bester_zug: Option<(usize, usize, Measured)> = None;
        for (pos, &kandidat) in offen.iter().enumerate() {
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
    };

    SearchResult {
        best: measure(config, "gefunden", &chosen),
        chosen,
        pool_size: pool.len(),
        measured,
        stop,
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
    let mut best: Option<(Vec<Candidate>, Measured)> = None;
    let mut measured = 0;
    let mut aktuell: Vec<usize> = Vec::new();
    let mut benutzt = vec![false; pool.len()];

    fn rekursion(
        config: &BalanceConfig,
        pool: &[Candidate],
        max_len: usize,
        aktuell: &mut Vec<usize>,
        benutzt: &mut Vec<bool>,
        measured: &mut usize,
        best: &mut Option<(Vec<Candidate>, Measured)>,
    ) {
        if !aktuell.is_empty() {
            let zeilen: Vec<Candidate> = aktuell.iter().map(|&i| pool[i].clone()).collect();
            let gemessen = measure(config, "erschoepfend", &zeilen);
            *measured += 1;
            let besser = match best.as_ref() {
                None => true,
                Some((_, bisher)) => gemessen.mean > bisher.mean,
            };
            if besser {
                *best = Some((zeilen, gemessen));
            }
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
            rekursion(config, pool, max_len, aktuell, benutzt, measured, best);
            aktuell.pop();
            benutzt[i] = false;
        }
    }

    rekursion(
        config,
        pool,
        max_len,
        &mut aktuell,
        &mut benutzt,
        &mut measured,
        &mut best,
    );

    let (chosen, gemessen) = best.expect("kein einziger Kandidat — dann gibt es nichts zu suchen");
    SearchResult {
        best: gemessen,
        chosen,
        pool_size: pool.len(),
        measured,
        stop: Stop::PoolExhausted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::index_of;

    #[test]
    fn jeder_zauber_bekommt_mindestens_eine_zeile() {
        // Der Gegenfall, ohne den die Suche später im Leeren liefe: Ein Zauber
        // ohne Kandidatenzeile kann in keiner gefundenen Rotation vorkommen —
        // und die Suche meldete ihn als „nie gewählt", obwohl sie ihn nie zur
        // Wahl hatte.
        for index in 0..SPELL_COUNT {
            let zeilen = candidates_for(index);
            assert!(
                !zeilen.is_empty(),
                "Zauber {} ({}) hat keine Kandidatenzeile",
                index,
                SPELLS[index].id
            );
            assert!(
                zeilen.iter().any(|k| k.origin == Origin::Always),
                "Zauber {} hat keine unbedingte Zeile — dann kann die Suche \
                 nicht herausfinden, dass er ohne Bedingung besser laeuft",
                SPELLS[index].id
            );
        }
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
        for index in 0..SPELL_COUNT {
            let Some(spender) = SPELLS[index].spender else {
                continue;
            };
            gefunden += 1;
            let zeile = candidates_for(index)
                .into_iter()
                .find(|k| k.origin == Origin::SpenderThreshold)
                .unwrap_or_else(|| panic!("{} hat keine Schwellen-Zeile", SPELLS[index].id));
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
        assert!(
            alle.len() > SPELL_COUNT,
            "{} Kandidaten fuer {} Zauber — keine einzige Bedingung abgeleitet",
            alle.len(),
            SPELL_COUNT
        );
    }

    #[test]
    fn die_reihenfolge_ist_bei_zwei_laeufen_dieselbe() {
        // GAME-DESIGN 18.4. Bei gleichwertigen Kandidaten entscheidet die
        // Reihenfolge, welcher gewinnt — streute sie, streute das Ergebnis.
        assert_eq!(all_candidates(), all_candidates());
    }
}
