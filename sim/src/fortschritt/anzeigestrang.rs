//! Der Anzeigestrang: viermal je Sekunde eine Momentaufnahme, daraus beide
//! Ausgaben.
//!
//! Alles, was nicht Zählen ist — Ableitungen, Prozentwerte, Restzeit,
//! Formatierung, Schreiben —, passiert hier und nur hier. Der heiße Pfad sieht
//! von diesem Strang nichts außer, dass seine Cache-Zeile viermal je Sekunde
//! gelesen wird.

use std::io::{self, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::anzeige::{utc_iso8601, Anzeige, Ereignis, RechnerZeile};
use super::fenster::{Fenster, FENSTER};
use super::restzeit::{restzeit_aus, Kostenbild, Rechner, Restzeit, WIEDERHOLUNG_LOKAL};
use super::zaehlwerk::{Arbeitsmenge, Momentaufnahme, Zaehlwerk};
use crate::drosselung::RuntimeControl;
use crate::engpass::Erhebung;
use crate::kalibrierung::Arbeitsart;

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
    /// Die Engpasserhebung (#934) — `None`, wenn der Lauf keine angelegt hat.
    /// Dann steht in der Zeile „Engpass", dass nichts erhoben wird.
    pub engpass: Option<Arc<Erhebung>>,
}

impl Laufbeschreibung {
    /// Eine Beschreibung mit Rechnerkennung, Laufkennung und erlaubten Kernen
    /// dieses Prozesses.
    ///
    /// Die Rechnerkennung liest dieselbe Stelle wie der Profilschlüssel der
    /// Fähigkeitserhebung — ein Rechner soll in Profil und Strom gleich heißen.
    #[must_use]
    pub fn neu(phase: &str, boss: Option<&str>, straenge: usize, kosten: Kostenbild) -> Self {
        let host_id = crate::kalibrierung::schluessel_erheben("").host;
        let start_ns = seit_epoche().as_nanos();
        Self {
            run_id: format!("{host_id}-{}-{start_ns}", std::process::id()),
            host_id,
            phase: phase.to_owned(),
            boss: boss.map(ToOwned::to_owned),
            straenge: straenge.max(1),
            straenge_erlaubt: crate::hostcaps::erlaubte_kerne().map(|k| k.len()),
            kosten,
            drosselung: None,
            wiederholung_p: WIEDERHOLUNG_LOKAL,
            engpass: None,
        }
    }

    /// Die Anzeige zu einem Fenster — rein, ohne Uhr: `seit_epoche` kommt
    /// herein, damit ein Prüffall sie festlegen kann.
    ///
    /// `None` nur bei einem leeren Fenster.
    #[must_use]
    pub fn anzeige(
        &self,
        fenster: &Fenster,
        ereignis: Ereignis,
        sequenz: u64,
        seit_epoche: Duration,
    ) -> Option<Anzeige> {
        let probe = fenster.letzte()?.clone();
        let raten = fenster.raten();
        // Über dasselbe Fenster wie `gleitend`: älteste gegen jüngste Probe.
        let engpass = self
            .engpass
            .as_ref()
            .and_then(|e| e.bild_im_fenster(fenster));
        let drosselung = self.drosselung.as_ref().map(|c| c.snapshot());
        // Die Stränge, die gerade rechnen DÜRFEN: Eine Steuerdatei mit
        // `cpu_threads: 2` lässt von acht Strängen zwei arbeiten.
        let wirksam = drosselung.map_or(self.straenge, |d| {
            if d.settings.paused {
                0
            } else {
                d.settings.cpu_threads.min(self.straenge)
            }
        });
        let restzeit = if ereignis == Ereignis::Ende {
            Restzeit::Fertig
        } else if matches!(probe.stand.menge, Arbeitsmenge::Offen) {
            // AC3 vor jeder Rechnung: Hier wird nicht einmal eine Rate gesucht.
            Restzeit::Plateau
        } else if wirksam == 0 {
            Restzeit::Unbestimmt("die Steuerung lässt keinen Strang rechnen")
        } else {
            match fenster.strangrate() {
                None => Restzeit::Unbestimmt(
                    "erst ab vier Messsekunden — darunter ist eine Streuung Zufall",
                ),
                Some(rate) => restzeit_aus(
                    &probe,
                    &self.kosten,
                    &[Rechner {
                        id: self.host_id.clone(),
                        straenge: wirksam,
                        raten: vec![(Arbeitsart::ScalarFullFight, rate)],
                        trend: fenster.strangtrend().unwrap_or(0.0),
                    }],
                    raten.as_ref(),
                    self.wiederholung_p,
                ),
            }
        };
        Some(Anzeige {
            ereignis,
            sequenz,
            utc: utc_iso8601(seit_epoche),
            phase: self.phase.clone(),
            boss: self.boss.clone(),
            raten,
            kosten: self.kosten,
            restzeit,
            rechner: vec![RechnerZeile {
                id: self.host_id.clone(),
                straenge: self.straenge,
                aktiv: raten.map_or(0, |r| r.aktive_straenge),
                erlaubt: self.straenge_erlaubt,
            }],
            drosselung,
            engpass,
            probe,
        })
    }

    /// Eine Momentaufnahme des Zählwerks, ergänzt um das, was nur die
    /// Engpasserhebung liest — der eine Ort, an dem eine Probe entsteht.
    fn probe(&self, werk: &Zaehlwerk) -> Momentaufnahme {
        let mut probe = werk.momentaufnahme();
        if let Some(e) = &self.engpass {
            e.ergaenzen(&mut probe.engpass);
        }
        probe
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
    ///
    /// # Panics
    ///
    /// Wenn das Betriebssystem keinen Strang mehr hergibt — dann läuft auch der
    /// Lauf selbst nicht.
    #[must_use]
    pub fn starten(werk: Arc<Zaehlwerk>, beschreibung: Laufbeschreibung, ausgabe: Ausgabe) -> Self {
        let (halt, empfang) = mpsc::channel();
        let griff = std::thread::Builder::new()
            .name("rota-anzeige".to_owned())
            .spawn(move || takten(&werk, &beschreibung, ausgabe, &empfang))
            .expect("der Anzeigestrang ließ sich nicht starten");
        Self { halt, griff }
    }

    /// Hält den Strang an, schreibt die Meldung `end` und gibt sie zurück.
    ///
    /// # Errors
    ///
    /// Ein Schreibfehler einer der beiden Ausgaben, oder ein abgestürzter
    /// Anzeigestrang.
    pub fn beenden(self) -> io::Result<Anzeige> {
        // Ist der Strang schon an einem Schreibfehler geendet, gibt es keinen
        // Empfänger mehr — sein Fehler kommt dann über `join`.
        let _ = self.halt.send(());
        self.griff
            .join()
            .map_err(|_| io::Error::other("der Anzeigestrang ist abgestürzt"))?
    }
}

/// Der Rumpf des Anzeigestrangs.
///
/// # Warum der Abstand ab der letzten Aufnahme zählt und nicht ab dem Start
///
/// Ein fester Raster (`Start + k · 250 ms`) holt einen verspäteten Takt nach,
/// und zwei Meldungen lägen dann dichter als 250 ms. Gemessen wird ab der
/// letzten Momentaufnahme: Zwischen zwei Meldungen liegen nie weniger als
/// [`ANZEIGETAKT`], und die Verarbeitung eines Takts verschiebt den nächsten
/// nicht, weil sie von der Wartezeit abgezogen wird.
fn takten(
    werk: &Zaehlwerk,
    beschreibung: &Laufbeschreibung,
    mut ausgabe: Ausgabe,
    halt: &mpsc::Receiver<()>,
) -> io::Result<Anzeige> {
    let mut fenster = Fenster::neu(FENSTER);
    let mut sequenz = 0;
    let mut zeilen_vorher = 0;
    let mut zeile_geschrieben: Option<Instant> = None;

    let mut aufgenommen = Instant::now();
    fenster.aufnehmen(beschreibung.probe(werk));
    sequenz += 1;
    if let Some(a) = beschreibung.anzeige(&fenster, Ereignis::Start, sequenz, seit_epoche()) {
        schreibe_strom(&mut ausgabe, &a, beschreibung)?;
    }

    loop {
        let warten = ANZEIGETAKT.saturating_sub(aufgenommen.elapsed());
        match halt.recv_timeout(warten) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        aufgenommen = Instant::now();
        fenster.aufnehmen(beschreibung.probe(werk));
        sequenz += 1;
        let Some(a) = beschreibung.anzeige(&fenster, Ereignis::Fortschritt, sequenz, seit_epoche())
        else {
            continue;
        };
        schreibe_strom(&mut ausgabe, &a, beschreibung)?;
        let faellig =
            ausgabe.terminal || zeile_geschrieben.is_none_or(|t| t.elapsed() >= ZEILENABSTAND);
        if faellig {
            zeilen_vorher = schreibe_zeile(&mut ausgabe, &a, zeilen_vorher)?;
            zeile_geschrieben = Some(Instant::now());
        }
    }

    fenster.aufnehmen(beschreibung.probe(werk));
    sequenz += 1;
    let ende = beschreibung
        .anzeige(&fenster, Ereignis::Ende, sequenz, seit_epoche())
        .ok_or_else(|| io::Error::other("das Fenster ist leer"))?;
    schreibe_strom(&mut ausgabe, &ende, beschreibung)?;
    schreibe_zeile(&mut ausgabe, &ende, zeilen_vorher)?;
    Ok(ende)
}

/// Eine Meldung in den JSON-Strom — und sofort hinaus, damit ein Leser mit
/// `tail -f` sie sieht, solange der Lauf noch rechnet.
fn schreibe_strom(
    ausgabe: &mut Ausgabe,
    anzeige: &Anzeige,
    beschreibung: &Laufbeschreibung,
) -> io::Result<()> {
    if let Some(strom) = &mut ausgabe.jsonl {
        writeln!(
            strom,
            "{}",
            anzeige.jsonl(&beschreibung.run_id, &beschreibung.host_id)
        )?;
        strom.flush()?;
    }
    Ok(())
}

/// Der Block für Menschen. Im Terminal überschreibt er den vorigen: Cursor um
/// dessen Zeilenzahl zurück an den Zeilenanfang, Rest des Bildschirms löschen.
fn schreibe_zeile(ausgabe: &mut Ausgabe, anzeige: &Anzeige, vorher: usize) -> io::Result<usize> {
    let Some(ziel) = &mut ausgabe.zeile else {
        return Ok(vorher);
    };
    let zeilen = anzeige.zeilen();
    let mut text = String::new();
    if ausgabe.terminal && vorher > 0 {
        text.push_str(&format!("\x1b[{vorher}F\x1b[J"));
    }
    for z in &zeilen {
        text.push_str(z);
        text.push('\n');
    }
    if !ausgabe.terminal {
        text.push('\n');
    }
    ziel.write_all(text.as_bytes())?;
    ziel.flush()?;
    Ok(zeilen.len())
}

fn seit_epoche() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
}
