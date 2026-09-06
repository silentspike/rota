#[path = "../time_fields.rs"]
mod time_fields;

use serde_json::json;

#[test]
fn existing_and_new_duration_fields_must_follow_the_grid() {
    for name in [
        "channelMs",
        "channelTickMs",
        "cooldownReductionMs",
        "extendShieldMs",
        "extendDecayMs",
        "futureEffectMs",
    ] {
        let invalid = json!({"spells": [{name: 12001}]});
        let error = time_fields::validate(&invalid, "school", 10).unwrap_err();
        assert!(error.contains(name));
        assert!(time_fields::validate(&json!({"spells": [{name: 12000}]}), "school", 10).is_ok());
    }
}

#[test]
fn threshold_is_an_integer_but_does_not_schedule_an_event() {
    assert!(
        time_fields::validate(&json!({"cooldownReductionBelowMs": 10005}), "school", 10).is_ok()
    );
    for value in [
        json!(-1),
        json!(1.5),
        json!("10000"),
        json!(null),
        json!(9007199254740992u64),
    ] {
        assert!(
            time_fields::validate(&json!({"cooldownReductionBelowMs": value}), "school", 10)
                .is_err()
        );
    }
}

#[test]
fn invalid_duration_types_cannot_be_treated_as_absent() {
    for value in [json!(-1), json!(1.5), json!("10000"), json!(null)] {
        assert!(time_fields::validate(
            &json!({"spells": [{"extendShieldMs": value}]}),
            "school",
            10
        )
        .is_err());
    }
}
