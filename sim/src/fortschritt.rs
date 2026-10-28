//! Fortschritt, zwei Durchsatzzahlen und eine ehrliche Restzeit (Issue #933, E15.2).
//!
//! # Was dieses Modul leistet
//!
//! Ein Lauf gab bis hierher nichts aus, bis er fertig war. Bei Sekunden ist das
//! gleichgültig; bei einer Orakel-Tabelle mit hunderttausenden Zellen weiß
//! niemand, ob der Lauf arbeitet, hängt oder gleich fertig ist.
//!
//! ```text
//!   heißer Pfad      ein AtomicU64 je Strang, eigene Cache-Zeile   (zaehlwerk)
//!        │
//!   Anzeigestrang    viermal je Sekunde eine Momentaufnahme         (anzeigestrang)
//!        │
//!   Fenster          die letzten 30 Sekunden, nicht der ganze Lauf  (fenster)
//!        │
//!   Restzeit         Ablaufsimulation und hergeleitete Spanne       (restzeit)
//!        │
//!   Anzeige ──────── Zeile für Menschen  ·  JSON Lines für alles andere  (anzeige)
//! ```
//!
//! # Die eine Regel, an der alles hängt
//!
//! **Zwei Ausgabewege, EIN Datenstrom.** Beide Ausgaben entstehen aus derselben
//! [`Anzeige`], und die entsteht aus derselben [`Momentaufnahme`] desselben
//! [`Zaehlwerk`]s. Es gibt keinen zweiten Zähler — auch nicht im Programm
//! `rota-sim`, das bis zu diesem Issue einen eigenen führte und damit die
//! *angeforderten* Kämpfe zählte, nicht die gerechneten (siehe dort).
//!
//! # Was hier NICHT steht
//!
//! **Keine Engpassdiagnose.** Die Zeile „Engpass" sagt bis E15.3 (#934)
//! ausdrücklich „keine Diagnose". Eine geratene Ursache wäre schlimmer als
//! keine: Sie schickt den nächsten Umbau an die falsche Stelle.
//!
//! **Keine Restzeit für eine offene Suche.** Das ist eine Zusicherung, keine
//! Lücke — [`Arbeitsmenge::Offen`] kennt keine Restmenge, und eine Zahl dort wäre
//! erfunden. Angezeigt wird stattdessen, seit wann das Beste unverändert ist.

mod anzeige;
mod anzeigestrang;
mod fenster;
mod restzeit;
mod zaehlwerk;

pub use anzeige::{format_ganzzahl, utc_iso8601, Anzeige, Ereignis, RechnerZeile};
pub use anzeigestrang::{Anzeigestrang, Ausgabe, Laufbeschreibung, ANZEIGETAKT, ZEILENABSTAND};
pub use fenster::{Fenster, Raten, FENSTER};
pub use restzeit::{
    ablauf, restzeit, restzeit_aus, Ablauf, Ablauffehler, Herleitung, Kampfzahl, Kostenbild,
    Rechner, Restanteil, Restkosten, Restzeit, Stufe, WIEDERHOLUNG_LOKAL,
};
pub use zaehlwerk::{
    bestes, kandidat, menge, runde_ergaenzt, strang, suchrunde, Anmeldung, Arbeitsmenge, Bestes,
    Laufstand, Momentaufnahme, Strang, Strangzaehler, Weg, Zaehlwerk, CACHE_ZEILE,
    WEGE_OHNE_LAUF,
};
