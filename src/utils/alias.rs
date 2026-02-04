use crate::core::config::Config;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn load_aliases() -> anyhow::Result<HashMap<String, String>> {
    let path = aliases_path()?;
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let content = fs::read_to_string(path)?;
    let map = serde_json::from_str::<HashMap<String, String>>(&content)?;
    Ok(map)
}

pub fn save_aliases(map: &HashMap<String, String>) -> anyhow::Result<()> {
    let path = aliases_path()?;
    let content = serde_json::to_string_pretty(map)?;
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, content)?;
    Ok(())
}

pub fn is_reserved_word(word: &str) -> bool {
    matches!(
        word,
        "dialy" | "last" | "tag" | "retag" | "redir" | "cache" | "tasks" | "alias"
    )
}

pub fn split_command_line(input: &str) -> anyhow::Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;

    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(current);
                    current = String::new();
                }
            }
            _ => current.push(c),
        }
    }

    if in_single || in_double {
        anyhow::bail!("Comillas sin cerrar en alias");
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

fn aliases_path() -> anyhow::Result<PathBuf> {
    Config::aliases_path()
}
