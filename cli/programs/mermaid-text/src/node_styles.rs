use std::collections::BTreeMap;

use crate::color::Style;

#[derive(Default)]
pub(crate) struct NodeStyles {
    definitions: BTreeMap<String, Style>,
    assignments: BTreeMap<String, Vec<String>>,
    direct: BTreeMap<String, Style>,
}

impl NodeStyles {
    pub fn define(&mut self, classes: &str, style: Style) {
        for name in classes.split(',').map(str::trim) {
            let current = self.definitions.entry(name.to_owned()).or_default();
            *current = current.overlay(style);
        }
    }

    pub fn assign(&mut self, ids: &str, classes: &str) {
        for id in ids.split(',').map(str::trim) {
            self.assignments
                .entry(id.to_owned())
                .or_default()
                .extend(classes.split(',').map(|name| name.trim().to_owned()));
        }
    }

    pub fn set(&mut self, ids: &str, style: Style) {
        for id in ids.split(',').map(str::trim) {
            let current = self.direct.entry(id.to_owned()).or_default();
            *current = current.overlay(style);
        }
    }

    pub fn resolve(&self, id: &str, theme: Style) -> Style {
        let mut style = theme.overlay(self.definitions.get("default").copied().unwrap_or_default());
        if let Some(classes) = self.assignments.get(id) {
            for class in classes {
                if let Some(&over) = self.definitions.get(class) {
                    style = style.overlay(over);
                }
            }
        }
        style.overlay(self.direct.get(id).copied().unwrap_or_default())
    }
}
