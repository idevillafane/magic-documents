use crate::core::frontmatter;
use crate::tags::parser::{extract_primary_tag, TagPath};
use crate::utils::vault::VaultWalker;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ScanItem {
    pub path: PathBuf,
    pub primary_tag: Option<TagPath>,
    pub secondary_tags: Vec<TagPath>,
}

/// Scan the vault and return structured tag info per file.
/// - Primary tag: `{ #tag/path }` first non-empty line after frontmatter.
/// - Secondary tags: frontmatter tags + body #tags, including primary.
/// - Ignores #tags inside fenced code blocks.
pub fn scan_tags(vault: &Path, templates_path: &Path) -> anyhow::Result<Vec<ScanItem>> {
    let mut items = Vec::new();

    VaultWalker::new(vault)
        .exclude_templates(templates_path)
        .walk(|path, content| {
            let (fm, body) = frontmatter::extract(content).unwrap_or_default();

            let primary = extract_primary_tag(&body);
            let mut secondary = TagPath::from_frontmatter(&fm);
            secondary.extend(extract_body_tags(&body));

            if let Some(primary_tag) = primary.as_ref() {
                secondary.push(primary_tag.clone());
            }

            let secondary = dedupe_tags(secondary);

            items.push(ScanItem {
                path: path.to_path_buf(),
                primary_tag: primary,
                secondary_tags: secondary,
            });
            Ok(())
        })?;

    Ok(items)
}

fn extract_body_tags(body: &str) -> Vec<TagPath> {
    let mut tags = Vec::new();
    let mut in_code_block = false;

    for line in body.split('\n') {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        // Extract #tags anywhere in the line (hierarchical allowed)
        for tag in extract_hash_tags_from_line(line) {
            tags.push(tag);
        }
    }

    tags
}

fn extract_hash_tags_from_line(line: &str) -> Vec<TagPath> {
    let mut tags = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'#' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() {
                let c = bytes[end] as char;
                if c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '/' {
                    end += 1;
                } else {
                    break;
                }
            }

            if end > start {
                let tag_str = &line[start..end];
                let parts: Vec<String> = tag_str
                    .split('/')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect();
                if !parts.is_empty() {
                    tags.push(TagPath(parts));
                }
            }

            i = end;
        } else {
            i += 1;
        }
    }

    tags
}

fn dedupe_tags(tags: Vec<TagPath>) -> Vec<TagPath> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for tag in tags {
        let key = tag.to_slash_string();
        if seen.insert(key) {
            out.push(tag);
        }
    }

    out
}
