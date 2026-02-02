use serde_yaml::Mapping;
use std::fs;
use std::path::Path;

/// Write merged frontmatter (YAML) + body to file
pub fn write_note(path: &Path, fm: &Mapping, body: &str) -> anyhow::Result<()> {
    let mut out = String::new();
    if !fm.is_empty() {
        let fm_str = serde_yaml::to_string(fm)?;
        out.push_str("---\n");
        out.push_str(&fm_str);
        out.push_str("---\n");
    }
    out.push_str(body);
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, out)?;
    Ok(())
}

/// Find notebook case-insensitive
pub fn find_notebook_case_insensitive(vault: &Path, name: &str) -> Option<std::path::PathBuf> {
    let lower = name.to_lowercase();
    let Ok(dir) = fs::read_dir(vault) else {
        return None;
    };

    for res in dir.flatten() {
        let Ok(ft) = res.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        if res
            .file_name()
            .to_str()
            .is_some_and(|s| s.to_lowercase() == lower)
        {
            return Some(res.path());
        }
    }
    None
}

/// Read projects list file (one per line), ignoring empty lines
pub fn read_projects(projects_file: &Path) -> anyhow::Result<Vec<String>> {
    if !projects_file.exists() {
        return Ok(vec![]);
    }
    let s = fs::read_to_string(projects_file)?;
    Ok(s.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// Append project to projects file
pub fn append_project(projects_file: &Path, project: &str) -> anyhow::Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(projects_file)?;
    writeln!(f, "{}", project)?;
    Ok(())
}
