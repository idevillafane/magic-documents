use std::fs;
use std::path::Path;

/// Configuration for walking through a vault
pub struct VaultWalker<'a> {
    vault: &'a Path,
    exclude_templates: bool,
    exclude_hidden: bool,
    templates_path: Option<&'a Path>,
}

impl<'a> VaultWalker<'a> {
    /// Create a new VaultWalker
    pub fn new(vault: &'a Path) -> Self {
        Self {
            vault,
            exclude_templates: false,
            exclude_hidden: true,
            templates_path: None,
        }
    }

    /// Exclude the templates directory from walking
    pub fn exclude_templates(mut self, templates_path: &'a Path) -> Self {
        self.exclude_templates = true;
        self.templates_path = Some(templates_path);
        self
    }

    /// Include or exclude hidden directories (starting with .)
    pub fn exclude_hidden(mut self, exclude: bool) -> Self {
        self.exclude_hidden = exclude;
        self
    }

    /// Walk through the vault and call the visitor for each markdown file
    /// The visitor receives the file path and content
    pub fn walk<F>(&self, mut visitor: F) -> anyhow::Result<()>
    where
        F: FnMut(&Path, &str) -> anyhow::Result<()>,
    {
        self.walk_dir(self.vault, &mut visitor)
    }

    fn walk_dir<F>(&self, dir: &Path, visitor: &mut F) -> anyhow::Result<()>
    where
        F: FnMut(&Path, &str) -> anyhow::Result<()>,
    {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip hidden directories if configured
                if self.exclude_hidden {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if dir_name.starts_with('.') {
                            continue;
                        }
                    }
                }

                // Skip templates directory if configured
                if self.exclude_templates {
                    if let Some(templates_path) = self.templates_path {
                        if path == templates_path {
                            continue;
                        }
                    }
                }

                self.walk_dir(&path, visitor)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Ok(content) = fs::read_to_string(&path) {
                    visitor(&path, &content)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_vault_walker_basic() {
        let temp = TempDir::new().unwrap();
        let vault = temp.path();

        // Create some test files
        fs::write(vault.join("note1.md"), "# Note 1").unwrap();
        fs::write(vault.join("note2.md"), "# Note 2").unwrap();
        fs::write(vault.join("readme.txt"), "Not a markdown").unwrap();

        let mut count = 0;
        VaultWalker::new(vault)
            .walk(|_path, _content| {
                count += 1;
                Ok(())
            })
            .unwrap();

        assert_eq!(count, 2); // Only .md files
    }

    #[test]
    fn test_vault_walker_excludes_hidden() {
        let temp = TempDir::new().unwrap();
        let vault = temp.path();

        // Create visible and hidden directories
        fs::create_dir(vault.join("visible")).unwrap();
        fs::create_dir(vault.join(".hidden")).unwrap();

        fs::write(vault.join("visible/note.md"), "# Note").unwrap();
        fs::write(vault.join(".hidden/secret.md"), "# Secret").unwrap();

        let mut count = 0;
        VaultWalker::new(vault)
            .walk(|_path, _content| {
                count += 1;
                Ok(())
            })
            .unwrap();

        assert_eq!(count, 1); // Only visible/note.md
    }

    #[test]
    fn test_vault_walker_excludes_templates() {
        let temp = TempDir::new().unwrap();
        let vault = temp.path();

        fs::create_dir(vault.join("Notes")).unwrap();
        fs::create_dir(vault.join("Templates")).unwrap();

        fs::write(vault.join("Notes/note.md"), "# Note").unwrap();
        fs::write(vault.join("Templates/template.md"), "# Template").unwrap();

        let templates_path = vault.join("Templates");

        let mut count = 0;
        VaultWalker::new(vault)
            .exclude_templates(&templates_path)
            .walk(|_path, _content| {
                count += 1;
                Ok(())
            })
            .unwrap();

        assert_eq!(count, 1); // Only Notes/note.md
    }
}
