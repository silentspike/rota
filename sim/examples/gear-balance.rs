//! Der Balance-Bericht zur Ausruestung (Issue E6-5, AC4).
//!
//! ```text
//! cargo run --release --example gear-balance
//! ```
//!
//! Er ist fuer Menschen geschrieben, nicht fuer eine Auswertung: Jede Zahl
//! steht neben dem Satz, der sagt, was sie bedeutet — und neben dem, was sie
//! **nicht** bedeutet.

use rota_sim::gear::{POOL_ESSENCE, POOL_GEAR_EFFECTS};
use rota_sim::gear_balance::{
    compare_variants, spread_factor, stacked_factor, tipping_point, tipping_surcharge,
};

/// Wie viele Faelle der Varianten-Vergleich rechnet.
const CASES: u32 = 200_000;

/// Die Budgets, fuer die der Kipppunkt gezeigt wird. **Vorlaeufig.**
const BUDGETS: [f64; 5] = [0.15, 0.3, 0.6, 1.2, 2.4];

/// Wie viele Toepfe ein Affix-Aufbau bespielt: CATALYSIS, PURITY, YIELD.
const AFFIX_POOLS: u32 = 3;

fn linie() {
    println!("{}", "-".repeat(76));
}

fn main() {
    println!();
    linie();
    println!("BALANCE-BERICHT AUSRUESTUNG — Issue E6-5");
    println!("  Erzeugt aus dem Simulator. Zwei Fragen, zwei sehr verschiedene");
    println!("  Antwortguetegrade — der Unterschied steht bei jeder dabei.");
    linie();

    messung_2();
    modellrechnung();
    was_fehlt();
}

/// Messung 2 — sechs Faktoren oder vier Toepfe? **Eine echte Messung.**
fn messung_2() {
    let ergebnis = compare_variants(CASES, POOL_ESSENCE, POOL_GEAR_EFFECTS);

    println!();
    println!("MESSUNG 2 — Vereinheitlichung der Formel: Kosmetik oder nicht?");
    println!("  GEMESSEN. Kein Modell, keine Annahme.");
    println!();
    println!("  Verglichen wird die KLAMMERUNG:");
    println!("      (Grundschaden x f1) x f2      gegen      Grundschaden x (f1 x f2)");
    println!();
    println!("  {:<44} {:>12}", "Faelle gerechnet", ergebnis.cases);
    println!(
        "  {:<44} {:>12}   ({:.1} %)",
        "davon mit abweichendem ROHWERT",
        ergebnis.raw_differs,
        100.0 * f64::from(ergebnis.raw_differs) / f64::from(ergebnis.cases)
    );
    println!(
        "  {:<44} {:>12}",
        "davon mit abweichendem GERUNDETEM Schaden", ergebnis.damage_differs
    );
    println!();

    if ergebnis.is_measurable() {
        let einer_von = f64::from(ergebnis.cases) / f64::from(ergebnis.damage_differs);
        println!("  ERGEBNIS: Die Vereinheitlichung ist KEINE Kosmetik.");
        println!();
        println!("  Sie verschiebt den Schaden in etwa einem von {einer_von:.0} Faellen um");
        println!("  einen Punkt. Das klingt nach nichts und ist es nicht: Der Gleichlauf");
        println!("  von Spiel und Simulator beruht auf BITGLEICHHEIT (18.4). Ein Umbau");
        println!("  muesste auf beiden Seiten exakt gleich erfolgen — sonst laufen sie");
        println!("  auseinander, und zwar selten genug, dass es niemandem auffaellt.");
        println!();
        println!("  EMPFEHLUNG: bei den sieben benannten Faktoren bleiben. Nicht weil");
        println!("  der Unterschied gross ist, sondern weil er EXISTIERT.");
    } else {
        println!("  ERGEBNIS: Die Vereinheitlichung waere Kosmetik.");
        println!();
        println!("  ACHTUNG: Diese Aussage stand hier schon einmal — aus einem Vergleich,");
        println!("  der dieselbe Rechnung zweimal ausfuehrte. Wenn sie wieder erscheint,");
        println!("  pruefe zuerst, ob der Vergleich ueberhaupt zwei verschiedene Wege");
        println!("  rechnet (sim/src/gear_balance.rs, compare_variants).");
    }

    println!();
    println!("  WARUM ES BISHER NIEMANDEM AUFGEFALLEN IST: Der rohe Wert weicht in gut");
    println!("  einem Drittel der Faelle ab — die Rundung schluckt fast alles davon.");
    println!("  Sichtbar wird es nur, wenn das Ergebnis genau auf einer .5-Grenze liegt:");
    println!();
    println!("      2099 x 1,2 x 1,25");
    println!("        einzeln    3148,4999999999995  ->  3148");
    println!("        zusammen   3148,5              ->  3149");
    println!();
    println!("  Beide Wege rechnen richtig. Der Unterschied entsteht erst beim Runden,");
    println!("  und er geht nicht immer in dieselbe Richtung.");
}

/// Messung 1 — Satz gegen Affix. **Eine Modellrechnung, keine Messung.**
fn modellrechnung() {
    println!();
    linie();
    println!("MESSUNG 1 — bester Satz-Aufbau gegen besten Affix-Aufbau");
    println!("  MODELLRECHNUNG, KEINE MESSUNG AM KAMPF. Warum, steht unten.");
    println!();
    println!("  Beide Aufbauten bekommen dasselbe Budget. Der Affix-Aufbau verteilt es");
    println!("  auf {AFFIX_POOLS} Toepfe (CATALYSIS, PURITY, YIELD — je ein eigener Faktor,");
    println!("  13.1b); der Satz-Aufbau sammelt es in einem (Satz-Boni und Legendaere");
    println!("  liegen zusammen und sind dort additiv).");
    println!();
    println!(
        "  {:>8}  {:>12}  {:>12}  {:>14}  {:>10}",
        "Budget", "Affix", "Satz", "Kipppunkt", "Aufschlag"
    );
    for budget in BUDGETS {
        println!(
            "  {:>8.2}  {:>12.4}  {:>12.4}  {:>14.4}  {:>9.1} %",
            budget,
            spread_factor(budget, AFFIX_POOLS),
            stacked_factor(budget),
            tipping_point(budget, AFFIX_POOLS),
            100.0 * tipping_surcharge(budget, AFFIX_POOLS)
        );
    }
    println!();
    println!("  KIPPPUNKT ist die Antwort auf AC2: der Satz-Bonus, den ein gesammelter");
    println!("  Aufbau braeuchte, um mit dem verteilten gleichzuziehen. AUFSCHLAG sagt,");
    println!("  um wie viel mehr das ist.");
    println!();
    println!("  ERGEBNIS: Der verteilte Aufbau liegt bei JEDEM Budget vorn, und der");
    println!("  Abstand waechst mit dem Budget. Genau das will 13.1b — 'weil sie sich");
    println!("  multiplizieren, will man alle drei mischen statt einen zu stapeln'.");
    println!();
    println!("  FUER DIE HARTE BALANCE-ANFORDERUNG AUS 13.5 heisst das: Saetze und");
    println!("  Affixe sind NICHT von selbst gleichwertig. Damit sie es werden, muss");
    println!("  der Satz-Bonus um den Aufschlag ueber dem Affix-Budget liegen — bei");
    println!("  einem Budget von 0,60 also um gut ein Fuenftel.");
}

/// Was der Bericht **nicht** sagt — und warum.
fn was_fehlt() {
    println!();
    linie();
    println!("WAS DIESE ZAHLEN NICHT SIND");
    linie();
    println!();
    println!("  MESSUNG 1 IST EINE MODELLRECHNUNG. Sie folgt aus der Topf-Arithmetik,");
    println!("  nicht aus gerechneten Kaempfen. Der Grund ist keine Bequemlichkeit:");
    println!();
    println!("    Kein einziger Ausruestungs-Topf wird ausserhalb von gear.rs gelesen.");
    println!("    factors.gear_effects wird nur in balance.rs gesetzt — kuenstlich in");
    println!("    der Messung, nie aus echter Ausruestung.");
    println!("    CATALYSIS und PURITY sind nirgends verdrahtet.");
    println!("    YIELD ist wirkungslos, weil Krit ruht (STEP1-SPEC 2).");
    println!();
    println!("  Ein Kampf-Vergleich auf dieser Grundlage saehe aus wie 'Saetze sind");
    println!("  ueberlegen', waehrend in Wahrheit nur die Gegenseite nicht eingebaut");
    println!("  ist. Er waere schlimmer als keine Zahl, weil er wie eine Messung");
    println!("  aussaehe.");
    println!();
    println!("  ES GIBT AUCH KEINE FEHLERBALKEN. Der Lauf ist heute vollstaendig");
    println!("  deterministisch — kein Krit, keine Zufalls-Procs (21.1). Ueber 200");
    println!("  Durchgaenge: Schaden 4240,0 +/- 0,0. Ein Fehlerbalken von null sagt");
    println!("  nichts ueber die Sicherheit einer Aussage, sondern nur, dass nichts");
    println!("  wuerfelt.");
    println!();
    println!("  DIE ZAHLEN DER MODELLRECHNUNG SIND VORLAEUFIG. Welche Budgets ein");
    println!("  Aufbau tatsaechlich erreicht, haengt am Gegenstands-Katalog, und der");
    println!("  steht vor E10 aus (GAME-DESIGN 22). Was NICHT vorlaeufig ist: dass der");
    println!("  verteilte Aufbau vorn liegt und dass der Abstand mit dem Budget waechst.");
    println!("  Das folgt aus der Struktur, nicht aus den Zahlen.");
    println!();
    linie();
    println!();
}
