//! Die Aktions-Prioritätenliste (APL) und ihre Auswertung.
//!
//! Aus GAME-DESIGN 18.5, dem zweiten von SimulationCraft übernommenen Konzept:
//!
//! > Die Rotation als geordnete Regelliste: „Wirke A, wenn Bedingung X. Sonst B,
//! > wenn Y." Beherrschbarer Suchraum, und das Ergebnis ist für Menschen lesbar.
//!
//! # Warum eine eigene Sprache und keine Rust-Funktionen
//!
//! Bis hierher war eine Rotation eine Funktion ([`crate::rotation`]). Das genügt
//! für vier feste Prüfrotationen und für nichts darüber hinaus: Der Balance-Läufer
//! (E2-4) soll Rotationen **verändern**, vergleichen und die beste finden. Eine
//! Regelliste als Text lässt sich erzeugen, abwandeln und in einen Bericht
//! schreiben; eine Rust-Funktion muss neu übersetzt werden.
//!
//! Der zweite Grund steht ebenfalls in 18.5: **lesbar für Menschen.** Ein
//! Balance-Ergebnis, das niemand nachvollziehen kann, ist keine Entscheidungs-
//! grundlage.
//!
//! # Die Sprache in einer Zeile
//!
//! ```text
//! <zauber> [if <bedingung>]
//! ```
//!
//! Eine Regel je Zeile, in absteigender Priorität. Die erste Regel, deren
//! Bedingung zutrifft **und deren Zauber wirkbar ist**, gewinnt. Ohne `if` gilt
//! die Regel immer — das ist der Füller am Ende.
//!
//! ```text
//! # Die gute Rotation aus STEP1-SPEC § 4
//! smoulder if dot.smoulder.remains < gcd
//! pyre     if heat >= 15
//! kindle
//! ```
//!
//! ## Dass der Zauber wirkbar sein muss, steht nicht in der Liste
//!
//! Eine Regel wird übersprungen, wenn ihre Abklingzeit noch läuft — ohne dass
//! das jemand hinschreiben müsste. Das ist SimulationCraft-Semantik und hat zwei
//! Gründe: Die Listen werden kürzer und damit lesbar, und eine vergessene
//! Bereitschafts-Prüfung kann nicht dazu führen, dass ein Zauber gewirkt wird,
//! den es gerade nicht gibt. Die Referenz-Rotationen prüfen `ready(...)` in jeder
//! Regel ausdrücklich; als APL fällt das weg, ohne dass sich das Ergebnis ändert.
//!
//! # Werte, die eine Bedingung lesen kann
//!
//! | Ausdruck | Bedeutung |
//! | --- | --- |
//! | `<ressource>` (z. B. `heat`) | Aktueller Stand der Schul-Mechanik |
//! | `resource` | Dasselbe, unabhängig vom Namen der Schule |
//! | `cooldown.<zauber>.ready` | Abklingzeit abgelaufen |
//! | `cooldown.<zauber>.remains` | Restliche Abklingzeit in ms |
//! | `dot.<zauber>.active` | Brand-Effekt läuft |
//! | `dot.<zauber>.remains` | Restlaufzeit in ms |
//! | `dot.<zauber>.ticks` | Bisherige Ticks |
//! | `buff.<zauber>.active` | Verstärkung dieses Zaubers läuft |
//! | `buff.<zauber>.remains` | Ihre Restlaufzeit in ms |
//! | `boss.hp` | Verbleibende Lebenspunkte |
//! | `boss.hp_pct` | Verbleibende Lebenspunkte in Prozent (ganzzahlig) |
//! | `boss.casting` | Ein Boss-Zauber läuft gerade |
//! | `boss.cast_remains` | Zeit bis der **laufende** Zauber landet, in ms |
//! | `boss.next_strike_in` | Zeit bis der **nächste** Eintrag ansteht, in ms |
//! | `time` | Verstrichene Kampfzeit in ms |
//! | `fight.remains` | Zeit bis zum Überdruck in ms |
//! | `gcd` | Die globale Wartezeit als Zahl |
//! | `decay` | Stapel der Schwächung (PUTREFACTION) |
//! | `shield` | Die Schutzschicht (COAGULATION) |
//! | `control` | Kontroll-Punkte (FIXATION) |
//!
//! **Sieben dieser Zeilen fehlten bis #797**, obwohl der Parser die Werte längst
//! las: die drei Schul-Mechaniken aus #526, die beiden Verstärkungs-Werte aus
//! #553 und `boss.casting`/`boss.cast_remains` aus #797 selbst. Eine Tabelle
//! neben einer Aufzählung im Code läuft auseinander, und niemand merkt es —
//! `alle_dokumentierten_werte_lassen_sich_lesen` in
//! `sim/tests/apl_dokumentierte_werte.rs` fängt seitdem wenigstens die eine
//! Richtung.
//!
//! Zahlen sind ganzzahlig. Zeiten dürfen `1500`, `1500ms` oder `1.5s` heißen —
//! immer ganzzahlig gerechnet, denn Fließkomma-Zeit ist die erste Regel, die
//! GAME-DESIGN 18.4 verbietet.
//!
//! Vergleiche: `<`, `<=`, `>`, `>=`, `==`, `!=`. Verknüpfung mit `and` und `or`,
//! wobei `and` stärker bindet. Klammern gibt es bewusst nicht — sie würden den
//! Auswerter verzweigen lassen, und keine bisher gebrauchte Rotation braucht sie.
//!
//! # Zeitabhängige Bedingungen wecken den Simulator
//!
//! Der Kampf springt von Ereignis zu Ereignis und befragt die Rotation nur dort
//! ([`crate::fight`]). Eine Bedingung wie `dot.smoulder.remains < gcd` wird aber
//! durch **bloßes Verstreichen von Zeit** wahr, ohne dass ein Ereignis anliegt.
//!
//! Das war der Vorabfund aus Issue #53: In den Prüfrotationen fällt es nicht auf,
//! weil der Füller den GCD ständig neu auslöst und dadurch dicht genug geweckt
//! wird. Eine Rotation mit Wartezeiten träfe den Zeitpunkt nicht.
//!
//! [`Apl::next_wakeup`] rechnet deshalb aus, wann die **früheste** zeitabhängige
//! Bedingung ihren Wahrheitswert ändern kann, und der Kampf plant dafür einen
//! Weckruf ein. Damit ist die Grenze aus #53 nicht nur beschrieben, sondern weg.

mod parse;

pub use parse::{parse, AplError, AplName};

use crate::data::{Spell, MAX_LEVEL, RESOURCE, SPELLS};
use crate::fight::Fight;
use crate::time::{Millis, GCD_MS};

/// Eine Zahl, wie die Sprache sie kennt: ganzzahlig, mit Vorzeichen.
///
/// `i64` statt `u32`, weil Vergleiche Zwischenergebnisse erzeugen können, die
/// unter null gehen — und ein stillschweigender Unterlauf wäre genau die Art
/// Fehler, die ein Balance-Ergebnis unbrauchbar macht, ohne aufzufallen.
pub type Number = i64;

/// Woher ein Wert stammt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    /// Eine feste Zahl aus der Liste.
    Literal(Number),
    /// Der Stand der Schul-Ressource.
    Resource,
    /// Restliche Abklingzeit eines Zaubers.
    CooldownRemains(usize),
    /// Ob die Abklingzeit abgelaufen ist.
    CooldownReady(usize),
    /// Restlaufzeit eines Brand-Effekts.
    DotRemains(usize),
    /// Ob ein Brand-Effekt läuft.
    DotActive(usize),
    /// Bisherige Ticks eines Brand-Effekts.
    DotTicks(usize),
    /// Verbleibende Lebenspunkte des Bosses.
    BossHp,
    /// Verbleibende Lebenspunkte in Prozent, ganzzahlig abgerundet.
    BossHpPercent,
    /// Ob der Boss gerade einen Zauber wirkt (Issue #797).
    ///
    /// **Ohne diesen Wert war Reaktionsspiel für die Suche unausdrückbar.**
    /// Der Spieler sieht den Cast (`src/render/hud.ts` führt `castStart` und
    /// `casting`, dazu zwei Klänge) und kann antworten; die Rotationssuche
    /// konnte es nicht. `QUENCH` und `PIN` waren nur über `time >= 15000`
    /// ansteuerbar — also über eine fest verdrahtete Zahl aus der Zeitleiste
    /// eines einzelnen Bosses.
    ///
    /// Was daran hing, ist gemessen (#472): An einem Boss mit einem Schlag,
    /// den man abwenden MUSS, wählte der Balance-Läufer bei keinem Gewicht
    /// einen FIXATION-Zauber. Nicht weil die Schule zu schwach ist, sondern
    /// weil die Bedingung fehlte, unter der sie sich lohnt.
    BossCasting,
    /// Millisekunden, bis der laufende Boss-Zauber landet (Issue #797).
    ///
    /// **Warum das nicht aus `BossCasting` folgt:** Mit „wirkt gerade" lässt
    /// sich nur „jetzt sofort" ausdrücken. PIN muss aber früh genug fallen,
    /// dass QUENCH danach noch in sein Fenster trifft — das ist eine Aussage
    /// über die Restzeit, nicht über den Zustand.
    ///
    /// **0, wenn nichts läuft** — dieselbe Übereinkunft wie [`Value::DotRemains`]
    /// bei erloschenem Brand. Das ist unbequem: `boss.cast_remains < 2000`
    /// trifft dann auch, wenn der Boss gar nichts wirkt, und jede Zeile muss
    /// zusätzlich `boss.casting` prüfen. Ein sehr großer Ersatzwert wäre hier
    /// bequemer gewesen, hätte aber zwei Bedeutungen von „remains" in
    /// dieselbe Sprache gebracht — und die unbequeme Übereinkunft ist die,
    /// die man später ohne Bruch ändern kann.
    BossCastRemains,
    /// Millisekunden, bis der nächste Zeitleisten-Eintrag ansteht (Issue #797).
    ///
    /// **Warum das nicht aus [`Value::BossCastRemains`] folgt:** Der beschreibt
    /// den *laufenden* Zauber. PIN gilt laut Zauberdaten dem **nächsten** und
    /// verpufft, wenn er einen ohnehin unterbrechbaren trifft — mit
    /// `boss.cast_remains` lässt sich also gar keine brauchbare Zeile für ihn
    /// bilden. Genau daran ist AC2 von #797 gescheitert: `pin if boss.casting`
    /// feuert in dem Moment, in dem PIN wirkungslos ist.
    ///
    /// **Wenn kein Schlag mehr kommt, die verbleibende Kampfzeit** — also
    /// derselbe Wert wie [`Value::FightRemains`]. Das ist die Übereinkunft,
    /// und sie ist eine andere als bei `BossCastRemains` (dort 0). Der Grund
    /// ist nicht Bequemlichkeit: Für den laufenden Zauber gibt es mit
    /// [`Value::BossCasting`] einen Zustandswert, der den leeren Fall abfängt;
    /// für den kommenden gibt es keinen. Eine 0 träfe `< 2000` am Kampfende
    /// dauerhaft — ausgerechnet dort, wo nichts mehr kommt.
    ///
    /// **Er sagt „ansteht", nicht „landet".** Ein Eintrag mit `phase` wird
    /// übersprungen, wenn der Boss die Phase bis dahin nicht erreicht hat, und
    /// geprüft wird das erst zum Fälligkeitszeitpunkt. Der Simulator kann es
    /// nicht besser wissen — und der Spieler weiß es auch nicht.
    BossNextStrikeIn,
    /// Verstrichene Kampfzeit.
    Time,
    /// Zeit bis zum Überdruck.
    FightRemains,
    /// Die globale Wartezeit.
    Gcd,
    /// Die Zahl der liegenden Zersetzungs-Stapel (PUTREFACTION, Issue #526).
    ///
    /// **Die Kernentscheidung dieser Schule** (GAME-DESIGN 13.1): Der Aufbau
    /// vor einem grossen Schadensfenster. Ohne diesen Wert kann eine Rotation
    /// sie nicht ausdruecken — sie drueckt Knoepfe in ihrer Reihenfolge.
    Decay,
    /// Der Stand der Schutzschicht (COAGULATION, Issue #526).
    Shield,
    /// Die Kontroll-Punkte (FIXATION, Issue #526).
    Control,
    /// Laeuft das Fenster der Verstaerkung, die dieser Zauber legt? (#553)
    ///
    /// # Warum nach ZAUBER und nicht nach Art
    ///
    /// Verstaerkungen liegen nach ART und Zielgroesse (Issue #489), nicht
    /// nach Zauber — `buffs.factors[kind]` je Vorrat. Eine Rotation kennt
    /// aber Zauber, keine Arten: `kindle if buff.flashpoint.active` ist die
    /// Frage, die ein Spieler stellt.
    ///
    /// Der Zauber-Index wird deshalb beim Auswerten in (Vorrat, Art)
    /// uebersetzt. Traegt der Zauber gar keine Verstaerkung, ist die Antwort
    /// `false` — das ist kein Fehler, sondern eine Bedingung, die nie greift.
    BuffActive(usize),
    /// Wie lange das Fenster dieses Zaubers noch laeuft, in ms (#553).
    BuffRemains(usize),
}

impl Value {
    /// Liest den Wert aus dem Kampfzustand.
    ///
    /// Wahrheitswerte werden als 1 und 0 gelesen; damit braucht der Auswerter
    /// nur einen Zahlentyp, und `dot.smoulder.active == 1` ist ausdrückbar,
    /// ohne dass es jemand so schreiben müsste.
    #[must_use]
    pub fn read(self, fight: &Fight) -> Number {
        match self {
            Value::Literal(n) => n,
            Value::Resource => Number::from(fight.resource()),
            Value::CooldownRemains(i) => Number::from(fight.cooldown_remaining_ms(i)),
            Value::CooldownReady(i) => Number::from(u8::from(fight.cooldown_ready(i))),
            Value::DotRemains(i) => Number::from(fight.dot_remaining_ms(i)),
            Value::DotActive(i) => Number::from(u8::from(fight.dot_active(i))),
            Value::DotTicks(i) => Number::from(fight.dot_tick_count(i)),
            Value::BossHp => Number::from(fight.boss_hp_remaining()),
            Value::BossHpPercent => Number::from(fight.boss_hp_percent()),
            Value::BossCasting => Number::from(u8::from(fight.casting().is_some())),
            Value::BossCastRemains => Number::from(fight.boss_cast_remaining_ms()),
            Value::BossNextStrikeIn => Number::from(fight.next_strike_in_ms()),
            Value::Time => Number::from(fight.time()),
            Value::FightRemains => Number::from(fight.fight_remaining_ms()),
            Value::Gcd => Number::from(GCD_MS),
            Value::Decay => Number::from(u32::try_from(fight.decay_stacks()).unwrap_or(u32::MAX)),
            Value::Shield => Number::from(fight.shield()),
            Value::Control => Number::from(fight.control_points()),
            Value::BuffActive(i) => Number::from(u8::from(fight.buff_active(i))),
            Value::BuffRemains(i) => Number::from(fight.buff_remaining_ms(i)),
        }
    }

    /// Ob sich dieser Wert allein durch verstreichende Zeit ändert.
    ///
    /// Genau diese Werte brauchen einen Weckruf — alle anderen ändern sich nur,
    /// wenn ohnehin ein Ereignis eintritt.
    ///
    /// **Ein exhaustives `match` ohne Auffang-Zweig, und das ist der Punkt.**
    ///
    /// Hier stand ein `matches!`. Das ist nicht exhaustiv: Ein neuer Wert, den
    /// niemand einträgt, gilt stillschweigend als zeitunabhängig, und der
    /// Compiler schweigt dazu. Genau das ist beim Bau von [`Value::BossCastRemains`]
    /// (#797) passiert — gefunden hat es kein Prüflauf, sondern das Lesen
    /// dieser Liste.
    ///
    /// Mit `match` und ohne `_` ist es keine Warnung mehr, sondern ein Fehler
    /// beim Übersetzen: Wer einen Wert hinzufügt, MUSS ihn einordnen. Die
    /// Kosten sind zwei Zeilen pro neuem Wert; der Ertrag ist, dass die
    /// Fehlentscheidung gar nicht erst compilieren kann.
    pub const fn is_time_dependent(self) -> bool {
        match self {
            // Diese Werte ändern sich mit der Uhr, ohne dass ein Ereignis
            // eintritt — ohne Weckruf würde die Rotation ihr Fenster verpassen.
            Value::CooldownRemains(_)
            | Value::CooldownReady(_)
            | Value::DotRemains(_)
            | Value::DotActive(_)
            | Value::BuffActive(_)
            | Value::BuffRemains(_)
            | Value::Time
            | Value::FightRemains
            // Die Restzeit eines Boss-Zaubers sinkt mit jeder Millisekunde (#797).
            | Value::BossCastRemains
            // Und die Zeit bis zum nächsten Schlag ebenso (#797). Der Wecker
            // ist hier nicht Beiwerk, sondern der Zweck: Ein Fenster wie
            // `boss.next_strike_in <= 2500` öffnet sich ausschließlich durch
            // Verstreichen — es gibt kein Ereignis, das es aufstößt.
            | Value::BossNextStrikeIn => true,
            // Diese ändern sich nur an Ereignissen, die die Rotation ohnehin
            // wecken.
            //
            // `BossCasting` gehört hierher, obwohl es einen Boss-Zauber
            // betrifft: Sein Wert kippt an genau zwei Stellen — der Zauber
            // beginnt, der Zauber landet. Beide sind Ereignisse.
            Value::Literal(_)
            | Value::Resource
            | Value::DotTicks(_)
            | Value::BossHp
            | Value::BossHpPercent
            | Value::BossCasting
            | Value::Gcd
            | Value::Decay
            | Value::Shield
            | Value::Control => false,
        }
    }
}

/// Wie zwei Werte verglichen werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
}

impl Compare {
    #[must_use]
    const fn apply(self, left: Number, right: Number) -> bool {
        match self {
            Compare::Less => left < right,
            Compare::LessEqual => left <= right,
            Compare::Greater => left > right,
            Compare::GreaterEqual => left >= right,
            Compare::Equal => left == right,
            Compare::NotEqual => left != right,
        }
    }
}

/// Ein einzelner Vergleich, etwa `heat >= 15`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Term {
    /// Linke Seite.
    pub left: Value,
    /// Vergleichsart.
    pub compare: Compare,
    /// Rechte Seite.
    pub right: Value,
}

impl Term {
    #[must_use]
    fn holds(self, fight: &Fight) -> bool {
        self.compare
            .apply(self.left.read(fight), self.right.read(fight))
    }
}

/// Eine Bedingung: `or`-verknüpfte Gruppen von `and`-verknüpften Vergleichen.
///
/// Die Form ergibt sich daraus, dass `and` stärker bindet als `or` und es keine
/// Klammern gibt — jede Bedingung lässt sich damit als „eine dieser Gruppen muss
/// vollständig zutreffen" schreiben.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Condition {
    /// Die `or`-Gruppen. Leer heißt: trifft immer zu.
    pub groups: Vec<Vec<Term>>,
}

impl Condition {
    /// Ob die Bedingung im aktuellen Zustand zutrifft.
    #[must_use]
    pub fn holds(&self, fight: &Fight) -> bool {
        if self.groups.is_empty() {
            return true;
        }
        self.groups
            .iter()
            .any(|group| group.iter().all(|term| term.holds(fight)))
    }

    /// Ob die Bedingung überhaupt keine Vergleiche enthält.
    #[must_use]
    pub fn is_always(&self) -> bool {
        self.groups.is_empty()
    }

    /// Alle Vergleiche, unabhängig von der Gruppierung.
    fn terms(&self) -> impl Iterator<Item = &Term> {
        self.groups.iter().flatten()
    }
}

/// Eine Regel: ein Zauber und die Bedingung, unter der er gewirkt wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// Index des Zaubers in [`crate::data::SPELLS`].
    pub spell: usize,
    /// Wann die Regel greift.
    pub condition: Condition,
    /// Zeile in der Quelldatei — für Fehlermeldungen und Berichte.
    pub line: usize,
}

/// Eine vollständige Prioritätenliste.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Apl {
    /// Die Regeln in absteigender Priorität.
    pub rules: Vec<Rule>,
    /// Name der Liste, für Berichte.
    pub name: String,
}

impl crate::fight::Decide for Apl {
    fn decide(&self, fight: &Fight) -> Option<usize> {
        Apl::decide(self, fight)
    }

    fn next_wakeup(&self, fight: &Fight) -> Option<Millis> {
        Apl::next_wakeup(self, fight)
    }
}

impl Apl {
    /// Entscheidet, welcher Zauber jetzt gewirkt wird.
    ///
    /// Die erste Regel gewinnt, deren Zauber wirkbar ist **und** deren Bedingung
    /// zutrifft. `None` heißt warten.
    ///
    /// Fordert keinen Speicher an: Es wird nur gelesen und verglichen.
    #[must_use]
    pub fn decide(&self, fight: &Fight) -> Option<usize> {
        for rule in &self.rules {
            if !fight.is_ready(rule.spell) {
                continue;
            }
            if rule.condition.holds(fight) {
                return Some(rule.spell);
            }
        }
        None
    }

    /// Wann eine zeitabhängige Bedingung frühestens umschlagen kann.
    ///
    /// Der Kampf springt von Ereignis zu Ereignis; ohne diesen Wert würde eine
    /// Bedingung, die allein durch verstreichende Zeit wahr wird, erst beim
    /// nächsten ohnehin anstehenden Ereignis bemerkt — und das kann Sekunden
    /// später sein (Vorabfund aus Issue #53).
    ///
    /// Zurückgegeben wird ein Zeitpunkt **echt nach** `fight.time()`, oder
    /// `None`, wenn keine Bedingung von der Uhr abhängt.
    #[must_use]
    pub fn next_wakeup(&self, fight: &Fight) -> Option<Millis> {
        let now = fight.time();
        let mut earliest: Option<Millis> = None;

        for rule in &self.rules {
            for term in rule.condition.terms() {
                for (value, other) in [(term.left, term.right), (term.right, term.left)] {
                    if !value.is_time_dependent() {
                        continue;
                    }
                    // Der Schwellwert ist nur dann bekannt, wenn die andere
                    // Seite selbst nicht von der Uhr abhängt. Zwei zeitabhängige
                    // Seiten (`dot.a.remains < dot.b.remains`) laufen aufeinander
                    // zu; dafür gäbe es einen Umschlagpunkt, aber keine der
                    // gebrauchten Rotationen schreibt so etwas. Solche Vergleiche
                    // werden hier übersprungen — sie werden weiterhin an jedem
                    // Ereignis geprüft, nur eben nicht dazwischen.
                    if other.is_time_dependent() {
                        continue;
                    }
                    if let Some(at) = switch_point(value, other.read(fight), fight) {
                        if at > now {
                            earliest = Some(earliest.map_or(at, |best| best.min(at)));
                        }
                    }
                }
            }
        }
        earliest
    }
}

/// Wann ein zeitabhängiger Wert die Schwelle `threshold` erreicht.
///
/// Alle diese Werte laufen **gleichmäßig herunter** (Restzeiten) oder **hinauf**
/// (die Uhr), je Millisekunde um eins. Der Umschlagpunkt ist damit eine einfache
/// Rechnung und keine Suche.
///
/// # Exhaustiv, seit hier ein Wert durchgefallen ist (#797)
///
/// **Hier stand ein `_ => None`, und das war die eigentliche Lücke.**
/// [`Value::is_time_dependent`] ist bewusst exhaustiv, damit ein neuer Wert
/// eingeordnet werden MUSS — aber diese Einordnung entscheidet nur, ob
/// [`Apl::next_wakeup`] den Wert überhaupt ansieht. Terminiert wird der Weckruf
/// **hier**, und der Auffang-Zweig gab für jeden nicht eingetragenen Wert
/// stillschweigend „kein Umschlagpunkt" zurück.
///
/// Gemessen an `Value::BossNextStrikeIn`: Der Wert stand ordnungsgemäß in
/// `is_time_dependent`, und trotzdem wurde kein Weckruf gestellt. Die Bedingung
/// schlug erst um, wenn ein **fremdes** Ereignis den Kampf ohnehin anhielt —
/// die gemessenen Wurfzeitpunkte lagen auf einem 1000-ms-Raster statt auf dem
/// Umschlagpunkt (10000 statt 9000, 8000 statt 7500), und bei einem engen
/// Fenster fiel die Entscheidung gar nicht.
///
/// **Das ist der Wächter am falschen Ende**: Er stand an der Frage „hängt der
/// Wert an der Uhr?" statt an der Frage „wann schlägt er um?". Die zweite ist
/// die, deren Antwort gebraucht wird. Ohne Auffang-Zweig muss jeder neue Wert
/// hier beantwortet werden — auch mit `None`, dann aber sichtbar und begründet.
fn switch_point(value: Value, threshold: Number, fight: &Fight) -> Option<Millis> {
    let now = fight.time();
    let at = |remaining_now: Millis, target: Number| -> Option<Millis> {
        // Ein Restwert sinkt je Millisekunde um eins: Er erreicht `target` in
        // `remaining_now - target` Millisekunden.
        let delta = Number::from(remaining_now) - target;
        if delta <= 0 {
            return None;
        }
        u32::try_from(delta).ok().map(|d| now.saturating_add(d))
    };

    match value {
        Value::CooldownRemains(i) => at(fight.cooldown_remaining_ms(i), threshold),
        Value::CooldownReady(i) => {
            let remains = fight.cooldown_remaining_ms(i);
            (remains > 0).then(|| now.saturating_add(remains))
        }
        Value::DotRemains(i) => at(fight.dot_remaining_ms(i), threshold),
        Value::DotActive(i) => {
            let remains = fight.dot_remaining_ms(i);
            (remains > 0).then(|| now.saturating_add(remains))
        }
        Value::Time => {
            let target = u32::try_from(threshold).ok()?;
            (target > now).then_some(target)
        }
        Value::FightRemains => at(fight.fight_remaining_ms(), threshold),
        // Der Wert, für den diese Funktion exhaustiv gemacht wurde (#797). Er
        // sinkt je Millisekunde um eins wie jede Restzeit — und anders als bei
        // den Werten darunter gibt es KEIN Ereignis, das das Fenster öffnet:
        // Der Zeitleisten-Eintrag ist eingeplant, aber erst zu seiner
        // Fälligkeit. Eine Zeile `boss.next_strike_in <= 3000` fragt nach dem
        // Zeitpunkt 3000 ms DAVOR, und dort steht nichts in der Schlange.
        Value::BossNextStrikeIn => at(fight.next_strike_in_ms(), threshold),
        // **Ab hier: kein Umschlagpunkt, und zwar mit Grund.**
        //
        // Diese Werte hängen zwar an der Uhr, aber ihr Umschlag fällt mit
        // einem Ereignis zusammen, das den Kampf ohnehin anhält:
        //
        //   BuffActive/BuffRemains  die Verstärkung stellt beim Wirken einen
        //                           Weckruf auf ihr Ende (`fight.rs`, „Dauer
        //                           UND Wakeup mit demselben Wert")
        //   BossCastRemains         der Einschlag steht als `BossCastLands`
        //                           in der Schlange
        //
        // **Für `BossCastRemains` ist das nur die halbe Wahrheit**, und das
        // gehört hier hin statt in ein Gedächtnis: Das Ereignis liegt am ENDE
        // des Casts. Eine Zeile `boss.cast_remains < 500` will 500 ms davor
        // entscheiden und wird bis heute nur bemerkt, wenn zufällig etwas
        // anderes weckt — dieselbe Lücke, die `BossNextStrikeIn` oben
        // geschlossen bekommt. Sie wird hier NICHT mitgeschlossen: Ein
        // zusätzlicher Weckruf ändert gefundene Rotationen und damit gemessene
        // Zahlen, und das ist eine eigene Entscheidung mit eigener Messung.
        Value::BuffActive(_) | Value::BuffRemains(_) | Value::BossCastRemains => None,
        // Und diese ändern sich überhaupt nicht durch bloßes Verstreichen —
        // `is_time_dependent` sortiert sie als `false` ein, und
        // `Apl::next_wakeup` fragt hier gar nicht erst nach. Der Zweig steht
        // trotzdem, weil ohne ihn der Auffang-Zweig zurückkäme, den diese
        // Funktion gerade losgeworden ist.
        Value::Literal(_)
        | Value::Resource
        | Value::DotTicks(_)
        | Value::BossHp
        | Value::BossHpPercent
        | Value::BossCasting
        | Value::Gcd
        | Value::Decay
        | Value::Shield
        | Value::Control => None,
    }
}

/// Namen, unter denen die Schul-Ressource ansprechbar ist.
///
/// Beides ist erlaubt: der Name aus `data/` (`heat`) für lesbare Listen, und
/// `resource` für Listen, die über Schulgrenzen hinweg funktionieren sollen.
#[must_use]
pub fn resource_names() -> [&'static str; 2] {
    [RESOURCE.id, "resource"]
}

/// Sucht einen Zauber für die Sprache.
#[must_use]
pub fn spell_index(name: &str) -> Option<usize> {
    SPELLS
        .iter()
        .position(|spell| spell.id == name && ist_spielbar(spell))
}

/// Gehoert dieser Zauber zum Spiel? (Issue #499)
///
/// **Die Bedingung steht genau hier und sonst nirgends.** Vorher trugen
/// `spell_index` und `spielbare_zauber` denselben Vergleich zweimal — die
/// Wirkung war richtig, aber wer `MAX_LEVEL` spaeter durch eine Funktion oder
/// eine Konfiguration ersetzt, aendert vielleicht nur eine der beiden. Dann
/// schlaegt die Liste Zauber vor, die das Wirken abweist, oder umgekehrt.
///
/// Gefunden hat es der Orchestrator bei der Abnahme: Sein `sed` traf nur eine
/// der beiden Stellen, und trotzdem fiel nur ein Test.
#[must_use]
pub fn ist_spielbar(spell: &Spell) -> bool {
    spell.unlock_level <= MAX_LEVEL as u16
}

/// Die Zauber, die eine Rotation ueberhaupt nennen darf.
///
/// **Nicht `SPELLS` selbst** (Issue #499): Pruef-Zauber liegen ueber der
/// Stufengrenze und kommen im Spiel nicht vor. Wer sie in einer Rotation
/// nennt, misst mit dem Balance-Laeufer etwas, das es nicht gibt — und nichts
/// wird dabei rot.
///
/// Dieselbe Grenze wie in der Schadens-Tabelle des Berichts und in der
/// Kandidatenzaehlung der Rotationssuche. Alle drei fragen: gehoert dieser
/// Eintrag zum Spiel?
pub fn spielbare_zauber() -> impl Iterator<Item = &'static Spell> {
    // **Nicht die Bedingung wiederholen** — sie steht in `ist_spielbar`.
    // `position()` auf DIESEM Strom waere uebrigens falsch: Er gaebe den Platz
    // im gefilterten Strom zurueck, nicht den Index in `SPELLS`. Deshalb teilen
    // sich die beiden die Bedingung und nicht den Iterator.
    SPELLS.iter().filter(|s| ist_spielbar(s))
}

use std::fmt;

// ---------------------------------------------------------------------------
// Die Gegenrichtung zum Parser (Issue #237, AC3)
// ---------------------------------------------------------------------------
//
// Bis hierher war die APL-Seite eine EINBAHNSTRASSE: Text -> Struktur. Die
// Rueckrichtung gab es nicht, weil sie nie gebraucht wurde — bisher hat immer
// ein Mensch die Liste geschrieben.
//
// Mit der Rotations-Suche kehrt sich das um: Sie findet eine Struktur, und
// wenn niemand sie in eine `.apl`-Datei schreiben kann, hat sie sie fuer
// niemanden gefunden.
//
// **Geprueft wird diese Ausgabe ueber den Parser**, und zwar nicht auf
// Parsebarkeit, sondern auf Bedeutung: Die zurueckgelesene Fassung muss
// dieselbe Zahl messen. Ein Text, der durchgeht und etwas anderes bedeutet,
// ist schlimmer als einer, der nicht durchgeht — er faellt niemandem auf.

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(n) => write!(f, "{n}"),
            Self::Resource => write!(f, "{}", crate::data::RESOURCE.id),
            Self::CooldownRemains(i) => {
                write!(f, "cooldown.{}.remains", crate::data::SPELLS[*i].id)
            }
            Self::CooldownReady(i) => write!(f, "cooldown.{}.ready", crate::data::SPELLS[*i].id),
            Self::DotRemains(i) => write!(f, "dot.{}.remains", crate::data::SPELLS[*i].id),
            Self::DotActive(i) => write!(f, "dot.{}.active", crate::data::SPELLS[*i].id),
            Self::DotTicks(i) => write!(f, "dot.{}.ticks", crate::data::SPELLS[*i].id),
            Self::BossHp => write!(f, "boss.hp"),
            Self::BossHpPercent => write!(f, "boss.hp_pct"),
            Self::BossCasting => write!(f, "boss.casting"),
            Self::BossCastRemains => write!(f, "boss.cast_remains"),
            Self::BossNextStrikeIn => write!(f, "boss.next_strike_in"),
            Self::Time => write!(f, "time"),
            Self::FightRemains => write!(f, "fight.remains"),
            Self::Gcd => write!(f, "gcd"),
            Self::Decay => write!(f, "decay"),
            Self::Shield => write!(f, "shield"),
            Self::Control => write!(f, "control"),
            Self::BuffActive(i) => write!(f, "buff.{}.active", crate::data::SPELLS[*i].id),
            Self::BuffRemains(i) => write!(f, "buff.{}.remains", crate::data::SPELLS[*i].id),
        }
    }
}

impl fmt::Display for Compare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Equal => "==",
            Self::NotEqual => "!=",
        })
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.left, self.compare, self.right)
    }
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Die `or`-Gruppen werden mit `or` verbunden, die Vergleiche darin mit
        // `and` — dieselbe Schachtelung wie beim Lesen, sonst bedeutet der
        // ausgegebene Text etwas anderes als die Struktur.
        for (i, group) in self.groups.iter().enumerate() {
            if i > 0 {
                f.write_str(" or ")?;
            }
            for (j, term) in group.iter().enumerate() {
                if j > 0 {
                    f.write_str(" and ")?;
                }
                write!(f, "{term}")?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", crate::data::SPELLS[self.spell].id)?;
        if !self.condition.is_always() {
            write!(f, " if {}", self.condition)?;
        }
        Ok(())
    }
}

impl fmt::Display for Apl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rule in &self.rules {
            writeln!(f, "{rule}")?;
        }
        Ok(())
    }
}
