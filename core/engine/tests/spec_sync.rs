//! The embedded variant files must match the docs they are copied from:
//! - `variants/cv.standard.toml` == the `resolved` block of
//!   `docs/game/variants/cape-verde/standard.md`
//! - `variants/base.oware.toml` == the `base.oware` column of the parameters table in
//!   `docs/game/variants/README.md`, and that table lists exactly the config's rule keys.

use std::path::{Path, PathBuf};

const IDENTITY_KEYS: &[&str] = &[
    "id",
    "version",
    "name",
    "country",
    "default_for_country",
    "extends",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// Convert one simple YAML scalar / flow value (as used in spec front matter and the
/// parameters table) into a TOML value expression.
fn yaml_value_to_toml(v: &str) -> Option<String> {
    let v = v.split(" #").next().unwrap().trim();
    if v == "null" || v == "none" || v.is_empty() {
        return None;
    }
    if v.starts_with('{') {
        let inner = v.trim_start_matches('{').trim_end_matches('}');
        let parts: Vec<String> = inner
            .split(',')
            .map(|kv| {
                let (k, val) = kv.split_once(':').expect("flow mapping entry");
                format!("{} = {}", k.trim(), yaml_value_to_toml(val).expect("value"))
            })
            .collect();
        return Some(format!("{{ {} }}", parts.join(", ")));
    }
    if v.starts_with('[') || v == "true" || v == "false" || v.parse::<i64>().is_ok() {
        return Some(v.to_string());
    }
    Some(format!("\"{v}\""))
}

fn rule_table(src: &str) -> toml::Table {
    let mut t: toml::Table = src.parse().expect("TOML");
    for k in IDENTITY_KEYS {
        t.remove(*k);
    }
    t
}

/// A variant file's rule keys equal its spec's `resolved` block, and its identity matches.
fn assert_toml_equals_spec(spec_path: &str, toml_path: &str, id: &str, version: u16) {
    let spec = read(spec_path);
    let mut lines = spec
        .lines()
        .skip_while(|l| l.trim_end() != "resolved:")
        .skip(1);
    let mut toml_src = String::new();
    for line in lines.by_ref() {
        if !line.starts_with("  ") {
            break;
        }
        let (k, v) = line.trim().split_once(':').expect("key: value");
        if let Some(val) = yaml_value_to_toml(v) {
            toml_src.push_str(&format!("{} = {}\n", k.trim(), val));
        }
    }
    let from_spec: toml::Table = toml_src.parse().expect("converted spec block");
    assert!(
        !from_spec.is_empty(),
        "no resolved block found in {spec_path}"
    );
    let from_file = rule_table(&read(toml_path));
    assert_eq!(
        from_file, from_spec,
        "{toml_path} differs from the `resolved` block of {spec_path}"
    );

    // The identity keys match the spec's front matter too.
    let v = ouril_engine::variant_version(id, version).unwrap();
    assert!(spec.contains(&format!("name: {}", v.name)), "{id}: name");
    assert!(
        spec.contains(&format!("country: {}", v.country)),
        "{id}: country"
    );
    assert_eq!(
        spec.contains("default_for_country: true"),
        v.default_for_country,
        "{id}: default_for_country"
    );
}

#[test]
fn cv_standard_toml_equals_spec_resolved_block() {
    assert_toml_equals_spec(
        "docs/game/variants/cape-verde/standard.md",
        "core/engine/variants/cv.standard.toml",
        "cv.standard",
        1,
    );
}

#[test]
fn cv_continuous_toml_equals_spec_resolved_block() {
    assert_toml_equals_spec(
        "docs/game/variants/cape-verde/continuous.md",
        "core/engine/variants/cv.continuous.toml",
        "cv.continuous",
        1,
    );
}

fn readme_parameters() -> Vec<(String, Option<String>)> {
    let readme = read("docs/game/variants/README.md");
    let section = readme
        .split("## Rule parameters")
        .nth(1)
        .expect("Rule parameters section")
        .split("\n## ")
        .next()
        .unwrap();
    section
        .lines()
        .filter(|l| l.starts_with("| `"))
        .map(|l| {
            let name = l.split('`').nth(1).unwrap().to_string();
            let last_cell = l
                .trim_end()
                .trim_end_matches('|')
                .rsplit_once("| ")
                .unwrap()
                .1;
            let default = last_cell.split('`').nth(1).and_then(yaml_value_to_toml);
            (name, default)
        })
        .collect()
}

#[test]
fn base_oware_toml_equals_readme_defaults() {
    let params = readme_parameters();
    assert!(
        params.len() >= 17,
        "parameters table not parsed: {params:?}"
    );
    let mut toml_src = String::new();
    for (k, v) in &params {
        if let Some(v) = v {
            toml_src.push_str(&format!("{k} = {v}\n"));
        }
    }
    let from_readme: toml::Table = toml_src.parse().expect("converted README defaults");
    let from_file = rule_table(&read("core/engine/variants/base.oware.toml"));
    assert_eq!(
        from_file, from_readme,
        "base.oware.toml differs from the README table"
    );
}

#[test]
fn readme_parameters_are_exactly_the_config_rule_keys() {
    let names: std::collections::BTreeSet<String> =
        readme_parameters().into_iter().map(|(k, _)| k).collect();
    let v = ouril_engine::variant_version("cv.standard", 1).unwrap();
    let json = serde_json::to_value(v).unwrap();
    let keys: std::collections::BTreeSet<String> = json
        .as_object()
        .unwrap()
        .keys()
        .filter(|k| !IDENTITY_KEYS.contains(&k.as_str()))
        .cloned()
        .collect();
    assert_eq!(names, keys);
}

#[test]
fn cv_across_toml_equals_spec_resolved_block() {
    assert_toml_equals_spec(
        "docs/game/variants/cape-verde/across.md",
        "core/engine/variants/cv.across.toml",
        "cv.across",
        1,
    );
}

#[test]
fn cv_across_continuous_toml_equals_spec_resolved_block() {
    assert_toml_equals_spec(
        "docs/game/variants/cape-verde/across-continuous.md",
        "core/engine/variants/cv.across-continuous.toml",
        "cv.across-continuous",
        1,
    );
}
