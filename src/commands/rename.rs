use crate::commands::retag;
use crate::core::config::Config;
use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Renombra directorios sincronizando bidireccionalmente entre workspace productivo y vault
/// Ejecuta retag automáticamente después del rename
pub fn run(config: &Config, new_name: &str, no_retag: bool) -> Result<()> {
    let current_dir = env::current_dir()?.canonicalize()?;
    let vault = PathBuf::from(&config.vault).canonicalize()?;
    let tag_root = vault.join(&config.tag_root);

    // Determine if we're in productive workspace or vault
    if current_dir.starts_with(&vault) {
        // We're in vault → rename both productive and vault dirs
        rename_from_vault(config, &current_dir, &vault, &tag_root, new_name, no_retag)
    } else {
        // We're in productive dir → rename both vault and productive dirs
        rename_from_productive(config, &current_dir, &vault, &tag_root, new_name, no_retag)
    }
}

fn rename_from_productive(
    config: &Config,
    current_dir: &Path,
    _vault: &Path,
    tag_root: &Path,
    new_name: &str,
    no_retag: bool,
) -> Result<()> {
    // Find matching dir_mapping
    let dir_mappings = config
        .dir_mappings
        .as_ref()
        .context("No hay dir_mappings configurados")?;

    let mut matched_mapping: Option<(PathBuf, String)> = None;
    let mut longest_match_len = 0;

    for (work_prefix, doc_subpath) in dir_mappings {
        let work_prefix = PathBuf::from(work_prefix).canonicalize()?;
        if current_dir.starts_with(&work_prefix) {
            let match_len = work_prefix.components().count();
            if match_len > longest_match_len {
                longest_match_len = match_len;
                matched_mapping = Some((work_prefix, doc_subpath.clone()));
            }
        }
    }

    let (work_prefix, doc_subpath) = matched_mapping
        .context("Directorio actual no coincide con ningún dir_mapping configurado")?;

    // Calculate relative path from work_prefix to current_dir
    let relative = current_dir
        .strip_prefix(&work_prefix)
        .context("No se pudo calcular path relativo")?;

    // Build current vault dir path
    let vault_dir = tag_root.join(&doc_subpath).join(relative);

    if !vault_dir.exists() {
        anyhow::bail!(
            "Directorio vault no existe: {}\nPrimero crea notas con 'mad -o \"Title\"'",
            vault_dir.display()
        );
    }

    // Calculate new paths
    let new_vault_dir = vault_dir
        .parent()
        .context("No se pudo obtener directorio padre")?
        .join(new_name);

    let new_productive_dir = current_dir
        .parent()
        .context("No se pudo obtener directorio padre")?
        .join(new_name);

    // Check if targets exist
    if new_vault_dir.exists() {
        anyhow::bail!("Directorio vault ya existe: {}", new_vault_dir.display());
    }
    if new_productive_dir.exists() {
        anyhow::bail!(
            "Directorio productivo ya existe: {}",
            new_productive_dir.display()
        );
    }

    let old_name = current_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?");

    // Rename vault directory
    fs::rename(&vault_dir, &new_vault_dir).with_context(|| {
        format!(
            "Error renombrando vault '{}' → '{}'",
            vault_dir.display(),
            new_vault_dir.display()
        )
    })?;

    // Rename productive directory
    fs::rename(current_dir, &new_productive_dir).with_context(|| {
        format!(
            "Error renombrando productivo '{}' → '{}'",
            current_dir.display(),
            new_productive_dir.display()
        )
    })?;

    println!(
        "\x1b[33m{}\x1b[0m → \x1b[32m{}\x1b[0m",
        old_name, new_name
    );

    // Execute retag if not disabled
    if !no_retag {
        let vault = PathBuf::from(&config.vault);
        // Change to new vault dir to run retag
        std::env::set_current_dir(&new_vault_dir)?;
        retag::run(&vault, config, ".", true, false)?; // no_backup=true, no_alias=false (keep old tags as aliases)
    }

    Ok(())
}

fn rename_from_vault(
    config: &Config,
    current_dir: &Path,
    _vault: &Path,
    tag_root: &Path,
    new_name: &str,
    no_retag: bool,
) -> Result<()> {
    // Calculate relative path from tag_root to current_dir
    let relative = current_dir
        .strip_prefix(tag_root)
        .context("Directorio actual no está dentro de tag_root")?;

    let dir_mappings = config
        .dir_mappings
        .as_ref()
        .context("No hay dir_mappings configurados")?;

    // Find matching mapping by checking if relative path starts with doc_subpath
    let mut matched_mapping: Option<(PathBuf, String, PathBuf)> = None;
    let mut longest_match_len = 0;

    for (work_prefix, doc_subpath) in dir_mappings {
        let doc_path = PathBuf::from(doc_subpath);
        if relative.starts_with(&doc_path) {
            let match_len = doc_path.components().count();
            if match_len > longest_match_len {
                longest_match_len = match_len;
                let remainder = relative
                    .strip_prefix(&doc_path)
                    .unwrap_or(Path::new(""))
                    .to_path_buf();
                matched_mapping =
                    Some((PathBuf::from(work_prefix), doc_subpath.clone(), remainder));
            }
        }
    }

    let (work_prefix, _doc_subpath, remainder) = matched_mapping
        .context("Directorio vault actual no coincide con ningún dir_mapping configurado")?;

    // Build current productive dir path
    let productive_dir = work_prefix.join(&remainder);

    if !productive_dir.exists() {
        anyhow::bail!(
            "Directorio productivo no existe: {}",
            productive_dir.display()
        );
    }

    // Calculate new paths
    let new_productive_dir = productive_dir
        .parent()
        .context("No se pudo obtener directorio padre")?
        .join(new_name);

    let new_vault_dir = current_dir
        .parent()
        .context("No se pudo obtener directorio padre")?
        .join(new_name);

    // Check if targets exist
    if new_productive_dir.exists() {
        anyhow::bail!(
            "Directorio productivo ya existe: {}",
            new_productive_dir.display()
        );
    }
    if new_vault_dir.exists() {
        anyhow::bail!("Directorio vault ya existe: {}", new_vault_dir.display());
    }

    let old_name = current_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("?");

    // Rename productive directory first
    fs::rename(&productive_dir, &new_productive_dir).with_context(|| {
        format!(
            "Error renombrando productivo '{}' → '{}'",
            productive_dir.display(),
            new_productive_dir.display()
        )
    })?;

    // Rename vault directory
    fs::rename(current_dir, &new_vault_dir).with_context(|| {
        format!(
            "Error renombrando vault '{}' → '{}'",
            current_dir.display(),
            new_vault_dir.display()
        )
    })?;

    println!(
        "\x1b[33m{}\x1b[0m → \x1b[32m{}\x1b[0m",
        old_name, new_name
    );

    // Execute retag if not disabled
    if !no_retag {
        let vault = PathBuf::from(&config.vault);
        // Change to new vault dir to run retag
        std::env::set_current_dir(&new_vault_dir)?;
        retag::run(&vault, config, ".", true, false)?; // no_backup=true, no_alias=false (keep old tags as aliases)
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_rename_from_productive() {
        let tmp = TempDir::new().unwrap();
        let vault = tmp.path().join("vault");
        let tag_root = vault.join("Notas");
        let work_dir = tmp.path().join("work");
        let project_dir = work_dir.join("proyecto");

        fs::create_dir_all(&tag_root).unwrap();
        fs::create_dir_all(&project_dir).unwrap();

        // Create vault dir
        let vault_project = tag_root.join("dev").join("proyecto");
        fs::create_dir_all(&vault_project).unwrap();

        // Create config
        let mut dir_mappings = HashMap::new();
        dir_mappings.insert(
            work_dir.to_str().unwrap().to_string(),
            "dev".to_string(),
        );

        let config = Config {
            vault: vault.to_str().unwrap().to_string(),
            date: "%Y-%m-%d".to_string(),
            time: "%H:%M".to_string(),
            tag_root: "Notas".to_string(),
            notes_dir: "Notas".to_string(),
            diary_dir: "Diario".to_string(),
            templates_dir: "Templates".to_string(),
            dir_mappings: Some(dir_mappings),
            default_nametype: None,
            editor: None,
            editor_mode: None,
            timeprint: None,
        };

        // Change to project dir and rename both dirs
        env::set_current_dir(&project_dir).unwrap();
        let project_canonical = project_dir.canonicalize().unwrap();
        let vault_canonical = vault.canonicalize().unwrap();
        let tag_root_canonical = tag_root.canonicalize().unwrap();
        rename_from_productive(
            &config,
            &project_canonical,
            &vault_canonical,
            &tag_root_canonical,
            "new-name",
            true, // no_retag for test
        )
        .unwrap();

        // Check old names don't exist
        assert!(!vault_project.exists());
        assert!(!project_dir.exists());

        // Check new names exist
        let new_vault_project = tag_root.join("dev").join("new-name");
        let new_project_dir = work_dir.join("new-name");
        assert!(new_vault_project.exists());
        assert!(new_project_dir.exists());
    }

    #[test]
    fn test_rename_from_vault() {
        let tmp = TempDir::new().unwrap();
        let vault = tmp.path().join("vault");
        let tag_root = vault.join("Notas");
        let work_dir = tmp.path().join("work");
        let project_dir = work_dir.join("proyecto");

        fs::create_dir_all(&tag_root).unwrap();
        fs::create_dir_all(&project_dir).unwrap();

        // Create vault dir
        let vault_project = tag_root.join("dev").join("proyecto");
        fs::create_dir_all(&vault_project).unwrap();

        // Create config
        let mut dir_mappings = HashMap::new();
        dir_mappings.insert(
            work_dir.to_str().unwrap().to_string(),
            "dev".to_string(),
        );

        let config = Config {
            vault: vault.to_str().unwrap().to_string(),
            date: "%Y-%m-%d".to_string(),
            time: "%H:%M".to_string(),
            tag_root: "Notas".to_string(),
            notes_dir: "Notas".to_string(),
            diary_dir: "Diario".to_string(),
            templates_dir: "Templates".to_string(),
            dir_mappings: Some(dir_mappings),
            default_nametype: None,
            editor: None,
            editor_mode: None,
            timeprint: None,
        };

        // Change to vault dir and rename both dirs
        env::set_current_dir(&vault_project).unwrap();
        let vault_project_canonical = vault_project.canonicalize().unwrap();
        let vault_canonical = vault.canonicalize().unwrap();
        let tag_root_canonical = tag_root.canonicalize().unwrap();
        rename_from_vault(
            &config,
            &vault_project_canonical,
            &vault_canonical,
            &tag_root_canonical,
            "new-name",
            true, // no_retag for test
        )
        .unwrap();

        // Check old names don't exist
        assert!(!project_dir.exists());
        assert!(!vault_project.exists());

        // Check new names exist
        let new_project_dir = work_dir.join("new-name");
        let new_vault_project = tag_root.join("dev").join("new-name");
        assert!(new_project_dir.exists());
        assert!(new_vault_project.exists());
    }
}
