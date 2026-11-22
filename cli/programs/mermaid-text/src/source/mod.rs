use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::color::Rgb;

mod themes;

#[derive(serde::Deserialize)]
struct Frontmatter {
    title: Option<String>,
    #[serde(default)]
    config: DiagramConfig,
}

#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct DiagramConfig {
    pub git_graph: GitGraphConfig,
    pub packet: PacketConfig,
    theme: Value,
    #[serde(deserialize_with = "deserialize_theme_variables")]
    theme_variables: BTreeMap<String, Value>,
}

impl DiagramConfig {
    pub fn theme_value(&self, key: &str) -> Option<&Value> {
        let mut parts = key.split('.');
        let first = self.theme_variables.get(parts.next()?)?;
        parts.try_fold(first, |value, part| value.get(part))
    }

    pub fn theme_color(&self, key: &str) -> Option<Rgb> {
        Rgb::parse(self.theme_value(key)?.as_str()?)
    }

    fn apply_init(&mut self, directive: &str) {
        let Some((name, payload)) = directive
            .strip_prefix("%%{")
            .and_then(|s| s.strip_suffix("}%%"))
            .and_then(|s| s.split_once(':'))
        else {
            return;
        };
        if !matches!(
            name.trim().to_ascii_lowercase().as_str(),
            "init" | "initialize"
        ) {
            return;
        }
        let payload = payload.trim();
        let parsed = serde_json::from_str::<Value>(payload)
            .or_else(|_| serde_json::from_str(&payload.replace('\'', "\"")))
            .ok();
        let (theme, variables) = match parsed {
            Some(Value::Object(mut fields)) => (
                fields.remove("theme"),
                match fields.remove("themeVariables") {
                    Some(Value::Object(variables)) => variables.into_iter().collect(),
                    _ => BTreeMap::new(),
                },
            ),
            Some(_) => return,
            None => {
                let Ok(yaml_serde::Value::Mapping(mut fields)) = yaml_serde::from_str(payload)
                else {
                    return;
                };
                (
                    fields
                        .remove("theme")
                        .map(|value| Value::from(value.as_str())),
                    fields
                        .remove("themeVariables")
                        .map(theme_variables)
                        .unwrap_or_default(),
                )
            }
        };
        if let Some(theme) = theme {
            self.theme = theme;
        }
        for (key, value) in variables {
            merge_theme_value(self.theme_variables.entry(key).or_default(), value);
        }
    }

    fn apply_theme_defaults(&mut self, diagram: &str) {
        let Some(mut defaults) = self
            .theme
            .as_str()
            .and_then(|name| themes::variables(name, diagram))
        else {
            return;
        };
        for (key, value) in std::mem::take(&mut self.theme_variables) {
            merge_theme_value(defaults.entry(key).or_default(), value);
        }
        self.theme_variables = defaults;
    }
}

fn deserialize_theme_variables<'de, D>(deserializer: D) -> Result<BTreeMap<String, Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    yaml_serde::Value::deserialize(deserializer).map(theme_variables)
}

fn theme_variables(value: yaml_serde::Value) -> BTreeMap<String, Value> {
    let yaml_serde::Value::Mapping(map) = value else {
        return BTreeMap::new();
    };
    map.into_iter()
        .filter_map(|(key, value)| {
            let yaml_serde::Value::String(key) = key else {
                return None;
            };
            Some((key, serde_json::to_value(value).ok()?))
        })
        .collect()
}

fn merge_theme_value(current: &mut Value, update: Value) {
    match (current, update) {
        (Value::Object(existing), Value::Object(fields)) => {
            for (key, value) in fields {
                merge_theme_value(existing.entry(key).or_insert(Value::Null), value);
            }
        }
        (current, update) => *current = update,
    }
}

#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct GitGraphConfig {
    pub main_branch_name: Option<String>,
}

#[derive(Default, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct PacketConfig {
    pub bits_per_row: Option<std::num::NonZeroU16>,
}

pub(crate) struct Source {
    pub header: String,
    pub body: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub config: DiagramConfig,
    source_lines: Vec<usize>,
}

impl Source {
    pub fn parse(source: &str) -> Result<Self, String> {
        if source
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err("unexpected control character in diagram".into());
        }
        let mut input = source
            .trim_start_matches('\u{feff}')
            .lines()
            .enumerate()
            .peekable();
        while input.next_if(|(_, line)| line.trim().is_empty()).is_some() {}
        let mut title = None;
        let mut config = DiagramConfig::default();
        if input.next_if(|(_, line)| line.trim() == "---").is_some() {
            let mut yaml = String::new();
            loop {
                let (_, line) = input.next().ok_or("unclosed Mermaid frontmatter")?;
                if line.trim() == "---" {
                    break;
                }
                yaml.push_str(line);
                yaml.push('\n');
            }
            let metadata: Option<Frontmatter> = yaml_serde::from_str(&yaml)
                .map_err(|err| format!("invalid Mermaid frontmatter: {err}"))?;
            if let Some(metadata) = metadata {
                if metadata
                    .title
                    .as_deref()
                    .is_some_and(|title| title.chars().any(|ch| ch.is_control() && ch != '\n'))
                {
                    return Err("unexpected control character in Mermaid title".into());
                }
                title = metadata.title;
                config = metadata.config;
            }
        }
        let mut lines = Vec::new();
        let mut directive = String::new();
        for (index, line) in input {
            let text = line.trim();
            if !directive.is_empty() || text.starts_with("%%{") {
                if !directive.is_empty() {
                    directive.push('\n');
                }
                directive.push_str(text);
                if text.ends_with("}%%") {
                    config.apply_init(&directive);
                    directive.clear();
                }
            } else if !text.is_empty() && !text.starts_with("%%") {
                if line.contains('\r') {
                    return Err("unexpected carriage return in diagram".into());
                }
                lines.push((index + 1, line));
            }
        }
        if !directive.is_empty() {
            return Err("unclosed Mermaid directive".into());
        }
        let mut lines = lines.into_iter();
        let (first_number, first) = lines.next().ok_or("expected a Mermaid diagram")?;
        let first = first.trim_start();
        let token = first.split([' ', '\t', ';', ':']).next().unwrap_or("");
        let header = match token {
            "classDiagram-v2" => "classDiagram",
            "flowchart-elk" | "flowchart-v2" => "flowchart",
            header => header,
        };
        config.apply_theme_defaults(header);
        let suffix = first[token.len()..]
            .strip_prefix(':')
            .unwrap_or(&first[token.len()..]);
        let mut body = String::with_capacity(source.len());
        body.push_str(header);
        body.push_str(suffix);
        let mut source_lines = vec![first_number];
        let mut description = None;
        let mut prelude = true;
        while let Some((number, line)) = lines.next() {
            let text = line.trim();
            if prelude {
                if let Some(value) = text.strip_prefix("accTitle:") {
                    title.get_or_insert_with(|| value.trim().to_owned());
                    continue;
                }
                if let Some(value) = text.strip_prefix("accDescr:") {
                    description = Some(value.trim().to_owned());
                    continue;
                }
                if let Some(mut part) = text
                    .strip_prefix("accDescr")
                    .and_then(|s| s.trim_start().strip_prefix('{'))
                {
                    let mut parts = Vec::new();
                    loop {
                        if let Some(last) = part.trim_end().strip_suffix('}') {
                            if !last.trim().is_empty() {
                                parts.push(last.trim());
                            }
                            break;
                        }
                        if !part.trim().is_empty() {
                            parts.push(part.trim());
                        }
                        part = lines.next().ok_or("unclosed accDescr block")?.1;
                    }
                    description = Some(parts.join("\n"));
                    continue;
                }
                prelude = matches!(
                    text.split_whitespace().next(),
                    Some("title" | "title:" | "direction" | "showData")
                );
            }
            body.push('\n');
            body.push_str(line.trim_end());
            source_lines.push(number);
        }
        Ok(Self {
            header: header.to_owned(),
            body,
            title,
            description,
            config,
            source_lines,
        })
    }

    pub fn diagnostic(&self, error: String) -> String {
        let Some((location, message)) = error.split_once(": ") else {
            return error;
        };
        let Some((name, index)) = location.rsplit_once("line ") else {
            return error;
        };
        let Some(number) = index
            .parse::<usize>()
            .ok()
            .and_then(|i| i.checked_sub(1))
            .and_then(|i| self.source_lines.get(i))
        else {
            return error;
        };
        format!("{name}line {number}: {message}")
    }
}
