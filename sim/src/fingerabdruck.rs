//! Der Fingerabdruck eines Balance-Ergebnisses — über jedes Feld, bitgenau
//! (#919 AC6, #891 N6).
//!
//! # Wozu
//!
//! „Zwei Läufe mit unterschiedlicher Strangzahl liefern bitgleich dasselbe
//! Ergebnis. Nachgewiesen über den Fingerabdruck, nicht über formatierte
//! Zahlen" (#919 AC6). Der Strangvergleich davor nahm eine Auswahl: Mittelwerte,
//! min/max, Siege und Wirkungen. Ohne die Sortierung nach Startwert in
//! [`crate::balance::run`] blieb er grün (Job d27fee0e, nachgefahren in Job
//! dfe079cf).
//!
//! Gemessen am 29.10. in Job dfe079cf, ohne diese Sortierung:
//!
//! ```text
//! weichen ab     die Probenfolge (samples.seed, .damage, .killed_at_ms, ...)
//!                damage.stddev/.stderr/.ci95, kill_time_ms.stddev/.stderr/.ci95
//! bleiben gleich Mittelwerte, min, max, Anzahlen, Siege, Tode, Wirkungen, und
//!                jede Statistik, deren Werte vor der Auswertung nach Größe
//!                sortiert werden
//! ```
//!
//! **Die Streuung allein trägt nicht.** Im selben Lauf blieb `damage.stddev`
//! bei vier Strängen gegen den voreingestellten Boss bitgleich, obwohl die
//! Proben anders angeordnet ankamen: Ob eine andere Summationsreihenfolge im
//! letzten Bit auffällt, hängt an den Zahlen. Die Startwert-Folge zeigt die
//! Umordnung immer. Deshalb geht hier **jedes** Feld ein, nicht eine Auswahl.
//!
//! # Was nicht eingeht
//!
//! Nur `threads`: Die Aufteilung darf das Ergebnis nicht berühren, und genau das
//! soll der Abdruck prüfen. Ginge sie ein, unterschieden sich zwei gesunde Läufe
//! mit verschiedener Strangzahl immer.
//!
//! # Die Destrukturierung ohne `..` ist Absicht
//!
//! Wie in [`crate::balance::BalanceResult::ergebnisentwurf`]: Bekommt das
//! Ergebnis, eine Probe oder eine Statistik ein Feld dazu, bricht der Bau hier.
//! Ohne den Bruch wäre die Voreinstellung „geht still nicht ein" — und ein
//! Abdruck, der ein Feld nicht sieht, meldet für dieses Feld immer Gleichheit.
//!
//! # Dasselbe Verfahren wie überall
//!
//! FNV-1a über 32 Bit aus [`crate::checksum`], Gleitkommawerte über ihre Bits.
//! Zwei Zahlen, die sich im letzten Bit unterscheiden, drucken dieselbe Zeile
//! und ergeben trotzdem verschiedene Abdrücke.

use crate::balance::{BalanceResult, Sample, Statistics};
use crate::checksum::{mix_float, mix_int, FNV_OFFSET};
use crate::time::Millis;

/// Ein Feld des Ergebnisses mit seinem eigenen Abdruck.
///
/// **Einzeln statt nur als eine Zahl**, damit ein Vergleich sagen kann, WELCHES
/// Feld abweicht. „Die Abdrücke stimmen nicht" hieße, die Stelle von Hand zu
/// suchen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feld {
    /// Der Name wie im Ergebnis, etwa `damage.stddev` oder `samples.seed`.
    pub name: String,
    /// Der Abdruck dieses einen Felds.
    pub abdruck: u32,
}

/// Der Abdruck des ganzen Ergebnisses: alle [`felder`] in ihrer Reihenfolge.
#[must_use]
pub fn abdruck(ergebnis: &BalanceResult) -> u32 {
    felder(ergebnis)
        .iter()
        .fold(FNV_OFFSET, |h, feld| mix_int(h, feld.abdruck as i32))
}

/// Die Namen der Felder, in denen sich zwei Ergebnisse unterscheiden.
///
/// Leer heißt bitgleich, Feld für Feld. Die Felder stehen in beiden in
/// derselben Reihenfolge, weil [`felder`] sie aus der Struktur aufzählt.
#[must_use]
pub fn abweichungen(a: &BalanceResult, b: &BalanceResult) -> Vec<String> {
    felder(a)
        .into_iter()
        .zip(felder(b))
        .filter(|(x, y)| x.abdruck != y.abdruck)
        .map(|(x, _)| x.name)
        .collect()
}

/// Jedes Feld des Ergebnisses mit seinem Abdruck, in fester Reihenfolge.
#[must_use]
pub fn felder(ergebnis: &BalanceResult) -> Vec<Feld> {
    let BalanceResult {
        apl_name,
        iterations,
        base_seed,
        threads: _,
        damage,
        kills,
        damage_taken,
        damage_incoming,
        absorbed,
        healed,
        boss_healed,
        deaths,
        casts,
        kill_time_ms,
        death_time_ms,
        samples,
    } = ergebnis;

    let mut felder = Vec::new();
    let mut feld = |name: &str, wert: u32| {
        felder.push(Feld {
            name: name.to_owned(),
            abdruck: wert,
        });
    };
    feld("apl_name", mix_text(FNV_OFFSET, apl_name));
    feld("iterations", mix_int(FNV_OFFSET, *iterations as i32));
    feld("base_seed", mix_int(FNV_OFFSET, *base_seed as i32));
    for (name, statistik) in [
        ("damage", damage),
        ("damage_taken", damage_taken),
        ("damage_incoming", damage_incoming),
        ("absorbed", absorbed),
        ("healed", healed),
        ("boss_healed", boss_healed),
        ("kill_time_ms", kill_time_ms),
        ("death_time_ms", death_time_ms),
    ] {
        for (teil, wert) in statistik_felder(statistik) {
            feld(&format!("{name}.{teil}"), wert);
        }
    }
    feld("kills", mix_usize(FNV_OFFSET, *kills));
    feld("deaths", mix_usize(FNV_OFFSET, *deaths));
    feld(
        "casts",
        casts
            .iter()
            .fold(mix_usize(FNV_OFFSET, casts.len()), |h, n| {
                mix_int(h, *n as i32)
            }),
    );
    for (teil, wert) in proben_felder(samples) {
        feld(&format!("samples.{teil}"), wert);
    }
    felder
}

/// Die sieben Felder einer Statistik, jedes mit seinem Abdruck.
fn statistik_felder(statistik: &Statistics) -> [(&'static str, u32); 7] {
    let Statistics {
        count,
        mean,
        stddev,
        stderr,
        ci95,
        min,
        max,
    } = *statistik;
    [
        ("count", mix_usize(FNV_OFFSET, count)),
        ("mean", mix_float(FNV_OFFSET, mean)),
        ("stddev", mix_float(FNV_OFFSET, stddev)),
        ("stderr", mix_float(FNV_OFFSET, stderr)),
        ("ci95", mix_float(FNV_OFFSET, ci95)),
        ("min", mix_float(FNV_OFFSET, min)),
        ("max", mix_float(FNV_OFFSET, max)),
    ]
}

/// Die Proben, **je Feld über die ganze Folge** und in ihrer Reihenfolge.
///
/// `samples.seed` ist die Startwert-Folge: Sie unterscheidet „dieselben
/// Durchgänge, anders angeordnet" von „dieselben Durchgänge". Die übrigen acht
/// binden jeden Wert an seinen Platz in der Folge.
fn proben_felder(proben: &[Sample]) -> [(&'static str, u32); 9] {
    let anfang = mix_usize(FNV_OFFSET, proben.len());
    let mut h = [anfang; 9];
    for probe in proben {
        let Sample {
            seed,
            damage,
            killed_at_ms,
            damage_taken,
            absorbed,
            healed,
            boss_healed,
            died,
            died_at_ms,
        } = *probe;
        h[0] = mix_int(h[0], seed as i32);
        h[1] = mix_int(h[1], damage as i32);
        h[2] = mix_zeitpunkt(h[2], killed_at_ms);
        h[3] = mix_int(h[3], damage_taken as i32);
        h[4] = mix_int(h[4], absorbed as i32);
        h[5] = mix_int(h[5], healed as i32);
        h[6] = mix_int(h[6], boss_healed as i32);
        h[7] = mix_int(h[7], i32::from(died));
        h[8] = mix_zeitpunkt(h[8], died_at_ms);
    }
    [
        ("seed", h[0]),
        ("damage", h[1]),
        ("killed_at_ms", h[2]),
        ("damage_taken", h[3]),
        ("absorbed", h[4]),
        ("healed", h[5]),
        ("boss_healed", h[6]),
        ("died", h[7]),
        ("died_at_ms", h[8]),
    ]
}

/// Ein Zeitpunkt oder keiner — „kein Sieg" und „Sieg bei 0 ms" sind
/// verschiedene Aussagen und ergeben verschiedene Abdrücke.
fn mix_zeitpunkt(hash: u32, zeitpunkt: Option<Millis>) -> u32 {
    match zeitpunkt {
        Some(ms) => mix_int(mix_int(hash, 1), ms as i32),
        None => mix_int(hash, 0),
    }
}

/// Eine Anzahl über beide Hälften ihrer 64 Bit.
fn mix_usize(hash: u32, wert: usize) -> u32 {
    let wert = wert as u64;
    mix_int(
        mix_int(hash, wert as u32 as i32),
        (wert >> 32) as u32 as i32,
    )
}

/// Länge und Bytes — sonst gäben `"ab"` und `"a"` + `"b"` denselben Abdruck.
fn mix_text(hash: u32, text: &str) -> u32 {
    text.bytes().fold(mix_usize(hash, text.len()), |h, byte| {
        mix_int(h, i32::from(byte))
    })
}

#[cfg(test)]
mod tests {
    use super::{abdruck, abweichungen, felder};
    use crate::apl::{parse, AplName};
    use crate::balance::{run_vollstaendig, BalanceConfig, BalanceResult};

    /// Ein kleiner Lauf mit Würfel — die Streuungen sind dann nicht null.
    fn lauf() -> BalanceResult {
        let liste = parse(AplName("step1-good"), include_str!("../apl/step1-good.apl"))
            .expect("die eingebaute Liste ist gültig");
        let mut worn = crate::gear::WornValues::none();
        worn.purity = 500;
        let config = BalanceConfig {
            iterations: 24,
            worn,
            ..BalanceConfig::default()
        };
        run_vollstaendig(&config, &liste)
    }

    /// **Vertauschte Proben ändern den Abdruck** — die Eigenschaft, an der die
    /// Sortierung nach Startwert hängt. Ohne Strang und ohne Zeitzufall: Zwei
    /// Proben werden von Hand getauscht, sonst bleibt alles gleich.
    #[test]
    fn vertauschte_proben_aendern_den_abdruck() {
        let a = lauf();
        let mut b = a.clone();
        b.samples.swap(0, 1);
        assert_ne!(
            abdruck(&a),
            abdruck(&b),
            "der Tausch zweier Proben blieb unsichtbar"
        );
        let abweichend = abweichungen(&a, &b);
        assert!(
            abweichend.contains(&"samples.seed".to_owned()),
            "die Startwert-Folge meldet den Tausch nicht: {abweichend:?}"
        );
        // Kein Feld außerhalb der Proben darf sich durch den Tausch ändern —
        // sonst nennte der Vergleich Felder, die gar nicht abweichen.
        assert!(
            abweichend.iter().all(|n| n.starts_with("samples.")),
            "der Tausch der Proben aendert {abweichend:?}"
        );
    }

    /// **Das letzte Bit einer Streuung ändert den Abdruck** — und der Vergleich
    /// nennt genau dieses Feld.
    #[test]
    fn das_letzte_bit_einer_streuung_aendert_den_abdruck() {
        let a = lauf();
        assert!(
            a.damage.stddev > 0.0,
            "der Lauf streut nicht — kein Wuerfel"
        );
        let mut b = a.clone();
        b.damage.stddev = f64::from_bits(a.damage.stddev.to_bits() ^ 1);
        // Dieselbe Zeile, wie sie gedruckt würde — der Unterschied liegt
        // unterhalb dessen, was eine Ausgabe auf Stellen zeigt.
        assert_eq!(
            format!("{:.6}", a.damage.stddev),
            format!("{:.6}", b.damage.stddev)
        );
        assert_ne!(abdruck(&a), abdruck(&b));
        assert_eq!(abweichungen(&a, &b), ["damage.stddev"]);
    }

    /// **Die Strangzahl geht nicht ein** — sie ist das, wovon das Ergebnis
    /// unabhängig sein muss.
    #[test]
    fn die_strangzahl_geht_nicht_ein() {
        let a = lauf();
        let mut b = a.clone();
        b.threads = a.threads + 7;
        assert_eq!(abdruck(&a), abdruck(&b));
        assert!(abweichungen(&a, &b).is_empty());
    }

    /// **Jedes Feld hat einen eigenen Namen.** Zwei gleichnamige Felder
    /// machten die Meldung eines Vergleichs mehrdeutig.
    #[test]
    fn jedes_feld_hat_einen_eigenen_namen() {
        let liste = felder(&lauf());
        let mut namen: Vec<&str> = liste.iter().map(|f| f.name.as_str()).collect();
        let anzahl = namen.len();
        namen.sort_unstable();
        namen.dedup();
        assert_eq!(namen.len(), anzahl, "doppelte Feldnamen in {namen:?}");
    }
}
