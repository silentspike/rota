//! Belastbare Messumgebung — misst, und bricht ab, wenn sie es nicht kann (#896)
//!
//! ```text
//! cargo remote -c --manifest-path sim/Cargo.toml -- \
//!   run --release --features diagnostics --example messumgebung
//! ```
//!
//! # Warum es diesen Laeufer gibt
//!
//! Der Durchsatz des Simulators streute um **12 Prozent**, weil die Maschine
//! nie ruhig ist. Ein Umbau, der 10 Prozent bringt, waere darin nicht
//! nachweisbar — und einer, der 10 Prozent **kostet**, fiele nicht auf.
//!
//! Zwei Messfehler sind in diesem Projekt schon passiert und haben je einen
//! halben Tag gekostet: zu kurze Laeufe, in denen der Aufwaermvorgang
//! dominierte, und gleichzeitig laufende Messungen, die sich gegenseitig
//! ausbremsten. Beides faengt dieser Laeufer ab, statt es hinterher zu
//! erklaeren.
//!
//! # Die Haltung dahinter
//!
//! **Lieber kein Ergebnis als ein unbelastbares.** Ein Abbruch mit der
//! Begruendung „PSI cpu avg10 stand bei 14,2" kostet fuenf Minuten. Eine Zahl,
//! die falsch ist und wie eine Messung aussieht, kostet einen Tag — und sie
//! kostet ihn spaeter, wenn niemand mehr weiss, unter welchen Umstaenden sie
//! entstanden ist.
//!
//! Deshalb steht am Anfang jeder Zahl eine **Zusicherung**: Wie viele Kaempfe,
//! wie viele Stichproben, wie viele Zustaende sind eingegangen? Eine Null dort
//! heisst „nicht gemessen" und wird als BLOCKED gemeldet, nie als Ergebnis.
//!
//! # Was er NICHT tut
//!
//! Er optimiert nichts und sagt nichts ueber erreichbare Beschleunigung. Er
//! stellt fest, wo der Bestand heute steht — und ob die Maschine ueberhaupt
//! eine Aussage hergibt.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, Boss, SPELLS};
use rota_sim::diagnostics;
use rota_sim::rotation_search::{all_candidates, greedy, Candidate, SearchResult};

// ---------------------------------------------------------------------------
// Die Schwellen, an denen abgebrochen wird
// ---------------------------------------------------------------------------

/// Mindestdauer eines Messabschnitts.
///
/// Das Issue verlangt mindestens 20 Sekunden statt der bisherigen 1 bis 4.
/// Der Grund steht in seiner Vorgeschichte: 5000 Kaempfe je Strang sind etwa
/// 0,1 Sekunden, und darin dominiert der Aufwaermvorgang — der erste
/// Durchsatzwert des Projekts lag deshalb um 30 Prozent daneben.
const MINDESTDAUER: Duration = Duration::from_secs(20);

/// Die tatsaechliche Messdauer, aus `ROTA_MESSDAUER_S` oder [`MINDESTDAUER`].
///
/// # Warum es diesen Schalter gibt — und warum er nichts ueberspringt
///
/// Ein Funktionstest („laeuft der Bericht ueberhaupt durch?") braucht keine
/// zwanzig Sekunden je Lastfall. Er braucht aber **alle acht Lastfaelle**, alle
/// Abschnitte und alle Zusicherungen — sonst prueft er nicht den Laeufer,
/// sondern eine Teilmenge davon. Deshalb verkuerzt dieser Schalter die Dauer
/// und nichts sonst; es gibt keinen Weg, einen Lastfall auszulassen.
///
/// **Unter [`MINDESTDAUER`] traegt jeder Bericht einen Vermerk**, dass seine
/// Zahlen keine Messung sind. Ohne den saehe ein Kurzlauf genauso aus wie ein
/// echter — und das ist genau die Verwechslung, die diesem Projekt schon
/// zweimal einen halben Tag gekostet hat.
fn messdauer() -> Duration {
    std::env::var("ROTA_MESSDAUER_S")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|s| *s > 0)
        .map_or(MINDESTDAUER, Duration::from_secs)
}

/// Wie lange vor der Messung warmgelaufen wird, ohne dass es zaehlt.
const AUFWAERMDAUER: Duration = Duration::from_secs(2);

// Die beiden PSI-Schwellen stehen seit Issue #912 in `rota_sim::hostcaps` —
// mitsamt ihrer Begruendung. Sie werden oben eingebunden.

/// Wie stark die Rate zwischen den Abschnitten eines Laufs schwanken darf.
///
/// Das ist die Selbstpruefung: Wenn die Maschine waehrend der Messung unruhig
/// **wird**, sagt kein Vorab-Wert etwas darueber. Nur die gemessene Rate
/// selbst zeigt es.
const RATENSTREUUNG_MAX: f64 = 0.05;

/// In wie viele Abschnitte ein Messlauf zerfaellt.
///
/// Mindestens drei, sonst gibt es keine Streuung zu pruefen; mehr als fuenf
/// machen die Abschnitte zu kurz, um den Aufwaermvorgang draussen zu halten.
const ABSCHNITTE: usize = 4;

/// Wie viele Teilbloecke der Median einer A/B-Rate zusammenfasst.
///
/// # Warum das nicht dasselbe ist wie [`ABSCHNITTE`]
///
/// [`ABSCHNITTE`] teilt einen Lastfall-Messlauf, um zu sehen, ob die Rate
/// waehrend der Messung WEGLAEUFT — dort ist die Spannweite die Aussage.
/// Hier geht es um die Gegenrichtung: Der Median soll die Rate moeglichst
/// genau treffen, und dafuer sind mehr Stichproben besser.
///
/// **Das ist keine Absenkung des Kriteriums.** Der Median ueber acht
/// Teilbloecke schaetzt dieselbe Groesse wie der ueber vier, nur genauer; die
/// Gesamtdauer bleibt gleich. Was sich aendert, ist die Streuung der
/// SCHAETZUNG — nicht die der Maschine.
///
/// Gemessen am 2026-08-18: mit vier Teilbloecken 3,57 Prozent bei 20 s und
/// 4,30 Prozent bei 40 s. Laengere Bloecke halfen also nicht, was gegen
/// Messrauschen und fuer die Maschine als Ursache spricht (siehe die
/// Lastmessung des Orchestrators: vier Claude-Prozesse, `kswapd0` dauerhaft
/// aktiv, 9 von 14 GB belegt).
fn ab_teilbloecke() -> usize {
    std::env::var("ROTA_AB_TEILE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n >= 2)
        .unwrap_or(ABSCHNITTE)
}

// ---------------------------------------------------------------------------
// Die acht Lasten
// ---------------------------------------------------------------------------

/// Woraus der Kandidatenpool eines Lastfalls besteht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pool {
    /// Zauber ohne Brand, Verstaerkung, Kanal, Heilung, Schild, Unterbrechung.
    NurSchaden,
    /// Verstaerkungen, dazu die reinen Schadenszauber als Traeger.
    Verstaerkungen,
    /// Brand-Effekte, dazu die reinen Schadenszauber als Traeger.
    Brand,
    /// Kanalisierungen, dazu die reinen Schadenszauber als Traeger.
    Kanal,
    /// Alles, was es gibt.
    Voll,
    /// Unterbrechung, Heilung und Schild, dazu die reinen Schadenszauber.
    Verteidigung,
}

/// Wie gemessen wird.
///
/// # Warum die sechs Lastarten NICHT ueber eine Suche laufen
///
/// Weil eine Suche die Last nicht erzeugt, sondern wegoptimiert. Gemessen am
/// 2026-08-18, dritter Bericht: Der Lastfall „verstaerkungslastig" fand die
/// Rotation
///
/// ```text
///   kindle [unbedingt] > slag [unbedingt] > flashpoint [unbedingt]
/// ```
///
/// Beide Verstaerkungen standen drin, gewirkt wurde **keine**. Eine
/// Prioritaetenliste nimmt die erste zutreffende Regel, und `kindle` trifft
/// immer zu — die Suche hatte es vor die Vorgabe gesetzt. Der Trace-Hash war
/// Bit fuer Bit der von „nur Schaden".
///
/// Drei Anlaeufe, drei Stufen desselben Irrtums: erst kein Pool-Filter, dann
/// eine Vorgabe ohne feste Position, dann eine Vorgabe mit Bedingungen, die
/// nie zutrafen. **Erst die gedruckte Rotation hat es gezeigt** — das
/// Zeilenprofil allein sagte nur „0 Verstaerkung" und nicht, warum.
///
/// Eine Lastart ist deshalb eine **feste Rotation**: Pflichtzauber vorn,
/// Fuellzauber hinten, keine Suche dazwischen. Was gewirkt wird, steht damit
/// fest, bevor der erste Kampf laeuft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verfahren {
    /// Feste Rotation, viele Kaempfe — misst den rohen Kernel.
    Kampf,
    /// Ein vollstaendiger Greedy-Lauf ueber den Pool.
    Greedy,
    /// Ein Lauf mit Wiederaufbau — siehe [`lns`].
    Lns,
}

/// Ein Lastfall.
struct Lastfall {
    /// Wie er im Bericht heisst.
    name: &'static str,
    /// Gegen welchen Boss.
    boss: &'static str,
    /// Woraus der Pool besteht.
    pool: Pool,
    /// Wie gesucht wird.
    verfahren: Verfahren,
    /// Obergrenze der Rotationslaenge.
    max_len: usize,
}

/// Die acht Lasten aus AC3.
///
/// **Sie sind nicht nur verschieden benannt, sondern verschieden belegt.** Der
/// Bericht druckt je Fall, wie viele Brand-, Verstaerkungs- und Kanalzeilen
/// die Suche tatsaechlich gewaehlt hat, und vergleicht am Ende alle acht
/// Trace-Hashes gegeneinander. Zwei Lastfaelle mit demselben Hash waeren
/// derselbe Lastfall unter zwei Namen — und der Bericht sagte es, statt
/// acht Zeilen zu drucken, von denen zwei dasselbe messen.
const LASTEN: &[Lastfall] = &[
    Lastfall {
        name: "nur Schaden",
        boss: "boss-1-1",
        pool: Pool::NurSchaden,
        verfahren: Verfahren::Kampf,
        max_len: 6,
    },
    Lastfall {
        name: "verstaerkungslastig",
        boss: "boss-1-1",
        pool: Pool::Verstaerkungen,
        verfahren: Verfahren::Kampf,
        max_len: 6,
    },
    Lastfall {
        name: "brandlastig",
        boss: "boss-1-1",
        pool: Pool::Brand,
        verfahren: Verfahren::Kampf,
        max_len: 6,
    },
    Lastfall {
        name: "kanallastig",
        boss: "boss-1-1",
        pool: Pool::Kanal,
        verfahren: Verfahren::Kampf,
        max_len: 6,
    },
    Lastfall {
        name: "Boss mit HP-Phase",
        boss: "test-phase",
        pool: Pool::Voll,
        verfahren: Verfahren::Kampf,
        max_len: 5,
    },
    Lastfall {
        name: "Ueberleben und Unterbrechen",
        boss: "test-caster",
        pool: Pool::Verteidigung,
        verfahren: Verfahren::Kampf,
        max_len: 6,
    },
    Lastfall {
        name: "vollstaendiger Greedy-Lauf",
        boss: "boss-1-1",
        pool: Pool::Voll,
        verfahren: Verfahren::Greedy,
        max_len: 6,
    },
    Lastfall {
        name: "vollstaendiger LNS-Lauf",
        boss: "boss-1-1",
        pool: Pool::Voll,
        verfahren: Verfahren::Lns,
        max_len: 6,
    },
];

/// Ob ein Zauber reiner Schaden ist — kein Brand, keine Verstaerkung, kein
/// Kanal, keine Heilung, kein Schild, keine Unterbrechung.
fn ist_reiner_schaden(spell: usize) -> bool {
    let s = &SPELLS[spell];
    s.dot.is_none()
        && s.buff.is_none()
        && s.channel_ms == 0
        && s.heal.is_none()
        && s.shield.is_none()
        && !s.interrupt
        && s.base_damage > 0
}

/// Waehlt die Kandidatenzeilen eines Lastfalls aus.
///
/// **Die Traeger kommen immer mit.** Ein Pool aus lauter Brand-Effekten ohne
/// einen einzigen Schadenszauber waere kein „brandlastiger" Kampf, sondern
/// ein kaputter: Der Boss faellt nie, die Suche laeuft in die Laengengrenze,
/// und gemessen waere die Grenze und nicht die Last.
fn pool_fuer(art: Pool, alle: &[Candidate]) -> Vec<Candidate> {
    alle.iter()
        .filter(|k| {
            let s = &SPELLS[k.spell];
            match art {
                Pool::Voll => true,
                Pool::NurSchaden => ist_reiner_schaden(k.spell),
                Pool::Verstaerkungen => s.buff.is_some() || ist_reiner_schaden(k.spell),
                Pool::Brand => s.dot.is_some() || ist_reiner_schaden(k.spell),
                Pool::Kanal => s.channel_ms > 0 || ist_reiner_schaden(k.spell),
                Pool::Verteidigung => {
                    s.interrupt
                        || s.heal.is_some()
                        || s.shield.is_some()
                        || ist_reiner_schaden(k.spell)
                }
            }
        })
        .cloned()
        .collect()
}

/// Wie viele Zeilen einer Art ein Lastfall mindestens tragen muss.
const PFLICHTZEILEN: usize = 2;

/// Die Zeilen, mit denen ein Lastfall zwingend anfaengt.
///
/// # Warum eine Suche die Last nicht von selbst erzeugt
///
/// Gemessen am 2026-08-18, erster durchgelaufener Bericht: Der Lastfall
/// „verstaerkungslastig" waehlte **null** Verstaerkungszeilen — die Suche
/// nahm eine reine Schadenszeile, weil die mehr bringt. Sein Trace-Hash war
/// Bit fuer Bit derselbe wie der von „nur Schaden": zwei Lastfaelle, ein
/// Lastfall.
///
/// Aufgefallen ist es nur, weil der Bericht die Hashes gegeneinander prueft
/// und das Zeilenprofil mitdruckt. Ohne beides haetten dort acht Zeilen
/// gestanden, von denen zwei dasselbe messen — und niemand haette es gesehen.
///
/// **Ein Lastfall ist deshalb eine Vorgabe, keine Hoffnung.** Was hier
/// zurueckkommt, steht in der Rotation, bevor die Suche ueberhaupt anfaengt.
/// Die Suche darf sie ergaenzen, aber nicht ersetzen (`greedy_from`).
///
/// Was das kostet, benannt: Die gefundene Rotation ist damit **nicht** die
/// beste, die es gibt, sondern die beste mit dieser Last. Fuer eine
/// Durchsatzmessung ist das richtig; wer Balance misst, nimmt `balance.rs`.
fn pflicht_fuer(art: Pool, alle: &[Candidate]) -> Vec<Candidate> {
    let passt = |k: &Candidate| -> bool {
        let s = &SPELLS[k.spell];
        match art {
            Pool::Voll | Pool::NurSchaden => false,
            Pool::Verstaerkungen => s.buff.is_some(),
            Pool::Brand => s.dot.is_some(),
            Pool::Kanal => s.channel_ms > 0,
            Pool::Verteidigung => s.interrupt || s.heal.is_some() || s.shield.is_some(),
        }
    };

    // Je Zauber hoechstens eine Zeile: Zwei Bedingungen desselben Zaubers
    // waeren zwar zwei Zeilen, aber dieselbe Wirkung — und die Last soll aus
    // verschiedenen Zaubern kommen.
    //
    // **Nur unbedingte Zeilen (`Origin::Always`).** Gemessen am 2026-08-18,
    // zweiter Bericht: Mit beliebigen Zeilen standen zwar zwei Verstaerkungen
    // in der Rotation, aber der Trace-Hash blieb Bit fuer Bit der von „nur
    // Schaden" — die Bedingungen trafen nie zu, also wurde nie gewirkt.
    //
    // **Eine Zeile in der Rotation ist keine Last.** Erst der gewirkte Zauber
    // ist eine. Das ist derselbe Unterschied wie zwischen einem Prueffall, der
    // laeuft, und einem, der etwas prueft.
    // **Braende brauchen „nachlegen", nicht „unbedingt".** Der teuerste Befund
    // dieses Issues, gemessen am 2026-08-18:
    //
    // Mit `smoulder [unbedingt]` vorn wird der Brand bei JEDEM GCD (1500 ms)
    // neu angewandt. Er tickt aber erst nach 2000 ms, und `refreshMode:
    // "reset"` setzt ihn dabei zurueck — es faellt **nie ein Tick**. Der
    // Lastfall meldete 30 gewirkte Brandzauber und machte
    // **Schaden 0**.
    //
    // Aufgefallen ist es erst beim Nachweis zu AC4: Ein hundertfacher
    // Tickschaden (40 auf 4000) bewegte den Trace-Hash nicht. Der Hash war
    // richtig — es gab nichts zu bewegen. Drei Fehlspuren vorher: der Bau, der
    // Ruecktransfer, die Zeitstempel des Bauservers. Keine davon war es.
    //
    // Die Lehre steht schon in `Origin::DotRefresh`: Fuer einen Brand ist
    // „nachlegen, bevor er auslaeuft" die sinnvolle Zeile. Sie war die ganze
    // Zeit da.
    let bevorzugt = if matches!(art, Pool::Brand) {
        rota_sim::rotation_search::Origin::DotRefresh
    } else {
        rota_sim::rotation_search::Origin::Always
    };

    let mut gewaehlt: Vec<Candidate> = Vec::new();
    for k in alle.iter().filter(|k| passt(k) && k.origin == bevorzugt) {
        if gewaehlt.iter().any(|v: &Candidate| v.spell == k.spell) {
            continue;
        }
        gewaehlt.push(k.clone());
        if gewaehlt.len() == PFLICHTZEILEN {
            break;
        }
    }
    gewaehlt
}

/// Wie viele der gewaehlten Zeilen zu welcher Art gehoeren.
///
/// Das ist der **Beleg**, dass ein Lastfall seine Last erzeugt. „brandlastig"
/// als Name kostet nichts; drei gewaehlte Brand-Zeilen sind eine Messung.
struct Profil {
    /// Zeilen mit Brand-Effekt.
    brand: usize,
    /// Zeilen mit Verstaerkung.
    verstaerkung: usize,
    /// Zeilen mit Kanalisierung.
    kanal: usize,
    /// Zeilen mit Verteidigungswirkung.
    verteidigung: usize,
    /// Zeilen insgesamt.
    gesamt: usize,
}

impl Profil {
    /// Aus den tatsaechlichen WIRKUNGEN eines Kampfes, nicht aus den Zeilen.
    ///
    /// # Warum nicht aus `SearchResult::chosen`
    ///
    /// Weil eine Zeile in der Rotation noch nichts tut. Gemessen am
    /// 2026-08-18: Zwei Verstaerkungszeilen standen in der Rotation, ihre
    /// Bedingungen trafen aber nie zu — der Kampfverlauf war Bit fuer Bit
    /// derselbe wie ohne sie. Das Zeilenprofil meldete „2 Verstaerkung", die
    /// Last war null.
    ///
    /// `RunResult::casts` zaehlt, was gewirkt wurde. Das laesst sich nicht
    /// dadurch erfuellen, dass etwas dasteht.
    fn von_wirkungen(casts: &[u32]) -> Self {
        let mut p = Self {
            brand: 0,
            verstaerkung: 0,
            kanal: 0,
            verteidigung: 0,
            gesamt: 0,
        };
        for (index, &anzahl) in casts.iter().enumerate() {
            if anzahl == 0 {
                continue;
            }
            let n = anzahl as usize;
            let s = &SPELLS[index];
            p.gesamt += n;
            if s.dot.is_some() {
                p.brand += n;
            }
            if s.buff.is_some() {
                p.verstaerkung += n;
            }
            if s.channel_ms > 0 {
                p.kanal += n;
            }
            if s.interrupt || s.heal.is_some() || s.shield.is_some() {
                p.verteidigung += n;
            }
        }
        p
    }
}

// ---------------------------------------------------------------------------
// LNS — aus einer gefundenen Liste einen Teil herausnehmen und neu fuellen
// ---------------------------------------------------------------------------

/// Wie viele Durchgaenge ein Kampf je Messung bekommt.
///
/// # Warum weniger als die Vorgabe von `BalanceConfig`
///
/// Gemessen am 2026-08-18 mit der Vorgabe: Ein vollstaendiger Greedy-Lauf
/// bewertete 3192 Rotationen und rechnete **3 194 000 Kaempfe** — 85 Sekunden
/// einstraengig, rund 27 Sekunden ueber acht Straenge. Vier Abschnitte davon
/// sind knapp zwei Minuten, und LNS faehrt mehrere solche Laeufe.
///
/// **Was hier gemessen wird, ist die Messumgebung — nicht die Balance.** Die
/// Zahl der Durchgaenge bestimmt, wie genau eine Rotation bewertet wird; die
/// ART der Last aendert sie nicht. Ein brandlastiger Kampf bleibt brandlastig,
/// ob er 200 oder 1000 Mal gerechnet wird.
///
/// **Was das kostet, und was nicht:** Mit weniger Durchgaengen waechst der
/// Standardfehler, und `clearly_better_than` wird strenger — die Suche hoert
/// frueher auf. Der Bericht druckt deshalb je Lastfall, wie viele Rotationen
/// bewertet wurden. Eine Zahl, die zwischen zwei Staenden springt, waere ein
/// Hinweis auf genau diesen Effekt und nicht auf den Kernel.
///
/// Wer Balance misst, nimmt `balance.rs` oder `rotation-search.rs`. Dieser
/// Laeufer misst Durchsatz.
const DURCHGAENGE: u32 = 200;

/// Wie viele Runden LNS faehrt.
const LNS_RUNDEN: usize = 3;
/// Wie viele Zeilen LNS je Runde herausnimmt.
const LNS_ZERSTOERUNG: usize = 2;

/// Ein Lauf mit Wiederaufbau.
///
/// # Warum hier und nicht aus der Bibliothek
///
/// LNS steht heute als **private** Funktion in `examples/suchguete-erweitert.rs`
/// (dort `fn lns`, Issue #882). Aus einem anderen Beispiel heraus ist sie nicht
/// erreichbar — Beispiele sind je ein eigener Kasten.
///
/// Diese Fassung ist deshalb bewusst die **kleinstmoegliche**: Sie faehrt
/// weniger Runden und traegt keine Guetebilanz. Sie soll eine LAST erzeugen,
/// die sich von einem Greedy-Lauf unterscheidet — nicht eine zweite Wahrheit
/// darueber sein, wie gut LNS sucht. Wer Guete misst, nimmt `suchguete-erweitert`.
///
/// Der saubere Weg waere, LNS in `rotation_search` zu heben. Das ist ein
/// Eingriff in fremde, gerade erst eingegangene Arbeit und liegt beim
/// Orchestrator, nicht bei diesem Issue.
fn lns(config: &BalanceConfig, pool: &[Candidate], max_len: usize) -> SearchResult {
    let mut bestes = greedy(config, pool, max_len);
    // **Die Arbeit ALLER Runden, nicht nur der uebernommenen.**
    //
    // `SearchResult::measured` zaehlt, was EIN Greedy-Lauf bewertet hat.
    // Uebernimmt LNS eine Runde nicht, faellt deren Arbeit sonst unter den
    // Tisch — und der Bericht meldet fuer LNS dieselbe Zahl wie fuer Greedy.
    //
    // Gemessen am 2026-08-18: genau so eingetreten, beide 2258. Die Zusicherung
    // ueber die zwei Suchlaeufe schlug an und meldete „dann laeuft das zweite
    // Verfahren nicht" — was nicht stimmte: `time_to_best` lag bei 13,9 s
    // gegen 3,9 s, LNS lief also sehr wohl. Falsch war die ZAHL, nicht der
    // Lauf. Ein Nenner, der Arbeit verschweigt, macht jede Rate darueber
    // wertlos.
    let mut bewertet = bestes.measured;
    for runde in 0..LNS_RUNDEN {
        if bestes.chosen.len() <= LNS_ZERSTOERUNG {
            break;
        }
        // Welche Zeilen herausfallen, entsteht aus der Rundennummer — kein
        // Zufallsgenerator, weil der Simulator deterministisch bleiben muss
        // (GAME-DESIGN 18.4).
        let ab = (runde * LNS_ZERSTOERUNG) % bestes.chosen.len();
        let rest: Vec<Candidate> = bestes
            .chosen
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let im_fenster = *i >= ab && *i < ab + LNS_ZERSTOERUNG;
                let umlauf = ab + LNS_ZERSTOERUNG > bestes.chosen.len()
                    && *i < (ab + LNS_ZERSTOERUNG) % bestes.chosen.len();
                !(im_fenster || umlauf)
            })
            .map(|(_, k)| k.clone())
            .collect();

        let mut neu_pool = rest.clone();
        for k in pool {
            if !neu_pool
                .iter()
                .any(|v| v.spell == k.spell && v.condition == k.condition)
            {
                neu_pool.push(k.clone());
            }
        }
        let versuch = greedy(config, &neu_pool, max_len);
        bewertet += versuch.measured;
        if versuch.best.clearly_better_than(&bestes.best) {
            bestes = versuch;
        }
    }
    SearchResult {
        measured: bewertet,
        ..bestes
    }
}

// ---------------------------------------------------------------------------
// Systemdruck — gemessen, nicht geraten
// ---------------------------------------------------------------------------

/// Die Umgebungsmessung steht seit Issue #912 in der Bibliothek.
///
/// **Sie stand bis dahin hier, privat.** Ein Beispiel kann nichts exportieren;
/// die Faehigkeitserhebung haette dieselben Funktionen ein zweites Mal gebaut —
/// und ein Nachbau antwortet plausibel aus einer zweiten Quelle. Laufen beide
/// auseinander, weiss niemand, welche recht hat.
///
/// Beim Heben ist genau das einmal beinahe passiert: `PSI_IO_MEM_MAX` stand in
/// der neuen Fassung auf 20 statt auf den 10 von hier. Der Kommentar dort haelt
/// es fest.
use rota_sim::hostcaps::{
    auslagerung, drucksignale, erlaubte_kerne, kerne_gesamt, kontextwechsel, psi_some_avg10,
    takt_mhz, taktdrosselung, PSI_CPU_MAX,
};

// ---------------------------------------------------------------------------
// Die sechs Metriken
// ---------------------------------------------------------------------------

/// Ein Messwert, der auch „nicht messbar" sein kann.
///
/// **Der Unterschied zwischen 0,00 und „es gibt das gar nicht" ist der Kern
/// dieses Issues.** Eine Trefferquote ohne Cache ist keine Null — eine Null
/// hiesse, der Cache trifft nie. Wer sie druckt, erzeugt einen Befund, wo
/// keine Messung war.
enum Wert {
    /// Gemessen.
    Zahl(f64),
    /// Nicht messbar, mit Grund.
    Fehlt(&'static str),
}

impl std::fmt::Display for Wert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Zahl(x) if *x >= 1000.0 => write!(f, "{x:>14.0}"),
            Self::Zahl(x) => write!(f, "{x:>14.4}"),
            Self::Fehlt(grund) => write!(f, "{:>14}", format!("n/v ({grund})")),
        }
    }
}

/// Die sechs Metriken eines Lastfalls.
struct Metriken {
    /// Roher Kernel: vollstaendig gerechnete Kaempfe je Sekunde.
    full_fights: Wert,
    /// Bewertete Kandidatenzeilen je Sekunde.
    resolved_candidates: Wert,
    /// Ob die Kanonisierung traegt.
    cache_hit_rate: Wert,
    /// Welcher Anteil ueberhaupt noch teuer ist.
    full_fight_ratio: Wert,
    /// Suchqualitaet gegen Wanduhr, in Sekunden.
    time_to_best: Wert,
    /// Das eigentliche Balance-Ziel.
    verified_elites: Wert,
}

// ---------------------------------------------------------------------------
// Der Messlauf
// ---------------------------------------------------------------------------

/// Was ein Lastfall hergegeben hat.
struct Ergebnis {
    /// Name des Lastfalls.
    name: &'static str,
    /// Die sechs Metriken.
    metriken: Metriken,
    /// Der Trace-Hash des Bosses.
    trace: u32,
    /// Wie viele Zustaende in den Hash eingegangen sind.
    trace_states: u64,
    /// Womit die Suche aufgehoert hat.
    profil: Profil,
    /// Ob dieser Fall eine Lastart ist (feste Rotation) oder ein Suchlauf.
    ist_lastart: bool,
    /// Die gefundene Rotation, Zeile fuer Zeile.
    ///
    /// Ohne sie sagt ein Zeilenprofil von „0 Verstaerkung" nicht, ob die
    /// Vorgabe fehlte, verworfen wurde oder nur nie zum Zuge kam.
    rotation: Vec<String>,
    /// Die Zeilen, mit denen der Lastfall zwingend anfaengt.
    ///
    /// **Die Zusicherung zur Last.** Steht hier eine leere Liste, hat der
    /// Lastfall gar keine Vorgabe bekommen — und was der Bericht dann misst,
    /// ist irgendeine Rotation und nicht die benannte Last.
    pflicht: Vec<String>,
    /// Zahl der bewerteten Rotationen.
    measured: usize,
    /// Groesse des Pools.
    pool_size: usize,
    /// Streuung der Rate zwischen den Abschnitten.
    ratenstreuung: f64,
    /// Die Zaehler aus dem Kalibrierlauf.
    zaehler: diagnostics::Counters,
}

/// Baut die Konfiguration eines Lastfalls.
fn config_fuer(boss: &'static Boss, threads: usize) -> BalanceConfig {
    BalanceConfig {
        boss,
        threads,
        iterations: DURCHGAENGE,
        ..BalanceConfig::default()
    }
}

/// Was ein Durchgang hervorgebracht hat — egal ob Kampf oder Suche.
struct Lauf {
    /// Die Rotation, die gemessen wurde.
    apl: rota_sim::apl::Apl,
    /// Ihre Zeilen.
    zeilen: Vec<Candidate>,
    /// Wie viele Rotationen dafuer bewertet wurden.
    ///
    /// Bei einer Lastart ist das **eine** — es wird nicht gesucht. Die Zahl
    /// steht trotzdem hier, weil `resolved_candidates/s` sie als Zaehler
    /// braucht und eine feste 1 ehrlicher ist als eine weggelassene Metrik.
    measured: usize,
    /// Wie viele Kandidatenzeilen zur Wahl standen.
    pool_size: usize,
}

/// Die feste Rotation einer Lastart.
///
/// **Pflichtzauber vorn, ein Fuellzauber hinten.** Die Reihenfolge ist die
/// ganze Sache: Eine Prioritaetenliste nimmt die erste zutreffende Regel, und
/// ein unbedingter Schadenszauber trifft immer zu. Steht er vorn, kommt
/// dahinter nichts mehr zum Zug — genau daran sind am 2026-08-18 drei Anlaeufe
/// gescheitert.
///
/// Der Fueller ist noetig, damit der Kampf ueberhaupt Schaden macht: Eine
/// Rotation aus zwei Verstaerkungen mit dreissig Sekunden Abklingzeit steht
/// die meiste Zeit still, und gemessen waere das Warten.
fn feste_rotation(fall: &Lastfall, pool: &[Candidate]) -> Vec<Candidate> {
    let mut zeilen = pflicht_fuer(fall.pool, pool);
    let fueller = pool.iter().find(|k| {
        ist_reiner_schaden(k.spell)
            && k.origin == rota_sim::rotation_search::Origin::Always
            && !zeilen.iter().any(|z| z.spell == k.spell)
    });
    if let Some(f) = fueller {
        zeilen.push(f.clone());
    }
    zeilen
}

/// Faehrt einen Lastfall einmal durch.
fn suche(fall: &Lastfall, config: &BalanceConfig, pool: &[Candidate]) -> Lauf {
    match fall.verfahren {
        Verfahren::Kampf => {
            let zeilen = feste_rotation(fall, pool);
            let gemessen = rota_sim::rotation_search::measure(config, fall.name, &zeilen);
            Lauf {
                apl: gemessen.apl,
                zeilen,
                measured: 1,
                pool_size: pool.len(),
            }
        }
        Verfahren::Greedy => {
            let r = greedy(config, pool, fall.max_len);
            Lauf {
                apl: r.best.apl,
                zeilen: r.chosen,
                measured: r.measured,
                pool_size: r.pool_size,
            }
        }
        Verfahren::Lns => {
            let r = lns(config, pool, fall.max_len);
            Lauf {
                apl: r.best.apl,
                zeilen: r.chosen,
                measured: r.measured,
                pool_size: r.pool_size,
            }
        }
    }
}

/// Misst einen Lastfall.
///
/// Der Ablauf ist dreiteilig, und die Reihenfolge ist der Punkt:
///
/// 1. **Kalibrierlauf** mit eingeschalteter Erfassung — er sagt, wie viele
///    Kaempfe, Zustaende und `expire_buffs`-Aufrufe ein Durchgang kostet.
/// 2. **Aufwaermen** ohne Zeitnahme.
/// 3. **Durchsatzlauf** mit AUSgeschalteter Erfassung, in Abschnitten.
///
/// Schritt 1 und 3 messen dieselbe Arbeit: Der Simulator ist deterministisch,
/// dieselbe Konfiguration ergibt dieselben Kaempfe. Deshalb darf die Zahl aus
/// dem Kalibrierlauf als Nenner des Durchsatzes dienen, ohne dass die
/// Erfassung die Zeitnahme verfaelscht.
fn miss(fall: &Lastfall, threads: usize) -> Result<Ergebnis, String> {
    let boss = boss_by_id(fall.boss).ok_or_else(|| format!("Boss {} fehlt", fall.boss))?;
    let config = config_fuer(boss, threads);
    let alle = all_candidates();
    let pool = pool_fuer(fall.pool, &alle);
    if pool.is_empty() {
        return Err(format!("Pool von {} ist leer", fall.name));
    }
    let pflicht: Vec<String> = pflicht_fuer(fall.pool, &pool)
        .iter()
        .map(|k| SPELLS[k.spell].id.to_string())
        .collect();
    if pflicht.is_empty() && !matches!(fall.pool, Pool::Voll | Pool::NurSchaden) {
        return Err(format!(
            "{}: keine unbedingte Kandidatenzeile der geforderten Art im Pool. \
             Ohne Vorgabe waere das kein Lastfall, sondern eine freie Suche unter \
             falschem Namen.",
            fall.name
        ));
    }

    // Fortschritt geht nach stderr, der Bericht nach stdout. Ein Laeufer, der
    // zwanzig Minuten schweigt und dann alles auf einmal druckt, ist von einem
    // haengenden nicht zu unterscheiden — am 2026-08-18 habe ich genau deshalb
    // zwei Laeufe abgebrochen, ohne zu wissen, an welcher Stelle sie standen.
    //
    // Das ist NICHT die Fortschrittsanzeige, die #896 ausschliesst: Die meint
    // eine Anzeige waehrend eines Suchlaufs. Hier steht nur, welcher von acht
    // Lastfaellen gerade laeuft und was er bisher gekostet hat.
    let angefangen = Instant::now();
    eprintln!("  [{}] Kalibrierlauf …", fall.name);

    // 1 — Kalibrierlauf, EINSTRAENGIG. Zwei Gruende, und beide sind gemessen:
    //
    // a) Die Zaehler sind strang-lokal (`thread_local!`, wie die Aufzeichnung
    //    in `crate::checksum`). Schaltet der Hauptstrang sie ein und rechnen
    //    acht Arbeitsstraenge, sieht er NICHTS — der erste Lauf dieses
    //    Laeufers meldete deshalb "0 gezaehlt" bei korrekt gebautem Merkmal.
    // b) Ein Trace-Hash ueber acht Straenge waere von der Reihenfolge
    //    abhaengig, in der das Betriebssystem sie einplant — also nicht
    //    reproduzierbar. Genau das soll er aber pruefen.
    let kalibrier_config = config_fuer(boss, 1);
    diagnostics::reset();
    diagnostics::enable();
    let kalibriert = suche(fall, &kalibrier_config, &pool);
    let zaehler = diagnostics::snapshot();
    diagnostics::disable();
    diagnostics::reset();

    // 1b — Der Trace-Hash, aus EINEM Kampf und nicht aus dem ganzen Suchlauf.
    //
    // AC4 verlangt „je Boss ein Trace-Hash", nicht je Suchlauf. Ein einzelner
    // Kampf mit der gefundenen Rotation genuegt dafuer und ist reproduzierbar.
    //
    // Der Unterschied ist nicht kosmetisch: `checksum_at` mischt fuenf Werte
    // je Zauber und laeuft je Takt. Am ganzen Suchlauf gemessen, lief die
    // Kalibrierung eines vollstaendigen Greedy-Laufs am 2026-08-18 ueber
    // zwoelf Minuten und war nicht fertig — bei 140 Prozent CPU, also
    // einstraengig rechnend.
    let (trace, profil, trace_schaden) = {
        diagnostics::reset();
        diagnostics::enable_trace();
        let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
            boss,
            ..rota_sim::FightConfig::default()
        });
        let ergebnis = fight.run_apl(&kalibriert.apl);
        let z = diagnostics::snapshot();
        diagnostics::disable_trace();
        diagnostics::reset();
        (z, Profil::von_wirkungen(&ergebnis.casts), ergebnis.damage)
    };

    // **Ein Lastfall ohne Schaden ist kein Lastfall.** Gemessen am
    // 2026-08-18: „brandlastig" wirkte 30 Brandzauber und machte 0 Schaden,
    // weil jeder GCD den Brand zurueckstellte, bevor er ticken konnte. Ohne
    // diese Zusicherung stand der Fall vier Berichte lang als gueltig da.
    if trace_schaden == 0 {
        return Err(format!(
            "{}: der Kampf macht 0 Schaden. Die Rotation steht, aber sie \
             richtet nichts aus — gemessen wird dann Leerlauf unter dem Namen \
             einer Last.",
            fall.name
        ));
    }

    if zaehler.fights() == 0 {
        return Err(format!(
            "{}: die Instrumentierung hat nichts gezaehlt. Gebaut ohne \
             --features diagnostics? Ohne sie gibt es keinen Nenner, und ein \
             Durchsatz ohne Nenner ist keine Zahl.",
            fall.name
        ));
    }

    // Die Zusicherung zum Nenner: Der Kalibrierlauf lief mit einem Strang, der
    // Durchsatzlauf mit {threads}. Beide duerfen nur dann dieselbe Kampfzahl
    // haben, wenn sie DIESELBE Arbeit machen — und das ist pruefbar, weil
    // `measured` sagt, wie viele Rotationen bewertet wurden.
    //
    // Ohne diese Probe waere der Nenner eine Annahme ueber die Strangaufteilung,
    // und jede Durchsatzzahl haette einen ungemessenen Faktor darin.
    eprintln!(
        "  [{}] Kalibrierung und Trace nach {:.1} s ({} Kaempfe, {} Rotationen, \
         {} Trace-Zustaende) — Querprobe …",
        fall.name,
        angefangen.elapsed().as_secs_f64(),
        zaehler.fights(),
        kalibriert.measured,
        trace.trace_states()
    );

    let quer = suche(fall, &config, &pool);
    if quer.measured != kalibriert.measured {
        return Err(format!(
            "{}: die Suche bewertet mit {threads} Straengen {} Rotationen, mit \
             einem aber {}. Dann gilt die kalibrierte Kampfzahl nicht als Nenner \
             fuer den Durchsatzlauf.",
            fall.name, quer.measured, kalibriert.measured
        ));
    }

    // 2 — Aufwaermen.
    eprintln!(
        "  [{}] nach {:.1} s — aufwaermen …",
        fall.name,
        angefangen.elapsed().as_secs_f64()
    );
    let warm = Instant::now();
    while warm.elapsed() < AUFWAERMDAUER {
        let _ = suche(fall, &config, &pool);
    }

    // 3 — Durchsatz in Abschnitten.
    let je_abschnitt = messdauer() / u32::try_from(ABSCHNITTE).unwrap_or(1);
    let mut raten = Vec::with_capacity(ABSCHNITTE);
    let mut durchgaenge_gesamt = 0u64;
    let mut zeit_gesamt = Duration::ZERO;
    let mut time_to_best = Duration::ZERO;

    for abschnitt in 0..ABSCHNITTE {
        eprintln!(
            "  [{}] nach {:.1} s — Abschnitt {}/{ABSCHNITTE} …",
            fall.name,
            angefangen.elapsed().as_secs_f64(),
            abschnitt + 1
        );
        let start = Instant::now();
        let mut durchgaenge = 0u64;
        while start.elapsed() < je_abschnitt {
            let vor = Instant::now();
            let _ = suche(fall, &config, &pool);
            time_to_best = vor.elapsed();
            durchgaenge += 1;
        }
        let dauer = start.elapsed();
        #[allow(clippy::cast_precision_loss)]
        raten.push(durchgaenge as f64 / dauer.as_secs_f64());
        durchgaenge_gesamt += durchgaenge;
        zeit_gesamt += dauer;
    }

    let mittel = raten.iter().sum::<f64>() / raten.len() as f64;
    let ratenstreuung = if mittel > 0.0 {
        let max = raten.iter().copied().fold(f64::MIN, f64::max);
        let min = raten.iter().copied().fold(f64::MAX, f64::min);
        (max - min) / mittel
    } else {
        f64::INFINITY
    };

    #[allow(clippy::cast_precision_loss)]
    let sekunden = zeit_gesamt.as_secs_f64();
    #[allow(clippy::cast_precision_loss)]
    let kaempfe = durchgaenge_gesamt as f64 * zaehler.fights() as f64;
    #[allow(clippy::cast_precision_loss)]
    let kandidaten = durchgaenge_gesamt as f64 * kalibriert.measured as f64;

    let metriken = Metriken {
        full_fights: Wert::Zahl(kaempfe / sekunden),
        resolved_candidates: if fall.verfahren == Verfahren::Kampf {
            // Eine Lastart SUCHT nicht. Hier eine Rate zu drucken hiesse,
            // „eine bewertete Rotation je Kampfblock" als Suchleistung
            // auszugeben — eine Zahl, die stimmt und nichts bedeutet.
            Wert::Fehlt("keine Suche")
        } else {
            Wert::Zahl(kandidaten / sekunden)
        },
        // Es gibt im Bestand keinen Cache und keine Kanonisierung — gemessen
        // ueber sim/src, sim/examples und sim/tests: kein Treffer auf "cache",
        // "canonical", "kanonis", "memo". Eine 0,00 hier waere die Aussage
        // "der Cache trifft nie", und die waere falsch.
        cache_hit_rate: Wert::Fehlt("kein Cache"),
        // Jede bewertete Zeile braucht heute einen vollen Kampf. Das ist eine
        // MESSUNG und kein Platzhalter: An dieser 1,0 sieht man spaeter, ob
        // eine Abkuerzung ueberhaupt greift.
        full_fight_ratio: Wert::Zahl(1.0),
        time_to_best: Wert::Zahl(time_to_best.as_secs_f64()),
        // Ein Suchlauf, der durchgelaufen ist, ist ein bestaetigtes Ergebnis:
        // `greedy` misst jede Zeile ueber den skalaren Referenzpfad, es gibt
        // keinen schnellen Pfad, der etwas ueberspringen koennte.
        //
        // **Hochgerechnet wird erst ab zwei Durchgaengen** (Praezisierung des
        // Orchestrators zu meiner Meldung 8060). Aus EINEM Lauf auf eine
        // Stunde zu schliessen ist keine Rate, sondern eine Erfindung: Der
        // einzelne Lauf traegt seinen ganzen Aufwaermvorgang, und ob der
        // zweite genauso lange braucht, ist damit ungemessen.
        verified_elites: if fall.verfahren == Verfahren::Kampf {
            // Eine Lastart bestaetigt keine Rotation — sie rechnet eine feste
            // durch. „Verifizierte Eliten" waeren hier schlicht die Zahl der
            // Kampfbloecke, hochgerechnet: eine grosse Zahl ohne Bedeutung.
            // Der erste Bericht meldete dafuer 4 089 998 je Stunde.
            Wert::Fehlt("keine Suche")
        } else if durchgaenge_gesamt < 2 {
            Wert::Fehlt("nur 1 Lauf")
        } else {
            #[allow(clippy::cast_precision_loss)]
            Wert::Zahl(durchgaenge_gesamt as f64 / sekunden * 3600.0)
        },
    };

    Ok(Ergebnis {
        name: fall.name,
        metriken,
        trace: trace.trace(),
        trace_states: trace.trace_states(),
        profil,
        ist_lastart: fall.verfahren == Verfahren::Kampf,
        rotation: kalibriert
            .zeilen
            .iter()
            .map(|k| format!("{} [{}]", SPELLS[k.spell].id, k.origin.label()))
            .collect(),
        pflicht,
        measured: kalibriert.measured,
        pool_size: kalibriert.pool_size,
        ratenstreuung,
        zaehler,
    })
}

// ---------------------------------------------------------------------------
// Bericht
// ---------------------------------------------------------------------------

/// Ein einzelner Durchsatzwert, ohne den ganzen Bericht drumherum.
///
/// Nimmt die Zeit fuer eine feste Zahl von Durchgaengen — **nicht** die Zahl
/// der Durchgaenge in fester Zeit. Der Unterschied ist der Punkt: Bei fester
/// Zeit haengt die Zahl davon ab, wo der letzte Durchgang gerade stand, und
/// genau diese Randunschaerfe ist bei kurzen Laeufen die halbe Streuung.
/// Eine Rate samt dem Druck, unter dem sie entstanden ist.
struct Blockwert {
    /// Die gemessene Rate.
    rate: f64,
    /// Groesster PSI-cpu-Wert waehrend des Blocks.
    psi_max: f64,
    /// Kontextwechsel je Sekunde waehrend des Blocks.
    ctxt_rate: f64,
    /// Mittlerer Takt waehrend des Blocks, in MHz.
    takt_mhz: f64,
    /// Sekunden seit Laufbeginn, als dieser Block ANFING.
    ///
    /// # Wofuer, wenn der Bericht doch eine Reihenfolge hat
    ///
    /// Um eine fremde Stoerung einem Block zuzuordnen. Am 2026-08-19 wurde
    /// waehrend einer A,A-Reihe neunmal gebaut, und die Frage war, welcher
    /// Block es abbekommen hat. Ohne dieses Feld ging das nur ueber eine
    /// Rueckrechnung: Gesamtdauer durch Blockzahl, dann mal Index.
    ///
    /// **Diese Rechnung ist falsch, und ihr Fehler waechst mit dem Index.**
    /// Sie setzt gleich lange Bloecke voraus; tatsaechlich kommen je Block
    /// [`AUFWAERMDAUER`] dazu, Teilbloecke enden nicht auf die Sekunde, und
    /// verworfene Bloecke stehen gar nicht erst im Bericht. Die Zuordnung war
    /// damals nur an EINER Stelle belegbar — dort, wo zwei unabhaengige
    /// Quellen zufaellig zusammenfielen.
    ///
    /// Gemessen hat dieses Feld den Fehler nicht.
    seit_beginn_s: f64,
}

/// Wann dieser Lauf angefangen hat — einmal gesetzt, danach unveraenderlich.
///
/// Gehalten werden **beide** Zeitrechnungen, und das hat je einen Grund:
///
/// | Wert | wofuer |
/// | --- | --- |
/// | [`Instant`] | die Abstaende je Block — monoton, springt nicht bei einer Zeitumstellung |
/// | Unix-Sekunde | der Bezug nach aussen — eindeutig, ohne Zeitzone |
///
/// # Warum keine Uhrzeit
///
/// Die Standardbibliothek kennt den Zeitzonen-Offset nicht, und das Projekt
/// hat keine Zeit-Abhaengigkeit (kein `chrono`, kein `jiff`; im ganzen Repo
/// keine Verwendung von `SystemTime` ausser dieser). Eine Ausgabe in UTC waere
/// die naheliegende Loesung und die gefaehrlichere: Bau-Logs, IPC-Nachrichten
/// und `ps` sind Lokalzeit, im Sommer zwei Stunden daneben. Ein Bericht, der
/// „18:26" sagt, waehrend das Bau-Log „20:26" sagt, erzeugt genau die
/// Verwechslung, gegen die dieses Feld gebaut ist.
///
/// Die Unix-Sekunde ist unmissverstaendlich, und `date -d @<sekunde>` macht
/// daraus Lokalzeit — einmal je Lauf statt einmal je Block.
static LAUFBEGINN: OnceLock<(Instant, u64)> = OnceLock::new();

/// Sekunden seit dem ersten Aufruf dieser Funktion im Prozess.
fn seit_beginn_s() -> f64 {
    laufbeginn().0.elapsed().as_secs_f64()
}

fn laufbeginn() -> &'static (Instant, u64) {
    LAUFBEGINN.get_or_init(|| {
        let unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        (Instant::now(), unix)
    })
}

fn eine_rate(
    fall: &Lastfall,
    threads: usize,
    durchgaenge: u32,
    erfassung: bool,
) -> Option<Blockwert> {
    // **Vor dem Aufwaermen abgelesen, nicht danach.** Die Zeit soll sagen,
    // wann dieser Block die Maschine zu belasten anfing — eine fremde
    // Stoerung trifft das Aufwaermen genauso wie die Messung.
    let angefangen_s = seit_beginn_s();
    let boss = boss_by_id(fall.boss)?;
    let config = config_fuer(boss, threads);
    let pool = pool_fuer(fall.pool, &all_candidates());

    diagnostics::reset();
    if erfassung {
        diagnostics::enable();
    } else {
        diagnostics::disable();
    }

    // **Ueber die volle Messdauer, in Teilbloecken, und der MEDIAN daraus.**
    //
    // Zwei Gruende, beide gemessen am 2026-08-18:
    //
    // a) Die erste Fassung mass je Block rund zwei Sekunden und meldete
    //    26,5 Prozent Streuung. Das Issue verlangt mindestens zwanzig, und
    //    zwar genau deshalb: In zwei Sekunden dominiert, was die Maschine
    //    gerade sonst noch tut.
    // b) AC1 sagt „streut der MEDIAN um weniger als 3 %" — nicht „die
    //    Spannweite der Einzelwerte". Ein einzelner Block, den ein fremder
    //    Prozess trifft, ist ein Ausreisser; der Median ist genau das
    //    Werkzeug, das ihn nicht mitzaehlt. Der Ausreisser verschwindet damit
    //    nicht aus der Welt, er verschiebt nur nicht mehr das Ergebnis.
    // **Vorlauf, bevor die Uhr laeuft.**
    //
    // Diese Funktion hatte lange keinen — `miss` waermt zwei Sekunden vor, die
    // A/B- und A,A-Reihe gar nicht. Das war ein Mangel, und er ist behoben.
    //
    // **Er erklaert die Streuung NICHT.** Die Vermutung lag nahe und wurde
    // geprueft: In den sechs Reihen vom 2026-08-18 liegen die ersten zwei
    // Paare nur in EINER unter dem Median der uebrigen — und zwar in genau der,
    // an der die Vermutung entstand. In einer anderen wird es ohne die ersten
    // Paare sogar schlechter.
    //
    // Ein struktureller Mangel, der ueberall gilt, kann keinen Unterschied
    // erklaeren, der nur an einer Stelle auftritt. Der Vorlauf steht hier, weil
    // eine Messung mit ihm besser ist als eine ohne — nicht, weil er ein
    // Raetsel loest.
    let warm = Instant::now();
    while warm.elapsed() < AUFWAERMDAUER {
        let _ = suche(fall, &config, &pool);
    }

    let teile = ab_teilbloecke();
    let mut teilraten = Vec::with_capacity(teile);
    let je_teil = messdauer() / u32::try_from(teile).unwrap_or(1);

    // **Der Druck wird WAEHREND der Messung gelesen, nicht nur davor.**
    //
    // Gemessen am 2026-08-18: Eine A/B-Reihe streute um 26 Prozent, obwohl
    // PSI vor dem Lauf bei 0,05 stand. Waehrenddessen lief `zauber-abdeckung`
    // einer fremden Sitzung mit 100 Prozent CPU. Ein Vorab-Wert sagt ueber die
    // naechsten fuenf Minuten nichts — er ist eine Momentaufnahme, und eine
    // Messung ist keine.
    let mut psi_max: f64 = 0.0;
    let mut takte: Vec<f64> = Vec::with_capacity(teile);
    let erlaubt = erlaubte_kerne().unwrap_or_default();
    let ctxt_vor = kontextwechsel();
    let gesamt_start = Instant::now();

    for _ in 0..teile {
        let start = Instant::now();
        let mut gezaehlt = 0u64;
        while start.elapsed() < je_teil {
            for _ in 0..durchgaenge {
                let _ = suche(fall, &config, &pool);
            }
            gezaehlt += u64::from(durchgaenge);
        }
        let dauer = start.elapsed().as_secs_f64();
        if dauer > 0.0 {
            #[allow(clippy::cast_precision_loss)]
            teilraten.push(gezaehlt as f64 / dauer);
        }
        if let Some(p) = psi_some_avg10("/proc/pressure/cpu") {
            psi_max = psi_max.max(p);
        }
        if let Some(t) = takt_mhz(&erlaubt) {
            takte.push(t);
        }
    }

    let gesamt = gesamt_start.elapsed().as_secs_f64();
    #[allow(clippy::cast_precision_loss)]
    let ctxt_rate = match (ctxt_vor, kontextwechsel()) {
        (Some(a), Some(b)) if gesamt > 0.0 => (b.saturating_sub(a)) as f64 / gesamt,
        _ => f64::NAN,
    };

    diagnostics::disable();
    diagnostics::reset();

    let takt = if takte.is_empty() {
        f64::NAN
    } else {
        median(&mut takte)
    };
    (!teilraten.is_empty()).then(|| Blockwert {
        rate: median(&mut teilraten),
        psi_max,
        ctxt_rate,
        takt_mhz: takt,
        seit_beginn_s: angefangen_s,
    })
}

/// Der Wert, den DIESES Paar beigetragen hat — oder `None`, wenn es keinen gab.
///
/// # Der Fehler, den diese Funktion behebt
///
/// Die Ausgabezeile las beide Spalten über `last()`. Ein verworfener Block
/// landet nicht im Vektor; `last()` lieferte dann den letzten **nicht**
/// verworfenen Wert — also einen Block aus einem **früheren Paar**. Der
/// Bericht zeigte fremde Zahlen, ohne es zu sagen.
///
/// Gemessen am 2026-08-19, drei aufeinanderfolgende Zeilen eines echten Laufs:
///
/// ```text
///   Paar 4   A1 55010.66   A2 57215.12   ...   ab 372.0 / 310.0 s
///   Paar 5   A1 60559.48   A2 57215.12   ...   ab 496.0 / 310.0 s
///   Paar 6   A1 60559.48   A2 59603.32   ...   ab 496.0 / 682.0 s
/// ```
///
/// `57215.12/s bei 310.0 s` ist Paar **3**s B-Wert, dreimal wiederholt.
///
/// **Aufgefallen ist es nur an der Zeit.** Eine wiederholte Rate von 57215/s
/// sieht aus wie eine Messung; eine Zeitspalte, die rückwärts läuft, sieht
/// nach nichts aus. Der Fehler betraf Rate, PSI, Kontextwechsel und Takt
/// genauso und stand seit dem 2026-08-18 in dieser Datei.
///
/// Auf die Mediane wirkte er sich **nicht** aus — die rechnen über die
/// Vektoren, nicht über die Ausgabe. Betroffen war jede einzeln zitierte
/// Zeile.
fn aus_diesem_paar(werte: &[Blockwert], vorher: usize) -> Option<&Blockwert> {
    (werte.len() > vorher).then(|| werte.last()).flatten()
}

/// Taugen die mitgeschriebenen Blockzeiten fuer eine Stoerungsanalyse?
///
/// # Warum strenge Monotonie und nicht nur Monotonie
///
/// Eine konstante Folge ist monoton. Genau sie entsteht aber bei dem Fehler,
/// den diese Pruefung fangen soll: ein Feld, das einmal gesetzt und danach
/// nicht mehr aktualisiert wird. Es traege in jedem Block dieselbe Zahl, waere
/// monoton, laege im Lauf-Zeitraum — und waere wertlos.
///
/// Deshalb zwei Bedingungen statt einer: Die Zeiten muessen **wachsen**, und
/// sie muessen im Lauf liegen. Die zweite allein faengt eine eingefrorene Uhr
/// nicht, die erste allein faengt eine Uhr nicht, die aus einer anderen
/// Zeitrechnung stammt.
fn zeiten_taugen(zeiten: &[f64], lauf_dauer_s: f64) -> Result<(), String> {
    if zeiten.len() < 2 {
        return Err(format!(
            "nur {} Blockzeit(en) — zu wenig, um eine Reihenfolge zu belegen",
            zeiten.len()
        ));
    }
    for paar in zeiten.windows(2) {
        if paar[1] <= paar[0] {
            return Err(format!(
                "die Blockzeiten wachsen nicht: {:.3} s folgt auf {:.3} s. \
                 Eine eingefrorene Uhr sieht genauso aus wie eine langsame.",
                paar[1], paar[0]
            ));
        }
    }
    // Ab hier sind mindestens zwei Werte sicher — oben geprueft. Der Zugriff
    // auf den ersten und letzten braucht deshalb keine zweite Fallunterscheidung.
    let letzte = *zeiten.last().expect("mindestens zwei Werte");
    if letzte > lauf_dauer_s {
        return Err(format!(
            "eine Blockzeit ({letzte:.3} s) liegt hinter dem Ende des Laufs \
             ({lauf_dauer_s:.3} s) — sie stammt dann nicht aus diesem Lauf"
        ));
    }
    let erste = zeiten[0];
    if erste < 0.0 {
        return Err(format!(
            "eine Blockzeit ist negativ ({erste:.3} s) — vor dem Laufbeginn hat \
             nichts stattgefunden"
        ));
    }
    Ok(())
}

/// Der Median einer Messreihe.
fn median(werte: &mut [f64]) -> f64 {
    werte.sort_by(f64::total_cmp);
    let n = werte.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        werte[n / 2]
    } else {
        (werte[n / 2 - 1] + werte[n / 2]) / 2.0
    }
}

/// Die A/B-Wechselreihe aus AC1 und AC6.
///
/// # Warum abwechselnd und nicht erst alle A, dann alle B
///
/// Weil eine Maschine driftet. Laufen erst acht A und danach acht B, steckt
/// jede Erwaermung, jede fremde Last und jede Taktaenderung des Zeitraums
/// vollstaendig in B — und der Vergleich misst die Uhrzeit statt den
/// Unterschied. Im Wechsel trifft dieselbe Drift beide Seiten.
///
/// # Warum mindestens sieben
///
/// Das Issue sagt es und begruendet es: „Zwei Laeufe genuegen ausdruecklich
/// nicht — sie koennen zufaellig uebereinstimmen."
/// Das Urteil ueber eine A/B-Reihe — ohne Messung, ohne Bericht, ohne Uhr.
///
/// WOZU ES DAS GIBT. Bis zum 2026-08-21 stand die Verknuepfung als
/// `ac1 && ac6 && zeiten_ok` am Ende von `ab_reihe`, also hinter 540 Zeilen
/// Messung. Um sie zu erreichen, musste eine vollstaendige Reihe gelingen —
/// vier Anlaeufe starben vorher an `BLOCKED`, jeder aus einem anderen Grund.
/// Das war keine Pechserie, sondern die Bauweise: Eine Aussage ueber eine
/// Verknuepfung hing an Maschinenlast, mit der sie nichts zu tun hat.
struct Bewertung {
    ac1: bool,
    ac6: bool,
    zeiten_ok: bool,
}

impl Bewertung {
    /// **Die Blockzeiten zaehlen mit.** Nicht weil sie die Streuung beruehren —
    /// die steht unabhaengig davon —, sondern weil ein Bericht mit unbrauchbaren
    /// Zeiten spaeter fuer eine Stoerungsanalyse herangezogen wuerde, fuer die er
    /// nicht taugt. Ein Ruecklauf mit Exit 0 waere die Einladung dazu.
    fn bestanden(&self) -> bool {
        self.ac1 && self.ac6 && self.zeiten_ok
    }
}

/// Die beiden Kriterien aus den drei Kennzahlen, mit der Zeitpruefung daneben.
///
/// `unterschied`, `streu_a` und `streu_b` sind Anteile, keine Prozente: 0.03
/// ist die Drei-Prozent-Schwelle aus #896.
fn bewerte(streu_a: f64, streu_b: f64, unterschied: f64, zeiten_ok: bool) -> Bewertung {
    Bewertung {
        ac1: streu_a < 0.03 && streu_b < 0.03,
        ac6: unterschied <= streu_a.max(streu_b),
        zeiten_ok,
    }
}

fn ab_reihe(
    bericht: &mut String,
    threads: usize,
    paare: usize,
    durchgaenge: u32,
    gleiche_variante: bool,
) -> bool {
    // Der billigste Lastfall genuegt: Gemessen wird die MESSUMGEBUNG, nicht
    // der Kampf. Ein teurer Fall verlaengert die Reihe, ohne die Streuung
    // aussagekraeftiger zu machen.
    let fall = &LASTEN[0];

    let _ = writeln!(
        bericht,
        "\n{} ueber Lastfall \"{}\", {paare} Paare, je {durchgaenge} Durchgaenge",
        if gleiche_variante {
            "A,A-Reihe"
        } else {
            "A/B-Wechselreihe"
        },
        fall.name
    );
    let _ = writeln!(
        bericht,
        "{}",
        if gleiche_variante {
            "  BEIDE Durchgaenge mit Erfassung AUS — die Spalten heissen A1 und A2.\n\
             \x20 Streut auch das, liegt die Ursache nicht zwischen zwei Aufrufen.\n"
        } else {
            "  A = Erfassung AUS   B = Erfassung AN (beide mit gebautem Merkmal)\n"
        }
    );

    // **Vorlauf-Pruefung — eine Hypothese, die geprueft und gefallen ist.**
    //
    // In der A,A-Reihe vom 2026-08-18 17:07 lagen die vier niedrigsten Werte
    // genau in den Paaren 1 und 2, in BEIDEN Durchgaengen. Ohne sie fiel die
    // Streuung von 9,51 auf 5,02 Prozent. Naheliegende Erklaerung: ein
    // fehlender Vorlauf, also kalte Caches und ungewaermter Allokator. Sie
    // hatte sogar eine Entsprechung im Code — diese Funktion waermte nicht auf,
    // waehrend `miss` zwei Sekunden vorheizt.
    //
    // **Geprueft an den fuenf FRUEHEREN Reihen**, deren Rohwerte schon vorlagen,
    // als die Hypothese entstand — sie koennen von ihr nicht beeinflusst sein:
    //
    // ```text
    //   Reihe                         unter dem Median?   Streuung alle -> ohne
    //   8 Kerne, 20 s (fremde Last)        0 / 4           28,25 % -> 26,69 %
    //   8 Kerne, 20 s (ruhig)              2 / 4            3,53 % ->  3,39 %
    //   8 Kerne, 40 s                      1 / 4            4,93 % ->  4,95 %
    //   4 Kerne, 20 s (ruhig)              3 / 4           10,78 % ->  9,58 %
    //   4 Kerne, 20 s (ruhig)              2 / 4           11,68 % -> 11,68 %
    //   4 Kerne, A,A (die FUNDreihe)       4 / 4            9,51 % ->  5,02 %
    // ```
    //
    // Bei Zufall waeren zwei von vier zu erwarten. **Der Effekt existiert nur
    // dort, wo nach ihm gesucht wurde** — und in einer Reihe wird es ohne
    // Vorlauf sogar schlechter. Waere der Vorlauf die Ursache, muesste er in
    // allen sechs auftreten; allen sechs fehlte er.
    //
    // Die Zahl bleibt trotzdem im Bericht: Ein Effekt, der einmal auftrat, kann
    // wiederkommen, und dann soll er sichtbar sein statt in der Gesamtstreuung
    // zu verschwinden.
    let _ = writeln!(
        bericht,
        "  Vorlauf-Pruefung: Liegen die Paare 1 und 2 unter dem Median der\n\
         \x20 uebrigen? Als Ursache der Streuung ist das WIDERLEGT (siehe unten),\n\
         \x20 die Zahl steht trotzdem in jedem Bericht — ein Effekt, der einmal\n\
         \x20 auftrat, kann wiederkommen.\n"
    );

    let mut a = Vec::with_capacity(paare);
    let mut b = Vec::with_capacity(paare);
    let mut verworfen = 0usize;
    let mut alle_zeiten: Vec<f64> = Vec::with_capacity(paare * 2);
    let mut zeiten_ok = true;
    for i in 0..paare {
        // Die Reihenfolge innerhalb eines Paares wechselt mit, sonst steht A
        // immer direkt hinter B und erbt dessen Nachlauf.
        let (erst, dann) = if gleiche_variante {
            // A,A: ZWEIMAL DIESELBE VARIANTE hintereinander im selben Aufruf.
            //
            // Die Frage dahinter (Auftrag 8184): Streut auch das, dann liegt
            // die Ursache NICHT zwischen zwei Aufrufen — also nicht an fremder
            // Last, die zwischendurch kommt und geht, sondern im Verfahren
            // selbst oder in einer Restlast, die dauerhaft anliegt.
            (false, false)
        } else if i % 2 == 0 {
            (false, true)
        } else {
            (true, false)
        };
        let (vor_a, vor_b) = (a.len(), b.len());
        let r1 = eine_rate(fall, threads, durchgaenge, erst);
        let r2 = eine_rate(fall, threads, durchgaenge, dann);

        // **Ein Block, der unter Druck lief, ist keine Messung.** Er wird
        // verworfen und die Zahl der Verwuerfe unten ausgewiesen — nicht still
        // weggelassen. Wer nur die uebrigen sieht, haelt eine unruhige
        // Maschine fuer eine ruhige, und das ist der teurere Irrtum.
        // Im A,A-Modus unterscheiden sich die beiden Durchgaenge NICHT in der
        // Erfassung, sondern nur in ihrer Stellung. `nach_b` traegt deshalb die
        // Zuordnung, nicht `erfassung` — sonst landeten beide im selben Topf
        // und der Bericht koennte sie nicht einzeln ausweisen, was der Auftrag
        // ausdruecklich verlangt.
        for (nach_b, erfassung, wert) in [(false, erst, r1), (true, dann, r2)] {
            let Some(w) = wert else { continue };
            // **In Messreihenfolge gesammelt, samt der verworfenen.** Wer sie
            // ausliesse, saehe eine Luecke im Zeitverlauf genau dort, wo die
            // Stoerung war — und die Pruefung darunter liefe ueber einen
            // Verlauf, den es so nie gab.
            alle_zeiten.push(w.seit_beginn_s);
            if w.psi_max > PSI_CPU_MAX {
                verworfen += 1;
                let _ = writeln!(
                    bericht,
                    // **Die Zeit gehoert gerade hier hin.** Ein verworfener
                    // Block ist der wahrscheinlichste Treffer einer fremden
                    // Stoerung — und der einzige, der aus der Reihe
                    // verschwindet. Ohne seine Zeit sucht man ihn spaeter
                    // zwischen den uebrigen.
                    "  Paar {:>2}   {} VERWORFEN — PSI cpu stieg auf {:.2} \
                     (Grenze {PSI_CPU_MAX:.2}), {:.0} Kontextwechsel/s, ab {:.1} s",
                    i + 1,
                    match (gleiche_variante, nach_b, erfassung) {
                        (true, false, _) => "A1",
                        (true, true, _) => "A2",
                        (false, _, true) => "B",
                        (false, _, false) => "A",
                    },
                    w.psi_max,
                    w.ctxt_rate,
                    w.seit_beginn_s
                );
                continue;
            }
            if nach_b {
                b.push(w);
            } else {
                a.push(w);
            }
        }

        // Nur drucken, wenn dieses Paar ueberhaupt etwas beigetragen hat.
        // Sonst stuende hier eine Zeile aus lauter NaN unter der Zeile, die
        // schon erklaert hat, warum — zweimal dasselbe, einmal unlesbar.
        if a.len() != vor_a || b.len() != vor_b {
            // **Je Spalte pruefen, nicht je Zeile.** Wurde nur EINE Seite
            // verworfen, traegt die andere weiterhin bei — die verworfene
            // darf dann aber nicht ueber `last()` gelesen werden, sonst zeigt
            // sie den Wert eines FRUEHEREN Paares. Siehe [`aus_diesem_paar`].
            let aw = aus_diesem_paar(&a, vor_a);
            let bw = aus_diesem_paar(&b, vor_b);
            let z =
                |w: Option<&Blockwert>, hol: fn(&Blockwert) -> f64, breite: usize, nach: usize| {
                    w.map_or_else(
                        || format!("{:>breite$}", "n/v"),
                        |b| format!("{:>breite$.nach$}", hol(b)),
                    )
                };
            let _ = writeln!(
                bericht,
                "  Paar {:>2}   {} {}/s   {} {}/s   PSI bis {} / {}   \
                 ctxt/s {} / {}   MHz {} / {}   ab {} / {} s",
                i + 1,
                if gleiche_variante { "A1" } else { "A " },
                z(aw, |w| w.rate, 10, 2),
                if gleiche_variante { "A2" } else { "B " },
                z(bw, |w| w.rate, 10, 2),
                z(aw, |w| w.psi_max, 5, 2),
                z(bw, |w| w.psi_max, 5, 2),
                z(aw, |w| w.ctxt_rate, 7, 0),
                z(bw, |w| w.ctxt_rate, 7, 0),
                z(aw, |w| w.takt_mhz, 6, 0),
                z(bw, |w| w.takt_mhz, 6, 0),
                z(aw, |w| w.seit_beginn_s, 7, 1),
                z(bw, |w| w.seit_beginn_s, 7, 1),
            );
        }
    }

    if verworfen > 0 {
        let _ = writeln!(
            bericht,
            "\n  {verworfen} Bloecke verworfen, weil die Maschine waehrend der \
             Messung unruhig war."
        );
    }

    // **Die Blockzeiten pruefen sich selbst, sonst sind sie nur ein Feld.**
    //
    // Ein Wert, den niemand prueft, ist tot: Er kann einfrieren, aus einer
    // anderen Zeitrechnung stammen oder gar nicht gesetzt werden, und der
    // Bericht saehe unveraendert aus. Erst diese Pruefung macht aus der Zahl
    // eine Zusicherung.
    // Die Obergrenze ist der Stand JETZT, am Ende der Reihe — jede Blockzeit
    // muss davor liegen. (Nicht die Reihendauer: Die Blockzeiten zaehlen ab
    // Laufbeginn, nicht ab Reihenbeginn, und ein Vergleich gegen die Dauer
    // waere um den Vorlauf des Berichts daneben.)
    let jetzt_s = seit_beginn_s();
    match zeiten_taugen(&alle_zeiten, jetzt_s) {
        Ok(()) => {
            let _ = writeln!(
                bericht,
                "\n  Blockzeiten: {} Werte, streng wachsend, von {:.1} bis {:.1} s \
                 nach Laufbeginn.\n  \x20 Eine fremde Stoerung mit Zeitstempel laesst \
                 sich damit ohne Rueckrechnung zuordnen.",
                alle_zeiten.len(),
                alle_zeiten.first().copied().unwrap_or(f64::NAN),
                alle_zeiten.last().copied().unwrap_or(f64::NAN),
            );
        }
        Err(grund) => {
            // Kein blosser Hinweis: Eine Messumgebung, die ihre eigene
            // Zusicherung nicht haelt, blockiert alles Weitere (#896,
            // Stop conditions). Ein Vermerk im Fliesstext wuerde ueberlesen.
            let _ = writeln!(
                bericht,
                "\n  BLOCKZEITEN UNBRAUCHBAR — {grund}\n  \x20 Die Streuungszahlen \
                 oben stehen weiter; die ZUORDNUNG einer Stoerung zu einem Block \
                 ist mit diesem Lauf nicht moeglich."
            );
            zeiten_ok = false;
        }
    }

    // **Trifft die Vorhersage?** Zwei Rechnungen, beide auf denselben Daten.
    {
        let werte: Vec<(usize, f64)> = a
            .iter()
            .chain(b.iter())
            .enumerate()
            .map(|(i, w)| (i % paare.max(1), w.rate))
            .collect();
        let vorlauf: Vec<f64> = werte
            .iter()
            .filter(|(paar, _)| *paar < 2)
            .map(|(_, r)| *r)
            .collect();
        let rest: Vec<f64> = werte
            .iter()
            .filter(|(paar, _)| *paar >= 2)
            .map(|(_, r)| *r)
            .collect();

        if vorlauf.len() >= 2 && rest.len() >= 4 {
            let mut v = vorlauf.clone();
            let mut r = rest.clone();
            let med_vorlauf = median(&mut v);
            let med_rest = median(&mut r);
            let spanne = |werte: &[f64]| -> f64 {
                let max = werte.iter().copied().fold(f64::MIN, f64::max);
                let min = werte.iter().copied().fold(f64::MAX, f64::min);
                let mut k = werte.to_vec();
                let m = median(&mut k);
                if m > 0.0 {
                    (max - min) / m * 100.0
                } else {
                    f64::NAN
                }
            };
            let unter = vorlauf.iter().filter(|x| **x < med_rest).count();

            let _ = writeln!(bericht, "\n  Pruefung der Vorhersage");
            let _ = writeln!(
                bericht,
                "    Paare 1 und 2   {} Werte   Median {:>9.2}/s",
                vorlauf.len(),
                med_vorlauf
            );
            let _ = writeln!(
                bericht,
                "    uebrige Paare   {} Werte   Median {:>9.2}/s   Streuung {:>6.2} %",
                rest.len(),
                med_rest,
                spanne(&rest)
            );
            let mut alles: Vec<f64> = werte.iter().map(|(_, r)| *r).collect();
            let _ = writeln!(
                bericht,
                "    alle Werte      {} Werte   Median {:>9.2}/s   Streuung {:>6.2} %",
                alles.len(),
                median(&mut alles),
                spanne(&werte.iter().map(|(_, r)| *r).collect::<Vec<f64>>())
            );
            let _ = writeln!(
                bericht,
                "    {} von {} Vorlaufwerten liegen unter dem Median der uebrigen — {}",
                unter,
                vorlauf.len(),
                if unter == vorlauf.len() {
                    "VORHERSAGE GETROFFEN"
                } else if unter * 2 > vorlauf.len() {
                    "teilweise getroffen"
                } else {
                    "VORHERSAGE VERFEHLT"
                }
            );
            let _ = writeln!(
                bericht,
                "    Ohne Vorlauf faellt die Streuung von {:.2} auf {:.2} Prozent.",
                spanne(&werte.iter().map(|(_, r)| *r).collect::<Vec<f64>>()),
                spanne(&rest)
            );
        }
    }

    // **Folgt die Streuung dem Takt?** (Auflage aus 8194)
    //
    // Nicht „schwankt der Takt ja/nein" — das ordnet jedem Fall genau eine
    // Ursache zu, obwohl bei Streuung meist mehrere zusammenwirken. Gefragt
    // ist, wie viel Streuung BEI GLEICHEM TAKT uebrig bleibt:
    //
    //   folgt dem Takt, Reststreuung klein   -> der Governor
    //   folgt dem Takt, Reststreuung gross   -> beides
    //   unabhaengig vom Takt                 -> Verfahren oder Restlast
    {
        let alle: Vec<&Blockwert> = a.iter().chain(b.iter()).collect();
        let mit_takt: Vec<&&Blockwert> = alle.iter().filter(|w| w.takt_mhz.is_finite()).collect();
        if mit_takt.len() >= 4 {
            let mut takte: Vec<f64> = mit_takt.iter().map(|w| w.takt_mhz).collect();
            let takt_mitte = median(&mut takte);
            let spanne = |werte: &[f64]| -> f64 {
                if werte.len() < 2 {
                    return f64::NAN;
                }
                let max = werte.iter().copied().fold(f64::MIN, f64::max);
                let min = werte.iter().copied().fold(f64::MAX, f64::min);
                let mut k = werte.to_vec();
                let m = median(&mut k);
                if m > 0.0 {
                    (max - min) / m * 100.0
                } else {
                    f64::NAN
                }
            };
            let unten: Vec<f64> = mit_takt
                .iter()
                .filter(|w| w.takt_mhz <= takt_mitte)
                .map(|w| w.rate)
                .collect();
            let oben: Vec<f64> = mit_takt
                .iter()
                .filter(|w| w.takt_mhz > takt_mitte)
                .map(|w| w.rate)
                .collect();
            let mut u = unten.clone();
            let mut o = oben.clone();

            let _ = writeln!(
                bericht,
                "\n  Takt und Streuung — geteilt am Median {takt_mitte:.0} MHz"
            );
            let _ = writeln!(
                bericht,
                "    unteres Band  {:>2} Bloecke   Median {:>9.2}/s   Reststreuung {:>6.2} %",
                unten.len(),
                median(&mut u),
                spanne(&unten)
            );
            let _ = writeln!(
                bericht,
                "    oberes Band   {:>2} Bloecke   Median {:>9.2}/s   Reststreuung {:>6.2} %",
                oben.len(),
                median(&mut o),
                spanne(&oben)
            );
            let mut alle_raten: Vec<f64> = mit_takt.iter().map(|w| w.rate).collect();
            let gesamt = spanne(&alle_raten);
            let _ = writeln!(
                bericht,
                "    ueber alles  {:>2} Bloecke   Median {:>9.2}/s   Streuung     {:>6.2} %",
                mit_takt.len(),
                median(&mut alle_raten),
                gesamt
            );
            let rest = spanne(&unten).max(spanne(&oben));
            let _ = writeln!(
                bericht,
                "    Bleibt bei gleichem Takt {:.2} % von {:.2} % — {}",
                rest,
                gesamt,
                if !rest.is_finite() || !gesamt.is_finite() {
                    "zu wenige Bloecke je Band fuer eine Aussage"
                } else if rest < gesamt * 0.5 {
                    "die Streuung folgt dem Takt"
                } else {
                    "der Takt erklaert sie NICHT allein"
                }
            );
        } else {
            let _ = writeln!(
                bericht,
                "\n  Takt und Streuung: zu wenige Bloecke mit Taktwert ({}).",
                mit_takt.len()
            );
        }
    }

    // **Zweites Verwurfsignal: Verdraengung.**
    //
    // Gemessen am 2026-08-18, Reihe auf vier Kernen: Ein einziger B-Block fiel
    // um acht Prozent ab (809,24 gegen sonst 868 bis 904). Sein PSI stand bei
    // 4,01 — UNTER der Grenze von 5,00, die ihn deshalb nicht fing. Seine
    // Kontextwechselrate aber lag bei 30612 gegen sonst 23954 bis 26559: 22
    // Prozent ueber dem Median der Reihe.
    //
    // Kontextwechsel messen Verdraengung durch FREMDE Prozesse, nicht die
    // Messgroesse selbst. Ein Block, der 22 Prozent haeufiger verdraengt wurde,
    // ist kein vergleichbarer Block.
    //
    // **Die Schwelle ist relativ zur eigenen Reihe.** Eine absolute Zahl waere
    // geraten — 25000 auf vier Kernen sind 45000 auf acht, und auf einer
    // anderen Maschine etwas Drittes. Der Median der Reihe ist gemessen.
    //
    // Das ist KEINE Absenkung des Kriteriums: Verworfen wird ein Block, nicht
    // eine Anforderung. Jeder Verwurf steht mit Wert und Median im Bericht,
    // und bleiben zu wenige uebrig, meldet die Reihe BLOCKED.
    let ctxt_grenze = {
        let mut alle: Vec<f64> = a
            .iter()
            .chain(b.iter())
            .map(|w| w.ctxt_rate)
            .filter(|r| r.is_finite())
            .collect();
        if alle.is_empty() {
            f64::INFINITY
        } else {
            median(&mut alle) * 1.20
        }
    };

    let mut verdraengt = 0usize;
    let mut filtern = |liste: Vec<Blockwert>, name: &str| -> Vec<f64> {
        liste
            .into_iter()
            .filter(|w| {
                if w.ctxt_rate.is_finite() && w.ctxt_rate > ctxt_grenze {
                    verdraengt += 1;
                    let _ = writeln!(
                        bericht,
                        "  {name} VERWORFEN — {:.0} Kontextwechsel/s, mehr als 20 % \
                         ueber dem Median der Reihe ({:.0})",
                        w.ctxt_rate,
                        ctxt_grenze / 1.20
                    );
                    return false;
                }
                true
            })
            .map(|w| w.rate)
            .collect()
    };
    let a: Vec<f64> = filtern(a, "A");
    let b: Vec<f64> = filtern(b, "B");
    if verdraengt > 0 {
        let _ = writeln!(
            bericht,
            "  {verdraengt} weitere Bloecke verworfen — sie wurden waehrend der \
             Messung haeufiger verdraengt als der Rest der Reihe."
        );
    }

    if a.len() < 7 || b.len() < 7 {
        let _ = writeln!(
            bericht,
            "\n  BLOCKED — {} A-Laeufe und {} B-Laeufe. AC1 verlangt 7 bis 10.",
            a.len(),
            b.len()
        );
        return false;
    }

    let streuung = |werte: &[f64]| -> f64 {
        let max = werte.iter().copied().fold(f64::MIN, f64::max);
        let min = werte.iter().copied().fold(f64::MAX, f64::min);
        let mut kopie = werte.to_vec();
        let m = median(&mut kopie);
        if m > 0.0 {
            (max - min) / m
        } else {
            f64::INFINITY
        }
    };

    let streu_a = streuung(&a);
    let streu_b = streuung(&b);
    let med_a = median(&mut a.clone());
    let med_b = median(&mut b.clone());
    let unterschied = if med_a > 0.0 {
        (med_b - med_a).abs() / med_a
    } else {
        f64::INFINITY
    };

    let _ = writeln!(
        bericht,
        "\n  Median A   {med_a:>10.2}/s   Streuung {:>6.2} %",
        streu_a * 100.0
    );
    let _ = writeln!(
        bericht,
        "  Median B   {med_b:>10.2}/s   Streuung {:>6.2} %",
        streu_b * 100.0
    );
    let _ = writeln!(
        bericht,
        "  Unterschied A gegen B   {:>6.2} %",
        unterschied * 100.0
    );

    let bewertung = bewerte(streu_a, streu_b, unterschied, zeiten_ok);
    let _ = writeln!(
        bericht,
        "\n  AC1  Streuung unter 3 %                       {}",
        if bewertung.ac1 { "PASS" } else { "FAIL" }
    );
    let _ = writeln!(
        bericht,
        "  AC6  Unterschied innerhalb der Streuung       {}",
        if bewertung.ac6 { "PASS" } else { "FAIL" }
    );
    let _ = writeln!(
        bericht,
        "  Blockzeiten brauchbar                         {}",
        if bewertung.zeiten_ok { "PASS" } else { "FAIL" }
    );
    if !bewertung.ac1 {
        let _ = writeln!(
            bericht,
            "\n  Das Kriterium wird NICHT gesenkt. Eine Messumgebung, die ihre \
             eigene\n  Zusicherung nicht haelt, blockiert alles Weitere (#896 Stop \
             conditions)."
        );
    }
    bewertung.bestanden()
}

fn main() {
    // **Als Erstes, vor allem anderen.** [`LAUFBEGINN`] setzt sich beim ersten
    // Aufruf selbst; geschaehe der irgendwo mitten im Programm, hiesse
    // „Laufbeginn" in Wahrheit „als der Bericht angefangen wurde" — und jede
    // Blockzeit waere um die Zeit davor zu klein. Der Fehler waere klein und
    // deshalb schwer zu finden.
    let _ = laufbeginn();

    let args: Vec<String> = std::env::args().collect();
    let aa_modus = args.iter().any(|a| a == "--aa");
    let ab_modus = aa_modus || args.iter().any(|a| a == "--ab");
    let paare: usize = args
        .iter()
        .position(|a| a == "--ab" || a == "--aa")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let durchgaenge: u32 = std::env::var("ROTA_AB_DURCHGAENGE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    let mut bericht = String::new();
    let _ = writeln!(bericht, "MESSUMGEBUNG (#896)");
    let _ = writeln!(bericht, "===================\n");

    // --- Nur die Trace-Hashes, wenn danach gefragt ---
    //
    // Der Nachweis zu AC4 braucht denselben Lauf zweimal, einmal mit einem
    // absichtlich veraenderten Zauberwert. Ueber den vollen Bericht waeren das
    // zweimal mehrere Minuten, nur um zwei Zeilen zu vergleichen.
    //
    // **Vor der Umgebungspruefung, und das mit Absicht.** Ein Trace-Hash misst
    // keine Zeit: Er haengt an den Spielregeln, nicht an der Maschine. Waere er
    // hinter der Kernbindung und der Druckpruefung, liesse er sich auf einer
    // beschaeftigten Maschine gar nicht abfragen — gemessen am 2026-08-18, als
    // der Bauserver den Lauf mit „PSI io 20,37" abbrach und damit auch den
    // Hash-Vergleich verweigerte, fuer den der Druck voellig gleichgueltig ist.
    if args.iter().any(|a| a == "--trace") {
        let _ = writeln!(bericht, "\nTrace-Hashes je Lastfall\n");
        let alle = all_candidates();
        for fall in LASTEN {
            let Some(boss) = boss_by_id(fall.boss) else {
                let _ = writeln!(bericht, "  {:<28} BLOCKED — Boss fehlt", fall.name);
                continue;
            };
            let pool = pool_fuer(fall.pool, &alle);
            let lauf = suche(fall, &config_fuer(boss, 1), &pool);

            diagnostics::reset();
            diagnostics::enable_trace();
            let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
                boss,
                ..rota_sim::FightConfig::default()
            });
            let ergebnis = fight.run_apl(&lauf.apl);
            let z = diagnostics::snapshot();
            diagnostics::disable_trace();
            diagnostics::reset();

            // **Schaden und Kampfende stehen daneben, nicht nur der Hash.**
            // Ein Hash allein sagt nicht, ob der Kampf ueberhaupt etwas getan
            // hat — und beim Nachweis zu AC4 ist genau das die Frage: Ein
            // veraenderter Zauberwert, der den Schaden nicht bewegt, kann auch
            // den Hash nicht bewegen, und dann liegt es nicht am Hash.
            let _ = writeln!(
                bericht,
                "  {:<28} {:<12} 0x{:08x}  ueber {:>4} Zustaende   \
                 Schaden {:>7}   {}",
                fall.name,
                fall.boss,
                z.trace(),
                z.trace_states(),
                ergebnis.damage,
                ergebnis.killed_at_ms.map_or_else(
                    || "Boss ueberlebt".to_string(),
                    |t| format!("Sieg bei {t} ms")
                )
            );
        }
        let _ = writeln!(
            bericht,
            "\n  Ein Hash, der sich bei geaenderten Spielregeln NICHT bewegt, \
             prueft nichts.\n  Der Nachweis dazu gehoert zu AC4 und steht im \
             Pull Request."
        );
        print!("{bericht}");
        return;
    }

    // --- Umgebung, bevor irgendetwas gemessen wird ---
    let kerne = erlaubte_kerne();
    let gesamt = kerne_gesamt();
    let gebunden = match &kerne {
        Some(k) => k.len() < gesamt,
        None => false,
    };
    let threads = kerne.as_ref().map_or(1, Vec::len).max(1);

    let _ = writeln!(bericht, "Umgebung");
    let _ = writeln!(
        bericht,
        "  Kerne der Maschine     {gesamt}\n  erlaubte Kerne         {}",
        kerne.as_ref().map_or_else(
            || "unbekannt".to_string(),
            |k| format!("{} ({k:?})", k.len())
        )
    );
    let _ = writeln!(bericht, "  Straenge               {threads}");
    // **Der Bezugspunkt fuer jede Blockzeit im Bericht.** Als Unix-Sekunde,
    // weil sie eindeutig ist; `date -d @<sekunde>` macht daraus Lokalzeit.
    // Einmal je Lauf umrechnen statt bei jedem Block raten.
    let _ = writeln!(
        bericht,
        "  Laufbeginn             {} (Unix-Sekunde; date -d @… gibt Lokalzeit)",
        laufbeginn().1
    );
    let _ = writeln!(
        bericht,
        "  Instrumentierung       {}",
        if cfg!(feature = "diagnostics") {
            "gebaut (--features diagnostics)"
        } else {
            "NICHT gebaut"
        }
    );

    if !gebunden {
        let _ = writeln!(
            bericht,
            "\nBLOCKED — keine Kernbindung.\n\
             \x20 Der Prozess darf alle {gesamt} Kerne benutzen. Dann entscheidet der\n\
             \x20 Scheduler, wo gerechnet wird, und zwei Laeufe sind nicht vergleichbar.\n\
             \x20 Starte gebunden, zum Beispiel:\n\
             \x20   taskset -c 0-7 <befehl>"
        );
        print!("{bericht}");
        std::process::exit(2);
    }

    // --- Drucksignale vor dem Lauf ---
    let signale = drucksignale();
    let _ = writeln!(bericht, "\nDrucksignale vor dem Lauf");
    if signale.is_empty() {
        let _ = writeln!(
            bericht,
            "  KEINE — /proc/pressure fehlt. Ohne PSI kann dieser Laeufer\n\
             \x20 Systemdruck nicht erkennen; das ist ein Blocker, kein Nullwert."
        );
        print!("{bericht}");
        std::process::exit(2);
    }
    for s in &signale {
        let _ = writeln!(
            bericht,
            "  {:<24} {:>8.2}   Grenze {:>6.2}   {}   {}",
            s.name,
            s.wert,
            s.grenze,
            if s.ueberschritten() { "UEBER" } else { "ok   " },
            s.quelle
        );
    }
    // Diese drei sind ZAEHLER seit dem Systemstart, keine Raten — allein
    // sagen sie nichts. Der A/B-Modus liest sie vor und nach jedem Lauf und
    // meldet die Differenz; hier stehen sie als Ausgangswert.
    let _ = writeln!(
        bericht,
        "  {:<24} {:>8}   /proc/stat ctxt",
        "Kontextwechsel",
        kontextwechsel().map_or_else(|| "n/v".to_string(), |n| n.to_string()),
    );
    let _ = writeln!(
        bericht,
        "  {:<24} {:>8}   /proc/vmstat pswpin+pswpout",
        "ausgelagerte Seiten",
        auslagerung().map_or_else(|| "n/v".to_string(), |n| n.to_string()),
    );
    let _ = writeln!(
        bericht,
        "  {:<24} {:>8}   core_throttle_count",
        "Taktdrosselung",
        taktdrosselung().map_or_else(|| "n/v".to_string(), |n| n.to_string()),
    );

    if let Some(s) = signale.iter().find(|s| s.ueberschritten()) {
        let _ = writeln!(
            bericht,
            "\nABBRUCH — die Maschine ist unruhig.\n\
             \x20 Signal:  {}\n\
             \x20 Wert:    {:.2}\n\
             \x20 Grenze:  {:.2}\n\
             \x20 Quelle:  {}\n\
             \x20 Es wird NICHTS gemessen. Eine Zahl von hier waere unbelastbar,\n\
             \x20 saehe aber aus wie eine Messung — und das ist der teurere Fehler.",
            s.name, s.wert, s.grenze, s.quelle
        );
        print!("{bericht}");
        std::process::exit(3);
    }

    // --- A/B-Wechselreihe, wenn danach gefragt ---
    if ab_modus {
        let bestanden = ab_reihe(&mut bericht, threads, paare, durchgaenge, aa_modus);
        print!("{bericht}");
        std::process::exit(i32::from(!bestanden));
    }

    // --- Die acht Lasten ---
    let dauer = messdauer();
    let _ = writeln!(
        bericht,
        "\nAcht Lasten, je {} s in {ABSCHNITTE} Abschnitten",
        dauer.as_secs()
    );
    if dauer < MINDESTDAUER {
        let _ = writeln!(
            bericht,
            "  KEINE MESSUNG — {} s liegen unter den geforderten {} s (#896 Scope).\n\
             \x20 Diese Zahlen taugen zum Nachsehen, ob der Laeufer durchlaeuft,\n\
             \x20 und zu nichts sonst: Bei so kurzen Abschnitten dominiert der\n\
             \x20 Aufwaermvorgang, und genau daran lag der erste Durchsatzwert\n\
             \x20 dieses Projekts um 30 Prozent daneben.",
            dauer.as_secs(),
            MINDESTDAUER.as_secs()
        );
    }
    let _ = writeln!(bericht);

    let mut ergebnisse = Vec::new();
    let mut blockiert = Vec::new();
    for fall in LASTEN {
        match miss(fall, threads) {
            Ok(e) => ergebnisse.push(e),
            Err(grund) => blockiert.push((fall.name, grund)),
        }
    }

    for e in &ergebnisse {
        let _ = writeln!(bericht, "{}", e.name);
        let _ = writeln!(
            bericht,
            "  full_fights/s          {}\n  \
               resolved_candidates/s  {}\n  \
               cache_hit_rate         {}\n  \
               full_fight_ratio       {}\n  \
               time_to_best           {}\n  \
               verified_elites/hour   {}",
            e.metriken.full_fights,
            e.metriken.resolved_candidates,
            e.metriken.cache_hit_rate,
            e.metriken.full_fight_ratio,
            e.metriken.time_to_best,
            e.metriken.verified_elites
        );
        let _ = writeln!(
            bericht,
            "  Trace-Hash             0x{:08x} ueber {} Zustaende",
            e.trace, e.trace_states
        );
        let _ = writeln!(
            bericht,
            "  Last belegt durch      {} Zeilen: {} Brand, {} Verstaerkung, \
             {} Kanal, {} Verteidigung",
            e.profil.gesamt,
            e.profil.brand,
            e.profil.verstaerkung,
            e.profil.kanal,
            e.profil.verteidigung
        );
        let _ = writeln!(
            bericht,
            "  gefundene Rotation     {}",
            e.rotation.join(" > ")
        );
        let _ = writeln!(
            bericht,
            "  Pflichtzeilen          {}",
            if e.pflicht.is_empty() {
                "keine (freie Suche)".to_string()
            } else {
                e.pflicht.join(", ")
            }
        );
        let _ = writeln!(
            bericht,
            "  Pool {} Zeilen, {} Rotationen bewertet, Ratenstreuung {:.2} %",
            e.pool_size,
            e.measured,
            e.ratenstreuung * 100.0
        );
        let z = &e.zaehler;
        let _ = writeln!(
            bericht,
            "  Schlange               p50 {} · p95 {} · p99 {} · max {} \
             (aus {} Stichproben, {} Ueberlauf)",
            z.percentile(0.50).map_or("n/v".into(), |v| v.to_string()),
            z.percentile(0.95).map_or("n/v".into(), |v| v.to_string()),
            z.percentile(0.99).map_or("n/v".into(), |v| v.to_string()),
            z.max(),
            z.samples(),
            z.overflow()
        );
        let _ = writeln!(
            bericht,
            "  expire_buffs           {} von {} Aufrufen trafen ({})\n",
            z.expire_hits(),
            z.expire_calls(),
            z.expire_hit_rate()
                .map_or_else(|| "n/v".to_string(), |q| format!("{:.2} %", q * 100.0))
        );
    }

    for (name, grund) in &blockiert {
        let _ = writeln!(bericht, "{name}\n  BLOCKED — {grund}\n");
    }

    // --- Zwei Zusicherungen ueber den ganzen Lauf ---
    let _ = writeln!(bericht, "Zusicherungen");

    // **Nur die Lastarten.** Fuer die beiden Suchlaeufe waere derselbe
    // Trace-Hash kein Fehler, sondern das erwartete Ergebnis: LNS startet von
    // der Greedy-Liste und uebernimmt nur, was messbar besser ist. Findet es
    // nichts Besseres, steht am Ende dieselbe Rotation — und ihr Hash ist
    // derselbe.
    //
    // Gemessen am 2026-08-18: genau so eingetreten, beide 0x6dd23a7a. Die
    // erste Fassung dieser Zusicherung hat es als Fehler gemeldet und damit
    // die falsche Frage gestellt. Zwei Suchlaeufe unterscheiden sich in der
    // ARBEIT, nicht im Ergebnis — dafuer steht `measured` weiter unten.
    let mut hashes: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    for e in ergebnisse.iter().filter(|e| e.ist_lastart) {
        hashes.entry(e.trace).or_default().push(e.name);
    }
    let doppelt: Vec<_> = hashes.values().filter(|v| v.len() > 1).collect();
    if doppelt.is_empty() {
        let _ = writeln!(
            bericht,
            "  {} Lastarten haben paarweise verschiedene Trace-Hashes — sie \
             rechnen verschiedene Kaempfe.",
            hashes.values().map(Vec::len).sum::<usize>()
        );
    } else {
        for gruppe in doppelt {
            let _ = writeln!(
                bericht,
                "  ACHTUNG: gleicher Trace-Hash fuer {gruppe:?} — das sind nicht zwei \
                 Lastfaelle, sondern einer unter zwei Namen."
            );
        }
    }

    // Die Zusicherung fuer die beiden Suchlaeufe: Sie duerfen dieselbe
    // Rotation finden, aber nicht dieselbe Arbeit dafuer leisten. Waeren auch
    // die bewerteten Rotationen gleich, liefe LNS gar nicht — und der achte
    // Lastfall waere der siebte unter zweitem Namen.
    let suchlaeufe: Vec<&Ergebnis> = ergebnisse.iter().filter(|e| !e.ist_lastart).collect();
    if suchlaeufe.len() == 2 {
        let (a, b) = (suchlaeufe[0], suchlaeufe[1]);
        if a.measured == b.measured {
            let _ = writeln!(
                bericht,
                "  ACHTUNG: \"{}\" und \"{}\" bewerten beide {} Rotationen — dann \
                 laeuft das zweite Verfahren nicht.",
                a.name, b.name, a.measured
            );
        } else {
            let _ = writeln!(
                bericht,
                "  Die zwei Suchlaeufe leisten verschiedene Arbeit: {} bewertet {} \
                 Rotationen, {} bewertet {}.",
                a.name, a.measured, b.name, b.measured
            );
        }
    }

    let unruhig: Vec<&str> = ergebnisse
        .iter()
        .filter(|e| e.ratenstreuung > RATENSTREUUNG_MAX)
        .map(|e| e.name)
        .collect();
    if unruhig.is_empty() {
        let _ = writeln!(
            bericht,
            "  Jede Rate blieb innerhalb von {:.0} % ueber alle {ABSCHNITTE} Abschnitte.",
            RATENSTREUUNG_MAX * 100.0
        );
    } else {
        let _ = writeln!(
            bericht,
            "  ABBRUCH — die Rate schwankte waehrend der Messung: {unruhig:?}\n\
             \x20 Kein Vorab-Signal sieht das; nur die gemessene Rate selbst zeigt es."
        );
    }

    if ergebnisse.is_empty() {
        let _ = writeln!(
            bericht,
            "  NICHTS GEMESSEN — {} Lastfaelle blockiert. Das ist FEHLGESCHLAGEN, \
             nicht bestanden.",
            blockiert.len()
        );
    }

    print!("{bericht}");

    if ergebnisse.is_empty() || !unruhig.is_empty() || !blockiert.is_empty() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // **Die Parser stehen seit #912 in der Bibliothek, ihre Pruefaelle hier.**
    // Das ist Absicht und keine Nachlaessigkeit: Sie sind an dieser Zeilenform
    // gemessen worden, und wer sie mit umzieht, verliert die Faelle, die aus
    // einem echten Vorfall stammen. `hostcaps` traegt eigene daneben.
    use rota_sim::hostcaps::{hoechste_kernnummer, kernliste, psi_aus_text, Signal};

    /// Ein Blockwert zum Prüfen — die Zahlen sind beliebig, nur unterscheidbar.
    fn probe(rate: f64, zeit: f64) -> Blockwert {
        Blockwert {
            rate,
            psi_max: 1.0,
            ctxt_rate: 10_000.0,
            takt_mhz: 2_000.0,
            seit_beginn_s: zeit,
        }
    }

    /// **Der Fall, für den `aus_diesem_paar` gebaut ist.**
    ///
    /// Der Vektor ist gewachsen — dann gehört der letzte Wert zu diesem Paar.
    #[test]
    fn ein_beigetragener_block_wird_geliefert() {
        let werte = vec![probe(100.0, 0.0), probe(200.0, 62.0)];
        let w = aus_diesem_paar(&werte, 1).expect("gewachsen, also ein Wert");
        assert!((w.rate - 200.0).abs() < 1e-9);
        assert!((w.seit_beginn_s - 62.0).abs() < 1e-9);
    }

    /// **Der Fehler von vorher, als Prüffall.**
    ///
    /// Der Vektor ist NICHT gewachsen — der Block wurde verworfen. `last()`
    /// allein lieferte hier den Wert des vorigen Paares (100.0 bei 0.0 s) und
    /// der Bericht zeigte ihn, als wäre er gemessen worden.
    #[test]
    fn ein_verworfener_block_liefert_keinen_fremden_wert() {
        let werte = vec![probe(100.0, 0.0)];
        assert!(
            aus_diesem_paar(&werte, 1).is_none(),
            "nicht gewachsen heisst verworfen — dann darf hier KEIN Wert stehen, \
             erst recht nicht der des vorigen Paares"
        );
        // Und die Gegenprobe zum leeren Vektor, damit die Null nicht der
        // einzige Grund ist, warum None herauskommt.
        assert!(aus_diesem_paar(&[], 0).is_none());
    }

    /// Brauchbare Blockzeiten kommen durch — die Gegenprobe zu allem darunter.
    ///
    /// Ohne sie wäre nicht zu unterscheiden, ob die Prüfung die Fehler fängt
    /// oder ob sie **alles** ablehnt.
    #[test]
    fn wachsende_blockzeiten_gehen_durch() {
        assert!(zeiten_taugen(&[0.0, 12.5, 25.1, 37.8], 40.0).is_ok());
    }

    /// **Der Fall, für den dieses Feld überhaupt geprüft wird.**
    ///
    /// Eine konstante Folge ist monoton, liegt im Lauf-Zeitraum und ist
    /// trotzdem wertlos: Genau so sähe ein Feld aus, das einmal gesetzt und nie
    /// aktualisiert wird. Eine Prüfung auf bloße Monotonie wäre hier grün — und
    /// damit ein Wächter, der den einzigen Fehler durchlässt, gegen den er
    /// gebaut ist.
    #[test]
    fn eine_eingefrorene_uhr_faellt_durch() {
        let fehler = zeiten_taugen(&[7.0, 7.0, 7.0, 7.0], 40.0)
            .expect_err("konstante Zeiten müssen abgelehnt werden");
        assert!(
            fehler.contains("wachsen nicht"),
            "die Meldung soll den Grund nennen: {fehler}"
        );
    }

    #[test]
    fn ruecklaeufige_blockzeiten_fallen_durch() {
        assert!(zeiten_taugen(&[0.0, 20.0, 10.0], 40.0).is_err());
    }

    /// Eine Zeit hinter dem Laufende stammt aus einer anderen Zeitrechnung.
    ///
    /// Das ist der Fehler, den eine Monotonie-Prüfung allein NICHT fängt:
    /// Unix-Sekunden statt Sekunden seit Laufbeginn wachsen einwandfrei — sie
    /// liegen nur um eineinhalb Milliarden daneben.
    #[test]
    fn eine_zeit_hinter_dem_laufende_faellt_durch() {
        let fehler = zeiten_taugen(&[0.0, 10.0, 1_755_600_000.0], 40.0)
            .expect_err("eine Zeit hinter dem Laufende muss abgelehnt werden");
        assert!(
            fehler.contains("hinter dem Ende"),
            "die Meldung soll den Grund nennen: {fehler}"
        );
    }

    /// Ein einzelner Wert belegt keine Reihenfolge.
    #[test]
    fn weniger_als_zwei_blockzeiten_belegen_nichts() {
        assert!(zeiten_taugen(&[], 40.0).is_err());
        assert!(zeiten_taugen(&[1.0], 40.0).is_err());
    }

    /// Der Parser muss die ECHTE Zeilenform treffen, nicht eine gedachte.
    ///
    /// Der Text stammt Zeichen fuer Zeichen aus `/proc/pressure/cpu` dieser
    /// Maschine, aufgenommen am 2026-08-18.
    #[test]
    fn psi_liest_avg10_aus_der_echten_zeilenform() {
        let echt = "some avg10=1.21 avg60=0.57 avg300=1.52 total=39216469353\n";
        assert_eq!(psi_aus_text(echt), Some(1.21));

        // `full` steht in `/proc/pressure/io` VOR nichts, aber in memory und io
        // gibt es beide Zeilen — die falsche zu erwischen waere still falsch.
        let mit_full = "some avg10=0.63 avg60=0.26 avg300=0.16 total=1\n\
                        full avg10=99.00 avg60=99.00 avg300=99.00 total=2\n";
        assert_eq!(
            psi_aus_text(mit_full),
            Some(0.63),
            "die full-Zeile darf nicht gelesen werden"
        );
    }

    #[test]
    fn psi_ohne_passende_zeile_ist_keine_null() {
        assert_eq!(psi_aus_text(""), None);
        assert_eq!(psi_aus_text("full avg10=1.0\n"), None);
        assert_eq!(
            psi_aus_text("some avg60=1.0 total=2\n"),
            None,
            "avg10 fehlt — das ist keine Null, sondern keine Angabe"
        );
    }

    #[test]
    fn kernliste_versteht_bereich_einzelwert_und_gemischt() {
        assert_eq!(kernliste("0-7"), Some((0..=7).collect()));
        assert_eq!(kernliste("2,4,6"), Some(vec![2, 4, 6]));
        assert_eq!(
            kernliste("0-3,8-11"),
            Some(vec![0, 1, 2, 3, 8, 9, 10, 11]),
            "gemischte Form — kommt auf Maschinen mit abgeschalteten Kernen vor"
        );
        assert_eq!(kernliste(" 0-1 "), Some(vec![0, 1]), "Leerraum ringsum");
    }

    #[test]
    fn kernliste_meldet_unsinn_statt_ihn_zu_raten() {
        assert_eq!(kernliste(""), None);
        assert_eq!(kernliste("abc"), None);
        assert_eq!(kernliste("7-0"), None, "rueckwaerts ist keine Liste");
    }

    /// Die Zahl, an der die Kernbindung haengt.
    ///
    /// `available_parallelism` taugt dafuer nicht — sie beachtet die Bindung
    /// bereits und liefert unter `taskset -c 0-7` auf dieser 16-Kern-Maschine
    /// **8**. Der Vergleich „erlaubt < gesamt" waere damit einer gegen sich
    /// selbst, und der Laeufer meldete gebunden „nicht gebunden".
    #[test]
    fn hoechste_kernnummer_zaehlt_ab_null() {
        assert_eq!(hoechste_kernnummer("0-15\n"), Some(16));
        assert_eq!(hoechste_kernnummer("0-7"), Some(8));
        assert_eq!(hoechste_kernnummer("0"), Some(1), "ein einziger Kern");
        assert_eq!(hoechste_kernnummer("0-3,8-11\n"), Some(12));
        assert_eq!(hoechste_kernnummer(""), None);
    }

    #[test]
    fn median_bei_gerader_und_ungerader_anzahl() {
        assert!((median(&mut [3.0, 1.0, 2.0]) - 2.0).abs() < f64::EPSILON);
        assert!(
            (median(&mut [4.0, 1.0, 3.0, 2.0]) - 2.5).abs() < f64::EPSILON,
            "bei gerader Anzahl das Mittel der beiden mittleren"
        );
        assert!((median(&mut [5.0]) - 5.0).abs() < f64::EPSILON);
        assert!(median(&mut []).is_nan(), "kein Median ohne Werte");
    }

    /// Der Median darf nicht vom Mittelwert abweichen, wo beide gleich sein
    /// muessen — und muss es, wo ein Ausreisser drin ist.
    #[test]
    fn median_ist_gegen_ausreisser_unempfindlich() {
        let mut mit_ausreisser = [100.0, 101.0, 102.0, 103.0, 9000.0];
        assert!(
            (median(&mut mit_ausreisser) - 102.0).abs() < f64::EPSILON,
            "der Median ignoriert die 9000 — genau dafuer steht er hier und \
             nicht der Mittelwert"
        );
    }

    /// Ein Signal ueber seiner Grenze muss auch als solches gelten.
    #[test]
    fn signal_erkennt_die_ueberschreitung() {
        let unter = Signal {
            name: "test",
            wert: 4.99,
            grenze: 5.0,
            quelle: "test",
        };
        let ueber = Signal {
            name: "test",
            wert: 5.01,
            grenze: 5.0,
            quelle: "test",
        };
        assert!(!unter.ueberschritten());
        assert!(ueber.ueberschritten());
    }

    /// Ein fehlender Wert wird als solcher gedruckt, nicht als Null.
    #[test]
    fn fehlender_wert_druckt_sich_nicht_als_null() {
        assert!(format!("{}", Wert::Fehlt("kein Cache")).contains("n/v"));
        assert!(!format!("{}", Wert::Fehlt("kein Cache")).contains("0.00"));
        assert!(format!("{}", Wert::Zahl(0.0)).contains("0.0000"));
    }

    /// Die acht Lastfaelle muessen acht sein — und verschieden heissen.
    #[test]
    fn es_sind_acht_lasten_mit_verschiedenen_namen() {
        assert_eq!(LASTEN.len(), 8, "AC3 verlangt acht Lasten");
        let mut namen: Vec<&str> = LASTEN.iter().map(|l| l.name).collect();
        namen.sort_unstable();
        let vorher = namen.len();
        namen.dedup();
        assert_eq!(namen.len(), vorher, "zwei Lastfaelle heissen gleich");

        // Und je ein Greedy- und ein LNS-Lauf, wie AC3 sie nennt.
        assert_eq!(
            LASTEN
                .iter()
                .filter(|l| l.verfahren == Verfahren::Greedy)
                .count(),
            1
        );
        assert_eq!(
            LASTEN
                .iter()
                .filter(|l| l.verfahren == Verfahren::Lns)
                .count(),
            1
        );
    }

    /// Jede Lastart muss ihre Pflichtzeilen auch bekommen.
    ///
    /// Ohne diesen Fall faellt eine Art, fuer die es keine unbedingte Zeile
    /// gibt, erst im Messlauf auf — nach zwanzig Sekunden je Abschnitt.
    #[test]
    fn jede_lastart_findet_ihre_pflichtzeilen() {
        let alle = all_candidates();
        for fall in LASTEN {
            if matches!(fall.pool, Pool::Voll | Pool::NurSchaden) {
                continue;
            }
            let pool = pool_fuer(fall.pool, &alle);
            let pflicht = pflicht_fuer(fall.pool, &pool);
            assert!(
                !pflicht.is_empty(),
                "{}: keine unbedingte Zeile der geforderten Art im Pool",
                fall.name
            );
        }
    }

    /// Der Fuellzauber gehoert ans ENDE, sonst kommt die Last nie zum Zug.
    ///
    /// Das ist der Fehler, an dem am 2026-08-18 drei Anlaeufe gescheitert
    /// sind: `kindle [unbedingt]` stand vorn, und eine Prioritaetenliste nimmt
    /// die erste zutreffende Regel.
    #[test]
    fn der_fuellzauber_steht_hinten() {
        let alle = all_candidates();
        for fall in LASTEN.iter().filter(|l| l.verfahren == Verfahren::Kampf) {
            // `NurSchaden` und `Voll` haben keine Pflichtzeilen, also auch
            // keine Reihenfolge zu wahren. Bei `Voll` kommt die Last vom BOSS
            // (`test-phase`) und nicht von der Zauberart — der Fall hier
            // pruefte ihn zuerst mit und meldete „keine Fuellzeile" fuer einen
            // Lastfall, der gar keine braucht.
            if matches!(fall.pool, Pool::NurSchaden | Pool::Voll) {
                continue;
            }
            let pool = pool_fuer(fall.pool, &alle);
            let zeilen = feste_rotation(fall, &pool);
            assert!(zeilen.len() >= 2, "{}: keine Fuellzeile", fall.name);
            let letzte = zeilen.last().expect("mindestens zwei");
            assert!(
                ist_reiner_schaden(letzte.spell),
                "{}: hinten steht {}, kein reiner Schadenszauber",
                fall.name,
                SPELLS[letzte.spell].id
            );
            for vorn in &zeilen[..zeilen.len() - 1] {
                assert!(
                    !ist_reiner_schaden(vorn.spell),
                    "{}: {} steht VOR der Last und verdraengt sie",
                    fall.name,
                    SPELLS[vorn.spell].id
                );
            }
        }
    }
}

#[cfg(test)]
mod lastfaelle_machen_etwas {
    use super::*;

    /// **Jede Lastart muss Schaden anrichten.**
    ///
    /// Der teuerste Befund dieses Issues: „brandlastig" wirkte 30 Brandzauber
    /// und machte **0 Schaden**. `smoulder [unbedingt]` legte den Brand bei
    /// jedem GCD (1500 ms) neu an, er tickt aber erst nach 2000 ms, und
    /// `refreshMode: "reset"` setzte ihn dabei zurueck — es fiel nie ein Tick.
    ///
    /// Vier Berichte lang stand der Fall als gueltig da. Aufgefallen ist er
    /// erst, als ein HUNDERTFACHER Tickschaden den Trace-Hash nicht bewegte —
    /// und auch dann erst nach drei Fehlspuren (Bau, Ruecktransfer,
    /// Zeitstempel des Bauservers).
    ///
    /// Dieser Fall haette ihn in der ersten Minute gefunden.
    #[test]
    fn jede_lastart_macht_schaden() {
        let alle = all_candidates();
        for fall in LASTEN.iter().filter(|l| l.verfahren == Verfahren::Kampf) {
            let boss = boss_by_id(fall.boss).expect("Boss steht in den Daten");
            let pool = pool_fuer(fall.pool, &alle);
            let lauf = suche(fall, &config_fuer(boss, 1), &pool);

            let mut fight = rota_sim::Fight::new(rota_sim::FightConfig {
                boss,
                ..rota_sim::FightConfig::default()
            });
            let ergebnis = fight.run_apl(&lauf.apl);

            assert!(
                ergebnis.damage > 0,
                "{}: 0 Schaden. Die Rotation steht ({}), aber sie richtet nichts aus.",
                fall.name,
                lauf.zeilen
                    .iter()
                    .map(|k| SPELLS[k.spell].id)
                    .collect::<Vec<_>>()
                    .join(" > ")
            );
        }
    }

    /// Braende werden nachgelegt, nicht bei jedem GCD zurueckgesetzt.
    ///
    /// Die Regel dahinter steht in `pflicht_fuer`. Dieser Fall haelt sie fest:
    /// Wer sie auf `Origin::Always` zuruecksetzt, bekommt wieder eine Last aus
    /// lauter Erneuerungen ohne einen einzigen Tick.
    #[test]
    fn brandlast_legt_nach_statt_zurueckzusetzen() {
        let alle = all_candidates();
        let fall = LASTEN
            .iter()
            .find(|l| l.pool == Pool::Brand)
            .expect("es gibt einen brandlastigen Fall");
        let pool = pool_fuer(fall.pool, &alle);
        let pflicht = pflicht_fuer(fall.pool, &pool);

        assert!(!pflicht.is_empty(), "keine Brand-Pflichtzeile");
        for k in &pflicht {
            assert_eq!(
                k.origin,
                rota_sim::rotation_search::Origin::DotRefresh,
                "{} steht als {:?} in der Pflicht — mit [unbedingt] faellt kein Tick",
                SPELLS[k.spell].id,
                k.origin
            );
        }
    }
}

/// Die Verknuepfung der drei Kriterien (#896).
///
/// Eigenes Modul, weil diese Faelle mit den Lastfaellen darueber nichts zu tun
/// haben: Sie brauchen keinen Kampf, keine Uhr und keine Maschine.
#[cfg(test)]
mod bewertung_verknuepft_die_kriterien {
    use super::*;

    //
    // Bis zum 2026-08-21 war sie nur ueber einen vollstaendigen Messlauf
    // erreichbar; vier Anlaeufe starben vorher an BLOCKED. Diese drei Faelle
    // pruefen sie in Millisekunden — und der mittlere ist der Grund, warum es
    // sie gibt.
    //
    // WAS SIE NICHT BELEGEN: dass `ab_reihe` `bestanden()` auch zurueckgibt.
    // Diese eine Zeile bleibt ungeprueft. Die Luecke ist damit nicht
    // geschlossen, sondern von 540 Zeilen auf eine geschrumpft, die bei jedem
    // Lesen sichtbar ist.

    // Ein Satz Werte fuer alle drei Faelle, damit "einziger Unterschied ist das
    // letzte Argument" auch dann noch stimmt, wenn jemand die Zahlen anfasst.
    // Streuung 1,0 % und 1,2 %, Unterschied 0,5 % — alle drei Kriterien halten.
    const GUT_A: f64 = 0.01;
    const GUT_B: f64 = 0.012;
    const GUT_UNTERSCHIED: f64 = 0.005;

    #[test]
    fn gute_werte_bestehen() {
        let b = bewerte(GUT_A, GUT_B, GUT_UNTERSCHIED, true);
        assert!(b.ac1, "1,0 % und 1,2 % liegen unter der 3-%-Schwelle");
        assert!(b.ac6, "0,5 % liegt innerhalb der groesseren Streuung 1,2 %");
        assert!(b.bestanden());
    }

    #[test]
    fn unbrauchbare_blockzeiten_kippen_das_urteil() {
        // DER VERDRAHTUNGSNACHWEIS. Einziger Unterschied zum Fall darueber ist
        // das letzte Argument — ac1 und ac6 bleiben beweisbar unberuehrt.
        let b = bewerte(GUT_A, GUT_B, GUT_UNTERSCHIED, false);
        assert!(b.ac1, "ac1 darf sich nicht mitbewegen");
        assert!(b.ac6, "ac6 darf sich nicht mitbewegen");
        assert!(
            !b.bestanden(),
            "zeiten_ok=false muss das Gesamturteil kippen, obwohl AC1 und AC6 halten"
        );
    }

    #[test]
    fn zu_grosse_streuung_kippt_trotz_brauchbarer_zeiten() {
        // Die Gegenrichtung: Ohne sie belegt der Fall oben nur, dass IRGENDEIN
        // false das Urteil kippt — nicht, dass jeder Teil einzeln zaehlt.
        let b = bewerte(0.04, GUT_B, GUT_UNTERSCHIED, true);
        assert!(!b.ac1, "4 % liegen ueber der Schwelle");
        assert!(b.zeiten_ok);
        assert!(!b.bestanden());
    }
}
