use std::collections::HashMap;

const BASE: &str = include_str!("../schemas/base.json");
const CONCEPT: &str = include_str!("../schemas/concept.json");
const PAPER: &str = include_str!("../schemas/paper.json");
const SKILL: &str = include_str!("../schemas/skill.json");
const DOC: &str = include_str!("../schemas/doc.json");
const SECTION: &str = include_str!("../schemas/section.json");

/// Returns a map of schema filename → embedded JSON content.
pub fn default_schemas() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("base.json", BASE),
        ("concept.json", CONCEPT),
        ("paper.json", PAPER),
        ("skill.json", SKILL),
        ("doc.json", DOC),
        ("section.json", SECTION),
    ])
}
