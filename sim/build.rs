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
/// Verzeichnis der Boss-Dateien, relativ zum Paketverzeichnis `sim/`.
///
/// **Alle** Dateien darin werden eingelesen, nicht eine benannte. Das ist die
/// Bedingung fuer AC1 aus Issue #96: Ein neuer Boss entsteht als Datendatei,
/// ohne dass eine Zeile Programmcode geaendert wird.
const BOSSES_DIR: &str = "../data/bosses";
/// Die Grundwerte des Spielers, relativ zum Paketverzeichnis `sim/`.
const PLAYER_JSON: &str = "../data/player/base.json";
const LEVELS_JSON: &str = "../data/player/levels.json";

/// Welcher Boss der Standard ist, wenn keiner gewaehlt wird.
///
/// Er steht hier und nicht in den Daten: Welcher Boss zuerst kommt, ist eine
/// Frage des Spielaufbaus (GAME-DESIGN 12.7, Instanz-Freischaltung) und keine
/// Eigenschaft des Bosses selbst.
const DEFAULT_BOSS_ID: &str = "first-crucible";

/// Fassung des Datenformats, die dieser Simulator versteht (`STEP1-SPEC.md` § 5).
const EXPECTED_SCHEMA: u64 = 1;

fn main() {
    // Ohne diese beiden Zeilen merkt Cargo eine geaenderte Zauber-Datei nicht und
    // baut mit veralteten Werten weiter — der unangenehmste Fehler ueberhaupt,
    // weil er sich wie ein Rechenfehler anfuehlt.
    println!("cargo:rerun-if-changed={SPELLS_JSON}");
    println!("cargo:rerun-if-changed={COMMON_SPELLS_JSON}");
    println!("cargo:rerun-if-changed={TEST_BUFF_JSON}");
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
    println!("cargo:rerun-if-changed=build.rs");

    let spells = read_json(SPELLS_JSON);
    let common = read_json(COMMON_SPELLS_JSON);
    let coagulation = read_json(COAGULATION_JSON);
    let fixation = read_json(FIXATION_JSON);
    let sublimation = read_json(SUBLIMATION_JSON);
    let putrefaction = read_json(PUTREFACTION_JSON);
    let test_cast = read_json(TEST_CAST_JSON);
    let test_buff = read_json(TEST_BUFF_JSON);
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
    emit_spells(
        &mut out,
        &spells,
        &[
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
        ],
    );

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
    emit_bosses(&mut out, &bosses);
    emit_player(&mut out, &read_json(PLAYER_JSON));
    emit_levels(&mut out, &read_json(LEVELS_JSON));

    let dest = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR fehlt")).join("data_generated.rs");
    fs::write(&dest, out).unwrap_or_else(|e| panic!("{} nicht schreibbar: {e}", dest.display()));
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
fn emit_spells(out: &mut String, spells: &Value, weitere: &[(&str, &Value)]) {
    let resource_id = text(
        spells.get("resource").expect("resource geprueft"),
        "id",
        "calcination.json/resource",
    );
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
    let mut list: Vec<(&Value, Option<&str>)> = list.iter().map(|s| (s, eigene_schule)).collect();
    for (datei, quelle) in weitere {
        let eintraege = quelle
            .get("spells")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{datei}: Liste \"spells\" fehlt"));
        // Die Schule steht an der DATEI, nicht am Zauber — `common.json` traegt
        // keine, und seine Zauber gehoeren deshalb keiner an.
        let schule = quelle.get("school").and_then(Value::as_str);
        list.extend(eintraege.iter().map(|s| (s, schule)));
    }

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
    for (spell, schule) in &list {
        emit_spell(out, spell, resource_id, *schule);
    }
    let _ = writeln!(out, "];\n");

    // Benannte Indizes, damit Rotationen nicht mit nackten Zahlen hantieren.
    // Auch sie stammen aus der Datei, nicht aus dieser Quelldatei.
    for (index, (spell, _)) in list.iter().enumerate() {
        let id = text(spell, "id", "calcination.json/spells[]");
        let konst = id.to_uppercase().replace(['-', ' '], "_");
        let _ = writeln!(out, "/// Index von `{id}` in [`SPELLS`].");
        let _ = writeln!(out, "pub const {konst}: usize = {index};");
    }
    out.push('\n');
}

fn emit_spell(out: &mut String, spell: &Value, resource_id: &str, school: Option<&str>) {
    let id = text(spell, "id", "calcination.json/spells[]");
    let whose = format!("calcination.json/spells[{id}]");

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
    let _ = writeln!(
        out,
        "        cooldown_reduction_ms: {},",
        opt_uint(spell, "cooldownReductionMs", &whose, 0)
    );
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
    emit_putrefaction(out, spell);
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

fn emit_bosses(out: &mut String, bosses: &[(String, Value)]) {
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
        emit_boss(out, boss, name);
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

fn emit_boss(out: &mut String, boss: &Value, whose: &str) {
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
    emit_resistances(out, boss, whose);
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
fn emit_resistances(out: &mut String, boss: &Value, whose: &str) {
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
/// **NUR die Wachstumstabelle, NICHT die Erfahrungskurve.** Der Simulator misst
/// Kaempfe, nicht Fortschritt: Was auf Stufe 50 passiert, haengt an den
/// Grundwerten; wie lange jemand fuer Stufe 50 braucht, an der Kurve — und das
/// eine folgt nicht aus dem anderen. Eine gespiegelte Kurve waere eine zweite
/// Stelle, die auseinanderlaufen kann, ohne dass irgendetwas davon abhinge.
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
fn emit_putrefaction(out: &mut String, spell: &Value) {
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
    let schild_ms = spell
        .get("extendShieldMs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stapel_ms = spell
        .get("extendDecayMs")
        .and_then(Value::as_u64)
        .unwrap_or(0);
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

            let _ = writeln!(
                out,
                "        control: Some(Control {{ cost: {cost}, effect: {effekt}, amount: {amount} }}),"
            );
        }
    }
}
