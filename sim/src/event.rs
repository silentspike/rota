//! Die Ereignis-Warteschlange.
//!
//! GAME-DESIGN 18.5 nennt als erstes von SimulationCraft uebernommenes Konzept:
//!
//! > **Ereignisgesteuert** statt fester Takt — zwischen zwei Tastendruecken
//! > passiert ausser DoT-Ticks nichts.
//!
//! Ein Kampf ueber 60 Sekunden hat bei 10 ms Takt 6.000 Schritte, von denen rund
//! 5.950 nichts tun. Der Simulator springt stattdessen von Ereignis zu Ereignis.
//!
//! # Warum die Reihenfolge so streng festgelegt ist
//!
//! Aus 18.4: *"Feste Ereignis-Reihenfolge, keine Hash-Container. Sonst weichen
//! Spiel und Simulator ab — und man sucht tagelang."*
//!
//! Treffen mehrere Ereignisse auf dieselbe Millisekunde, entscheidet die
//! Reihenfolge ueber das Ergebnis. Ein Beispiel aus Schritt 1: Bei 12.000 ms
//! fallen ein DoT-Tick (+1 Hitze) und ein Verfall-Schritt (−1 Hitze) zusammen.
//! Beide Reihenfolgen ergeben denselben Hitze-Stand — aber nur, weil hier kein
//! Deckel greift. Steht die Hitze bei 20, verpufft der Tick in der einen
//! Reihenfolge und wird in der anderen mitgenommen. Deshalb ist die Rangfolge
//! Teil der Regeln und nicht dem Zufall der Sammlung ueberlassen.
//!
//! Die Rangfolge bildet die Takt-Schleife des Spiels ab
//! (`src/devtools/reference-run.ts`, `tools/step1-check.py`):
//! erst die passiven Wirkungen, dann die Entscheidung des Spielers.

use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

use crate::time::Millis;

/// Was zu einem Zeitpunkt geschieht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Ein Brand-Effekt tickt.
    DotTick {
        /// Index des Zaubers in `data::SPELLS`.
        spell: u8,
        /// Welche Anwendung des Effekts gemeint ist.
        ///
        /// Beim Erneuern verfaellt der alte Effekt (`refreshMode: "reset"`),
        /// seine bereits eingeplanten Ticks liegen aber noch in der Schlange.
        /// Ohne diesen Zaehler wuerde ein alter Tick, der zufaellig auf denselben
        /// Zeitpunkt faellt wie ein neuer, **doppelt** zuschlagen.
        generation: u16,
    },
    /// Die Ressource verfaellt um einen Schritt.
    ResourceDecay,
    /// Die Energieleiste des Bosses steigt um einen Schritt.
    ///
    /// Die zweite Uhr (GAME-DESIGN 11.2, Baustein 1). Sie steht **nach** dem
    /// Ressourcen-Verfall in der Rangfolge, weil beides passive Wirkungen sind
    /// und die Reihenfolge festliegen muss — nicht, weil eine wichtiger waere.
    EnergyTick,
    /// Der toedliche Zaehler steigt um einen Schritt (Baustein 2, Issue #216).
    ///
    /// **Zwischen Energieleiste und Zeitleiste**, und zwar genau dort, wo er
    /// auch im Spiel steht (`Combat::tick`). Die Rangfolge dieser Aufzaehlung
    /// entscheidet, was bei gleichem Zeitpunkt zuerst laeuft; steht sie hier
    /// anders als dort, laufen beide Seiten bei jedem Zusammentreffen
    /// auseinander — und zwar nur dann, was es schwer zu finden macht.
    DoomTick,
    /// Die davonlaufende Reaktion senkt den Ring (Baustein 6, Issue #247).
    ///
    /// **Zwischen Zaehler und Schwaechung**, weil sie im Spiel genau dort steht
    /// (`Combat::tick`: Energieleiste, Zaehler, Reaktion, Schwaechung, Stapel).
    RunawayTick,
    /// Die Schwaechung ohne Heil-Loesung tickt (Baustein 8, Issue #240).
    AfflictionTick,
    /// Ein exponentieller Stapel kommt dazu (Baustein 3, Issue #216).
    StackTick,
    /// Ein Eintrag der Boss-Zeitleiste wird faellig.
    ///
    /// Steht nach der Energieleiste, aus demselben Grund: Beides sind
    /// Handlungen des Bosses, und ihre Reihenfolge muss festliegen, damit
    /// zwei Ereignisse auf demselben Zeitpunkt in beiden Umsetzungen gleich
    /// laufen.
    TimelineStep {
        /// Fortschrittszaehler der Zeitleiste, **nicht** der Platz in der Liste.
        ///
        /// `u32`, seit sich die Zeitleiste wiederholen kann (#174): Der Zaehler
        /// waechst dann ueber die Laenge der Liste hinaus. Ein `as u16` haette
        /// still abgeschnitten, der Abgleich in `timeline_step` waere
        /// fehlgeschlagen — und die Puppe haette einfach aufgehoert zu handeln.
        index: u32,
    },
    /// Ein laufender Boss-Zauber ist fertig gewirkt und trifft — **falls** er
    /// nicht inzwischen unterbrochen wurde.
    ///
    /// Steht vor dem Weckruf und hinter den uebrigen Boss-Handlungen: Er ist
    /// eine passive Wirkung wie sie, und der Spieler soll im selben Takt noch
    /// entscheiden koennen.
    BossCastLands {
        /// Fortschrittszaehler der Zeitleiste, siehe [`EventKind::TimelineStep`].
        index: u32,
    },
    /// Eine Teilwirkung einer **Kanalisierung** faellt an (Issue #323).
    ///
    /// Steht unmittelbar VOR [`EventKind::PlayerCastLands`], weil der Block im
    /// Spiel genau dort steht (`Combat::stepPassive`). Die Reihenfolge ist
    /// keine Geschmacksfrage: Ein abbrechender Schlag des Bosses laeuft vorher
    /// und raeumt — dann faellt diese Teilwirkung nicht mehr, und genau das
    /// heisst "der erreichte Anteil bleibt".
    ChannelTick {
        /// Index des Zaubers in `data::SPELLS`.
        spell: u8,
        /// Welcher Wirkvorgang gemeint ist — dieselbe Vorsichtsmassnahme wie
        /// bei [`EventKind::PlayerCastLands`]. Eine abgebrochene Kanalisierung
        /// laesst ihre restlichen Termine in der Schlange stehen; ohne diesen
        /// Zaehler zahlte ein spaeterer Wurf desselben Zaubers sie nachtraeglich
        /// aus.
        generation: u16,
    },
    /// Die Wirkzeit eines SPIELER-Zaubers ist um — er wirkt jetzt
    /// (GAME-DESIGN 7.1, Issue #304).
    ///
    /// Steht hinter allen Handlungen des Bosses und vor dem Weckruf, genau wie
    /// in `Combat::stepPassive`: Ein abbrechender Schlag im selben Takt muss
    /// den Zauber noch erwischen koennen. Stuende die Aufloesung davor, kaeme
    /// der Schaden durch und der Abbruch ginge ins Leere.
    ///
    /// **Bei einer Kanalisierung raeumt dieser Termin nur** (Issue #323): Ihre
    /// Wirkung ist ueber die Schritte bereits gefallen. Ohne ihn bliebe
    /// `player_casting` gesetzt, und jeder spaetere Druck "braeche" eine
    /// laengst beendete Kanalisierung ab — gemessen: 3 Abbrueche statt 1.
    PlayerCastLands {
        /// Index des Zaubers in `data::SPELLS`.
        spell: u8,
        /// Welcher Wirkvorgang gemeint ist.
        ///
        /// Dieselbe Vorsichtsmassnahme wie bei `DotTick`: Ein abgebrochener
        /// Zauber laesst seinen Weckruf in der Schlange stehen. Ohne diesen
        /// Zaehler loeste ein spaeterer Wirkvorgang desselben Zaubers ihn
        /// versehentlich frueh aus.
        generation: u16,
    },
    /// Ein Schritt einer Heilung ueber Zeit kommt an (POULTICE, Issue #481).
    ///
    /// Steht **zwischen Ressourcen-Verfall und Energieleiste**, weil der Block
    /// in `Combat::stepPassive` genau dort steht. Diese Reihenfolge ist keine
    /// Geschmacksfrage: Sie entscheidet, was bei gleichem Zeitpunkt zuerst
    /// laeuft — und ein Heil-Schritt, der vor statt nach dem Schlag des Bosses
    /// ankommt, ergibt eine andere Anzeige und damit eine andere Pruefsumme.
    HealTick {
        /// Welche Anwendung gemeint ist.
        ///
        /// Dieselbe Vorsichtsmassnahme wie bei `DotTick`: Ein zweiter Wurf
        /// setzt die Heilung neu auf, seine Vorgaenger liegen aber noch in der
        /// Schlange. Ohne diesen Zaehler heilten beide nebeneinander weiter.
        generation: u16,
    },
    /// Nur aufwachen: Zu diesem Zeitpunkt kann sich eine Entscheidung ergeben
    /// (Abklingzeit oder globale Wartezeit ist abgelaufen), oder es ist zu
    /// pruefen, ob der Boss gefallen ist.
    Wakeup,
}

impl EventKind {
    /// Rang innerhalb derselben Millisekunde. Kleiner heisst frueher.
    ///
    /// Passive Wirkungen zuerst, in der Reihenfolge der Referenz
    /// (`step_passive`: erst der Tick, dann der Verfall), danach alles, was zu
    /// einer Entscheidung fuehren kann.
    /// Der Rang dieser Art in der Ereignis-Reihenfolge.
    ///
    /// Oeffentlich seit #909: Der Beweis zu AC5 (die Bitaufteilung des
    /// Schluessels) braucht die tatsaechlich vergebenen Raenge, und ein
    /// Prueffall soll sie **ablesen** statt sie im Quelltext nachzuschlagen.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            EventKind::DotTick { .. } => 0,
            EventKind::ResourceDecay => 1,
            // **Der Heil-Schritt zwischen Verfall und Energieleiste**
            // (POULTICE, Issue #481) — genau wie in `Combat::stepPassive`. Die
            // folgenden Raenge ruecken dadurch nach; ihre ABSTAENDE bleiben,
            // und nur die zaehlen.
            EventKind::HealTick { .. } => 2,
            EventKind::EnergyTick => 3,
            // Zaehler und Stapel stehen zwischen Energieleiste und Zeitleiste,
            // genau wie in `Combat::tick` (Issue #216). Die drei folgenden
            // Raenge ruecken dadurch nach — ihre ABSTAENDE bleiben, und nur
            // die zaehlen.
            EventKind::DoomTick => 4,
            // Die Schwaechung steht zwischen Zaehler und Stapel, genau wie in
            // `Combat::tick` (Issue #240) — die folgenden Raenge ruecken
            // dadurch nach, ihre ABSTAENDE bleiben.
            EventKind::RunawayTick => 5,
            EventKind::AfflictionTick => 6,
            EventKind::StackTick => 7,
            // **Der landende Boss-Zauber VOR dem Vorruecken der Zeitleiste**
            // (Issue #331). Bis hierher war es umgekehrt, und beide Seiten
            // loesten dieselbe Lage entgegengesetzt auf.
            //
            // # Warum diese Reihenfolge die richtige ist
            //
            // Faellt der Schritt zuerst, ueberschreibt der neue Eintrag das
            // `casting` des alten — der angefangene Zauber landet nie, obwohl
            // ihn niemand unterbrochen hat. Der Boss unterbricht sich selbst.
            //
            // Das nimmt dem Spieler einen Schaden, den er VERDIENT hat: Er
            // hat nicht unterbrochen, also muss er den Treffer nehmen. Genau
            // das ist die Entscheidung, um die es in einem Rotations-Spiel
            // geht — wer sie falsch trifft, soll die Folge spueren.
            //
            // Gemessen an `test-collision`: Der Simulator liess zwei von vier
            // Zaubern ausfallen (960 statt 1920 erlittener Schaden). Der
            // Spieler bekam geschenkt, was er haette verhindern muessen.
            EventKind::BossCastLands { .. } => 8,
            EventKind::TimelineStep { .. } => 9,
            // **Die Teilwirkung einer Kanalisierung unmittelbar VOR der
            // landenden Wirkzeit** (Issue #323) — dieselbe Stelle wie im Spiel,
            // wo der Block in `Combat::stepPassive` direkt darueber steht.
            //
            // Der Abstand zu allem davor ist das Entscheidende: Ein
            // abbrechender Schlag des Bosses (Rang 8) laeuft frueher und
            // raeumt, dann faellt diese Teilwirkung nicht mehr. Genau das
            // heisst "der erreichte Anteil bleibt" — und nichts darueber
            // hinaus.
            EventKind::ChannelTick { .. } => 10,
            // Die eigene Wirkzeit zuletzt unter den Zeit-Ereignissen und vor
            // dem Weckruf (Issue #304) — dieselbe Stelle wie im Spiel.
            EventKind::PlayerCastLands { .. } => 11,
            EventKind::Wakeup => 12,
        }
    }
}

/// Ein eingeplantes Ereignis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    /// Wann es faellig ist.
    pub time: Millis,
    /// Was geschieht.
    pub kind: EventKind,
    /// Einfuege-Nummer. Loest Gleichstand auf, damit die Reihenfolge auch bei
    /// gleicher Zeit **und** gleichem Rang eindeutig ist.
    seq: u32,
}

impl Event {
    /// Die Einfuege-Nummer.
    ///
    /// Oeffentlich seit #909, damit ein Prueffall belegen kann, dass
    /// [`EventQueue::clear`] sie wirklich auf null setzt. Ohne das wuechse sie
    /// ueber alle Kaempfe hinweg und sprengte irgendwann ihr Bitfeld im
    /// Schluessel.
    #[must_use]
    pub const fn seq(self) -> u32 {
        self.seq
    }
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time
            .cmp(&other.time)
            .then_with(|| self.kind.rank().cmp(&other.kind.rank()))
            .then_with(|| self.seq.cmp(&other.seq))
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// --------------------------------------------------------- Der 64-Bit-Schluessel

/// Breite des Zeitfeldes im Schluessel — der volle Bereich von [`Millis`].
pub const ZEIT_BITS: u32 = 32;

/// Breite des Rangfeldes.
///
/// Der hoechste vergebene Rang ist 12 ([`EventKind::rank`]), vier Bit fassen
/// 0 bis 15. [`schluessel_breiten_passen`] rechnet das ueber **alle** Varianten
/// nach, statt es zu behaupten.
pub const RANG_BITS: u32 = 4;

/// Breite des Reihenfolge-Feldes — der Rest.
pub const SEQ_BITS: u32 = 64 - ZEIT_BITS - RANG_BITS;

/// Groesster Wert, den `seq` tragen kann.
pub const SEQ_MAX: u32 = (1u32 << SEQ_BITS) - 1;

/// Ein Ereignis als **eine** Zahl, in der Ordnung `time > rang > seq`.
///
/// # Warum ein einziger `u64`
///
/// [`Event::cmp`] vergleicht drei Felder nacheinander, mit zwei
/// Verzweigungen bei Gleichstand. Als Schluessel ist es **ein** Vergleich —
/// und die Warteschlange vergleicht bei jedem Einfuegen und Entnehmen.
///
/// Die Ordnung bleibt dabei **exakt dieselbe**: Das hoeherwertige Feld
/// entscheidet zuerst, genau wie `then_with`.
///
/// # Der Beweis, den AC5 verlangt
///
/// | Feld | Bits | Wertebereich | warum kein Ueberlauf |
/// | --- | --- | --- | --- |
/// | `time` | 32 | `Millis` = `u32` | der Typ selbst ist die Schranke |
/// | `rang` | 4 | 0..=12 | hoechster vergebener Rang, ueber alle Varianten nachgerechnet |
/// | `seq` | 28 | 0..=268 435 455 | `clear()` setzt ihn je Kampf auf 0; [`EventQueue::push`] prueft die Schranke |
///
/// 32 + 4 + 28 = 64 — kein Feld ueberlappt, keins bleibt uebrig.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Schluessel(pub u64);

impl Schluessel {
    /// Baut den Schluessel aus den drei Feldern.
    ///
    /// # Panics
    ///
    /// In Debug-Bauten, wenn `rang` oder `seq` ihre Breite sprengen. Im
    /// Release-Bau werden sie maskiert — dort faengt [`EventQueue::push`] den
    /// Fall vorher ab.
    #[must_use]
    pub const fn neu(time: Millis, rang: u8, seq: u32) -> Self {
        debug_assert!((rang as u32) < (1 << RANG_BITS), "Rang sprengt sein Feld");
        debug_assert!(seq <= SEQ_MAX, "seq sprengt sein Feld");
        Self(
            ((time as u64) << (RANG_BITS + SEQ_BITS))
                | (((rang as u64) & ((1 << RANG_BITS) - 1)) << SEQ_BITS)
                | ((seq as u64) & ((1u64 << SEQ_BITS) - 1)),
        )
    }

    /// Die Zeit zurueck aus dem Schluessel.
    #[must_use]
    pub const fn zeit(self) -> Millis {
        (self.0 >> (RANG_BITS + SEQ_BITS)) as Millis
    }
}

/// Ob die gewaehlten Breiten fuer **alle** Ereignisarten reichen.
///
/// Steht oeffentlich, damit ein Prueffall den Beweis aus AC5 nachrechnen kann,
/// statt ihn zu glauben. Kommt eine Ereignisart mit hoeherem Rang dazu, faellt
/// dieser Test — und nicht erst der Kampf.
#[must_use]
pub fn schluessel_breiten_passen(raenge: &[u8]) -> bool {
    ZEIT_BITS + RANG_BITS + SEQ_BITS == 64
        && raenge.iter().all(|r| u32::from(*r) < (1 << RANG_BITS))
}

/// Die Warteschlange.
///
/// Eine Halde mit **vorab angeforderter** Kapazitaet: Solange sie nicht
/// ueberlaeuft, fordert weder `push` noch `pop` Speicher an. Das ist die Regel
/// *"Keine Speicheranforderung im Schritt, zusammenhaengende Listen"* aus 18.4;
/// `tests/allocation.rs` misst nach, dass sie eingehalten wird.
/// Plaetze im festen Feld (Issue #909, Punkt 2 — Groesse nach AC3).
///
/// # Warum 48 und nicht 32 oder 64
///
/// **Gemessen, nicht gesetzt.** Ueber 147 076 800 Stichproben im LNS-Lauf
/// (Messumgebung #896, 19.08.):
///
/// ```text
/// p50  7   p95 11   p99 12   max 18      0 Ueberlauf
/// ```
///
/// Das Issue nennt als Ausgangswert „hoechstens neun"; gemessen sind heute
/// **18**. Die Zahl im Issue ist ueberholt — sie stammt von vor den
/// Bausteinen, die seither dazugekommen sind.
///
/// | Groesse | Urteil |
/// | --- | --- |
/// | 32 | **zu klein**: die Strukturrechnung ueber die vorgesehenen Systeme ergibt 33 als untere Schranke |
/// | 48 | **gewaehlt**: 2,7-facher Abstand zum gemessenen Maximum, 1,45-facher zur unteren Schranke |
/// | 64 | traegt ebenso, kostet aber ein Drittel mehr Zeilen im Cache je Durchlauf |
///
/// Der Abstand ist Absicht: Das Maximum von 18 gilt fuer die heutigen
/// Lastarten. Ein neuer Baustein verschiebt es, und der Rueckfall soll die
/// Ausnahme bleiben, nicht der Normalfall.
pub const FELD_KAPAZITAET: usize = 48;

/// Die offenen Ereignisse einer [`EventQueue`], gesichert für einen Prüfpunkt
/// der inkrementellen Auswertung (Issue #917).
///
/// Nur so viele Einträge wie offen sind, nicht das ganze Feld: Ein Kampf hat
/// gemessen höchstens 18 offene Ereignisse (siehe [`FELD_KAPAZITAET`]), das Feld
/// fasst 48.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schlangenstand {
    feld: Vec<(Schluessel, EventKind)>,
    halde: Vec<Event>,
    next_seq: u32,
}

impl Schlangenstand {
    /// Wie viele Ereignisse offen waren.
    #[must_use]
    pub fn len(&self) -> usize {
        self.feld.len() + self.halde.len()
    }

    /// Ob keines offen war.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.feld.is_empty() && self.halde.is_empty()
    }
}

/// Die Warteschlange: festes Feld, Halde nur als Rueckfall.
///
/// # Warum kein Halden-Baum mehr
///
/// Gemessen liegen hoechstens 18 Ereignisse gleichzeitig an. Ein
/// `BinaryHeap` zahlt dafuer `log n` Vertauschungen je Einfuegen und
/// Entnehmen, dazu Zeigerverfolgung — bei 16,1 Prozent der Laufzeit
/// (Profil aus #909). Ein linearer Suchlauf ueber 18 zusammenhaengende
/// `u64` ist auf heutiger Hardware schneller, weil er eine einzige
/// Cache-Zeile liest und nicht verzweigt.
///
/// # Und warum die Halde trotzdem bleibt
///
/// Als **Rueckfall** (AC4). Ein Lastprofil, das 48 Plaetze sprengt, darf
/// nicht abstuerzen und nicht stillschweigend Ereignisse verlieren — es muss
/// **dasselbe Ergebnis** liefern, nur langsamer. Ein Rueckfall, der nie
/// ausgeloest hat, ist nicht geprueft; [`EventQueue::ueberlaeufe`] zaehlt
/// mit, damit ein Prueffall das belegen kann.
#[derive(Debug)]
pub struct EventQueue {
    /// Die anstehenden Ereignisse, **unsortiert**.
    ///
    /// Unsortiert ist Absicht: Einfuegen kostet dann nichts (ans Ende), und
    /// das Entnehmen sucht das Minimum ohnehin in einem Durchlauf. Sortiert
    /// einzufuegen kostete im Mittel das halbe Feld an Verschiebungen.
    feld: [(Schluessel, EventKind); FELD_KAPAZITAET],
    /// Wie viele Plaetze des Feldes belegt sind.
    len: usize,
    /// Der Rueckfall, wenn das Feld voll ist.
    heap: BinaryHeap<Reverse<Event>>,
    next_seq: u32,
}

impl EventQueue {
    /// Legt eine Schlange mit Platz fuer `capacity` gleichzeitig offene
    /// Ereignisse an.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            feld: [(Schluessel(0), EventKind::Wakeup); FELD_KAPAZITAET],
            len: 0,
            // Der Rueckfall bleibt vorab angefordert: Er soll im Ueberlauf
            // keinen Speicher anfordern muessen (GAME-DESIGN 18.4).
            heap: BinaryHeap::with_capacity(capacity),
            next_seq: 0,
        }
    }

    /// Ob der Rueckfall gerade Ereignisse haelt.
    #[must_use]
    pub fn im_rueckfall(&self) -> bool {
        !self.heap.is_empty()
    }

    /// Plant ein Ereignis ein.
    ///
    /// Ins feste Feld, solange Platz ist; sonst in den Rueckfall. Die Ordnung
    /// bleibt in beiden Faellen dieselbe, weil [`EventQueue::pop`] das Minimum
    /// ueber **beide** nimmt.
    pub fn push(&mut self, time: Millis, kind: EventKind) {
        let seq = self.next_seq;
        self.next_seq += 1;

        if self.len < FELD_KAPAZITAET && seq <= SEQ_MAX {
            self.feld[self.len] = (Schluessel::neu(time, kind.rank(), seq), kind);
            self.len += 1;
            return;
        }

        // Rueckfall: Feld voll ODER seq sprengt sein Bitfeld. Der zweite Fall
        // ist der unwahrscheinlichere und wird trotzdem gefuehrt — ein
        // maskierter seq waere eine stille Umordnung, und die faende niemand.
        // Gezaehlt wird in `diagnostics`, NICHT hier: Diese Schlange wird je
        // Kampf geleert, ein Zaehler darin ueberlebte das Zuruecksetzen nicht
        // — und `reset_completeness` verbietet zu Recht, dass irgendein Feld
        // es doch tut. Der Rueckfall liesse sich dann nie nachweisen.
        crate::diagnostics::note_queue_overflow();
        self.heap.push(Reverse(Event { time, kind, seq }));
    }

    /// Der Platz des kleinsten Eintrags im Feld, falls es einen gibt.
    ///
    /// Ein linearer Durchlauf ueber `len` zusammenhaengende `u64` — genau
    /// das, was der feste Speicher billig macht.
    fn kleinster_im_feld(&self) -> Option<usize> {
        let mut beste: Option<usize> = None;
        for i in 0..self.len {
            match beste {
                Some(b) if self.feld[i].0 >= self.feld[b].0 => {}
                _ => beste = Some(i),
            }
        }
        beste
    }

    /// Zeitpunkt des naechsten faelligen Ereignisses, ohne es zu entnehmen.
    #[must_use]
    pub fn peek_time(&self) -> Option<Millis> {
        let aus_feld = self.kleinster_im_feld().map(|i| self.feld[i].0);
        let aus_heap = self
            .heap
            .peek()
            .map(|Reverse(e)| Schluessel::neu(e.time, e.kind.rank(), e.seq.min(SEQ_MAX)));
        match (aus_feld, aus_heap) {
            (Some(a), Some(b)) => Some(a.min(b).zeit()),
            (Some(a), None) => Some(a.zeit()),
            (None, Some(b)) => Some(b.zeit()),
            (None, None) => None,
        }
    }

    /// Entnimmt das naechste faellige Ereignis.
    ///
    /// **Das Minimum ueber Feld UND Rueckfall.** Beides getrennt zu leeren
    /// waere die stille Variante des Fehlers, den der Rueckfall verhindern
    /// soll: Die Reihenfolge stimmte dann nur innerhalb jeder Haelfte.
    pub fn pop(&mut self) -> Option<Event> {
        let aus_feld = self.kleinster_im_feld();
        let feld_schluessel = aus_feld.map(|i| self.feld[i].0);
        let heap_schluessel = self
            .heap
            .peek()
            .map(|Reverse(e)| Schluessel::neu(e.time, e.kind.rank(), e.seq.min(SEQ_MAX)));

        let nimm_aus_feld = match (feld_schluessel, heap_schluessel) {
            (Some(a), Some(b)) => a <= b,
            (Some(_), None) => true,
            _ => false,
        };

        if nimm_aus_feld {
            let i = aus_feld.expect("oben geprueft");
            let (schluessel, kind) = self.feld[i];
            // Die Luecke mit dem letzten Eintrag schliessen: Das Feld ist
            // unsortiert, also spielt die Stelle keine Rolle — und es spart
            // das Verschieben des Rests.
            self.len -= 1;
            self.feld[i] = self.feld[self.len];
            return Some(Event {
                time: schluessel.zeit(),
                kind,
                seq: (schluessel.0 & ((1u64 << SEQ_BITS) - 1)) as u32,
            });
        }
        self.heap.pop().map(|Reverse(e)| e)
    }

    /// Wie viele Ereignisse offen sind.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len + self.heap.len()
    }

    /// Ob die Schlange leer ist.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0 && self.heap.is_empty()
    }

    /// Platz, der ohne neue Speicheranforderung zur Verfuegung steht.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.heap.capacity()
    }

    /// Sichert die offenen Ereignisse in `ziel` (Issue #917).
    ///
    /// Gesichert wird, was die Reihenfolge bestimmt: die Schlüssel samt Art und
    /// die nächste Einfüge-Nummer. Die Lage im Feld gehört nicht dazu — das Feld
    /// ist unsortiert, [`EventQueue::pop`] nimmt das Minimum über eindeutige
    /// Schlüssel. Der Rückfall wird nach Schlüssel sortiert gesichert, damit zwei
    /// gleiche Schlangen auch gleiche Sicherungen ergeben.
    ///
    /// `ziel` wird überschrieben und behält seinen Speicher; in einer Folge von
    /// Sicherungen fordert das nur beim ersten Mal Speicher an.
    pub fn sichern(&self, ziel: &mut Schlangenstand) {
        ziel.feld.clear();
        ziel.feld.extend_from_slice(&self.feld[..self.len]);
        ziel.halde.clear();
        ziel.halde.extend(self.heap.iter().map(|Reverse(e)| *e));
        ziel.halde.sort_unstable();
        ziel.next_seq = self.next_seq;
    }

    /// Stellt die Schlange auf einen gesicherten Stand zurück (Issue #917).
    ///
    /// Die freien Plätze des Feldes werden geleert wie in
    /// [`EventQueue::clear`]; danach entnimmt [`EventQueue::pop`] dieselben
    /// Ereignisse in derselben Reihenfolge wie die gesicherte Schlange.
    pub fn laden(&mut self, stand: &Schlangenstand) {
        self.feld = [(Schluessel(0), EventKind::Wakeup); FELD_KAPAZITAET];
        self.feld[..stand.feld.len()].copy_from_slice(&stand.feld);
        self.len = stand.feld.len();
        self.heap.clear();
        for event in &stand.halde {
            self.heap.push(Reverse(*event));
        }
        self.next_seq = stand.next_seq;
    }

    /// Leert die Schlange, behaelt aber den Speicher.
    pub fn clear(&mut self) {
        // Das Feld wird ueberschrieben, nicht nur `len` genullt. Funktional
        // genuegte `len = 0` — aber `reset_completeness` prueft, dass nach
        // dem Zuruecksetzen KEIN Feld anders aussieht als bei einer frischen
        // Schlange, und dieser Waechter hat recht: Ein liegengebliebener
        // Zustand ist von einem echten Leck nicht zu unterscheiden.
        //
        // Kosten: 48 Schreibvorgaenge je KAMPF, nicht je Zeitschritt. Gegen
        // die 59 340 Zyklen eines Kampfes ist das nicht messbar.
        self.feld = [(Schluessel(0), EventKind::Wakeup); FELD_KAPAZITAET];
        self.len = 0;
        self.heap.clear();
        self.next_seq = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frueheste_zeit_kommt_zuerst() {
        let mut q = EventQueue::with_capacity(8);
        q.push(3000, EventKind::Wakeup);
        q.push(1000, EventKind::ResourceDecay);
        q.push(2000, EventKind::Wakeup);

        assert_eq!(q.pop().map(|e| e.time), Some(1000));
        assert_eq!(q.pop().map(|e| e.time), Some(2000));
        assert_eq!(q.pop().map(|e| e.time), Some(3000));
        assert!(q.pop().is_none());
    }

    #[test]
    fn bei_gleicher_zeit_entscheidet_der_rang() {
        let mut q = EventQueue::with_capacity(8);
        // Bewusst in der falschen Reihenfolge eingefuegt.
        q.push(2000, EventKind::Wakeup);
        q.push(2000, EventKind::ResourceDecay);
        q.push(
            2000,
            EventKind::DotTick {
                spell: 1,
                generation: 0,
            },
        );

        assert!(matches!(
            q.pop().map(|e| e.kind),
            Some(EventKind::DotTick { .. })
        ));
        assert_eq!(q.pop().map(|e| e.kind), Some(EventKind::ResourceDecay));
        assert_eq!(q.pop().map(|e| e.kind), Some(EventKind::Wakeup));
    }

    #[test]
    fn bei_gleichem_rang_entscheidet_die_einfuege_reihenfolge() {
        let mut q = EventQueue::with_capacity(8);
        q.push(
            500,
            EventKind::DotTick {
                spell: 7,
                generation: 0,
            },
        );
        q.push(
            500,
            EventKind::DotTick {
                spell: 9,
                generation: 0,
            },
        );

        assert_eq!(
            q.pop().map(|e| e.kind),
            Some(EventKind::DotTick {
                spell: 7,
                generation: 0
            })
        );
        assert_eq!(
            q.pop().map(|e| e.kind),
            Some(EventKind::DotTick {
                spell: 9,
                generation: 0
            })
        );
    }

    #[test]
    fn peek_time_entnimmt_nicht() {
        let mut q = EventQueue::with_capacity(4);
        q.push(1234, EventKind::Wakeup);
        assert_eq!(q.peek_time(), Some(1234));
        assert_eq!(q.peek_time(), Some(1234));
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn leeren_behaelt_den_speicher() {
        let mut q = EventQueue::with_capacity(64);
        let vorher = q.capacity();
        for t in 0..50 {
            q.push(t * 10, EventKind::Wakeup);
        }
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.capacity(), vorher, "clear darf nicht neu anfordern");
    }
}
