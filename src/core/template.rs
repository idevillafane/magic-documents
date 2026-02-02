use crate::core::frontmatter;
use serde_yaml::Mapping;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Read template file path if exists, else return empty mapping/body
pub fn read(path: &Path) -> anyhow::Result<(Mapping, String)> {
    if !path.exists() {
        return Ok((Mapping::new(), String::new()));
    }
    let txt = fs::read_to_string(path)?;
    frontmatter::extract(&txt)
}

/// Render template body with variable substitution
pub fn render_body(body: &str, vars: &BTreeMap<String, String>) -> String {
    let mut r = body.to_string();
    for (k, v) in vars {
        let placeholder = format!("{{{{{}}}}}", k);
        r = r.replace(&placeholder, v);
    }
    r
}
