#[path = "../time_fields.rs"]
mod time_fields;

use serde_json::json;

#[test]
fn a_channel_rejects_a_missing_or_zero_tick_before_codegen() {
    for spell in [
        json!({"id": "channel", "channelMs": 4000}),
        json!({"id": "channel", "channelMs": 4000, "channelTickMs": 0}),
    ] {
        let school = json!({"spells": [spell]});
        let error = time_fields::validate(&school, "school", 10).unwrap_err();
        assert_eq!(
            error,
            "school.spells[0].channelTickMs: a channel requires a positive tick interval"
        );
    }
}

#[test]
fn a_valid_channel_and_nonchannel_spells_are_still_accepted() {
    for spell in [
        json!({"id": "channel", "channelMs": 4000, "channelTickMs": 1000}),
        json!({"id": "instant"}),
        json!({"id": "instant", "channelMs": 0, "channelTickMs": 0}),
    ] {
        let school = json!({"spells": [spell]});
        assert!(time_fields::validate(&school, "school", 10).is_ok());
    }
}
