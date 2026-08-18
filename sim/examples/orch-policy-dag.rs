//! MESSWERKZEUG DES ORCHESTRATORS — nicht committen.
//!
//! Prueft die staerkste Behauptung des externen Urteils: dass eine Suchrunde
//! ihre Kandidaten als EINEN geteilten Entscheidungsgraphen auswerten koennte
//! statt als N getrennte Kaempfe.
//!
//! # Was hier gemessen wird
//!
//! `greedy_opts` baut in JEDER Runde die Menge
//!
//! ```text
//!   fuer jeden offenen Kandidaten  (bis 93)
//!     fuer jede Einfuegestelle     (0..=len)
//!       ein VOLLER Kampf
//! ```
//!
//! Bei einer Liste der Laenge 5 sind das 93 x 6 = 558 Vollkaempfe — die Zahl,
//! die das externe Urteil nennt. Alle starten im selben Zustand, mit demselben
//! Build, demselben Boss, demselben Startwert.
//!
//! Die Frage: Wie stark teilen sich ihre Cast-Folgen einen Anfang?
//!
//! # Warum die Cast-Folge das richtige Mass ist
//!
//! Der Kampf ist deterministisch. Zwei Rotationen mit identischer Cast-Folge
//! bis Entscheidung n haben nach Entscheidung n denselben Zustand — ein DAG
//! koennte dieses Praefix EINMAL rechnen. Umgekehrt koennen zwei Verlaeufe
//! mit verschiedenem Praefix spaeter wieder zusammenlaufen; das findet der
//! Praefixbaum NICHT. Der gemessene Faktor ist damit eine **Untergrenze**.
//!
//! # Kontrollzeilen
//!
//! Zwei Faelle mit vorher bekanntem Ergebnis laufen mit, weil sonst nicht
//! auffaellt, wenn der Messweg die Antwort erzeugt:
//!
//! ```text
//!   N identische Folgen   -> Faktor muss exakt N sein
//!   N Folgen, die sich ab Entscheidung 0 unterscheiden -> Faktor ~1
//! ```

use std::collections::BTreeMap;

use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{all_candidates, greedy, Candidate};
use rota_sim::{Fight, FightConfig};

const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-2-2", "boss-3-4"];
const BOSS: &str = "boss-1-1";
const DAUER_MS: u32 = 60_000;
const SEED: u32 = 5;

/// Ein Praefixbaum ueber Entscheidungsfolgen. Jeder Knoten ist ein Zustand,
/// den ein DAG genau einmal rechnen muesste.
#[derive(Default)]
struct Trie {
    kinder: BTreeMap<(u32, u8), Trie>,
}

impl Trie {
    fn einfuegen(&mut self, folge: &[(u32, u8)]) {
        let mut k = self;
        for &schritt in folge {
            k = k.kinder.entry(schritt).or_default();
        }
    }
    fn knoten(&self) -> usize {
        1 + self.kinder.values().map(Trie::knoten).sum::<usize>()
    }
    /// Wie viele Knoten liegen auf Tiefe `d`? Sagt, wo die Teilung aufhoert.
    fn breite(&self, d: usize, out: &mut Vec<usize>) {
        if out.len() <= d {
            out.resize(d + 1, 0);
        }
        out[d] += 1;
        for kind in self.kinder.values() {
            kind.breite(d + 1, out);
        }
    }
}

fn faktor(folgen: &[Vec<(u32, u8)>]) -> (usize, usize, f64) {
    let mut t = Trie::default();
    for f in folgen {
        t.einfuegen(f);
    }
    let summe: usize = folgen.iter().map(Vec::len).sum();
    // Wurzel zaehlt nicht als Transition.
    let knoten = t.knoten() - 1;
    let f = if knoten == 0 {
        0.0
    } else {
        summe as f64 / knoten as f64
    };
    (summe, knoten, f)
}

fn entscheidungen_b(boss_id: &str, apl: &Apl) -> Vec<(u32, u8)> {
    let boss = boss_by_id(boss_id).expect("Boss fehlt");
    let mut fight = Fight::new(FightConfig {
        seed: SEED,
        boss,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(apl);
    fight
        .log()
        .entries()
        .into_iter()
        .filter(|e| e.kind == LogKind::Cast)
        .map(|e| (e.time, e.spell))
        .collect()
}

fn entscheidungen(apl: &Apl) -> Vec<(u32, u8)> {
    entscheidungen_b(BOSS, apl)
}

fn liste(zeilen: &[Candidate]) -> Apl {
    Apl {
        name: "v".to_owned(),
        rules: zeilen
            .iter()
            .enumerate()
            .map(|(i, k)| Rule {
                line: i + 1,
                spell: k.spell,
                condition: k.condition.clone(),
            })
            .collect(),
    }
}

fn main() {
    // ---- Kontrollzeilen zuerst. Ohne sie ist keine Zahl darunter belastbar.
    let dummy: Vec<(u32, u8)> = (0..40).map(|i| (i * 1000, (i % 7) as u8)).collect();
    let gleich: Vec<Vec<(u32, u8)>> = (0..558).map(|_| dummy.clone()).collect();
    let (_, _, f_gleich) = faktor(&gleich);
    let verschieden: Vec<Vec<(u32, u8)>> = (0..558)
        .map(|n| (0..40).map(|i| (i * 1000, (n % 251) as u8)).collect())
        .collect();
    let (_, _, f_versch) = faktor(&verschieden);
    println!("== Kontrollzeilen ==");
    println!("  558 identische Folgen   Faktor {f_gleich:.1}   (erwartet 558,0)");
    println!("  558 ab Schritt 0 verschiedene Folgen  Faktor {f_versch:.2}   (erwartet nahe 1)");
    assert!(
        (f_gleich - 558.0).abs() < 0.01,
        "Kontrollzeile 1 gebrochen — der Messweg zaehlt falsch"
    );
    assert!(
        f_versch < 3.0,
        "Kontrollzeile 2 gebrochen — der Messweg teilt, wo nichts zu teilen ist"
    );
    println!();

    // ---- Die echte Runde nachstellen.
    let config = BalanceConfig {
        iterations: 1,
        base_seed: SEED,
        threads: 1,
        duration_ms: Some(DAUER_MS),
        stop_on_kill: false,
        factors: rota_sim::factors::NEUTRAL,
        boss: boss_by_id(BOSS).expect("Boss fehlt"),
        ..BalanceConfig::default()
    };
    let pool = all_candidates();
    println!("Kandidatenpool: {}", pool.len());

    // Eine realistische Basisliste: die ersten Runden von greedy.
    for basis_len in [0usize, 2, 5] {
        let basis: Vec<Candidate> = if basis_len == 0 {
            Vec::new()
        } else {
            greedy(&config, &pool, basis_len).chosen
        };
        // Genau das, was greedy_opts in DIESER Runde messen wuerde.
        let offen: Vec<&Candidate> = pool.iter().filter(|k| !basis.contains(k)).collect();
        let mut folgen = Vec::new();
        for k in &offen {
            for stelle in 0..=basis.len() {
                let mut v = basis.clone();
                v.insert(stelle, (*k).clone());
                folgen.push(entscheidungen(&liste(&v)));
            }
        }
        let (summe, knoten, f) = faktor(&folgen);
        let mut breite = Vec::new();
        let mut t = Trie::default();
        for x in &folgen {
            t.einfuegen(x);
        }
        t.breite(0, &mut breite);

        println!();
        println!("== Runde bei Listenlaenge {basis_len} ==");
        println!("  Kandidaten in dieser Runde        {}", folgen.len());
        println!("  Entscheidungen zusammen           {summe}");
        println!("  einzigartige Zustaende (Trie)     {knoten}");
        println!("  TEILUNGSFAKTOR                    {f:.2}x");
        let leer = folgen.iter().filter(|f| f.is_empty()).count();
        println!("  davon Folgen ohne einen Cast      {leer}");
        print!("  Trie-Breite je Tiefe 1..12        ");
        for d in 1..=12.min(breite.len().saturating_sub(1)) {
            print!("{} ", breite[d]);
        }
        println!();

        // ---- Policy-Aequivalenz: wie viele Kandidaten sind VOLLSTAENDIG
        // verhaltensgleich? Das ist die Verallgemeinerung der
        // Same-Spell-Subsumption, die das externe Urteil fordert.
        let mut klassen: BTreeMap<Vec<(u32, u8)>, usize> = BTreeMap::new();
        for f in &folgen {
            *klassen.entry(f.clone()).or_default() += 1;
        }
        let groesste = klassen.values().copied().max().unwrap_or(0);
        let mehrfach: usize = klassen.values().filter(|&&n| n > 1).sum();
        println!(
            "  verhaltensgleiche Klassen         {} von {} Kandidaten",
            klassen.len(),
            folgen.len()
        );
        println!(
            "  AEQUIVALENZFAKTOR                 {:.2}x   (groesste Klasse {groesste}, {mehrfach} Kandidaten in Mehrfachklassen)",
            folgen.len() as f64 / klassen.len() as f64
        );

        // ---- Ist die groesste Klasse die UNVERAENDERTE Basisrotation?
        // Wenn ja, ist der Grossteil der Ersparnis kein DAG, sondern die
        // Regel "die eingefuegte Zeile greift nie" — und die ist mit einer
        // Bedingung zu haben, nicht mit einem Graphen.
        let basisfolge = entscheidungen(&liste(&basis));
        let wie_basis = klassen.get(&basisfolge).copied().unwrap_or(0);
        println!("  davon Folge = unveraenderte Basis  {wie_basis}");

        // ---- Was bleibt dem DAG NACH der Deduplizierung?
        // Das ist die Zahl, die ueber die Reihenfolge der beiden Massnahmen
        // entscheidet. Sind die Gewinne nicht additiv, ist der teure Umbau
        // nach dem billigen fast wertlos.
        let einzigartig: Vec<Vec<(u32, u8)>> = klassen.keys().cloned().collect();
        let (s2, k2, f2) = faktor(&einzigartig);
        println!("  --- nach Deduplizierung ---");
        println!("  einzigartige Folgen               {}", einzigartig.len());
        println!("  Entscheidungen zusammen           {s2}");
        println!("  Zustaende im Trie                 {k2}");
        println!("  REST-TEILUNGSFAKTOR fuer den DAG  {f2:.2}x");

        // ---- GEGENPROBE mit der guenstigsten Lesart des Vorschlags.
        //
        // Mein Trie-Schluessel ist (Zeit, Zauber). Wer nur die AKTIONSFOLGE
        // betrachtet und die Zeit weglaesst, misst mehr Teilung — das waere
        // die optimistischste Auslegung. Sie ist als Zustandsmass FALSCH
        // (verschiedene Zeit heisst verschiedener Zustand), aber sie ist die
        // Obergrenze dessen, was der Vorschlag ueberhaupt versprechen kann.
        let nur_zauber: Vec<Vec<(u32, u8)>> = folgen
            .iter()
            .map(|f| f.iter().map(|&(_, sp)| (0u32, sp)).collect())
            .collect();
        let (_, _, f_nz) = faktor(&nur_zauber);
        let mut kl_nz: BTreeMap<Vec<(u32, u8)>, usize> = BTreeMap::new();
        for f in &nur_zauber {
            *kl_nz.entry(f.clone()).or_default() += 1;
        }
        let einz_nz: Vec<Vec<(u32, u8)>> = kl_nz.keys().cloned().collect();
        let (_, _, f_nz_rest) = faktor(&einz_nz);
        println!("  --- Gegenprobe OHNE Zeitstempel (Obergrenze) ---");
        println!("  Teilungsfaktor                    {f_nz:.2}x");
        println!("  einzigartige Aktionsfolgen        {}", einz_nz.len());
        println!("  Rest fuer den DAG danach          {f_nz_rest:.2}x");
    }

    // ---- Regel-Interaktion: koennen zwei Zeilen ueberhaupt je gleichzeitig
    // zur Wahl stehen? Wo nicht, ist ihre Reihenfolge bedeutungslos
    // (Partial-Order Reduction).
    //
    // Konservativ gemessen: nur Paare, bei denen sich die Bedingungen
    // STRUKTURELL ausschliessen, zaehlen als kommutierend. Alles Unklare
    // bleibt unveraendert — eine falsch als kommutierend erklaerte Paarung
    // wuerde still falsche Ergebnisse liefern.
    println!();
    println!("== Regel-Interaktion ueber alle {} Zeilen ==", pool.len());
    let mut gleicher_zauber = 0usize;
    let mut subsumiert = 0usize;
    for i in 0..pool.len() {
        for j in (i + 1)..pool.len() {
            if pool[i].spell == pool[j].spell {
                gleicher_zauber += 1;
                let a_frei = matches!(pool[i].origin, rota_sim::rotation_search::Origin::Always);
                let b_frei = matches!(pool[j].origin, rota_sim::rotation_search::Origin::Always);
                if a_frei != b_frei {
                    subsumiert += 1;
                }
            }
        }
    }
    let paare = pool.len() * (pool.len() - 1) / 2;
    println!("  Paare gesamt                      {paare}");
    println!("  Paare mit demselben Zauber        {gleicher_zauber}");
    println!(
        "  davon eine Anordnung tot          {subsumiert}   ({:.2} % aller Paare)",
        100.0 * subsumiert as f64 / paare as f64
    );
    println!();
    // ---- NACHGETRAGEN: derselbe Test an vier verschiedenen Bossen.
    // Der urspruengliche Befund stammte von EINEM Boss — das war die
    // groesste benannte Schwaeche der Messung.
    println!();
    println!("== Teilung an vier Bossen, Basislaenge 5 ==");
    println!(
        "{:<12} {:>10} {:>10} {:>12} {:>12}",
        "Boss", "Kandidat.", "Teilung", "Aequivalenz", "Rest-DAG"
    );
    for b in BOSSE {
        let Some(bo) = boss_by_id(b) else { continue };
        let c = BalanceConfig { boss: bo, ..config };
        let basis = greedy(&c, &pool, 5).chosen;
        let offen: Vec<&Candidate> = pool.iter().filter(|k| !basis.contains(k)).collect();
        let mut folgen = Vec::new();
        for k in &offen {
            for stelle in 0..=basis.len() {
                let mut v = basis.clone();
                v.insert(stelle, (*k).clone());
                folgen.push(entscheidungen_b(b, &liste(&v)));
            }
        }
        let (_, _, f) = faktor(&folgen);
        let mut kl: BTreeMap<Vec<(u32, u8)>, usize> = BTreeMap::new();
        for x in &folgen {
            *kl.entry(x.clone()).or_default() += 1;
        }
        let einz: Vec<Vec<(u32, u8)>> = kl.keys().cloned().collect();
        let (_, _, f2) = faktor(&einz);
        println!(
            "{:<12} {:>10} {:>9.2}x {:>11.2}x {:>11.2}x",
            b,
            folgen.len(),
            f,
            folgen.len() as f64 / kl.len() as f64,
            f2
        );
    }
    println!();

    println!("  ACHTUNG: das ist die BEKANNTE Subsumption, strukturell gezaehlt.");
    println!("  Ob zwei Zeilen VERSCHIEDENER Zauber je gleichzeitig gueltig und");
    println!("  bereit sind, sagt diese Zaehlung NICHT — dafuer braucht es eine");
    println!("  Erreichbarkeitsanalyse, nicht eine Betrachtung der Bedingungen.");
}
