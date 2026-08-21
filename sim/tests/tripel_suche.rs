//! Die kleinste nützliche Einheit hat drei Zeilen (Issue #797).
//!
//! # Der Mangel, gemessen
//!
//! #558 hat der Suche Paare beigebracht: zwei Zeilen, die einzeln verlieren
//! und zusammen tragen. Für PIN reicht das nicht, und der Grund steht in den
//! Zauberdaten.
//!
//! ```text
//! bind     erzeugt Kontroll-Punkte      allein: ein mittelmäßiger Füller
//! pin      macht den Schlag angreifbar  allein: NICHT WIRKBAR, kostet 3
//! quench   unterbricht ihn              allein: findet nie etwas Laufendes
//! ```
//!
//! **Auch kein Paar daraus trägt.** Ohne Erzeuger ist PIN nicht wirkbar; ohne
//! PIN hat QUENCH nichts zu unterbrechen; Erzeuger plus QUENCH ohne PIN ist
//! ein Füller neben einer toten Zeile.
//!
//! Gemessen vor diesem Bau, am tödlichen Boss über die Gewichte 0, 1, 5, 20
//! und 100: Die Suche wählt die PIN-Zeile bei **keinem**. Mit einem Pool aus
//! genau diesen vier Zeilen wählt sie sie — der Wert war also erreichbar, der
//! Weg dorthin fehlte.
//!
//! # Was hier NICHT behauptet wird
//!
//! Dass Dreiergruppen die Suche allgemein verbessern. Sie sind eine Stufe, die
//! genau dort greift, wo die billigeren nichts mehr finden, und die Prüffälle
//! messen ihre Wirkung an dem Boss, für den sie gebaut wurde — plus die
//! Gegenprobe, dass sie anderswo nichts verschiebt.

use rota_sim::balance::BalanceConfig;
use rota_sim::data::{boss_by_id, SPELLS};
use rota_sim::log::LogKind;
use rota_sim::rotation_search::{
    all_candidates, candidates_for, greedy_zuege, tripel_zuege, Candidate, Origin, Zuege,
};

/// Der Boss mit dem Schlag, den man verhindern **muss**.
const TOEDLICH: &str = "test-lethal-strike";

/// Sein Zwilling — dieselbe Datei, Schlag auf 4600 gesenkt.
const UEBERLEBBAR: &str = "test-survivable-strike";

/// Wie lang die gesuchte Liste werden darf.
///
/// **Nicht knapp gewählt.** Eine Dreiergruppe braucht drei freie Plätze; bei
/// einer engen Grenze griffe `Stop::LengthLimit`, bevor die Gruppe überhaupt
/// zur Wahl steht — und „kein Unterschied" hieße dann „nie gefragt".
const MAX_LEN: usize = 10;

fn config(boss_id: &str, gewicht: f64) -> BalanceConfig {
    let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("{boss_id} fehlt im Datensatz"));
    BalanceConfig {
        boss,
        // Ausdrücklich: Die Voreinstellung ist `true` und beendet den Kampf beim
        // Tod des Bosses — der Schlag bei 12000 käme dann je nach Rotation gar
        // nicht vor.
        stop_on_kill: false,
        prevented_weight: gewicht,
        ..BalanceConfig::default()
    }
}

fn zeilen(ergebnis: &rota_sim::rotation_search::SearchResult) -> Vec<String> {
    ergebnis
        .chosen
        .iter()
        .map(|k| format!("{} if {}", SPELLS[k.spell].id, k.condition))
        .collect()
}

fn waehlt_pin(ergebnis: &rota_sim::rotation_search::SearchResult) -> bool {
    ergebnis
        .chosen
        .iter()
        .any(|k| k.origin == Origin::NextStrikeWindow)
}

// ---------------------------------------------------------------------------
// Die Gruppe entsteht aus den Daten
// ---------------------------------------------------------------------------

/// Die Dreiergruppe wird gebildet — und zwar aus allen drei Bestandteilen.
///
/// # Warum das vor der Wirkung geprüft wird
///
/// „Die Suche wählt PIN nicht" hat drei mögliche Ursachen: Die Gruppe entsteht
/// nicht, sie entsteht und gewinnt nicht, oder die Längengrenze greift vorher.
/// Alle drei sehen im Ergebnis gleich aus. Beim Paar-Zug hat genau diese
/// Trennung zwei Stunden gespart.
#[test]
fn die_gruppe_entsteht_aus_den_daten() {
    let pool = all_candidates();
    let offen: Vec<usize> = (0..pool.len()).collect();
    let gruppen = tripel_zuege(&pool, &offen);

    assert!(
        !gruppen.is_empty(),
        "keine einzige Dreiergruppe aus {} Kandidatenzeilen — dann kann der \
         Zug nie greifen",
        pool.len()
    );

    let benannt: Vec<(&str, &str, &str)> = gruppen
        .iter()
        .map(|(zeile, unterbrecher, erzeuger)| {
            (
                SPELLS[pool[offen[*zeile]].spell].id,
                SPELLS[pool[offen[*unterbrecher]].spell].id,
                SPELLS[pool[offen[*erzeuger]].spell].id,
            )
        })
        .collect();

    assert!(
        benannt
            .iter()
            .any(|(zeile, unterbrecher, _)| *zeile == "pin" && *unterbrecher == "quench"),
        "die Gruppe aus PIN und QUENCH fehlt; gebildet wurden {benannt:?}"
    );

    // Jede Gruppe muss einen ECHTEN Erzeuger tragen. Ohne diese Zeile wäre
    // auch eine Gruppe grün, deren drittes Glied ein beliebiger Füller ist —
    // und dann wäre der Suchraum wieder der, den `tripel_zuege` vermeidet.
    for (zeile, unterbrecher, erzeuger_id) in &benannt {
        let erzeuger = SPELLS
            .iter()
            .find(|s| s.id == *erzeuger_id)
            .unwrap_or_else(|| panic!("{erzeuger_id} fehlt im Zauberdatensatz"));
        assert!(
            erzeuger.control_gain > 0,
            "die Gruppe ({zeile}, {unterbrecher}, {erzeuger_id}) traegt als \
             drittes Glied einen Zauber ohne control_gain — dann ist es kein \
             Erzeuger, sondern eine beliebige dritte Zeile"
        );
    }
}

/// Die Ränge im Block stimmen: die Zeile darum, dann der Unterbrecher, dann
/// der Erzeuger.
///
/// # Warum das keine Kosmetik ist
///
/// Eine Prioritätenliste wird bei **jeder** Entscheidung von oben gelesen. Die
/// unbedingte Zeile des Erzeugers trifft in jedem Takt, in dem seine
/// Abklingzeit gefallen ist — bei `bind` jeden GCD. Stünde sie oben, käme PIN
/// nie an die Reihe, und die Gruppe wäre aus demselben Grund tot, aus dem sie
/// gebaut wurde.
#[test]
fn die_raenge_im_block_stimmen() {
    let pool = all_candidates();
    let offen: Vec<usize> = (0..pool.len()).collect();
    let gruppen = tripel_zuege(&pool, &offen);

    // **Trägerzeile** (#1042): Ohne sie prüft eine leere Liste nullmal, und der
    // Fall wäre grün, ohne einen einzigen Rang angesehen zu haben. Eine Schleife
    // ist keine Zusicherung — sie ist eine Zusicherung **mal Anzahl**, und bei
    // null ist das Ergebnis wahr.
    assert!(
        !gruppen.is_empty(),
        "keine einzige Dreiergruppe aus {} Kandidatenzeilen — dann laeuft die \
         Schleife unten nullmal und dieser Fall prueft nichts",
        pool.len()
    );

    for (zeile_pos, unterbrecher_pos, erzeuger_pos) in gruppen {
        assert_eq!(
            pool[offen[zeile_pos]].origin,
            Origin::NextStrikeWindow,
            "das erste Glied muss die Zeile sein, um die es geht — gemessen an \
             test-metronome, siehe die_reihenfolge_im_block_ist_gemessen"
        );
        assert_eq!(
            pool[offen[unterbrecher_pos]].origin,
            Origin::BossCastWindow,
            "das zweite Glied muss der Unterbrecher sein"
        );
        assert_eq!(
            pool[offen[erzeuger_pos]].origin,
            Origin::Always,
            "das dritte Glied muss die unbedingte Zeile des Erzeugers sein — \
             eine bedingte koennte in dem Takt, in dem sie gebraucht wird, \
             nicht zutreffen"
        );
    }
}

/// Die Reihenfolge im Block ist **gemessen**, nicht abgeleitet.
///
/// # Die Ableitung, die falsch war
///
/// Zuerst stand PIN oben, mit einer Begründung, die zwingend klang: „PIN muss
/// fallen, BEVOR der Schlag kommt, also gehört er nach oben." Sie verwechselt
/// Reihenfolge in der **Zeit** mit Rang in der **Liste** — eine
/// Prioritätenliste ist keine Sequenz, sie wird bei jeder Entscheidung neu von
/// oben gelesen.
///
/// # Wie hier gemessen wird
///
/// Die Reihenfolge kommt aus [`tripel_zuege`] selbst, wird als Rotation
/// gespielt und gegen die vertauschte gefahren. Der Fall behauptet **nicht**,
/// welche richtig ist — er verlangt nur, dass die gebaute nicht schlechter ist
/// als die andere.
///
/// Damit kann er scheitern, ohne dass jemand die Erwartung nachzieht: Ist die
/// vertauschte besser, steht die Zahl in der Meldung, und der Code gehört
/// geändert, nicht der Test.
#[test]
fn die_reihenfolge_im_block_ist_gemessen() {
    use rota_sim::apl::{parse, AplName};
    use rota_sim::{Fight, FightConfig};

    let pool = all_candidates();
    let offen: Vec<usize> = (0..pool.len()).collect();
    let gruppen = tripel_zuege(&pool, &offen);
    // **Die erste Gruppe, nicht die einzige.** Es gibt eine je Erzeuger — heute
    // drei. Für die Frage nach der REIHENFOLGE genügt eine: Sie ist in allen
    // dieselbe, weil `tripel_zuege` sie fest vergibt.
    let (zeile_pos, unterbrecher_pos, erzeuger_pos) = *gruppen
        .first()
        .expect("ohne Gruppe misst dieser Fall nichts — siehe die_gruppe_entsteht_aus_den_daten");

    // **Die unbedingte Zeile hat KEINE Bedingung**, und das hat der erste
    // Entwurf nicht bedacht: `format!("{} if {}")` erzeugte für den Erzeuger
    // `bind if ` — der Parser weist das ab („Nach if fehlt die Bedingung"), und
    // der Fall brach ab, bevor er irgendetwas messen konnte.
    //
    // Gefunden hat es der Lauf, nicht das Lesen: In der Kandidatenliste steht
    // die Bedingung als leere Zeichenkette da, und `Origin::Always` heißt genau
    // das — es gibt keine.
    let als_text = |k: &Candidate| {
        let bedingung = k.condition.to_string();
        if bedingung.is_empty() {
            format!("{}\n", SPELLS[k.spell].id)
        } else {
            format!("{} if {bedingung}\n", SPELLS[k.spell].id)
        }
    };
    let erste = als_text(&pool[offen[zeile_pos]]);
    let zweite = als_text(&pool[offen[unterbrecher_pos]]);
    let dritte = als_text(&pool[offen[erzeuger_pos]]);

    // Ein Füller unten, damit kein Takt leer bleibt und beide Läufe dieselbe
    // GCD-Last tragen.
    let fueller = "wisp\n";

    /// Was am Ende des Kampfes vom Spieler übrig ist, und wie oft PIN geworfen
    /// wurde.
    ///
    /// **Der Name sagt jetzt, was zurückkommt.** Er hieß `erlittener_schaden`
    /// und lieferte `player_hp()` — also das Gegenteil: übriges Leben, bei dem
    /// MEHR besser ist. Der Kommentar daneben sagte „weniger ist besser". Die
    /// Zusicherung darunter war für Leben richtig geschrieben, sodass beides
    /// zusammen funktioniert hätte und niemanden gewarnt hätte.
    fn lauf(boss_id: &str, text: &str) -> (u32, usize) {
        let boss = boss_by_id(boss_id).unwrap_or_else(|| panic!("{boss_id} fehlt im Datensatz"));
        let apl = parse(AplName("reihenfolge"), text)
            .unwrap_or_else(|e| panic!("Liste ist ungueltig: {e:?}\n{text}"));
        let mut fight = Fight::new(FightConfig {
            boss,
            ..FightConfig::default()
        });
        let _ = fight.run_apl(&apl);

        // `pin_index`, nicht `pin`: Der Wert ist ein Platz im Zauberdatensatz,
        // kein Zauber. Derselbe Unterschied, an dem in dieser Datei schon
        // `erlittener_schaden` gescheitert ist.
        let pin_index = SPELLS
            .iter()
            .position(|s| s.id == "pin")
            .expect("pin fehlt im Zauberdatensatz");
        let pin_wuerfe = fight
            .log()
            .entries()
            .iter()
            .filter(|ev| ev.kind == LogKind::Cast && usize::from(ev.spell) == pin_index)
            .count();

        (fight.player_hp(), pin_wuerfe)
    }

    let gebaut = format!("{erste}{zweite}{dritte}{fueller}");
    let vertauscht = format!("{zweite}{erste}{dritte}{fueller}");

    // **Gemessen wird an einem Boss, an dem die Frage überhaupt entscheidbar
    // ist**, und das war der erste Entwurf nicht.
    //
    // Er fuhr `test-lethal-strike`: EIN Eintrag, keine Schleife. Ergebnis
    // 5000 gegen 5000 Leben, je zwei PIN-Würfe — Gleichstand. Und der ist
    // zwangsläufig: Die Lage, um die es geht — der Boss wirkt GERADE, während
    // der nächste Schlag schon nah ist —, kann dort gar nicht eintreten. Ein
    // Fall, der genau den Fall nicht enthält, für den er gebaut wurde.
    //
    // **Über MEHRERE entscheidbare Bosse, nicht über einen.** Die erste Fassung
    // fuhr `test-metronome` allein — eine Stichprobe von eins, und genau die
    // hatte mich vorher in die Irre geführt. Ein einzelner Boss kann eine
    // Zeitleiste haben, an der die Reihenfolge zufällig so ausgeht.
    //
    // Entscheidbar heißt: Schleife, mehr als ein Eintrag, unterbrechbarer Cast
    // UND Schlag. Nur dort überlappen sich `boss.casting` und ein naher
    // `boss.next_strike_in`.
    const ENTSCHEIDBAR: [&str; 4] = ["test-metronome", "boss-1-1", "boss-2-3", "boss-4-4"];

    let mut schlechter_bei: Vec<String> = Vec::new();
    let mut reihe: Vec<String> = Vec::new();
    for boss_id in ENTSCHEIDBAR {
        let (g, gp) = lauf(boss_id, &gebaut);
        let (v, vp) = lauf(boss_id, &vertauscht);
        reihe.push(format!("{boss_id}: gebaut {g}/{gp}, vertauscht {v}/{vp}"));
        if g < v {
            schlechter_bei.push(format!("{boss_id} ({g} gegen {v})"));
        }
    }
    println!("Reihenfolge je Boss (Leben/PIN-Wuerfe):");
    for z in &reihe {
        println!("    {z}");
    }

    assert!(
        schlechter_bei.is_empty(),
        "die gebaute Reihenfolge ist an diesen Bossen SCHLECHTER als die \
         vertauschte: {schlechter_bei:?}. Zu aendern ist tripel_zuege, nicht \
         dieser Fall.\n\ngebaut:\n{gebaut}\nvertauscht:\n{vertauscht}"
    );

    // Der tödliche Boss steht daneben, weil sein Gleichstand ein BEFUND ÜBER
    // DEN FALL ist und keiner über die Reihenfolge: EIN Eintrag, keine
    // Schleife, die Überlappung kann nicht eintreten.
    let (leben_toedlich_gebaut, pin_gebaut) = lauf(TOEDLICH, &gebaut);
    let (leben_toedlich_vertauscht, pin_vertauscht) = lauf(TOEDLICH, &vertauscht);
    let leben_gebaut = leben_toedlich_gebaut;
    let leben_vertauscht = leben_toedlich_vertauscht;
    println!(
        "Am toedlichen Boss (EIN Eintrag, keine Schleife): gebaut \
         {leben_toedlich_gebaut}, vertauscht {leben_toedlich_vertauscht} — dort \
         kann die Reihenfolge nichts bedeuten, und Gleichstand ist kein Befund."
    );

    // **Die Trägerzeile, ohne die der Vergleich nichts bedeutet.** Wird PIN in
    // keinem der beiden Läufe geworfen, sind beide Zahlen gleich, die
    // Zusicherung darunter ist grün, und gemessen wurde nichts. Das ist der
    // Prüffall, der nicht scheitern kann — hier fällt er auf.
    assert!(
        pin_gebaut > 0 || pin_vertauscht > 0,
        "PIN wurde in KEINEM der beiden Laeufe geworfen — dann vergleicht \
         dieser Fall zwei Rotationen, in denen der Zauber gar nicht vorkommt, \
         und seine Reihenfolge kann nichts bedeuten.\n\ngebaut:\n{gebaut}"
    );

    // **Die Zahlen werden ausgegeben, nicht nur verglichen.** Ein grüner Fall
    // heißt hier „nicht schlechter" — das kann BESSER oder GLEICH bedeuten, und
    // Gleichstand ist keine Bestätigung der Reihenfolge, sondern eine offene
    // Frage. Wer nur das `ok` liest, hält beides für dasselbe.
    //
    // Sichtbar mit `--nocapture`; im stillen Lauf kostet es nichts.
    println!(
        "Reihenfolge gemessen: gebaut {leben_gebaut} Leben / {pin_gebaut} PIN-Wuerfe, \
         vertauscht {leben_vertauscht} Leben / {pin_vertauscht} PIN-Wuerfe — {}",
        if leben_gebaut > leben_vertauscht {
            "die gebaute Ordnung ist BESSER"
        } else if leben_gebaut == leben_vertauscht {
            "GLEICHSTAND, die Frage bleibt offen"
        } else {
            "die gebaute Ordnung ist SCHLECHTER"
        }
    );

    assert!(
        leben_gebaut >= leben_vertauscht,
        "die vertauschte Reihenfolge laesst den Spieler mit MEHR Leben \
         zurueck ({leben_vertauscht} gegen {leben_gebaut}; PIN geworfen \
         {pin_vertauscht} gegen {pin_gebaut} mal) — dann steht der Block \
         falsch herum, und zu aendern ist tripel_zuege, nicht dieser \
         Fall.\n\ngebaut:\n{gebaut}\nvertauscht:\n{vertauscht}"
    );
}

// ---------------------------------------------------------------------------
// Die Wirkung, mit Gegenprobe
// ---------------------------------------------------------------------------

/// Mit Dreiergruppen findet die Suche die PIN-Zeile am **vollen** Pool.
///
/// # Die Gegenprobe steht in derselben Zusicherung
///
/// „Mit Gruppen wird PIN gewählt" allein sagt nichts darüber, ob die Gruppen es
/// waren — der Gewinn könnte von den Paaren kommen oder vom Rauschen. Gemessen
/// wird deshalb **dieselbe Stelle** einmal mit Paaren und ohne Gruppen, einmal
/// mit beidem. Genau dafür trägt [`Zuege`] zwei getrennte Schalter.
#[test]
fn erst_mit_dreiergruppen_findet_die_suche_die_zeile() {
    let pool = all_candidates();
    let cfg = config(TOEDLICH, 1.0);

    let nur_paare = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::MIT_PAAREN);
    let mit_gruppen = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::ALLE);

    // **Die Trägerzeile.** Wurde keine Gruppe geprüft, sagt ein Unterschied
    // oder sein Ausbleiben nichts über die Gruppen aus.
    assert!(
        mit_gruppen.tripel_geprueft > 0,
        "es wurde keine einzige Dreiergruppe geprueft — dann misst dieser Fall \
         nicht die Gruppen, sondern irgendetwas anderes. Stop: {:?}, Laenge {}",
        mit_gruppen.stop,
        mit_gruppen.chosen.len()
    );
    assert_eq!(
        nur_paare.tripel_geprueft, 0,
        "der Gegenlauf darf keine Gruppen pruefen — sonst vergleicht er sich \
         mit sich selbst"
    );

    assert!(
        !waehlt_pin(&nur_paare),
        "ohne Gruppen darf die PIN-Zeile nicht gefunden werden; sonst belegt \
         der Unterschied unten nichts. Gefunden: {:?}",
        zeilen(&nur_paare)
    );
    assert!(
        waehlt_pin(&mit_gruppen),
        "mit Gruppen muss die Suche die PIN-Zeile finden. Gefunden: {:?} \
         (geprueft: {} Gruppen)",
        zeilen(&mit_gruppen),
        mit_gruppen.tripel_geprueft
    );
}

/// Am überlebbaren Zwilling ändert sich nichts.
///
/// # Warum genau dieser Boss die Gegenprobe ist
///
/// `test-survivable-strike` unterscheidet sich vom tödlichen in **einer Zahl**
/// (4600 statt 18400). Der Spieler überlebt dort ohnehin, der Kampf wird nicht
/// abgeschnitten, und PIN ist reiner Verlust an Zeit.
///
/// Wählt die Suche ihn auch da, sobald sie Gruppen kennt, dann wählt sie ihn
/// unabhängig von seiner Wirkung — und der Fall darüber belegt nicht, was er
/// zu belegen vorgibt.
#[test]
fn am_ueberlebbaren_zwilling_aendert_sich_nichts() {
    let pool = all_candidates();
    let cfg = config(UEBERLEBBAR, 1.0);

    let nur_paare = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::MIT_PAAREN);
    let mit_gruppen = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::ALLE);

    // **Trägerzeile** (#1042): Lieferten BEIDE Suchen eine leere Rotation, wäre
    // alles darunter erfüllt — leer gleich leer, und PIN steht in keiner. Der
    // Fall wäre grün und hätte zwei Nichtse verglichen.
    assert!(
        !nur_paare.chosen.is_empty() && !mit_gruppen.chosen.is_empty(),
        "eine der beiden Suchen hat gar keine Rotation gefunden ({} gegen {} \
         Zeilen) — dann vergleicht dieser Fall zwei Nichtse",
        nur_paare.chosen.len(),
        mit_gruppen.chosen.len()
    );

    assert!(
        !waehlt_pin(&mit_gruppen),
        "am ueberlebbaren Zwilling bringt PIN nichts; wird er dort gewaehlt, \
         waehlt die Suche ihn unabhaengig von seiner Wirkung. \
         Gefunden: {:?}",
        zeilen(&mit_gruppen)
    );
    assert_eq!(
        zeilen(&nur_paare),
        zeilen(&mit_gruppen),
        "an einem Boss, an dem die Gruppe nichts bringt, darf sie die gefundene \
         Rotation nicht veraendern"
    );
}

/// An einem gewöhnlichen Boss verschiebt sich die gefundene Rotation nicht.
///
/// # Die Erwartung, an der ich am ehesten falsch liege
///
/// Der Gruppen-Zug wird an derselben Stelle gefragt wie der Paar-Zug — dort,
/// wo kein Einzelzug mehr trägt. Findet er an einem gewöhnlichen Boss
/// irgendeine Gruppe, die minimal besser ist, ändert sich dessen beste
/// Rotation, und **jede Zahl, die daran hängt, wandert mit**.
///
/// Dieser Fall ist die Bremse dagegen. Bricht er, ist das kein Grund, ihn
/// anzupassen: Dann ist der Eingriff nicht folgenlos, und das gehört gemeldet,
/// bevor irgendein Balance-Wert neu gelesen wird.
#[test]
fn an_einem_gewoehnlichen_boss_bleibt_die_rotation_gleich() {
    let pool = all_candidates();
    let cfg = config("test-metronome", 1.0);

    let nur_paare = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::MIT_PAAREN);
    let mit_gruppen = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::ALLE);

    // **Trägerzeile** (#1042), dieselbe wie am Zwilling: Zwei leere Rotationen
    // sind gleich, und der Vergleich darunter wäre grün, ohne etwas gesehen zu
    // haben.
    assert!(
        !nur_paare.chosen.is_empty() && !mit_gruppen.chosen.is_empty(),
        "eine der beiden Suchen hat gar keine Rotation gefunden ({} gegen {} \
         Zeilen) — dann vergleicht dieser Fall zwei Nichtse",
        nur_paare.chosen.len(),
        mit_gruppen.chosen.len()
    );

    assert_eq!(
        zeilen(&nur_paare),
        zeilen(&mit_gruppen),
        "die Dreiergruppen haben die gefundene Rotation an einem Boss \
         veraendert, fuer den sie nicht gebaut wurden. Das ist ein Befund, \
         keine Zahl zum Nachziehen: geprueft wurden {} Gruppen",
        mit_gruppen.tripel_geprueft
    );
}

/// Der Schalter schaltet wirklich — gemessen an der Zahl der Messungen.
///
/// # Warum die Rotation dafür nicht reicht
///
/// Sind beide Ergebnisse gleich, kann das heißen „die Gruppen ändern nichts"
/// oder „der Schalter tut nichts". Die Zahl der geprüften Gruppen trennt das:
/// Sie ist mit dem Schalter größer als null und ohne ihn genau null.
#[test]
fn der_schalter_schaltet() {
    let pool = all_candidates();
    let cfg = config(TOEDLICH, 1.0);

    let ohne = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::MIT_PAAREN);
    let mit = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::ALLE);

    assert_eq!(ohne.tripel_geprueft, 0);
    assert!(
        mit.tripel_geprueft > 0,
        "mit eingeschaltetem Gruppen-Zug muss mindestens eine Gruppe geprueft \
         werden"
    );
    assert!(
        mit.measured > ohne.measured,
        "wer Gruppen prueft, misst mehr Rotationen; gemessen {} gegen {}",
        mit.measured,
        ohne.measured
    );

    // Und die dritte Stufe: ganz ohne zusammengesetzte Zuege darf auch kein
    // Paar geprueft werden. Sonst wäre `NUR_EINZELN` nicht das, was der Name
    // sagt.
    let einzeln = greedy_zuege(&cfg, &pool, MAX_LEN, &[], Zuege::NUR_EINZELN);
    assert_eq!(einzeln.paare_geprueft, 0);
    assert_eq!(einzeln.tripel_geprueft, 0);
}

/// Ein Zauber ohne Kosten bekommt keine Gruppe.
///
/// # Was diese Zeile verhindert
///
/// Der Erzeuger ist nur dann ein **Glied** der Gruppe, wenn das erste Glied
/// überhaupt bezahlt werden muss. Ohne diese Bedingung wäre das dritte Glied
/// eine beliebige zusätzliche Zeile, und der Suchraum genau der, den
/// `tripel_zuege` vermeiden soll.
#[test]
fn ohne_kosten_keine_gruppe() {
    let pool: Vec<Candidate> = ["pin", "quench", "bind"]
        .iter()
        .map(|id| {
            SPELLS
                .iter()
                .position(|s| s.id == *id)
                .unwrap_or_else(|| panic!("{id} fehlt im Zauberdatensatz"))
        })
        .flat_map(candidates_for)
        .collect();
    let offen: Vec<usize> = (0..pool.len()).collect();

    // Träger: Mit den echten Daten (PIN kostet 3) entsteht die Gruppe.
    assert!(
        !tripel_zuege(&pool, &offen).is_empty(),
        "mit den echten Daten muss die Gruppe entstehen — sonst prueft die \
         Zusicherung unten das Fehlen aus dem falschen Grund"
    );

    // Und die Bedingung selbst: Das ERSTE Glied — die Zeile, um die es geht —
    // hat echte Kosten. Es stand zwischenzeitlich an zweiter Stelle; die
    // Messung an test-metronome hat die Ordnung zurueckgedreht.
    for (zeile_pos, _, _) in tripel_zuege(&pool, &offen) {
        let kosten = SPELLS[pool[offen[zeile_pos]].spell]
            .control
            .map_or(0, |c| c.cost);
        assert!(
            kosten > 0,
            "{} kostet nichts und braucht deshalb keinen Erzeuger",
            SPELLS[pool[offen[zeile_pos]].spell].id
        );
    }
}
