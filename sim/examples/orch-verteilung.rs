//! MESSWERKZEUG DES ORCHESTRATORS — nicht committen.
//!
//! Beantwortet zwei Fragen in EINEM Lauf, weil sie zusammengehoeren:
//!
//! 1. **Wie viele Kaempfe je Sekunde schafft dieses System?**
//! 2. **Liefert es dasselbe Ergebnis wie die Zielmaschine?**
//!
//! Die zweite Frage entscheidet ueber die erste. Ein Rechner, der schnell
//! ist, aber andere Zahlen liefert, ist kein zusaetzlicher Rechner, sondern
//! ein zweites Ergebnis — und der Plan macht Bitgleichheit zum Pflichttest.
//!
//! Aufruf mit Strangzahl:  `orch-verteilung [threads]`
//! Ohne Angabe: alle verfuegbaren Kerne, danach ein Lauf mit einem Strang.

use std::time::Instant;

use rota_sim::apl::{Apl, Rule};
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::factors::NEUTRAL;
use rota_sim::rotation_search::{all_candidates, greedy};

/// Feste Arbeitsmenge — auf jedem System identisch, sonst ist der Vergleich
/// wertlos.
const KAEMPFE: u32 = 200_000;
const BOSS: &str = "boss-1-1";
const LAST: &str = "orch-maxload";
const SEED: u32 = 5;

fn config(threads: usize, iterations: u32) -> BalanceConfig {
    BalanceConfig {
        iterations,
        base_seed: SEED,
        threads,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss: boss_by_id(BOSS).expect("Boss fehlt"),
        ..BalanceConfig::default()
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let kerne = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);
    let threads: usize = argv.get(1).and_then(|s| s.parse().ok()).unwrap_or(kerne);

    println!("== System ==");
    println!("  verfuegbare Straenge   {kerne}");
    println!("  benutzte Straenge      {threads}");
    println!("  Zielarchitektur        {}", std::env::consts::ARCH);
    // Welcher Befehlssatz steht bereit? Entscheidet, ob E11.7 hier lauffaehig
    // waere — und ob FMA die Rundung veraendern koennte.
    #[cfg(target_arch = "x86_64")]
    {
        println!(
            "  avx {} · avx2 {} · fma {}",
            is_x86_feature_detected!("avx"),
            is_x86_feature_detected!("avx2"),
            is_x86_feature_detected!("fma")
        );
    }
    println!();

    // ---- Eine feste Rotation, damit die Arbeit ueberall dieselbe ist.
    // greedy() wuerde je nach Timing verschieden weit kommen.
    let pool = all_candidates();
    let gefunden = greedy(&config(1, 20), &pool, 5);
    let apl = Apl {
        name: "fest".to_owned(),
        rules: gefunden
            .chosen
            .iter()
            .enumerate()
            .map(|(i, k)| Rule {
                line: i + 1,
                spell: k.spell,
                condition: k.condition.clone(),
            })
            .collect(),
    };
    println!("== Arbeitspaket ==");
    println!("  Rotation aus greedy, {} Zeilen", apl.rules.len());
    for r in &apl.rules {
        println!("    {:>2}  spell {:>3}", r.line, r.spell);
    }
    println!("  Kaempfe                {KAEMPFE}");
    println!();

    // ---- BITGLEICHHEIT ZUERST. Ohne sie ist der Durchsatz bedeutungslos.
    let probe = rota_sim::balance::run(&config(1, 1000), &apl);
    println!("== Ergebnis-Fingerabdruck (1000 Kaempfe, ein Strang) ==");
    println!("  damage.mean            {:.10}", probe.damage.mean);
    println!("  damage.min             {}", probe.damage.min);
    println!("  damage.max             {}", probe.damage.max);
    println!("  damage_taken.mean      {:.10}", probe.damage_taken.mean);
    println!("  DIESE VIER ZAHLEN MUESSEN AUF JEDEM SYSTEM IDENTISCH SEIN.");
    println!();

    // ---- SCHAERFERER TEST. Der Fingerabdruck oben ist schwach: bei NEUTRAL
    // ist jeder Kampf gleich (min == max), und eine Summe gleicher Werte
    // deckt keine Rundungsabweichung auf. Jetzt mit krummen Faktoren, die
    // die volle Multiplikations- und Rundungskette auslösen — und ueber
    // mehrere Bosse, damit verschiedene Ereignisfolgen mitlaufen.
    println!("== Fingerabdruck MIT krummen Faktoren (der belastbare Test) ==");
    let krumm = rota_sim::factors::DamageFactors {
        essence: 1.137,
        talents: 1.0813,
        gear_effects: 0.9437,
        buffs: 1.2109,
        synergy: 1.0331,
        resistance: 0.8719,
        vulnerability: 1.1543,
        crit: 1.0757,
    };
    let mut summe = 0.0f64;
    for boss_id in ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"] {
        let Some(b) = boss_by_id(boss_id) else {
            continue;
        };
        for seed in [1u32, 7, 42] {
            let c = BalanceConfig {
                iterations: 200,
                base_seed: seed,
                threads: 1,
                duration_ms: None,
                stop_on_kill: false,
                factors: krumm,
                boss: b,
                ..BalanceConfig::default()
            };
            let r = rota_sim::balance::run(&c, &apl);
            summe += r.damage.mean + r.damage_taken.mean;
            println!(
                "  {boss_id} seed {seed:>2}   mean {:>18.10}   min {:>7}  max {:>7}",
                r.damage.mean, r.damage.min, r.damage.max
            );
        }
    }
    println!("  SUMME ALLER MITTELWERTE  {summe:.10}");
    println!("  ACHTUNG: eine formatierte Dezimalzahl ist KEIN Bitvergleich —");
    println!("  zwei verschiedene f64 koennen dieselben 10 Stellen zeigen.");
    println!("  Der Bitvergleich steht unten.");
    println!();

    // ---- DER EIGENTLICHE ARCHITEKTUR-TEST: bitweise, ueber to_bits().
    //
    // `{:.10}` rundet. Zwei f64, die sich im letzten Bit unterscheiden,
    // drucken dieselbe Zeile — genau der Messfehler, den dieser Lauf
    // aufdecken soll. Deshalb wird jeder Wert als Bitmuster gehasht.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a, 64 bit
    let mut mische = |bits: u64| {
        for b in bits.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    };
    let mut werte = 0usize;
    for boss_id in ["boss-1-1", "boss-1-2", "boss-1-3", "boss-1-4"] {
        let Some(b) = boss_by_id(boss_id) else {
            continue;
        };
        for seed in [1u32, 7, 42] {
            let c = BalanceConfig {
                iterations: 200,
                base_seed: seed,
                threads: 1,
                duration_ms: None,
                stop_on_kill: false,
                factors: krumm,
                boss: b,
                ..BalanceConfig::default()
            };
            let r = rota_sim::balance::run(&c, &apl);
            // Jeder Gleitkommawert als BITMUSTER, nicht als Text.
            for v in [
                r.damage.mean,
                r.damage_taken.mean,
                r.healed.mean,
                r.absorbed.mean,
            ] {
                mische(v.to_bits());
                werte += 1;
            }
            // min/max ebenso als Bitmuster — ein Trace-Unterschied darf nicht
            // durchrutschen, nur weil die Mittelwerte zufaellig gleich sind.
            for v in [
                r.damage.min,
                r.damage.max,
                r.damage_taken.min,
                r.damage_taken.max,
            ] {
                mische(v.to_bits());
                werte += 1;
            }
        }
    }
    println!("== BITWEISER FINGERABDRUCK ==");
    println!("  gehashte Werte         {werte}");
    println!("  FNV-1a ueber to_bits   {h:#018x}");
    println!("  DIESE ZAHL muss auf jedem System identisch sein.");
    println!();

    // ---- Durchsatz. Zwei Laeufe: der erste waermt, der zweite zaehlt.
    //
    // **DIE RATE IST SEIT #908 NICHT MEHR IMMER EINE DURCHSATZMESSUNG**, und
    // das steht hier, weil die Zahl sonst zwei verschiedene Dinge bedeutet und
    // beide gleich aussehen (Issue #912).
    //
    // `balance::run` rechnet seit dem 2026-08-19 (7af79720) EINEN Kampf statt
    // aller, wenn nachweislich nichts wuerfelt — und vervielfacht ihn dann auf
    // `iterations`. Die Statistik hat danach wirklich 200 000 Werte, aber
    // gerechnet wurde einer.
    //
    // Dieses Werkzeug ist einen Tag AELTER (b2244dc6, 2026-08-18). Seine
    // Referenzwerte — 210 094 auf dem Laptop, 121 689 auf gonet — stammen
    // aus der Zeit davor und sind echte Durchsatzzahlen. Die Zahl, die es
    // HEUTE meldet, ist es nicht: Zaehler aus der Anforderung, Nenner aus
    // einem einzigen Lauf. Gemessen am 2026-08-22: 9 788 279 auf dem Laptop.
    //
    // Es gibt im `BalanceResult` kein Feld fuer die tatsaechlich gerechneten
    // Kaempfe — `iterations` ist die angeforderte Zahl, und das ist dort
    // richtig. Deshalb wird die Bedingung gefragt, an der die Reduktion haengt.
    let reduziert = rota_sim::durchgaenge::ein_durchgang_genuegt(
        rota_sim::data::EFFECT_NODES,
        &config(threads, KAEMPFE).worn,
    );
    let _ = rota_sim::balance::run(&config(threads, KAEMPFE / 10), &apl);
    let t0 = Instant::now();
    let ergebnis = rota_sim::balance::run(&config(threads, KAEMPFE), &apl);
    let dauer = t0.elapsed();
    let pro_s = f64::from(KAEMPFE) / dauer.as_secs_f64();

    println!("== Durchsatz ==");
    println!("  Dauer                  {:.3} s", dauer.as_secs_f64());
    if reduziert {
        println!("  KAEMPFE JE SEKUNDE     {pro_s:.0}   <== KEINE DURCHSATZMESSUNG");
        println!(
            "  ACHTUNG: `ein_durchgang_genuegt` ist WAHR — es wurde EIN Kampf gerechnet\n\
             \x20 und auf {KAEMPFE} vervielfacht. Die Zahl oben teilt die angeforderte Menge\n\
             \x20 durch die Dauer eines einzigen Laufs.\n\
             \x20\n\
             \x20 Wer einen Durchsatz braucht, misst mit getragenen Werten, bei denen\n\
             \x20 wirklich gewuerfelt wird — etwa `purity = 375`. Siehe\n\
             \x20 `examples/e15-0-faehigkeiten.rs` (Issue #912)."
        );
    } else {
        println!("  KAEMPFE JE SEKUNDE     {pro_s:.0}");
    }
    println!("  je Kampf               {:.2} us", 1e6 / pro_s);
    println!(
        "  Gegenprobe mean        {:.10}   (muss zur Probe oben passen: {})",
        ergebnis.damage.mean,
        if (ergebnis.damage.mean - probe.damage.mean).abs() < 1e-9 {
            "ja"
        } else {
            "NEIN — Strangzahl aendert das Ergebnis!"
        }
    );

    // ---- ENDAUSBAU-LASTPROFIL (E11.0b): derselbe Build gegen einen Boss
    // mit ZEHN gleichzeitigen Systemen statt hoechstens vier.
    if let Some(lastboss) = boss_by_id(LAST) {
        let c = BalanceConfig {
            iterations: KAEMPFE / 4,
            base_seed: SEED,
            threads,
            duration_ms: None,
            stop_on_kill: false,
            factors: NEUTRAL,
            boss: lastboss,
            ..BalanceConfig::default()
        };
        let _ = rota_sim::balance::run(
            &BalanceConfig {
                iterations: 2000,
                ..c
            },
            &apl,
        );
        let t = Instant::now();
        let r = rota_sim::balance::run(&c, &apl);
        let d = t.elapsed();
        let ps = f64::from(KAEMPFE / 4) / d.as_secs_f64();
        println!("== Endausbau-Lastprofil (orch-maxload, 10 Systeme) ==");
        println!("  KAEMPFE JE SEKUNDE     {ps:.0}");
        println!("  je Kampf               {:.2} us", 1e6 / ps);
        println!(
            "  gegen boss-1-1         Faktor {:.2}x langsamer",
            pro_s / ps
        );
        println!("  Schaden mean           {:.1}", r.damage.mean);
        println!();
    } else {
        println!("== Lastprofil-Boss nicht gefunden ==");
    }

    println!("== Systemlast waehrend der Messung pruefen ==");
    println!("  (uptime/vmstat separat aufrufen — eine unruhige Maschine");
    println!("   liefert eine Zahl, die nichts bedeutet)");
}
