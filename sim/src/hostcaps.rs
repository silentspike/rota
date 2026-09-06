//! Was ein Rechner meldet — und was er wirklich leistet (Issue #912, E15.0).
//!
//! # Warum gemeldete Werte nicht genügen
//!
//! Prozessor-Flags sagen nicht, was ein Rechner leistet. Was sie verschweigen,
//! ist an den drei vorhandenen Rechnern jeweils belegbar:
//!
//! | was Flags nicht sagen | Beispiel aus dem Bestand |
//! | --- | --- |
//! | heterogene Kerne | der i5-1235U meldet 12 Stränge — vier auf P-Cores, acht auf E-Cores |
//! | Fremdlast | gonet läuft dauerhaft bei Load 3 bis 7; 12 Stränge sind nie 12 Stränge |
//! | Speicherbandbreite | DDR3-1333 gegen DDR4-3200, Faktor 2,4 — in keinem CPU-Flag |
//! | Drosselung durch Wärme | der 5900HS ist ein 35-Watt-Mobilchip mit geteiltem Budget |
//! | Container-Grenzen | ein Container meldet die Kerne des Hosts, darf sie aber nicht nutzen |
//!
//! Deshalb trägt [`HostCaps`] **beides**: die gemeldeten Werte und die
//! gemessene Rate daneben. Wer nur die eine Hälfte liest, bekommt genau die
//! Zahl, die er ohnehin geraten hätte.
//!
//! # Warum die Umgebungsmessung hier steht und nicht im Messprogramm
//!
//! Die Funktionen für PSI, Takt, Drosselung und Kernbindung standen bis zu
//! diesem Issue **privat** in `examples/messumgebung.rs`. Ein Beispiel kann
//! nichts exportieren; jedes weitere Werkzeug hätte sie nachgebaut.
//!
//! **Ein Nachbau antwortet plausibel aus einer zweiten Quelle.** Er ist von der
//! Vorlage nicht zu unterscheiden, solange beide dasselbe sagen — und wenn sie
//! auseinanderlaufen, weiß niemand, welche recht hat. Sie stehen deshalb hier,
//! und `messumgebung.rs` ruft sie auf.
//!
//! # Keine Abhängigkeit, auch nicht für das Einlesen
//!
//! `sim/Cargo.toml` führt für die Bibliothek **keine** Laufzeit-Abhängigkeit,
//! und das ist Absicht (der Kommentar dort nennt den Grund: Determinismus).
//! Alles hier liest `/proc` und `/sys` von Hand. Wo ein Wert fehlt, steht
//! `None` — **nicht** eine Null.
//!
//! Der Unterschied ist der Kern: Eine Drosselungszahl von 0 heißt „nie
//! gedrosselt". Ein fehlendes `thermal_throttle` heißt „diese Maschine sagt es
//! nicht". Wer das zweite als das erste druckt, erzeugt einen Befund, wo keine
//! Messung war.

use std::fs;

/// Obergrenze für `some avg10` aus `/proc/pressure/cpu`, in Prozent.
///
/// **PSI und nicht Load Average**, und das ist keine Geschmacksfrage: Der Load
/// Average zählt auch Prozesse mit, die auf Platte warten, und mittelt über
/// eine ganze Minute. Er meldet Druck, wenn er längst vorbei ist, und
/// verschweigt ihn, solange er frisch ist. `some avg10` sagt, welcher Anteil
/// der letzten zehn Sekunden mindestens eine Aufgabe auf CPU warten ließ — das
/// ist genau die Größe, die eine Durchsatzmessung verfälscht.
///
/// 5 Prozent ist die Schwelle, unter der die Streuung in einer Vormessung
/// unter 3 Prozent blieb (2026-08-18, `examples/messumgebung.rs`). Sie ist eine
/// **Wahl**, keine Naturkonstante.
pub const PSI_CPU_MAX: f64 = 5.0;

/// Obergrenze für `some avg10` aus `/proc/pressure/io` und `/memory`.
///
/// Großzügiger als die CPU-Schwelle: Ein Simulator, der nichts von der Platte
/// liest und nichts nachfordert, verträgt fremde Ein- und Ausgabe besser als
/// fremde Rechenlast.
///
/// **Zehn, nicht zwanzig.** Beim Heben dieser Konstante aus
/// `examples/messumgebung.rs` stand hier zunächst 20 — eine Zahl, die ich
/// gewählt und nicht übernommen hatte. Genau das ist der Fehler, gegen den das
/// Heben gerichtet ist: Zwei Fassungen derselben Schwelle, die plausibel
/// aussehen und verschieden entscheiden.
pub const PSI_IO_MEM_MAX: f64 = 10.0;

/// Welche Befehlssatz-Erweiterungen der Prozessor anbietet.
///
/// **Nur gemeldet, nicht gemessen** — und genau deshalb steht die Struktur in
/// [`HostCaps`] neben `calibrated_fights_per_s` und nicht an dessen Stelle.
/// `avx2` sagt, dass der Befehl existiert; es sagt nichts darüber, ob er auf
/// dieser Maschine schneller ist als der skalare Weg.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IsaSet {
    /// AVX1 — auf `gonet` vorhanden, AVX2 dort nicht.
    pub avx: bool,
    /// AVX2.
    pub avx2: bool,
    /// Verschmolzenes Multiplizieren und Addieren.
    ///
    /// **Es kann die Rundung verändern**, und deshalb steht es hier neben den
    /// Leistungsflags: Ein Pfad mit FMA ist nicht nur schneller, er rechnet
    /// unter Umständen andere letzte Bits — und Auflage 1 verlangt
    /// Bitgleichheit.
    pub fma: bool,
    /// Bit-Manipulation, Satz 1.
    pub bmi1: bool,
    /// Bit-Manipulation, Satz 2.
    pub bmi2: bool,
    /// Halbe Gleitkommazahlen.
    pub f16c: bool,
    /// AVX-512, Grundbefehle (`avx512f`).
    pub avx512: bool,
}

impl IsaSet {
    /// Was dieser Prozessor gerade anbietet.
    ///
    /// Auf anderen Architekturen als x86-64 ist alles `false` — das ist eine
    /// Aussage über den Befehlssatz, keine über die Leistung.
    #[must_use]
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self {
                avx: is_x86_feature_detected!("avx"),
                avx2: is_x86_feature_detected!("avx2"),
                fma: is_x86_feature_detected!("fma"),
                bmi1: is_x86_feature_detected!("bmi1"),
                bmi2: is_x86_feature_detected!("bmi2"),
                f16c: is_x86_feature_detected!("f16c"),
                avx512: is_x86_feature_detected!("avx512f"),
            }
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            Self::default()
        }
    }

    /// Ob ein vektorisierter Vollkampf auf dieser Maschine überhaupt gebaut
    /// werden könnte.
    ///
    /// **`avx` allein reicht**, und das ist der Grund für die Unterscheidung:
    /// `gonet` hat AVX1, aber weder AVX2 noch FMA. Eine Rate „vektorisierter
    /// Vollkampf" ist dort messbar und eine andere als auf einer AVX2-Maschine
    /// — eine Prüfung auf `avx2` erklärte den Rechner fälschlich für ungeeignet.
    #[must_use]
    pub const fn vektorisierbar(self) -> bool {
        self.avx
    }
}

/// Was eine Grafikeinheit meldet.
///
/// **Heute überall `None`.** Die gemeinsame Kalibrierung von Haupt- und
/// Grafikprozessor hängt an einem Auswerter, den es nicht gibt (#912, „out of
/// scope"). Die Felder stehen trotzdem, weil sie im Vertrag stehen: Ein Worker,
/// der sie später anders benennt, erzeugt Felder, die kein anderes Bauteil
/// findet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuCaps {
    /// Hersteller, etwa `NVIDIA`.
    pub vendor: String,
    /// Grafikspeicher in Bytes.
    pub vram: u64,
    /// CUDA-Rechenfähigkeit als Haupt- und Nebenzahl.
    pub cuda_cc: Option<(u32, u32)>,
    /// Welcher Weg die Einheit anspricht, etwa `cuda` oder `vulkan`.
    pub backend: String,
}

/// Ein Zeitstempel als Sekunden seit dem 1.1.1970.
///
/// **Kein `SystemTime`**, weil dieser Wert gespeichert und wieder eingelesen
/// wird und ein Profil sonst an einer Formatierung hinge. `0` heißt: die Uhr
/// war nicht lesbar — auch das ist eine Aussage und keine Zeit.
pub type Timestamp = u64;

/// Was ein Rechner meldet und was er leistet.
///
/// Die Feldnamen sind der Vertrag aus #912 und **nicht frei gewählt**.
#[derive(Debug, Clone)]
pub struct HostCaps {
    // ------------------------------------------------------------- gemeldet
    /// Stränge, die die Maschine hat — unabhängig von der Bindung.
    pub threads_total: usize,
    /// Stränge, die dieser Prozess benutzen darf.
    ///
    /// **Der Unterschied zu `threads_total` ist der Befund**, nicht die Zahl
    /// selbst: Ein Container meldet die Kerne des Hosts und darf sie nicht
    /// nutzen. Wer nur `available_parallelism()` liest, sieht beide Werte
    /// gleich, weil die Funktion die Bindung bereits einrechnet.
    pub threads_usable: usize,
    /// L2 je Kern in Bytes, oder 0.
    pub l2_per_core: u64,
    /// L3 gesamt in Bytes, oder 0.
    pub l3: u64,
    /// Arbeitsspeicher gesamt in Bytes, oder 0.
    pub ram_total: u64,
    /// Freier Arbeitsspeicher in Bytes, oder 0.
    pub ram_free: u64,
    /// Die Befehlssätze.
    pub isa: IsaSet,
    /// Die Grafikeinheit, falls eine erkannt wurde.
    pub gpu: Option<GpuCaps>,

    // -------------------------------------------------------------- GEMESSEN
    /// Kämpfe je Sekunde, **gemessen**.
    ///
    /// `0.0` heißt: noch nicht kalibriert. Es heißt nicht „schafft nichts".
    pub calibrated_fights_per_s: f64,
    /// Relative Streuung der Kalibrierung, als Anteil (0,03 = drei Prozent).
    pub calibration_spread: f64,
    /// Wann kalibriert wurde.
    pub measured_at: Timestamp,
}

impl HostCaps {
    /// Alles, was sich ohne Rechenlast erheben lässt.
    ///
    /// Die drei gemessenen Felder bleiben auf Null — sie füllt
    /// [`crate::kalibrierung`].
    #[must_use]
    pub fn erheben() -> Self {
        let erlaubt = erlaubte_kerne();
        Self {
            threads_total: kerne_gesamt(),
            threads_usable: erlaubt.as_ref().map_or_else(
                || std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get),
                Vec::len,
            ),
            l2_per_core: cache_bytes("index2").unwrap_or(0),
            l3: cache_bytes("index3").unwrap_or(0),
            ram_total: meminfo_bytes("MemTotal:").unwrap_or(0),
            ram_free: meminfo_bytes("MemAvailable:").unwrap_or(0),
            isa: IsaSet::detect(),
            gpu: None,
            calibrated_fights_per_s: 0.0,
            calibration_spread: 0.0,
            measured_at: 0,
        }
    }

    /// Ob die Maschine an eine Teilmenge ihrer Kerne gebunden ist.
    #[must_use]
    pub const fn gebunden(&self) -> bool {
        self.threads_usable < self.threads_total
    }
}

/// Ein Drucksignal mit seinem Wert.
#[derive(Debug, Clone, Copy)]
pub struct Signal {
    /// Wie es heißt.
    pub name: &'static str,
    /// Was gemessen wurde.
    pub wert: f64,
    /// Ab wo es zu viel ist.
    pub grenze: f64,
    /// Woher der Wert kommt — damit man ihn nachschlagen kann.
    pub quelle: &'static str,
}

impl Signal {
    /// Ob dieses Signal seine Grenze überschreitet.
    ///
    /// **Streng größer, nicht größer-gleich.** Ein Wert genau auf der Grenze
    /// gilt als in Ordnung — sonst wäre `PSI_CPU_MAX = 5.0` in Wahrheit „unter
    /// fünf", und die Zahl im Kommentar meinte etwas anderes als die im Code.
    #[must_use]
    pub fn ueberschritten(&self) -> bool {
        self.wert > self.grenze
    }
}

/// Liest `some avg10` aus einer PSI-Datei.
///
/// Format (Linux, `/proc/pressure/cpu`):
///
/// ```text
/// some avg10=0.00 avg60=0.00 avg300=0.00 total=0
/// ```
///
/// `None`, wenn die Datei fehlt — auf einem Kernel ohne PSI ist das kein
/// Fehler, aber der Bericht muss es sagen, statt 0,0 zu drucken.
#[must_use]
pub fn psi_some_avg10(pfad: &str) -> Option<f64> {
    psi_aus_text(&fs::read_to_string(pfad).ok()?)
}

/// Der Parser dazu, getrennt vom Dateizugriff.
///
/// **Damit er prüfbar ist.** Ein Parser, der nur über `/proc` erreichbar ist,
/// lässt sich nur auf der Maschine prüfen, auf der er gerade läuft — und dort
/// steht immer derselbe Wert. Eine kaputte Zerlegung fällt so nie auf, weil sie
/// zufällig plausible Zahlen liefert.
#[must_use]
pub fn psi_aus_text(text: &str) -> Option<f64> {
    let zeile = text.lines().find(|z| z.starts_with("some "))?;
    let feld = zeile.split_whitespace().find(|f| f.starts_with("avg10="))?;
    feld.trim_start_matches("avg10=").parse().ok()
}

/// Gesamtzahl der Kontextwechsel seit dem Systemstart.
#[must_use]
pub fn kontextwechsel() -> Option<u64> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let zeile = text.lines().find(|z| z.starts_with("ctxt "))?;
    zeile.split_whitespace().nth(1)?.parse().ok()
}

/// Ein- und ausgelagerte Seiten seit dem Systemstart.
#[must_use]
pub fn auslagerung() -> Option<u64> {
    let text = fs::read_to_string("/proc/vmstat").ok()?;
    let hole = |name: &str| -> Option<u64> {
        text.lines()
            .find(|z| z.starts_with(name))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    Some(hole("pswpin ")? + hole("pswpout ")?)
}

/// Mittlerer Takt der angegebenen Kerne, in MHz.
///
/// # Warum das mitgemessen wird
///
/// Auf dem Laptop steht das Plattform-Profil auf `performance`, der Governor
/// aber auf `powersave` (gemessen 2026-08-18). Auf einem Laptop-Ryzen heißt
/// das: Der Takt folgt der Last mit Verzögerung und fällt in Pausen ab —
/// gemessen 2386 MHz bei einer Obergrenze von 4680.
///
/// Genau das erzeugt Streuung zwischen IDENTISCHEN Läufen: Der eine fängt bei
/// hohem Takt an, der nächste bei niedrigem.
///
/// **Gemessen wird nur, nicht gestellt.** Das Profil berührt die ganze Maschine
/// und alle Sitzungen darauf.
#[must_use]
pub fn takt_mhz(kerne: &[usize]) -> Option<f64> {
    let mut summe = 0f64;
    let mut anzahl = 0usize;
    for k in kerne {
        let pfad = format!("/sys/devices/system/cpu/cpu{k}/cpufreq/scaling_cur_freq");
        if let Ok(text) = fs::read_to_string(&pfad) {
            if let Ok(khz) = text.trim().parse::<f64>() {
                summe += khz / 1000.0;
                anzahl += 1;
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    (anzahl > 0).then(|| summe / anzahl as f64)
}

/// Wie oft die CPU wegen Hitze gedrosselt hat.
///
/// Auf einem Laptop ist das der häufigste Grund, warum derselbe Lauf zweimal
/// verschieden schnell ist — und der einzige, den keine Lastmessung sieht.
#[must_use]
pub fn taktdrosselung() -> Option<u64> {
    let mut summe = 0u64;
    let mut gefunden = false;
    let dir = fs::read_dir("/sys/devices/system/cpu").ok()?;
    for eintrag in dir.flatten() {
        let pfad = eintrag.path().join("thermal_throttle/core_throttle_count");
        if let Ok(text) = fs::read_to_string(&pfad) {
            if let Ok(n) = text.trim().parse::<u64>() {
                summe += n;
                gefunden = true;
            }
        }
    }
    gefunden.then_some(summe)
}

/// Nimmt alle Drucksignale auf.
#[must_use]
pub fn drucksignale() -> Vec<Signal> {
    let mut v = Vec::new();
    for (name, pfad, grenze) in [
        ("PSI cpu some avg10", "/proc/pressure/cpu", PSI_CPU_MAX),
        ("PSI io some avg10", "/proc/pressure/io", PSI_IO_MEM_MAX),
        (
            "PSI memory some avg10",
            "/proc/pressure/memory",
            PSI_IO_MEM_MAX,
        ),
    ] {
        if let Some(wert) = psi_some_avg10(pfad) {
            v.push(Signal {
                name,
                wert,
                grenze,
                quelle: pfad,
            });
        }
    }
    v
}

/// Welche Kerne dieser Prozess benutzen darf.
///
/// Liest `Cpus_allowed_list` aus `/proc/self/status`, zum Beispiel `0-7` oder
/// `2,4,6`. Ohne Bindung stehen dort alle Kerne der Maschine.
#[must_use]
pub fn erlaubte_kerne() -> Option<Vec<usize>> {
    let text = fs::read_to_string("/proc/self/status").ok()?;
    let zeile = text.lines().find(|z| z.starts_with("Cpus_allowed_list:"))?;
    kernliste(zeile.split(':').nth(1)?)
}

/// Zerlegt eine Kernliste der Form `0-7`, `2,4,6` oder `0-3,8-11`.
///
/// Getrennt vom Dateizugriff, aus demselben Grund wie [`psi_aus_text`]: Die
/// gemischte Form mit Bereich UND Einzelwert kommt auf dem Laptop nie vor, auf
/// einer Maschine mit abgeschalteten Kernen aber sehr wohl.
#[must_use]
pub fn kernliste(liste: &str) -> Option<Vec<usize>> {
    let mut kerne = Vec::new();
    for teil in liste.trim().split(',') {
        if let Some((a, b)) = teil.split_once('-') {
            let (a, b): (usize, usize) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
            if b < a {
                return None;
            }
            kerne.extend(a..=b);
        } else {
            kerne.push(teil.trim().parse().ok()?);
        }
    }
    (!kerne.is_empty()).then_some(kerne)
}

/// Wie viele Kerne `present` aufführt, also höchste Nummer plus eins.
#[must_use]
pub fn hoechste_kernnummer(text: &str) -> Option<usize> {
    let letzte = text.trim().rsplit([',', '-']).next()?;
    letzte.parse::<usize>().ok().map(|hoechste| hoechste + 1)
}

/// Wie viele Kerne die Maschine hat — **unabhängig von der Bindung**.
///
/// # Warum nicht `available_parallelism`
///
/// Weil die Funktion die CPU-Affinität bereits berücksichtigt. Unter
/// `taskset -c 0-7` liefert sie auf einer 16-Kern-Maschine **8**, nicht 16. Ein
/// Vergleich „erlaubte Kerne < alle Kerne" wäre damit immer falsch.
///
/// Gemessen am 2026-08-18: `taskset -c 0-7` auf einem 5900HS ergab
/// `available_parallelism() = 8` und `Cpus_allowed_list = 0-7` — beide Werte
/// gebunden, der Vergleich also gegen sich selbst.
#[must_use]
pub fn kerne_gesamt() -> usize {
    let aus_sysfs = fs::read_to_string("/sys/devices/system/cpu/present")
        .ok()
        .and_then(|text| hoechste_kernnummer(&text));
    aus_sysfs.unwrap_or_else(|| {
        std::thread::available_parallelism().map_or(0, std::num::NonZeroUsize::get)
    })
}

/// Größe eines Cache-Index von CPU 0, in Bytes.
///
/// `index2` ist üblicherweise L2, `index3` L3. Die Zuordnung ist **nicht**
/// garantiert, und deshalb steht sie hier als Annahme und nicht als Tatsache:
/// Auf Maschinen ohne L3 rutscht der Index. Wer die Zahl braucht, prüft sie
/// gegen `lscpu` — der Bericht druckt beide.
#[must_use]
pub fn cache_bytes(index: &str) -> Option<u64> {
    let pfad = format!("/sys/devices/system/cpu/cpu0/cache/{index}/size");
    let text = fs::read_to_string(pfad).ok()?;
    groesse_aus_text(text.trim())
}

/// Zerlegt eine Größenangabe der Form `512K`, `16M` oder `1024`.
///
/// Getrennt vom Dateizugriff, damit sie prüfbar ist — dieselbe Begründung wie
/// bei [`psi_aus_text`].
#[must_use]
pub fn groesse_aus_text(text: &str) -> Option<u64> {
    let (zahl, faktor) = match text.chars().last()? {
        'K' | 'k' => (&text[..text.len() - 1], 1024),
        'M' | 'm' => (&text[..text.len() - 1], 1024 * 1024),
        'G' | 'g' => (&text[..text.len() - 1], 1024 * 1024 * 1024),
        _ => (text, 1),
    };
    zahl.trim().parse::<u64>().ok().map(|n| n * faktor)
}

/// Ein Feld aus `/proc/meminfo`, in Bytes.
///
/// Die Datei führt Kibibytes; die Umrechnung steht hier und nicht beim
/// Aufrufer, damit sie einmal existiert.
#[must_use]
pub fn meminfo_bytes(feld: &str) -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    let zeile = text.lines().find(|z| z.starts_with(feld))?;
    let kib: u64 = zeile.split_whitespace().nth(1)?.parse().ok()?;
    Some(kib * 1024)
}

/// Ein Vorrat gleichartiger Kerne (Issue #912, AC6).
///
/// # Warum getrennte Vorräte und nicht eine Kernzahl
///
/// Der i5-1235U meldet zwölf Stränge — vier auf zwei P-Kernen mit 4,4 GHz und
/// 1,25 MiB L2, acht auf E-Kernen mit 3,3 GHz und 512 KiB. Ein Arbeitspaket,
/// das auf einem P-Kern in einer Sekunde durchläuft, braucht auf einem E-Kern
/// länger — und wer beide aus demselben Vorrat bedient, wartet am Ende auf den
/// langsamsten.
///
/// **Die Klassen werden gemessen, nicht angenommen**: über
/// `cpufreq/cpuinfo_max_freq` je Kern. Eine Maschine ohne heterogene Kerne
/// liefert genau einen Vorrat — das ist der Normalfall und kein Mangel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kernvorrat {
    /// Die Kernnummern dieses Vorrats.
    pub kerne: Vec<usize>,
    /// Höchsttakt in kHz, wie ihn `cpuinfo_max_freq` meldet.
    pub max_khz: u64,
    /// L2 je Kern in Bytes, oder 0.
    pub l2_per_core: u64,
}

/// Teilt die erlaubten Kerne nach ihrem Höchsttakt in Vorräte.
///
/// Absteigend sortiert: Der schnellste Vorrat steht vorn. `None`, wenn kein
/// Kern seinen Höchsttakt meldet — auf einer Maschine ohne `cpufreq` ist das
/// kein Fehler, aber der Aufrufer darf es nicht als „ein Vorrat" lesen.
#[must_use]
pub fn kernvorraete(kerne: &[usize]) -> Option<Vec<Kernvorrat>> {
    let mut nach_takt: Vec<(u64, usize)> = Vec::new();
    for &k in kerne {
        let pfad = format!("/sys/devices/system/cpu/cpu{k}/cpufreq/cpuinfo_max_freq");
        if let Ok(text) = fs::read_to_string(&pfad) {
            if let Ok(khz) = text.trim().parse::<u64>() {
                nach_takt.push((khz, k));
            }
        }
    }
    if nach_takt.is_empty() {
        return None;
    }
    nach_takt.sort_unstable();
    nach_takt.reverse();
    let mut vorraete: Vec<Kernvorrat> = Vec::new();
    for (khz, k) in nach_takt {
        if let Some(letzter) = vorraete.last_mut() {
            if letzter.max_khz == khz {
                letzter.kerne.push(k);
                continue;
            }
        }
        vorraete.push(Kernvorrat {
            kerne: vec![k],
            max_khz: khz,
            l2_per_core: cache_bytes_von_kern(k, "index2").unwrap_or(0),
        });
    }
    for v in &mut vorraete {
        v.kerne.sort_unstable();
    }
    Some(vorraete)
}

/// Größe eines Cache-Index eines bestimmten Kerns, in Bytes.
#[must_use]
pub fn cache_bytes_von_kern(kern: usize, index: &str) -> Option<u64> {
    let pfad = format!("/sys/devices/system/cpu/cpu{kern}/cache/{index}/size");
    groesse_aus_text(fs::read_to_string(pfad).ok()?.trim())
}

/// Die aktuelle Uhrzeit als Sekunden seit 1970, oder 0.
#[must_use]
pub fn jetzt() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn psi_aus_text_liest_die_some_zeile() {
        let text = "some avg10=1.23 avg60=0.00 avg300=0.00 total=0\n\
                    full avg10=9.99 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(psi_aus_text(text), Some(1.23));
    }

    #[test]
    fn psi_aus_text_nimmt_nicht_die_full_zeile() {
        // **Die Gegenprobe zur Zeile darüber.** Ohne sie wäre nicht belegt,
        // dass der Parser die richtige der beiden Zeilen nimmt — und `full`
        // steht auf einer belasteten Maschine deutlich höher.
        let text = "full avg10=9.99 avg60=0.00 avg300=0.00 total=0\n";
        assert_eq!(psi_aus_text(text), None);
    }

    #[test]
    fn psi_aus_text_meldet_fehlende_datei_als_none() {
        assert_eq!(psi_aus_text(""), None);
        assert_eq!(psi_aus_text("irgendwas anderes\n"), None);
    }

    #[test]
    fn kernliste_kennt_bereich_einzelwert_und_gemischt() {
        assert_eq!(kernliste("0-7"), Some((0..=7).collect()));
        assert_eq!(kernliste("2,4,6"), Some(vec![2, 4, 6]));
        assert_eq!(kernliste("0-3,8-11"), Some(vec![0, 1, 2, 3, 8, 9, 10, 11]));
    }

    #[test]
    fn kernliste_weist_einen_verdrehten_bereich_zurueck() {
        // `7-0` ist keine leere Liste, sondern ein Formatfehler. Wer ihn als
        // leer liest, meldet „keine Kerne erlaubt" und der Läufer bricht mit
        // einer Begründung ab, die nichts mit der Ursache zu tun hat.
        assert_eq!(kernliste("7-0"), None);
        assert_eq!(kernliste(""), None);
    }

    #[test]
    fn hoechste_kernnummer_zaehlt_ab_null() {
        assert_eq!(hoechste_kernnummer("0-15"), Some(16));
        assert_eq!(hoechste_kernnummer("0-3,8-11"), Some(12));
        assert_eq!(hoechste_kernnummer("0"), Some(1));
    }

    #[test]
    fn groesse_aus_text_kennt_die_einheiten() {
        assert_eq!(groesse_aus_text("512K"), Some(512 * 1024));
        assert_eq!(groesse_aus_text("16M"), Some(16 * 1024 * 1024));
        assert_eq!(groesse_aus_text("1024"), Some(1024));
        assert_eq!(groesse_aus_text("kaputt"), None);
    }

    #[test]
    fn isa_meldet_auf_dieser_maschine_etwas_bestimmtes() {
        // **Keine Zusicherung über den Inhalt.** Welche Befehlssätze anliegen,
        // hängt an der Maschine — auf gonet ist `avx2` false, hier true, und
        // eine Zusicherung darauf wäre auf einer der beiden falsch.
        //
        // Geprüft wird die Eigenschaft, die überall gilt: `avx2` ohne `avx`
        // gibt es nicht. Ein Erkennungsfehler, der einzelne Flags vertauscht,
        // fällt daran auf.
        let isa = IsaSet::detect();
        if isa.avx2 {
            assert!(isa.avx, "avx2 ohne avx — die Erkennung ist vertauscht");
        }
        assert_eq!(isa.vektorisierbar(), isa.avx);
    }

    #[test]
    fn kernvorraete_decken_alle_erlaubten_kerne_genau_einmal() {
        let Some(erlaubt) = erlaubte_kerne() else {
            return; // Kein /proc/self/status — dann sagt dieser Fall nichts.
        };
        let Some(vorraete) = kernvorraete(&erlaubt) else {
            return; // Keine cpufreq-Angaben; auch das ist kein Fehler.
        };
        assert!(!vorraete.is_empty(), "Vorratsliste leer trotz Some");

        // **Jeder Kern in genau einem Vorrat.** Ein Kern in zweien bekäme
        // doppelt Arbeit, einer in keinem bliebe leer — beides sieht in einer
        // Durchsatzmessung wie eine langsame Maschine aus.
        let mut alle: Vec<usize> = vorraete.iter().flat_map(|v| v.kerne.clone()).collect();
        alle.sort_unstable();
        let mut erwartet: Vec<usize> = erlaubt
            .iter()
            .copied()
            .filter(|k| {
                std::fs::read_to_string(format!(
                    "/sys/devices/system/cpu/cpu{k}/cpufreq/cpuinfo_max_freq"
                ))
                .is_ok()
            })
            .collect();
        erwartet.sort_unstable();
        assert_eq!(alle, erwartet, "Kerne doppelt oder fehlend");

        // Absteigend nach Takt, damit der schnellste Vorrat vorn steht.
        for paar in vorraete.windows(2) {
            assert!(
                paar[0].max_khz > paar[1].max_khz,
                "Vorräte nicht absteigend: {} dann {}",
                paar[0].max_khz,
                paar[1].max_khz
            );
        }
    }

    #[test]
    fn erheben_liefert_plausible_kernzahlen() {
        let caps = HostCaps::erheben();
        assert!(caps.threads_total > 0, "keine Kerne gefunden");
        assert!(
            caps.threads_usable <= caps.threads_total,
            "mehr benutzbare als vorhandene Stränge: {} > {}",
            caps.threads_usable,
            caps.threads_total
        );
        // Die drei gemessenen Felder sind nach `erheben` bewusst leer.
        assert_eq!(caps.calibrated_fights_per_s, 0.0);
        assert_eq!(caps.measured_at, 0);
    }
}
