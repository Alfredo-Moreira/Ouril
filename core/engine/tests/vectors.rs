//! Runs every JSON test vector under `core/test-vectors/` against `ouril-engine`
//! (format: `docs/architecture/test-vectors.md`).
//!
//! - Every file is validated against `core/test-vectors/schema/test-vector.schema.json`.
//! - A vector that can't be run (unknown variant version, bad setup, schema error) is a
//!   **failure**, never a skip.
//! - Seed conservation is asserted in every state.

use std::path::{Path, PathBuf};

use ouril_engine::{
    apply_move, legal_moves, new_game, variant_version, GameState, GameStateJson, MoveEvent,
    Player, VariantConfig,
};
use serde_json::Value;

fn vectors_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../test-vectors")
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "schema") {
                continue;
            }
            collect_json(&p, out);
        } else if p.extension().is_some_and(|e| e == "json") {
            out.push(p);
        }
    }
}

fn all_vector_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_json(&vectors_root(), &mut files);
    files
}

fn schema_validator() -> jsonschema::Validator {
    let path = vectors_root().join("schema/test-vector.schema.json");
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("read schema"))
        .expect("schema JSON");
    jsonschema::draft202012::new(&schema).expect("valid JSON Schema")
}

fn total_seeds(v: &VariantConfig) -> u32 {
    2 * v.pits_per_side as u32 * v.seeds_per_pit as u32
}

fn check_conservation(v: &VariantConfig, s: &GameState, ctx: &str) -> Result<(), String> {
    let sum: u32 = s.board().iter().map(|&p| p as u32).sum::<u32>()
        + s.stores.iter().map(|&p| p as u32).sum::<u32>();
    if sum != total_seeds(v) {
        return Err(format!(
            "{ctx}: seeds not conserved: {sum} != {}",
            total_seeds(v)
        ));
    }
    Ok(())
}

fn setup_state(v: &VariantConfig, setup: &Value) -> Result<GameState, String> {
    let initial_first = match setup {
        Value::String(s) if s == "initial" => Some(Player::South),
        Value::Object(o) if o.get("initial") == Some(&Value::Bool(true)) => Some(
            o.get("first_player")
                .map(|p| serde_json::from_value(p.clone()).map_err(|e| e.to_string()))
                .transpose()?
                .unwrap_or(Player::South),
        ),
        _ => None,
    };
    if let Some(first) = initial_first {
        return Ok(new_game(v, first));
    }
    let j: GameStateJson =
        serde_json::from_value(setup.clone()).map_err(|e| format!("setup: {e}"))?;
    let s = GameState::try_from(j).map_err(|e| format!("setup: {e}"))?;
    if s.pits_per_side != v.pits_per_side {
        return Err(format!(
            "setup has {} pits per side, variant has {}",
            s.pits_per_side, v.pits_per_side
        ));
    }
    check_conservation(v, &s, "setup")?;
    Ok(s)
}

fn run_vector(v: &VariantConfig, vector: &Value) -> Result<(), String> {
    let mut state = setup_state(v, &vector["setup"])?;
    let steps = vector["steps"].as_array().ok_or("steps must be an array")?;
    for (i, step) in steps.iter().enumerate() {
        let ctx = format!("step {i}");
        if let Some(expected) = step.get("expect_legal_moves") {
            let expected: Vec<u8> =
                serde_json::from_value(expected.clone()).map_err(|e| format!("{ctx}: {e}"))?;
            let got = legal_moves(v, &state);
            if got != expected {
                return Err(format!("{ctx}: legal moves {got:?}, expected {expected:?}"));
            }
        }
        let Some(m) = step.get("move") else {
            continue;
        };
        let m = m
            .as_u64()
            .ok_or(format!("{ctx}: move must be an integer"))? as u8;
        let result = apply_move(v, &state, m);
        if let Some(code) = step.get("expect_error") {
            let code = code
                .as_str()
                .ok_or(format!("{ctx}: expect_error must be a string"))?;
            match result {
                Err(e) if e.code() == code => continue, // state unchanged
                Err(e) => return Err(format!("{ctx}: move {m}: error {e}, expected {code}")),
                Ok(_) => return Err(format!("{ctx}: move {m} succeeded, expected error {code}")),
            }
        }
        let r = result.map_err(|e| format!("{ctx}: move {m} failed: {e}"))?;
        check_conservation(v, &r.state, &ctx)?;
        if let Some(exp) = step.get("expect") {
            let got = serde_json::to_value(&r.state).unwrap();
            for key in ["pits", "stores", "to_move", "status"] {
                if let Some(e) = exp.get(key) {
                    if &got[key] != e {
                        return Err(format!("{ctx}: {key} = {}, expected {e}", got[key]));
                    }
                }
            }
            if let Some(e) = exp.get("events") {
                let expected: Vec<MoveEvent> = serde_json::from_value(e.clone())
                    .map_err(|err| format!("{ctx}: events: {err}"))?;
                if r.events != expected {
                    return Err(format!(
                        "{ctx}: events\n  got:      {}\n  expected: {}",
                        serde_json::to_string(&r.events).unwrap(),
                        serde_json::to_string(&expected).unwrap()
                    ));
                }
            }
        }
        state = r.state;
    }
    Ok(())
}

#[test]
fn all_test_vectors_pass() {
    let root = vectors_root();
    let validator = schema_validator();
    let files = all_vector_files();
    assert!(
        !files.is_empty(),
        "no test vectors found under {}",
        root.display()
    );

    let mut failures = Vec::new();
    let mut runs = 0;
    for path in &files {
        let rel = path
            .strip_prefix(&root)
            .unwrap()
            .with_extension("")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(path).unwrap();
        let vector: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                failures.push(format!("{rel}: invalid JSON: {e}"));
                continue;
            }
        };
        let schema_errors: Vec<String> = validator
            .iter_errors(&vector)
            .map(|e| format!("{} at {}", e, e.instance_path()))
            .collect();
        if !schema_errors.is_empty() {
            failures.push(format!("{rel}: schema: {}", schema_errors.join("; ")));
            continue;
        }
        if vector["id"].as_str() != Some(rel.as_str()) {
            failures.push(format!(
                "{rel}: id {} must equal the file path",
                vector["id"]
            ));
            continue;
        }
        for vref in vector["variants"].as_array().unwrap() {
            let id = vref["id"].as_str().unwrap();
            let version = vref["version"].as_u64().unwrap() as u16;
            // Unknown variant versions are failures, not skips.
            let Some(v) = variant_version(id, version) else {
                failures.push(format!(
                    "{rel}: unknown variant {id}@{version} (not skipped: failing)"
                ));
                continue;
            };
            runs += 1;
            if let Err(e) = run_vector(v, &vector) {
                failures.push(format!("{rel} [{id}@{version}]: {e}"));
            }
        }
    }
    println!("ran {runs} vector runs from {} files", files.len());
    assert!(
        failures.is_empty(),
        "{} of {} vector files failed:\n{}",
        failures.len(),
        files.len(),
        failures.join("\n")
    );
}

/// The schema itself rejects malformed vectors (guards against a schema that accepts anything).
#[test]
fn schema_rejects_malformed_vectors() {
    let validator = schema_validator();
    let good = serde_json::json!({
        "id": "cv.standard/x/y", "description": "d", "source": "s",
        "variants": [{"id": "cv.standard", "version": 1}],
        "setup": "initial",
        "steps": [{"expect_legal_moves": [0, 1, 2, 3, 4, 5]}]
    });
    assert!(validator.is_valid(&good));
    let mut bad = good.clone();
    bad["steps"] =
        serde_json::json!([{"move": 2, "expect": {"pits": [1]}, "expect_error": "must_feed"}]);
    assert!(
        !validator.is_valid(&bad),
        "expect and expect_error together"
    );
    let mut bad = good.clone();
    bad["steps"] = serde_json::json!([{"move": 2, "expect_error": "nope"}]);
    assert!(!validator.is_valid(&bad), "unknown error code");
    let mut bad = good.clone();
    bad["setup"] = serde_json::json!({"pits": [4, 4], "stores": [0, 0], "to_move": "east"});
    assert!(!validator.is_valid(&bad), "bad player");
    let mut bad = good;
    bad.as_object_mut().unwrap().remove("source");
    assert!(!validator.is_valid(&bad), "missing source");
}

/// `setup: {"initial": true, "first_player": ...}` starts from `new_game`.
#[test]
fn initial_setup_runs_from_new_game() {
    let v = variant_version("cv.standard", 1).unwrap();
    let vector = serde_json::json!({
        "setup": {"initial": true, "first_player": "north"},
        "steps": [{"expect_legal_moves": [6, 7, 8, 9, 10, 11]}]
    });
    run_vector(v, &vector).unwrap();
}
