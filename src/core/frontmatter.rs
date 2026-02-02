use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;

/// Extract front matter (---yaml---) returning (map, body_after)
pub fn extract(text: &str) -> anyhow::Result<(Mapping, String)> {
    if let Some(rest) = text.strip_prefix("---") {
        if let Some(pos) = rest.find("\n---") {
            let fm_raw = &rest[..pos + 1];
            let body = &rest[pos + 4..];
            let fm = serde_yaml::from_str::<Mapping>(fm_raw)?;
            return Ok((fm, body.to_string()));
        }
    }
    Ok((Mapping::new(), text.to_string()))
}

/// Merge two YAML mappings (dst overwritten by src)
pub fn merge(mut base: Mapping, override_map: Mapping) -> Mapping {
    for (k, v) in override_map.into_iter() {
        base.insert(k, v);
    }
    base
}

/// Render mapping keys/values replacing {{var}} placeholders (only scalar strings)
pub fn render(m: Mapping, vars: &BTreeMap<String, String>) -> Mapping {
    let mut out = Mapping::new();
    for (k, v) in m.into_iter() {
        let key = k.clone();
        let value = match v {
            Value::String(s) => Value::String(render_string(&s, vars)),
            other => other,
        };
        out.insert(key, value);
    }
    out
}

/// Simple template interpolation for strings
fn render_string(s: &str, vars: &BTreeMap<String, String>) -> String {
    let mut r = s.to_string();
    for (k, v) in vars {
        let placeholder = format!("{{{{{}}}}}", k);
        r = r.replace(&placeholder, v);
    }
    r
}
