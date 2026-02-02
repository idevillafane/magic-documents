use super::parser::TagPath;
use super::tree::TagNode;
use crate::core::frontmatter;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct TagCache {
    version: u32,
    timestamp: i64,
    root: TagNode,
}

pub fn load(vault: &Path, config_dir: &Path) -> anyhow::Result<TagNode> {
    let cache_path = config_dir.join("tags_cache.json");

    if cache_path.exists() {
        if let Ok(cache_content) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str::<TagCache>(&cache_content) {
                return Ok(cache.root);
            }
        }
    }

    let root = collect(vault)?;
    update(vault, config_dir, &root)?;
    Ok(root)
}

pub fn update(_vault: &Path, config_dir: &Path, root: &TagNode) -> anyhow::Result<()> {
    let cache_path = config_dir.join("tags_cache.json");

    let cache = TagCache {
        version: 1,
        timestamp: Local::now().timestamp(),
        root: root.clone(),
    };

    let cache_json = serde_json::to_string_pretty(&cache)?;
    fs::write(&cache_path, cache_json)?;

    Ok(())
}

pub fn collect(vault: &Path) -> anyhow::Result<TagNode> {
    let mut root = TagNode::new("root".to_string());

    // Load config to get templates directory
    let config = crate::core::config::Config::load_default()?;
    let templates_path = vault.join(&config.templates_dir);

    crate::utils::vault::VaultWalker::new(vault)
        .exclude_hidden(true) // Exclude hidden directories
        .exclude_templates(&templates_path) // Exclude templates
        .walk(|_path, content| {
            if let Ok((fm, _)) = frontmatter::extract(content) {
                let tag_paths = TagPath::from_frontmatter(&fm);
                for tag_path in tag_paths {
                    root.insert_path(&tag_path.0);
                }
            }
            Ok(())
        })?;

    Ok(root)
}
