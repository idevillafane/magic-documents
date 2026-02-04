use crate::tags::tree::TagNode;
use crate::core::config::Config;
use crate::vault::scan;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
struct PrimaryTagCacheFile {
    version: u32,
    timestamp: i64,
    root: TagNode,
    dirs_by_tag: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct PrimaryTagCache {
    pub root: TagNode,
    pub dirs_by_tag: HashMap<String, Vec<String>>,
}

pub fn load(
    vault: &Path,
    config_dir: &Path,
    templates_path: &Path,
) -> anyhow::Result<PrimaryTagCache> {
    let cache_path = Config::primary_cache_path().unwrap_or_else(|_| config_dir.join("primary_tags_cache.json"));

    if cache_path.exists() {
        if let Ok(cache_content) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str::<PrimaryTagCacheFile>(&cache_content) {
                return Ok(PrimaryTagCache {
                    root: cache.root,
                    dirs_by_tag: cache.dirs_by_tag,
                });
            }
        }
    }

    let cache = collect(vault, templates_path)?;
    update(config_dir, &cache)?;
    Ok(cache)
}

pub fn update(config_dir: &Path, cache: &PrimaryTagCache) -> anyhow::Result<()> {
    let cache_path = Config::primary_cache_path().unwrap_or_else(|_| config_dir.join("primary_tags_cache.json"));

    let cache_file = PrimaryTagCacheFile {
        version: 1,
        timestamp: Local::now().timestamp(),
        root: cache.root.clone(),
        dirs_by_tag: cache.dirs_by_tag.clone(),
    };

    let cache_json = serde_json::to_string_pretty(&cache_file)?;
    fs::write(&cache_path, cache_json)?;

    Ok(())
}

pub fn collect(vault: &Path, templates_path: &Path) -> anyhow::Result<PrimaryTagCache> {
    let mut root = TagNode::new("root".to_string());
    let mut dirs_by_tag: HashMap<String, HashSet<String>> = HashMap::new();

    let items = scan::scan_tags(vault, templates_path)?;

    for item in items {
        if let Some(primary) = item.primary_tag {
            root.insert_path(&primary.0);

            let dir = item
                .path
                .parent()
                .unwrap_or(vault)
                .strip_prefix(vault)
                .unwrap_or(item.path.parent().unwrap_or(vault))
                .to_string_lossy()
                .to_string();

            let key = primary.to_slash_string();
            dirs_by_tag.entry(key).or_default().insert(dir);
        }
    }

    let dirs_by_tag = dirs_by_tag
        .into_iter()
        .map(|(k, v)| {
            let mut dirs: Vec<String> = v.into_iter().collect();
            dirs.sort();
            (k, dirs)
        })
        .collect::<HashMap<String, Vec<String>>>();

    Ok(PrimaryTagCache { root, dirs_by_tag })
}

#[allow(dead_code)]
fn primary_cache_path(config_dir: &Path) -> PathBuf {
    config_dir.join("primary_tags_cache.json")
}
