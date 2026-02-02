use clap::Parser;
use mad::commands;
use mad::core::config::Config;
use mad::utils::cli::{Args, EditorMode, TmanAction, ValidatedArgs};
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let validated = args.validate()?;

    match validated {
        ValidatedArgs::Tman(action) => {
            let (_, vault) = load_config()?;
            match action {
                TmanAction::List => commands::tman::list_tags(&vault, false)?,
                TmanAction::ListAll => commands::tman::list_tags(&vault, true)?,
                TmanAction::Rename => commands::tman::rename_tags(&vault)?,
                TmanAction::Find => commands::tman::find_by_tag(&vault)?,
                TmanAction::Visual => commands::tman::visual_selector()?,
                TmanAction::Interactive => commands::tman::run(&vault)?,
            }
        }
        ValidatedArgs::Daily {
            editor,
            skip_timestamp,
        } => {
            let (mut config, vault) = load_config()?;
            if skip_timestamp {
                config.timeprint = Some(false);
            }
            let editor_cmd = resolve_editor(&config, editor);
            commands::daily::run(config, vault, editor_cmd)?;
        }
        ValidatedArgs::Last {
            count,
            editor,
            skip_timestamp,
        } => {
            let (mut config, vault) = load_config()?;
            if skip_timestamp {
                config.timeprint = Some(false);
            }
            let editor_cmd = resolve_editor(&config, editor);
            commands::last::run(vault, config, count, editor_cmd)?;
        }
        ValidatedArgs::LastNote {
            editor,
            skip_timestamp,
        } => {
            let (mut config, vault) = load_config()?;
            if skip_timestamp {
                config.timeprint = Some(false);
            }
            let editor_cmd = resolve_editor(&config, editor);
            commands::recent::open_last_note(vault, config, editor_cmd)?;
        }
        ValidatedArgs::Create {
            title,
            target_dir,
            editor,
            skip_timestamp,
        } => {
            let (mut config, vault) = load_config()?;
            if skip_timestamp {
                config.timeprint = Some(false);
            }
            let editor_cmd = resolve_editor(&config, editor);
            commands::create::run(config, vault, title, target_dir, editor_cmd)?;
        }
        ValidatedArgs::Retag { target, no_backup, no_alias } => {
            let (config, vault) = load_config()?;
            commands::retag::run(&vault, &config, &target, no_backup, no_alias)?;
        }
        ValidatedArgs::Redir { target, no_backup } => {
            let (config, vault) = load_config()?;
            commands::redir::run(&vault, &config, &target, no_backup)?;
        }
        ValidatedArgs::Obsidian {
            title,
            editor,
            skip_timestamp,
        } => {
            let (mut config, vault) = load_config()?;
            if skip_timestamp {
                config.timeprint = Some(false);
            }
            let editor_cmd = resolve_editor(&config, editor);
            commands::obsidian::run(&vault, config, title, editor_cmd)?;
        }
        ValidatedArgs::Migrate => {
            let (config, vault) = load_config()?;
            commands::migrate::run(&vault, &config)?;
        }
    }

    Ok(())
}

fn resolve_editor(config: &Config, mode: EditorMode) -> Option<String> {
    match mode {
        EditorMode::Default => None, // Use config's editor_mode
        EditorMode::UseConfig => {
            // Force use of external editor from config or fallback to vi
            Some(config.editor.clone().unwrap_or_else(|| "vi".to_string()))
        }
        EditorMode::Custom(cmd) => Some(cmd),
    }
}

fn load_config() -> anyhow::Result<(Config, PathBuf)> {
    let config = match Config::load_default() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            eprintln!("Create ~/.config/magic-documents/config.toml with keys: vault, date, time");
            std::process::exit(1);
        }
    };

    let vault = Path::new(&config.vault).to_path_buf();

    if !vault.exists() {
        eprintln!("Vault does not exist: {}", vault.display());
        std::process::exit(1);
    }

    Ok((config, vault))
}
