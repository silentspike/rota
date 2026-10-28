//! Engpasserkennung: eine Diagnose, keine Zahlenliste (Issue #934, E15.3).
//!
//! # Wofür
//!
//! Die Fortschrittsanzeige aus #933 sagt, **wie schnell** ein Lauf ist. Dieses
//! Modul beantwortet die Frage daneben: **wo lohnt der nächste Umbau?** Mehr
//! Messwerte helfen dabei nicht — die liefert `perf stat` auch. Gebraucht wird
//! ein Satz, der sagt, welche Ressource bremst und was als Nächstes zu tun ist.
//!
//! Und der eigentliche Zweck liegt eine Zeile tiefer: Jede Optimierung des
//! Plans beruht auf einer heute gemessenen Verteilung. Der Wächter ist das
//! Bauteil, das **meldet, wann eine dieser Annahmen abgelaufen ist** — die
//! Zeile [`Regel::AblaufAnnahme`].
//!
//! # Aufbau
//!
//! ```text
//!   Kernklasse ──► Adapter        welche Zählerkodierung was bedeutet   (kernklasse)
//!        │
//!   Leistungszähler               perf_event_open, ohne Kiste            (zaehler)
//!   Zählwerk aus #933             Arbeit gegen Warten je Strang, Wege
//!   diagnostics aus #896          Schlange, Ablaufprüfung
//!   /proc/pressure                Druckwerte
//!        │
//!   Rohstand je Momentaufnahme ──► Messwerte über das Fenster           (messwerte)
//!        │
//!   Grundlage desselben Rechners, derselben Kernklasse,                  (grundlage)
//!   desselben Lastprofils
//!        │
//!   Regeltabelle, eine reine Funktion ──► Diagnose als Satz              (regeln)
//! ```
//!
//! # Was hier NICHT steht
//!
//! - **Die Umbauten, die die Diagnose empfiehlt.** Sie nennt den nächsten
//!   Schritt, sie geht ihn nicht.
//! - **Die Erhebung der Grundlage über die acht Lastprofile, ihre Speicherung
//!   je Kombination und ihre Erneuerung bei geändertem Kernel.** Das ist
//!   E15.3b (#972). Hier gibt es genau eine Grundlage je Lauf, aus einer Datei,
//!   und die Prüfung, dass ihr Schlüssel zum Lauf passt.
//! - **Feste Schwellen für Zählerwerte.** „Befehlsdurchsatz unter 2,0" bedeutet
//!   auf Zen 3 und Zen 5 etwas Verschiedenes. Ohne passende Grundlage werden
//!   die Zählerzeilen nicht geprüft — sie fallen nicht auf eine Zahl zurück.
//!
//! **Eine Diagnose ist kein Beweis.** Sie sagt, wo sich das Nachsehen lohnt.
//!
//! Bedienung und Feldbeschreibung des Stroms: `docs/FORTSCHRITT.md`.

mod erhebung;
mod grundlage;
mod kernklasse;
mod messwerte;
mod regeln;
mod zaehler;

pub use erhebung::{Engpassbild, Erhebung};
pub use grundlage::{Grundlage, Grundlagenfehler, Schluessel, Vergleichswerte};
pub use kernklasse::{
    Adapter, Ereignis, Kernklasse, Prozessor, Quelle, Rolle, RUHENDE_KERNKLASSEN,
};
pub use messwerte::{
    abschnitt, Ablauftreffer, Druck, Kampfzaehlung, Messwerte, Netzwerte, Rohstand,
    Schlangenbelegung, Strangzeit, Wegtreffer, Zaehlerlage, Zaehlerwerte,
};
pub use regeln::{
    diagnose, Befund, Diagnose, Regel, ABLAUF_TREFFER_ANTEIL, DRITTE_EBENE_ANTEIL, DURCHSATZ_MINUS,
    FEHLSPRUENGE_PLUS, NETZ_WARTEN_ANTEIL, SCHLANGE_BELEGUNG_ANTEIL, SPEICHER_WARTEZEIT_PLUS,
    STRANG_WARTEN_ANTEIL, ZYKLEN_JE_EREIGNIS_PLUS,
};
pub use zaehler::{
    prozess_cpu_ns, strang_cpu_ns, Gruppenstand, Leistungszaehler, Sperre, Zaehlerstand,
};
