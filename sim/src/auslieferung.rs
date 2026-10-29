//! Der Prüflauf vor Auslieferung über alle wertungsfähigen Bosse (#891 N1, N4).
//!
//! # Wofür
//!
//! Der Masterplan (Abschnitt Simulator, „Prüflauf vor Auslieferung") verlangt:
//!
//! > Die Simulation läuft über jede Kombination aus Boss × Schwierigkeit ×
//! > Erfolg und bestätigt, dass eine Lösung existiert […]. Ohne gefundene
//! > Lösung gilt der Boss als **nicht auslieferbar**. → als automatischer Test
//! > formulieren.
//!
//! Bis #891 N1 rechnete der Bericht genau eine Kombination: THE FIRST
//! CRUCIBLE, und die Machbarkeit mit der festen Liste `step1-good`. Ein Boss,
//! den niemand prüft, kann nicht als unschaffbar auffallen. Dieses Modul
//! rechnet die Ausgaben 2 bis 4 des Masterplans je Kombination:
//!
//! | Ausgabe | hier | Bibliothek |
//! | --- | --- | --- |
//! | 2 Zauber-Vergleichstabelle | [`Pruefung::tabelle`] | [`crate::viabilitaet`] |
//! | 3 Boss-Machbarkeit | [`Pruefung::machbarkeit`] | [`crate::report::feasibility_gegen`] |
//! | 4 Erreichbarkeit der Erfolge | [`Pruefung::erfolge`] | [`crate::report::erfolge_pruefen`] |
//!
//! Ausgabe 1 (Skalierungsfaktoren je Spielwert) ist #891 N2 und nicht Teil
//! dieses Laufs.
//!
//! # Die drei Achsen
//!
//! * **Boss** — [`ScoringSet::all_scored`]: jeder Boss einer Spielinstanz, aus
//!   den Daten abgeleitet, nicht hier aufgezählt (#981). Ein neuer Boss in einer
//!   Spielinstanz steht damit ohne Zutun im Prüfplan.
//! * **Schwierigkeit** — [`Stufe`]. Im Spiel gibt es heute nur Normal
//!   (`app.ts` führt `DIFFICULTY_NORMAL` fest). Eine Stufe ist für diesen Lauf
//!   **ein anderer Boss** ([`Stufe::boss`]): mehr Leben, ein anderes Zeitlimit,
//!   weitere Mechaniken. Kommen Heroisch und Mythisch, bekommen sie hier eine
//!   Variante und in [`Stufe::boss`] ihren Boss; der übrige Lauf bleibt, wie er
//!   ist.
//! * **Erfolg** — [`crate::report::Erfolg`]. Heute der Sieg und die drei
//!   abgeleiteten Zeitmarken; boss-eigene kommen als Varianten dort dazu.
//!
//! # Was eine Lösung ist
//!
//! Die **gesuchte** Rotation: gierig, Länge bis [`MAX_LEN`], gegen genau den
//! Boss der Kombination ([`loesung_suchen`]). Ob sie siegt, entscheidet die
//! Siegbedingung des Bosses ([`crate::objective::Outcome::of`]) — bei
//! `runaway` ist das Überstehen der Frist der Sieg, nicht der Fall des Rings.
//! Dieselbe Bedingung bestimmt, **wonach** gesucht wird
//! ([`crate::rotation_search::bewerte`]): Schaden, bei `runaway` der Ring am
//! Fristende (Festlegung des Orchestrators vom 2026-10-29).
//!
//! **Eine nicht gefundene Lösung ist ein Such-Befund**, kein Beweis, dass es
//! keine gibt. Der Masterplan lässt trotzdem nur die gefundene gelten: Ein
//! Boss, für den die Suche nichts findet, ist nicht auslieferbar, bis sie
//! etwas findet oder jemand eine Lösung vorlegt.
//!
//! # Wanduhr und Kampfzahl je Posten (#891 N4)
//!
//! Jede Kombination zerfällt in fünf [`Teil`]e, und jeder wird für sich
//! gemessen: Wanduhr, gerechnete Kämpfe, gelieferte Durchgänge, abgeschlossene
//! Auswertungen ([`Posten`]). Gezählt wird über das Zählwerk aus
//! [`crate::fortschritt`] — dieselben Zähler, die `rota-sim` anzeigt, kein
//! zweiter daneben. An diesen Zahlen hängen die ruhenden Pakete in #891
//! Abschnitt 4: Ihr Auslöser ist ein Anteil an der Wanduhr oder eine Laufzeit,
//! und ohne Messung je Posten wäre eine Zurückstellung eine stille Streichung.

use std::time::{Duration, Instant};

use crate::balance::BalanceConfig;
use crate::data::{Boss, EFFECT_NODES};
use crate::factors::NEUTRAL;
use crate::fight::{Fight, FightConfig, RunResult};
use crate::fortschritt::Zaehlwerk;
use crate::report::{
    erfolge, erfolge_pruefen, feasibility_gegen, is_shippable, AchievementCheck, Feasibility,
};
use crate::rotation_search::{all_candidates, greedy, Candidate, SearchResult};
use crate::scoring::ScoringSet;
use crate::viabilitaet::{
    befunde_zur_elite, mechanikproben, schulproben, spielbare_zauber, Befundtabelle, Mechanikprobe,
    Schulprobe,
};

/// Höchstlänge der gesuchten Liste und der Projektionen — dieselbe wie im
/// Abdeckungs-Wächter aus #550.
pub const MAX_LEN: usize = 6;

/// Wie viele Listen eine Projektion höchstens messen darf.
///
/// Gemessen am 2026-10-29 auf node3 (CT 105): First-crucible bei Länge 6
/// braucht 53 Projektionen mit zusammen 1 266 939 Listen, die größte hat
/// 79 209; der ganze Abschnitt 1 lief in 15,0 s. Die Grenze liegt so, dass
/// keine Projektion an ihr scheitert, die sich in Sekunden rechnen lässt — sie
/// fängt erst die Projektionen, die mit der Länge der Elite in die Millionen
/// wachsen (neun Zeilen bei Länge 8 sind schon 623 529 Listen,
/// `exhaustive_messungen(9, 8)`).
pub const BUDGET: u64 = 1_000_000;

// ---------------------------------------------------------------- Achsen

/// Eine Schwierigkeitsstufe — die zweite Achse des Prüflaufs.
///
/// **Heute genau eine.** Normal ist der Boss, wie er in `data/bosses/` steht.
/// Heroisch und Mythisch (Masterplan A.4: „mehr Leben, härteres Zeitlimit,
/// zusätzliche Mechaniken") gibt es im Spiel noch nicht; ihre Zahlen und
/// Mechaniken hier vorwegzunehmen hieße, sie nebenbei festzulegen (#1177,
/// Abschnitt 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stufe {
    /// Der Boss, wie er in den Daten steht.
    Normal,
}

impl Stufe {
    /// Alle Stufen, die das Spiel heute kennt.
    pub const ALLE: [Self; 1] = [Self::Normal];

    /// Die Kennung — dieselbe wie im Spiel (`src/loot/rates.ts`).
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Normal => "normal",
        }
    }

    /// Der Name im Bericht.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
        }
    }

    /// Der Boss, gegen den auf dieser Stufe gerechnet wird.
    ///
    /// **Ohne Platzhalter-Arm**: Eine neue Stufe baut erst, wenn sie hier ihren
    /// Boss bekommt. Ein `_ => boss` rechnete sie still als Normal.
    #[must_use]
    pub const fn boss(self, boss: &'static Boss) -> &'static Boss {
        match self {
            Self::Normal => boss,
        }
    }
}

/// Eine Zeile des Prüfplans: ein Boss auf einer Stufe.
#[derive(Debug, Clone, Copy)]
pub struct Kombination {
    /// Der Boss, wie er in den Daten steht.
    pub boss: &'static Boss,
    /// Die Stufe.
    pub stufe: Stufe,
}

impl Kombination {
    /// Der Boss, gegen den gerechnet wird — der aus den Daten auf dieser Stufe.
    #[must_use]
    pub const fn gegner(&self) -> &'static Boss {
        self.stufe.boss(self.boss)
    }
}

/// Jede Kombination aus Boss und Stufe, in der Reihenfolge des Bestands.
///
/// Die Menge ist eine [`ScoringSet`]: Ein Prüfprofil kann hier gar nicht erst
/// hineingeraten (#981).
#[must_use]
pub fn kombinationen(menge: &ScoringSet, stufen: &[Stufe]) -> Vec<Kombination> {
    menge
        .bosses()
        .iter()
        .flat_map(|&boss| stufen.iter().map(move |&stufe| Kombination { boss, stufe }))
        .collect()
}

/// Der Prüfplan des Nachtlaufs: jeder wertungsfähige Boss auf jeder Stufe.
#[must_use]
pub fn pruefplan() -> Vec<Kombination> {
    kombinationen(&ScoringSet::all_scored(), &Stufe::ALLE)
}

// ---------------------------------------------------------------- Rahmen

/// Woraus gesucht wird und wie weit.
///
/// Der Nachtlauf nimmt [`Rahmen::voll`]. Prüffälle nehmen einen kleinen
/// Vorrat, damit derselbe Weg im Debug-Bau in Sekunden durch ist.
#[derive(Debug, Clone)]
pub struct Rahmen {
    /// Der Suchraum der gierigen Suche.
    pub pool: Vec<Candidate>,
    /// Die Zauber, die einen Befund brauchen.
    pub grundmenge: Vec<usize>,
    /// Höchstlänge der Suche und der Projektionen.
    pub max_len: usize,
    /// Listen je Projektion, höchstens.
    pub budget: u64,
}

impl Rahmen {
    /// Jeder spielbare Zauber, jede Kandidatenzeile, Länge bis [`MAX_LEN`].
    #[must_use]
    pub fn voll(budget: u64) -> Self {
        Self {
            pool: all_candidates(),
            grundmenge: spielbare_zauber(),
            max_len: MAX_LEN,
            budget,
        }
    }
}

/// Die Einstellungen jeder Messung des Prüflaufs.
///
/// **Ein Durchgang mit festem Startwert, ohne Ausrüstung**, wie in der
/// erschöpfenden Suche: Ohne Ausrüstung würfelt der Kampf nicht, also ist jede
/// Messung exakt — und nur dann trägt ein Beweis aus den Projektionen.
/// `stop_on_kill` steht auf `false`, sonst wären zwei verschieden schnelle
/// Listen nicht vergleichbar (`rotation_search::measure`); die Machbarkeit
/// fragt den Sieg getrennt ab ([`feasibility_gegen`]).
#[must_use]
pub fn messkonfiguration(boss: &'static Boss) -> BalanceConfig {
    BalanceConfig {
        iterations: 1,
        base_seed: 0,
        threads: 1,
        duration_ms: None,
        stop_on_kill: false,
        factors: NEUTRAL,
        boss,
        ..BalanceConfig::default()
    }
}

// ---------------------------------------------------------------- Posten

/// Die Teile einer Kombination, je für sich gemessen (#891 N4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Teil {
    /// Die gierige Suche nach der Elite.
    Suche,
    /// Zeugen und erschöpfende Projektionen der Zauber-Vergleichstabelle.
    Befunde,
    /// Ob der Boss ohne jede einzelne Schule zu schaffen ist.
    Schulproben,
    /// Welche der drei Antworten jede Mechanik für sich löst.
    Mechanikproben,
    /// Sieg, Reserve und Erfolge der gefundenen Lösung.
    Machbarkeit,
}

impl Teil {
    /// Alle fünf, in der Reihenfolge, in der eine Kombination sie rechnet.
    pub const ALLE: [Self; 5] = [
        Self::Suche,
        Self::Befunde,
        Self::Schulproben,
        Self::Mechanikproben,
        Self::Machbarkeit,
    ];

    /// Der Name im Bericht.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Suche => "Suche",
            Self::Befunde => "Befunde",
            Self::Schulproben => "Schulproben",
            Self::Mechanikproben => "Mechanikproben",
            Self::Machbarkeit => "Machbarkeit",
        }
    }
}

/// Was ein Teil gekostet hat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posten {
    /// Welcher Teil.
    pub teil: Teil,
    /// Wanduhr, vom ersten bis zum letzten Kampf des Teils.
    pub wanduhr: Duration,
    /// Gerechnete Kämpfe — vom Kern gerechnet, nicht angefordert.
    pub kaempfe: u64,
    /// Gelieferte Durchgänge, einschließlich der aus einem Kampf
    /// vervielfachten (#908).
    pub durchgaenge: u64,
    /// Abgeschlossene Auswertungen (eine je gemessener Liste).
    pub auswertungen: u64,
}

/// Rechnet `arbeit` mit eigenem Zählwerk und hält fest, was sie gekostet hat.
///
/// Das Zählwerk zählt jeden Kampf, der über [`crate::balance`] läuft, und die
/// Kämpfe, die ein Aufrufer selbst meldet ([`crate::fortschritt::strang`]).
/// Die Anmeldung endet mit dem Teil; eine äußere bleibt danach bestehen.
fn messen<T>(teil: Teil, arbeit: impl FnOnce() -> T) -> (T, Posten) {
    let werk = Zaehlwerk::neu(1);
    let anmeldung = werk.anmelden();
    let beginn = Instant::now();
    let wert = arbeit();
    let wanduhr = beginn.elapsed();
    drop(anmeldung);
    let stand = werk.momentaufnahme();
    (
        wert,
        Posten {
            teil,
            wanduhr,
            kaempfe: stand.kaempfe,
            durchgaenge: stand.durchgaenge,
            auswertungen: stand.kandidaten_gesamt(),
        },
    )
}

// ---------------------------------------------------------------- Prüfung

/// Alles, was eine Kombination ergeben hat.
#[derive(Debug, Clone)]
pub struct Pruefung {
    /// Welche Kombination.
    pub kombination: Kombination,
    /// Ausgabe 2: vier Befunde je Zauber, gegen die gesuchte Elite.
    pub tabelle: Befundtabelle,
    /// Der Lauf der Elite, für Schaden und Anteil je Zauber.
    pub elite_lauf: RunResult,
    /// Ob der Boss ohne jede einzelne Schule zu schaffen ist.
    pub schulen: Vec<Schulprobe>,
    /// Welche Antworten jede Mechanik löst — oder die Bausteine, die sich nicht
    /// für sich messen lassen.
    pub mechaniken: Result<Vec<Mechanikprobe>, Vec<&'static str>>,
    /// Ausgabe 3: Sieg und Reserve der gefundenen Lösung.
    pub machbarkeit: Feasibility,
    /// Ausgabe 4: je Erfolg, ob eine Lösung gefunden wurde.
    pub erfolge: Vec<AchievementCheck>,
    /// Was jeder Teil gekostet hat, in der Reihenfolge von [`Teil::ALLE`].
    pub posten: Vec<Posten>,
}

impl Pruefung {
    /// Ob diese Kombination ausgeliefert werden darf: für jeden ihrer Erfolge
    /// eine gefundene Lösung.
    #[must_use]
    pub fn auslieferbar(&self) -> bool {
        is_shippable(&self.erfolge)
    }

    /// Wanduhr über alle Teile.
    #[must_use]
    pub fn wanduhr(&self) -> Duration {
        self.posten.iter().map(|p| p.wanduhr).sum()
    }

    /// Kämpfe über alle Teile.
    #[must_use]
    pub fn kaempfe(&self) -> u64 {
        self.posten.iter().map(|p| p.kaempfe).sum()
    }

    /// Die erschöpfenden Projektionen des Teils [`Teil::Befunde`] als Stapel.
    ///
    /// **Für die Auslöser in #891 Abschnitt 4**: Eine Projektion ist ein
    /// homogener Stapel — dieselbe Liste von Zeilen, jede Reihenfolge jeder
    /// Teilmenge, derselbe Boss. Ob ein Stapel groß genug für einen anderen
    /// Rechenweg ist, sagt seine Größe, nicht die Summe.
    #[must_use]
    pub fn stapel(&self) -> Stapel {
        let groessen: Vec<u64> = self
            .tabelle
            .zeilen
            .iter()
            .filter_map(|z| z.befund.projektion())
            .map(|p| p.listen)
            .collect();
        Stapel {
            projektionen: groessen.len(),
            listen: groessen.iter().sum(),
            groesste: groessen.iter().copied().max().unwrap_or(0),
        }
    }
}

/// Wie viele erschöpfende Projektionen eine Kombination gerechnet hat, und wie
/// groß sie waren.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stapel {
    /// Wie viele Projektionen.
    pub projektionen: usize,
    /// Listen über alle zusammen.
    pub listen: u64,
    /// Listen der größten.
    pub groesste: u64,
}

/// Die Lösung, die der Prüflauf einer Kombination zugrunde legt.
///
/// **Die gesuchte Rotation**, gierig gegen genau diesen Boss. Sie ist zugleich
/// die Elite der Zauber-Vergleichstabelle: Ausgabe 2 sagt, was die Liste wirkt,
/// deren Sieg Ausgabe 3 und 4 prüfen.
#[must_use]
pub fn loesung_suchen(config: &BalanceConfig, rahmen: &Rahmen) -> SearchResult {
    greedy(config, &rahmen.pool, rahmen.max_len)
}

/// Rechnet eine Kombination: Ausgaben 2 bis 4, je Teil gemessen.
///
/// # Panics
///
/// Wenn der Kampf würfelt — dann trägt keine Messung aus einem Durchgang, und
/// die Machbarkeit aus einem Kampf wäre eine Stichprobe von eins. Ebenso, wenn
/// die Elite im Einzelkampf anders wirkt als im Lauf: Dann gehörte die
/// Schadensspalte zu einem anderen Kampf.
#[must_use]
pub fn pruefe(kombination: Kombination, rahmen: &Rahmen) -> Pruefung {
    let gegner = kombination.gegner();
    let config = messkonfiguration(gegner);
    assert!(
        crate::durchgaenge::ein_durchgang_genuegt(EFFECT_NODES, &config.worn),
        "der Pruefllauf rechnet je Liste einen Kampf — das traegt nur, wenn nichts wuerfelt"
    );

    let (elite, suche) = messen(Teil::Suche, || loesung_suchen(&config, rahmen));

    let ((tabelle, elite_lauf), befunde) = messen(Teil::Befunde, || {
        let tabelle = befunde_zur_elite(
            &config,
            elite,
            &rahmen.grundmenge,
            rahmen.max_len,
            rahmen.budget,
        );
        // Schaden je Zauber gibt der Balance-Lauf nicht her, nur der einzelne
        // Kampf. Er wird mitgezählt wie jeder andere.
        let lauf = Fight::new(FightConfig {
            stop_on_kill: config.stop_on_kill,
            boss: gegner,
            log_capacity: 0,
            ..FightConfig::default()
        })
        .run_apl(&tabelle.elite.best.apl);
        if let Some(strang) = crate::fortschritt::strang() {
            strang.kampf();
        }
        (tabelle, lauf)
    });
    // Derselbe Kampf wie im Lauf — und das wird geprüft, nicht angenommen.
    for zeile in &tabelle.zeilen {
        assert_eq!(
            elite_lauf.casts[zeile.spell], zeile.casts,
            "{} wird an {} im Einzelkampf {}-mal gewirkt, im Lauf {}-mal — die Schadensspalte \
             gehoerte dann zu einem anderen Kampf",
            zeile.id, gegner.id, elite_lauf.casts[zeile.spell], zeile.casts
        );
    }

    let (schulen, schulposten) = messen(Teil::Schulproben, || {
        schulproben(&config, &rahmen.pool, &rahmen.grundmenge, rahmen.max_len)
    });
    let (mechaniken, mechanikposten) = messen(Teil::Mechanikproben, || {
        mechanikproben(&config, &rahmen.pool, rahmen.max_len)
    });
    let ((machbarkeit, erfolgsliste), machbarkeitsposten) = messen(Teil::Machbarkeit, || {
        let machbarkeit = feasibility_gegen(gegner, &tabelle.elite.best.apl, None);
        let liste = erfolge_pruefen(&erfolge(gegner), &machbarkeit);
        (machbarkeit, liste)
    });

    Pruefung {
        kombination,
        tabelle,
        elite_lauf,
        schulen,
        mechaniken,
        machbarkeit,
        erfolge: erfolgsliste,
        posten: vec![
            suche,
            befunde,
            schulposten,
            mechanikposten,
            machbarkeitsposten,
        ],
    }
}

/// Ob der Lauf ausgeliefert werden darf: jede Kombination auslieferbar.
///
/// **Ein leerer Lauf gibt nichts frei** — `all()` auf einer leeren Liste wäre
/// wahr, und ein Prüfplan, der versehentlich leer ist, liefe grün durch.
#[must_use]
pub fn alle_auslieferbar(pruefungen: &[Pruefung]) -> bool {
    !pruefungen.is_empty() && pruefungen.iter().all(Pruefung::auslieferbar)
}

/// Die Kombinationen, die nicht ausgeliefert werden dürfen.
#[must_use]
pub fn nicht_auslieferbar(pruefungen: &[Pruefung]) -> Vec<&Pruefung> {
    pruefungen.iter().filter(|p| !p.auslieferbar()).collect()
}

/// Summe je Teil über alle Kombinationen, in der Reihenfolge von
/// [`Teil::ALLE`].
#[must_use]
pub fn summe_je_teil(pruefungen: &[Pruefung]) -> Vec<Posten> {
    Teil::ALLE
        .iter()
        .map(|&teil| {
            pruefungen
                .iter()
                .flat_map(|p| p.posten.iter())
                .filter(|p| p.teil == teil)
                .fold(
                    Posten {
                        teil,
                        wanduhr: Duration::ZERO,
                        kaempfe: 0,
                        durchgaenge: 0,
                        auswertungen: 0,
                    },
                    |summe, p| Posten {
                        teil,
                        wanduhr: summe.wanduhr + p.wanduhr,
                        kaempfe: summe.kaempfe + p.kaempfe,
                        durchgaenge: summe.durchgaenge + p.durchgaenge,
                        auswertungen: summe.auswertungen + p.auswertungen,
                    },
                )
        })
        .collect()
}
