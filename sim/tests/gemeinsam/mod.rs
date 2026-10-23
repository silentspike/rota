//! Was die Suiten teilen, die das gebaute Programm aufrufen.
//!
//! # Warum die Pfade zur LAUFZEIT bestimmt werden
//!
//! `env!("CARGO_BIN_EXE_rota-sim")` und `env!("CARGO_MANIFEST_DIR")` werden zur
//! **Bauzeit** eingesetzt. Wird auf einem anderen Rechner gebaut als
//! ausgeführt — und genau das tut dieses Projekt: bauen auf rustbuild,
//! ausführen auf der Zielmaschine — stehen darin Pfade, die es hier nicht
//! gibt:
//!
//! ```text
//! /tmp/builds/9932248061915886481/target/debug/rota-sim
//!   liess sich nicht starten: No such file or directory (os error 2)
//! ```
//!
//! Am 2026-08-18 sind daran acht Suiten gescheitert, und keine davon hatte
//! einen Fehler im Code. Ein Symlink heilt es, aber er ist eine Zusatzbedingung
//! je Arbeitsverzeichnis — wer ihn nicht kennt, sucht den Fehler im Test.
//!
//! Diese Fassung braucht ihn nicht: Sie geht von `current_exe()` aus, und das
//! ist der Pfad, unter dem das Testbinary **wirklich** liegt.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Das gebaute `rota-sim`, gefunden ohne eingebrannte Pfade.
///
/// Ein Testbinary liegt in `target/<profil>/deps/`; das Programm liegt eine
/// Ebene darüber in `target/<profil>/`. Der Weg dorthin ist zwei Verzeichnisse
/// hoch und wieder hinein.
///
/// **Der eingebaute Pfad hat weiterhin Vorrang**, wenn es ihn gibt: Er ist der
/// von Cargo vorgesehene Weg und stimmt immer, wenn lokal gebaut wurde.
pub fn programm() -> PathBuf {
    let eingebaut = PathBuf::from(env!("CARGO_BIN_EXE_rota-sim"));
    if eingebaut.is_file() {
        return eingebaut;
    }
    let eigenes = std::env::current_exe().expect("das eigene Testbinary hat einen Pfad");
    let dateiname = if cfg!(windows) {
        "rota-sim.exe"
    } else {
        "rota-sim"
    };
    let ueber_deps = eigenes
        .parent()
        .and_then(Path::parent)
        .map(|p| p.join(dateiname));
    match ueber_deps {
        Some(pfad) if pfad.is_file() => pfad,
        // Gibt es beides nicht, ist der eingebaute Pfad die ehrlichere
        // Fehlermeldung — sie nennt, was gesucht wurde.
        _ => eingebaut,
    }
}

/// Das Wurzelverzeichnis des Projekts, ebenfalls ohne eingebrannte Pfade.
///
/// Gesucht wird vom Testbinary aus aufwärts nach dem Verzeichnis, das `sim/`
/// und `data/` enthält. Beide zusammen, nicht eines allein: `sim/` gibt es
/// auch im Bau-Verzeichnis.
pub fn wurzel() -> PathBuf {
    let eigenes = std::env::current_exe().expect("das eigene Testbinary hat einen Pfad");
    let mut kandidat: &Path = &eigenes;
    while let Some(eltern) = kandidat.parent() {
        if eltern.join("sim/Cargo.toml").is_file() && eltern.join("data").is_dir() {
            return eltern.to_path_buf();
        }
        kandidat = eltern;
    }
    // Rückfall auf den eingebauten Pfad. Auf dem Bau-Dienst (cargo remote v2)
    // ist das der Weg, der trägt: Dort liegt das Testbinary unter
    // /target/current/…, außerhalb der Quellkopie unter /workspace/ — die Suche
    // aufwärts findet nichts, `CARGO_MANIFEST_DIR` ist /workspace/sim (Pfade aus
    // dem v2-Protokoll vom 2026-10-23). Liegt ein Binary dagegen an einem Ort
    // ohne Quellen darüber und gibt es den eingebauten Pfad nicht, scheitert der
    // Test hier mit einer Meldung, die den gesuchten Ort nennt, statt mit einem
    // leeren Ergebnis.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Ruft das Programm und liefert (Rückgabewert, stdout, stderr).
pub fn rufe(argumente: &[&str]) -> (i32, String, String) {
    let programm = programm();
    let ausgabe = Command::new(&programm)
        .args(argumente)
        .current_dir(wurzel())
        .output()
        .unwrap_or_else(|fehler| {
            panic!(
                "{} liess sich nicht starten: {fehler}\n\
                 Gesucht wurde ausgehend von {:?}",
                programm.display(),
                std::env::current_exe()
            )
        });
    (
        ausgabe.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&ausgabe.stdout).into_owned(),
        String::from_utf8_lossy(&ausgabe.stderr).into_owned(),
    )
}
