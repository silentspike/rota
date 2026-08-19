//! Wie viel ein Spieler höchstens aushält — **gemessen, nicht gerechnet** (#797).
//!
//! # Wofür diese Zahl gebraucht wird
//!
//! Issue #797 AC5 verlangt einen Boss mit einem Cast, den man verhindern
//! **muss**. „Muss" heißt: Der Schlag übersteigt alles, was der Spieler an
//! Verteidigung aufbauen kann — sonst wählt die Suche weiter COAGULATION, weil
//! ein Schild direkt fängt und ohne Bedingung wirkt.
//!
//! Diese Datei liefert die Zahl, gegen die der Schaden dieses Bosses zu
//! bemessen ist.
//!
//! # Warum sie GEMESSEN wird und nicht aus den Zauberdaten gerechnet
//!
//! Ein erster Entwurf sollte die Absorption aus `data/spells/coagulation.json`
//! ausrechnen — Beträge mal Stapel, über dieselbe Grenze wie der Kampf. Das
//! wäre ein **Nachbau der Kampfregel im Prüffall** gewesen, also eine zweite
//! Umsetzung derselben Sache.
//!
//! Ein Nachbau antwortet plausibel aus einer zweiten Quelle. Ändert jemand die
//! Stapelregel in `fight.rs`, rechnet der Test weiter nach der alten und bleibt
//! grün — genau die Klasse Fehler, gegen die er schützen soll.
//!
//! Hier läuft deshalb ein **echter Kampf**. Die Zahl entsteht in derselben
//! Maschinerie, die auch die Suche fährt: Ändert sich ein Betrag, eine
//! Stapelgrenze oder ein Talent, ändert sich diese Messung mit, ohne dass
//! jemand etwas nachpflegen muss.
//!
//! # Warum ALLE Anordnungen gefahren werden und nicht eine ausgewählte
//!
//! Die erste Fassung fuhr eine einzige Rotation, `plate crust scab`, und
//! begründete die Reihenfolge aus der Stapelregel. Der Orchestrator
//! widersprach: Der dritte Platz bringe als `crust` 900 statt 200, die Zahl sei
//! also eine Untergrenze. **Beide Rechnungen waren falsch, und zwar aus
//! demselben Grund:**
//!
//! Eine APL ist eine **Prioritätsliste, keine Sequenz** (`apl/mod.rs`: „Eine
//! Regel je Zeile, in absteigender Priorität. Die erste Regel, deren Bedingung
//! zutrifft und deren Zauber wirkbar ist, gewinnt."). Sie wird über den ganzen
//! Kampf immer wieder ausgewertet. „Drei Zeilen" heißt nicht „drei Drücke", und
//! keine der beiden Summen — 3500 hier, 4200 dort — beschreibt, was der Kampf
//! tut.
//!
//! Zwei Sitzungen haben unabhängig voneinander dieselbe Ableitung aus gelesenen
//! Regeln gebaut, und beide lagen daneben. Deshalb rät hier niemand mehr eine
//! Reihenfolge: Es werden **alle** Anordnungen aller nichtleeren Teilmengen der
//! spielbaren Schilde gefahren und das Maximum genommen. Das sind 64 Kämpfe von
//! je 12 Sekunden simulierter Zeit.
//!
//! # Warum `test-resistant`
//!
//! Seine Zeitleiste ist **leer** — kein Schlag verbraucht die Schicht, während
//! sie aufgebaut wird. Und mit 999999 Lebenspunkten endet der Kampf nicht
//! vorzeitig. Gemessen wird die Kapazität, nicht ihr Verbrauch.

use rota_sim::apl::{parse, AplName};
use rota_sim::data::boss_by_id;
use rota_sim::{Fight, FightConfig};

/// Der Zeitpunkt, zu dem die Schicht stehen muss.
///
/// Derselbe, an dem der geplante Boss aus AC5 zuschlägt. Der Zeitpunkt gehört
/// zur Messung dazu, weil Schichten **ablaufen**: `crucible-skin` hält 6000 ms,
/// `scab` 8000, `crust` 12000, `plate` 15000. Zu welchem Zeitpunkt welche
/// Kombination am meisten trägt, ist nicht offensichtlich — und genau deshalb
/// wird gesucht statt gerechnet.
const MESSZEITPUNKT_MS: u32 = 12_000;

/// Die spielbaren Schilde, abgelesen aus `data/spells/coagulation.json`.
///
/// # Wie diese Liste zustande kam
///
/// Über **alle** Zauberdateien nach einem `shield`-Block gesucht, nicht aus dem
/// Gedächtnis aufgeschrieben. Der erste Entwurf kannte nur drei davon:
/// `crucible-skin` (1800, der zweitstärkste) fehlte in den Listen beider
/// Sitzungen, die darüber gesprochen haben.
///
/// `test-channel-shield` steht bewusst nicht hier: `unlockLevel 101`, also
/// außerhalb dessen, was ein Spieler je lernt. Er würde die Obergrenze
/// aufblähen und den Boss aus AC5 gegen eine Verteidigung bemessen, die es im
/// Spiel nicht gibt.
const SCHILDE: [&str; 4] = ["plate", "crucible-skin", "crust", "scab"];

/// Ein Messergebnis samt der Anordnung, die es erzeugt hat.
///
/// Die Anordnung gehört dazu: Ohne sie wäre bei einem überraschenden Ergebnis
/// nicht erkennbar, ob die Suche etwas Sinnvolles gefunden hat.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Messung {
    schicht: u32,
    leben: u32,
    grenze: u32,
    anordnung: Vec<&'static str>,
}

/// Alle Anordnungen aller nichtleeren Teilmengen von [`SCHILDE`].
///
/// Für vier Zauber sind das 4 + 12 + 24 + 24 = 64 Listen. Teilmengen gehören
/// dazu, weil ein zusätzlicher Zauber in der Liste **schaden** kann: Er
/// verbraucht einen Platz im gemeinsamen Stapelzähler, der einem stärkeren
/// Schild fehlt.
fn alle_anordnungen() -> Vec<Vec<&'static str>> {
    fn rekursion(
        rest: &[&'static str],
        praefix: &mut Vec<&'static str>,
        aus: &mut Vec<Vec<&'static str>>,
    ) {
        if !praefix.is_empty() {
            aus.push(praefix.clone());
        }
        for (i, zauber) in rest.iter().enumerate() {
            let mut weiter = rest.to_vec();
            weiter.remove(i);
            praefix.push(zauber);
            rekursion(&weiter, praefix, aus);
            praefix.pop();
        }
    }

    let mut aus = Vec::new();
    rekursion(&SCHILDE, &mut Vec::new(), &mut aus);
    aus
}

/// Fährt eine einzelne Prioritätsliste und liest ab, was am Ende steht.
fn fahre(anordnung: &[&'static str], schild_talent_permille: u32) -> Messung {
    let boss = boss_by_id("test-resistant").expect("test-resistant fehlt im Datensatz");
    let text = anordnung
        .iter()
        .map(|z| format!("{z}\n"))
        .collect::<String>();
    let apl = parse(AplName("schild-obergrenze"), &text)
        .unwrap_or_else(|e| panic!("Liste {anordnung:?} ist ungueltig: {e:?}"));

    let mut fight = Fight::new(FightConfig {
        boss,
        duration_ms: Some(MESSZEITPUNKT_MS),
        talent_shield_amount_permille: schild_talent_permille,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(&apl);

    let schicht = fight.shield();
    let leben = fight.player_hp();
    Messung {
        schicht,
        leben,
        grenze: schicht + leben,
        anordnung: anordnung.to_vec(),
    }
}

/// Was ein Spieler höchstens aushält: das Maximum über **alle** Anordnungen.
///
/// `schild_talent_permille` hebt die Dicke jeder Schicht — der Weg, über den
/// die Verteidigung im Spiel tatsächlich wächst. 0 ist die Voreinstellung.
fn miss_obergrenze(schild_talent_permille: u32) -> Messung {
    alle_anordnungen()
        .iter()
        .map(|a| fahre(a, schild_talent_permille))
        .max_by_key(|m| m.grenze)
        .expect("alle_anordnungen liefert nie eine leere Menge")
}

/// Der Faktor, um den ein Schlag die gemessene Obergrenze übersteigen muss,
/// damit er als **unabwendbar** gilt (Issue #797, AC5).
///
/// # Warum ein Faktor und kein Zuschlag
///
/// **Er trägt auch dann, wenn sich die gesamte Verteidigung verdoppelt** — und
/// das ist mehr, als ein einzelner Balance-Durchgang je tut. Ein Zuschlag wie
/// „tausend darüber" wäre beim nächsten Durchgang wieder knapp.
///
/// **Und ein Zuschlag hat eine Einheit.** Er hängt an der heutigen Zahlenskala:
/// Stellt jemand das Spiel eines Tages auf andere Größenordnungen um — fünfzig-
/// statt fünftausend Lebenspunkte —, ist „tausend darüber" bedeutungslos. Ein
/// Faktor ist skaleninvariant, ein Zuschlag nicht.
pub const UNABWENDBAR_FAKTOR: u32 = 2;

/// Der Suchraum ist vollständig — und zwar gegen eine unabhängig gerechnete
/// Erwartung, nicht gegen sich selbst.
///
/// Die Erwartung ist die Summe der Variationen ohne Wiederholung,
/// `sum(k=1..n) n!/(n-k)!`, hier im Test noch einmal hingeschrieben. Eine
/// Zusicherung, die ihre Erwartung aus dem Prüfling bezöge, würde beim
/// Vertauschen brav mitsortieren und nie brechen.
#[test]
fn der_suchraum_ist_vollstaendig() {
    let n = SCHILDE.len();
    let erwartet: usize = (1..=n).map(|k| ((n - k + 1)..=n).product::<usize>()).sum();

    let listen = alle_anordnungen();
    assert_eq!(
        listen.len(),
        erwartet,
        "der Suchraum hat {} Listen statt {erwartet} — dann fährt die Messung \
         nicht alle Anordnungen ab und liefert eine Untergrenze",
        listen.len()
    );

    // Keine Liste doppelt: eine Wiederholung würde die Zahl oben erfüllen und
    // trotzdem eine Anordnung auslassen.
    let mut sortiert = listen.clone();
    sortiert.sort_unstable();
    sortiert.dedup();
    assert_eq!(
        sortiert.len(),
        listen.len(),
        "der Suchraum enthält Wiederholungen — dann fehlt an anderer Stelle eine \
         Anordnung, ohne dass die Anzahl es zeigt"
    );
}

/// Die Obergrenze steht und ist von beiden Teilen getragen.
///
/// # Was diese Zusicherungen fangen
///
/// Sie prüfen **Eigenschaften, keine Zahlen**: dass eine Schicht überhaupt
/// entsteht, dass der Spieler unverletzt bleibt (kein Schlag stört), und dass
/// beide Teile zusammen die Grenze ergeben. Eine feste Zahl stünde hier
/// falsch — sie ist der Gegenstand der Messung, nicht ihr Kriterium.
#[test]
fn die_obergrenze_wird_gemessen_und_besteht_aus_zwei_teilen() {
    let m = miss_obergrenze(0);

    assert!(
        m.schicht > 0,
        "keine Schicht aufgebaut — dann misst dieser Fall nichts. \
         Gemessen: Schicht {}, Leben {}",
        m.schicht,
        m.leben
    );
    assert!(
        m.leben > 0,
        "der Spieler ist gestorben, obwohl test-resistant keine Zeitleiste hat"
    );
    assert_eq!(
        m.grenze,
        m.schicht + m.leben,
        "die Grenze ist die Summe beider Teile"
    );

    println!(
        "OBERGRENZE GEMESSEN: Schicht {} + Leben {} = {}  (Anordnung {:?})",
        m.schicht, m.leben, m.grenze, m.anordnung
    );
    println!(
        "DARAUS FOLGT FUER AC5: ein Schlag ab {} Schaden ist unabwendbar \
         (Faktor {UNABWENDBAR_FAKTOR})",
        m.grenze * UNABWENDBAR_FAKTOR
    );
}

/// Die Wahl der Anordnung **macht einen Unterschied**.
///
/// # Warum das hierher gehört
///
/// Käme jede der 64 Anordnungen auf dieselbe Zahl, wäre die ganze Suche
/// Zierrat und die Obergrenze zufällig die einer beliebigen Liste. Diese
/// Zusicherung hält fest, dass zwischen bester und schlechtester Anordnung
/// überhaupt eine Spanne liegt — sie bricht, wenn jemand die Stapelmechanik so
/// ändert, dass die Reihenfolge nichts mehr bewirkt.
///
/// Sie nennt bewusst **keine** erwartete Spanne: deren Größe ist Balance-Sache.
#[test]
fn die_anordnung_macht_einen_unterschied() {
    let alle: Vec<Messung> = alle_anordnungen().iter().map(|a| fahre(a, 0)).collect();

    let beste = alle.iter().max_by_key(|m| m.grenze).expect("nicht leer");
    let schlechteste = alle.iter().min_by_key(|m| m.grenze).expect("nicht leer");

    assert!(
        beste.grenze > schlechteste.grenze,
        "alle {} Anordnungen ergeben dieselbe Grenze {} — dann bewirkt die \
         Reihenfolge nichts, und die Suche ueber sie ist wertlos",
        alle.len(),
        beste.grenze
    );

    println!(
        "SPANNE: beste {} {:?} gegen schlechteste {} {:?}",
        beste.grenze, beste.anordnung, schlechteste.grenze, schlechteste.anordnung
    );
}

/// Ein Talent, das Schilde stärkt, **hebt die gemessene Obergrenze**.
///
/// # Warum dieser Fall existiert
///
/// Die Fälle darüber prüfen Eigenschaften eines einzelnen Messdurchgangs. Sie
/// blieben grün, wenn [`miss_obergrenze`] die Schicht gar nicht mehr in ihre
/// Zahl aufnähme, solange irgendein positiver Wert herauskommt — dann wäre die
/// Obergrenze eine Zahl über Leben allein, und der Boss aus AC5 wäre gegen die
/// falsche Größe bemessen.
///
/// Dieser Fall vergleicht **zwei Messungen, die sich in genau einer Sache
/// unterscheiden**, und verlangt, dass sich die Zahl mitbewegt. Die Differenz
/// zweier Läufe misst, was ein einzelner Lauf nicht messen kann.
///
/// # Warum über das Talent und nicht über die Zauberdaten
///
/// `talent_shield_amount_permille` ist der Weg, über den die Verteidigung im
/// **laufenden Spiel** wächst — ein Spieler, der Talentpunkte hineinsteckt.
/// Damit schlägt dieser Fall auf genau die Änderung an, gegen die die
/// Obergrenze halten muss.
///
/// Der zweite Grund ist praktisch: Der Nachweis braucht **keine Datendatei**.
/// Er läuft ohne Mutation, ohne Rückbau und ohne Eingriff in `data/`.
///
/// # Warum das Kriterium keine feste Zahl nennt
///
/// Verlangt wird `mit > ohne`, nicht „mit == ohne + 1234". Die Größe des
/// Zuwachses ist Sache der Balance und darf sich ändern; dass er ankommt,
/// darf es nicht. Ein fester Wert würde bei jeder Talentanpassung brechen,
/// ohne dass etwas kaputt wäre — und niemand liest einen Wächter zweimal, der
/// aus dem falschen Grund rot wird.
#[test]
fn ein_schildtalent_hebt_die_gemessene_obergrenze() {
    /// Deutlich über 0, damit der Unterschied auch nach der Ganzzahl-Rundung
    /// im Kampf sichtbar bleibt: +50 % auf jede Schicht.
    const TALENT_PERMILLE: u32 = 500;

    let ohne = miss_obergrenze(0);
    let mit = miss_obergrenze(TALENT_PERMILLE);

    assert_eq!(
        ohne.leben, mit.leben,
        "das Leben darf sich zwischen den Messungen nicht unterscheiden — sonst \
         hat die zweite mehr geändert als das Talent, und die Differenz der \
         Obergrenzen belegt nicht, was sie belegen soll"
    );

    assert!(
        mit.schicht > ohne.schicht,
        "das Talent kommt in der Schicht nicht an: ohne {}, mit {}. Entweder \
         wirkt talent_shield_amount_permille nicht, oder der gemessene Kampf \
         nimmt es nicht auf.",
        ohne.schicht,
        mit.schicht
    );

    assert!(
        mit.grenze > ohne.grenze,
        "die Obergrenze bewegt sich nicht mit dem Talent: ohne {}, mit {}. Dann \
         misst diese Datei etwas anderes als die Verteidigung, die der Spieler \
         tatsächlich aufbauen kann — und die Schadenszahl des Bosses aus AC5 \
         wäre gegen die falsche Größe bemessen.",
        ohne.grenze,
        mit.grenze
    );

    println!(
        "TALENT WIRKT: Schicht {} -> {}, Obergrenze {} -> {} (Talent {TALENT_PERMILLE} Promille)",
        ohne.schicht, mit.schicht, ohne.grenze, mit.grenze
    );
}
