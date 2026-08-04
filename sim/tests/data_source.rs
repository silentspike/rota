//! AC5 aus Issue #53 — die Daten stammen nachweislich aus `data/`.
//!
//! Die Gefahr, gegen die dieser Test gerichtet ist, ist eine sehr konkrete:
//! Jemand tippt die Zahlen aus `STEP1-SPEC.md` in eine Rust-Datei ab, alles
//! rechnet richtig, und ein halbes Jahr spaeter aendert ein Balance-Schritt
//! `data/spells/calcination.json` — worauf das Spiel neu rechnet und der
//! Simulator weiter die alten Werte nimmt. Der Fehler faellt dann nicht auf,
//! **weil beide fuer sich plausibel bleiben.**
//!
//! Geprueft wird deshalb Feld fuer Feld gegen die Datei selbst. Der Test liest
//! sie bewusst auf einem anderen Weg als `build.rs`: dort `fs::read_to_string`
//! zur Bauzeit, hier `include_str!` — sonst pruefte ein Pfadfehler sich selbst mit.

use serde_json::Value;

use rota_sim::data::{KINDLE, PYRE, SMOULDER};
use rota_sim::{BOSS, RESOURCE, SPELLS, SPELL_COUNT};

/// Der unveraenderte Inhalt der Zauber-Datei.
const SPELLS_JSON: &str = include_str!("../../data/spells/calcination.json");
const COMMON_SPELLS_JSON: &str = include_str!("../../data/spells/common.json");
const COAGULATION_JSON: &str = include_str!("../../data/spells/coagulation.json");
const FIXATION_JSON: &str = include_str!("../../data/spells/fixation.json");
const PUTREFACTION_JSON: &str = include_str!("../../data/spells/putrefaction.json");
/// Der unveraenderte Inhalt der Boss-Datei.
const BOSS_JSON: &str = include_str!("../../data/bosses/first-crucible.json");

fn spells_json() -> Value {
    serde_json::from_str(SPELLS_JSON).expect("data/spells/calcination.json ist kein gueltiges JSON")
}

/// Die schulunabhaengigen Sonderzauber (GAME-DESIGN 7.4).
fn common_spells_json() -> Value {
    serde_json::from_str(COMMON_SPELLS_JSON)
        .expect("data/spells/common.json ist kein gueltiges JSON")
}

/// Alle Zauber in der Reihenfolge, in der `build.rs` sie einkompiliert:
/// erst die der Schule, dann die schulunabhaengigen, dann die Koagulation.
///
/// **Die Reihenfolge ist die Zusicherung**, nicht bloss die Menge. Sie
/// bestimmt die Indizes, und die stehen in jeder aufgezeichneten Runde —
/// neue Zauber kommen deshalb hinten dazu, nie dazwischen.
fn alle_zauber(
    schule: &Value,
    gemeinsam: &Value,
    zersetzung: &Value,
    koagulation: &Value,
    fixierung: &Value,
) -> Vec<Value> {
    let mut liste: Vec<Value> = schule["spells"]
        .as_array()
        .expect("calcination.json: spells fehlt")
        .clone();
    liste.extend(
        gemeinsam["spells"]
            .as_array()
            .expect("common.json: spells fehlt")
            .iter()
            .cloned(),
    );
    liste.extend(
        koagulation["spells"]
            .as_array()
            .expect("coagulation.json: spells fehlt")
            .iter()
            .cloned(),
    );
    liste.extend(
        fixierung["spells"]
            .as_array()
            .expect("fixation.json: spells fehlt")
            .iter()
            .cloned(),
    );
    liste.extend(
        zersetzung["spells"]
            .as_array()
            .expect("putrefaction.json: spells fehlt")
            .iter()
            .cloned(),
    );
    liste
}

fn boss_json() -> Value {
    serde_json::from_str(BOSS_JSON)
        .expect("data/bosses/first-crucible.json ist kein gueltiges JSON")
}

fn spell_node(json: &Value, id: &str) -> Value {
    json["spells"]
        .as_array()
        .expect("spells ist keine Liste")
        .iter()
        .find(|s| s["id"] == id)
        .unwrap_or_else(|| panic!("Zauber {id} steht nicht in der Datei"))
        .clone()
}

#[test]
fn ac5_die_schul_mechanik_stammt_aus_der_datei() {
    let json = spells_json();
    let resource = &json["resource"];

    assert_eq!(RESOURCE.id, resource["id"].as_str().unwrap());
    assert_eq!(RESOURCE.name, resource["name"].as_str().unwrap());
    assert_eq!(u64::from(RESOURCE.cap), resource["cap"].as_u64().unwrap());
    assert_eq!(
        u64::from(RESOURCE.decay_per_step_ms),
        resource["decayPerStepMs"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(RESOURCE.decay_amount),
        resource["decayAmount"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(RESOURCE.highlight_at),
        resource["highlightAt"].as_u64().unwrap()
    );
}

#[test]
fn ac5_jeder_zauber_stimmt_feld_fuer_feld_mit_der_datei_ueberein() {
    let json = spells_json();
    let gemeinsam = common_spells_json();
    let koagulation: Value =
        serde_json::from_str(COAGULATION_JSON).expect("coagulation.json ist kein gueltiges JSON");
    let fixierung: Value =
        serde_json::from_str(FIXATION_JSON).expect("fixation.json ist kein gueltiges JSON");
    let zersetzung: Value =
        serde_json::from_str(PUTREFACTION_JSON).expect("putrefaction.json ist kein gueltiges JSON");
    let liste = alle_zauber(&json, &gemeinsam, &zersetzung, &koagulation, &fixierung);

    assert_eq!(
        SPELL_COUNT,
        liste.len(),
        "einkompiliert sind {SPELL_COUNT} Zauber, in der Datei stehen {}",
        liste.len()
    );

    for (index, node) in liste.iter().enumerate() {
        let spell = &SPELLS[index];
        let id = node["id"].as_str().unwrap();

        assert_eq!(
            spell.id, id,
            "Zauber {index}: die Reihenfolge weicht von der Datei ab"
        );
        assert_eq!(spell.name, node["name"].as_str().unwrap(), "{id}: name");
        assert_eq!(
            u64::from(spell.unlock_level),
            node["unlockLevel"].as_u64().unwrap(),
            "{id}: unlockLevel"
        );
        assert_eq!(
            u64::from(spell.base_damage),
            node["baseDamage"].as_u64().unwrap(),
            "{id}: baseDamage"
        );
        assert_eq!(
            u64::from(spell.cooldown_ms),
            node["cooldownMs"].as_u64().unwrap(),
            "{id}: cooldownMs"
        );
        assert_eq!(
            u64::from(spell.cast_time_ms),
            node["castTimeMs"].as_u64().unwrap(),
            "{id}: castTimeMs"
        );
        assert_eq!(
            spell.triggers_gcd,
            node["triggersGcd"].as_bool().unwrap(),
            "{id}: triggersGcd"
        );

        let gewinn = node
            .get("resourceGain")
            .and_then(|g| g.get("heat"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert_eq!(u64::from(spell.resource_gain), gewinn, "{id}: resourceGain");
    }
}

#[test]
fn ac5_der_brand_effekt_stammt_aus_der_datei() {
    let json = spells_json();
    let node = spell_node(&json, "smoulder");
    let dot_json = &node["dot"];
    let dot = SPELLS[SMOULDER]
        .dot
        .expect("smoulder hat einen Brand-Effekt");

    assert_eq!(
        u64::from(dot.tick_damage),
        dot_json["tickDamage"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(dot.tick_interval_ms),
        dot_json["tickIntervalMs"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(dot.duration_ms),
        dot_json["durationMs"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(dot.resource_gain_per_tick),
        dot_json["resourceGainPerTick"]["heat"].as_u64().unwrap()
    );
    // `refreshMode` hat im einkompilierten Datensatz kein Feld: build.rs bricht
    // ab, sobald dort etwas anderes als "reset" steht (STEP1-SPEC § 5). Dass die
    // Datei weiterhin "reset" sagt, gehoert trotzdem geprueft — sonst faellt eine
    // Aenderung erst beim naechsten vollstaendigen Bau auf.
    assert_eq!(dot_json["refreshMode"].as_str().unwrap(), "reset");
}

#[test]
fn ac5_der_entlade_zauber_stammt_aus_der_datei() {
    let json = spells_json();
    let node = spell_node(&json, "pyre");
    let spender_json = &node["spender"];
    let spender = SPELLS[PYRE].spender.expect("pyre ist ein Entlade-Zauber");

    assert_eq!(
        u64::from(spender.damage_per_point),
        spender_json["damagePerPoint"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(spender.bonus_threshold),
        spender_json["bonusThreshold"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(spender.bonus_multiplier),
        spender_json["bonusMultiplier"].as_u64().unwrap()
    );
    assert_eq!(spender_json["resource"].as_str().unwrap(), RESOURCE.id);
    assert!(spender_json["consumesAll"].as_bool().unwrap());
}

#[test]
fn ac5_der_boss_stammt_aus_der_datei() {
    let json = boss_json();
    assert_eq!(BOSS.id, json["id"].as_str().unwrap());
    assert_eq!(BOSS.name, json["name"].as_str().unwrap());
    assert_eq!(u64::from(BOSS.hp), json["hp"].as_u64().unwrap());
    assert_eq!(
        u64::from(BOSS.overpressure_ms),
        json["overpressureMs"].as_u64().unwrap()
    );
}

#[test]
fn ac5_der_fueller_hat_keinen_brand_effekt_und_kein_entladen() {
    // Die Gegenprobe: Was in der Datei fehlt, darf einkompiliert nicht auftauchen.
    // Ohne sie wuerde ein build.rs, das jedem Zauber vorsichtshalber einen
    // Brand-Effekt anhaengt, von den Tests oben nicht bemerkt.
    let json = spells_json();
    let node = spell_node(&json, "kindle");
    assert!(
        node.get("dot").is_none(),
        "kindle hat in der Datei kein dot"
    );
    assert!(node.get("spender").is_none());
    assert!(SPELLS[KINDLE].dot.is_none());
    assert!(SPELLS[KINDLE].spender.is_none());
}
