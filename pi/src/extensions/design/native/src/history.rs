use crate::documents::Documents;
use serde_json::Value;
use std::collections::BTreeSet;

/// A saved design-data state while a project is open; React source files are not included.
pub struct Snapshot {
    pub id: u64,
    pub summary: String,
    pub documents: Documents,
}

pub fn describe_change(previous: &Documents, next: &Documents) -> String {
    let mut parts = Vec::new();
    for field in ["elements", "parameters"] {
        let before = previous.design[field].as_object().unwrap();
        let after = next.design[field].as_object().unwrap();
        let keys: BTreeSet<_> = before.keys().chain(after.keys()).collect();
        for key in keys {
            if before.get(key) == after.get(key) {
                continue;
            }
            if field == "parameters" {
                let parameter = after.get(key).or_else(|| before.get(key)).unwrap();
                let label = parameter["label"].as_str().unwrap_or(key);
                let value = after.get(key).map(|value| &value["value"]);
                parts.push(match value {
                    Some(Value::String(value)) => format!("{label} → {value}"),
                    Some(value) => format!("{label} → {value}"),
                    None => format!("Remove {label}"),
                });
            } else {
                parts.push(format!("Element · {key}"));
            }
        }
    }
    if previous.workspace["artboards"] != next.workspace["artboards"] {
        parts.push("Artboards".to_owned());
    }
    if previous.workspace["comments"] != next.workspace["comments"] {
        parts.push("Comments".to_owned());
    }
    match parts.len() {
        0 => "Design".to_owned(),
        1 | 2 => parts.join(", "),
        count => format!("{}, +{}", parts[..2].join(", "), count - 2),
    }
}
