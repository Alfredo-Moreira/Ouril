//! Variant registry. Configs are embedded at build time from `core/engine/variants/*.toml`
//! (copied from each spec's `resolved` front matter). Every released version stays here
//! forever (`docs/architecture/versioning.md`).
//!
//! A variant file may declare `extends = "<id>"`: keys it leaves out are taken from the
//! parent (recursively). Abstract bases such as `base.oware` supply defaults only and are
//! not listed by [`variants`].

use std::sync::OnceLock;

use crate::VariantConfig;

/// Abstract bases (`extends` targets). Not playable, not listed.
const BASES: &[(&str, &str)] = &[("base.oware", include_str!("../variants/base.oware.toml"))];

/// Embedded variant files. Add new `id@version` files here; never edit a released one.
const SOURCES: &[(&str, &str)] = &[
    (
        "cv.standard.toml",
        include_str!("../variants/cv.standard.toml"),
    ),
    (
        "cv.continuous.toml",
        include_str!("../variants/cv.continuous.toml"),
    ),
    ("cv.across.toml", include_str!("../variants/cv.across.toml")),
    (
        "cv.across-continuous.toml",
        include_str!("../variants/cv.across-continuous.toml"),
    ),
];

fn parse_table(name: &str, src: &str) -> toml::Table {
    src.parse::<toml::Table>()
        .unwrap_or_else(|e| panic!("invalid embedded variant {name}: {e}"))
}

/// Resolve `extends` chains: the child's keys win over the parent's.
fn resolve(name: &str, mut table: toml::Table, depth: usize) -> toml::Table {
    assert!(depth < 8, "variant {name}: `extends` chain too deep");
    let Some(parent) = table.remove("extends") else {
        return table;
    };
    let parent_id = parent
        .as_str()
        .unwrap_or_else(|| panic!("variant {name}: `extends` must be a string"));
    let parent_table = if let Some((_, src)) = BASES.iter().find(|(id, _)| *id == parent_id) {
        parse_table(parent_id, src)
    } else if let Some((pname, src)) = SOURCES.iter().find(|(_, src)| {
        parse_table(name, src)
            .get("id")
            .and_then(|v| v.as_str())
            .is_some_and(|id| id == parent_id)
    }) {
        parse_table(pname, src)
    } else {
        panic!("variant {name}: unknown parent {parent_id:?}");
    };
    let mut merged = resolve(parent_id, parent_table, depth + 1);
    // Identity keys are never inherited.
    for key in ["id", "version", "name", "country", "default_for_country"] {
        merged.remove(key);
    }
    for (k, v) in table {
        merged.insert(k, v);
    }
    merged
}

/// Parse and resolve one embedded variant source into a complete config.
pub(crate) fn load(name: &str, src: &str) -> VariantConfig {
    let table = resolve(name, parse_table(name, src), 0);
    toml::Value::Table(table)
        .try_into::<VariantConfig>()
        .unwrap_or_else(|e| panic!("variant {name} does not resolve to a complete config: {e}"))
}

fn registry() -> &'static [VariantConfig] {
    static REGISTRY: OnceLock<Vec<VariantConfig>> = OnceLock::new();
    REGISTRY.get_or_init(|| SOURCES.iter().map(|(name, src)| load(name, src)).collect())
}

/// Every known variant, every version.
pub fn variants() -> &'static [VariantConfig] {
    registry()
}

/// Latest version of every variant for an ISO 3166-1 alpha-2 country code (lowercase).
pub fn variants_for_country(country: &str) -> Vec<&'static VariantConfig> {
    let mut out: Vec<&'static VariantConfig> = Vec::new();
    for v in registry().iter().filter(|v| v.country == country) {
        if out.iter().all(|o| o.id != v.id) {
            out.push(variant(&v.id).expect("variant exists"));
        }
    }
    out
}

/// The country's default variant (latest version), if any.
pub fn default_variant(country: &str) -> Option<&'static VariantConfig> {
    registry()
        .iter()
        .filter(|v| v.country == country && v.default_for_country)
        .max_by_key(|v| v.version)
}

/// Latest version of a variant by ID (new offline games use the latest version).
pub fn variant(id: &str) -> Option<&'static VariantConfig> {
    registry()
        .iter()
        .filter(|v| v.id == id)
        .max_by_key(|v| v.version)
}

/// An exact `id@version` (replays and online games pin the version).
pub fn variant_version(id: &str, version: u16) -> Option<&'static VariantConfig> {
    registry()
        .iter()
        .find(|v| v.id == id && v.version == version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn every_variant_is_valid_and_each_country_has_one_default() {
        let mut defaults: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for v in variants() {
            crate::validate(v).unwrap_or_else(|e| panic!("{}@{}: {e}", v.id, v.version));
            assert!(
                v.id.starts_with(&format!("{}.", v.country)),
                "{} must start with its country",
                v.id
            );
            defaults.entry(&v.country).or_default();
            if v.default_for_country {
                let ids = defaults.get_mut(v.country.as_str()).unwrap();
                if !ids.contains(&v.id.as_str()) {
                    ids.push(&v.id);
                }
            }
        }
        for (country, ids) in defaults {
            assert_eq!(
                ids.len(),
                1,
                "country {country} must have exactly one default: {ids:?}"
            );
        }
        // No duplicate id@version.
        let mut seen = std::collections::BTreeSet::new();
        for v in variants() {
            assert!(
                seen.insert((v.id.clone(), v.version)),
                "duplicate {}@{}",
                v.id,
                v.version
            );
        }
    }

    #[test]
    fn extends_fills_in_missing_keys_from_the_parent() {
        let src = r#"
            id = "xx.test"
            version = 1
            name = "Test"
            country = "xx"
            default_for_country = true
            extends = "base.oware"
            grand_slam = "forbidden"
        "#;
        let v = load("xx.test", src);
        assert_eq!(v.grand_slam, crate::GrandSlam::Forbidden);
        assert_eq!(v.single_seed_rule, crate::SingleSeedRule::Allowed);
        assert_eq!(v.win_threshold, 25);
        assert_eq!(v.match_scoring, None);
    }

    #[test]
    #[should_panic(expected = "does not resolve to a complete config")]
    fn incomplete_variant_without_extends_is_rejected() {
        load(
            "xx.bad",
            r#"id = "xx.bad"
version = 1
name = "Bad"
country = "xx"
default_for_country = false
pits_per_side = 6"#,
        );
    }
}
