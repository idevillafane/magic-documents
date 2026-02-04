use crate::core::config::Config;
use crate::tags::{cache as tags_cache, primary_cache};
use crate::utils::cli::CacheKind;
use std::path::Path;

pub fn run(vault: &Path, config: &Config, kind: CacheKind) -> anyhow::Result<()> {
    let config_dir = Config::config_dir()?;
    let templates_path = vault.join(&config.templates_dir);

    match kind {
        CacheKind::All => {
            let root = tags_cache::collect(vault)?;
            tags_cache::update(vault, &config_dir, &root)?;
            let cache = primary_cache::collect(vault, &templates_path)?;
            primary_cache::update(&config_dir, &cache)?;
            println!("✅ Cache de tags regenerado (incluye dir-tags).");
        }
        CacheKind::DirTags => {
            let cache = primary_cache::collect(vault, &templates_path)?;
            primary_cache::update(&config_dir, &cache)?;
            println!("✅ Cache de dir-tags regenerado.");
        }
    }

    Ok(())
}
