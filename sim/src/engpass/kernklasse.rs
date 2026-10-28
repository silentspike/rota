//! Kernklassen und ihre Adapter (AC3): derselbe Zähler bedeutet nicht überall
//! dasselbe.
//!
//! # Warum ein Adapter je Kernklasse
//!
//! Die drei Zwischenspeicher-Ebenen, die Speicher-Wartezeit: Für keine davon
//! gibt es ein allgemeines Ereignis, das auf beiden Zielrechnern dasselbe
//! zählt. Abgelesen am 2026-10-28 aus den Ereignistabellen von `perf 6.19.11`
//! (`perf list --details`, für Zen 5 mit `PERF_CPUID=AuthenticAMD-26-44-0`):
//!
//! | Größe | Zen 3 (Laptop, 5900HS) | Zen 5 (node3, 9700X) |
//! | --- | --- | --- |
//! | Fehlgriffe L1d | `ls_dmnd_fills_from_sys` Maske `0x5f` | dasselbe Ereignis, Maske `0xff` |
//! | Fehlgriffe L2 | Maske `0x5e` — ohne `lcl_l2` | Maske `0xfe` — ohne `local_l2` |
//! | Fehlgriffe L3 | Maske `0x5c` — ohne L2 und eigenen CCX | Maske `0xfc` |
//! | Speicher-Wartezeit | `l2_latency.l2_cycles_waiting_on_fills` (`0x62`/`0x01`), **geviertelt gezählt** | `ex_no_retire.load_not_complete` (`0xd6`/`0xa2`) |
//!
//! Die Masken sind nicht austauschbar: Zen 5 hat ein achtes Bit
//! (`alternate_memories`, `0x80`), das es auf Zen 3 nicht gibt. Und die
//! Speicher-Wartezeit ist auf den beiden Klassen **nicht einmal dieselbe
//! Größe** — Zen 3 kennt das Ereignis `ex_no_retire` nicht, Zen 5 das
//! Ereignis `l2_latency` nicht.
//!
//! # Warum zwei Gruppen zu je vier
//!
//! Auf dem Laptop lassen sich **fünf** Zähler gleichzeitig planen, nicht
//! sechs: Der NMI-Wächter (`/proc/sys/kernel/nmi_watchdog = 1`) belegt einen.
//! Gemessen am 2026-10-28 mit `perf stat -e '{…}'`: fünf Ereignisse zu 100 %
//! gezählt, sechs `<not counted>`. Die sieben Rollen passen also nicht in eine
//! Gruppe. Jede Gruppe trägt deshalb **ihre eigenen Zyklen**: Alles, was aus
//! einer Gruppe als Anteil gerechnet wird, teilt durch Zyklen derselben
//! Gruppe — kein Verhältnis überspannt zwei Gruppen, die der Kernel
//! abwechselnd plant.
//!
//! # Die ruhenden Klassen
//!
//! Sandy Bridge, Alder Lake P und Gracemont E **ruhen — kein Zielrechner
//! (seit 2026-10-28)**, Entscheidung des Auftraggebers (Issue #934, AC3). Sie
//! werden erkannt und benannt, ihr Adapter wird nicht gebaut: Ein Lauf dort
//! sagt, warum er die Zählerzeilen nicht prüft, statt Kodierungen einer anderen
//! Klasse zu benutzen. Die Grafikeinheit bekommt einen Adapter, falls er nötig
//! wird — heute rechnet kein Lauf dort.

/// Was ein Zähler misst — die Rolle in der Regeltabelle, nicht seine
/// Kodierung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rolle {
    /// Kernzyklen außerhalb des Halts.
    Zyklen,
    /// Ausgeführte Befehle.
    Befehle,
    /// Falsch vorhergesagte Sprünge.
    Fehlspruenge,
    /// Bedarfsfüllungen des L1-Datenspeichers, gleich seinen Fehlgriffen.
    FehlgriffeL1,
    /// Füllungen von jenseits des eigenen L2.
    FehlgriffeL2,
    /// Füllungen von jenseits des eigenen L3 (anderer CCX, Speicher).
    FehlgriffeL3,
    /// Zyklen, in denen auf Speicherdaten gewartet wird.
    Speicherwartezeit,
}

impl Rolle {
    /// Der Schlüssel im JSON-Strom.
    #[must_use]
    pub const fn schluessel(self) -> &'static str {
        match self {
            Self::Zyklen => "cycles",
            Self::Befehle => "instructions",
            Self::Fehlspruenge => "branch_misses",
            Self::FehlgriffeL1 => "l1d_misses",
            Self::FehlgriffeL2 => "l2_misses",
            Self::FehlgriffeL3 => "l3_misses",
            Self::Speicherwartezeit => "memory_wait_cycles",
        }
    }
}

/// Wie ein Ereignis beim Betriebssystem bestellt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quelle {
    /// Ein allgemeines Hardware-Ereignis (`PERF_TYPE_HARDWARE`); der Kernel
    /// bildet es je Prozessor auf die passende Kodierung ab.
    Allgemein(u64),
    /// Eine rohe Kodierung (`PERF_TYPE_RAW`): Ereignisnummer und Maske, wie
    /// sie in den Tabellen von `perf` stehen.
    Roh {
        /// Ereignisnummer, bis zwölf Bit (`0x1a0` gibt es auf Zen 5).
        ereignis: u16,
        /// Die Maske.
        maske: u8,
    },
}

impl Quelle {
    /// `PERF_COUNT_HW_CPU_CYCLES`.
    pub const ZYKLEN: Self = Self::Allgemein(0);
    /// `PERF_COUNT_HW_INSTRUCTIONS`.
    pub const BEFEHLE: Self = Self::Allgemein(1);
    /// `PERF_COUNT_HW_BRANCH_MISSES`.
    pub const FEHLSPRUENGE: Self = Self::Allgemein(5);

    /// Typ und `config` für `perf_event_attr`.
    ///
    /// Die Bitlage der rohen Kodierung steht in
    /// `/sys/bus/event_source/devices/cpu/format/`: `event` belegt
    /// `config:0-7,32-35`, `umask` belegt `config:8-15` — abgelesen am
    /// Laptop am 2026-10-28. Die oberen vier Bit der Ereignisnummer liegen
    /// also NICHT direkt über den unteren acht.
    #[must_use]
    pub const fn typ_und_config(self) -> (u32, u64) {
        match self {
            Self::Allgemein(nummer) => (0, nummer),
            Self::Roh { ereignis, maske } => {
                let e = ereignis as u64;
                (
                    4,
                    (e & 0xff) | ((maske as u64) << 8) | (((e >> 8) & 0xf) << 32),
                )
            }
        }
    }
}

/// Ein Zähler eines Adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ereignis {
    /// Was er in der Regeltabelle bedeutet.
    pub rolle: Rolle,
    /// Wie er bestellt wird.
    pub quelle: Quelle,
    /// Womit der gelesene Wert malgenommen wird — 4 bei einem Ereignis, das
    /// laut Tabelle „divided by four" zählt.
    pub faktor: u64,
    /// Der Name in den Tabellen von `perf`, damit man ihn nachschlagen kann.
    pub name: &'static str,
}

/// Was der Adapter einer Kernklasse misst.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adapter {
    /// Für welche Klasse.
    pub kernklasse: Kernklasse,
    /// Zwei Gruppen; jede beginnt mit ihren eigenen Zyklen.
    pub gruppen: [&'static [Ereignis]; 2],
    /// Was man über die Zähler dieser Klasse wissen muss, um sie zu lesen.
    pub hinweis: &'static str,
}

impl Adapter {
    /// Die Gruppe und den Platz darin, an dem eine Rolle gezählt wird — die
    /// Zyklen zählen beide Gruppen, gemeint ist dann die erste.
    #[must_use]
    pub fn platz(&self, rolle: Rolle) -> Option<(usize, usize)> {
        self.gruppen.iter().enumerate().find_map(|(g, ereignisse)| {
            ereignisse
                .iter()
                .position(|e| e.rolle == rolle)
                .map(|p| (g, p))
        })
    }
}

const ZEN3: Adapter = Adapter {
    kernklasse: Kernklasse::Zen3,
    gruppen: [
        &[
            Ereignis {
                rolle: Rolle::Zyklen,
                quelle: Quelle::ZYKLEN,
                faktor: 1,
                name: "cycles",
            },
            Ereignis {
                rolle: Rolle::Befehle,
                quelle: Quelle::BEFEHLE,
                faktor: 1,
                name: "instructions",
            },
            Ereignis {
                rolle: Rolle::Fehlspruenge,
                quelle: Quelle::FEHLSPRUENGE,
                faktor: 1,
                name: "branch-misses",
            },
            Ereignis {
                rolle: Rolle::FehlgriffeL1,
                quelle: Quelle::Roh {
                    ereignis: 0x43,
                    maske: 0x5f,
                },
                faktor: 1,
                name: "ls_dmnd_fills_from_sys (alle Quellen)",
            },
        ],
        &[
            Ereignis {
                rolle: Rolle::Zyklen,
                quelle: Quelle::ZYKLEN,
                faktor: 1,
                name: "cycles",
            },
            Ereignis {
                rolle: Rolle::FehlgriffeL2,
                quelle: Quelle::Roh {
                    ereignis: 0x43,
                    maske: 0x5e,
                },
                faktor: 1,
                name: "ls_dmnd_fills_from_sys (ohne lcl_l2)",
            },
            Ereignis {
                rolle: Rolle::FehlgriffeL3,
                quelle: Quelle::Roh {
                    ereignis: 0x43,
                    maske: 0x5c,
                },
                faktor: 1,
                name: "ls_dmnd_fills_from_sys (ohne lcl_l2, int_cache)",
            },
            Ereignis {
                rolle: Rolle::Speicherwartezeit,
                quelle: Quelle::Roh {
                    ereignis: 0x62,
                    maske: 0x01,
                },
                faktor: 4,
                name: "l2_latency.l2_cycles_waiting_on_fills",
            },
        ],
    ],
    hinweis: "Zen 3: Die Speicher-Wartezeit zählt beide Stränge eines Kerns — rechnet \
              auf dem Geschwisterstrang Fremdlast, steckt sie mit darin.",
};

const ZEN5: Adapter = Adapter {
    kernklasse: Kernklasse::Zen5,
    gruppen: [
        &[
            Ereignis {
                rolle: Rolle::Zyklen,
                quelle: Quelle::ZYKLEN,
                faktor: 1,
                name: "cycles",
            },
            Ereignis {
                rolle: Rolle::Befehle,
                quelle: Quelle::BEFEHLE,
                faktor: 1,
                name: "instructions",
            },
            Ereignis {
                rolle: Rolle::Fehlspruenge,
                quelle: Quelle::FEHLSPRUENGE,
                faktor: 1,
                name: "branch-misses",
            },
            Ereignis {
                rolle: Rolle::FehlgriffeL1,
                quelle: Quelle::Roh {
                    ereignis: 0x43,
                    maske: 0xff,
                },
                faktor: 1,
                name: "ls_dmnd_fills_from_sys.all",
            },
        ],
        &[
            Ereignis {
                rolle: Rolle::Zyklen,
                quelle: Quelle::ZYKLEN,
                faktor: 1,
                name: "cycles",
            },
            Ereignis {
                rolle: Rolle::FehlgriffeL2,
                quelle: Quelle::Roh {
                    ereignis: 0x43,
                    maske: 0xfe,
                },
                faktor: 1,
                name: "ls_dmnd_fills_from_sys (ohne local_l2)",
            },
            Ereignis {
                rolle: Rolle::FehlgriffeL3,
                quelle: Quelle::Roh {
                    ereignis: 0x43,
                    maske: 0xfc,
                },
                faktor: 1,
                name: "ls_dmnd_fills_from_sys (ohne local_l2, local_ccx)",
            },
            Ereignis {
                rolle: Rolle::Speicherwartezeit,
                quelle: Quelle::Roh {
                    ereignis: 0xd6,
                    maske: 0xa2,
                },
                faktor: 1,
                name: "ex_no_retire.load_not_complete",
            },
        ],
    ],
    hinweis: "Zen 5: Die Speicher-Wartezeit sind Zyklen ohne Abschluss, weil der älteste \
              Befehl auf Ladedaten wartet — je Strang, nicht je Kern.",
};

/// Die Kernklassen, die der Wächter unterscheidet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kernklasse {
    /// AMD Familie 0x19, Zen 3 — der Laptop (Ryzen 9 5900HS).
    Zen3,
    /// AMD Familie 0x1A, Zen 5 — node3 (Ryzen 7 9700X).
    Zen5,
    /// Intel Sandy Bridge — **ruht**, kein Zielrechner.
    SandyBridge,
    /// Intel Alder Lake mit P- und E-Kernen — **ruht**, kein Zielrechner.
    AlderLake,
    /// Nichts davon.
    Unbekannt,
}

/// Die Klassen, deren Adapter ruhen, mit dem Grund — einmal, nicht je Klasse.
pub const RUHENDE_KERNKLASSEN: &str = "Sandy Bridge, Alder Lake P und Gracemont E ruhen — kein \
     Zielrechner (seit 2026-10-28, Entscheidung des Auftraggebers, Issue #934 AC3)";

impl Kernklasse {
    /// Die Kennung in Grundlage und Strom.
    #[must_use]
    pub const fn kennung(self) -> &'static str {
        match self {
            Self::Zen3 => "zen3",
            Self::Zen5 => "zen5",
            Self::SandyBridge => "sandy-bridge",
            Self::AlderLake => "alder-lake",
            Self::Unbekannt => "unbekannt",
        }
    }

    /// Die Umkehrung von [`Kernklasse::kennung`].
    #[must_use]
    pub fn aus_kennung(kennung: &str) -> Option<Self> {
        [
            Self::Zen3,
            Self::Zen5,
            Self::SandyBridge,
            Self::AlderLake,
            Self::Unbekannt,
        ]
        .into_iter()
        .find(|k| k.kennung() == kennung)
    }

    /// Der Adapter — oder warum es keinen gibt.
    ///
    /// # Errors
    ///
    /// Bei einer ruhenden oder unbekannten Klasse, mit dem Grund als Satz.
    pub fn adapter(self) -> Result<&'static Adapter, &'static str> {
        match self {
            Self::Zen3 => Ok(&ZEN3),
            Self::Zen5 => Ok(&ZEN5),
            Self::SandyBridge | Self::AlderLake => Err(RUHENDE_KERNKLASSEN),
            Self::Unbekannt => {
                Err("unbekannte Kernklasse — ohne Adapter ist keine Zählerkodierung gedeutet")
            }
        }
    }
}

/// Was `/proc/cpuinfo` über den Prozessor sagt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prozessor {
    /// `vendor_id`, etwa `AuthenticAMD`.
    pub hersteller: String,
    /// `cpu family`, dezimal.
    pub familie: u32,
    /// `model`, dezimal.
    pub modell: u32,
    /// `model name`.
    pub name: String,
}

impl Prozessor {
    /// Liest den ersten Prozessor aus dem Text von `/proc/cpuinfo`.
    ///
    /// `None`, wenn eine der vier Angaben fehlt — dann ist die Klasse nicht
    /// bestimmt, und geraten wird sie nicht.
    #[must_use]
    pub fn aus_cpuinfo(text: &str) -> Option<Self> {
        let feld = |name: &str| {
            text.lines()
                .take_while(|z| !z.trim().is_empty())
                .find_map(|z| {
                    let (schluessel, wert) = z.split_once(':')?;
                    (schluessel.trim() == name).then(|| wert.trim().to_owned())
                })
        };
        Some(Self {
            hersteller: feld("vendor_id")?,
            familie: feld("cpu family")?.parse().ok()?,
            modell: feld("model")?.parse().ok()?,
            name: feld("model name")?,
        })
    }

    /// Liest den Prozessor dieses Rechners.
    #[must_use]
    pub fn erheben() -> Option<Self> {
        Self::aus_cpuinfo(&std::fs::read_to_string("/proc/cpuinfo").ok()?)
    }

    /// Die Kernklasse.
    ///
    /// Die Modellbereiche folgen der Zuordnung, nach der `perf` seine
    /// Ereignistabellen wählt — abgelesen am 2026-10-28, indem `perf list`
    /// mit gesetztem `PERF_CPUID` je Modell gefragt wurde, welche Tabelle es
    /// lädt: Zen 3 ist Familie 25 mit den Modellen `0x0`–`0xf`, `0x2_`,
    /// `0x4_` und `0x5_`; `0x1_`, `0x3_` und `0x6_` laden die Tabelle von
    /// Zen 4. Familie 26 lädt für jedes gefragte Modell (`0x0` bis `0x80`)
    /// die von Zen 5. Zen 4 ist kein Zielrechner und bleibt unbekannt, statt
    /// die Kodierungen von Zen 3 zu erben.
    #[must_use]
    pub fn kernklasse(&self) -> Kernklasse {
        let oben = self.modell >> 4;
        match (self.hersteller.as_str(), self.familie) {
            ("AuthenticAMD", 25) if matches!(oben, 0 | 2 | 4 | 5) => Kernklasse::Zen3,
            ("AuthenticAMD", 26) => Kernklasse::Zen5,
            ("GenuineIntel", 6) if matches!(self.modell, 0x2a | 0x2d) => Kernklasse::SandyBridge,
            ("GenuineIntel", 6) if matches!(self.modell, 0x97 | 0x9a) => Kernklasse::AlderLake,
            _ => Kernklasse::Unbekannt,
        }
    }
}
