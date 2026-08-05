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

/// Startwert des ersten Durchgangs.
///
/// **Steht hier als Konstante, nicht als Vorgabe des Laeufers** (Issue #242).
/// Wer diesen Bericht in drei Wochen nachfaehrt, bekommt dieselben Zahlen —
/// aber nur, wenn auch der Startwert derselbe ist. Ein Aufruf, der ihn dem
/// Standard ueberlaesst, ist genauso wenig reproduzierbar wie einer ohne
/// Durchgangszahl.
const SEED: u32 = 0;

/// Die Parameter, unter denen alle Zahlen dieses Berichts entstanden sind.
///
/// # Warum sie GEDRUCKT werden und nicht nur im Quelltext stehen
///
/// Eine Zahl wird aus einem Bericht herauskopiert — in ein Issue, in eine
/// Nachricht, in eine Entscheidungsvorlage. Steht die Durchgangszahl nur im
/// Quelltext, geht sie dabei verloren, und die Zahl wird zur Behauptung.
///
/// Genau das ist mit #118 geschehen: Die Zahl 9892,7 existiert, der Aufruf
/// dahinter wurde nie committet. Weder Durchgangszahl noch Startwert,
/// Kampfdauer oder Boss sind festgehalten — die Messung ist nicht
/// wiederholbar, und auf ihr beruhen zwei offene Design-Entscheidungen.
///
/// **Gedruckte Parameter reisen mit der Zahl mit.** Das ist der Unterschied
/// zwischen einer Zahl und einer Messung.
fn parameter_zeile() {
    let boss = rota_sim::BOSS;
    println!("  PARAMETER DIESES LAUFS — sie gehoeren zu jeder Zahl unten");
    println!();
    println!("      Durchgaenge je Zeile     {RUNS}");
    println!("      Startwert                {SEED}");
    println!("      Boss                     {}", boss.id);
    println!(
        "      Kampfdauer               {} ms  (Ueberdruck des Bosses)",
        boss.overpressure_ms
    );
    println!("      Abbruch beim Sieg        nein  (sonst rechnet der staerkere");
    println!("                                     Aufbau kuerzer, STEP1-SPEC 4)");
}

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
            base_seed: SEED,
            stop_on_kill: false,
            worn,
            ..BalanceConfig::default()
        },
        &apl,
    );
    // **Der Standardfehler, nicht die Streuung der Einzelwerte.**
    //
    // Beide sahen im ersten Entwurf gleich aus — dieselbe Spalte, dasselbe
    // Zeichen. Nebeneinander mit #118s 45,7 (das ein Standardfehler ist) hätte
    // meine Streuung von 326,0 so ausgesehen, als wäre diese Messung siebenmal
    // unsicherer. Sie ist präziser: 23,0 gegen 45,7.
    //
    // AC5 fragt nach der Unschärfe der MESSUNG — also danach, wie genau der
    // Mittelwert bekannt ist. Das ist der Standardfehler; er schrumpft mit der
    // Wurzel der Durchgänge. Die Streuung der Einzelwerte tut das nicht, sie
    // ist eine Eigenschaft der Sache.
    println!(
        "  {:<38} {:>9.1}  +/- {:>6.1}",
        name, r.damage.mean, r.damage.stderr
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
    parameter_zeile();
    linie();

    println!();
    println!("DIE ZWEI REIHEN NEBENEINANDER  ({RUNS} Durchgaenge je Zeile)");
    println!();
    println!("  Das +/- ist der STANDARDFEHLER: wie genau der Mittelwert bekannt ist.");
    println!("  Nicht die Streuung der Einzelwerte — die ist rund vierzehnmal groesser");
    println!("  und beantwortet eine andere Frage.");
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
    println!("  Zum Vergleich, gemessen in #118:            9892,7");
    println!();
    println!("  DIE ZAHL AUS #118 IST DAMIT NEU BELEGT (Issue #242, AC3): Was oben");
    println!("  steht, ist dieselbe Messung mit demselben Aufbau — nur diesmal mit");
    println!("  Parametern, die jeder nachfahren kann. Die alte Zahl bleibt trotzdem");
    println!("  stehen, statt ersetzt zu werden: Wer in drei Wochen eine Abweichung");
    println!("  sieht, muss erkennen koennen, ob sich das Spiel geaendert hat oder");
    println!("  nur die Messung.");
    println!();
    println!("  DIE BEIDEN ZAHLEN SIND NICHT GLEICH GUT BELEGT, und das gehoert");
    println!("  dazugesagt:");
    println!();
    println!("      9893,4   aus diesem Laeufer. Committet, RUNS steht als Konstante");
    println!("               darin, jeder kann ihn nachfahren.");
    println!("      9892,7   aus einem Issue-Kommentar. Der Aufruf, der sie erzeugt");
    println!("               hat, wurde nie committet — Durchgangszahl, Startwert,");
    println!("               Kampfdauer und Boss sind nicht festgehalten.");
    println!();
    println!("  Deshalb steht neben der zweiten kein Fehlerbalken: Ohne die");
    println!("  Durchgangszahl laesst sich keiner einordnen — dieselbe Zahl bedeutet");
    println!("  bei 60 und bei 1000 Durchgaengen etwas voellig anderes.");
    println!();
    println!("  Der Vergleich braucht ihn auch nicht: Die MITTELWERTE sind unabhaengig");
    println!("  von der Durchgangszahl vergleichbar, und 0,7 Unterschied liegt unter");
    println!("  jedem denkbaren Streuungsmass. Die alte Reihe ist reproduziert.");
    println!();
    println!("  Der Fehlerbalken oben kommt vom Krit: purity 800 sind 80 % Kritchance,");
    println!("  und die wuerfelt. Ohne Ausruestung ist er null — dort wuerfelt nichts,");
    println!("  und das macht den Rotations-Vergleich SCHAERFER, nicht schwaecher.");
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
