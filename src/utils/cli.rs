use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Herramienta CLI para gestión de notas Markdown
#[derive(Parser, Debug)]
#[command(
    name = "mad",
    about = "Magic Documents",
    after_help = "USO:\n  mad <comando> [args]\n  mad [-t|--title] \"TITULO\" [DIR]\n\nComandos: dialy, last, tag, retag, redir, cache, tasks, alias\nPara ayuda: mad <comando> -h"
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Forzar título (permite palabras reservadas)
    #[arg(short = 't', long = "title", value_name = "TITULO")]
    pub title: Option<String>,

    /// Título de la nota (posicional)
    #[arg(value_name = "TITULO")]
    pub title_pos: Option<String>,

    /// Directorio destino (posicional, implica tag derivado del path)
    #[arg(value_name = "DIR")]
    pub target_dir: Option<String>,

    /// Alias para obsidian (crear/abrir nota desde directorio productivo)
    #[arg(short = 'q', long = "quick", value_name = "TITULO")]
    pub quick: Option<String>,

    /// Usar editor configurado
    #[arg(short = 'e')]
    pub editor_flag: bool,

    /// Especificar editor a usar
    #[arg(long = "editor", value_name = "EDITOR")]
    pub editor_cmd: Option<String>,

    /// No agregar timestamp al abrir nota existente
    #[arg(short = 'i', long = "no-id")]
    pub no_id: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Daily note
    #[command(name = "dialy")]
    Dialy,

    /// Última nota o últimas N
    Last {
        /// Número de notas a listar
        count: Option<usize>,
    },

    /// Gestión de tags
    Tag {
        /// Acción: list (default) | rename | find | log
        action: Option<String>,
    },

    /// Re-tag archivo(s) según la ubicación del directorio
    Retag {
        /// Archivo o directorio
        target: String,
        /// No crear archivos .bak
        #[arg(long = "no-bak")]
        no_bak: bool,
        /// No agregar alias al cambiar tag primario
        #[arg(long = "no-alias")]
        no_alias: bool,
    },

    /// Mover archivo(s) a directorios según su dir-tag
    Redir {
        /// Archivo o directorio
        target: String,
        /// No crear archivos .bak
        #[arg(long = "no-bak")]
        no_bak: bool,
    },

    /// Regenerar cache
    Cache {
        /// Tipo: all (default) | dir-tags
        kind: Option<String>,
    },

    /// Tareas pendientes
    Tasks {
        /// Marca todas las tareas (peligroso)
        #[arg(long = "force-check-everywhere")]
        force_check_everywhere: bool,
    },

    /// Crear alias de comandos
    Alias {
        /// Nombre del alias
        name: String,
        /// Comando completo (usar comillas)
        command: String,
    },
}

#[derive(Debug)]
pub enum TmanAction {
    List,
    Rename,
    Find,
    Log,
}

#[derive(Debug, Clone, Copy)]
pub enum CacheKind {
    All,
    DirTags,
}

impl Args {
    /// Valida y procesa los argumentos
    pub fn validate(self) -> anyhow::Result<ValidatedArgs> {
        self.validate_inner(0)
    }

    fn validate_inner(self, depth: usize) -> anyhow::Result<ValidatedArgs> {
        if depth > 5 {
            anyhow::bail!("Alias recursivo detectado");
        }
        // Handle quick (obsidian)
        if let Some(title) = self.quick {
            let editor = if self.editor_flag && self.editor_cmd.is_some() {
                anyhow::bail!("No se pueden usar -e y --editor al mismo tiempo");
            } else if self.editor_flag {
                EditorMode::UseConfig
            } else if let Some(cmd) = self.editor_cmd {
                EditorMode::Custom(cmd)
            } else {
                EditorMode::Default
            };
            return Ok(ValidatedArgs::Obsidian {
                title,
                editor,
                skip_timestamp: self.no_id,
            });
        }

        // Subcommands
        if let Some(cmd) = self.command {
            // Disallow title/dir when a command is used
            if self.title.is_some() || self.title_pos.is_some() || self.target_dir.is_some() {
                anyhow::bail!("No se puede combinar un comando con título o directorio");
            }

            return match cmd {
                Command::Dialy => Ok(ValidatedArgs::Daily {
                    editor: resolve_editor(self.editor_flag, self.editor_cmd)?,
                    skip_timestamp: self.no_id,
                }),
                Command::Last { count } => Ok(ValidatedArgs::Last {
                    count: count.unwrap_or(1),
                    editor: resolve_editor(self.editor_flag, self.editor_cmd)?,
                    skip_timestamp: self.no_id,
                }),
                Command::Tag { action } => {
                    let action = parse_tag_action(action.as_deref())?;
                    Ok(ValidatedArgs::Tman(action))
                }
                Command::Retag {
                    target,
                    no_bak,
                    no_alias,
                } => Ok(ValidatedArgs::Retag {
                    target,
                    no_backup: no_bak,
                    no_alias,
                }),
                Command::Redir { target, no_bak } => Ok(ValidatedArgs::Redir {
                    target,
                    no_backup: no_bak,
                }),
                Command::Cache { kind } => Ok(ValidatedArgs::Cache {
                    kind: parse_cache_kind(kind.as_deref())?,
                }),
                Command::Tasks { force_check_everywhere } => Ok(ValidatedArgs::Tasks {
                    mark_all: force_check_everywhere,
                }),
                Command::Alias { name, command } => Ok(ValidatedArgs::Alias { name, command }),
            };
        }

        // Create note (no command)
        let editor = resolve_editor(self.editor_flag, self.editor_cmd)?;
        let skip_timestamp = self.no_id;

        let title_flag = self.title.is_some();
        let title = self.title.or(self.title_pos);
        let target_dir = self.target_dir.map(PathBuf::from);

        // Alias expansion (only if no --title and single-word title)
        if !title_flag {
            if let Some(ref word) = title {
                if !word.contains(char::is_whitespace) {
                    let aliases = crate::utils::alias::load_aliases().unwrap_or_default();
                    if let Some(cmdline) = aliases.get(word) {
                        let mut args = crate::utils::alias::split_command_line(cmdline)?;
                        if args.first().map(|s| s.as_str()) == Some("mad") {
                            args.remove(0);
                        }
                        args.insert(0, "mad".to_string());
                        let expanded = Args::parse_from(args);
                        return expanded.validate_inner(depth + 1);
                    }
                }
            }
        }

        if title.is_none() {
            anyhow::bail!("Falta título o comando. Usa: mad <comando> -h");
        }

        // Enforce: single-word titles require --title
        if !title_flag {
            if let Some(ref t) = title {
                if !t.contains(char::is_whitespace) {
                    anyhow::bail!("Para títulos de una sola palabra usa --title");
                }
            }
        }

        Ok(ValidatedArgs::Create {
            title,
            target_dir,
            editor,
            skip_timestamp,
        })
    }
}

fn resolve_editor(editor_flag: bool, editor_cmd: Option<String>) -> anyhow::Result<EditorMode> {
    let editor = if editor_flag && editor_cmd.is_some() {
        anyhow::bail!("No se pueden usar -e y --editor al mismo tiempo");
    } else if editor_flag {
        EditorMode::UseConfig
    } else if let Some(cmd) = editor_cmd {
        EditorMode::Custom(cmd)
    } else {
        EditorMode::Default
    };
    Ok(editor)
}

fn parse_tag_action(raw: Option<&str>) -> anyhow::Result<TmanAction> {
    match raw.unwrap_or("list") {
        "list" | "ls" => Ok(TmanAction::List),
        "rename" | "rn" => Ok(TmanAction::Rename),
        "find" | "search" => Ok(TmanAction::Find),
        "log" | "visual" | "telescope" => Ok(TmanAction::Log),
        other => anyhow::bail!(
            "Acción de tag desconocida: '{}'. Usa: list, rename, find, log",
            other
        ),
    }
}

fn parse_cache_kind(raw: Option<&str>) -> anyhow::Result<CacheKind> {
    match raw.unwrap_or("all") {
        "all" => Ok(CacheKind::All),
        "dir-tags" | "dir" | "dirs" => Ok(CacheKind::DirTags),
        // Backwards-compatible aliases
        "path-tags" | "path" | "paths" | "primary" => Ok(CacheKind::DirTags),
        other => anyhow::bail!("Tipo de cache desconocido: '{}'. Usa: all, dir-tags", other),
    }
}


#[derive(Debug)]
pub enum EditorMode {
    Default,        // Usar modo de la config (integrated o external)
    UseConfig,      // Forzar uso del editor configurado (o vi)
    Custom(String), // Usar editor específico
}

#[derive(Debug)]
pub enum ValidatedArgs {
    Create {
        title: Option<String>,
        target_dir: Option<PathBuf>,
        editor: EditorMode,
        skip_timestamp: bool,
    },
    Daily {
        editor: EditorMode,
        skip_timestamp: bool,
    },
    Last {
        count: usize,
        editor: EditorMode,
        skip_timestamp: bool,
    },
    Tman(TmanAction),
    Retag {
        target: String,
        no_backup: bool,
        no_alias: bool,
    },
    Redir {
        target: String,
        no_backup: bool,
    },
    Obsidian {
        title: String,
        editor: EditorMode,
        skip_timestamp: bool,
    },
    Cache {
        kind: CacheKind,
    },
    Tasks {
        mark_all: bool,
    },
    Alias {
        name: String,
        command: String,
    },
    Rename {
        new_name: String,
        no_retag: bool,
    },
}
