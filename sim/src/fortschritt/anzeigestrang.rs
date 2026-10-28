//! Der Anzeigestrang: viermal je Sekunde eine Momentaufnahme, daraus beide
//! Ausgaben.
//!
//! Alles, was nicht Zählen ist — Ableitungen, Prozentwerte, Restzeit,
//! Formatierung, Schreiben —, passiert hier und nur hier. Der heiße Pfad sieht
//! von diesem Strang nichts außer, dass seine Cache-Zeile viermal je Sekunde
//! gelesen wird.

use std::io::{self, Write};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use super::anzeige::{Anzeige, Ereignis};
use super::fenster::Fenster;
use super::restzeit::Kostenbild;
use super::zaehlwerk::Zaehlwerk;
use crate::drosselung::RuntimeControl;

/// Der Takt des Anzeigestrangs: viermal je Sekunde.
pub const ANZEIGETAKT: Duration = Duration::from_millis(250);

/// Wie oft die Zeile geschrieben wird, wenn sie in kein Terminal geht.
///
/// Im Terminal wird sie in jedem Takt überschrieben. In eine Datei umgeleitet,
/// wären das 14 400 Blöcke je Stunde — der JSON-Strom ist für die Maschine da,
/// die Zeile für einen Menschen, der in die Datei schaut.
pub const ZEILENABSTAND: Duration = Duration::from_secs(10);

/// Was über einen Lauf feststeht, bevor er beginnt.
#[derive(Debug, Clone)]
pub struct Laufbeschreibung {
    /// Eindeutig je Lauf: Rechner, Prozess, Startzeit.
    pub run_id: String,
    /// Der Rechner, auf dem gezählt wird.
    pub host_id: String,
    /// Was läuft: `greedy`, `fest`, `gestaffelt`, `bench` …
    pub phase: String,
    /// Gegen welchen Boss.
    pub boss: Option<String>,
    /// Arbeitsstränge des Laufs.
    pub straenge: usize,
    /// Kerne, die der Prozess benutzen darf.
    pub straenge_erlaubt: Option<usize>,
    /// Was eine Auswertung kostet.
    pub kosten: Kostenbild,
    /// Die Steuerung aus `--control-file`, falls es eine gibt.
    pub drosselung: Option<Arc<RuntimeControl>>,
    /// Wiederholungswahrscheinlichkeit je Aufgabe.
    pub wiederholung_p: f64,
}

impl Laufbeschreibung {
    /// Eine Beschreibung mit Rechnerkennung, Laufkennung und erlaubten Kernen
    /// dieses Prozesses.
    #[must_use]
    pub fn neu(phase: &str, boss: Option<&str>, straenge: usize, kosten: Kostenbild) -> Self {
        todo!("#933 {phase} {boss:?} {straenge} {kosten:?}")
    }

    /// Die Anzeige zu einem Fenster — rein, ohne Uhr: `seit_epoche` kommt
    /// herein, damit ein Prüffall sie festlegen kann.
    #[must_use]
    pub fn anzeige(
        &self,
        fenster: &Fenster,
        ereignis: Ereignis,
        sequenz: u64,
        seit_epoche: Duration,
    ) -> Option<Anzeige> {
        todo!("#933 {fenster:?} {ereignis:?} {sequenz} {seit_epoche:?}")
    }
}

/// Wohin die Anzeige schreibt.
pub struct Ausgabe {
    /// Die Zeile für Menschen, gewöhnlich der Fehlerkanal.
    pub zeile: Option<Box<dyn Write + Send>>,
    /// Ob `zeile` ein Terminal ist — dann wird überschrieben.
    pub terminal: bool,
    /// Der JSON-Strom, gewöhnlich eine Datei.
    pub jsonl: Option<Box<dyn Write + Send>>,
}

/// Der laufende Anzeigestrang.
pub struct Anzeigestrang {
    halt: mpsc::Sender<()>,
    griff: JoinHandle<io::Result<Anzeige>>,
}

impl Anzeigestrang {
    /// Startet den Strang. Er schreibt sofort eine Meldung `start`.
    #[must_use]
    pub fn starten(werk: Arc<Zaehlwerk>, beschreibung: Laufbeschreibung, ausgabe: Ausgabe) -> Self {
        todo!("#933 {werk:?} {beschreibung:?} {}", ausgabe.terminal)
    }

    /// Hält den Strang an, schreibt die Meldung `end` und gibt sie zurück.
    ///
    /// # Errors
    ///
    /// Ein Schreibfehler einer der beiden Ausgaben.
    pub fn beenden(self) -> io::Result<Anzeige> {
        let _ = self.halt.send(());
        todo!("#933 {:?}", self.griff.thread().id())
    }
}
