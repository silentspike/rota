//! Verwandelt `data/*.json` zur **Bauzeit** in statische Rust-Werte.
//!
//! GAME-DESIGN 18.3 verlangt, dass ein neuer Zauber ein Datensatz ist und kein
//! neuer Programmcode; Issue E2-1 verlangt zusaetzlich, dass diese Daten
//! **einkompiliert** sind statt zur Laufzeit geladen. Beides zusammen ergibt
//! genau diesen Schritt: `data/` ist und bleibt die einzige Wahrheit, aber im
//! fertigen Simulator liegt kein JSON und kein Parser mehr — nur noch Zahlen.
//!
//! Das hat drei Gruende, die alle in `GAME-DESIGN.md` 18.4 stehen:
//!
//! 1. **Keine Speicheranforderung im Schritt.** Ein Laufzeit-Parser wuerde
//!    Zeichenketten und Sammlungen anlegen; statische Werte tun das nicht.
//! 2. **Keine Hash-Container.** Aus einer JSON-Abbildung wird hier eine Liste
//!    mit fester Reihenfolge — die Reihenfolge der Datei.
//! 3. **Kein Dateizugriff zur Laufzeit**, also auch keine Abhaengigkeit davon,
//!    aus welchem Verzeichnis der Simulator gestartet wurde.
//!
//! Weicht eine Datei vom erwarteten Aufbau ab, bricht der Bau mit einer
//! benannten Meldung ab. Raten waere hier besonders teuer: Ein stillschweigend
//! auf 0 gesetzter Wert ergaebe einen Simulator, der plausibel aussieht und
//! falsch rechnet.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

mod time_fields;

/// Wie viele Verstaerkungs-Arten es gibt — **eine Kopie von
/// `sim/src/buffs.rs`** (Issue #420).
///
/// # Warum sie doppelt dasteht
///
/// `build.rs` laeuft VOR dem Paket und kann nichts aus `src/` importieren. Die
/// Zahl hier ist deshalb kein Verweis, sondern eine zweite Quelle — und zwei
/// Quellen koennen auseinanderlaufen.
///
/// **Was das Auseinanderlaufen faengt:** Waechst der Vorrat in `buffs.rs` und
/// nicht hier, lehnt dieser Bau eine gueltige Art ab — laut und beim Bauen.
/// Waechst er hier und nicht dort, laesst dieser Bau eine Art durch, die
/// `Buffs::apply` still verwirft; dagegen steht der Prueffall
/// `buff_arten_liegen_im_vorrat` in `sim/src/data.rs`, der jeden Zauber gegen
/// den ECHTEN Vorrat haelt.
///
/// Die zweite Richtung ist die gefaehrliche, und nur sie braucht einen
/// Prueffall — die erste meldet sich von selbst.
const BUFF_KIND_CAPACITY: usize = 8;

/// Pfad zur Zauber-Datei, relativ zum Paketverzeichnis `sim/`.
const SPELLS_JSON: &str = "../data/spells/calcination.json";
/// Derselbe Name ohne Pfad — er steht in jeder Beanstandung, die diese Datei
/// betrifft.
///
/// **Getrennt, weil `const` nicht rechnen kann**, nicht weil es zwei Wahrheiten
/// sein sollen: `SPELLS_JSON.rsplit('/')` ist in einem `const`-Ausdruck nicht
/// erlaubt. Gegen die Drift steht die Zusicherung in [`main`], die prüft, dass
/// der Pfad auf diesen Namen endet.
const SPELLS_DATEI: &str = "calcination.json";
/// Die schulunabhaengigen Sonderzauber (GAME-DESIGN 7.4), relativ zu `sim/`.
///
/// Getrennt von der Schule, weil sie **keiner** angehoeren: Ein Boss-Zauber
/// laesst sich unterbrechen ODER durch FIXATION verlangsamen ODER durch
/// COAGULATION aushalten (11.3). In `calcination.json` waere Unterbrechen eine
/// Faehigkeit der Kalzination — und das ist es nicht.
const COMMON_SPELLS_JSON: &str = "../data/spells/common.json";
/// Die Ueberlebens-Schule (GAME-DESIGN 4.1, Issue #79), relativ zu `sim/`.
const COAGULATION_JSON: &str = "../data/spells/coagulation.json";
/// Die Fixierung (GAME-DESIGN 4.1, Issue #100), relativ zu `sim/`.
const FIXATION_JSON: &str = "../data/spells/fixation.json";
/// SUBLIMATION: der Verkuerzer und der Ruecksetzer (Issue #201).
const SUBLIMATION_JSON: &str = "../data/spells/sublimation.json";
const PUTREFACTION_JSON: &str = "../data/spells/putrefaction.json";
/// Der Pruef-Zauber zur Wirkzeit (Issue #304), relativ zu `sim/`.
///
/// **Er gehoert nicht zum Spiel** — seine Stufe liegt ueber der Grenze, und
/// beide Stellen, die dem Spieler Zauber anbieten, filtern darueber. Er steht
/// hier, weil der Simulator dieselbe Zauberliste braucht wie das Spiel: Was
/// nicht in `SPELLS` steht, laesst sich in keiner Aufzeichnung druecken und
/// damit nicht auf Gleichlauf pruefen.
const TEST_CAST_JSON: &str = "../data/spells/test-cast.json";
/// Der Pruef-Zauber zur Verstaerkung (Issue #420). Eigene Datei und nicht ein
/// Feld auf `test-cast`, weil der dort der WIRKZEIT-Fall ist: Wer spaeter die
/// Wirkzeit misst, haette sonst still eine Verstaerkung dabei.
const TEST_BUFF_JSON: &str = "../data/spells/test-buff.json";
/// Der Pruef-Zauber zur Kanalisierung (Issue #323). Eigene Datei aus zwei
/// Gruenden: Der Parser lehnt Wirkzeit UND Kanalisierung am selben Zauber ab,
/// und `test-cast` steht in sechs Aufzeichnungen — ein zweiter Pruefzweck
/// darauf machte jede davon mehrdeutig.
const TEST_CHANNEL_JSON: &str = "../data/spells/test-channel.json";
/// Der Pruef-Zauber zur HEILUNGS-Kanalisierung (Issue #323). Er traegt die
/// Zahlen des Design-Beispiels woertlich (500 Heilung ueber 5 s); `test-channel`
/// daneben bleibt der SCHADENS-Fall. Zwei Pruefzwecke an einem Zauber machten
/// jede Messung mehrdeutig — dieselbe Begruendung wie bei test-buff neben
/// test-cast.
const TEST_CHANNEL_HEAL_JSON: &str = "../data/spells/test-channel-heal.json";
/// Der Pruef-Zauber zur SCHUTZ-Kanalisierung (Issue #323, GAME-DESIGN 20.5
/// Punkt 3). `maxStacks: 1` ist Absicht: Dort scheiterte die verworfene
/// Variante.
const TEST_CHANNEL_SHIELD_JSON: &str = "../data/spells/test-channel-shield.json";
/// Verzeichnis der Boss-Dateien, relativ zum Paketverzeichnis `sim/`.
///
/// **Alle** Dateien darin werden eingelesen, nicht eine benannte. Das ist die
/// Bedingung fuer AC1 aus Issue #96: Ein neuer Boss entsteht als Datendatei,
/// ohne dass eine Zeile Programmcode geaendert wird.
const BOSSES_DIR: &str = "../data/bosses";

/// Die aufgezeichneten Runden, die `rota-sim verify` gegen den Simulator haelt
/// (#970).
///
/// **Sie liegen unter `tests/`, werden aber NICHT nur von Tests gebraucht.**
/// Ein `[[bin]]` hat keinen Zugriff auf `serde_json` — das steht unter
/// `[dev-dependencies]` —, kann die Dateien also nicht lesen. Eingebettet
/// braucht der Selbsttest zur Laufzeit kein JSON, und die Bibliothek bleibt
/// abhaengigkeitsfrei.
const RECORDINGS_DIR: &str = "../tests/parity/recordings";
/// Die Grundwerte des Spielers, relativ zum Paketverzeichnis `sim/`.
const PLAYER_JSON: &str = "../data/player/base.json";
const LEVELS_JSON: &str = "../data/player/levels.json";
/// Die Daten der Fortschritts-Simulation (Issue #922): Gegenstaende samt
/// Fallrate, der Ertrag eines Sieges und die Instanzen mit ihrer
/// Freischaltung. Sie gehen in `laufbahn_generated.rs`, nicht in
/// `data_generated.rs` — der Kampf liest keine davon.
const ITEMS_JSON: &str = "../data/items.json";
const REWARD_JSON: &str = "../data/player/reward.json";
const INSTANCES_JSON: &str = "../data/instances.json";

/// Welcher Boss der Standard ist, wenn keiner gewaehlt wird.
///
/// Er steht hier und nicht in den Daten: Welcher Boss zuerst kommt, ist eine
/// Frage des Spielaufbaus (GAME-DESIGN 12.7, Instanz-Freischaltung) und keine
/// Eigenschaft des Bosses selbst.
const DEFAULT_BOSS_ID: &str = "first-crucible";

/// Fassung des Datenformats, die dieser Simulator versteht (`STEP1-SPEC.md` § 5).
const EXPECTED_SCHEMA: u64 = 1;

fn main() {
    // **Die beiden Schreibweisen desselben Namens bleiben beieinander.**
    // `SPELLS_DATEI` steht in jeder Beanstandung zu dieser Datei; wer den Pfad
    // umbenennt und den kurzen Namen vergisst, verschickt ab dann wieder
    // Meldungen, die in die falsche Datei zeigen. Genau das war der Fehler,
    // den diese Konstante behebt — er soll nicht auf einem anderen Weg
    // zurueckkommen.
    assert!(
        SPELLS_JSON.ends_with(SPELLS_DATEI),
        "SPELLS_JSON {SPELLS_JSON:?} endet nicht auf SPELLS_DATEI {SPELLS_DATEI:?} — \
         die Beanstandungen nennten dann eine andere Datei als die gelesene"
    );
    // Ohne diese beiden Zeilen merkt Cargo eine geaenderte Zauber-Datei nicht und
    // baut mit veralteten Werten weiter — der unangenehmste Fehler ueberhaupt,
    // weil er sich wie ein Rechenfehler anfuehlt.
    println!("cargo:rerun-if-changed={SPELLS_JSON}");
    println!("cargo:rerun-if-changed={COMMON_SPELLS_JSON}");
    println!("cargo:rerun-if-changed={TEST_BUFF_JSON}");
    println!("cargo:rerun-if-changed={TEST_CHANNEL_JSON}");
    println!("cargo:rerun-if-changed={TEST_CHANNEL_HEAL_JSON}");
    println!("cargo:rerun-if-changed={TEST_CHANNEL_SHIELD_JSON}");
    println!("cargo:rerun-if-changed={COAGULATION_JSON}");
    println!("cargo:rerun-if-changed={FIXATION_JSON}");
    println!("cargo:rerun-if-changed={SUBLIMATION_JSON}");
    println!("cargo:rerun-if-changed={PUTREFACTION_JSON}");
    println!("cargo:rerun-if-changed={TEST_CAST_JSON}");
    // Auf das VERZEICHNIS horchen, nicht auf einzelne Dateien: Sonst merkt
    // Cargo einen neu hinzugefuegten Boss nicht und baut mit der alten Liste
    // weiter — der Fehler saehe aus, als fehle die Datei.
    println!("cargo:rerun-if-changed={BOSSES_DIR}");
    println!("cargo:rerun-if-changed={PLAYER_JSON}");
    println!("cargo:rerun-if-changed={LEVELS_JSON}");
    println!("cargo:rerun-if-changed={ITEMS_JSON}");
    println!("cargo:rerun-if-changed={REWARD_JSON}");
    println!("cargo:rerun-if-changed={INSTANCES_JSON}");
    // Auf das Verzeichnis, aus demselben Grund wie bei den Bossen: Eine neue
    // Aufzeichnung soll den Bau ausloesen, nicht stillschweigend fehlen.
    println!("cargo:rerun-if-changed={RECORDINGS_DIR}");
    println!("cargo:rerun-if-changed=build.rs");

    let spells = read_json(SPELLS_JSON);
    let common = read_json(COMMON_SPELLS_JSON);
    let coagulation = read_json(COAGULATION_JSON);
    let fixation = read_json(FIXATION_JSON);
    let sublimation = read_json(SUBLIMATION_JSON);
    let putrefaction = read_json(PUTREFACTION_JSON);
    let test_cast = read_json(TEST_CAST_JSON);
    let test_buff = read_json(TEST_BUFF_JSON);
    let test_channel = read_json(TEST_CHANNEL_JSON);
    let test_channel_heal = read_json(TEST_CHANNEL_HEAL_JSON);
    let test_channel_shield = read_json(TEST_CHANNEL_SHIELD_JSON);
    let bosses = read_bosses();

    let mut out = String::with_capacity(8 * 1024);
    out.push_str(
        "// ERZEUGT VON build.rs — NICHT VON HAND AENDERN.\n\
         // Quelle: data/spells/calcination.json und data/bosses/*.json\n\n",
    );

    emit_school(&mut out, &spells);
    emit_resource(&mut out, &spells);
    // Die Reihenfolge dieser Liste bestimmt die Zauber-Indizes. Sie ist
    // dieselbe wie in `src/core/data.ts` (Konstante `CALCINATION`) und darf
    // nur HINTEN wachsen.
    // **Eine** Liste der Zusatz-Dateien, zweimal benutzt: fuer die
    // Zauber-Tabelle und fuer die Wirkungsbeschreibung (Issue #911). Zwei
    // getrennte Aufzaehlungen koennten auseinanderlaufen, und der Fehler waere
    // still — ein Zauber ohne Wirkungsbeschreibung faellt nirgends auf.
    let weitere: [(&str, &Value); 10] = [
        ("common.json", &common),
        ("coagulation.json", &coagulation),
        ("fixation.json", &fixation),
        ("sublimation.json", &sublimation),
        ("putrefaction.json", &putrefaction),
        ("test-cast.json", &test_cast),
        // **Ganz zuletzt**, hinter test-cast: Dessen Index steht in sechs
        // Aufzeichnungen, und die Reihenfolge dieser Liste ist die
        // Zauberreihenfolge (Issue #408, GAME-DESIGN 18.7).
        ("test-buff.json", &test_buff),
        // **Dahinter**, aus demselben Grund wie test-buff hinter test-cast:
        // Die Reihenfolge dieser Liste IST die Zauberreihenfolge (#408,
        // GAME-DESIGN 18.7), und bestehende Indizes duerfen sich nicht
        // verschieben.
        ("test-channel.json", &test_channel),
        // **Und ganz zuletzt der Heil-Kanal**, aus demselben Grund: Die
        // Reihenfolge dieser Liste IST die Zauberreihenfolge (#408,
        // GAME-DESIGN 18.7). Hinten anzuhaengen laesst jeden bestehenden
        // Index stehen.
        ("test-channel-heal.json", &test_channel_heal),
        // Ganz zuletzt, aus demselben Grund: Die Reihenfolge dieser Liste
        // IST die Zauberreihenfolge (#408).
        ("test-channel-shield.json", &test_channel_shield),
    ];
    for (name, data) in std::iter::once(("calcination.json", &spells)).chain(weitere) {
        time_fields::validate(data, name, RASTER_MS).unwrap_or_else(|problem| panic!("{problem}"));
    }
    emit_spells(&mut out, &spells, &weitere);

    // **Vor allem anderen, was die Ressource betrifft** (Issue #506).
    // `spells` ist calcination.json — die einzige Datei, die heute eine
    // Ressource fuehrt. Die Liste enthaelt alle uebrigen Schulen.
    pruefe_nur_eine_ressource(&[
        ("calcination.json", &spells),
        ("common.json", &common),
        ("coagulation.json", &coagulation),
        ("fixation.json", &fixation),
        ("sublimation.json", &sublimation),
        ("putrefaction.json", &putrefaction),
    ]);
    // Issue #981: Die Schulenmenge entsteht aus denselben Zauberdateien, die
    // schon `weitere` fuehrt — abgeleitet, nicht aufgeschrieben. Sie geht in
    // die Bosse, weil dort die Widerstands-Schluessel dagegen geprueft werden.
    let schulen_menge = schulen(&spells, &weitere);
    emit_instance_classes(&mut out);
    emit_bosses(&mut out, &bosses, &schulen_menge);
    emit_player(&mut out, &read_json(PLAYER_JSON));
    emit_levels(&mut out, &read_json(LEVELS_JSON));
    // Issue #911: die Wirkungsbeschreibung, aus denselben Dateien wie alles
    // darueber. Sie steht zuletzt, weil sie nichts erzeugt, worauf die anderen
    // sich stuetzen — sie beschreibt nur, was die anderen bereits tragen.
    emit_effects(&mut out, &spells, &weitere, &bosses);
    // **Nach den Zaubern**, weil die Belegung einer Aufzeichnung Zauber-NAMEN
    // nennt und hier zu Indizes aufgeloest wird (#970).
    //
    // Die Reihenfolge kommt aus `zauber_liste` — derselben Quelle, aus der
    // auch die Zauber-Tabelle und die Wirkungsbeschreibung entstehen. Sie ein
    // drittes Mal zusammenzusetzen waere genau die Drift, gegen die #911 die
    // Funktion ausgelagert hat.
    let spell_ids: Vec<String> = zauber_liste(&spells, &weitere)
        .iter()
        .map(|(spell, _, datei)| text(spell, "id", &format!("{datei}/spells[]")).to_owned())
        .collect();
    emit_recordings(&mut out, &spell_ids);

    let dest = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR fehlt")).join("data_generated.rs");
    fs::write(&dest, out).unwrap_or_else(|e| panic!("{} nicht schreibbar: {e}", dest.display()));

    // Issue #922: die Daten der Schleife UEBER dem Kampf, in eine eigene
    // Datei. `data_generated.rs` beschreibt den Kampf; was ein Sieg einbringt,
    // gehoert nicht hinein.
    let laufbahn = emit_laufbahn(
        &read_json(ITEMS_JSON),
        &read_json(REWARD_JSON),
        &read_json(LEVELS_JSON),
        &read_json(INSTANCES_JSON),
        &bosses,
    );
    let dest =
        PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR fehlt")).join("laufbahn_generated.rs");
    fs::write(&dest, laufbahn)
        .unwrap_or_else(|e| panic!("{} nicht schreibbar: {e}", dest.display()));
}

// ------------------------------------------------- Fortschritt (Issue #922)

/// Wie viele Affix-Plaetze eine Qualitaet hat — **eine Kopie von
/// `QUALITY_AFFIX_SLOTS` in `sim/src/gear.rs`**, aus demselben Grund wie
/// [`BUFF_KIND_CAPACITY`]: `build.rs` kann nichts aus `src/` einbinden.
///
/// Laeuft sie auseinander, faengt es `parity_laufbahn_regeln.rs`: Die
/// getragenen Werte dort rechnet das Spiel mit SEINER Tabelle.
const QUALITY_AFFIX_SLOTS: [usize; 4] = [1, 2, 3, 3];

/// Welche Affix-Arten einen Wert tragen, in der Reihenfolge der Felder von
/// `WornValues` (`gear.rs`) — und wie `feldFuerAffixArt` in `src/gear/worn.ts`
/// sie abbildet.
///
/// `active` fehlt mit Absicht: Das ist ein Topf ohne Wert, und ein Affix
/// darauf verfiele still (GAME-DESIGN 13.1b, Issue #421b). Das Spiel lehnt ihn
/// beim Laden ab; dieser Bau auch.
const AFFIX_FELDER: [(&str, &str); 5] = [
    ("catalysis", "FELD_CATALYSIS"),
    ("purity", "FELD_PURITY"),
    ("yield", "FELD_YIELD"),
    ("essence", "FELD_ESSENCE"),
    ("gearEffects", "FELD_GEAR_EFFECTS"),
];

/// Erzeugt `laufbahn_generated.rs`: Katalog, Fallrate, Ertrag, Erfahrung,
/// Instanzen und die Stufe jedes Bosses.
///
/// # Was hier geprueft wird und warum hier
///
/// Dieselben Pruefungen wie beim Laden im Spiel (`src/gear/catalog.ts`,
/// `src/core/data.ts`) — eine Datei, die das Spiel ablehnt, soll auch keinen
/// Simulator ergeben, der mit ihr rechnet. Dazu EINE eigene:
///
/// **Ein Satz-Teil darf nur bei einem Pruefboss fallen.** Die
/// Fortschritts-Simulation fuehrt die Satz-Tabelle nicht; ein Satz-Teil, das
/// ein Spielboss fallen laesst, bekaeme dort keinen Satz-Bonus — still. Der
/// Bau bricht deshalb, bis jemand `data/sets.json` in die Schleife holt.
fn emit_laufbahn(
    items: &Value,
    reward: &Value,
    levels: &Value,
    instances: &Value,
    bosses: &[(String, Value)],
) -> String {
    let mut out = String::with_capacity(8 * 1024);
    out.push_str(
        "// ERZEUGT VON build.rs — NICHT VON HAND AENDERN.\n\
         // Quelle: data/items.json, data/player/reward.json, data/player/levels.json,\n\
         // data/instances.json und data/bosses/*.json (Feld \"level\").\n\n",
    );

    // ---- Bosse: Kennung -> Instanz, und welche Instanzen Pruefprofile sind.
    let mut boss_instanz: Vec<(String, String)> = Vec::new();
    for (name, boss) in bosses {
        boss_instanz.push((
            text(boss, "id", name).to_owned(),
            text(boss, "instance", name).to_owned(),
        ));
    }
    let ist_pruefboss = |id: &str| {
        boss_instanz
            .iter()
            .any(|(b, i)| b == id && PROBE_INSTANCES.contains(&i.as_str()))
    };

    // ---- Katalog
    let whose = "items.json";
    let drop = uint(items, "dropChancePerMille", whose);
    assert!(
        drop <= 1000,
        "{whose}: dropChancePerMille ist {drop} — erlaubt sind 0 bis 1000"
    );
    let eintraege = items
        .get("items")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{whose}: Feld \"items\" fehlt"));
    let mut gesehen: Vec<&str> = Vec::new();
    let _ = writeln!(
        out,
        "/// Wie oft ein Sieg ueberhaupt etwas fallen laesst, in Promille."
    );
    let _ = writeln!(out, "pub const DROP_CHANCE_PER_MILLE: u32 = {drop};\n");
    let _ = writeln!(
        out,
        "/// Alle Gegenstaende in der Reihenfolge der Datei — die Reihenfolge, aus\n\
         /// der `rollLoot` waehlt. **Keine Hash-Abbildung** (GAME-DESIGN 18.3)."
    );
    let _ = writeln!(
        out,
        "pub static KATALOG: [Vorlage; {}] = [",
        eintraege.len()
    );
    for (index, item) in eintraege.iter().enumerate() {
        let wer = format!("{whose}/items[{index}]");
        let id = text(item, "id", &wer);
        assert!(!id.is_empty(), "{wer}: ein Gegenstand ohne Kennung");
        assert!(
            !gesehen.contains(&id),
            "{wer}: die Kennung \"{id}\" kommt zweimal vor"
        );
        gesehen.push(id);
        let stufe = uint(item, "itemLevel", &wer);
        assert!(
            (1..=100).contains(&stufe),
            "{wer}: \"{id}\" hat Stufe {stufe} — erlaubt sind 1 bis 100"
        );
        let guete = uint(item, "quality", &wer) as usize;
        assert!(
            guete < QUALITY_AFFIX_SLOTS.len(),
            "{wer}: \"{id}\" hat Qualitaet {guete}"
        );
        let platz = uint(item, "slot", &wer);
        assert!(platz < 10, "{wer}: \"{id}\" liegt auf Platz {platz}");
        let arten: Vec<&str> = item
            .get("affixKinds")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        assert!(
            arten.len() <= QUALITY_AFFIX_SLOTS[guete],
            "{wer}: \"{id}\" nennt {} Werte-Arten, hat bei Qualitaet {guete} aber {} \
             Affix-Plaetze",
            arten.len(),
            QUALITY_AFFIX_SLOTS[guete]
        );
        let felder: Vec<&str> = arten
            .iter()
            .map(|art| {
                AFFIX_FELDER
                    .iter()
                    .find(|(name, _)| name == art)
                    .map(|(_, feld)| *feld)
                    .unwrap_or_else(|| {
                        panic!(
                            "{wer}: \"{id}\" traegt die Werte-Art \"{art}\", die keinen Wert \
                             fuellt. Wertbar sind {:?}.",
                            AFFIX_FELDER.map(|(n, _)| n)
                        )
                    })
            })
            .collect();
        let gebunden = item
            .get("boundToBoss")
            .and_then(Value::as_str)
            .unwrap_or("");
        let satz = match item.get("setId") {
            None => -1_i64,
            Some(Value::Number(n)) if n.is_i64() => n.as_i64().expect("is_i64 geprueft"),
            Some(other) => panic!("{wer}: setId muss eine ganze Zahl sein, ist {other}"),
        };
        assert!(
            (-1..6).contains(&satz),
            "{wer}: \"{id}\" gehoert zu Satz {satz} — erlaubt sind 0 bis 5 oder -1"
        );
        assert!(
            satz == -1 || ist_pruefboss(gebunden),
            "{wer}: \"{id}\" ist ein Satz-Teil (Satz {satz}) und kann bei einem Spielboss \
             fallen (gebunden an \"{gebunden}\"). Die Fortschritts-Simulation \
             (sim/src/laufbahn/regeln.rs) kennt die Satz-Tabelle nicht und rechnete den \
             Bonus still als null. Erst data/sets.json dort einlesen, dann diese Pruefung \
             loesen."
        );
        let _ = writeln!(
            out,
            "    Vorlage {{ id: {id:?}, slot: {platz}, item_level: {stufe}, quality: {guete}, \
             affixe: &[{}], gebunden_an: {gebunden:?}, satz: {satz} }},",
            felder.join(", ")
        );
    }
    let _ = writeln!(out, "];\n");

    // ---- Ertrag eines Sieges (reward.json)
    let whose = "player/reward.json";
    let abfall: Vec<u64> = reward
        .get("falloffPerMille")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{whose}: Feld \"falloffPerMille\" fehlt"))
        .iter()
        .map(|v| {
            v.as_u64()
                .unwrap_or_else(|| panic!("{whose}: Abstandsfaktor ist keine ganze Zahl"))
        })
        .collect();
    assert!(!abfall.is_empty(), "{whose}: falloffPerMille ist leer");
    assert!(
        abfall.iter().all(|f| *f <= 1000),
        "{whose}: ein Abstandsfaktor liegt ueber 1000 Promille"
    );
    let gleitkomma = |feld: &str| {
        reward
            .get(feld)
            .and_then(Value::as_f64)
            .unwrap_or_else(|| panic!("{whose}: Feld \"{feld}\" fehlt oder ist keine Zahl"))
    };
    let _ = writeln!(
        out,
        "/// Der abnehmende Ertrag in Promille, Index = Stufenabstand."
    );
    let _ = write!(
        out,
        "pub static FALLOFF_PER_MILLE: [u32; {}] = [",
        abfall.len()
    );
    for f in &abfall {
        let _ = write!(out, "{f}, ");
    }
    let _ = writeln!(out, "];\n");
    let _ = writeln!(out, "/// Punkte je Sekunde hoch Exponent.");
    let _ = writeln!(
        out,
        "pub const POINTS_PER_SECOND: f64 = {:?};",
        gleitkomma("pointsPerSecond")
    );
    let _ = writeln!(out, "/// Der Exponent der Punktekurve (12.6: quadratisch).");
    let _ = writeln!(
        out,
        "pub const POINTS_EXPONENT: f64 = {:?};",
        gleitkomma("pointsExponent")
    );
    let _ = writeln!(
        out,
        "/// Ring-Prozentpunkte in Sekunden-Einheiten (runaway-Bosse, #256)."
    );
    let _ = writeln!(
        out,
        "pub const HELD_POINTS_PER_PERCENT: f64 = {:?};",
        gleitkomma("heldPointsPerPercent")
    );
    let _ = writeln!(
        out,
        "/// Rest-Ertrag fuers blosse Wiederholen, in Promille (#377)."
    );
    let _ = writeln!(
        out,
        "pub const REPEAT_PER_MILLE: f64 = {:?};\n",
        gleitkomma("repeatPerMille")
    );

    // ---- Erfahrung (levels.json) — die Kurve, die `emit_levels` auslaesst.
    let whose = "player/levels.json";
    let je_stufe = uint(levels, "xpPerBossLevel", whose);
    assert!(je_stufe > 0, "{whose}: xpPerBossLevel muss positiv sein");
    let max_level = uint(levels, "maxLevel", whose);
    let bedarf: Vec<u64> = levels
        .get("xpToNextLevel")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{whose}: Feld \"xpToNextLevel\" fehlt"))
        .iter()
        .map(|v| {
            v.as_u64()
                .unwrap_or_else(|| panic!("{whose}: Erfahrungsbedarf ist keine ganze Zahl"))
        })
        .collect();
    assert!(
        bedarf.len() as u64 + 1 == max_level,
        "{whose}: {} Eintraege in xpToNextLevel fuer {max_level} Stufen — es braucht einen \
         je Aufstieg, also maxLevel - 1",
        bedarf.len()
    );
    let _ = writeln!(out, "/// Erfahrung je Boss-Stufe fuer einen Sieg.");
    let _ = writeln!(out, "pub const XP_PER_BOSS_LEVEL: u64 = {je_stufe};");
    let _ = writeln!(
        out,
        "/// Erfahrung bis zur naechsten Stufe. Index 0 ist der Weg von Stufe 1 auf 2."
    );
    let _ = write!(
        out,
        "pub static XP_TO_NEXT_LEVEL: [u64; {}] = [",
        bedarf.len()
    );
    for b in &bedarf {
        let _ = write!(out, "{b}, ");
    }
    let _ = writeln!(out, "];\n");

    // ---- Instanzen, nach `order` sortiert wie `INSTANCES` im Spiel.
    let whose = "instances.json";
    let liste = instances
        .get("instances")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{whose}: Feld \"instances\" fehlt"));
    let mut instanzen: Vec<(String, u64, u64, Option<String>)> = Vec::new();
    for (index, eintrag) in liste.iter().enumerate() {
        let wer = format!("{whose}/instances[{index}]");
        let id = text(eintrag, "id", &wer).to_owned();
        assert!(
            !instanzen.iter().any(|(i, ..)| *i == id),
            "{wer}: die Kennung \"{id}\" kommt zweimal vor"
        );
        let nummer = uint(eintrag, "number", &wer);
        assert!(nummer >= 1, "{wer}: \"{id}\" hat number {nummer}");
        let ordnung = opt_uint(eintrag, "order", &wer, 0);
        let verlangt = eintrag
            .get("requires")
            .and_then(Value::as_str)
            .map(str::to_owned);
        instanzen.push((id, nummer, ordnung, verlangt));
    }
    for (id, _, _, verlangt) in &instanzen {
        if let Some(v) = verlangt {
            assert!(v != id, "{whose}: \"{id}\" verlangt sich selbst");
            assert!(
                instanzen.iter().any(|(i, ..)| i == v),
                "{whose}: \"{id}\" verlangt \"{v}\", die es nicht gibt"
            );
        }
    }
    // Stabil sortiert, wie `[...alle].sort((a, b) => a.order - b.order)`.
    instanzen.sort_by_key(|(_, _, ordnung, _)| *ordnung);
    let _ = writeln!(
        out,
        "/// Alle Instanzen, nach `order` sortiert — dieselbe Folge wie `INSTANCES`."
    );
    let _ = writeln!(
        out,
        "pub static INSTANZEN: [Instanz; {}] = [",
        instanzen.len()
    );
    for (id, nummer, ordnung, verlangt) in &instanzen {
        let _ = writeln!(
            out,
            "    Instanz {{ id: {id:?}, number: {nummer}, order: {ordnung}, requires: {verlangt:?} }},"
        );
    }
    let _ = writeln!(out, "];\n");

    // ---- Die Stufe jedes Bosses, in der Reihenfolge von `BOSSES`.
    let _ = writeln!(
        out,
        "/// Die Stufe jedes Bosses, Index wie `crate::data::BOSSES`. Fehlt das Feld,\n\
         /// gilt 1 — wie beim Laden im Spiel (`src/core/data.ts`)."
    );
    let _ = writeln!(out, "pub static BOSS_STUFEN: [u32; {}] = [", bosses.len());
    for (name, boss) in bosses {
        let stufe = opt_uint(boss, "level", name, 1).max(1);
        let _ = writeln!(out, "    {stufe},");
    }
    let _ = writeln!(out, "];");
    out
}

// ------------------------------------------------------------------ Einlesen

fn read_json(rel: &str) -> Value {
    read_json_at(Path::new(rel))
}

fn read_json_at(path: &Path) -> Value {
    let rel = path.display().to_string();
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{rel} nicht lesbar: {e} (Arbeitsverzeichnis: sim/)"));
    let value: Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("{rel} ist kein gueltiges JSON: {e}"));

    let schema = value
        .get("schema")
        .and_then(Value::as_u64)
        .unwrap_or_else(|| panic!("{rel}: Feld \"schema\" fehlt"));
    assert_eq!(
        schema, EXPECTED_SCHEMA,
        "{rel}: Datenformat {schema}, dieser Simulator versteht {EXPECTED_SCHEMA}"
    );

    value
}

/// Liest ein Pflichtfeld als vorzeichenlose Ganzzahl.
///
/// Fliesskomma wird abgelehnt statt gerundet: Ganzzahlige Zeit ist die erste
/// Determinismus-Regel aus 18.4, und ein `1500.0` in der Datei waere ein Hinweis
/// darauf, dass jemand anderswo mit Sekunden rechnet.
fn uint(node: &Value, field: &str, whose: &str) -> u64 {
    let raw = node
        .get(field)
        .unwrap_or_else(|| panic!("{whose}: Feld \"{field}\" fehlt"));
    match raw {
        Value::Number(n) if n.is_u64() => n.as_u64().expect("is_u64 geprueft"),
        other => panic!("{whose}: \"{field}\" muss eine ganze Zahl >= 0 sein, ist aber {other}"),
    }
}

fn boolean(node: &Value, field: &str, whose: &str) -> bool {
    node.get(field)
        .and_then(Value::as_bool)
        .unwrap_or_else(|| panic!("{whose}: Feld \"{field}\" fehlt oder ist kein Wahrheitswert"))
}

fn text<'a>(node: &'a Value, field: &str, whose: &str) -> &'a str {
    node.get(field)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{whose}: Feld \"{field}\" fehlt oder ist keine Zeichenkette"))
}

/// Liest ein **optionales** Zahlenfeld. Fehlt es, gilt `default`.
///
/// Getrennt von [`uint`], und das ist Absicht: `uint` bricht bei einem
/// fehlenden Feld ab, weil ein stillschweigend auf 0 gesetzter Pflichtwert
/// einen Simulator ergaebe, der plausibel aussieht und falsch rechnet. Bei
/// einem Feld, das die meisten Zauber gar nicht fuehren, ist das Fehlen
/// dagegen die Regel und keine Nachlaessigkeit.
fn opt_uint(node: &Value, field: &str, whose: &str, default: u64) -> u64 {
    let Some(raw) = node.get(field) else {
        return default;
    };
    match raw {
        Value::Number(n) if n.is_u64() => n.as_u64().expect("is_u64 geprueft"),
        other => panic!("{whose}: \"{field}\" muss eine ganze Zahl >= 0 sein, ist aber {other}"),
    }
}

/// Liest `{ "heat": 3 }` und prueft dabei, dass die genannte Ressource die der
/// Schule ist. Ein Zauber, der eine unbekannte Ressource fuellt, waere im
/// Simulator sonst eine stille Null.
fn resource_amount(node: &Value, field: &str, whose: &str, resource_id: &str) -> u64 {
    let Some(map) = node.get(field) else {
        return 0;
    };
    let object = map
        .as_object()
        .unwrap_or_else(|| panic!("{whose}: \"{field}\" muss ein Objekt sein"));
    assert_eq!(
        object.len(),
        1,
        "{whose}: \"{field}\" nennt {} Ressourcen; Schritt 1 kennt genau eine",
        object.len()
    );
    let (key, value) = object.iter().next().expect("Laenge 1 geprueft");
    assert_eq!(
        key, resource_id,
        "{whose}: \"{field}\" nennt die Ressource \"{key}\", die Schule fuehrt aber \"{resource_id}\""
    );
    value
        .as_u64()
        .unwrap_or_else(|| panic!("{whose}: \"{field}.{key}\" muss eine ganze Zahl >= 0 sein"))
}

// ------------------------------------------------------------------ Erzeugen

fn emit_school(out: &mut String, spells: &Value) {
    let school = text(spells, "school", "calcination.json");
    let _ = writeln!(out, "/// Schule, aus der die Zauber stammen.");
    let _ = writeln!(out, "pub const SCHOOL: &str = {school:?};\n");
}

/// Bricht ab, wenn mehr als eine Schule eine eigene Ressource fuehrt
/// (Issue #506, gefunden von Worker-1).
///
/// # Die Bauart sieht nur EINE Ressource vor
///
/// `emit_resource` liest den Abschnitt fest aus `calcination.json` und
/// erzeugt daraus ein einzelnes `pub static RESOURCE`. Das Spiel fuehrt
/// dagegen eine Ressource JE SCHULE (`SchoolDef.resource`).
///
/// Solange nur CALCINATION eine hat, faellt der Unterschied nicht auf. Die
/// zweite Schule mit eigener Ressource wuerde still ignoriert: Der Simulator
/// rechnete weiter mit Hitze, das Spiel mit der neuen — und die Paritaet
/// meldete eine Abweichung an einer Stelle, die nicht die Ursache ist.
///
/// **Dieser Waechter ist bei seiner Einfuehrung gruen**, weil es heute nur
/// eine Ressource gibt. Belegt ist er deshalb durch die Gegenprobe in
/// `tests/eine_ressource.rs`, die einen zweiten Abschnitt nachstellt.
fn pruefe_nur_eine_ressource(dateien: &[(&str, &Value)]) {
    let mit: Vec<&str> = dateien
        .iter()
        .filter(|(_, inhalt)| inhalt.get("resource").is_some())
        .map(|(name, _)| *name)
        .collect();

    assert!(
        mit.len() <= 1,
        "Diese Schul-Dateien fuehren eine eigene Ressource: {mit:?}\n\
         `emit_resource` sieht nur EINE vor — es liest calcination.json fest \
         und erzeugt ein einzelnes RESOURCE. Jede weitere wuerde STILL \
         ignoriert, und der Simulator rechnete mit der falschen Groesse.\n\
         Vor der zweiten Ressource muss die Bauart auf eine Ressource JE \
         SCHULE umgestellt werden (Issue #506)."
    );
}

fn emit_resource(out: &mut String, spells: &Value) {
    let node = spells
        .get("resource")
        .unwrap_or_else(|| panic!("calcination.json: Abschnitt \"resource\" fehlt"));
    let whose = "calcination.json/resource";

    let _ = writeln!(
        out,
        "/// Die Schul-Mechanik, hier die Hitze (`STEP1-SPEC` 1.4)."
    );
    let _ = writeln!(out, "pub static RESOURCE: Resource = Resource {{");
    let _ = writeln!(out, "    id: {:?},", text(node, "id", whose));
    let _ = writeln!(out, "    name: {:?},", text(node, "name", whose));
    let _ = writeln!(out, "    cap: {},", uint(node, "cap", whose));
    let _ = writeln!(
        out,
        "    decay_per_step_ms: {},",
        uint(node, "decayPerStepMs", whose)
    );
    let _ = writeln!(
        out,
        "    decay_amount: {},",
        uint(node, "decayAmount", whose)
    );
    let _ = writeln!(
        out,
        "    highlight_at: {},",
        uint(node, "highlightAt", whose)
    );
    let _ = writeln!(out, "}};\n");
}

/// Die Zauberliste erzeugen.
///
/// `weitere` sind die Dateien HINTER der Schule, **in genau der Reihenfolge, in
/// der sie angehaengt werden** — jeweils mit ihrem Dateinamen fuer die
/// Fehlermeldung. Eine Liste statt einzelner Parameter, seit die siebte Datei
/// dazukam (Issue #304): clippy laesst hoechstens sieben Argumente zu, und die
/// Aufzaehlung am Aufrufer sagt ohnehin deutlicher, worauf es hier ankommt.
///
/// **Die Reihenfolge ist die Aussage.** Sie bestimmt die Indizes, und die
/// stehen in jeder aufgezeichneten Runde; wer eine Datei dazwischenschiebt,
/// deutet jede alte Aufzeichnung still um.
/// Die Zauber aller Dateien in **einer** festen Reihenfolge.
///
/// Ausgelagert, weil DREI Stellen sie brauchen: [`emit_spells`] fuer die
/// Zauber-Tabelle, [`emit_effects`] fuer die Wirkungsbeschreibung (Issue #911)
/// und [`emit_recordings`] fuer die Belegung der Aufzeichnungen (Issue #970). Zwei getrennte Aufbauten waeren zwei Listen, die auseinanderlaufen
/// koennen — und genau diese Sorte Drift ist der Anlass fuer #911.
/// **Die DATEI wird mitgegeben, nicht später erraten.**
///
/// Bis zum 2026-08-22 gab diese Funktion nur `(Zauber, Schule)` zurück, und
/// `emit_spell` baute seinen Meldungstext mit einem eingebrannten
/// `calcination.json`. Jede Beanstandung nannte damit dieselbe Datei, aus
/// welcher der sechs der Zauber auch stammte:
///
/// ```text
/// calcination.json/spells[harden]: extendShieldMs 12001 …   harden steht in coagulation.json
/// calcination.json/spells[decay]/decayWard: …               decay steht in putrefaction.json
/// ```
///
/// Beide Meldungen schicken den Leser in die falsche Datei — und die zweite
/// hat es bis in ein Issue geschafft, bevor jemand nachgesehen hat.
///
/// **Aus der Schule ableiten wäre der falsche Weg gewesen:** `common.json`
/// trägt keine, und die fünf `test-*.json` bräuchten je einen Sonderfall.
/// Der Aufrufer *kennt* den Dateinamen; ihn durchzureichen ist gelesen statt
/// geraten.
fn zauber_liste<'a>(
    spells: &'a Value,
    weitere: &[(&'a str, &'a Value)],
) -> Vec<(&'a Value, Option<&'a str>, &'a str)> {
    let list = spells
        .get("spells")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("calcination.json: Liste \"spells\" fehlt"));
    assert!(!list.is_empty(), "calcination.json: \"spells\" ist leer");

    // Die Sonderzauber stehen **hinter** den Schulzaubern, immer. Die
    // Reihenfolge ist Teil des Determinismus (GAME-DESIGN 18.3): Sie bestimmt
    // die Indizes, und die stehen in jeder aufgezeichneten Runde. Sie
    // umzustellen deutete jede alte Aufzeichnung still um.
    // **Neue Zauber kommen HINTEN dazu, nie dazwischen.** Die Reihenfolge
    // bestimmt die Indizes, und die stehen in jeder aufgezeichneten Runde —
    // QUENCH von Platz 3 auf Platz 6 zu schieben deutete jede alte
    // Aufzeichnung still um, ohne dass ein Prueffall es meldet.
    let eigene_schule = spells.get("school").and_then(Value::as_str);
    let mut list: Vec<(&Value, Option<&str>, &str)> = list
        .iter()
        .map(|s| (s, eigene_schule, SPELLS_DATEI))
        .collect();
    for (datei, quelle) in weitere {
        let eintraege = quelle
            .get("spells")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{datei}: Liste \"spells\" fehlt"));
        // Die Schule steht an der DATEI, nicht am Zauber — `common.json` traegt
        // keine, und seine Zauber gehoeren deshalb keiner an.
        let schule = quelle.get("school").and_then(Value::as_str);
        list.extend(eintraege.iter().map(|s| (s, schule, *datei)));
    }
    list
}

fn emit_spells(out: &mut String, spells: &Value, weitere: &[(&str, &Value)]) {
    let resource_id = text(
        spells.get("resource").expect("resource geprueft"),
        "id",
        "calcination.json/resource",
    );

    let list = zauber_liste(spells, weitere);

    let _ = writeln!(
        out,
        "/// Anzahl der Zauber. Feste Groesse fuer alle Zustands-Listen im Kampf —\n\
         /// dadurch braucht der Simulator im Schritt keinen Speicher anzufordern.",
    );
    let _ = writeln!(out, "pub const SPELL_COUNT: usize = {};\n", list.len());

    let _ = writeln!(
        out,
        "/// Die Zauber in der Reihenfolge der Datei. **Keine Hash-Abbildung** —\n\
         /// die Reihenfolge ist Teil des Determinismus (GAME-DESIGN 18.3)."
    );
    let _ = writeln!(out, "pub static SPELLS: [Spell; SPELL_COUNT] = [");
    for (spell, schule, datei) in &list {
        emit_spell(out, spell, resource_id, *schule, datei);
    }
    let _ = writeln!(out, "];\n");

    // Benannte Indizes, damit Rotationen nicht mit nackten Zahlen hantieren.
    // Auch sie stammen aus der Datei, nicht aus dieser Quelldatei.
    for (index, (spell, _, datei)) in list.iter().enumerate() {
        let id = text(spell, "id", &format!("{datei}/spells[]"));
        let konst = id.to_uppercase().replace(['-', ' '], "_");
        let _ = writeln!(out, "/// Index von `{id}` in [`SPELLS`].");
        let _ = writeln!(out, "pub const {konst}: usize = {index};");
    }
    out.push('\n');
}

fn emit_spell(
    out: &mut String,
    spell: &Value,
    resource_id: &str,
    school: Option<&str>,
    datei: &str,
) {
    let id = text(spell, "id", &format!("{datei}/spells[]"));
    let whose = format!("{datei}/spells[{id}]");

    let _ = writeln!(out, "    Spell {{");
    let _ = writeln!(out, "        id: {id:?},");
    // Die Schule des Zaubers — aus der DATEI, in der er steht (Issue #328).
    // `None` fuer die schulunabhaengigen Sonderzauber (GAME-DESIGN 7.4):
    // Wer keiner Schule angehoert, den kann auch kein Schul-Widerstand
    // abhalten. Dieselbe Zuordnung wie `mitSchule` in `src/core/data.ts`.
    let _ = match school {
        Some(name) => writeln!(out, "        school: Some({name:?}),"),
        None => writeln!(out, "        school: None,"),
    };
    let _ = writeln!(out, "        name: {:?},", text(spell, "name", &whose));
    let _ = writeln!(
        out,
        "        unlock_level: {},",
        uint(spell, "unlockLevel", &whose)
    );
    let _ = writeln!(
        out,
        "        base_damage: {},",
        uint(spell, "baseDamage", &whose)
    );
    let _ = writeln!(
        out,
        "        cooldown_ms: {},",
        uint(spell, "cooldownMs", &whose)
    );
    let _ = writeln!(
        out,
        "        cast_time_ms: {},",
        uint(spell, "castTimeMs", &whose)
    );
    // Die Kanalisierung (Issue #323). Beides zugleich schliesst der
    // TS-Parser aus; hier steht nur die Uebertragung, damit die Felder
    // ueberhaupt auf beiden Seiten stehen.
    let _ = writeln!(
        out,
        "        channel_ms: {},",
        opt_uint(spell, "channelMs", &whose, 0)
    );
    // **`channelTickMs` MUSS auf dem Raster liegen** (Issue #1104).
    //
    // Der Abstand wird in `fight.rs` zum Termin gerechnet: Der ERSTE Tick geht
    // durch `snap_up`, die folgenden nicht — dort steht `at += schritt`. Und
    // `hasted_ms` rastert zwar sein Ergebnis, kehrt aber bei Tempo 0 mit dem
    // ROHEN Wert zurueck (`haste.rs`, `if permille == 0 { return base_ms }`).
    // Ohne Tempo ist der Rohwert also der Schritt.
    //
    // Ein `channelTickMs: 333` legte damit Termine auf 333, 666, 999 — neben
    // das 10-ms-Raster des Spiels. Die TypeScript-Seite prueft heute, dass
    // `channelMs` ein Vielfaches von `channelTickMs` ist; sie prueft NICHT,
    // dass der Tick selbst darstellbar ist. Zwei Rasterfragen, und nur eine
    // davon war beantwortet.
    let channel_tick = opt_uint(spell, "channelTickMs", &whose, 0);
    assert!(
        channel_tick.is_multiple_of(RASTER_MS),
        "{whose}: channelTickMs {channel_tick} liegt nicht auf dem \
         {RASTER_MS}-ms-Raster des Spiels — die Teilwirkungen nach der ersten \
         laegen zwischen zwei Takten"
    );
    let _ = writeln!(out, "        channel_tick_ms: {channel_tick},");
    let _ = writeln!(
        out,
        "        triggers_gcd: {},",
        boolean(spell, "triggersGcd", &whose)
    );
    // Optional: Die allermeisten Zauber unterbrechen nichts, und sie sollen
    // das Feld gar nicht erst nennen muessen.
    let unterbricht = spell
        .get("interrupt")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(out, "        interrupt: {unterbricht},");
    // Das Verhaengnis senken (Baustein 2; Issue #561) — dieselbe Bauart eine
    // Zeile hoeher: Ob der Zauber es tut, steht in den Daten; WIE VIEL er
    // senkt, steht beim Boss.
    let senkt_verhaengnis = spell
        .get("clearsDoom")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(out, "        clears_doom: {senkt_verhaengnis},");
    // **Den NAECHSTEN Zauber von der Wartezeit befreien** (SUBLIME, #480).
    //
    // Unterscheidet sich von `triggersGcd` in dem, worauf es zeigt:
    // `triggersGcd` sagt, ob DIESER Zauber die Wartezeit ausloest — eine
    // Eigenschaft, die in den Daten steht und sich nie aendert. Dieses Feld
    // setzt einen MERKER, der bis zum naechsten Wurf lebt.
    //
    // Merker sind neu im Datenmodell, und sie haben eine Bedingung: Sie
    // gehoeren in die Pruefsumme. Die Begruendung steht an der Stelle, an der
    // sie dort eingehaengt sind (`src/core/checksum.ts`).
    let befreit_naechsten = spell
        .get("freesNextGcd")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(out, "        frees_next_gcd: {befreit_naechsten},");
    // CONDENSE (#480), der zweite Merker. Er steht bewusst NEBEN dem ersten
    // und nicht am Ende der Datei: Merker sind eine eigene Sorte Feld, und
    // vier von ihnen einzeln verstreut waeren schwerer zu pflegen als eine
    // Gruppe. Dieselbe Ueberlegung wie in beiden Pruefsummen.
    let verdoppelt_naechsten = spell
        .get("doublesNextCast")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(out, "        doubles_next_cast: {verdoppelt_naechsten},");
    // RETORT (#480), der dritte Merker — hier aus demselben Grund wie der
    // zweite: nebeneinander, nicht verstreut. Sein Zustand traegt allerdings
    // eine ZAHL statt eines Ja/Nein, das Datenfeld bleibt ein Schalter.
    let erstattet_wartezeit = spell
        .get("refundsInterruptedGcd")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(
        out,
        "        refunds_interrupted_gcd: {erstattet_wartezeit},"
    );
    // Ein Unterbrecher, der die globale Wartezeit ausloest, waere die Ausnahme
    // aus GAME-DESIGN 6.6 mit falschem Vorzeichen: Er koennte nicht mehr
    // zwischen zwei Zauber gelegt werden, und der ganze Baustein waere tot.
    if unterbricht {
        assert!(
            !boolean(spell, "triggersGcd", &whose),
            "{whose}: Ein Unterbrecher darf keine globale Wartezeit ausloesen \
             (GAME-DESIGN 6.6 — die einzige Ausnahme im Spiel)"
        );
    }
    let _ = writeln!(
        out,
        "        resource_gain: {},",
        resource_amount(spell, "resourceGain", &whose, resource_id)
    );
    let _ = writeln!(
        out,
        "        cooldown_reduction_below_ms: {},",
        // **Optional, nicht pflicht** (#518): Ohne das Feld verkuerzt der
        // Zauber alles, wie vor der Aenderung. `uint` bricht bei fehlenden
        // Feldern ab — das ist fuer Pflichtfelder richtig und hier falsch.
        spell
            .get("cooldownReductionBelowMs")
            .and_then(Value::as_u64)
            .unwrap_or(0)
    );
    let _ = writeln!(
        out,
        "        cooldown_reduction_longest_only: {},",
        spell
            .get("cooldownReductionLongestOnly")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    );
    // **`cooldownReductionMs` MUSS auf dem Raster liegen** (Issue #1104).
    //
    // `sublimation.rs`, `shorten_cooldowns`: `*slot = slot.saturating_sub(
    // order.amount).max(now)` — ohne `snap_up`. Ein ungerasterter Betrag
    // verschiebt die Bereitschaft neben den Takt, und der Zauber waere im
    // Simulator zu einem Zeitpunkt bereit, den das Spiel nicht kennt.
    //
    // `cooldownReductionBelowMs` steht bewusst NICHT unter derselben Regel:
    // Es ist eine VERGLEICHSSCHWELLE (`slot - now >= below` in derselben
    // Datei), kein Zeitpunkt. Ein Wert daneben verschiebt, WELCHE Abklingzeit
    // erfasst wird, erzeugt aber keinen Termin zwischen zwei Takten. Der
    // Unterschied ist nicht die Einheit — beide heissen `…Ms` —, sondern was
    // aus dem Wert wird.
    let cd_reduction = opt_uint(spell, "cooldownReductionMs", &whose, 0);
    assert!(
        cd_reduction.is_multiple_of(RASTER_MS),
        "{whose}: cooldownReductionMs {cd_reduction} liegt nicht auf dem \
         {RASTER_MS}-ms-Raster des Spiels — die verkuerzte Abklingzeit endete \
         zwischen zwei Takten"
    );
    let _ = writeln!(out, "        cooldown_reduction_ms: {cd_reduction},");
    // **Eine ANZAHL statt eines Ja/Nein** (Issue #521). Die alte Form
    // `armsCooldownReset: true` liest sich weiter als 1 — sonst verlöre ein
    // Datensatz aus der Zeit davor still seine Wirkung, und der Bau meldete
    // nichts.
    let ladungen = spell
        .get("cooldownResetCharges")
        .and_then(Value::as_u64)
        .or_else(|| {
            spell
                .get("armsCooldownReset")
                .and_then(Value::as_bool)
                .and_then(|ja| ja.then_some(1))
        })
        .unwrap_or(0);
    assert!(
        ladungen <= u64::from(u8::MAX),
        "{whose}: cooldownResetCharges {ladungen} passt nicht in u8"
    );
    let _ = writeln!(out, "        cooldown_reset_charges: {ladungen},");
    emit_dot(out, spell, &whose, resource_id);
    emit_buff(out, spell, &whose);
    emit_spender(out, spell, &whose, resource_id);
    emit_coagulation(out, spell, &whose);
    emit_putrefaction(out, spell, &whose);
    emit_synergy(out, spell);
    emit_fixation(out, spell, &whose);
    emit_convert(out, spell, &whose);
    let _ = writeln!(out, "    }},");
}

/// Die Verstaerkung eines Zaubers (Issue #420).
///
/// # Warum hier geprueft wird und nicht erst im Kampf
///
/// `Buffs::apply` gibt `false` zurueck, wenn die Art ausserhalb des Vorrats
/// liegt — und `false` ist im Kampf nicht zu sehen: Der Zauber wirkt, der
/// Schaden faellt, nur die Verstaerkung bleibt aus. Ein Tippfehler in `kind`
/// waere damit ein Zauber, der leise nichts tut.
///
/// Dieselbe Haltung wie bei `consumesAll` zwei Funktionen weiter: lieber beim
/// Bauen scheitern als im Kampf still das Falsche rechnen. Die Gegenstelle im
/// Spiel ist `parseBuff` in `src/core/data.ts`, und sie prueft dasselbe — sonst
/// naehme eine Seite einen Datensatz an, den die andere ablehnt.
fn emit_buff(out: &mut String, spell: &Value, whose: &str) {
    let Some(buff) = spell.get("buff") else {
        let _ = writeln!(out, "        buff: None,");
        return;
    };
    let whose = format!("{whose}/buff");

    let kind = uint(buff, "kind", &whose);
    assert!(
        (kind as usize) < BUFF_KIND_CAPACITY,
        "{whose}: Verstaerkungs-Art {kind} liegt ausserhalb von 0..{}",
        BUFF_KIND_CAPACITY - 1
    );

    let factor = buff
        .get("factor")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("{whose}: Feld \"factor\" fehlt oder ist keine Zahl"));
    assert!(
        factor > 0.0,
        "{whose}: Verstaerkungs-Faktor {factor} muss groesser als 0 sein — ein \
         Faktor von null loeschte den Schaden, ein negativer kehrte ihn um"
    );
    // **Ein Faktor UNTER 1 macht eine Fliesskomma-Falle erreichbar** (Issue
    // #656). Das ist keine Design-Aussage — ein abschwaechender Verstaerker
    // waere voellig legitim. Es ist eine Rechen-Aussage, und sie steht hier,
    // weil sie beim EINFUEHREN auffallen muss und nicht Monate spaeter.
    //
    // `Buffs::factor` bildet ein PRODUKT ueber alle offenen Arten. Solange
    // jeder Einzelfaktor >= 1 ist, kann das Produkt nie knapp unter 1 landen —
    // gemessen: von 14 Verstaerkern liegt heute keiner darunter. Mit einem
    // Faktor < 1 wird es moeglich, und dann gilt (3999 Paare a * (1/a)
    // durchgerechnet, IEEE754-double):
    //
    //   568 davon ergeben 0.9999999999999999 statt 1.0   (14 Prozent)
    //
    // Der Kurzschluss `faktor == 1.0` greift dann NICHT. Das allein waere
    // folgenlos — die Rundung faengt es (0 Abweichungen ueber 100000 Werte).
    // Folgenreich ist die eine Stelle, die den Faktor UNGERUNDET weitergibt:
    // `update_buff_factor` -> `compute_damage`. Dort kippen 521 von 24000
    // Schadenswerten, sobald ein zweiter krummer Faktor mitspielt
    // (vulnerability 1.3: 221 von 4000, etwa base 5 -> 7 statt 6).
    //
    // WER HIER LANDET, weil er einen abschwaechenden Verstaerker bauen will:
    // Das ist erlaubt und gewollt. Nimm diese Zusicherung heraus UND baue die
    // Absicherung aus #656 AC2/AC3 — sonst zeigt sich der Fehler als
    // 1-Schaden-Abweichung, die niemandem auffaellt.
    assert!(
        factor >= 1.0,
        "{whose}: Verstaerkungs-Faktor {factor} liegt unter 1. Das ist nicht \
         verboten, aber es macht die Fliesskomma-Falle aus Issue #656 \
         erreichbar: Ein Produkt mit einem Faktor < 1 kann 0.9999999999999999 \
         ergeben statt 1.0, der Kurzschluss greift dann nicht, und ueber \
         update_buff_factor laeuft der Wert UNGERUNDET in compute_damage. \
         Lies den Kommentar ueber dieser Zeile, bevor du sie entfernst"
    );

    let gcds = uint(buff, "gcds", &whose);
    assert!(
        gcds > 0,
        "{whose}: Fensterlaenge {gcds} muss eine positive Zahl von Wartezeiten sein"
    );

    let _ = writeln!(out, "        buff: Some(Buff {{");
    let _ = writeln!(out, "            kind: {kind},");
    // `{:?}` und nicht `{}`: Die Debug-Ausgabe eines f64 ist verlustfrei
    // umkehrbar. Mit `{}` verlöre 1.0000000000000002 seine letzten Stellen, und
    // die beiden Seiten rechneten ab da mit verschiedenen Zahlen.
    let _ = writeln!(out, "            factor: {factor:?},");
    let _ = writeln!(out, "            gcds: {gcds},");
    // **Ohne Feld bleibt es beim Schaden** — das Verhalten vor #489, damit
    // bestehende Zauber unveraendert bleiben. Eine UNBEKANNTE Groesse bricht
    // dagegen ab, statt still im Schadens-Vorrat zu landen: Ein Tippfehler
    // gaebe sonst eine Verstaerkung, die wirkt, nur nicht dort, wo sie soll.
    let target = match buff.get("target").and_then(Value::as_str) {
        None | Some("damage") => "Damage",
        Some("control") => "Control",
        Some("resource") => "Resource",
        Some("tick") => "Tick",
        Some("stack") => "Stack",
        Some("cdreduce") => "CdReduce",
        Some("convert") => "Convert",
        Some("cap") => "Cap",
        Some(anderes) => panic!(
            "{whose}: unbekannte Zielgroesse \"{anderes}\" — erlaubt sind \
             damage, control, resource, tick, stack, cdreduce, convert"
        ),
    };
    let _ = writeln!(out, "            target: BuffTarget::{target},");
    let _ = writeln!(out, "        }}),");
}

fn emit_dot(out: &mut String, spell: &Value, whose: &str, resource_id: &str) {
    let Some(dot) = spell.get("dot") else {
        let _ = writeln!(out, "        dot: None,");
        return;
    };
    let whose = format!("{whose}/dot");

    // STEP1-SPEC 5: "reset" ist fuer V1 der einzige zulaessige Wert. Das Feld
    // existiert trotzdem, weil spaeter Zauber dazukommen koennen, die sich
    // anders verhalten — dann faellt es hier auf und wird bewusst erweitert.
    let mode = text(dot, "refreshMode", &whose);
    assert_eq!(
        mode, "reset",
        "{whose}: refreshMode \"{mode}\" ist nicht umgesetzt; \
         fuer Version 1 laesst STEP1-SPEC § 5 nur \"reset\" zu"
    );

    let _ = writeln!(out, "        dot: Some(Dot {{");
    let _ = writeln!(
        out,
        "            tick_damage: {},",
        uint(dot, "tickDamage", &whose)
    );
    let _ = writeln!(
        out,
        "            tick_interval_ms: {},",
        uint(dot, "tickIntervalMs", &whose)
    );
    let _ = writeln!(
        out,
        "            duration_ms: {},",
        uint(dot, "durationMs", &whose)
    );
    let _ = writeln!(
        out,
        "            resource_gain_per_tick: {},",
        resource_amount(dot, "resourceGainPerTick", &whose, resource_id)
    );
    // `decayPerTick` darf fehlen — die meisten Braende legen keine Stapel.
    // Null ist dann der neutrale Wert, und `add_decay_stacks` steigt bei null
    // sofort aus.
    let decay_per_tick = if dot.get("decayPerTick").is_some() {
        uint(dot, "decayPerTick", &whose)
    } else {
        0
    };
    let _ = writeln!(out, "            decay_per_tick: {decay_per_tick},");
    let _ = writeln!(out, "        }}),");
}

fn emit_spender(out: &mut String, spell: &Value, whose: &str, resource_id: &str) {
    let Some(spender) = spell.get("spender") else {
        let _ = writeln!(out, "        spender: None,");
        return;
    };
    let whose = format!("{whose}/spender");

    let named = text(spender, "resource", &whose);
    assert_eq!(
        named, resource_id,
        "{whose}: verbraucht \"{named}\", die Schule fuehrt aber \"{resource_id}\""
    );
    // `consumesAll: false` waere ein anderer Zauber-Typ (Teil-Verbrauch), den es
    // in Schritt 1 nicht gibt. Lieber hier abbrechen als still das Falsche rechnen.
    let consumes_all = boolean(spender, "consumesAll", &whose);
    assert!(
        consumes_all,
        "{whose}: consumesAll=false ist nicht umgesetzt; \
         Schritt 1 kennt nur Entlade-Zauber, die die Ressource vollstaendig verbrauchen"
    );

    let _ = writeln!(out, "        spender: Some(Spender {{");
    let _ = writeln!(
        out,
        "            damage_per_point: {},",
        uint(spender, "damagePerPoint", &whose)
    );
    let _ = writeln!(
        out,
        "            bonus_threshold: {},",
        uint(spender, "bonusThreshold", &whose)
    );
    let _ = writeln!(
        out,
        "            bonus_multiplier: {},",
        uint(spender, "bonusMultiplier", &whose)
    );
    let _ = writeln!(out, "        }}),");
}

/// Liest alle Boss-Dateien, nach Dateiname sortiert.
///
/// **Die Sortierung ist nicht Kosmetik.** `read_dir` liefert die Eintraege in
/// der Reihenfolge des Dateisystems, und die ist zwischen zwei Rechnern
/// verschieden. Ohne Sortierung stuenden die Bosse mal so und mal anders im
/// erzeugten Code — und jede Stelle, die ueber sie laeuft, waere nicht mehr
/// deterministisch (GAME-DESIGN 18.4).
fn read_bosses() -> Vec<(String, Value)> {
    let dir = Path::new(BOSSES_DIR);
    let mut dateien: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{BOSSES_DIR} nicht lesbar: {e}"))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    dateien.sort();

    assert!(
        !dateien.is_empty(),
        "{BOSSES_DIR} enthaelt keine Boss-Datei"
    );

    dateien
        .into_iter()
        .map(|path| {
            let name = path.display().to_string();
            let value = read_json_at(&path);
            pruefe_zeitleiste(&name, &value);
            (name, value)
        })
        .collect()
}

/// Die Zeitleiste eines Bosses muss in ihren Wiederholungs-Rahmen passen.
///
/// # Warum das hier steht und nicht im Prueffall
///
/// Diese Stelle erzeugt die Rust-Daten aus `data/bosses/`. Wer hier abbricht,
/// verhindert, dass ein widerspruechlicher Boss ueberhaupt uebersetzt wird —
/// statt ihn klaglos zu laden und den Fehler erst als Paritaetsbruch bei Takt
/// 4800 zu zeigen, wenn ueberhaupt jemand eine Aufzeichnung baut, die so weit
/// laeuft.
///
/// # Die SPANNWEITE entscheidet, nicht der Hoechstwert (Issue #565)
///
/// Ein Zyklus faengt beim ersten Eintrag an und hoert beim letzten auf. Der
/// letzte Eintrag eines Zyklus liegt bei `0*loop + max`, der erste des
/// naechsten bei `1*loop + min`. Vorwaerts geht es genau dann, wenn
///
/// ```text
/// loop > max - min
/// ```
///
/// Ein erster Entwurf der Regel lautete `loop > max` — sie haette
/// `test-metronome` angeschwaerzt (Eintraege 2000, 4000, 6000 bei loop 6000),
/// obwohl der dort einen tadellosen Takt alle zwei Sekunden schlaegt. Ein
/// Fehlalarm ist die teurere Richtung: Er kostet Vertrauen, und eine Pruefung,
/// der man nicht traut, wird abgeschaltet.
///
/// Bei GLEICHHEIT faellt der erste Eintrag des neuen Zyklus exakt auf den
/// letzten des alten — kein Rueckwaertslauf, aber zwei Ereignisse zur selben
/// Zeit. Deshalb ist die Grenze strikt.
fn pruefe_zeitleiste(name: &str, boss: &Value) {
    let Some(loop_ms) = boss.get("timelineLoopMs").and_then(Value::as_u64) else {
        return; // ohne Wiederholung gibt es keinen Rahmen, in den etwas passen muesste
    };
    let Some(eintraege) = boss.get("timeline").and_then(Value::as_array) else {
        return;
    };
    let zeiten: Vec<u64> = eintraege
        .iter()
        .filter_map(|e| e.get("atMs").and_then(Value::as_u64))
        .collect();
    if zeiten.len() < 2 {
        return; // ein einzelner Eintrag hat keine Spannweite
    }
    let max = zeiten.iter().copied().max().unwrap_or(0);
    let min = zeiten.iter().copied().min().unwrap_or(0);
    let spannweite = max - min;

    assert!(
        loop_ms > spannweite,
        "{name}: timelineLoopMs ist {loop_ms}, die Eintraege spannen aber \
         {spannweite} ms ({min} bis {max}). Der naechste Zyklus beginnt dann \
         VOR dem Ende des laufenden — die Zeitleiste laeuft rueckwaerts, und \
         Spiel und Simulator beantworten diese Lage verschieden. Entweder die \
         Eintraege in den Rahmen legen oder timelineLoopMs auf mindestens \
         {} erhoehen.",
        spannweite + 1
    );
}

/// Die `instance`-Werte, die eine **Spielinstanz** benennen (Issue #981, AC1).
///
/// # Warum eine Liste von INSTANZEN und keine von Bossen
///
/// Das Issue verbietet eine Liste von Boss-Kennungen, weil sie bei jedem neuen
/// Boss altert — lautlos. Eine Liste von Instanzen altert nicht mit den Bossen,
/// sondern nur mit dem Spielaufbau, und **ein unbekannter Wert bricht den Bau**
/// (AC4). Ein neuer Boss in `instance-2` braucht hier keine Zeile; ein neuer
/// Boss in `instance-5` erzwingt eine Entscheidung, statt still durchzugehen.
const SCORED_INSTANCES: [&str; 6] = [
    "prologue",
    "instance-1",
    "instance-2",
    "instance-3",
    "instance-4",
    "instance-5",
];

/// Die `instance`-Werte der Pruef- und Lastprofile.
///
/// Diese Bosse tragen absichtlich extreme Werte — `test-runaway` traegt eine
/// umgekehrte Siegbedingung, ein Grossteil von ihnen steht auf 999999 Leben,
/// damit der Kampf nicht von selbst endet. Sie liegen im selben
/// Verzeichnis wie die Spielbosse, und genau das ist die Gefahr: Eine Elite,
/// die versehentlich gegen einen von ihnen optimiert wurde, sieht im Ergebnis
/// vollkommen plausibel aus.
const PROBE_INSTANCES: [&str; 2] = ["test", "testing"];

/// Schreibt die Klassifikation als Daten, damit die Laufzeit sie **liest**
/// statt sie ein zweites Mal aufzuschreiben.
///
/// Zwei Listen an zwei Stellen laufen auseinander, und der Fehler ist still:
/// Der Bau bricht dann bei einem Wert, den die Laufzeit fuer gueltig haelt,
/// oder umgekehrt. Dieselbe Falle wie bei `RASTER_MS` — nur dass sie sich hier
/// ganz vermeiden laesst, weil es sich um Daten und nicht um eine Zahl im
/// Bauskript handelt.
fn emit_instance_classes(out: &mut String) {
    let _ = writeln!(
        out,
        "/// Die `instance`-Werte, die eine Spielinstanz benennen (Issue #981).\n\
         ///\n\
         /// **Erzeugt aus `build.rs`.** Dort steht die Regel, und dort bricht der\n\
         /// Bau, wenn ein Boss einen Wert traegt, der in keiner der beiden Listen\n\
         /// steht. Diese Konstante ist die Leseseite derselben Quelle."
    );
    let _ = writeln!(
        out,
        "pub const SCORED_INSTANCES: [&str; {}] = {:?};\n",
        SCORED_INSTANCES.len(),
        SCORED_INSTANCES
    );
    let _ = writeln!(
        out,
        "/// Die `instance`-Werte der Pruef- und Lastprofile (Issue #981)."
    );
    let _ = writeln!(
        out,
        "pub const PROBE_INSTANCES: [&str; {}] = {:?};\n",
        PROBE_INSTANCES.len(),
        PROBE_INSTANCES
    );
}

/// Prueft den `instance`-Wert eines Bosses (Issue #981, AC4).
///
/// **Bricht den Bau**, statt zu warnen. Ein Boss ohne zuordenbare Klasse waere
/// sonst entweder stillschweigend wertungsfaehig — dann verfaelscht ein
/// Lastprofil jede Elite — oder stillschweigend ausgeschlossen, dann fehlt ein
/// Spielboss in der Wertung und niemand sieht es.
fn pruefe_instanz(boss: &Value, whose: &str) {
    let instance = text(boss, "instance", whose);
    assert!(
        SCORED_INSTANCES.contains(&instance) || PROBE_INSTANCES.contains(&instance),
        "{whose}: instance \"{instance}\" ist keiner Klasse zugeordnet.\n\
         Wertungsfaehig sind {SCORED_INSTANCES:?}, Pruefprofile sind {PROBE_INSTANCES:?}.\n\
         Trag den Wert in build.rs ein — in SCORED_INSTANCES, wenn Spieler diese \
         Bosse sehen, sonst in PROBE_INSTANCES."
    );
}

fn emit_bosses(out: &mut String, bosses: &[(String, Value)], schulen: &[String]) {
    let _ = writeln!(
        out,
        "/// Anzahl der Bosse. Feste Groesse, damit Listen ueber Bosse ohne\n\
         /// Speicheranforderung auskommen."
    );
    let _ = writeln!(out, "pub const BOSS_COUNT: usize = {};\n", bosses.len());

    let _ = writeln!(
        out,
        "/// Alle Bosse, nach Dateiname sortiert. **Keine Hash-Abbildung** —\n\
         /// die Reihenfolge ist Teil des Determinismus (GAME-DESIGN 18.3)."
    );
    let _ = writeln!(out, "pub static BOSSES: [Boss; BOSS_COUNT] = [");
    for (name, boss) in bosses {
        pruefe_instanz(boss, name);
        emit_boss(out, boss, name, schulen);
    }
    let _ = writeln!(out, "];\n");

    let default = bosses
        .iter()
        .position(|(_, boss)| boss.get("id").and_then(Value::as_str) == Some(DEFAULT_BOSS_ID))
        .unwrap_or_else(|| {
            panic!("Kein Boss mit der Kennung \"{DEFAULT_BOSS_ID}\" in {BOSSES_DIR}")
        });

    let _ = writeln!(
        out,
        "/// Der Boss, gegen den gerechnet wird, wenn keiner gewaehlt ist."
    );
    let _ = writeln!(out, "pub static BOSS: &Boss = &BOSSES[{default}];\n");
}

fn emit_boss(out: &mut String, boss: &Value, whose: &str, schulen: &[String]) {
    let _ = writeln!(out, "    Boss {{");
    let _ = writeln!(out, "        id: {:?},", text(boss, "id", whose));
    let _ = writeln!(out, "        name: {:?},", text(boss, "name", whose));
    let _ = writeln!(
        out,
        "        instance: {:?},",
        text(boss, "instance", whose)
    );
    let _ = writeln!(out, "        hp: {},", uint(boss, "hp", whose));
    let _ = writeln!(
        out,
        "        overpressure_ms: {},",
        uint(boss, "overpressureMs", whose)
    );
    // Fehlt das Feld, wiederholt sich nichts — das ist der Normalfall und
    // gilt fuer jeden ausgelieferten Boss.
    let _ = writeln!(
        out,
        "        timeline_loop_ms: {},",
        match boss.get("timelineLoopMs").and_then(Value::as_u64) {
            Some(ms) => format!("Some({ms})"),
            None => "None".to_owned(),
        }
    );
    emit_phases(out, boss, whose);
    emit_energy(out, boss, whose);
    emit_doom(out, boss, whose);
    emit_stacks(out, boss, whose);
    emit_race(out, boss, whose);
    emit_boss_heal(out, boss, whose);
    emit_runaway(out, boss, whose);
    emit_escalation(out, boss, whose);
    emit_affliction(out, boss, whose);
    emit_timeline(out, boss, whose);
    emit_resistances(out, boss, whose, schulen);
    let _ = writeln!(out, "    }},");
}

/// Die Widerstaende des Bosses gegen die Schulen (GAME-DESIGN 11.4, Issue #328).
///
/// **Bis zum 2026-08-05 hat der Simulator dieses Feld ueberhaupt nicht
/// gelesen.** Das Spiel wendet es seit langem an (`combat.ts`,
/// `resistanceFactor` in den Schadensfaktoren) — hier stand nichts, und
/// `factors.resistance` blieb auf 1,0. Aufgefallen ist es nie, weil ALLE elf
/// Boss-Dateien `resistances: {}` tragen und die Suche im Spiel dann ebenfalls
/// 1,0 liefert. Eine Abweichung, die auf ihre erste Datenzeile gewartet hat.
///
/// Als Liste von Paaren und nicht als eine Zahl fuer `SCHOOL`: Die Zuordnung
/// steht so in den Daten, und ein Bau-Skript, das sie hier schon aufloest,
/// verlegt eine Regel aus den Daten in den Uebersetzungsvorgang.
/// Die Schulen, wie die **Zauberdaten** sie fuehren (Issue #981, AC7).
///
/// # Abgeleitet, nicht aufgeschrieben
///
/// Eine hingeschriebene Liste haette denselben Mangel wie die Sache, gegen die
/// sie schuetzt: Sie altert, sobald jemand eine Schule hinzufuegt, und der
/// Fehler waere still. Die Menge entsteht deshalb aus dem Feld `school` auf
/// **Datei**-Ebene — dort, wo es im Bestand steht; `common.json` traegt keines,
/// und seine Zauber gehoeren folglich keiner Schule an.
fn schulen(spells: &Value, weitere: &[(&str, &Value)]) -> Vec<String> {
    let mut namen: Vec<String> = Vec::new();
    let eigene = spells.get("school").and_then(Value::as_str);
    for schule in std::iter::once(eigene)
        .chain(
            weitere
                .iter()
                .map(|(_, quelle)| quelle.get("school").and_then(Value::as_str)),
        )
        .flatten()
    {
        if !namen.iter().any(|vorhanden| vorhanden == schule) {
            namen.push(schule.to_string());
        }
    }
    assert!(
        !namen.is_empty(),
        "Keine einzige Schule in den Zauberdaten gefunden — die Ableitung greift nicht, \
         und eine leere Menge liesse jeden Widerstands-Schluessel durchfallen"
    );
    namen
}

fn emit_resistances(out: &mut String, boss: &Value, whose: &str, schulen: &[String]) {
    let Some(map) = boss.get("resistances").and_then(Value::as_object) else {
        let _ = writeln!(out, "        resistances: &[],");
        return;
    };
    if map.is_empty() {
        let _ = writeln!(out, "        resistances: &[],");
        return;
    }
    let _ = writeln!(out, "        resistances: &[");
    for (school, faktor) in map {
        // **Der SCHLUESSEL, nicht nur der Wert** (Issue #981, AC7). Bis dahin
        // pruefte diese Schleife nur, dass der Wert eine Zahl ueber null ist.
        // Ein Eintrag `"CALZINATION": 0.5` ginge still durch: `resistance_for`
        // schlaegt mit Vorgabe 1.0 nach, der Widerstand griffe NIE, und der
        // Boss waere widerstandslos — waehrend seine Daten das Gegenteil sagen.
        assert!(
            schulen.iter().any(|bekannt| bekannt == school),
            "{whose}: Widerstand \"{school}\" benennt keine Schule.\n\
             Bekannt sind {schulen:?} — abgeleitet aus dem Feld \"school\" der \
             Zauberdateien, nicht aus einer Liste im Quelltext."
        );
        let wert = faktor
            .as_f64()
            .unwrap_or_else(|| panic!("{whose}: Widerstand \"{school}\" ist keine Zahl"));
        assert!(
            wert > 0.0,
            "{whose}: Widerstand \"{school}\" ist {wert} — ein Faktor von null \
             oder darunter macht den Schaden zunichte statt ihn zu mindern"
        );
        let _ = writeln!(out, "            ({school:?}, {wert:?}),");
    }
    let _ = writeln!(out, "        ],");
}

/// Das Takt-Raster des Spiels in Millisekunden (`TICK_MS` in `lib.rs`).
///
/// Hier noch einmal als Zahl, weil ein Bau-Skript die eigene Kiste nicht
/// einbinden kann. Wandert der Takt jemals, faellt es sofort auf: Der Test
/// `das_raster_im_bauskript_passt_zum_takt` vergleicht beide Werte.
const RASTER_MS: u64 = 10;

/// Die feste Abfolge der Boss-Handlungen (GAME-DESIGN 11.1).
///
/// Zwei Bedingungen werden hier erzwungen, weil beide Verstoesse **still**
/// waeren — der Kampf liefe weiter und rechnete nur falsch:
///
/// 1. **Aufsteigend sortiert.** Der Kampf haelt einen Zeiger und rueckt ihn
///    vor; ein Eintrag hinter einem spaeteren wuerde nie ausgeloest.
/// 2. **Auf dem Takt-Raster.** Das Spiel rechnet alle zehn Millisekunden, der
///    Simulator springt von Ereignis zu Ereignis. Ein Eintrag bei 333 ms
///    griffe im Simulator bei 333, im Spiel erst bei 340 — die Paritaet waere
///    hin, und zwar nur bei diesem einen Boss.
fn emit_timeline(out: &mut String, boss: &Value, whose: &str) {
    let Some(list) = boss.get("timeline").and_then(Value::as_array) else {
        let _ = writeln!(out, "        timeline: &[],");
        return;
    };
    if list.is_empty() {
        let _ = writeln!(out, "        timeline: &[],");
        return;
    }

    let mut voriger = 0_u64;
    let mut erster = true;
    let _ = writeln!(out, "        timeline: &[");
    for (index, entry) in list.iter().enumerate() {
        let whose = format!("{whose}/timeline[{index}]");
        let at = uint(entry, "atMs", &whose);
        assert!(
            erster || at > voriger,
            "{whose}: Die Zeitleiste muss aufsteigend stehen, \
             {at} ms kommt nach {voriger} ms"
        );
        assert!(
            at.is_multiple_of(RASTER_MS),
            "{whose}: {at} ms liegt nicht auf dem {RASTER_MS}-ms-Raster des \
             Spiels — der Simulator loeste frueher aus als das Spiel"
        );
        voriger = at;
        erster = false;

        // Die Art bestimmt, was der Eintrag tut. Ein unbekannter Wert bricht
        // den Bau ab, statt still zu einem Standardfall zu werden: Ein
        // Boss-Zauber, der versehentlich als Schlag gilt, waere nicht
        // unterbrechbar — und das saehe im Kampf wie ein Balance-Problem aus.
        let mechanic = text(entry, "mechanic", &whose);
        let kind = match mechanic {
            "strike" => "TimelineKind::Strike",
            "interruptibleCast" => "TimelineKind::InterruptibleCast",
            other => panic!(
                "{whose}: unbekannte Mechanik {other:?}. \
                 Bekannt sind \"strike\" und \"interruptibleCast\"."
            ),
        };
        let cast_time = opt_uint(entry, "castTimeMs", &whose, 0);
        assert!(
            cast_time.is_multiple_of(RASTER_MS),
            "{whose}: castTimeMs {cast_time} liegt nicht auf dem \
             {RASTER_MS}-ms-Raster des Spiels"
        );
        if mechanic == "interruptibleCast" {
            assert!(
                cast_time > 0,
                "{whose}: Ein unterbrechbarer Zauber ohne Wirkzeit kann nicht \
                 unterbrochen werden — er waere ein Schlag mit falschem Namen"
            );
        } else {
            assert!(
                cast_time == 0,
                "{whose}: Nur ein unterbrechbarer Zauber hat eine Wirkzeit"
            );
        }

        // Bricht dieser Angriff eine laufende Wirkzeit des SPIELERS ab?
        // (GAME-DESIGN 7.1, Issue #304.) Ohne Angabe `false` — ein bestehender
        // Boss aendert sein Verhalten durch dieses Feld nicht.
        let interrupts_player = entry
            .get("interruptsPlayer")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // **Die Phasen-Bindung** (Issue #557). Ohne Angabe `u32::MAX`: Das
        // Ereignis feuert immer, ein bestehender Boss aendert sein Verhalten
        // nicht. Dieselbe Regel wie bei `interruptsPlayer` darueber.
        let phase = entry
            .get("phase")
            .and_then(Value::as_u64)
            .and_then(|p| u32::try_from(p).ok())
            .map_or("u32::MAX".to_owned(), |p| p.to_string());

        let _ = writeln!(
            out,
            "            TimelineEntry {{ at_ms: {at}, mechanic: {:?}, \
             name: {:?}, kind: {kind}, damage: {}, cast_time_ms: {cast_time}, \
             interrupts_player: {interrupts_player}, phase: {phase} }},",
            mechanic,
            text(entry, "name", &whose),
            opt_uint(entry, "damage", &whose, 0)
        );
    }
    let _ = writeln!(out, "        ],");
}

/// Die Phasen-Schwellen eines Bosses (GAME-DESIGN 11.1).
///
/// Sie muessen **absteigend** stehen: Phase 0 ist der Start, und gewechselt
/// wird, sobald der Anteil der Lebenspunkte die naechste Schwelle
/// unterschreitet. Eine unsortierte Liste waere eine stille Falle — der Kampf
/// wuerde Phasen ueberspringen, ohne dass irgendwo etwas bricht.
fn emit_phases(out: &mut String, boss: &Value, whose: &str) {
    let Some(list) = boss.get("phases").and_then(Value::as_array) else {
        let _ = writeln!(out, "        phases: &[],");
        return;
    };
    if list.is_empty() {
        let _ = writeln!(out, "        phases: &[],");
        return;
    }

    let mut vorige = 101_u64;
    let _ = writeln!(out, "        phases: &[");
    for (index, phase) in list.iter().enumerate() {
        let whose = format!("{whose}/phases[{index}]");
        let at = uint(phase, "atHpPercent", &whose);
        assert!(
            at < vorige,
            "{whose}: Die Phasen-Schwellen muessen absteigend stehen, \
             {at} % kommt nach {vorige} %"
        );
        assert!(
            at > 0 && at < 100,
            "{whose}: Eine Schwelle bei {at} % ergibt keine Phase — \
             0 waere der Tod, 100 der Start"
        );
        vorige = at;
        let _ = writeln!(
            out,
            "            Phase {{ at_hp_percent: {at}, name: {:?} }},",
            text(phase, "name", &whose)
        );
    }
    let _ = writeln!(out, "        ],");
}

/// Die Energieleiste als zweite Uhr (GAME-DESIGN 11.2, Baustein 1).
fn emit_energy(out: &mut String, boss: &Value, whose: &str) {
    let energy = boss.get("energy");
    if energy.is_none() || energy == Some(&Value::Null) {
        let _ = writeln!(out, "        energy: None,");
        return;
    }
    let energy = energy.expect("auf None geprueft");
    let whose = format!("{whose}/energy");

    let per_step = uint(energy, "perStepMs", &whose);
    assert!(
        per_step > 0,
        "{whose}: perStepMs muss groesser als null sein, sonst stuende die Uhr"
    );
    // Dieselbe Falle wie bei der Zeitleiste, nur wiederkehrend: Bei 333 ms
    // stiege die Leiste im Simulator bei 333/666/999, im Spiel bei 340/670/1000
    // — der Abstand waechst mit jedem Schritt.
    assert!(
        per_step.is_multiple_of(RASTER_MS),
        "{whose}: perStepMs {per_step} liegt nicht auf dem {RASTER_MS}-ms-Raster \
         des Spiels — Simulator und Spiel liefen mit jedem Anstieg weiter auseinander"
    );
    let threshold = uint(energy, "threshold", &whose);
    assert!(
        threshold > 0,
        "{whose}: threshold muss groesser als null sein, sonst loeste sie sofort aus"
    );
    let reset_to = uint(energy, "resetTo", &whose);
    assert!(
        reset_to < threshold,
        "{whose}: resetTo {reset_to} liegt nicht unter threshold {threshold} — \
         die Leiste kaeme nie wieder herunter"
    );

    // **`onTrigger` ist PFLICHT, sobald `energy` steht** (Issue #568, AC2).
    //
    // Nicht optional, und das ist der ganze Punkt: Bis #568 stieg die Leiste,
    // erreichte ihre Schwelle, setzte zurueck — und keine Zeile im Kampf
    // fragte danach. Waere die Wirkung optional, liesse sich derselbe Zustand
    // jederzeit neu herstellen, ohne dass es auffiele. Der Bau bricht hier
    // ab, damit eine Leiste ohne Wirkung gar nicht erst entstehen kann.
    let trigger = energy.get("onTrigger").unwrap_or_else(|| {
        panic!(
            "{whose}: `energy` steht, aber `onTrigger` fehlt — eine Energieleiste \
             ohne Wirkung stiege wieder ins Leere (Issue #568). Trage \
             \"onTrigger\": {{ \"mechanic\": \"strike\", \"name\": ..., \"damage\": ... }} nach."
        )
    });
    let mechanic = trigger
        .get("mechanic")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{whose}/onTrigger: `mechanic` fehlt oder ist kein Text"));
    assert_eq!(
        mechanic, "strike",
        "{whose}/onTrigger: mechanic ist {mechanic:?}, erwartet wird \"strike\" — \
         der Ausloeser fuehrt einen Boss-Schlag ausserhalb der Zeitleiste. Ein \
         ausgeloester Boss-Zauber und der sofortige Tod als eigene Bauart sind \
         verworfen (der Tod ist ein strike mit hohem Schaden)."
    );
    let trigger_damage = uint(trigger, "damage", &format!("{whose}/onTrigger"));
    // Ein Schlag ueber null Schaden ist wieder eine Leiste ohne Wirkung, nur
    // eine Ebene tiefer versteckt — er bestuende jede Feldpruefung.
    assert!(
        trigger_damage > 0,
        "{whose}/onTrigger: damage ist 0 — ein Schlag ohne Schaden bewirkt nichts, \
         und die Leiste stiege wieder ins Leere"
    );
    let trigger_name = trigger
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("Overload");

    let _ = writeln!(out, "        energy: Some(Energy {{");
    let _ = writeln!(out, "            per_step_ms: {per_step},");
    let _ = writeln!(
        out,
        "            amount: {},",
        uint(energy, "amount", &whose)
    );
    let _ = writeln!(out, "            threshold: {threshold},");
    let _ = writeln!(out, "            reset_to: {reset_to},");
    let _ = writeln!(out, "            on_trigger: EnergyTrigger {{");
    let _ = writeln!(out, "                name: {trigger_name:?},");
    let _ = writeln!(out, "                damage: {trigger_damage},");
    let _ = writeln!(out, "            }},");
    let _ = writeln!(out, "        }}),");
}

/// Der toedliche Zaehler (GAME-DESIGN 11.2, Baustein 2; Issue #216).
///
/// Dieselben Pruefungen wie bei der Energieleiste, plus eine, die es dort nicht
/// gibt: Eine Ruecksetz-Handlung, die weniger nimmt als ein Takt bringt, ist
/// keine Rettung, sondern eine Verzoegerung mit Bedienaufwand. Der Zaehler
/// stiege trotz richtigen Spiels weiter — und das faende niemand, weil es wie
/// eine knappe Auslegung aussaehe.
fn emit_doom(out: &mut String, boss: &Value, whose: &str) {
    let doom = boss.get("doom");
    if doom.is_none() || doom == Some(&Value::Null) {
        let _ = writeln!(out, "        doom: None,");
        return;
    }
    let doom = doom.expect("auf None geprueft");
    let whose = format!("{whose}/doom");

    let per_step = uint(doom, "perStepMs", &whose);
    assert!(
        per_step > 0,
        "{whose}: perStepMs muss groesser als null sein, sonst stuende der Zaehler"
    );
    assert!(
        per_step.is_multiple_of(RASTER_MS),
        "{whose}: perStepMs {per_step} liegt nicht auf dem {RASTER_MS}-ms-Raster \
         des Spiels — Simulator und Spiel liefen mit jedem Anstieg weiter auseinander"
    );
    let threshold = uint(doom, "threshold", &whose);
    assert!(
        threshold > 0,
        "{whose}: threshold muss groesser als null sein, sonst toetete er sofort"
    );
    let amount = uint(doom, "amount", &whose);
    assert!(
        amount > 0,
        "{whose}: amount muss groesser als null sein, sonst stiege der Zaehler nie"
    );
    let clear_amount = uint(doom, "clearAmount", &whose);
    let clear_charges = uint(doom, "clearCharges", &whose);
    assert!(
        clear_charges == 0 || clear_amount > amount,
        "{whose}: clearAmount {clear_amount} nimmt nicht mehr, als ein Takt bringt \
         ({amount}) — der Zaehler stiege trotz der Handlung weiter"
    );

    let _ = writeln!(out, "        doom: Some(Doom {{");
    let _ = writeln!(out, "            per_step_ms: {per_step},");
    let _ = writeln!(out, "            amount: {amount},");
    let _ = writeln!(out, "            threshold: {threshold},");
    let _ = writeln!(out, "            clear_amount: {clear_amount},");
    let _ = writeln!(out, "            clear_charges: {clear_charges},");
    let _ = writeln!(out, "        }}),");
}

/// Der exponentielle Stapel (GAME-DESIGN 11.2, Baustein 3; Issue #216).
fn emit_stacks(out: &mut String, boss: &Value, whose: &str) {
    let stacks = boss.get("stacks");
    if stacks.is_none() || stacks == Some(&Value::Null) {
        let _ = writeln!(out, "        stacks: None,");
        return;
    }
    let stacks = stacks.expect("auf None geprueft");
    let whose = format!("{whose}/stacks");

    let per_step = uint(stacks, "perStepMs", &whose);
    assert!(
        per_step > 0,
        "{whose}: perStepMs muss groesser als null sein, sonst waechse der Stapel nie"
    );
    assert!(
        per_step.is_multiple_of(RASTER_MS),
        "{whose}: perStepMs {per_step} liegt nicht auf dem {RASTER_MS}-ms-Raster \
         des Spiels — Simulator und Spiel liefen mit jedem Schritt weiter auseinander"
    );
    let base_damage = uint(stacks, "baseDamage", &whose);
    assert!(
        base_damage > 0,
        "{whose}: baseDamage muss groesser als null sein, sonst richtete der Stapel nichts an"
    );
    // Ein Faktor von 1 waere linear — und damit ein anderer Baustein.
    let factor = uint(stacks, "factor", &whose);
    assert!(
        factor > 1,
        "{whose}: factor {factor} waechst nicht exponentiell — 11.2 verlangt 1 -> 3 -> 9"
    );
    let max_stacks = uint(stacks, "maxStacks", &whose);
    assert!(
        max_stacks > 0,
        "{whose}: maxStacks muss groesser als null sein — ohne Obergrenze verliesse \
         base_damage * factor^(n-1) den Wertebereich, und beide Seiten taeten das verschieden"
    );

    let _ = writeln!(out, "        stacks: Some(Stacks {{");
    let _ = writeln!(out, "            per_step_ms: {per_step},");
    let _ = writeln!(out, "            base_damage: {base_damage},");
    let _ = writeln!(out, "            factor: {factor},");
    let _ = writeln!(out, "            max_stacks: {max_stacks},");
    let _ = writeln!(out, "        }}),");
}

/// Das Wettrennen (GAME-DESIGN 11.2, Baustein 4; Issue #218).
fn emit_race(out: &mut String, boss: &Value, whose: &str) {
    let race = boss.get("race");
    if race.is_none() || race == Some(&Value::Null) {
        let _ = writeln!(out, "        race: None,");
        return;
    }
    let race = race.expect("auf None geprueft");
    let whose = format!("{whose}/race");

    let at_ms = uint(race, "atMs", &whose);
    assert!(
        at_ms.is_multiple_of(RASTER_MS),
        "{whose}: atMs {at_ms} liegt nicht auf dem {RASTER_MS}-ms-Raster des Spiels \
         — das Fenster begaenne auf beiden Seiten zu verschiedenen Zeitpunkten"
    );
    let window_ms = uint(race, "windowMs", &whose);
    assert!(
        window_ms > 0 && window_ms.is_multiple_of(RASTER_MS),
        "{whose}: windowMs {window_ms} muss groesser als null und auf dem \
         {RASTER_MS}-ms-Raster sein"
    );
    // Eine Schwelle von 0 waere sofort erreicht — das Fenster endete im selben
    // Takt, in dem es beginnt, und die Mechanik gaebe es nur auf dem Papier.
    let threshold = uint(race, "threshold", &whose);
    assert!(
        threshold > 0,
        "{whose}: threshold muss groesser als null sein, sonst endete das Fenster sofort"
    );

    let _ = writeln!(out, "        race: Some(Race {{");
    let _ = writeln!(out, "            at_ms: {at_ms},");
    let _ = writeln!(out, "            window_ms: {window_ms},");
    let _ = writeln!(out, "            threshold: {threshold},");
    let _ = writeln!(out, "        }}),");
}

/// Was ein Fehler an Fortschritt zurueckdreht (Baustein 5; Issue #218).
fn emit_boss_heal(out: &mut String, boss: &Value, whose: &str) {
    let heal = boss.get("bossHeal");
    if heal.is_none() || heal == Some(&Value::Null) {
        let _ = writeln!(out, "        boss_heal: None,");
        return;
    }
    let heal = heal.expect("auf None geprueft");
    let whose = format!("{whose}/bossHeal");
    let amount = uint(heal, "amount", &whose);
    assert!(
        amount > 0,
        "{whose}: amount muss groesser als null sein — eine Heilung von 0 saehe im \
         Datensatz aus wie eine, die etwas tut"
    );
    // **`interrupt` ist die Voreinstellung** (Issue #609): So bleibt jede
    // bestehende Bossdatei ohne Aenderung bei ihrem Verhalten. Ein unbekannter
    // Wert bricht den Bau, statt still auf den Standard zu fallen — dieselbe
    // Entscheidung wie im Leser des Spiels, nur eine Stufe haerter, weil hier
    // ein Panic zur Bauzeit moeglich ist.
    let ausloeser = match heal.get("on").and_then(Value::as_str) {
        None | Some("interrupt") => "Interrupt",
        Some("affliction") => {
            assert!(
                boss.get("affliction").is_some_and(|a| !a.is_null()),
                "{whose}: on ist \"affliction\", aber der Boss hat keine — die Heilung \
                 loeste nie aus und saehe im Datensatz aus wie eine Mechanik"
            );
            "Affliction"
        }
        Some(anderer) => panic!("{whose}: unbekannter Ausloeser \"{anderer}\" (on)"),
    };
    let _ = writeln!(out, "        boss_heal: Some(BossHeal {{");
    let _ = writeln!(out, "            amount: {amount},");
    let _ = writeln!(out, "            on: BossHealTrigger::{ausloeser},");
    let _ = writeln!(out, "        }}),");
}

/// Das umgekehrte Ziel (Baustein 6; Issue #247).
///
/// Dieselben Bedingungen wie im Leser des Spiels (`src/core/data.ts`) — beide
/// Seiten lesen dieselbe Datei und koennen einander nicht einbinden.
fn emit_runaway(out: &mut String, boss: &Value, whose: &str) {
    let run = boss.get("runaway");
    if run.is_none() || run == Some(&Value::Null) {
        let _ = writeln!(out, "        runaway: None,");
        return;
    }
    let run = run.expect("auf None geprueft");
    let whose = format!("{whose}/runaway");

    let per_step_ms = uint(run, "perStepMs", &whose);
    assert!(
        per_step_ms > 0 && per_step_ms.is_multiple_of(RASTER_MS),
        "{whose}: perStepMs {per_step_ms} muss groesser als null und auf dem \
         {RASTER_MS}-ms-Raster sein — sonst kippte die Reaktion auf beiden \
         Seiten zu verschiedenen Zeitpunkten"
    );
    let amount = uint(run, "amount", &whose);
    assert!(
        amount > 0,
        "{whose}: amount muss groesser als null sein, sonst laeuft die Reaktion nie davon"
    );

    let _ = writeln!(out, "        runaway: Some(Runaway {{");
    let _ = writeln!(out, "            per_step_ms: {per_step_ms},");
    let _ = writeln!(out, "            amount: {amount},");
    let _ = writeln!(out, "        }}),");
}

/// Die eskalierende Wiederholung (Baustein 7; Issue #240).
fn emit_escalation(out: &mut String, boss: &Value, whose: &str) {
    let esc = boss.get("escalation");
    if esc.is_none() || esc == Some(&Value::Null) {
        let _ = writeln!(out, "        escalation: None,");
        return;
    }
    let esc = esc.expect("auf None geprueft");
    let whose = format!("{whose}/escalation");

    let shrink = uint(esc, "castTimeShrinkMs", &whose);
    assert!(
        shrink > 0 && shrink.is_multiple_of(RASTER_MS),
        "{whose}: castTimeShrinkMs {shrink} muss groesser als null und auf dem \
         {RASTER_MS}-ms-Raster sein"
    );
    let min_cast = uint(esc, "minCastTimeMs", &whose);
    assert!(
        min_cast > 0 && min_cast.is_multiple_of(RASTER_MS),
        "{whose}: minCastTimeMs {min_cast} muss groesser als null und auf dem \
         {RASTER_MS}-ms-Raster sein — ohne Untergrenze waere der Baustein keine \
         Steigerung, sondern eine Unmoeglichkeit (11.7)"
    );

    let _ = writeln!(out, "        escalation: Some(Escalation {{");
    let _ = writeln!(out, "            cast_time_shrink_ms: {shrink},");
    let _ = writeln!(out, "            min_cast_time_ms: {min_cast},");
    let _ = writeln!(out, "        }}),");
}

/// Die Schwaechung ohne Heil-Loesung (Baustein 8; Issue #240).
fn emit_affliction(out: &mut String, boss: &Value, whose: &str) {
    let aff = boss.get("affliction");
    if aff.is_none() || aff == Some(&Value::Null) {
        let _ = writeln!(out, "        affliction: None,");
        return;
    }
    let aff = aff.expect("auf None geprueft");
    let whose = format!("{whose}/affliction");

    let at_ms = uint(aff, "atMs", &whose);
    assert!(
        at_ms.is_multiple_of(RASTER_MS),
        "{whose}: atMs {at_ms} liegt nicht auf dem {RASTER_MS}-ms-Raster"
    );
    let per_step_ms = uint(aff, "perStepMs", &whose);
    assert!(
        per_step_ms > 0 && per_step_ms.is_multiple_of(RASTER_MS),
        "{whose}: perStepMs {per_step_ms} muss groesser als null und auf dem \
         {RASTER_MS}-ms-Raster sein"
    );
    let damage = uint(aff, "damage", &whose);
    assert!(
        damage > 0,
        "{whose}: damage muss groesser als null sein, sonst richtet die Schwaechung nichts an"
    );

    let cleared: Vec<&str> = aff
        .get("clearedBy")
        .and_then(Value::as_array)
        .map(|liste| liste.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    // **Das "mehrere" aus 11.2 ist keine Empfehlung.** Ein Boss, der genau
    // einen Zauber verlangt, macht die Zusammenstellung zur Pflichtaufgabe —
    // und 11.3 sagt, dass kein Boss eine bestimmte Schule verlangen darf.
    assert!(
        cleared.len() >= 2,
        "{whose}: clearedBy nennt {} Zauber — 11.2 verlangt MEHRERE Loesungswege, \
         sonst ist man ohne den richtigen Zauber hilflos",
        cleared.len()
    );

    let _ = writeln!(out, "        affliction: Some(Affliction {{");
    let _ = writeln!(out, "            at_ms: {at_ms},");
    let _ = writeln!(out, "            per_step_ms: {per_step_ms},");
    let _ = writeln!(out, "            damage: {damage},");
    let _ = write!(out, "            cleared_by: &[");
    for id in &cleared {
        let _ = write!(out, "{id:?}, ");
    }
    let _ = writeln!(out, "],");
    let _ = writeln!(out, "        }}),");
}

/// Die Grundwerte des Spielers (GAME-DESIGN 6.1).
///
/// Aus Daten und nicht aus dem Code, weil sie **nicht Design** sind: Die
/// Zahlen fuer eingehenden Schaden stehen in GAME-DESIGN F.6 als „vor E4
/// auszuplanen" und sind nicht entschieden. Wer sie aendert, aendert eine
/// Datei — nicht eine Zeile Rust.
fn emit_player(out: &mut String, player: &Value) {
    let whose = "player/base.json";
    let vessel = uint(player, "vessel", whose);
    assert!(vessel > 0, "{whose}: vessel muss groesser als null sein");

    let ward = player
        .get("wardReduction")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("{whose}: Feld \"wardReduction\" fehlt"));
    assert!(
        (0.0..1.0).contains(&ward),
        "{whose}: wardReduction {ward} liegt nicht in [0, 1) — bei 1.0 waere \
         der Spieler unverwundbar, darueber wuerde ihn Schaden heilen"
    );

    // Der Balance-Deckel auf die Minderung (Issue #466). Er gilt fuer die
    // Minderung INSGESAMT — Basis plus Ausruestung —, und ein Deckel unter der
    // Basis waere keine Grenze, sondern eine Absenkung: Ausruestung minderte
    // dann weniger als gar keine.
    let ward_cap = uint(player, "wardCapPermille", whose);
    assert!(
        ward_cap as f64 > ward * 1000.0,
        "{whose}: wardCapPermille {ward_cap} muss ueber der Basis wardReduction \
         ({} Promille) liegen — sonst mindert Ausruestung weniger als keine",
        ward * 1000.0
    );

    let _ = writeln!(out, "/// Die Grundwerte des Spielers aus `data/player/`.");
    let _ = writeln!(
        out,
        "pub static PLAYER: &PlayerBase = &PlayerBase {{ vessel: {vessel}, ward_reduction: {ward:?}, ward_cap_permille: {ward_cap} }};\n"
    );
}

/// Das Wachstum der Grundwerte je Stufe (GAME-DESIGN 8.1; Issue #248).
///
/// **NUR die Wachstumstabelle, NICHT die Erfahrungskurve.** Der KAMPF misst
/// nicht Fortschritt: Was auf Stufe 50 passiert, haengt an den Grundwerten;
/// wie lange jemand fuer Stufe 50 braucht, an der Kurve — und das eine folgt
/// nicht aus dem anderen.
///
/// Seit #922 gibt es eine Stelle, die von der Kurve abhaengt: die
/// Fortschritts-Simulation. Sie bekommt sie aus [`emit_laufbahn`] in eine
/// eigene Datei, nicht hierher — und `parity_laufbahn_regeln.rs` haelt sie
/// gegen `grantXp` im Spiel, damit die zweite Stelle nicht still
/// auseinanderlaeuft.
///
/// Die Tabelle steht in Promille und ganzzahlig, weil `1,05^n` eine
/// Fliesskomma-Potenz ist: `Math.pow` in JavaScript und `f64::powf` hier sind
/// nicht garantiert bitgleich (GAME-DESIGN 18.4). Eine Tabelle rechnet gar
/// nicht, sie steht da.
fn emit_levels(out: &mut String, levels: &Value) {
    let whose = "player/levels.json";
    let max_level = uint(levels, "maxLevel", whose);
    assert!(max_level >= 1, "{whose}: maxLevel muss mindestens 1 sein");

    let faktoren: Vec<u64> = levels
        .get("factorPerMille")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("{whose}: Feld \"factorPerMille\" fehlt"))
        .iter()
        .map(|v| {
            v.as_u64()
                .unwrap_or_else(|| panic!("{whose}: Faktor ist keine ganze Zahl"))
        })
        .collect();

    assert!(
        faktoren.len() as u64 == max_level,
        "{whose}: {} Faktoren fuer {max_level} Stufen. Jede Stufe braucht genau einen — \
         sonst waechst der Spieler ab einer Stelle nicht mehr",
        faktoren.len()
    );
    assert!(
        faktoren.first() == Some(&1000),
        "{whose}: Stufe 1 traegt nicht 1000. Der Spieler startet mit den Werten aus \
         base.json, nicht darueber oder darunter"
    );
    for paar in faktoren.windows(2) {
        assert!(
            paar[1] > paar[0],
            "{whose}: die Faktoren steigen nicht durchgehend. Ein Aufstieg, der \
             schwaecher macht, ist keiner"
        );
    }

    let _ = writeln!(
        out,
        "/// Hoechste erreichbare Stufe (`data/player/levels.json`)."
    );
    let _ = writeln!(out, "pub const MAX_LEVEL: u32 = {max_level};\n");
    let _ = writeln!(
        out,
        "/// Faktor je Stufe in Promille. Index 0 ist Stufe 1 und traegt 1000."
    );
    let _ = write!(out, "pub static LEVEL_FACTOR_PER_MILLE: &[u32] = &[");
    for f in &faktoren {
        let _ = write!(out, "{f}, ");
    }
    let _ = writeln!(out, "];\n");
}

/// Die drei Wege der Koagulation (GAME-DESIGN 4.1, Issue #79).
///
/// Alle drei sind **optional**: Ein Zauber, der keiner von ihnen ist, nennt
/// die Felder gar nicht. Die drei Kalzinations-Zauber und der Unterbrecher
/// fuehren keines davon.
/// Die Felder der PUTREFACTION (Issue #97). Fehlen sie, stehen sie auf 0.
fn emit_putrefaction(out: &mut String, spell: &Value, whose: &str) {
    let stapel = spell
        .get("decay")
        .and_then(|d| d.get("stacks"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let je_stapel = spell
        .get("perDecayStackPercent")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let _ = writeln!(out, "        decay_stacks: {stapel},");
    let _ = writeln!(out, "        per_decay_stack_percent: {je_stapel},");
    // EMBER und PYROCLASM (#517). Beide Felder fehlen bei jedem bisherigen
    // Zauber — ein fehlendes Feld ist der Normalfall, nicht ein Mangel.
    let extend = spell
        .get("dotExtendTicks")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let _ = writeln!(out, "        dot_extend_ticks: {extend},");
    let verbraucht = spell
        .get("consumesDots")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(out, "        consumes_dots: {verbraucht},");
    // NECROSIS (#479) — der Entlader raeumt die Faeulnis-Stapel ab. Kein
    // `spender`: PUTREFACTION fuehrt keine Ressource, und die Pruefung weiter
    // unten (Zeile 706) haette den Bau zu Recht abgebrochen.
    let stapel_weg = spell
        .get("consumesDecayStacks")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let _ = writeln!(out, "        consumes_decay_stacks: {stapel_weg},");
    // HARDEN und SEPSIS (#523).
    //
    // **NULL IST HIER DER RICHTIGE VORGABEWERT, und das ist entschieden, nicht
    // uebersehen** (#1104): Die allermeisten Zauber verlaengern nichts und
    // sollen das Feld gar nicht erst nennen muessen. `uint()` waere falsch, es
    // machte das Feld zur Pflicht fuer alle sechzig Zauber. Null heisst
    // "verlaengert nicht" und wird unten in `combat.rs` genau so gelesen —
    // anders als bei einer GROESSE, wo eine stille Null wie eine gesetzte
    // aussaehe.
    //
    // **DIE RASTERPRUEFUNG DAGEGEN HAT GEFEHLT.** Bis zum 2026-08-22 waren
    // dies die einzigen beiden `Ms`-Felder ohne sie, waehrend zwanzig andere
    // Stellen `is_multiple_of(RASTER_MS)` erzwingen. Gemessen an
    // `extendShieldMs = 12001`: Bau und beide Pruefsuiten liefen durch, und
    // der Wert wirkte als 12010 — `combat.ts` vergleicht gegen `seitBeginn`,
    // das nur Vielfache des Rasters annimmt. Die Angabe log um neun
    // Millisekunden, und nichts sagte etwas.
    let schild_ms = spell
        .get("extendShieldMs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(
        schild_ms.is_multiple_of(RASTER_MS),
        "{whose}: extendShieldMs {schild_ms} liegt nicht auf dem {RASTER_MS}-ms-Raster \
         — der Wert wuerde erst im naechsten Takt wirken"
    );
    let stapel_ms = spell
        .get("extendDecayMs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    assert!(
        stapel_ms.is_multiple_of(RASTER_MS),
        "{whose}: extendDecayMs {stapel_ms} liegt nicht auf dem {RASTER_MS}-ms-Raster \
         — der Wert wuerde erst im naechsten Takt wirken"
    );
    let _ = writeln!(out, "        extend_shield_ms: {schild_ms},");
    let _ = writeln!(out, "        extend_decay_ms: {stapel_ms},");
}

/// Die Synergie-Beziehung (Issue #318). Fehlt sie, steht sie auf "kein Partner".
///
/// **Das Fenster steht in WARTEZEITEN, nicht in Millisekunden** — GAME-DESIGN
/// 327. Die Millisekunden entstehen erst im Kampf, durch dieselbe
/// hasted()-Stelle wie jede andere Zeit.
fn emit_synergy(out: &mut String, spell: &Value) {
    let partner = spell
        .get("synergy")
        .and_then(|s| s.get("partner"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let fenster = spell
        .get("synergy")
        .and_then(|s| s.get("windowGcds"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let _ = writeln!(out, "        synergy_partner: {partner:?},");
    let _ = writeln!(out, "        synergy_window_gcds: {fenster},");
}

fn emit_coagulation(out: &mut String, spell: &Value, whose: &str) {
    match spell.get("shield") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        shield: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/shield");
            let amount = uint(node, "amount", &whose);
            let duration = uint(node, "durationMs", &whose);
            let stacks = uint(node, "maxStacks", &whose);
            assert!(
                amount > 0,
                "{whose}: Eine Schutzschicht ohne Betrag schuetzt nichts"
            );
            assert!(
                duration.is_multiple_of(RASTER_MS),
                "{whose}: durationMs {duration} liegt nicht auf dem {RASTER_MS}-ms-Raster"
            );
            assert!(stacks > 0, "{whose}: maxStacks muss mindestens 1 sein");
            let _ = writeln!(
                out,
                "        shield: Some(Shield {{ amount: {amount}, duration_ms: {duration}, max_stacks: {stacks} }}),"
            );
        }
    }

    match spell.get("heal") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        heal: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/heal");
            let amount = uint(node, "amount", &whose);
            assert!(amount > 0, "{whose}: Eine Heilung ohne Betrag heilt nichts");
            // `overMs` darf fehlen — die meisten Heilungen kommen sofort an.
            let over_ms = opt_uint(node, "overMs", &whose, 0);
            let tick_interval_ms = opt_uint(node, "tickIntervalMs", &whose, 0);
            // `perMissingPercent` darf fehlen — die meisten Heilungen haengen
            // nicht am fehlenden Leben. `opt_uint` weist eine negative Zahl
            // von selbst ab; die TypeScript-Seite meldet sie beim Lesen.
            let per_missing_percent = opt_uint(node, "perMissingPercent", &whose, 0);
            let from_shield_percent = opt_uint(node, "fromShieldPercent", &whose, 0);
            // Ueber 100 Prozent waere kein Umwandeln mehr, sondern eine
            // Vervielfachung. Die TypeScript-Seite prueft dasselbe beim Lesen.
            assert!(
                from_shield_percent <= 100,
                "{whose}: fromShieldPercent={from_shield_percent} liegt ueber 100"
            );
            if over_ms > 0 {
                // **Auf dem Raster, sonst bliebe ein Reststueck liegen.** Die
                // TypeScript-Seite prueft dieselben drei Bedingungen beim Lesen
                // (`src/core/data.ts`); hier stehen sie ein zweites Mal, weil
                // ein Datensatz, der die eine Seite passiert und die andere
                // nicht, erst in der Paritaets-Pruefung auffiele — und dort als
                // Rechenfehler aussaehe statt als Datenfehler.
                assert!(
                    over_ms.is_multiple_of(RASTER_MS),
                    "{whose}: overMs={over_ms} liegt nicht auf dem {RASTER_MS}-ms-Raster"
                );
                assert!(
                    tick_interval_ms > 0,
                    "{whose}: overMs={over_ms} steht, tickIntervalMs fehlt — ohne Abstand \
                     liefe die Heilung in jedem {RASTER_MS}-ms-Takt an"
                );
                assert!(
                    tick_interval_ms.is_multiple_of(RASTER_MS),
                    "{whose}: tickIntervalMs={tick_interval_ms} liegt nicht auf dem \
                     {RASTER_MS}-ms-Raster"
                );
                assert!(
                    over_ms.is_multiple_of(tick_interval_ms),
                    "{whose}: overMs={over_ms} ist kein Vielfaches von \
                     tickIntervalMs={tick_interval_ms} — das letzte Stueck bliebe ohne Schritt"
                );
            }
            let _ = writeln!(
                out,
                "        heal: Some(Heal {{ amount: {amount}, over_ms: {over_ms}, \
                 tick_interval_ms: {tick_interval_ms}, \
                 per_missing_percent: {per_missing_percent}, \
                 from_shield_percent: {from_shield_percent} }}),"
            );
        }
    }

    match spell.get("decayWard") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        decay_ward: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/decayWard");
            let permille = uint(node, "permillePerStack", &whose);
            let duration = uint(node, "durationMs", &whose);
            // Null bewirkt nichts, und eine Schwaechung ohne Laufzeit endet in
            // dem Augenblick, in dem sie beginnt.
            assert!(
                permille > 0,
                "{whose}: permillePerStack {permille} bewirkt nichts"
            );
            assert!(duration > 0, "{whose}: decayWard ohne Laufzeit wirkt nie");
            assert!(
                duration.is_multiple_of(RASTER_MS),
                "{whose}: durationMs {duration} liegt nicht auf dem {RASTER_MS}-ms-Raster"
            );
            let _ = writeln!(
                out,
                "        decay_ward: Some(DecayWard {{ permille_per_stack: {permille}, duration_ms: {duration} }}),"
            );
        }
    }

    match spell.get("decayCap") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        decay_cap: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/decayCap");
            let bonus = uint(node, "bonus", &whose);
            let duration = uint(node, "durationMs", &whose);
            assert!(bonus > 0, "{whose}: Zuschlag {bonus} bewirkt nichts");
            assert!(duration > 0, "{whose}: decayCap ohne Laufzeit wirkt nie");
            // **Die Kapazitaetsgrenze, und sie bricht den BAU.** Ein Zuschlag
            // ueber die feste Struktur hinaus haette keinen Platz im Feld, und
            // die Pruefsumme laese Werte, die es nicht gibt. Lieber hier
            // abbrechen als still das Falsche rechnen — dieselbe Haltung wie
            // bei `consumesAll` (Zeile 709).
            //
            // Die Zahlen stehen in `fight.rs` und werden hier abgeschrieben;
            // `tests/decay_cap.rs` haelt beide gegeneinander fest.
            assert!(
                5 + bonus <= 8,
                "{whose}: Zuschlag {bonus} fuehrt ueber die Kapazitaet 8 hinaus \
                 (Grenze 5)"
            );
            assert!(
                duration.is_multiple_of(RASTER_MS),
                "{whose}: durationMs {duration} liegt nicht auf dem {RASTER_MS}-ms-Raster"
            );
            let _ = writeln!(
                out,
                "        decay_cap: Some(DecayCap {{ bonus: {bonus}, duration_ms: {duration} }}),"
            );
        }
    }

    match spell.get("conversion") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        conversion: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/conversion");
            let share = uint(node, "sharePercent", &whose);
            let duration = uint(node, "durationMs", &whose);
            // Ueber 100 % waere kein Schutz mehr, sondern eine Vervielfachung:
            // Der Spieler bekaeme mehr Schutz, als er Schaden macht.
            assert!(
                share > 0 && share <= 100,
                "{whose}: sharePercent {share} liegt nicht in (0, 100]"
            );
            assert!(
                duration.is_multiple_of(RASTER_MS),
                "{whose}: durationMs {duration} liegt nicht auf dem {RASTER_MS}-ms-Raster"
            );
            let _ = writeln!(
                out,
                "        conversion: Some(Conversion {{ share_percent: {share}, duration_ms: {duration} }}),"
            );
        }
    }

    match spell.get("reflect") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        reflect: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/reflect");
            let share = uint(node, "sharePercent", &whose);
            let duration = uint(node, "durationMs", &whose);
            // Dieselbe Grenze wie bei der Umwandlung: Ueber 100 % gaebe der
            // Spieler mehr zurueck, als bei ihm ankommt.
            assert!(
                share > 0 && share <= 100,
                "{whose}: sharePercent {share} liegt nicht in (0, 100]"
            );
            assert!(
                duration > 0,
                "{whose}: durationMs {duration} — ein Fenster ohne Dauer"
            );
            assert!(
                duration.is_multiple_of(RASTER_MS),
                "{whose}: durationMs {duration} liegt nicht auf dem {RASTER_MS}-ms-Raster"
            );
            let _ = writeln!(
                out,
                "        reflect: Some(Reflect {{ share_percent: {share}, duration_ms: {duration} }}),"
            );
        }
    }
}

/// Die Umwandlung aus einer schulfremden Quelle (Issue #520, EFFLUX).
///
/// Steht neben `emit_fixation` und nicht darin: `Control` verlangt
/// Kontroll-Punkte, `Convert` kommt ohne aus. Geteilt wird die Senke im
/// Kampf, nicht die Verpackung in den Daten.
fn emit_convert(out: &mut String, spell: &Value, whose: &str) {
    match spell.get("convert") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        convert: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/convert");
            let from = text(node, "from", &whose);
            assert_eq!(
                from, "cooldown",
                "{whose}: unbekannte Umwandlungs-Quelle \"{from}\"; \
                 umgesetzt ist nur \"cooldown\""
            );
            let per_second = uint(node, "perSecond", &whose);
            assert!(per_second > 0, "{whose}: Die Umwandlung hat keinen Betrag");
            let _ = writeln!(
                out,
                "        convert: Some(Convert {{ per_second: {per_second} }}),"
            );
        }
    }
}

/// Kontroll-Punkte und die drei Wirkungen (FIXATION, Issue #100).
fn emit_fixation(out: &mut String, spell: &Value, whose: &str) {
    let _ = writeln!(
        out,
        "        control_gain: {},",
        opt_uint(spell, "controlGain", whose, 0)
    );

    match spell.get("control") {
        None | Some(Value::Null) => {
            let _ = writeln!(out, "        control: None,");
        }
        Some(node) => {
            let whose = format!("{whose}/control");
            let cost = uint(node, "cost", &whose);
            assert!(
                cost > 0,
                "{whose}: Eine Wirkung ohne Kosten waere keine Ausgabe von \
                 Kontroll-Punkten, sondern eine passive Wirkung — und genau \
                 das ist FIXATION laut GAME-DESIGN 4.1 NICHT"
            );

            // Die drei Wirkungen stehen woertlich im Design. Ein unbekannter
            // Wert bricht den Bau ab, statt still zu einem Standardfall zu
            // werden: Ein Zauber, der versehentlich einen Schlag verzoegert
            // statt eine Phase, saehe im Kampf wie ein Balance-Problem aus.
            let (effekt, feld) = match text(node, "effect", &whose) {
                "slowCast" => ("ControlEffect::SlowCast", "amountMs"),
                "delayStrike" => ("ControlEffect::DelayStrike", "amountMs"),
                "holdPhase" => ("ControlEffect::HoldPhase", "amountPercent"),
                // Die zwei neuen aus Issue #478: WEDGE rechnet Punkte in
                // Schaden um (der Notausgang gegen Bosse ohne Mechanik),
                // CAPSTONE loest alle drei bisherigen auf einmal aus.
                "convertToDamage" => ("ControlEffect::ConvertToDamage", "amountPerPoint"),
                "allThree" => ("ControlEffect::AllThree", "amountMs"),
                // PIN (Issue #478): amountMs ist hier die FENSTERLAENGE, mit
                // der ein Schlag ohne Wirkzeit unterbrechbar wird — nicht eine
                // Verzoegerung wie bei delayStrike. Dasselbe Feld, andere
                // Bedeutung; deshalb steht der Grund im Zauber-Kommentar.
                "makeInterruptible" => ("ControlEffect::MakeInterruptible", "amountMs"),
                other => panic!(
                    "{whose}: unbekannte Wirkung {other:?}. Bekannt sind \
                     \"slowCast\", \"delayStrike\", \"holdPhase\", \
                     \"convertToDamage\", \"allThree\" und \"makeInterruptible\"."
                ),
            };
            let amount = uint(node, feld, &whose);
            assert!(amount > 0, "{whose}: {feld} muss groesser als null sein");
            if feld == "amountMs" {
                assert!(
                    amount.is_multiple_of(RASTER_MS),
                    "{whose}: {amount} ms liegt nicht auf dem {RASTER_MS}-ms-Raster"
                );
            } else {
                // Ueber 100 Prozentpunkte waere keine Verschiebung mehr,
                // sondern eine Schwelle unter null — die Phase kaeme nie.
                assert!(
                    amount < 100,
                    "{whose}: amountPercent {amount} verschoebe die Schwelle \
                     unter null; die Phase kaeme dann nie"
                );
            }

            // **Die Prozent-Senkung eigens, mit Rueckfall auf `amount`**
            // (Issue #478). Nur CAPSTONE fuehrt beide Felder, weil nur er alle
            // drei Wirkungen ausloest und die zwei Einheiten sprechen.
            //
            // Der Rueckfall haelt alle anderen unveraendert: ANCHOR traegt nur
            // `amountPercent`, also steht seine Zahl ohnehin in `amount` — und
            // fuer `slowCast`, `delayStrike` und die zwei aus #478 wird das
            // Feld nie gelesen.
            //
            // **Rueckfall auf 0, NICHT auf `amount`.** Ein erster Entwurf
            // nahm `amount` als Standard — bei `slowCast` sind das 2000
            // Millisekunden, und die Zusicherung darunter haette den Bau
            // abgebrochen, weil 2000 keine gueltige Prozentzahl ist.
            //
            // 0 ist richtig und deckt sich mit der TS-Seite
            // (`number(knoten['amountPercent'], 0)`): Wo die Wirkung keine
            // Prozente spricht, wird das Feld nie gelesen, und `fight.rs`
            // faellt bei 0 auf `amount` zurueck — wie `combat.ts`.
            let amount_percent = opt_uint(node, "amountPercent", &whose, 0);
            assert!(
                amount_percent < 100,
                "{whose}: amountPercent {amount_percent} verschoebe die Schwelle \
                 unter null; die Phase kaeme dann nie"
            );
            let _ = writeln!(
                out,
                "        control: Some(Control {{ cost: {cost}, effect: {effekt}, \
                 amount: {amount}, amount_percent: {amount_percent} }}),"
            );
        }
    }
}

// ------------------------------------------- Wirkungsbeschreibung (Issue #911)

/// Die Boss-Schluessel, deren Semantik dieser Bau **kennt**.
///
/// Alles, was hier nicht steht, wird zu einer Signatur **ohne** Semantik — und
/// loest damit den konservativen Rueckfall aus. Das ist der Zweck der Liste:
/// Ein neues Zusatz-System in den Bossdaten soll nicht still als „harmlos"
/// durchlaufen, sondern langsam werden, bis jemand es einordnet.
///
/// > Bei der Planung stand „sieben Zusatz-Systeme in den Bossdaten" — nachgezaehlt
/// > waren es elf. Genau diese Liste haette den Irrtum gemeldet.
const BEKANNTE_BOSS_SCHLUESSEL: &[&str] = &[
    // Kopfdaten, keine Wirkung
    "schema",
    "id",
    "name",
    "instance",
    "hp",
    "overpressureMs",
    "_comment",
    // Die elf Zusatz-Systeme
    "timeline",
    "phases",
    "energy",
    "doom",
    "affliction",
    "race",
    "runaway",
    "stacks",
    "escalation",
    "bossHeal",
    "timelineLoopMs",
    "resistances",
];

/// Ein Knoten der Wirkungsbeschreibung, wie er in den erzeugten Quelltext geht.
///
/// Alle Felder sind fertige Rust-Ausdruecke. Der Aufbau geht ueber
/// [`Knoten::konservativ`] und die Aktualisierungs-Syntax: Wer ein Feld
/// weglaesst, bekommt den vorsichtigen Wert und nicht den guenstigen.
struct Knoten {
    op: String,
    trigger: &'static str,
    target: &'static str,
    reads: String,
    writes: String,
    timing: &'static str,
    stacking: &'static str,
    randomness: String,
    monotonie: String,
    bounds: String,
    control_flow: String,
    origin: String,
}

impl Knoten {
    /// Ein Knoten, der nichts ueber sich behauptet.
    ///
    /// Jedes Feld steht auf dem Wert, der den Rueckfall ausloest. Wer etwas
    /// weiss, ueberschreibt es einzeln — wer nichts weiss, bleibt langsam.
    fn konservativ(op: &str, origin: String) -> Knoten {
        Knoten {
            op: op.to_string(),
            trigger: "Trigger::Unknown",
            target: "Target::Unknown",
            reads: "StateMask::UNKNOWN".to_string(),
            writes: "StateMask::UNKNOWN".to_string(),
            timing: "TimingSemantics::Unknown",
            stacking: "StackingSemantics::Unknown",
            randomness: "RandomnessSemantics::Unknown".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: "ControlFlowEffect::UNKNOWN".to_string(),
            origin,
        }
    }
}

/// Schreibt einen Knoten als Rust-Literal.
fn emit_knoten(out: &mut String, k: &Knoten) {
    let p = "crate::effects";
    let _ = writeln!(out, "    {p}::EffectNode {{");
    let _ = writeln!(out, "        op: {p}::EffectOp::{},", k.op);
    let _ = writeln!(out, "        trigger: {p}::{},", k.trigger);
    let _ = writeln!(out, "        target: {p}::{},", k.target);
    let _ = writeln!(out, "        reads: {},", maske(&k.reads));
    let _ = writeln!(out, "        writes: {},", maske(&k.writes));
    let _ = writeln!(out, "        timing: {p}::{},", k.timing);
    let _ = writeln!(out, "        stacking: {p}::{},", k.stacking);
    let _ = writeln!(out, "        randomness: {p}::{},", k.randomness);
    let _ = writeln!(out, "        monotonicity: {p}::{},", k.monotonie);
    let _ = writeln!(out, "        numeric_bounds: {p}::{},", k.bounds);
    let _ = writeln!(out, "        control_flow_effect: {p}::{},", k.control_flow);
    let _ = writeln!(out, "        origin: {:?},", k.origin);
    let _ = writeln!(out, "    }},");
}

/// Setzt den Modulpfad vor jeden `StateMask`-Namen eines Masken-Ausdrucks.
///
/// Damit bleiben die Masken oben in diesem Bau kurz lesbar
/// (`StateMask::RESOURCE.union(StateMask::DOTS)`) und im erzeugten Quelltext
/// trotzdem vollstaendig qualifiziert — dort gibt es kein `use`.
fn maske(ausdruck: &str) -> String {
    ausdruck.replace("StateMask::", "crate::effects::StateMask::")
}

/// Verbindet mehrere Masken zu einem Ausdruck.
fn masken(teile: &[&str]) -> String {
    match teile.split_first() {
        None => "StateMask::EMPTY".to_string(),
        Some((erste, rest)) => {
            let mut s = format!("StateMask::{erste}");
            for r in rest {
                s.push_str(&format!(".union(StateMask::{r})"));
            }
            s
        }
    }
}

/// Erzeugt die Wirkungsbeschreibung aus Zaubern und Bossen (Issue #911).
///
/// # Warum das hier steht und nicht von Hand gepflegt wird
///
/// Eine gepflegte Liste driftet gegen die Daten, und der Fehler ist still.
/// Diese Knoten entstehen aus denselben Dateien wie die Zauber-Tabelle
/// daneben — dieselbe Reihenfolge, dieselbe Quelle
/// ([`zauber_liste`]).
fn emit_effects(
    out: &mut String,
    spells: &Value,
    weitere: &[(&str, &Value)],
    bosses: &[(String, Value)],
) {
    // **Vor allem anderen**: Ohne vollstaendige Zuordnung prueft der Rest zu
    // wenig, und das faellt sonst nicht auf.
    pruefe_zuordnung_vollstaendig();

    let mut knoten: Vec<Knoten> = Vec::new();
    for (spell, _, _) in zauber_liste(spells, weitere) {
        let vorher = knoten.len();
        knoten_fuer_zauber(&mut knoten, spell);
        // Sofort nach dem Erzeugen und nicht am Ende: So nennt die Meldung den
        // Zauber, bei dem es klemmt, statt nur eine Gesamtzahl.
        let bausteine: Vec<String> = knoten[vorher..].iter().map(|k| k.op.clone()).collect();
        pruefe_wirkung_vollstaendig(spell, &bausteine);
    }
    for (datei, boss) in bosses {
        let vorher = knoten.len();
        knoten_fuer_boss(&mut knoten, datei, boss);
        let herkuenfte: Vec<String> = knoten[vorher..].iter().map(|k| k.origin.clone()).collect();
        pruefe_boss_wirkung_vollstaendig(boss, datei, &herkuenfte);
    }

    let _ = writeln!(
        out,
        "/// Die Wirkungsbeschreibung aller Zauber und Bosse (Issue #911).\n\
         ///\n\
         /// **Erzeugt aus `data/*.json`, nicht gepflegt.** Wer einen Zauber\n\
         /// aendert, aendert diese Liste mit; wer ein unbekanntes Feld in eine\n\
         /// Bossdatei schreibt, bekommt einen Knoten ohne Semantik und damit\n\
         /// ueberall den konservativen Rueckfall."
    );
    let _ = writeln!(
        out,
        "pub static EFFECT_NODES: &[crate::effects::EffectNode] = &["
    );
    for k in &knoten {
        emit_knoten(out, k);
    }
    let _ = writeln!(out, "];\n");
}

/// Die Knoten eines einzelnen Zaubers.
///
/// # Die Semantik hier ist gemessen, nicht angenommen
///
/// - **Brand-Effekte sind `Live`, nicht `Snapshot`**: `Fight::dot_tick` setzt
///   Widerstand und Stapel-Zuschlag bei **jedem** Tick neu und ruft erst dann
///   `compute_damage`. Ein festgehaltener Wert waere hier schlicht falsch.
/// - **Brand-Ticks ziehen aus `Stream::DotTick`**, nicht aus `Stream::Crit`
///   (Issue #602): Ein Tick darf die Wuerfe der gewirkten Zauber nicht
///   verschieben.
/// - **Heilung wuerfelt nicht.** `Stream::Heal` existiert, wird im Kampf aber
///   an keiner Stelle gezogen — gemessen ueber alle Aufrufe von `roll_crit`.
fn knoten_fuer_zauber(knoten: &mut Vec<Knoten>, spell: &Value) {
    let id = spell
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("<ohne id>")
        .to_string();
    let zahl = |feld: &str| spell.get(feld).and_then(Value::as_u64).unwrap_or(0);
    let ja = |feld: &str| spell.get(feld).and_then(Value::as_bool).unwrap_or(false);
    let hat = |feld: &str| spell.get(feld).is_some();

    let kanal = zahl("channelMs") > 0;

    if zahl("baseDamage") > 0 {
        knoten.push(Knoten {
            trigger: if kanal {
                "Trigger::OnTick"
            } else {
                "Trigger::OnCast"
            },
            target: "Target::Boss",
            reads: masken(&["RESOURCE", "BUFFS", "DECAY_STACKS", "RNG"]),
            writes: masken(&["BOSS_HP", "RNG"]),
            // Ein Kanal liest bei jedem Tick neu, ein einzelner Wurf nicht.
            timing: if kanal {
                "TimingSemantics::Live"
            } else {
                "TimingSemantics::Immediate"
            },
            stacking: "StackingSemantics::None",
            randomness: "RandomnessSemantics::CritRoll".to_string(),
            monotonie: monotonie_steigend(),
            // Nach oben offen: Verstaerkungen und Stapel-Zuschlaege heben den
            // Grundwert an. Eine Obergrenze waere hier eine Behauptung.
            bounds: "Bounds { min: Some(0), max: None }".to_string(),
            // Der Schaden selbst verzweigt nicht — WANN ein Kampf endet, ist
            // Sache der Boss-Semantik und steht an deren Knoten.
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("DealDamage", id.clone())
        });
    }

    if hat("dot") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Boss",
            reads: masken(&["BUFFS", "DECAY_STACKS", "RNG"]),
            writes: masken(&["DOTS", "BOSS_HP", "DECAY_STACKS", "RNG"]),
            // Gemessen in `Fight::dot_tick`: Widerstand und Zuschlaege werden
            // beim Tick gesetzt, nicht beim Auflegen.
            timing: "TimingSemantics::Live",
            stacking: "StackingSemantics::Refresh",
            // **Der echte Strom, nicht sein Name** (Issue #966). Hier stand
            // `"dot-tick"`, waehrend der Strom in `rng.rs` `"dotTick"` heisst —
            // die Zeichenkette zeigte auf nichts, und nichts hat es gemeldet.
            randomness: "RandomnessSemantics::Stream(crate::rng::Stream::DotTick)".to_string(),
            monotonie: monotonie_steigend(),
            bounds: "Bounds { min: Some(0), max: None }".to_string(),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ApplyDot", id.clone())
        });
    }

    if hat("buff") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["BUFFS"]),
            writes: masken(&["BUFFS"]),
            timing: "TimingSemantics::Snapshot",
            stacking: "StackingSemantics::Refresh",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds { min: Some(0), max: None }".to_string(),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ApplyBuff", id.clone())
        });
    }

    if ressourcen_gewinn(spell) > 0 || hat("spender") {
        let gewinn = ressourcen_gewinn(spell) as i64;
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["RESOURCE"]),
            writes: masken(&["RESOURCE"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Additive",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: monotonie_steigend(),
            // Der Entlader kennt seine Obergrenze nicht aus diesem Feld — dann
            // bleibt die Schranke offen statt geraten.
            bounds: if hat("spender") {
                "Bounds { min: None, max: Some(0) }".to_string()
            } else {
                format!("Bounds::exact(0, {gewinn})")
            },
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ModifyResource", id.clone())
        });
    }

    if zahl("cooldownReductionMs") > 0 {
        let wert = zahl("cooldownReductionMs") as i64;
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::AllCooldowns",
            reads: masken(&["COOLDOWNS"]),
            writes: masken(&["COOLDOWNS"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Additive",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: monotonie_steigend(),
            // Hier IST die Schranke eng, und das ist der Unterschied: Nur
            // deshalb bleibt die Zahl moeglicher Aktionen ableitbar.
            bounds: format!("Bounds::exact(0, {wert})"),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ReduceCooldown", id.clone())
        });
    }

    if zahl("cooldownResetCharges") > 0 {
        let ladungen = zahl("cooldownResetCharges") as i64;
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::OneCooldown",
            reads: masken(&["COOLDOWNS"]),
            writes: masken(&["COOLDOWNS"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::None",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: monotonie_steigend(),
            bounds: format!("Bounds::exact(0, {ladungen})"),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ResetCooldown", id.clone())
        });
    }

    if ja("freesNextGcd") || ja("doublesNextCast") || ja("refundsInterruptedGcd") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["GCD"]),
            writes: masken(&["GCD"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Replace",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            // Genau EIN naechster Wurf, nie mehr — deshalb eine enge Schranke.
            bounds: "Bounds::exact(0, 1)".to_string(),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("FreeNextGcd", id.clone())
        });
    }

    if hat("heal") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["PLAYER_HP"]),
            writes: masken(&["PLAYER_HP"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Additive",
            // Gemessen: `Stream::Heal` wird im Kampf nirgends gezogen.
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds { min: Some(0), max: None }".to_string(),
            // Am Erhalt des Spielers haengt der Abbruch des Kampfes.
            control_flow: "ControlFlowEffect { branches_on_damage: false, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("Heal", id.clone())
        });
    }

    if hat("shield") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["SHIELD"]),
            writes: masken(&["SHIELD"]),
            timing: "TimingSemantics::Snapshot",
            stacking: "StackingSemantics::Replace",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds { min: Some(0), max: None }".to_string(),
            control_flow: "ControlFlowEffect { branches_on_damage: false, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("Shield", id.clone())
        });
    }

    if hat("conversion") || hat("convert") || hat("reflect") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["SHIELD", "PLAYER_HP", "BOSS_HP"]),
            writes: masken(&["SHIELD", "PLAYER_HP", "BOSS_HP"]),
            // Die Umwandlung liest den Schaden, wenn er anfaellt — also live.
            timing: "TimingSemantics::Live",
            stacking: "StackingSemantics::Replace",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: "ControlFlowEffect { branches_on_damage: true, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("ConvertDamage", id.clone())
        });
    }

    // ---- Wirkungen auf den Boss-Zustand ----------------------------------
    //
    // Alle mit `ChangeBossState`, dem Baustein fuer Bosszustaende ohne eigene
    // Variante. **Kein neuer Baustein**: Die Aufzaehlung aus AC0 ist fest, und
    // ein zusaetzlicher waere hier auch sachlich falsch — was diese Felder
    // gemeinsam haben, ist genau, dass sie am Boss etwas veraendern.
    //
    // Ohne diesen Block bekamen 13 von 70 Zaubern gar keinen Knoten: die
    // sieben Kontroll-Zauber, `purge`, `quench`, `harden`, `sepsis`, `decay`
    // und `putrid-bloom`. Gemessen, nicht vermutet.
    if hat("control") || ja("clearsDoom") || ja("interrupt") || ja("consumesDecayStacks") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Boss",
            reads: masken(&["BOSS_SYSTEMS", "TIMELINE", "DECAY_STACKS"]),
            writes: masken(&["BOSS_SYSTEMS", "TIMELINE", "DECAY_STACKS"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Replace",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            // Kontrolle greift in die Zeitleiste ein: WANN der Boss schlaegt,
            // entscheidet mit ueber Leben und Tod des Spielers.
            control_flow: "ControlFlowEffect { branches_on_damage: false, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("ChangeBossState", id.clone())
        });
    }

    // Zersetzungs-Stapel liegen auf dem BOSS, nicht beim Spieler — so steht es
    // auch in `Fight::dot_tick`: „Stapel sind keine Schul-Ressource, sondern
    // liegen auf dem Boss".
    if hat("decay") || ja("consumesDots") || zahl("extendDecayMs") > 0 {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Boss",
            reads: masken(&["DECAY_STACKS", "DOTS"]),
            writes: masken(&["DECAY_STACKS", "DOTS"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Additive",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ChangeBossState", id.clone())
        });
    }

    // Kontrollpunkte sind eine eigene Groesse neben der Schul-Ressource.
    if zahl("controlGain") > 0 {
        let gewinn = zahl("controlGain") as i64;
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["RESOURCE"]),
            writes: masken(&["RESOURCE"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Additive",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: monotonie_steigend(),
            bounds: format!("Bounds::exact(0, {gewinn})"),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ModifyResource", id.clone())
        });
    }

    // Zeitlich begrenzte Zustaende des Spielers: die angehobene Stapel-Grenze,
    // die Schwaechung nach Stapelzahl und das Synergie-Fenster.
    if hat("decayCap") || hat("decayWard") || hat("synergy") {
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["BUFFS", "DECAY_STACKS"]),
            writes: masken(&["BUFFS"]),
            timing: "TimingSemantics::Snapshot",
            stacking: "StackingSemantics::Replace",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ApplyBuff", id.clone())
        });
    }

    // Einen laufenden Brand verlaengern.
    if zahl("dotExtendTicks") > 0 {
        let ticks = zahl("dotExtendTicks") as i64;
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Boss",
            reads: masken(&["DOTS"]),
            writes: masken(&["DOTS"]),
            timing: "TimingSemantics::Live",
            stacking: "StackingSemantics::Extend",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: monotonie_steigend(),
            bounds: format!("Bounds::exact(0, {ticks})"),
            control_flow: "ControlFlowEffect::NONE".to_string(),
            ..Knoten::konservativ("ApplyDot", id.clone())
        });
    }

    // Die Schutzschicht verlaengern (HARDEN).
    if zahl("extendShieldMs") > 0 {
        let ms = zahl("extendShieldMs") as i64;
        knoten.push(Knoten {
            trigger: "Trigger::OnCast",
            target: "Target::Player",
            reads: masken(&["SHIELD"]),
            writes: masken(&["SHIELD"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Extend",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: format!("Bounds::exact(0, {ms})"),
            control_flow: "ControlFlowEffect { branches_on_damage: false, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("Shield", id.clone())
        });
    }
}

/// Der Monotonie-**Hinweis** fuer eine steigende Wirkung.
///
/// Immer mit Bezug. Ein Hinweis ohne Bezug waere ein Zertifikat, das keines
/// ist — und genau davor warnt AC6.
fn monotonie_steigend() -> String {
    "LocalMonotonicityHint { direction: crate::effects::MonotonicDirection::Increasing, \
     domain: crate::effects::MonotonicityDomain::ExpectedDamageStableTrace }"
        .to_string()
}

/// Die Knoten eines Bosses — je Zusatz-System eine Signatur.
///
/// # AC4: auch eine Signatur deklariert ihre Semantik
///
/// Liefe jede ungewoehnliche Mechanik als allgemeine Signatur durch, braeche der
/// Bau **nie** mehr — und die 68 geplanten Bosse tragen per Design je eine
/// einmalige Regel. Deshalb bekommt jedes **bekannte** System hier seine
/// Semantik, und jedes **unbekannte** einen Knoten ohne: langsam, nie falsch.
fn knoten_fuer_boss(knoten: &mut Vec<Knoten>, datei: &str, boss: &Value) {
    let id = boss
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or(datei)
        .to_string();

    let Some(felder) = boss.as_object() else {
        return;
    };

    // `traegt_wirkung` und nicht `contains_key`: 33 Bossdateien FUEHREN das Feld
    // `phases`, aber nur 7 haben eine nicht-leere Liste. Ein Knoten fuer
    // `phases: []` beschriebe eine Wirkung, die es nicht gibt — der Fehler in
    // die andere Richtung, und er macht Aussagen wie „welche Ereigniskanaele
    // braucht dieser Kampf" unbrauchbar.
    if felder.get("phases").is_some_and(traegt_wirkung) {
        knoten.push(Knoten {
            trigger: "Trigger::OnPhaseChange",
            target: "Target::Boss",
            reads: masken(&["BOSS_HP", "PHASE"]),
            writes: masken(&["PHASE", "TIMELINE"]),
            timing: "TimingSemantics::Immediate",
            stacking: "StackingSemantics::Replace",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: "ControlFlowEffect { branches_on_damage: false, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("ChangePhase", format!("{id}/phases"))
        });
    }

    // ---- Die Zeitleiste: was der Boss TUT ---------------------------------
    //
    // **Das war die groesste Luecke der ersten Fassung.** Alle 35 Bosse fuehren
    // eine `timeline`, und sie erzeugte keinen einzigen Knoten — die Schlaege
    // des Bosses, seine Kernmechanik, standen in keiner Beschreibung. Dass
    // trotzdem jeder Boss Knoten hatte, lag allein an `resistances`; die
    // Zaehlung „35 von 35 Bossen beschrieben" war also richtig und wertlos.
    //
    // Je vorkommender Mechanik ein eigener Knoten, mit eigener Herkunft: Sonst
    // liesse sich ein Boss, der nur schlaegt, nicht von einem unterscheiden,
    // der auch unterbrechbar zaubert.
    let mut mechaniken: Vec<&str> = boss
        .get("timeline")
        .and_then(Value::as_array)
        .map(|eintraege| {
            eintraege
                .iter()
                .filter_map(|e| e.get("mechanic").and_then(Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    mechaniken.sort_unstable();
    mechaniken.dedup();
    for mechanik in mechaniken {
        knoten.push(Knoten {
            trigger: "Trigger::OnTimeline",
            target: "Target::Player",
            reads: masken(&["TIMELINE", "PHASE", "SHIELD"]),
            writes: masken(&["PLAYER_HP", "SHIELD", "TIMELINE"]),
            timing: "TimingSemantics::Live",
            stacking: "StackingSemantics::None",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds { min: Some(0), max: None }".to_string(),
            // Ein Schlag kann den Spieler toeten — hier haengt der Abbruch des
            // Kampfes wirklich am Schaden, anders als beim Spieler-Schaden.
            control_flow: "ControlFlowEffect { branches_on_damage: true, \
                           branches_on_hp: true, branches_on_victory: false, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("DealDamage", format!("{id}/timeline:{mechanik}"))
        });
    }

    // Der Ueberdruck — die harte Zeitgrenze des Kampfes. Er entscheidet ueber
    // Sieg und Niederlage und ist damit der Fall, vor dem AC6 ausdruecklich
    // warnt: Hier traegt kein Monotonie-Hinweis.
    if boss
        .get("overpressureMs")
        .and_then(Value::as_u64)
        .is_some_and(|v| v > 0)
    {
        knoten.push(Knoten {
            op: "CustomSignature(crate::effects::SignatureId(\"overpressure\"))".to_string(),
            trigger: "Trigger::OnTimeline",
            target: "Target::Player",
            reads: masken(&["TIMELINE", "BOSS_HP"]),
            writes: masken(&["PLAYER_HP"]),
            timing: "TimingSemantics::Live",
            stacking: "StackingSemantics::None",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: "ControlFlowEffect { branches_on_damage: false, \
                           branches_on_hp: true, branches_on_victory: true, unknown: false }"
                .to_string(),
            ..Knoten::konservativ("", format!("{id}/overpressureMs"))
        });
    }

    // Die uebrigen zehn bekannten Systeme. Jedes bekommt eine Signatur MIT
    // Semantik — der Unterschied zu einem unbekannten Feld ist genau das.
    for (schluessel, verzweigt_am_sieg) in [
        ("energy", false),
        ("doom", true),
        ("affliction", false),
        ("race", true),
        ("runaway", true),
        ("stacks", false),
        ("escalation", false),
        ("bossHeal", false),
        ("timelineLoopMs", false),
        ("resistances", false),
    ] {
        // Aus demselben Grund wie bei `phases` oben: `energy: null` bei 29 von
        // 33 Bossen, `resistances: {}` bei 31 von 35. Wer hier `contains_key`
        // fragt, beschreibt lauter Wirkungen, die es nicht gibt.
        if !felder.get(schluessel).is_some_and(traegt_wirkung) {
            continue;
        }
        knoten.push(Knoten {
            op: format!("CustomSignature(crate::effects::SignatureId({schluessel:?}))"),
            trigger: "Trigger::OnTimeline",
            target: "Target::Boss",
            reads: masken(&["BOSS_SYSTEMS", "TIMELINE", "BOSS_HP", "PLAYER_HP"]),
            writes: masken(&["BOSS_SYSTEMS", "BOSS_HP", "PLAYER_HP"]),
            timing: "TimingSemantics::Live",
            stacking: "StackingSemantics::Additive",
            randomness: "RandomnessSemantics::Deterministic".to_string(),
            monotonie: "LocalMonotonicityHint::UNKNOWN".to_string(),
            bounds: "Bounds::UNKNOWN".to_string(),
            control_flow: format!(
                "ControlFlowEffect {{ branches_on_damage: false, branches_on_hp: true, \
                 branches_on_victory: {verzweigt_am_sieg}, unknown: false }}"
            ),
            ..Knoten::konservativ("", format!("{id}/{schluessel}"))
        });
    }

    // Und jetzt das Gegenteil: alles, was dieser Bau NICHT kennt.
    for schluessel in felder.keys() {
        if BEKANNTE_BOSS_SCHLUESSEL.contains(&schluessel.as_str()) {
            continue;
        }
        // Ein Knoten ganz ohne Semantik. Jede registrierte Analyse geht darauf
        // in den Rueckfall — gewollt, und der einzige Weg, ein neues
        // Zusatz-System nicht still durchgehen zu lassen.
        knoten.push(Knoten::konservativ(
            &format!("CustomSignature(crate::effects::SignatureId({schluessel:?}))"),
            format!("{id}/{schluessel}"),
        ));
    }
}

/// Der Ressourcen-Gewinn eines Zaubers.
///
/// # Warum das nicht `as_u64` sein darf
///
/// `resourceGain` ist ein **Objekt** (`{"heat": 3}`), keine Zahl — als einziges
/// der Zahlenfelder eines Zaubers. Ein `as_u64()` darauf liefert stillschweigend
/// `None`, und der Zauber verschwindet aus der Wirkungsbeschreibung, ohne dass
/// irgendetwas rot wird.
///
/// **Gemessen beim Bau von Issue #911:** Genau so fehlten vier von sechs
/// Zaubern mit Ressourcen-Wirkung (kindle, bellows, ember, test-channel-heal) —
/// die Liste sah vollstaendig aus und war es nicht. Dagegen steht jetzt
/// zusaetzlich [`pruefe_wirkung_vollstaendig`], das ohne diesen Leser auskommt.
fn ressourcen_gewinn(spell: &Value) -> u64 {
    match spell.get("resourceGain") {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
        // Es gibt genau eine Ressource (`pruefe_nur_eine_ressource`), deshalb
        // ist das Maximum zugleich der einzige Wert.
        Some(Value::Object(map)) => map.values().filter_map(Value::as_u64).max().unwrap_or(0),
        _ => 0,
    }
}

/// Die Felder eines Zaubers, die eine Wirkung beschreiben.
///
/// Bewusst **ohne** die Kopfdaten (`id`, `name`, `unlockLevel`, Zeiten): Wer
/// eines dieser Felder traegt, muss in der Wirkungsbeschreibung vorkommen.
const WIRKUNGSFELDER: &[&str] = &[
    "baseDamage",
    "dot",
    "buff",
    "resourceGain",
    "spender",
    "cooldownReductionMs",
    "cooldownResetCharges",
    "freesNextGcd",
    "doublesNextCast",
    "refundsInterruptedGcd",
    "heal",
    "shield",
    "conversion",
    "convert",
    "reflect",
    // Die zweite Haelfte, nachgetragen: Ohne sie bekamen 13 von 70 Zaubern
    // gar keinen Knoten, und diese Pruefung schwieg dazu — sie kannte die
    // Felder nicht. Eine Liste, die nur die Haelfte nennt, sichert nur die
    // Haelfte ab, und das sieht von aussen aus wie eine ganze.
    "control",
    "controlGain",
    "clearsDoom",
    "interrupt",
    "decay",
    "decayCap",
    "decayWard",
    "dotExtendTicks",
    "consumesDots",
    "consumesDecayStacks",
    "extendShieldMs",
    "extendDecayMs",
    "perDecayStackPercent",
    "synergy",
];

/// Ob ein JSON-Wert eine Wirkung traegt — **typunabhaengig**.
///
/// Zahl ungleich null, `true`, nicht-leeres Objekt, nicht-leere Liste, nicht-leerer
/// Text. Diese Funktion kennt kein einziges Feld beim Namen und benutzt keinen
/// der Leser oben. Genau darin liegt ihr Wert: Sie kann nicht denselben
/// Typfehler machen wie der Leser, den sie absichert.
fn traegt_wirkung(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Bricht den Bau, wenn ein Zauber mit Wirkung keinen Knoten bekommen hat.
///
/// # Warum diese Pruefung nicht zirkulaer ist
///
/// Sie stellt die Frage auf der **JSON-Ebene** — traegt das Feld irgendeinen
/// nicht-leeren Wert? — und vergleicht mit dem, was [`knoten_fuer_zauber`]
/// tatsaechlich erzeugt hat. Liest der Leser ein Feld falsch (etwa ein Objekt
/// als Zahl), entsteht kein Knoten, und **diese** Pruefung meldet es. Eine
/// Pruefung, die denselben Leser benutzt, haette denselben blinden Fleck.
fn pruefe_wirkung_vollstaendig(spell: &Value, erzeugte_bausteine: &[String]) {
    let id = spell
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("<ohne id>");

    // **Jedes Feld einzeln**, nicht „hat der Zauber irgendeinen Knoten".
    // Der Unterschied ist der ganze Punkt: Ein Zauber mit Grundschaden UND
    // Heilung hat einen Knoten, auch wenn die Heilung ungelesen bleibt — eine
    // Zaehlung ueber ZAUBER haette das nie gemeldet.
    let mut fehlend: Vec<(&str, &str)> = Vec::new();
    for (feld, erwarteter_baustein) in FELD_ZU_BAUSTEIN {
        if !spell.get(*feld).is_some_and(traegt_wirkung) {
            continue;
        }
        if !erzeugte_bausteine.iter().any(|b| b == erwarteter_baustein) {
            fehlend.push((feld, erwarteter_baustein));
        }
    }
    assert!(
        fehlend.is_empty(),
        "Zauber \"{id}\" traegt Wirkungsfelder, fuer die kein passender Knoten entstanden ist: \
         {fehlend:?} (Feld, erwarteter Baustein). Erzeugt wurden {erzeugte_bausteine:?}. \
         Das ist der stille Fehler aus Issue #911 — wahrscheinlich liest `knoten_fuer_zauber` \
         eines dieser Felder mit dem falschen JSON-Typ (so geschehen bei `resourceGain`, einem \
         Objekt und keiner Zahl)."
    );
}

/// Welcher Baustein zu welchem Zauber-Feld gehoert.
///
/// # Warum diese Tabelle keine „gepflegte Liste" im schlechten Sinn ist
///
/// Sie ist eine, aber eine **abgesicherte**: [`pruefe_zuordnung_vollstaendig`]
/// bricht den Bau, wenn ein Feld aus [`WIRKUNGSFELDER`] hier fehlt. Eine Liste,
/// deren Unvollstaendigkeit laut auffaellt, ist etwas anderes als eine, deren
/// Drift still bleibt — und genau darum geht es in Issue #911.
///
/// Mehrere Felder duerfen auf denselben Baustein zeigen: `shield` und
/// `extendShieldMs` sind beide eine Schutzschicht.
const FELD_ZU_BAUSTEIN: &[(&str, &str)] = &[
    ("baseDamage", "DealDamage"),
    ("perDecayStackPercent", "DealDamage"),
    ("dot", "ApplyDot"),
    ("dotExtendTicks", "ApplyDot"),
    ("buff", "ApplyBuff"),
    ("decayCap", "ApplyBuff"),
    ("decayWard", "ApplyBuff"),
    ("synergy", "ApplyBuff"),
    ("resourceGain", "ModifyResource"),
    ("spender", "ModifyResource"),
    ("controlGain", "ModifyResource"),
    ("cooldownReductionMs", "ReduceCooldown"),
    ("cooldownResetCharges", "ResetCooldown"),
    ("freesNextGcd", "FreeNextGcd"),
    ("doublesNextCast", "FreeNextGcd"),
    ("refundsInterruptedGcd", "FreeNextGcd"),
    ("heal", "Heal"),
    ("shield", "Shield"),
    ("extendShieldMs", "Shield"),
    ("conversion", "ConvertDamage"),
    ("convert", "ConvertDamage"),
    ("reflect", "ConvertDamage"),
    ("control", "ChangeBossState"),
    ("clearsDoom", "ChangeBossState"),
    ("interrupt", "ChangeBossState"),
    ("consumesDecayStacks", "ChangeBossState"),
    ("decay", "ChangeBossState"),
    ("consumesDots", "ChangeBossState"),
    ("extendDecayMs", "ChangeBossState"),
];

/// Bricht den Bau, wenn ein Wirkungsfeld keinen Baustein zugeordnet bekommt.
///
/// Ohne diese Pruefung waere [`FELD_ZU_BAUSTEIN`] genau die Sorte Liste, gegen
/// die Issue #911 gebaut ist: Wer ein Feld zu [`WIRKUNGSFELDER`] hinzufuegt und
/// die Zuordnung vergisst, bekaeme eine Pruefung, die dieses Feld stillschweigend
/// uebergeht — und die Luecke saehe aus wie Abdeckung.
fn pruefe_zuordnung_vollstaendig() {
    let ohne: Vec<&&str> = WIRKUNGSFELDER
        .iter()
        .filter(|f| !FELD_ZU_BAUSTEIN.iter().any(|(feld, _)| feld == *f))
        .collect();
    assert!(
        ohne.is_empty(),
        "Diese Wirkungsfelder haben keinen Eintrag in FELD_ZU_BAUSTEIN: {ohne:?}. Damit \
         wuerde `pruefe_wirkung_vollstaendig` sie stillschweigend uebergehen, und die Luecke \
         saehe aus wie Abdeckung."
    );
}

/// Boss-Felder, die keine Wirkung beschreiben, sondern den Boss benennen.
///
/// Alles andere muss in der Wirkungsbeschreibung vorkommen — geprueft von
/// [`pruefe_boss_wirkung_vollstaendig`].
const BOSS_KOPFDATEN: &[&str] = &["schema", "id", "name", "instance", "hp", "_comment"];

/// Bricht den Bau, wenn ein wirkungstragendes Boss-Feld keinen Knoten hat.
///
/// # Das Gegenstueck zu [`pruefe_wirkung_vollstaendig`], und es hat gefehlt
///
/// Auf der Zauber-Seite war die Frage „hat jeder Zauber seine Beschreibung?"
/// nachgetragen worden — auf der Boss-Seite nicht, und dort war die groessere
/// Luecke: **Alle 35 Bosse fuehren eine `timeline`, und sie erzeugte keinen
/// einzigen Knoten.** Die Schlaege des Bosses, seine Kernmechanik, standen in
/// keiner Beschreibung.
///
/// Auffallen konnte das nicht, weil jeder Boss trotzdem Knoten hatte — allein
/// wegen `resistances`. Die Zaehlung „35 von 35 Bossen beschrieben" war richtig
/// und wertlos: Sie zaehlte Bosse, nicht Wirkungen. **Diese Pruefung zaehlt
/// Wirkungen.**
fn pruefe_boss_wirkung_vollstaendig(boss: &Value, datei: &str, herkuenfte: &[String]) {
    let id = boss.get("id").and_then(Value::as_str).unwrap_or(datei);
    let Some(felder) = boss.as_object() else {
        return;
    };
    let mut fehlend: Vec<&str> = Vec::new();
    for (schluessel, wert) in felder {
        if BOSS_KOPFDATEN.contains(&schluessel.as_str()) {
            continue;
        }
        if !traegt_wirkung(wert) {
            // Ein leeres oder auf `null` gesetztes Feld beschreibt nichts und
            // soll auch keinen Knoten haben.
            continue;
        }
        // Die Zeitleiste erzeugt Knoten je Mechanik, also mit Doppelpunkt im
        // Namen; die uebrigen genau einen mit dem Feldnamen.
        let praefix = format!("{id}/{schluessel}");
        if !herkuenfte
            .iter()
            .any(|h| *h == praefix || h.starts_with(&format!("{praefix}:")))
        {
            fehlend.push(schluessel);
        }
    }
    assert!(
        fehlend.is_empty(),
        "Boss \"{id}\" traegt die Felder {fehlend:?} mit Inhalt, aber kein Knoten der \
         Wirkungsbeschreibung nennt sie. Genau so blieb die `timeline` aller 35 Bosse \
         unbeschrieben, waehrend jeder Boss dank `resistances` Knoten hatte — eine Zaehlung \
         ueber BOSSE haette das nie gemeldet, nur eine ueber WIRKUNGEN."
    );
}

// ------------------------------------------------------- Aufzeichnungen (#970)

/// Kompiliert die aufgezeichneten Runden ein, die `rota-sim verify` prueft.
///
/// # Warum nur ein Teil der Dateien
///
/// `tests/parity/recordings/` enthaelt 86 Dateien in 47 verschiedenen Formen.
/// Die wenigsten sind aufgezeichnete Runden: Darunter liegen Pruefdaten fuer
/// Ausruestung, fuer Ereignisstroeme, fuer Buff-Ablaeufe — Dinge, die keinen
/// Kampf nachspielen und keine Belegung haben. Gemessen am 2026-08-18 tragen
/// **65 von 86** die vier Felder, die eine Runde ausmachen; die groesste Datei
/// von allen (`coagulation.json`, 665 KB) ist keine.
///
/// Die Auswahl ist deshalb keine Stichprobe, sondern die vollstaendige Menge
/// derer, die sich abspielen lassen. Wieviele es waren, steht als Konstante im
/// Generat — sonst waere aus "65" spaeter eine Zahl ohne Bezug.
///
/// # Warum nur die letzte Pruefsumme
///
/// Alle Takt-Pruefsummen waeren 230 999 Werte und rund 2,64 MB Rust-Code; die
/// letzte je Aufzeichnung sind 65 Werte. Faktor dreissig. Der Preis steht im
/// Doc von [`rota_sim::replay::EmbeddedRecording`] und in der Ausgabe von
/// `verify`: Eine Abweichung zeigt sich, aber nicht der Takt, in dem sie
/// beginnt.
fn emit_recordings(out: &mut String, spell_ids: &[String]) {
    let dir = Path::new(RECORDINGS_DIR);
    let mut dateien: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{RECORDINGS_DIR} nicht lesbar: {e}"))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    dateien.sort();
    let gesamt = dateien.len();
    assert!(gesamt > 0, "{RECORDINGS_DIR} enthaelt keine Datei");

    let mut brauchbar = Vec::new();
    let mut fremder_zauber = 0usize;

    for pfad in &dateien {
        let json = read_json_at(pfad);
        let Some(eintrag) = lies_aufzeichnung(&json, pfad, spell_ids, &mut fremder_zauber) else {
            continue;
        };
        brauchbar.push(eintrag);
    }

    let _ = writeln!(
        out,
        "/// Aufgezeichnete Runden, zur Bauzeit aus `tests/parity/recordings/`\n\
         /// eingebettet (#970).\n\
         ///\n\
         /// Sie tragen NUR die Pruefsumme des letzten Takts. Eine Abweichung\n\
         /// zeigt damit, DASS eine Aufzeichnung auseinandergeht — nicht, in\n\
         /// welchem Takt. Dafuer gibt es `cargo test --test parity_recording`."
    );
    let _ = writeln!(
        out,
        "pub static RECORDINGS: [crate::replay::EmbeddedRecording; {}] = [",
        brauchbar.len()
    );
    for a in &brauchbar {
        let _ = writeln!(out, "    crate::replay::EmbeddedRecording {{");
        let _ = writeln!(out, "        name: {:?},", a.name);
        let _ = writeln!(out, "        seed: {},", a.seed);
        let _ = writeln!(out, "        ticks: {},", a.ticks);
        let _ = write!(out, "        bindings: &[");
        for (slot, spell) in &a.bindings {
            let _ = write!(out, "({slot}, {spell}),");
        }
        let _ = writeln!(out, "],");
        let _ = write!(out, "        presses: &[");
        for (tick, slot) in &a.presses {
            let _ = write!(out, "({tick}, {slot}),");
        }
        let _ = writeln!(out, "],");
        let _ = writeln!(out, "        final_checksum: {},", a.final_checksum);
        match &a.boss_id {
            Some(id) => let_boss(out, id),
            None => {
                let _ = writeln!(out, "        boss_id: None,");
            }
        }
        let _ = writeln!(out, "    }},");
    }
    let _ = writeln!(out, "];\n");

    // **Die drei Zahlen gehoeren ins Generat, nicht in einen Kommentar.**
    // `verify` nennt sie in seiner Ausgabe; ohne sie waere "65 geprueft" eine
    // Zahl ohne Bezugsgroesse, und niemand saehe, dass 21 Dateien gar keine
    // Aufzeichnungen sind.
    let _ = writeln!(
        out,
        "/// Wie viele Dateien in `tests/parity/recordings/` liegen — auch die,\n\
         /// die keine Aufzeichnung sind.\n\
         pub const RECORDING_FILES_TOTAL: usize = {gesamt};\n"
    );
    let _ = writeln!(
        out,
        "/// Wie viele davon uebersprungen wurden, weil sie keine abspielbare\n\
         /// Runde beschreiben (fehlende Belegung, keine Tastendruecke, keine\n\
         /// Pruefsummen).\n\
         pub const RECORDINGS_SKIPPED: usize = {};\n",
        gesamt - brauchbar.len()
    );
    let _ = writeln!(
        out,
        "/// Wie viele uebersprungen wurden, weil sie einen Zauber nennen, den\n\
         /// dieser Bau nicht kennt. Teilmenge von [`RECORDINGS_SKIPPED`].\n\
         pub const RECORDINGS_SKIPPED_UNKNOWN_SPELL: usize = {fremder_zauber};\n"
    );
}

/// Liest eine Datei als Aufzeichnung — oder `None`, wenn sie keine ist.
///
/// Die Felder sind dieselben, die `tests/parity_recording.rs` liest. Fehlt
/// eines, ist die Datei keine Runde; das ist kein Fehler, sondern der
/// Normalfall fuer einundzwanzig der Dateien.
#[allow(clippy::type_complexity)]
fn lies_aufzeichnung(
    json: &Value,
    pfad: &Path,
    spell_ids: &[String],
    fremder_zauber: &mut usize,
) -> Option<Aufzeichnung> {
    let bindings_json = json.get("bindings")?.as_array()?;
    let presses_json = json.get("presses")?.as_array()?;
    let ticks = u32::try_from(json.get("ticks")?.as_u64()?).ok()?;
    let checksums = json.get("checksums")?.as_array()?;
    let letzte = checksums.last()?.as_u64()?;
    let final_checksum = u32::try_from(letzte).ok()?;

    let mut bindings = Vec::with_capacity(bindings_json.len());
    for eintrag in bindings_json {
        let spell_id = eintrag.get("spellId")?.as_str()?;
        let slot = u8::try_from(eintrag.get("slot")?.as_u64()?).ok()?;
        // **Ein unbekannter Zauber ist kein Baufehler.** Eine Aufzeichnung
        // kann aus einer Zeit stammen, in der es einen Zauber gab, den es
        // heute nicht mehr gibt — dann laesst sie sich nicht abspielen, und
        // der Bau soll deshalb nicht abbrechen. Gezaehlt wird es trotzdem.
        let Some(index) = spell_ids.iter().position(|id| id == spell_id) else {
            *fremder_zauber += 1;
            return None;
        };
        bindings.push((slot, index));
    }

    let mut presses = Vec::with_capacity(presses_json.len());
    for eintrag in presses_json {
        let tick = u32::try_from(eintrag.get("tick")?.as_u64()?).ok()?;
        let slot = u8::try_from(eintrag.get("slot")?.as_u64()?).ok()?;
        presses.push((tick, slot));
    }

    // Der Startwert fehlt in manchen Dateien; das Spiel schreibt ihn nur, wo
    // er etwas aendert. Null ist derselbe Vorgabewert wie in `FightConfig`.
    let seed = json
        .get("seed")
        .and_then(Value::as_u64)
        .and_then(|s| u32::try_from(s).ok())
        .unwrap_or(0);

    let name = pfad
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let boss_id = json
        .get("bossId")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    Some(Aufzeichnung {
        name,
        seed,
        ticks,
        bindings,
        presses,
        final_checksum,
        boss_id,
    })
}

/// Was `lies_aufzeichnung` einer Datei entnimmt.
///
/// Als Struktur statt als Tupel, seit `bossId` dazukam: Ein Tupel mit sieben
/// Feldern, davon zwei vom selben Typ, ist die naechste Stelle, an der jemand
/// zwei Werte vertauscht — und der Fehler waere still.
struct Aufzeichnung {
    name: String,
    seed: u32,
    ticks: u32,
    bindings: Vec<(u8, usize)>,
    presses: Vec<(u32, u8)>,
    final_checksum: u32,
    boss_id: Option<String>,
}

/// Schreibt die Boss-Kennung einer Aufzeichnung ins Generat.
fn let_boss(out: &mut String, id: &str) {
    let _ = writeln!(out, "        boss_id: Some({id:?}),");
}
