//! Die Dreiergruppen kommen über `greedy()` an — den Weg, den alle Aufrufer
//! benutzen (Issue #1053).
//!
//! # Die Lücke, die dieser Fall schließt
//!
//! Nach PR #1037 sind die Dreiergruppen der **Vorgabeweg**: 23 Aufrufstellen
//! erben sie, ohne dass jemand etwas umstellt. Geprüft wurde bis hierher nur
//! die Gruppen-Logik selbst — `sim/tests/tripel_suche.rs` ruft `tripel_zuege`
//! und `greedy_zuege` **direkt**.
//!
//! **Dass die Gruppen über `greedy()` ankommen, prüfte keiner.** Aufgefallen
//! ist es nur, weil jemand eine Mutation fuhr und sich wunderte, dass die
//! angekündigte Kontrolle grün blieb: `der_schalter_schaltet` ruft
//! `greedy_zuege` direkt und konnte eine Mutation in `greedy_opts` gar nicht
//! sehen.
//!
//! # Warum hier auf ÜBERLEBEN geprüft wird und nicht auf eine Zeile
//!
//! Der erste Entwurf verlangte, dass eine `NextStrikeWindow`-Zeile in der
//! gefundenen Rotation steht. Das prüft die **Darstellung** des Ergebnisses.
//! Geprüft gehört der **Ausgang**: Ein Ja/Nein kippt nicht, wenn jemand die
//! Kandidatenzeilen umbenennt oder anders zusammensetzt; eine Zahl mit knapper
//! Schwelle schon.
//!
//! Drei Wege, wie ein Fall aus dem falschen Grund rot würde — alle vermieden:
//! eine geprüfte **Zugfolge** statt des Ausgangs, eine **Zahl** mit knapper
//! Schwelle, und ein Schlag, der **knapp** überlebbar ist. Der Schlag hier ist
//! 18400 gegen eine gemessene Verteidigungs-Obergrenze von 9200 — Faktor zwei,
//! nicht knapp.

use rota_sim::apl::Apl;
use rota_sim::balance::BalanceConfig;
use rota_sim::data::boss_by_id;
use rota_sim::rotation_search::{all_candidates, greedy};
use rota_sim::{Fight, FightConfig};

/// Der Boss mit einem Schlag, den man verhindern **muss**.
const TOEDLICH: &str = "test-lethal-strike";

/// Sein Zwilling — dieselbe Datei, dieselbe Zeitleiste, Schlag auf 4600.
///
/// **Er ist die Kontrolle der MUTATION, nicht die des Prüffalls.** Wird er
/// unter derselben Mutation ebenfalls rot, misst der Fall nicht „die Gruppen
/// wenden den tödlichen Schlag ab", sondern „ohne Gruppen ist die Rotation
/// allgemein schlechter" — eine andere Aussage, und über `greedy()` nicht
/// spezifisch.
const UEBERLEBBAR: &str = "test-survivable-strike";

/// Wie lang die gesuchte Liste werden darf.
///
/// Eine Dreiergruppe braucht drei freie Plätze; bei einer engen Grenze griffe
/// `Stop::LengthLimit`, bevor sie zur Wahl steht.
const MAX_LEN: usize = 10;

/// Das Gewicht auf verhindertem Schaden.
///
/// **Größer als null, und das ist tragend**: Bei 0 ist PIN in jeder Runde der
/// schlechteste Zug (`baseDamage: 0` bei vollem GCD-Preis), und die Suche
/// *kann* ihn nicht wählen. Ein Fall bei Gewicht 0 prüfte die Gruppen nicht,
/// sondern die Gewichtung.
const GEWICHT: f64 = 1.0;

/// Sucht mit `greedy()` — dem Weg, den alle 23 Aufrufstellen benutzen — und
/// gibt zurück, ob die gefundene Rotation den Kampf überlebt.
///
/// Der zweite Rückgabewert ist die Länge der gefundenen Liste; sie trägt die
/// Zusicherung unten.
fn ueberlebt_mit_greedy(boss_id: &str) -> (bool, usize) {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("{boss_id} fehlt im Datensatz"));
    let config = BalanceConfig {
        boss,
        // **Ausdrücklich gesetzt.** Die Voreinstellung ist `true` und beendet
        // den Kampf, sobald der Boss fällt — der Schlag bei 12000 ms käme dann
        // je nach Rotation gar nicht vor, und der Fall prüfte, wie schnell
        // getötet wird.
        stop_on_kill: false,
        prevented_weight: GEWICHT,
        ..BalanceConfig::default()
    };

    let ergebnis = greedy(&config, &all_candidates(), MAX_LEN);
    let gefunden: Apl = ergebnis.best.apl.clone();

    // Die gefundene Liste in einem echten Kampf fahren. `greedy` misst über
    // mehrere Durchgänge und liefert einen Mittelwert; gefragt ist hier der
    // Ausgang, nicht die Zahl.
    //
    // **`stop_on_kill: false` MUSS hier genauso stehen wie in der Suche** —
    // der erste Entwurf hatte es vergessen und übernahm den Vorgabewert `true`.
    // Der Boss trägt 5200 Lebenspunkte; fällt er vor 12000 ms, endet der Kampf,
    // der Schlag kommt nie, und „überlebt" wäre **trivial wahr**. Der Fall wäre
    // grün gewesen, auch ohne die Dreiergruppen — richtig aus falschem Grund.
    //
    // Allgemeiner: Wer in einer anderen Umgebung PRÜFT als er GESUCHT hat,
    // misst nicht die gefundene Rotation, sondern eine andere Frage. Die
    // übrigen Felder stimmen (beide Vorgaben tragen `NEUTRAL`,
    // `WornValues::none()` und Talente auf 0) — dieses eine nicht.
    let mut fight = Fight::new(FightConfig {
        boss,
        stop_on_kill: false,
        ..FightConfig::default()
    });
    let _ = fight.run_apl(&gefunden);

    (!fight.player_dead(), ergebnis.chosen.len())
}

/// Über `greedy()` überlebt der Spieler den tödlichen Schlag.
///
/// # Was rot werden muss, und was grün bleiben muss
///
/// Die Mutation, gegen die dieser Fall gebaut ist, kommt von jemandem, der die
/// Gruppen-Logik nicht kennt und diesen Fall nicht gesehen hat: In `greedy()`
/// den Gruppen-Schalter beim Zusammenbauen der `greedy_opts`-Argumente **nicht
/// durchreichen** — `tripel_zuege` und `greedy_zuege` bleiben unberührt.
///
/// ```text
/// tödlicher Boss, mutiert     ROT     die Gruppen fehlen -> Tod
/// Zwilling, mutiert           GRÜN    kein Tod, nichts zu retten
/// die acht Fälle in tripel_suche.rs   GRÜN
/// ```
///
/// **Die naheliegende Mutation — `tripel_zuege` kaputtmachen — taugt nicht.**
/// Die acht bestehenden Fälle rufen die Gruppen-Logik direkt und würden
/// mitrot; ein neuer Fall, der darunter fällt, belegt nichts, was sie nicht
/// schon belegen. Erzeugt eine Mutation einen Zustand, den auch eine
/// **schwächere** Zusicherung erkannt hätte, misst sie die Zusicherung nicht.
#[test]
fn ueber_greedy_wendet_die_suche_den_toedlichen_schlag_ab() {
    let (ueberlebt, zeilen) = ueberlebt_mit_greedy(TOEDLICH);

    // **Trägerzeile.** Ohne sie wäre „überlebt nicht" auch bei einer leeren
    // Rotation erfüllt — und eine leere Rotation stirbt selbstverständlich.
    // Der Fall prüfte dann, dass die Suche überhaupt etwas findet.
    assert!(
        zeilen > 0,
        "greedy() hat gar keine Rotation gefunden — dann sagt das Ergebnis \
         unten nichts ueber die Dreiergruppen"
    );

    assert!(
        ueberlebt,
        "ueber greedy() ueberlebt der Spieler den Schlag nicht ({zeilen} Zeilen \
         gefunden). Der Schlag traegt 18400 gegen eine gemessene \
         Verteidigungs-Obergrenze von 9200 — abwendbar ist er nur, wenn PIN \
         ihn angreifbar macht und QUENCH ihn beendet, und dafuer braucht die \
         Suche die Dreiergruppen."
    );
}

/// Am Zwilling überlebt der Spieler ohnehin — die Kontrolle der Mutation.
///
/// # Warum dieser Fall hier steht und nicht in `tripel_suche.rs`
///
/// Er prüft nichts über die Gruppen. Er prüft, dass der Fall darüber etwas
/// über sie prüft: Fällt der Zwilling unter derselben Mutation ebenfalls, ist
/// die Aussage nicht „die Gruppen wenden den tödlichen Schlag ab", sondern
/// „ohne Gruppen ist die Rotation allgemein schlechter".
///
/// Beide Bosse unterscheiden sich in **genau einer Zahl** — 18400 gegen 4600,
/// bei gleicher Zeitleiste und gleichen Lebenspunkten.
#[test]
fn am_ueberlebbaren_zwilling_haengt_das_ueberleben_nicht_an_den_gruppen() {
    let (ueberlebt, zeilen) = ueberlebt_mit_greedy(UEBERLEBBAR);

    assert!(
        zeilen > 0,
        "greedy() hat am Zwilling gar keine Rotation gefunden — dann ist dieser \
         Fall als Kontrolle wertlos"
    );

    assert!(
        ueberlebt,
        "am Zwilling ueberlebt der Spieler nicht ({zeilen} Zeilen). Der Schlag \
         traegt dort 4600 gegen eine Obergrenze von 9200 — wer ihn nicht \
         ueberlebt, scheitert an etwas anderem als am Schlag, und dann ist die \
         Kontrolle fuer den Fall darueber nicht mehr gueltig."
    );
}
