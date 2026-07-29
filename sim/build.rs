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
/// Verzeichnis der Boss-Dateien, relativ zum Paketverzeichnis `sim/`.
///
/// **Alle** Dateien darin werden eingelesen, nicht eine benannte. Das ist die
/// Bedingung fuer AC1 aus Issue #96: Ein neuer Boss entsteht als Datendatei,
/// ohne dass eine Zeile Programmcode geaendert wird.
const BOSSES_DIR: &str = "../data/bosses";

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
    // Auf das VERZEICHNIS horchen, nicht auf einzelne Dateien: Sonst merkt
    // Cargo einen neu hinzugefuegten Boss nicht und baut mit der alten Liste
    // weiter — der Fehler saehe aus, als fehle die Datei.
    println!("cargo:rerun-if-changed={BOSSES_DIR}");
    println!("cargo:rerun-if-changed=build.rs");

    let spells = read_json(SPELLS_JSON);
    let bosses = read_bosses();

    let mut out = String::with_capacity(8 * 1024);
    out.push_str(
        "// ERZEUGT VON build.rs — NICHT VON HAND AENDERN.\n\
         // Quelle: data/spells/calcination.json und data/bosses/*.json\n\n",
    );

    emit_school(&mut out, &spells);
    emit_resource(&mut out, &spells);
    emit_spells(&mut out, &spells);
    emit_bosses(&mut out, &bosses);

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

fn emit_spells(out: &mut String, spells: &Value) {
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
    for spell in list {
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
    emit_dot(out, spell, &whose, resource_id);
    emit_spender(out, spell, &whose, resource_id);
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
    emit_phases(out, boss, whose);
    emit_energy(out, boss, whose);
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
        let _ = writeln!(
            out,
            "            TimelineEntry {{ at_ms: {at}, mechanic: {:?} }},",
            text(entry, "mechanic", &whose)
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
