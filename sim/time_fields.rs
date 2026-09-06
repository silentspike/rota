//! Validate every millisecond field before translating spell data (#1104).
//! Thresholds do not schedule events and need not be on the simulation grid.

use serde_json::Value;

pub fn validate(value: &Value, path: &str, tick_ms: u64) -> Result<(), String> {
    match value {
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                validate(child, &format!("{path}[{index}]"), tick_ms)?;
            }
        }
        Value::Object(values) => {
            for (key, child) in values {
                let location = format!("{path}.{key}");
                if key.ends_with("Ms") {
                    let millis = child
                        .as_u64()
                        .filter(|v| *v <= 9_007_199_254_740_991)
                        .ok_or_else(|| {
                            format!("{location}: expected a nonnegative safe integer")
                        })?;
                    if key != "cooldownReductionBelowMs" && millis % tick_ms != 0 {
                        return Err(format!(
                            "{location}: {millis} is not on the {tick_ms}-ms grid"
                        ));
                    }
                }
                validate(child, &location, tick_ms)?;
            }
        }
        _ => {}
    }
    Ok(())
}
