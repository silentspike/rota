//! Suchgüte, erweitert um **Beam**, **LNS** und **QD** (Fortsetzung von `suchguete`).
//!
//! # Warum es diese Datei gibt
//!
//! `suchguete` stellt drei Verfahren gegen die erschöpfende Suche: `greedy`
//! ohne Paare, `greedy` mit Paaren und den Bezugspunkt. Damit ist die Frage
//! „trifft die Suche das Optimum" beantwortet — aber nicht die Anschlussfrage:
//! **würde ein anderes Verfahren besser treffen, und was kostet es?**
//!
//! Diese Datei misst dieselben 60 Fälle mit denselben Zahlen und ergänzt drei
//! Verfahren:
//!
//! | Verfahren | Idee |
//! | --- | --- |
//! | **Beam(k)** | statt nur der besten Teilliste die besten `k` weiterverfolgen |
//! | **LNS** | aus einer gefundenen Liste einen Teil herausnehmen und neu füllen |
//! | **QD** | ein Archiv über Nischen füllen statt nur das Maximum zu suchen |
//!
//! # Alles deterministisch, kein Zufallszahlengeber
//!
//! Die Fälle entstehen aus ihrem Index (wie in `suchguete`), und auch LNS zieht
//! nicht zufällig: Welche Zeilen es herausnimmt, folgt aus Rundennummer und
//! Listenlänge. Zwei Läufe liefern dieselbe Ausgabe.
//!
//! # Die Achsen des QD-Archivs sind MEINE Wahl, keine Design-Entscheidung
//!
//! MAP-Elites braucht Nischen, und welche das sind, ist eine offene Frage an
//! den Auftraggeber (Dokument `SIM-MESSUNGEN.md`, Entscheidung 2). Damit hier
//! überhaupt etwas messbar ist, nehme ich zwei Achsen, die sich aus den Daten
//! ergeben und keine Design-Aussage treffen:
//!
//! * **Länge** der gefundenen Liste
//! * **Zahl der bedingten Zeilen** darin
//!
//! Wer andere Achsen setzt, bekommt andere Zahlen. Die Aussage „QD findet das
//! Maximum seltener als Beam, deckt dafür mehr Nischen ab" bleibt davon
//! unberührt — sie hängt am Verfahren, nicht an der Achsenwahl.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{
    all_candidates, exhaustive, greedy_opts, measure, Candidate, Origin,
};

/// Dieselben vier Bosse wie in `suchguete`.
const BOSSE: [&str; 4] = ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"];
/// Dieselbe Fallzahl.
const FAELLE: usize = 60;
/// Derselbe Startwert.
const SEED: u32 = 0;
/// Dieselbe Durchgangszahl — bei `purity = 0` ist jeder Durchgang bitgleich.
const RUNS: u32 = 1;
/// Breiten, mit denen Beam gemessen wird.
const BEAM_BREITEN: [usize; 3] = [1, 3, 10];
/// Wie viele Runden LNS fährt.
const LNS_RUNDEN: usize = 12;
/// Wie viele Zeilen LNS je Runde herausnimmt.
const LNS_ZERSTOERUNG: usize = 2;

struct Fall {
    nummer: usize,
    boss: &'static str,
    pool: Vec<Candidate>,
    max_len: usize,
}

fn config(boss_id: &str) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("Boss {boss_id} unbekannt"));
    BalanceConfig {
        iterations: RUNS,
        base_seed: SEED,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

/// Die Fälle, Zeichen für Zeichen wie in `suchguete` — sonst wären die Zahlen
/// nicht vergleichbar.
fn faelle() -> Vec<Fall> {
    let voll = all_candidates();
    (0..FAELLE)
        .map(|i| {
            let anzahl = 6 + (i * 3) % 4;
            let max_len = 3 + (i / BOSSE.len()) % 2;
            let versatz = (i * 17) % voll.len();
            let schritt = 1 + (i * 5) % 7;
            let mut pool: Vec<Candidate> = Vec::new();
            let mut k = versatz;
            while pool.len() < anzahl {
                let kandidat = voll[k % voll.len()].clone();
                if !pool.contains(&kandidat) {
                    pool.push(kandidat);
                }
                k += schritt;
                if k > versatz + voll.len() * schritt {
                    k = versatz + pool.len() + 1;
                }
            }
            Fall { nummer: i, boss: BOSSE[i % BOSSE.len()], pool, max_len }
        })
        .collect()
}

/// Beam Search mit Breite `k`.
///
/// Der Unterschied zu `greedy` ist eine Zeile: Statt nach jeder Position nur
/// die beste Teilliste zu behalten, behält Beam die besten `k`. Damit überlebt
/// eine Zeile, die allein schwach ist, aber ein Paar trägt — genau die
/// Schwäche, an der `greedy` scheitert.
fn beam(config: &BalanceConfig, pool: &[Candidate], max_len: usize, k: usize) -> (f64, usize) {
    let mut messungen = 0usize;
    // Jede Front ist eine Liste (Teilrotation, Wert).
    let mut front: Vec<(Vec<Candidate>, f64)> = vec![(Vec::new(), f64::NEG_INFINITY)];
    let mut bester = f64::NEG_INFINITY;

    for _ in 0..max_len {
        let mut naechste: Vec<(Vec<Candidate>, f64)> = Vec::new();
        for (liste, _) in &front {
            for kandidat in pool {
                if liste.contains(kandidat) {
                    continue;
                }
                let mut neu = liste.clone();
                neu.push(kandidat.clone());
                let wert = measure(config, "beam", &neu).mean;
                messungen += 1;
                if wert > bester {
                    bester = wert;
                }
                naechste.push((neu, wert));
            }
        }
        if naechste.is_empty() {
            break;
        }
        // Absteigend sortieren und die besten k behalten. Bei Gleichstand
        // entscheidet die Reihenfolge im Pool — deterministisch.
        naechste.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("kein NaN"));
        naechste.truncate(k);
        front = naechste;
    }
    (bester, messungen)
}

/// Large Neighbourhood Search: von einer gefundenen Liste aus wiederholt einen
/// Teil herausnehmen und mit `greedy` neu füllen.
///
/// **Welche Zeilen herausfliegen, folgt aus der Rundennummer** — kein
/// Zufallszahlengeber, damit zwei Läufe dieselbe Ausgabe liefern.
fn lns(config: &BalanceConfig, pool: &[Candidate], max_len: usize) -> (f64, usize) {
    let start = greedy_opts(config, pool, max_len, &[], true);
    let mut messungen = start.measured;
    let mut beste_liste = start.chosen.clone();
    let mut bester = start.best.mean;

    for runde in 0..LNS_RUNDEN {
        if beste_liste.len() <= LNS_ZERSTOERUNG {
            break;
        }
        // Deterministisch: ab Position (runde mod len) so viele Zeilen streichen.
        let ab = runde % beste_liste.len();
        let mut rest: Vec<Candidate> = Vec::new();
        for (i, c) in beste_liste.iter().enumerate() {
            let entfernt = (i >= ab && i < ab + LNS_ZERSTOERUNG)
                || (ab + LNS_ZERSTOERUNG > beste_liste.len()
                    && i < (ab + LNS_ZERSTOERUNG) % beste_liste.len());
            if !entfernt {
                rest.push(c.clone());
            }
        }
        let neu = greedy_opts(config, pool, max_len, &rest, true);
        messungen += neu.measured;
        if neu.best.mean > bester {
            bester = neu.best.mean;
            beste_liste = neu.chosen.clone();
        }
    }
    (bester, messungen)
}

/// Quality-Diversity über ein Archiv.
///
/// Statt nur das Maximum zu suchen, wird je Nische die beste gefundene Liste
/// behalten. Die Nische ist hier `(Länge, Zahl der bedingten Zeilen)` — siehe
/// den Vermerk im Dateikopf: **das ist meine Achsenwahl, keine
/// Design-Entscheidung.**
///
/// Als Emitter dient `greedy` von jedem Archiv-Eintrag aus. Damit hat das
/// Verfahren dieselbe Bauart wie MAP-Elites, ohne einen Zufallsgenerator zu
/// brauchen.
fn qd(config: &BalanceConfig, pool: &[Candidate], max_len: usize) -> (f64, usize, usize) {
    use std::collections::BTreeMap;
    let mut archiv: BTreeMap<(usize, usize), (Vec<Candidate>, f64)> = BTreeMap::new();
    let mut messungen = 0usize;

    let nische = |liste: &[Candidate]| {
        let bedingt = liste.iter().filter(|c| !matches!(c.origin, Origin::Always)).count();
        (liste.len(), bedingt)
    };

    // Aussaat: jede einzelne Zeile ist ein Archiv-Kandidat.
    for kandidat in pool {
        let liste = vec![kandidat.clone()];
        let wert = measure(config, "qd", &liste).mean;
        messungen += 1;
        let schluessel = nische(&liste);
        let besser = archiv.get(&schluessel).is_none_or(|(_, w)| wert > *w);
        if besser {
            archiv.insert(schluessel, (liste, wert));
        }
    }

    // Emitter: von jedem Archiv-Eintrag aus einen Schritt weiterbauen, das
    // Ergebnis in seine Nische einsortieren. Feste Rundenzahl statt Abbruch
    // nach Stagnation — sonst haengt die Messung an einer Schwelle.
    for _ in 0..max_len {
        let saat: Vec<Vec<Candidate>> = archiv.values().map(|(l, _)| l.clone()).collect();
        for liste in saat {
            if liste.len() >= max_len {
                continue;
            }
            for kandidat in pool {
                if liste.contains(kandidat) {
                    continue;
                }
                let mut neu = liste.clone();
                neu.push(kandidat.clone());
                let wert = measure(config, "qd", &neu).mean;
                messungen += 1;
                let schluessel = nische(&neu);
                let besser = archiv.get(&schluessel).is_none_or(|(_, w)| wert > *w);
                if besser {
                    archiv.insert(schluessel, (neu, wert));
                }
            }
        }
    }

    let bester = archiv.values().map(|(_, w)| *w).fold(f64::NEG_INFINITY, f64::max);
    (bester, messungen, archiv.len())
}

#[derive(Default)]
struct Bilanz {
    treffer: usize,
    regret_summe: f64,
    regret_max: f64,
    messungen: usize,
    schlechtester: Option<usize>,
}

impl Bilanz {
    fn buchen(&mut self, fall: usize, gefunden: f64, optimum: f64, messungen: usize) {
        self.messungen += messungen;
        if (gefunden - optimum).abs() < 1e-9 {
            self.treffer += 1;
            return;
        }
        let regret = if optimum.abs() < 1e-9 {
            0.0
        } else {
            (optimum - gefunden) / optimum * 100.0
        };
        self.regret_summe += regret;
        if regret > self.regret_max {
            self.regret_max = regret;
            self.schlechtester = Some(fall);
        }
    }

    fn zeile(&self, name: &str) {
        let verfehlt = FAELLE - self.treffer;
        let mittel = if verfehlt == 0 { 0.0 } else { self.regret_summe / verfehlt as f64 };
        println!(
            "  {name:<22} {:>2} / {FAELLE}  {:>5.1} %   {mittel:>7.2} %  {:>7.2} %   {:>8}   {}",
            self.treffer,
            self.treffer as f64 / FAELLE as f64 * 100.0,
            self.regret_max,
            self.messungen,
            self.schlechtester.map_or("—".into(), |f| format!("Fall {f}")),
        );
    }
}

fn main() {
    println!("== Suchguete, erweitert (Fortsetzung von `suchguete`) ==");
    println!();
    println!("  Faelle {FAELLE} · Bosse {BOSSE:?} · Startwert {SEED} · Durchgaenge {RUNS}");
    println!("  Kandidaten je Fall 6 bis 9 · Laenge 3 bis 4");
    println!("  Beam-Breiten {BEAM_BREITEN:?} · LNS {LNS_RUNDEN} Runden, {LNS_ZERSTOERUNG} Zeilen je Runde");
    println!("  QD-Nischen: (Laenge, Zahl bedingter Zeilen) — MEINE Achsenwahl, keine Design-Entscheidung");
    println!();

    // ZUSICHERUNG (Worker-3, 7971): Ist k >= P(n, L-1), hat der Beam nie etwas
    // abgeschnitten — dann MUSS er finden, was `exhaustive` findet. Faellt das
    // durch, ist der Beam kaputt und nicht die Erklaerung.
    {
        let alle = all_candidates();
        let pool: Vec<Candidate> = alle.iter().take(6).cloned().collect();
        let cfg = config("boss-1-1");
        let exakt = exhaustive(&cfg, &pool, 3);
        let (b, m) = beam(&cfg, &pool, 3, 30);
        println!("  ZUSICHERUNG n=6 L=3 k=30 (noetig waeren P(6,2)=30):");
        println!("    erschoepfend {:.1} in {} Messungen", exakt.best.mean, exakt.measured);
        println!("    beam k=30    {b:.1} in {m} Messungen");
        assert!((b - exakt.best.mean).abs() < 1e-9,
            "BEAM IST KAPUTT: bei k=30 darf nichts abgeschnitten werden, trotzdem {b} statt {}",
            exakt.best.mean);
        println!("    -> gleich, der Beam schneidet bei k=30 nichts ab");
        println!();
    }

    let faelle = faelle();
    let mut b_greedy = Bilanz::default();
    let mut b_paare = Bilanz::default();
    let mut b_beam: Vec<Bilanz> = BEAM_BREITEN.iter().map(|_| Bilanz::default()).collect();
    let mut b_lns = Bilanz::default();
    let mut b_qd = Bilanz::default();
    let mut nischen_gesamt = 0usize;

    for fall in &faelle {
        let cfg = config(fall.boss);
        let exakt = exhaustive(&cfg, &fall.pool, fall.max_len);
        let optimum = exakt.best.mean;

        let g = greedy_opts(&cfg, &fall.pool, fall.max_len, &[], false);
        b_greedy.buchen(fall.nummer, g.best.mean, optimum, g.measured);
        let p = greedy_opts(&cfg, &fall.pool, fall.max_len, &[], true);
        b_paare.buchen(fall.nummer, p.best.mean, optimum, p.measured);

        for (i, &k) in BEAM_BREITEN.iter().enumerate() {
            let (wert, m) = beam(&cfg, &fall.pool, fall.max_len, k);
            b_beam[i].buchen(fall.nummer, wert, optimum, m);
        }

        let (l, lm) = lns(&cfg, &fall.pool, fall.max_len);
        b_lns.buchen(fall.nummer, l, optimum, lm);

        let (q, qm, nischen) = qd(&cfg, &fall.pool, fall.max_len);
        b_qd.buchen(fall.nummer, q, optimum, qm);
        nischen_gesamt += nischen;
    }

    println!("  Verfahren               Treffer   Quote   Regret-M  Regret-Max   Messungen   schlechtester");
    println!("  {}", "-".repeat(93));
    b_greedy.zeile("greedy ohne Paare");
    b_paare.zeile("greedy mit Paaren");
    for (i, &k) in BEAM_BREITEN.iter().enumerate() {
        b_beam[i].zeile(&format!("beam k={k}"));
    }
    b_lns.zeile("LNS");
    b_qd.zeile("QD (Archiv)");
    println!(
        "  {:<22} {:>2} / {FAELLE}  100.0 %      0.00 %     0.00 %   {:>8}   —",
        "erschoepfend",
        FAELLE,
        faelle
            .iter()
            .map(|f| exhaustive(&config(f.boss), &f.pool, f.max_len).measured)
            .sum::<usize>()
    );
    println!();
    println!("  QD-Archiv: {:.1} Nischen je Fall im Mittel", nischen_gesamt as f64 / FAELLE as f64);
    println!();
    println!("  Regret-M ist das Mittel ueber die VERFEHLTEN Faelle, nicht ueber alle.");
}
