//! Die Leistungszähler des Betriebssystems, über `perf_event_open` (AC1, AC4).
//!
//! # Warum ohne Kiste
//!
//! Die Bibliothek führt keine Laufzeit-Abhängigkeit (`sim/Cargo.toml` nennt
//! den Grund). `perf_event_open` hat in der C-Bibliothek keinen eigenen
//! Eintrag; es wird über `syscall(2)` gerufen, und das steht in der
//! C-Bibliothek, gegen die `std` ohnehin bindet. Mehr braucht es nicht: ein
//! Systemaufruf zum Öffnen, `read(2)` über [`File`] zum Lesen, `close(2)`
//! beim Fallenlassen.
//!
//! # Warum `inherit` und Lesen im Anzeigestrang
//!
//! Geöffnet wird **einmal**, im Strang, der die Arbeitsstränge startet, mit
//! `inherit = 1`. Jeder später gestartete Strang erbt die Zähler — auch die
//! Arbeitsstränge, die [`crate::balance`] je Auswertung neu anlegt. Ein Lesen
//! des Gruppenführers liefert die Summe über den eigenen Strang, die
//! laufenden und die schon beendeten Erben: Der Kernel liest beim `read` die
//! Kinder mit (`perf_read_group` → `__perf_read_group_add`). Damit liest der
//! Anzeigestrang denselben Stand, den das Zählwerk aus #933 im selben
//! Augenblick zählt, und Fehlsprünge **je Kampf** teilen Zähler und Nenner
//! aus derselben Menge von Strängen.
//!
//! Die Alternative — je Arbeitspaket öffnen und schließen — kostete acht
//! Systemaufrufe je Paket und sieht während eines langen Pakets nichts.
//!
//! Was mitzählt, obwohl es kein Kampf ist: der Anzeigestrang selbst und ein
//! Beobachter der Steuerdatei, wenn sie nach dem Öffnen gestartet werden.
//! Beide schlafen fast immer; sie gehen in die Kosten je Kampf ein wie
//! Sortieren und Statistik des Leitstrangs, und zwar im Lauf wie in seiner
//! Grundlage gleich.
//!
//! # Nur Benutzerraum
//!
//! `exclude_kernel` und `exclude_hv` sind gesetzt. Bei
//! `perf_event_paranoid = 2` (Laptop, abgelesen am 2026-10-28) darf ein
//! gewöhnlicher Prozess nur so zählen; ohne die beiden Bits schlägt schon das
//! Öffnen fehl.

use std::fs::File;
use std::io::{self, Read};

use super::kernklasse::Adapter;

/// Warum es keine Leistungszähler gibt — **eine Fähigkeit, keine
/// Selbstverständlichkeit** (AC4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sperre {
    /// Der Fehler des Systemaufrufs, etwa 13 für `EACCES` — `None`, wenn gar
    /// kein Aufruf stattfand.
    pub errno: Option<i32>,
    /// `/proc/sys/kernel/perf_event_paranoid` beim Versuch — `None`, wenn
    /// nicht lesbar.
    pub paranoid: Option<i32>,
    /// Was nicht ging, als Satz.
    pub grund: String,
}

impl Sperre {
    /// Die Sperre als ein Satz mit allen Angaben.
    #[must_use]
    pub fn satz(&self) -> String {
        let mut s = self.grund.clone();
        if let Some(e) = self.errno {
            s.push_str(&format!(" (errno {e}{})", errno_name(e)));
        }
        if let Some(p) = self.paranoid {
            s.push_str(&format!(", perf_event_paranoid = {p}"));
        }
        s
    }
}

fn errno_name(e: i32) -> &'static str {
    match e {
        1 => " EPERM",
        2 => " ENOENT",
        13 => " EACCES",
        22 => " EINVAL",
        24 => " EMFILE",
        38 => " ENOSYS",
        95 => " EOPNOTSUPP",
        _ => "",
    }
}

/// Der Stand einer Zählergruppe, wie der Kernel ihn liefert.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gruppenstand {
    /// Nanosekunden, in denen die Gruppe eingeschaltet war — über alle
    /// zählenden Stränge summiert.
    pub eingeschaltet_ns: u64,
    /// Davon tatsächlich auf der Hardware gezählt. Weniger als
    /// `eingeschaltet_ns`, wenn der Kernel die Gruppen abwechselnd plant.
    pub gezaehlt_ns: u64,
    /// Die Werte in der Reihenfolge des Adapters, mit dem Faktor des
    /// Ereignisses malgenommen.
    pub werte: Vec<u64>,
}

/// Ein Zählerstand: je Gruppe des Adapters ein [`Gruppenstand`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Zaehlerstand {
    /// In der Reihenfolge der Gruppen des Adapters.
    pub gruppen: Vec<Gruppenstand>,
}

/// Geöffnete Leistungszähler, nach dem Plan eines [`Adapter`]s.
#[derive(Debug)]
pub struct Leistungszaehler {
    adapter: &'static Adapter,
    /// Je Gruppe die Dateien; die erste ist der Gruppenführer.
    gruppen: Vec<Vec<File>>,
    /// Ob der Gruppenführer die ganze Gruppe auf einmal liefert
    /// (`PERF_FORMAT_GROUP`). Ohne wird jedes Ereignis einzeln gelesen.
    gruppenweise: bool,
}

/// `PERF_FORMAT_TOTAL_TIME_ENABLED | PERF_FORMAT_TOTAL_TIME_RUNNING`.
const FORMAT_ZEITEN: u64 = 1 | 2;
/// `PERF_FORMAT_GROUP`.
const FORMAT_GRUPPE: u64 = 8;
/// `inherit` (Bit 1), `exclude_kernel` (Bit 5), `exclude_hv` (Bit 6).
const SCHALTER: u64 = (1 << 1) | (1 << 5) | (1 << 6);

impl Leistungszaehler {
    /// Öffnet die Zähler des Adapters für den laufenden Strang und alle
    /// Stränge, die er danach startet.
    ///
    /// # Errors
    ///
    /// Eine [`Sperre`] mit dem Fehler des ersten Ereignisses, das sich nicht
    /// öffnen ließ — im Container auf node3 `EACCES` schon beim ersten
    /// (Issue #934, AC4).
    pub fn oeffnen(adapter: &'static Adapter) -> Result<Self, Sperre> {
        match Self::oeffnen_mit(adapter, true) {
            Ok(z) => Ok(z),
            // Ein Kernel, der Gruppen mit `inherit` nicht auf einmal liest,
            // lehnt das Format mit EINVAL ab. Dann einzeln lesen, statt die
            // Fähigkeit zu verneinen.
            Err(s) if s.errno == Some(22) => Self::oeffnen_mit(adapter, false),
            Err(s) => Err(s),
        }
    }

    fn oeffnen_mit(adapter: &'static Adapter, gruppenweise: bool) -> Result<Self, Sperre> {
        let format = FORMAT_ZEITEN | if gruppenweise { FORMAT_GRUPPE } else { 0 };
        let mut gruppen = Vec::with_capacity(adapter.gruppen.len());
        for ereignisse in adapter.gruppen {
            let mut dateien: Vec<File> = Vec::with_capacity(ereignisse.len());
            for ereignis in ereignisse {
                let (typ, config) = ereignis.quelle.typ_und_config();
                let fuehrer = dateien.first().map(raw_fd);
                let datei = system::perf_event_open(typ, config, SCHALTER, format, fuehrer)
                    .map_err(|e| Sperre {
                        errno: e.raw_os_error(),
                        paranoid: paranoid(),
                        grund: format!("perf_event_open für {} gescheitert", ereignis.name),
                    })?;
                dateien.push(datei);
            }
            gruppen.push(dateien);
        }
        Ok(Self {
            adapter,
            gruppen,
            gruppenweise,
        })
    }

    /// Nach welchem Adapter gezählt wird.
    #[must_use]
    pub const fn adapter(&self) -> &'static Adapter {
        self.adapter
    }

    /// Liest alle Gruppen.
    ///
    /// # Errors
    ///
    /// Ein Lesefehler des Kernels, oder eine Antwort, die nicht so lang ist,
    /// wie das bestellte Format verlangt.
    pub fn lesen(&self) -> io::Result<Zaehlerstand> {
        let mut gruppen = Vec::with_capacity(self.gruppen.len());
        for (dateien, ereignisse) in self.gruppen.iter().zip(self.adapter.gruppen) {
            let mut stand = Gruppenstand::default();
            if self.gruppenweise {
                // [anzahl, eingeschaltet, gezählt, wert0, wert1, …]
                let w = woerter(&dateien[0], 3 + dateien.len())?;
                if w[0] as usize != dateien.len() {
                    return Err(io::Error::other(format!(
                        "der Kernel meldet {} Ereignisse in der Gruppe, geöffnet waren {}",
                        w[0],
                        dateien.len()
                    )));
                }
                stand.eingeschaltet_ns = w[1];
                stand.gezaehlt_ns = w[2];
                stand.werte = w[3..].to_vec();
            } else {
                for (i, datei) in dateien.iter().enumerate() {
                    // [wert, eingeschaltet, gezählt]
                    let w = woerter(datei, 3)?;
                    if i == 0 {
                        stand.eingeschaltet_ns = w[1];
                        stand.gezaehlt_ns = w[2];
                    }
                    stand.werte.push(w[0]);
                }
            }
            for (wert, ereignis) in stand.werte.iter_mut().zip(ereignisse.iter()) {
                *wert = wert.saturating_mul(ereignis.faktor);
            }
            gruppen.push(stand);
        }
        Ok(Zaehlerstand { gruppen })
    }
}

/// Liest `anzahl` Wörter zu acht Byte.
fn woerter(mut datei: &File, anzahl: usize) -> io::Result<Vec<u64>> {
    let mut puffer = vec![0u8; anzahl * 8];
    datei.read_exact(&mut puffer)?;
    Ok(puffer
        .as_chunks::<8>()
        .0
        .iter()
        .map(|c| u64::from_ne_bytes(*c))
        .collect())
}

fn raw_fd(datei: &File) -> i32 {
    use std::os::fd::AsRawFd;
    datei.as_raw_fd()
}

/// `perf_event_paranoid` dieses Rechners.
fn paranoid() -> Option<i32> {
    std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")
        .ok()?
        .trim()
        .parse()
        .ok()
}

/// Die CPU-Zeit des ganzen Prozesses in Nanosekunden — alle Stränge, auch
/// die schon beendeten (`CLOCK_PROCESS_CPUTIME_ID`).
///
/// **Die Zeitmessung des Rückfalls (AC4).** Sie braucht keine Erlaubnis und
/// zählt nicht mit, wie lange ein Strang auf einen Kern gewartet hat — anders
/// als die Wanduhr, die unter Fremdlast jede Rechnung länger aussehen lässt.
#[must_use]
pub fn prozess_cpu_ns() -> Option<u64> {
    system::cpu_ns(system::PROZESSUHR)
}

/// Die CPU-Zeit des laufenden Strangs in Nanosekunden
/// (`CLOCK_THREAD_CPUTIME_ID`).
///
/// **Die Arbeit eines Pakets** (#934): Ein Strang, den Fremdlast für eine
/// Zeitscheibe verdrängt, braucht an der Wanduhr länger, ohne mehr zu
/// rechnen — und die übrigen warten auf ihn. Das ist kein zu großes Paket,
/// sondern ein belegter Kern, und den meldet schon PSI. Gemessen am
/// 2026-10-28 auf dem Laptop bei PSI cpu 6–8 %: 1 000 Kämpfe je Strang und
/// Abschnitt, an der Wanduhr 18 % „Warten".
#[must_use]
pub fn strang_cpu_ns() -> Option<u64> {
    system::cpu_ns(system::STRANGUHR)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod system {
    use std::ffi::{c_int, c_long};
    use std::fs::File;
    use std::io;
    use std::os::fd::FromRawFd;

    extern "C" {
        /// `syscall(2)` der C-Bibliothek. Jedes Argument wird als `long`
        /// übergeben: Die C-Bibliothek liest sie mit `va_arg(ap, long)`, und
        /// ein schmalerer Wert hätte undefinierte obere Bits.
        fn syscall(nummer: c_long, ...) -> c_long;
        /// `clock_gettime(2)`.
        fn clock_gettime(uhr: c_int, zeit: *mut Zeitpunkt) -> c_int;
    }

    /// `struct timespec` auf x86-64.
    #[repr(C)]
    struct Zeitpunkt {
        sekunden: i64,
        nanosekunden: i64,
    }

    /// `__NR_perf_event_open` auf x86-64.
    const SYS_PERF_EVENT_OPEN: c_long = 298;
    /// `PERF_FLAG_FD_CLOEXEC`: Ein gestartetes Programm erbt die Datei nicht.
    const FD_CLOEXEC: c_long = 8;
    /// `CLOCK_PROCESS_CPUTIME_ID`.
    pub(super) const PROZESSUHR: c_int = 2;
    /// `CLOCK_THREAD_CPUTIME_ID`.
    pub(super) const STRANGUHR: c_int = 3;
    /// `pid` 0: der aufrufende Strang.
    const DIESER_STRANG: c_long = 0;
    /// `cpu` -1: auf jedem Kern, auf dem der Strang läuft.
    const JEDER_KERN: c_long = -1;
    /// `group_fd` -1: eine neue Gruppe.
    const NEUE_GRUPPE: c_long = -1;

    /// `struct perf_event_attr` in der ersten Fassung (`PERF_ATTR_SIZE_VER0`,
    /// 64 Byte). Der Kernel nimmt jede Fassung ab dieser an; alles, was er
    /// darüber hinaus kennt, gilt dann als null.
    #[repr(C)]
    struct Attr {
        typ: u32,
        groesse: u32,
        config: u64,
        sample_period: u64,
        sample_type: u64,
        read_format: u64,
        schalter: u64,
        wakeup_events: u32,
        bp_type: u32,
        config1: u64,
    }

    const _: () = assert!(std::mem::size_of::<Attr>() == 64);

    pub(super) fn perf_event_open(
        typ: u32,
        config: u64,
        schalter: u64,
        read_format: u64,
        fuehrer: Option<i32>,
    ) -> io::Result<File> {
        let attr = Attr {
            typ,
            groesse: 64,
            config,
            sample_period: 0,
            sample_type: 0,
            read_format,
            schalter,
            wakeup_events: 0,
            bp_type: 0,
            config1: 0,
        };
        let gruppe = fuehrer.map_or(NEUE_GRUPPE, c_long::from);
        // SAFETY: `attr` lebt bis zum Ende des Aufrufs und hat genau die
        // Größe, die `groesse` nennt. Der Kernel liest die Struktur nur.
        let fd = unsafe {
            syscall(
                SYS_PERF_EVENT_OPEN,
                std::ptr::from_ref(&attr),
                DIESER_STRANG,
                JEDER_KERN,
                gruppe,
                FD_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let fd = i32::try_from(fd).map_err(|_| io::Error::other("Dateinummer zu groß"))?;
        // SAFETY: Der Kernel hat diese Dateinummer eben für uns angelegt;
        // niemand sonst kennt oder schließt sie. `File` schließt sie beim
        // Fallenlassen genau einmal.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    pub(super) fn cpu_ns(uhr: c_int) -> Option<u64> {
        let mut zeit = Zeitpunkt {
            sekunden: 0,
            nanosekunden: 0,
        };
        // SAFETY: `zeit` ist ein gültiger, beschreibbarer `struct timespec`.
        let rc = unsafe { clock_gettime(uhr, &raw mut zeit) };
        if rc != 0 {
            return None;
        }
        let s = u64::try_from(zeit.sekunden).ok()?;
        let ns = u64::try_from(zeit.nanosekunden).ok()?;
        Some(s.saturating_mul(1_000_000_000).saturating_add(ns))
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod system {
    use std::fs::File;
    use std::io;

    pub(super) fn perf_event_open(
        _typ: u32,
        _config: u64,
        _schalter: u64,
        _read_format: u64,
        _fuehrer: Option<i32>,
    ) -> io::Result<File> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "perf_event_open gibt es hier nur unter Linux auf x86-64",
        ))
    }

    pub(super) const PROZESSUHR: i32 = 2;
    pub(super) const STRANGUHR: i32 = 3;

    pub(super) fn cpu_ns(_uhr: i32) -> Option<u64> {
        None
    }
}
