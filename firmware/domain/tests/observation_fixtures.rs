//! Fixture contract tests (PROTO-002/003): the observation bytes emitted
//! by the domain must conform to the frozen repository schema and match
//! the frozen valid fixture field-for-field.
//!
//! These run only on the host (`cfg(test)`) and read the real repo files,
//! so firmware output and the companion app fixtures cannot drift apart.

#![cfg(test)]

use spool_sentry_domain::calibration::CalibrationState;
use spool_sentry_domain::faults::{Fault, FaultList};
use spool_sentry_domain::observation::{
    ChannelState, EnvChannel, MassChannel, ObservationSource, serialize,
};

fn schema() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/schemas/observation-v0.1.schema.json"
    ))
    .expect("repository schema parses")
}

fn valid_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../docs/fixtures/observation-valid-v0.1.json"
    ))
    .expect("repository fixture parses")
}

/// The fixture-shaped observation the domain should be able to emit.
fn fixture_source<'a>() -> ObservationSource<'a> {
    ObservationSource {
        device_id: "random-resettable-id",
        sequence: 42,
        now_ms: 1_000,
        wall_epoch_ms: None,
        env: EnvChannel {
            state: ChannelState::Fresh,
            sample_at_ms: Some(300),
            temperature_c: Some(23.4),
            relative_humidity_pct: Some(18.2),
        },
        mass: MassChannel {
            state: ChannelState::Fresh,
            sample_at_ms: Some(750),
            gross_g: Some(812.0),
            net: Some(574.0),
            uncertainty_g: None,
            stable: true,
        },
        calibration: CalibrationState::Valid,
        faults: FaultList::from_slice(&[Fault::UncertaintyNotCharacterized]).unwrap(),
    }
}

#[test]
fn emitted_observation_matches_repo_fixture_semantically() {
    let emitted = serialize(&fixture_source()).expect("observation serializes");
    let emitted: serde_json::Value = serde_json::from_str(&emitted).expect("output is valid JSON");
    assert_eq!(
        emitted,
        valid_fixture(),
        "emitted bytes must match fixture values"
    );
}

#[test]
fn emitted_observation_conforms_to_schema() {
    let emitted = serialize(&fixture_source()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&emitted).unwrap();
    jsonschema_lite::validate(&schema(), &value);
}

#[test]
fn fault_channel_variants_all_conform_to_schema() {
    // Every wire state the domain can emit must validate against the
    // schema enums, including the fault-only shapes.
    let cases: Vec<(&str, ChannelState, ChannelState)> = vec![
        ("fresh", ChannelState::Fresh, ChannelState::Fresh),
        ("settling", ChannelState::Fresh, ChannelState::Settling),
        ("stale-env", ChannelState::Stale, ChannelState::Fresh),
        ("stale-mass", ChannelState::Fresh, ChannelState::Stale),
        (
            "disconnected",
            ChannelState::Disconnected,
            ChannelState::Disconnected,
        ),
        ("saturated", ChannelState::Fresh, ChannelState::Saturated),
        (
            "out-of-range",
            ChannelState::OutOfRange,
            ChannelState::OutOfRange,
        ),
        ("invalid", ChannelState::Invalid, ChannelState::Invalid),
    ];
    let schema = schema();
    for (label, env_state, mass_state) in cases {
        let src = ObservationSource {
            device_id: "random-resettable-id",
            sequence: 7,
            now_ms: 5_000,
            wall_epoch_ms: Some(1_788_912_000_000),
            env: EnvChannel {
                state: env_state,
                sample_at_ms: Some(4_500),
                temperature_c: None,
                relative_humidity_pct: None,
            },
            mass: MassChannel {
                state: mass_state,
                sample_at_ms: Some(4_900),
                gross_g: None,
                net: None,
                uncertainty_g: None,
                stable: false,
            },
            calibration: CalibrationState::Uncalibrated,
            faults: FaultList::from_slice(&[Fault::UncertaintyNotCharacterized]).unwrap(),
        };
        let emitted = serialize(&src).unwrap();
        let value: serde_json::Value = serde_json::from_str(&emitted)
            .unwrap_or_else(|e| panic!("{label}: output invalid JSON: {e}"));
        jsonschema_lite::validate(&schema, &value);
    }
}

/// Minimal Draft-2020-12 subset validator mirroring scripts/check_docs.py
/// semantics for the keys the observation schema uses.
mod jsonschema_lite {
    use serde_json::Value;

    pub fn validate(schema: &Value, value: &Value) {
        if let Some(expected) = schema.get("const") {
            assert_eq!(value, expected, "const mismatch at {value}");
        }
        if let Some(allowed) = schema.get("enum").and_then(Value::as_array) {
            assert!(allowed.contains(value), "{value} not in enum");
        }
        match schema.get("type") {
            Some(Value::String(t)) => check_type(t, value, schema),
            Some(Value::Array(types)) => {
                let ok = types
                    .iter()
                    .filter_map(Value::as_str)
                    .any(|t| type_matches(t, value));
                assert!(ok, "{value} has unexpected type");
                for t in types.iter().filter_map(Value::as_str) {
                    if type_matches(t, value) {
                        check_type(t, value, schema);
                    }
                }
            }
            _ => {}
        }
        if let Some(object) = value.as_object() {
            let required = schema.get("required").and_then(Value::as_array);
            if let Some(required) = required {
                for key in required.iter().filter_map(Value::as_str) {
                    assert!(object.contains_key(key), "missing required key {key}");
                }
            }
            let properties = schema.get("properties").and_then(Value::as_object);
            if schema.get("additionalProperties") == Some(&Value::Bool(false))
                && let Some(properties) = properties
            {
                for key in object.keys() {
                    assert!(properties.contains_key(key), "unexpected key {key}");
                }
            }
            if let Some(properties) = properties {
                for (key, item) in object {
                    if let Some(sub) = properties.get(key) {
                        validate(sub, item);
                    }
                }
            }
        }
        if let Some(items) = value.as_array() {
            if let Some(max) = schema.get("maxItems").and_then(Value::as_u64) {
                assert!((items.len() as u64) <= max, "too many items");
            }
            if let Some(item_schema) = schema.get("items") {
                for item in items {
                    validate(item_schema, item);
                }
            }
        }
    }

    fn check_type(t: &str, value: &Value, schema: &Value) {
        assert!(type_matches(t, value), "{value} should be {t}");
        if let Some(min) = schema.get("minimum").and_then(Value::as_f64)
            && let Some(n) = value.as_f64()
        {
            assert!(n >= min, "{n} below minimum {min}");
        }
        if let Some(max) = schema.get("maximum").and_then(Value::as_f64)
            && let Some(n) = value.as_f64()
        {
            assert!(n <= max, "{n} above maximum {max}");
        }
        if let Some(min) = schema.get("minLength").and_then(Value::as_u64)
            && let Some(s) = value.as_str()
        {
            assert!((s.len() as u64) >= min, "string too short: {s}");
        }
        if let Some(max) = schema.get("maxLength").and_then(Value::as_u64)
            && let Some(s) = value.as_str()
        {
            assert!((s.len() as u64) <= max, "string too long: {s}");
        }
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str)
            && let Some(s) = value.as_str()
        {
            assert!(matches_pattern(pattern, s), "{s} does not match {pattern}");
        }
    }

    fn type_matches(t: &str, value: &Value) -> bool {
        match t {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => panic!("unsupported type {t}"),
        }
    }

    /// Pattern check via the `regex` crate, which enforces ECMA-262 class
    /// semantics exactly as JSON Schema does (the schema's patterns use
    /// trailing-literal-dash classes like `[A-Za-z0-9_-]`).
    fn matches_pattern(pattern: &str, text: &str) -> bool {
        let re = regex::Regex::new(pattern).expect("schema pattern compiles");
        re.is_match(text)
    }
}
