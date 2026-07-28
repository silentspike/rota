//! Zeit — ganzzahlig, in Millisekunden.
//!
//! Die erste und folgenreichste Determinismus-Regel aus GAME-DESIGN 18.4:
//!
//! > Abklingzeit 8 s bei Tempo-Faktor 1,17 ergibt krumme Werte; Fliesskomma-Timer
//! > driften ueber hunderte Ticks.
//!
//! Deshalb gibt es in diesem Simulator **keinen einzigen Fliesskomma-Zeitwert**.
//! Wo in der Auswertung Sekunden gebraucht werden, entsteht die Umrechnung erst
//! bei der Ausgabe, nie in der Rechnung.

/// Kampfzeit in Millisekunden seit Kampfbeginn.
///
/// `u32` reicht fuer 49 Tage Kampfdauer und ist damit auch fuer die laengsten
/// Endgame-Kaempfe (GAME-DESIGN 10.1: bis ~10 Minuten) reichlich bemessen.
pub type Millis = u32;

/// Der Logik-Takt des Spiels (GAME-DESIGN 18.2, `STEP1-SPEC` § 2).
///
/// Der Simulator rechnet **ereignisgesteuert** und laeuft diesen Takt nicht ab —
/// er braucht ihn aber als Raster: Das Spiel kann nur zu Takt-Grenzen handeln,
/// und ein Simulator, der dazwischen wirkt, waere kein Abbild des Spiels mehr.
pub const TICK_MS: Millis = 10;

/// Die globale Wartezeit (GCD) aus `STEP1-SPEC` § 2.
///
/// Sie steht hier und nicht in `data/`, weil sie keine Eigenschaft eines Zaubers
/// ist, sondern eine Regel des Spiels. Sobald Tempo-Werte dazukommen
/// (ab Schritt 3), wird daraus ein gerechneter Wert — ganzzahlig, siehe
/// [`snap_up`].
pub const GCD_MS: Millis = 1500;

/// Rundet einen Zeitpunkt auf die naechste Takt-Grenze auf.
///
/// Das Spiel prueft seinen Zustand nur alle [`TICK_MS`]; eine Abklingzeit, die
/// rechnerisch bei 4507 ms endet, wird dort erst bei 4510 ms bemerkt. In
/// Schritt 1 faellt das nicht auf — alle Zeiten sind Vielfache von 500 ms.
/// **Ab Schritt 3 mit Tempo-Faktoren faellt es sofort auf**, und dann waere ein
/// Simulator ohne dieses Raster um bis zu einen Takt je Zauber zu schnell.
///
/// ```
/// use rota_sim::time::snap_up;
/// assert_eq!(snap_up(4500), 4500);
/// assert_eq!(snap_up(4501), 4510);
/// assert_eq!(snap_up(4509), 4510);
/// ```
#[must_use]
pub const fn snap_up(t: Millis) -> Millis {
    let rest = t % TICK_MS;
    if rest == 0 {
        t
    } else {
        t + (TICK_MS - rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takt_grenzen_bleiben_unveraendert() {
        for t in [0, TICK_MS, 1500, 40_500, 60_000] {
            assert_eq!(snap_up(t), t, "{t} liegt bereits auf einer Takt-Grenze");
        }
    }

    #[test]
    fn zwischenwerte_rutschen_nach_oben() {
        assert_eq!(snap_up(1), TICK_MS);
        assert_eq!(snap_up(9), TICK_MS);
        assert_eq!(snap_up(4507), 4510);
    }
}
