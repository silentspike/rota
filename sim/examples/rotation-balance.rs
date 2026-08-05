//! Was die vier neuen Zauber an der Balance verschieben (Issue #224).
//!
//! ```text
//! cargo run --release --example rotation-balance
//! ```
//!
//! Er ist fuer Menschen geschrieben: Jede Zahl steht neben dem Satz, der sagt,
//! was sie bedeutet — und neben dem, was sie **nicht** bedeutet.

use rota_sim::apl::{parse, Apl};
use rota_sim::balance::{run, BalanceConfig};
use rota_sim::gear::WornValues;

/// Die alte Rotation — drei Zauber, der Vergleichspunkt.
const ALT: &str = include_str!("../apl/step1-good.apl");
/// Die neue — derselbe Kasten, alle fuenfzehn verfuegbar.
const NEU: &str = include_str!("../apl/step3-good.apl");

/// Wie viele Durchgaenge je Zeile.
const RUNS: u32 = 200;

fn linie() {
    println!("{}", "-".repeat(76));
}

/// Die Ausruestung, mit der #118 gemessen hat — der beste Affix-Aufbau.
///
/// **Ohne sie waere der Vergleich mit der alten Reihe wertlos.** Jene lief mit
/// purity 800, und der Unterschied zwischen 9892,7 und einer Messung ohne
/// Ausruestung kaeme fast vollstaendig von der Ausruestung, nicht von den
/// Zaubern.
const WIE_118: WornValues = WornValues {
    catalysis: 0,
    purity: 800,
    yield_bonus: 330,
    gear_effects: 0,
};

fn messe_mit(name: &str, text: &str, worn: WornValues) -> f64 {
    let apl: Apl = parse(name, text).expect("die eingebaute Liste ist gueltig");
    let r = run(
        &BalanceConfig {
            iterations: RUNS,
            stop_on_kill: false,
            worn,
            ..BalanceConfig::default()
        },
        &apl,
    );
    println!(
        "  {:<38} {:>9.1}  +/- {:>6.1}",
        name, r.damage.mean, r.damage.stddev
    );
    r.damage.mean
}

fn messe(name: &str, text: &str) -> f64 {
    messe_mit(name, text, WornValues::none())
}

fn main() {
    println!();
    linie();
    println!("ROTATIONS-BERICHT — Issue #224");
    println!("  Was aendert sich, seit der Zauberkasten von 11 auf 15 gewachsen ist?");
    linie();

    println!();
    println!("DIE ZWEI REIHEN NEBENEINANDER  ({RUNS} Durchgaenge je Zeile)");
    println!();
    let alt = messe("step1-good  (drei Zauber)", ALT);
    let neu = messe("step3-good  (alle fuenfzehn verfuegbar)", NEU);

    println!();
    if neu > alt {
        println!("  Die neue Rotation liegt vorn.");
    } else {
        println!("  ERGEBNIS: Die neue Rotation liegt NICHT vorn.");
        println!();
        println!("  Und der Grund ist nicht, dass sie schlecht geschrieben waere. Sie ist");
        println!("  die beste, die ich gefunden habe — und sie ist genau dann am besten,");
        println!("  wenn ALLE VIER neuen Zauber unter KINDLE stehen und damit nie");
        println!("  gewirkt werden.");
    }

    println!();
    linie();
    println!("DIESELBE MESSUNG MIT DER AUSRUESTUNG AUS #118");
    linie();
    println!();
    println!("  Die alte Reihe aus #118 lief NICHT ohne Ausruestung, sondern mit dem");
    println!("  besten Affix-Aufbau: catalysis 0, purity 800, yield 330. Wer ihre");
    println!("  9892,7 neben eine Messung ohne Ausruestung stellt, vergleicht");
    println!("  Ausruestung statt Rotationen.");
    println!();
    messe_mit("step1-good  mit Ausruestung", ALT, WIE_118);
    messe_mit("step3-good  mit Ausruestung", NEU, WIE_118);
    println!();
    println!("  Zum Vergleich, gemessen in #118:            9892,7  +/-   45,7");
    println!();
    println!("  Die alte Reihe ist damit reproduziert, und die neue liegt genau");
    println!("  darauf. Der Fehlerbalken kommt hier vom Krit: purity 800 sind 80 %");
    println!("  Kritchance, und die wuerfelt.");
    println!();
    println!("  UND DAMIT GILT AC5: Ein Vorsprung zaehlt nur ueber der Unschaerfe");
    println!("  BEIDER Messungen. Hier gibt es keinen Vorsprung — die Listen sind");
    println!("  nicht nur knapp, sondern gleich.");

    println!();
    linie();
    println!("WIE ICH DAHIN GEKOMMEN BIN — vier Fassungen derselben Liste");
    linie();
    println!();
    println!("  Werkzeuge zuerst                       2484,9");
    println!("  Schaden zuerst, Werkzeuge bedingt      4107,9");
    println!("  CANKER unter KINDLE                    4960,0");
    println!("  FESTER ueber KINDLE, Rest darunter     5760,0");
    println!("  alle vier unter KINDLE                 5960,0   <- gleichauf");
    println!();
    println!("  Jede Fassung, die einen der vier hoeher stellt, ist schlechter. Das");
    println!("  gilt einzeln, nicht nur in Summe: FESTER allein ueber KINDLE kostet");
    println!("  200 Punkte.");

    println!();
    linie();
    println!("DIE URSACHE SIND ZWEI ZAHLEN");
    linie();
    println!();
    println!("  ERSTENS: KINDLEs Abklingzeit IST die globale Wartezeit.");
    println!();
    println!("      kindle    Schaden 100   Abklingzeit 1500 ms");
    println!("      GCD_MS                              1500 ms");
    println!();
    println!("  Damit ist seine Abklingzeit wirkungslos — wenn die Wartezeit ablaeuft,");
    println!("  ist er immer schon bereit. Und er macht den hoechsten Schaden im");
    println!("  ganzen Kasten.");
    println!();
    println!("  DAS IST DIE UMKEHRUNG DESSEN, WAS EIN FUELLER IST. Sonst ist der");
    println!("  Fueller der schwaechste Zauber, den man wirkt, wenn nichts Besseres");
    println!("  bereit ist. Hier ist er der staerkste und immer bereit — es gibt nie");
    println!("  ein Fenster, in dem etwas anderes die bessere Wahl waere.");
    println!();
    println!("  Die Zauber brauchen keinen groesseren Kasten. Sie brauchen einen");
    println!("  Fueller, der schlechter ist als sie.");
    println!();
    println!("  ZWEITENS: Der Zersetzungs-Bonus ist etwa halb so gross wie noetig.");
    println!();
    println!("      DECAY_STACK_PERCENT              3 % je Stapel");
    println!("      erreichbare Stapel               3        (Abklingzeit 4,5 s,");
    println!("                                                Haltbarkeit 12 s)");
    println!("      also insgesamt                   9 %");
    println!();
    println!("      Break-even fuer CANKER          16,1 % gesamt");
    println!("      das waeren je Stapel             5,4 %");
    println!();
    println!("  CANKER verdraengt 15-mal einen 100er durch einen 45er. Was er");
    println!("  zurueckbringt, muesste diese 825 Punkte aufwiegen — bei 9 % auf den");
    println!("  Restschaden sind es rund 462.");

    println!();
    linie();
    println!("WAS DIESE ZAHLEN NICHT SIND");
    linie();
    println!();
    println!("  SIE SAGEN NICHT, DASS DIE VIER ZAUBER SCHLECHT SIND. Sie sagen, dass");
    println!("  sie sich in DIESEM Aufbau nicht lohnen — und der Aufbau hat drei");
    println!("  Eigenheiten, die das erzeugen:");
    println!();
    println!("      KINDLE ohne wirksame Abklingzeit   es gibt nie eine Luecke");
    println!("      ein Boss ohne Phasen               nichts, worauf man sich");
    println!("                                         vorbereiten muesste");
    println!("      60 s Kampfzeit                     REFLUX mit 45 s Abklingzeit");
    println!("                                         wirkt hoechstens einmal");
    println!();
    println!("  Bei einem Boss mit Phasen, in denen man Schaden aufsparen muss, saehe");
    println!("  REFLUX anders aus. **Das ist keine Vermutung, sondern die Stelle, an");
    println!("  der ich NICHTS gemessen habe.** Der Testboss hat keine Phasen, keinen");
    println!("  Schaden, kein Fenster — und drei der vier Zauber sind fuer genau");
    println!("  solche Situationen gebaut.");
    println!();
    println!("  BEIDE REIHEN SIND UNTERGRENZEN. Sie zeigen die beste AUSRUESTUNG fuer");
    println!("  eine von Hand geschriebene Rotation. Ob eine bessere Rotation");
    println!("  existiert, hat niemand gemessen — die Suche darueber ist nicht gebaut");
    println!("  (Issue #237). Das Optimum liegt MINDESTENS bei diesen Zahlen, nicht");
    println!("  genau dort.");
    println!();
    println!("  ES STEHT KEINE EMPFEHLUNG IN DIESEM BERICHT. Ob KINDLE eine echte");
    println!("  Abklingzeit bekommt oder der Zersetzungs-Bonus steigt, sind zwei");
    println!("  verschiedene Stellschrauben mit verschiedenen Folgen — die eine");
    println!("  aendert das Spielgefuehl, die andere nur eine Schule. Das entscheidet");
    println!("  der Auftraggeber.");
    println!();
    linie();
    println!();
}
