//! Zaehler fuer den Diagnosepfad — Schlangenlaenge und `expire_buffs` (#896).
//!
//! # Warum diese beiden
//!
//! Die Kernel-Aufteilung des Simulators (Stand 2026-08-16) sieht so aus:
//!
//! ```text
//!   expire_buffs    23,8 %
//!   BinaryHeap      16,1 %
//!   APL-Auswertung   4,1 %
//! ```
//!
//! Beide grossen Posten sind bislang **unbeobachtet**. Von `expire_buffs` weiss
//! niemand, wie oft ein Aufruf ueberhaupt etwas abraeumt — laeuft er meist
//! vergeblich, ist die Zahl ein Optimierungshinweis; raeumt er fast immer,
//! ist sie es nicht. Von der Ereignisschlange weiss niemand, wie lang sie
//! wird; die Halden-Operationen kosten logarithmisch in dieser Laenge.
//!
//! Dieses Modul misst beides und behauptet nichts darueber, was man daraus
//! machen sollte. #896 misst, es optimiert nicht.
//!
//! # Warum die Auswertung NICHT hinter dem Merkmalsschalter steht
//!
//! Gegatet sind allein die **Aufrufstellen** in [`crate::fight`]
//! (`#[cfg(feature = "diagnostics")]`). Der Code hier ist immer uebersetzt,
//! und zwar aus einem gemessenen Grund: Die CI faehrt `cargo test` **ohne**
//! Merkmale (`.github/workflows/ci.yml`). Stuende die Auswertung hinter dem
//! Schalter, liefe kein einziger ihrer Prueffaelle in der CI — und ein
//! Perzentil, das niemand nachrechnet, ist eine Zahl ohne Deckung.
//!
//! Kosten hat das keine: Ohne den Schalter ruft niemand [`note_queue_len`]
//! oder [`note_expire`], und die Zaehler bleiben, was sie beim Start waren.
//!
//! # Warum ein Histogramm und keine Liste
//!
//! GAME-DESIGN 18.4: *"Keine Speicheranforderung im Schritt."* Eine Liste der
//! Stichproben waere genau das — bei 59 340 Zyklen je Kampf und Hunderttausenden
//! Kaempfen je Sekunde ausserdem unbezahlbar. Das Histogramm ist ein festes
//! Feld: Eintragen kostet einen Zugriff, und die Perzentile lassen sich daraus
//! **exakt** ausrechnen, solange kein Wert ueberlaeuft.
//!
//! Der Ueberlauf ist mitgezaehlt und wird mitgemeldet, statt still gedeckelt zu
//! werden: Eine Schlange laenger als [`crate::fight`]s `EVENT_CAPACITY` waere
//! selbst ein Befund — sie muesste nachfordern, und das verletzt dieselbe Regel.

use std::cell::{Cell, RefCell};

/// Groesster Schlangenstand mit eigenem Eimer.
///
/// Deckungsgleich mit `EVENT_CAPACITY` in [`crate::fight`]: Ab dieser Laenge
/// muesste die Schlange nachfordern, der Wert ist also die natuerliche Grenze.
pub const QUEUE_BUCKETS: usize = 257;

/// Ein Satz Zaehler.
///
/// [`Default`] ist der Nullstand: keine Stichprobe, kein Aufruf. Das ist
/// wichtig fuer die Auswertung — [`Counters::samples`] von 0 heisst
/// **nicht gemessen** und darf nie als „Laenge 0" gelesen werden.
#[derive(Debug, Clone)]
pub struct Counters {
    /// Wie oft jede Laenge beobachtet wurde, Index gleich Laenge.
    buckets: Box<[u64; QUEUE_BUCKETS]>,
    /// Beobachtungen jenseits von [`QUEUE_BUCKETS`].
    overflow: u64,
    /// Groesste beobachtete Laenge — exakt, auch im Ueberlauf.
    max: usize,
    /// Wie viele Stichproben insgesamt genommen wurden.
    samples: u64,
    /// Aufrufe von `expire_buffs`.
    expire_calls: u64,
    /// Aufrufe, die mindestens eine Verstaerkung abgeraeumt haben.
    expire_hits: u64,
    /// Vollstaendig durchgerechnete Kaempfe.
    fights: u64,
    /// Wie oft die Ereignisschlange in den Rueckfall ausgewichen ist (#909).
    ///
    /// **Hier und nicht in der Schlange selbst.** Sie wird je Kampf geleert;
    /// ein Zaehler darin ueberlebte das Zuruecksetzen nicht — und
    /// `reset_completeness` verbietet zu Recht, dass irgendein Feld es doch
    /// tut. Der Rueckfall laesst sich dann aber nie nachweisen, denn genau
    /// darum geht es in AC4.
    queue_overflows: u64,
    /// Fortlaufende Pruefsumme ueber alle Zustaende, die gemeldet wurden.
    trace: u32,
    /// Wie viele Zustaende in [`Counters::trace`] eingegangen sind.
    trace_states: u64,
    /// Neubestimmungen einer Ausruestungs-Auspraegung (Issue #908, AC6).
    gear_budgets: u64,
}

impl Default for Counters {
    fn default() -> Self {
        Self {
            buckets: Box::new([0; QUEUE_BUCKETS]),
            overflow: 0,
            max: 0,
            samples: 0,
            expire_calls: 0,
            expire_hits: 0,
            fights: 0,
            queue_overflows: 0,
            trace: crate::checksum::FNV_OFFSET,
            trace_states: 0,
            gear_budgets: 0,
        }
    }
}

impl Counters {
    /// Wie viele Stichproben der Schlangenlaenge genommen wurden.
    ///
    /// **Die Zusicherung des ganzen Datensatzes.** Steht hier 0, hat der Lauf
    /// die Schlange nie angesehen — dann ist jede Zahl daneben bedeutungslos,
    /// und der Bericht muss BLOCKED melden statt einer Null.
    #[must_use]
    pub const fn samples(&self) -> u64 {
        self.samples
    }

    /// Groesste beobachtete Schlangenlaenge.
    #[must_use]
    pub const fn max(&self) -> usize {
        self.max
    }

    /// Beobachtungen jenseits des Eimerbereichs.
    ///
    /// Ist das nicht 0, sind die Perzentile nur noch Untergrenzen — und die
    /// Schlange hat waehrend des Laufs Speicher nachgefordert.
    #[must_use]
    pub const fn overflow(&self) -> u64 {
        self.overflow
    }

    /// Aufrufe von `expire_buffs`.
    #[must_use]
    pub const fn expire_calls(&self) -> u64 {
        self.expire_calls
    }

    /// Aufrufe von `expire_buffs`, die etwas abgeraeumt haben.
    #[must_use]
    pub const fn expire_hits(&self) -> u64 {
        self.expire_hits
    }

    /// Neubestimmungen einer Ausruestungs-Auspraegung (Issue #908, AC6).
    ///
    /// **Null heisst hier etwas anderes als bei den uebrigen Zaehlern.** Bei
    /// [`Counters::samples`] belegt eine Null, dass nicht gemessen wurde. Hier
    /// ist Null das Ziel: Wer die Auspraegungen nachschlaegt, bestimmt sie
    /// nicht neu. Damit die Null etwas belegt, muss derselbe Zaehler an einer
    /// anderen Stelle nachweislich **hochgehen** — sonst ist sie nur die
    /// Abwesenheit einer Messung.
    #[must_use]
    pub const fn gear_budgets(&self) -> u64 {
        self.gear_budgets
    }

    /// Vollstaendig durchgerechnete Kaempfe.
    ///
    /// # Wozu, wenn der Laeufer doch selbst weiss, was er anstoesst
    ///
    /// Er weiss es **nicht**. Eine Suche entscheidet unterwegs, wie viele
    /// Kandidaten sie noch bewertet; die Zahl der Kaempfe steht erst hinterher
    /// fest. Und sie ausrechnen zu wollen (`Kandidaten * iterations`) hiesse,
    /// die Formel neben der Sache nachzubauen — der Nachbau antwortet dann
    /// plausibel aus einer zweiten Quelle, auch wenn er falsch liegt.
    ///
    /// Weil der Simulator deterministisch ist (GAME-DESIGN 18.4), gilt die in
    /// einem Kalibrierlauf gemessene Zahl auch fuer den Durchsatzlauf mit
    /// ausgeschalteter Erfassung — dieselbe Konfiguration, dieselben Kaempfe.
    #[must_use]
    pub const fn fights(&self) -> u64 {
        self.fights
    }

    /// Wie oft die Ereignisschlange in den Rueckfall ausgewichen ist (#909).
    ///
    /// **Die Messgroesse fuer AC4.** Bei normalen Bossen muss sie 0 sein —
    /// sonst belegt ihr Ausschlagen unter einem Lastprofil nichts.
    #[must_use]
    pub const fn queue_overflows(&self) -> u64 {
        self.queue_overflows
    }

    /// Der Trace-Hash ueber alle gemeldeten Zustaende.
    ///
    /// # Warum genau diese Verknuepfung
    ///
    /// `hash * 31 + zustand` ist **nicht frei gewaehlt** — es ist Zeichen fuer
    /// Zeichen dieselbe Zeile, die der Abspiel-Pfad in [`crate::fight`] fuehrt
    /// (`sequence = sequence.wrapping_mul(31).wrapping_add(hash)`), und der
    /// Zustand kommt aus `Fight::checksum_at`, also aus [`crate::checksum`].
    ///
    /// Ein eigener Hash-Mechanismus waere eine **zweite Wahrheit**: Zwei
    /// Verfahren, die dasselbe zu messen behaupten, weichen irgendwann ab, und
    /// dann sucht jemand einen Fehler in den Spielregeln, der in der Messung
    /// liegt.
    ///
    /// Der Anfangswert ist `FNV_OFFSET`, damit „nichts gemeldet" einen
    /// erkennbaren Wert hat und nicht 0 — 0 waere von einem echten Hash nicht
    /// zu unterscheiden. Wer sichergehen will, prueft
    /// [`Counters::trace_states`].
    #[must_use]
    pub const fn trace(&self) -> u32 {
        self.trace
    }

    /// Wie viele Zustaende in den Trace-Hash eingegangen sind.
    ///
    /// **Die Zusicherung zum Hash.** Ein Hash ueber null Zustaende ist eine
    /// Zahl wie jede andere; erst diese Zahl belegt, dass er etwas gesehen hat.
    #[must_use]
    pub const fn trace_states(&self) -> u64 {
        self.trace_states
    }

    /// Anteil der Aufrufe, die etwas abgeraeumt haben.
    ///
    /// `None` bei null Aufrufen — eine Quote ohne Nenner ist keine 0, sondern
    /// keine Aussage.
    #[must_use]
    pub fn expire_hit_rate(&self) -> Option<f64> {
        if self.expire_calls == 0 {
            return None;
        }
        #[allow(clippy::cast_precision_loss)]
        Some(self.expire_hits as f64 / self.expire_calls as f64)
    }

    /// Perzentil der Schlangenlaenge nach **nearest-rank**.
    ///
    /// Geliefert wird die kleinste Laenge, bei der der Anteil `p` der
    /// Stichproben erreicht ist. Der Rang ist `ceil(p * N)`, mindestens 1 —
    /// das ist die Definition, nach der p100 gleich [`Counters::max`] ist und
    /// p0 die kleinste beobachtete Laenge.
    ///
    /// **Aufrunden, nicht abrunden.** Bei `N = 10` und `p = 0,95` ist der Rang
    /// 10 und nicht 9; abgerundet laege das gemeldete p95 eine Stichprobe zu
    /// niedrig. Der Unterschied faellt genau dort auf, wo es zaehlt: am
    /// duennen Ende der Verteilung.
    ///
    /// `None`, solange keine Stichprobe vorliegt.
    #[must_use]
    pub fn percentile(&self, p: f64) -> Option<usize> {
        if self.samples == 0 {
            return None;
        }
        let anteil = p.clamp(0.0, 1.0);
        #[allow(clippy::cast_precision_loss)]
        let roh = anteil * self.samples as f64;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rang = (roh.ceil() as u64).max(1).min(self.samples);

        let mut gesehen = 0u64;
        for (laenge, anzahl) in self.buckets.iter().enumerate() {
            gesehen += anzahl;
            if gesehen >= rang {
                return Some(laenge);
            }
        }
        // Nur erreichbar, wenn der Rang im Ueberlauf liegt. Dann ist die
        // groesste beobachtete Laenge die beste verfuegbare Antwort — und
        // `overflow()` sagt dem Leser, dass sie eine Untergrenze ist.
        Some(self.max)
    }
}

thread_local! {
    /// Der schnelle Schalter — ein `Cell`, damit der heisse Pfad kein
    /// `RefCell` anfassen muss, solange die Erfassung aus ist.
    static ENABLED: Cell<bool> = const { Cell::new(false) };
    /// Der Schalter des Trace-Hashes, **getrennt** vom obigen.
    ///
    /// # Warum zwei Schalter und nicht einer
    ///
    /// Weil sie verschieden teuer sind. Die Zaehler kosten ein Inkrement je
    /// Takt. Der Trace-Hash kostet ein volles `Fight::checksum_at` — das
    /// mischt fuenf Werte **je Zauber** und laeuft ebenfalls je Takt.
    ///
    /// Gemessen am 2026-08-18: Mit einem gemeinsamen Schalter brauchte der
    /// Kalibrierlauf eines vollstaendigen Greedy-Laufs ueber zwoelf Minuten
    /// und war danach noch nicht fertig — bei 140 Prozent CPU, also
    /// einstraengig rechnend. Mit getrennten Schaltern kostet die Kalibrierung
    /// wieder das, was ein Suchlauf kostet.
    ///
    /// Sachlich gehoert es ohnehin getrennt: AC4 von #896 verlangt **je Boss**
    /// einen Trace-Hash, nicht je Suchlauf. Ein Kampf genuegt dafuer, und ein
    /// Kampf ist reproduzierbar.
    static TRACE_ENABLED: Cell<bool> = const { Cell::new(false) };
    static COUNTERS: RefCell<Counters> = RefCell::new(Counters::default());
}

/// Schaltet die Erfassung fuer den laufenden Strang ein.
///
/// **Je Strang getrennt**, wie [`crate::checksum`]s Aufzeichnung: Ein
/// Messlauf ueber sechzehn Straenge wuerde sich sonst gegenseitig in dieselben
/// Zaehler schreiben, und jede Zahl waere eine Summe unbekannter Herkunft.
pub fn enable() {
    ENABLED.with(|e| e.set(true));
}

/// Schaltet die Erfassung fuer den laufenden Strang aus.
pub fn disable() {
    ENABLED.with(|e| e.set(false));
}

/// Ob die Erfassung im laufenden Strang an ist.
#[must_use]
pub fn is_enabled() -> bool {
    ENABLED.with(Cell::get)
}

/// Schaltet den Trace-Hash fuer den laufenden Strang ein.
///
/// **Einstraengig benutzen.** Ueber mehrere Straenge haengt die Reihenfolge
/// der eingehenden Zustaende an der Einplanung des Betriebssystems, und ein
/// Hash, der bei jedem Lauf anders ausfaellt, prueft nichts.
pub fn enable_trace() {
    TRACE_ENABLED.with(|e| e.set(true));
}

/// Schaltet den Trace-Hash fuer den laufenden Strang aus.
pub fn disable_trace() {
    TRACE_ENABLED.with(|e| e.set(false));
}

/// Ob der Trace-Hash im laufenden Strang mitlaeuft.
#[must_use]
pub fn is_trace_enabled() -> bool {
    TRACE_ENABLED.with(Cell::get)
}

/// Setzt die Zaehler des laufenden Strangs auf null.
pub fn reset() {
    COUNTERS.with(|c| *c.borrow_mut() = Counters::default());
}

/// Gibt die Zaehler des laufenden Strangs heraus.
#[must_use]
pub fn snapshot() -> Counters {
    COUNTERS.with(|c| c.borrow().clone())
}

/// Ob dieser Bau die Aufrufstellen im Kampf enthaelt (#934).
///
/// Ohne das Merkmal ruft der Kampf weder [`note_queue_len`] noch
/// [`note_expire`] — dann sind Schlangenbelegung und Trefferquote der
/// Ablaufpruefung **nicht erhoben**, und das muss von „null" unterscheidbar
/// sein.
#[must_use]
pub const fn im_bau() -> bool {
    cfg!(feature = "diagnostics")
}

/// Rechnet die Zaehler eines anderen Strangs in die des laufenden ein (#934).
///
/// **Wozu.** Die Zaehler sind strang-lokal, und [`crate::balance`] legt je
/// Auswertung neue Arbeitsstraenge an. Ohne diese Vereinigung saehe ein
/// Leitstrang, der die Erfassung eingeschaltet hat, von einem mehrstraengigen
/// Lauf nichts — `examples/messumgebung.rs` kalibriert deshalb bis heute
/// einstraengig. `balance` reicht den Schalter an seine Arbeitsstraenge weiter
/// und vereinigt ihre Zaehler hier, nachdem sie fertig sind.
///
/// **Der Trace-Hash wird nicht vereinigt.** Er haengt an der Reihenfolge der
/// Zustaende, und ueber mehrere Straenge gibt es keine (siehe
/// [`enable_trace`]). Die Arbeitsstraenge bekommen seinen Schalter deshalb
/// nicht.
pub fn vereinigen(fremd: &Counters) {
    COUNTERS.with(|c| {
        let mut z = c.borrow_mut();
        for (eigen, anderer) in z.buckets.iter_mut().zip(fremd.buckets.iter()) {
            *eigen += anderer;
        }
        z.overflow += fremd.overflow;
        z.max = z.max.max(fremd.max);
        z.samples += fremd.samples;
        z.expire_calls += fremd.expire_calls;
        z.expire_hits += fremd.expire_hits;
        z.fights += fremd.fights;
        z.queue_overflows += fremd.queue_overflows;
        z.gear_budgets += fremd.gear_budgets;
    });
}

/// Die Kennzahlen, die die Engpasserkennung braucht (#934) — gelesen, ohne
/// das Histogramm zu kopieren.
#[must_use]
pub fn kampfzaehlung() -> crate::engpass::Kampfzaehlung {
    COUNTERS.with(|c| {
        let z = c.borrow();
        crate::engpass::Kampfzaehlung {
            schritte: z.samples,
            schlange_max: z.max,
            schlange_ueberlaeufe: z.queue_overflows,
            ablauf_aufrufe: z.expire_calls,
            ablauf_treffer: z.expire_hits,
        }
    })
}

/// Meldet, dass die Ereignisschlange in den Rueckfall ausgewichen ist (#909).
///
/// Anders als [`note_queue_len`] steht dieser Aufruf **nicht** hinter dem
/// Merkmalsschalter: Ein Ueberlauf ist kein Messwert, sondern ein Ereignis,
/// das auch in einem Lauf ohne Diagnose stattfindet — und dann will man
/// wissen, dass er stattgefunden hat. Ist die Erfassung aus, kostet der
/// Aufruf einen Vergleich.
pub fn note_queue_overflow() {
    if !is_enabled() {
        return;
    }
    COUNTERS.with(|c| c.borrow_mut().queue_overflows += 1);
}

/// Traegt eine beobachtete Schlangenlaenge ein.
///
/// Gerufen aus dem Kampf, hinter `#[cfg(feature = "diagnostics")]`. Ist die
/// Erfassung aus, kostet der Aufruf einen Vergleich.
pub fn note_queue_len(len: usize) {
    if !is_enabled() {
        return;
    }
    COUNTERS.with(|c| {
        let mut z = c.borrow_mut();
        z.samples += 1;
        if len > z.max {
            z.max = len;
        }
        if len < QUEUE_BUCKETS {
            z.buckets[len] += 1;
        } else {
            z.overflow += 1;
        }
    });
}

/// Traegt einen vollstaendig durchgerechneten Kampf ein.
pub fn note_fight() {
    if !is_enabled() {
        return;
    }
    COUNTERS.with(|c| c.borrow_mut().fights += 1);
}

/// Traegt eine **Neubestimmung** einer Ausruestungs-Auspraegung ein (#908, AC6).
///
/// Gezaehlt wird in [`crate::gear::budget_of`] — der Stelle, an der das Budget
/// einer Stufe in einer Schleife hochgerechnet wird, statt nachgeschlagen zu
/// werden. Ein Nachschlagen erhoeht den Zaehler nicht.
pub fn note_gear_budget() {
    if !is_enabled() {
        return;
    }
    COUNTERS.with(|c| c.borrow_mut().gear_budgets += 1);
}

/// Traegt einen Zustand in den Trace-Hash ein.
///
/// Der Aufrufer reicht `Fight::checksum_at(now)` herein — die Zustands-
/// Pruefsumme des Spiels, nicht etwas Nachgebautes.
///
/// **Die Erfassung muss der Aufrufer vorher pruefen.** `checksum_at` ist
/// teuer; als Argument ausgewertet, kostete es auch bei ausgeschalteter
/// Erfassung. Deshalb steht in `fight.rs` ein `if is_enabled()` davor und
/// nicht nur der Schalter hier drin.
pub fn note_state(state: u32) {
    if !is_trace_enabled() {
        return;
    }
    COUNTERS.with(|c| {
        let mut z = c.borrow_mut();
        z.trace = z.trace.wrapping_mul(31).wrapping_add(state);
        z.trace_states += 1;
    });
}

/// Traegt einen Aufruf von `expire_buffs` ein.
///
/// `hit` heisst: mindestens eine Verstaerkung ist abgelaufen.
pub fn note_expire(hit: bool) {
    if !is_enabled() {
        return;
    }
    COUNTERS.with(|c| {
        let mut z = c.borrow_mut();
        z.expire_calls += 1;
        if hit {
            z.expire_hits += 1;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Baut Zaehler aus einer Liste beobachteter Laengen — ohne den
    /// Merkmalsschalter, damit die Auswertung in der CI geprueft wird.
    fn aus_laengen(laengen: &[usize]) -> Counters {
        let mut z = Counters::default();
        for &l in laengen {
            z.samples += 1;
            if l > z.max {
                z.max = l;
            }
            if l < QUEUE_BUCKETS {
                z.buckets[l] += 1;
            } else {
                z.overflow += 1;
            }
        }
        z
    }

    #[test]
    fn ohne_stichprobe_gibt_es_kein_perzentil() {
        let leer = Counters::default();
        assert_eq!(leer.samples(), 0);
        assert_eq!(
            leer.percentile(0.5),
            None,
            "0 Stichproben sind keine Laenge 0"
        );
        assert_eq!(
            leer.expire_hit_rate(),
            None,
            "keine Aufrufe sind keine Quote 0"
        );
    }

    #[test]
    fn perzentile_nach_nearest_rank() {
        // Zehn Werte 1..=10. Der Rang zu p ist ceil(p*10).
        let z = aus_laengen(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(z.samples(), 10);
        assert_eq!(z.percentile(0.5), Some(5), "Rang 5");
        assert_eq!(z.percentile(0.95), Some(10), "Rang ceil(9,5) = 10");
        assert_eq!(z.percentile(0.99), Some(10), "Rang ceil(9,9) = 10");
        assert_eq!(z.percentile(1.0), Some(10));
        assert_eq!(z.percentile(0.0), Some(1), "Rang mindestens 1");
    }

    /// **Der Fall, an dem Auf- und Abrunden auseinandergehen.**
    ///
    /// Bei zwanzig Werten ist `0,95 * 20 = 19,0` — glatt. Bei einundzwanzig
    /// ist es `19,95`: aufgerundet Rang 20, abgerundet Rang 19. Genau ein
    /// Schritt Unterschied, und er faellt nur hier auf.
    #[test]
    fn aufrunden_nicht_abrunden() {
        let mut laengen: Vec<usize> = (1..=21).collect();
        laengen.sort_unstable();
        let z = aus_laengen(&laengen);
        assert_eq!(z.samples(), 21);
        assert_eq!(
            z.percentile(0.95),
            Some(20),
            "ceil(19,95) = 20; mit floor stuende hier 19"
        );
    }

    #[test]
    fn gleiche_laengen_ergeben_dieselbe_laenge() {
        let z = aus_laengen(&[7; 1000]);
        assert_eq!(z.percentile(0.5), Some(7));
        assert_eq!(z.percentile(0.99), Some(7));
        assert_eq!(z.max(), 7);
        assert_eq!(z.overflow(), 0);
    }

    #[test]
    fn ueberlauf_wird_gezaehlt_und_max_bleibt_exakt() {
        let z = aus_laengen(&[1, 2, QUEUE_BUCKETS + 5]);
        assert_eq!(z.overflow(), 1, "der Wert jenseits der Eimer");
        assert_eq!(
            z.max(),
            QUEUE_BUCKETS + 5,
            "max ist auch im Ueberlauf exakt"
        );
        assert_eq!(z.samples(), 3);
    }

    #[test]
    fn trefferquote_rechnet_gegen_die_aufrufe() {
        let z = Counters {
            expire_calls: 8,
            expire_hits: 2,
            ..Default::default()
        };
        assert!((z.expire_hit_rate().unwrap() - 0.25).abs() < f64::EPSILON);
    }

    /// Der Schalter ist je Strang getrennt — sonst summierten sich sechzehn
    /// Messstraenge in einen Zaehler.
    #[test]
    fn schalter_und_zaehler_gelten_je_strang() {
        reset();
        enable();
        note_queue_len(3);
        note_expire(true);
        note_expire(false);
        assert_eq!(snapshot().samples(), 1);
        assert_eq!(snapshot().expire_calls(), 2);
        assert_eq!(snapshot().expire_hits(), 1);

        let fremd = std::thread::spawn(|| {
            // Frischer Strang: Schalter aus, Zaehler leer.
            let vorher = is_enabled();
            note_queue_len(99);
            (vorher, snapshot().samples())
        })
        .join()
        .expect("Strang");
        assert!(!fremd.0, "der Schalter des Hauptstrangs gilt dort nicht");
        assert_eq!(fremd.1, 0, "und seine Zaehler auch nicht");

        disable();
        note_queue_len(4);
        assert_eq!(
            snapshot().samples(),
            1,
            "nach disable wird nichts mehr gezaehlt"
        );
        reset();
    }
}
