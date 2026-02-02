use serde_yaml::{Mapping, Value};

/// Representa un path de tag jerárquico (ej: ["proyecto", "cliente", "acme"])
/// Cada elemento del array YAML es un tag independiente.
/// La jerarquía se expresa con "/" dentro de cada string: "padre/hijo/nieto"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagPath(pub Vec<String>);

impl TagPath {
    /// Extrae todos los TagPaths del frontmatter
    /// Cada elemento del array YAML es un tag INDEPENDIENTE
    /// La jerarquía se expresa con "/" dentro del string: "padre/hijo/nieto"
    pub fn from_frontmatter(fm: &Mapping) -> Vec<Self> {
        const TAG_KEYS: &[&str] = &["tags", "tag", "Tags", "Tag"];

        for key in TAG_KEYS {
            if let Some(Value::Sequence(tag_list)) = fm.get(&Value::String((*key).to_string())) {
                let mut result = Vec::new();

                // Each array element is an INDEPENDENT tag
                for tag in tag_list {
                    if let Value::String(t) = tag {
                        let trimmed = t.trim();
                        if trimmed.is_empty() {
                            continue;
                        }

                        // Split on '/' for hierarchy within this single tag
                        let parts: Vec<String> = trimmed
                            .split('/')
                            .map(|p| p.trim().to_string())
                            .filter(|p| !p.is_empty())
                            .collect();

                        if !parts.is_empty() {
                            result.push(TagPath(parts));
                        }
                    }
                }

                return result;
            }
        }

        Vec::new()
    }

    /// Convierte el TagPath a formato slash-separated para serialización YAML
    pub fn to_slash_string(&self) -> String {
        self.0.join("/")
    }

    /// Convierte el TagPath a formato de display con separador personalizado
    pub fn to_string_with_separator(&self, separator: &str) -> String {
        self.0.join(separator)
    }

    /// Verifica si este TagPath comienza con otro TagPath
    pub fn starts_with(&self, other: &TagPath) -> bool {
        if other.0.len() > self.0.len() {
            return false;
        }
        self.0.starts_with(&other.0)
    }
}

impl From<Vec<String>> for TagPath {
    fn from(parts: Vec<String>) -> Self {
        TagPath(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_tag() {
        let yaml = serde_yaml::from_str(
            r#"
tags:
  - simple
"#,
        )
        .unwrap();

        let tags = TagPath::from_frontmatter(&yaml);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].0, vec!["simple"]);
    }

    #[test]
    fn test_parse_hierarchical_tag() {
        let yaml = serde_yaml::from_str(
            r#"
tags:
  - proyecto/cliente/acme
"#,
        )
        .unwrap();

        let tags = TagPath::from_frontmatter(&yaml);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].0, vec!["proyecto", "cliente", "acme"]);
    }

    #[test]
    fn test_parse_array_as_separate_tags() {
        // NEW BEHAVIOR: Each array element is an INDEPENDENT tag
        let yaml = serde_yaml::from_str(
            r#"
tags:
  - experta
  - administracion
"#,
        )
        .unwrap();

        let tags = TagPath::from_frontmatter(&yaml);
        assert_eq!(tags.len(), 2); // TWO separate tags now
        assert_eq!(tags[0].0, vec!["experta"]);
        assert_eq!(tags[1].0, vec!["administracion"]);
    }

    #[test]
    fn test_parse_multiple_hierarchical_tags() {
        // Multiple tags, each with their own hierarchy
        let yaml = serde_yaml::from_str(
            r#"
tags:
  - padre/hijo
  - otro/tag/profundo
"#,
        )
        .unwrap();

        let tags = TagPath::from_frontmatter(&yaml);
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].0, vec!["padre", "hijo"]);
        assert_eq!(tags[1].0, vec!["otro", "tag", "profundo"]);
    }

    #[test]
    fn test_to_slash_string() {
        let tag = TagPath(vec![
            "proyecto".to_string(),
            "cliente".to_string(),
            "acme".to_string(),
        ]);
        assert_eq!(tag.to_slash_string(), "proyecto/cliente/acme");
    }

    #[test]
    fn test_starts_with() {
        let parent = TagPath(vec!["proyecto".to_string(), "cliente".to_string()]);
        let child = TagPath(vec![
            "proyecto".to_string(),
            "cliente".to_string(),
            "acme".to_string(),
        ]);
        let unrelated = TagPath(vec!["otro".to_string()]);

        assert!(child.starts_with(&parent));
        assert!(!unrelated.starts_with(&parent));
        assert!(!parent.starts_with(&child));
    }
}
