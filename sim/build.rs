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
    println!("cargo:rerun-if-changed={COAGULATION_JSON}");
    println!("cargo:rerun-if-changed={FIXATION_JSON}");
    println!("cargo:rerun-if-changed={SUBLIMATION_JSON}");
    println!("cargo:rerun-if-changed={PUTREFACTION_JSON}");
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
    let bosses = read_bosses();

    let mut out = String::with_capacity(8 * 1024);
    out.push_str(
        "// ERZEUGT VON build.rs — NICHT VON HAND AENDERN.\n\
         // Quelle: data/spells/calcination.json und data/bosses/*.json\n\n",
    );

    emit_school(&mut out, &spells);
    emit_resource(&mut out, &spells);
    emit_spells(
        &mut out,
        &spells,
        &common,
        &coagulation,
        &fixation,
        &sublimation,
        &putrefaction,
    );
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

fn emit_spells(
    out: &mut String,
    spells: &Value,
    common: &Value,
    coagulation: &Value,
    fixation: &Value,
    sublimation: &Value,
    putrefaction: &Value,
) {
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
    let gemeinsame = common
        .get("spells")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("common.json: Liste \"spells\" fehlt"));
    // **Neue Zauber kommen HINTEN dazu, nie dazwischen.** Die Reihenfolge
    // bestimmt die Indizes, und die stehen in jeder aufgezeichneten Runde —
    // QUENCH von Platz 3 auf Platz 6 zu schieben deutete jede alte
    // Aufzeichnung still um, ohne dass ein Prueffall es meldet.
    let koagulation = coagulation
        .get("spells")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("coagulation.json: Liste \"spells\" fehlt"));
    let fixierung = fixation
        .get("spells")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("fixation.json: Liste \"spells\" fehlt"));
    let zersetzung = putrefaction
        .get("spells")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("putrefaction.json: Liste \"spells\" fehlt"));
    let sublimierung = sublimation
        .get("spells")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("sublimation.json: Liste \"spells\" fehlt"));
    let list: Vec<&Value> = list
        .iter()
        .chain(gemeinsame.iter())
        .chain(koagulation.iter())
        .chain(fixierung.iter())
        .chain(sublimierung.iter())
        .chain(zersetzung.iter())
        .collect();

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
    for spell in &list {
        emit_spell(out, spell, resource_id);
    }
    let _ = writeln!(out, "];\n");

    // Benannte Indizes, damit Rotationen nicht mit nackten Zahlen hantieren.
    // Auch sie stammen aus der Datei, nicht aus dieser Quelldatei.
    for (index, spell) in list.iter().enumerate() {
        let id = text(spell, "id", "calcination.json/spells[]");
        let konst = id.to_uppercase().replace(['-', ' '], "_");
        let _ = writeln!(out, "/// Index von `{id}` in [`SPELLS`].");
        let _ = writeln!(out, "pub const {konst}: usize = {index};");
    }
    out.push('\n');
}

fn emit_spell(out: &mut String, spell: &Value, resource_id: &str) {
    let id = text(spell, "id", "calcination.json/spells[]");
    let whose = format!("calcination.json/spells[{id}]");

    let _ = writeln!(out, "    Spell {{");
    let _ = writeln!(out, "        id: {id:?},");
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
        "        cooldown_reduction_ms: {},",
        opt_uint(spell, "cooldownReductionMs", &whose, 0)
    );
    let _ = writeln!(
        out,
        "        arms_cooldown_reset: {},",
        spell
            .get("armsCooldownReset")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    );
    emit_dot(out, spell, &whose, resource_id);
    emit_spender(out, spell, &whose, resource_id);
    emit_coagulation(out, spell, &whose);
    emit_putrefaction(out, spell);
    emit_fixation(out, spell, &whose);
    let _ = writeln!(out, "    }},");
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
            (name, value)
        })
        .collect()
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
    let _ = writeln!(out, "    }},");
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

        let _ = writeln!(
            out,
            "            TimelineEntry {{ at_ms: {at}, mechanic: {:?}, \
             name: {:?}, kind: {kind}, damage: {}, cast_time_ms: {cast_time} }},",
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

    let _ = writeln!(out, "        energy: Some(Energy {{");
    let _ = writeln!(out, "            per_step_ms: {per_step},");
    let _ = writeln!(
        out,
        "            amount: {},",
        uint(energy, "amount", &whose)
    );
    let _ = writeln!(out, "            threshold: {threshold},");
    let _ = writeln!(out, "            reset_to: {reset_to},");
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
    let _ = writeln!(out, "        boss_heal: Some(BossHeal {{");
    let _ = writeln!(out, "            amount: {amount},");
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

    let _ = writeln!(out, "/// Die Grundwerte des Spielers aus `data/player/`.");
    let _ = writeln!(
        out,
        "pub static PLAYER: &PlayerBase = &PlayerBase {{ vessel: {vessel}, ward_reduction: {ward:?} }};\n"
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
            let _ = writeln!(out, "        heal: Some(Heal {{ amount: {amount} }}),");
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
                other => panic!(
                    "{whose}: unbekannte Wirkung {other:?}. Bekannt sind \
                     \"slowCast\", \"delayStrike\" und \"holdPhase\" \
                     (GAME-DESIGN 4.1)."
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
