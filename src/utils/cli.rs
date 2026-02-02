use clap::Parser;
use std::path::PathBuf;

/// Herramienta CLI para gestión de notas Markdown
#[derive(Parser, Debug)]
#[command(
    name = "mad",
    about = "Magic Documents - Herramienta para crear y gestionar notas Markdown",
    after_help = "EJEMPLOS:\n  \
    mad \"Mi nueva nota\"        Crea nota con selección manual de tag\n  \
    mad \"Mi nota\" .            Crea nota en dir actual, tag auto-derivado\n  \
    mad \"Mi nota\" path/dir     Crea nota en dir específico, tag derivado\n  \
    mad -e \"Mi nota\"           Crea nota con editor configurado\n  \
    mad -d                      Daily note\n  \
    mad -l                      Últimas 5 notas\n  \
    mad --last 10               Últimas 10 notas\n  \
    mad -tl                     Listar tags\n  \
    mad --tman rename           Renombrar tags\n  \
    mad --retag file.md         Re-tag archivo según directorio\n  \
    mad --retag .               Re-tag recursivo en dir actual\n  \
    mad --redir file.md         Mover archivo según su tag\n  \
    mad --redir .               Mover todos según sus tags\n  \
    mad --migrate               Convertir tags [a,b] a [a/b] en todo el vault"
)]
pub struct Args {
    /// Crear o abrir daily note
    #[arg(short = 'd', long = "daily")]
    pub daily: bool,

    /// Mostrar últimas 5 notas editadas
    #[arg(short = 'l', conflicts_with_all = ["title", "name"])]
    pub last_flag: bool,

    /// Mostrar últimas N notas editadas
    #[arg(long = "last", value_name = "N", conflicts_with_all = ["title", "name"])]
    pub last_num: Option<usize>,

    /// Usar editor configurado
    #[arg(short = 'e')]
    pub editor_flag: bool,

    /// Especificar editor a usar
    #[arg(long = "editor", value_name = "EDITOR")]
    pub editor_cmd: Option<String>,

    /// No agregar timestamp al abrir nota existente
    #[arg(short = 'i', long = "no-id")]
    pub no_id: bool,

    /// Nombre de la nota
    #[arg(
        short = 'n',
        long = "name",
        value_name = "TÍTULO",
        conflicts_with = "title"
    )]
    pub name: Option<String>,

    /// Abrir última nota editada
    #[arg(short = 'L', long = "last-note", conflicts_with_all = ["title", "name", "daily", "last_flag", "last_num"])]
    pub last_note: bool,

    /// Gestión de tags: -tl (list), -ta (list-all), -tr (rename), -tf (find), -tv (visual/telescope), -t (interactive)
    #[arg(short = 't', value_name = "ACCIÓN", num_args = 0..=1, default_missing_value = "interactive")]
    pub tman: Option<String>,

    /// Gestión de tags (forma larga)
    #[arg(long = "tman", value_name = "ACCIÓN", conflicts_with = "tman")]
    pub tman_long: Option<String>,

    /// Re-tag archivo(s) según la ubicación del directorio
    #[arg(long = "retag", value_name = "FILE_OR_DIR", conflicts_with_all = ["title", "name", "daily", "last_flag", "last_num", "tman", "tman_long", "redir"])]
    pub retag: Option<String>,

    /// Mover archivo(s) a directorios según sus tags
    #[arg(long = "redir", value_name = "FILE_OR_DIR", conflicts_with_all = ["title", "name", "daily", "last_flag", "last_num", "tman", "tman_long", "retag", "migrate"])]
    pub redir: Option<String>,

    /// No crear archivos .bak al usar --retag o --redir
    #[arg(long = "no-bak")]
    pub no_bak: bool,

    /// Crear/abrir nota en Obsidian desde directorio productivo
    #[arg(short = 'o', long = "obsidian", value_name = "TÍTULO", conflicts_with_all = ["title", "name", "daily", "last_flag", "last_num", "tman", "tman_long", "retag", "redir", "migrate", "quick"])]
    pub obsidian: Option<String>,

    /// Alias para --obsidian (crear/abrir nota desde directorio productivo)
    #[arg(short = 'q', long = "quick", value_name = "TÍTULO", conflicts_with_all = ["title", "name", "daily", "last_flag", "last_num", "tman", "tman_long", "retag", "redir", "migrate", "obsidian"])]
    pub quick: Option<String>,

    /// Migración única: convertir tags array a formato slash
    #[arg(long = "migrate", conflicts_with_all = ["title", "name", "daily", "last_flag", "last_num", "tman", "tman_long", "retag", "redir"])]
    pub migrate: bool,

    /// Título de la nota (argumento posicional)
    #[arg(value_name = "TÍTULO")]
    pub title: Option<String>,

    /// Directorio destino (argumento posicional, implica tag derivado del path)
    #[arg(value_name = "DIR")]
    pub target_dir: Option<String>,
}

#[derive(Debug)]
pub enum TmanAction {
    List,
    ListAll,
    Rename,
    Find,
    Interactive,
    Visual,
}

impl Args {
    /// Valida y procesa los argumentos
    pub fn validate(self) -> anyhow::Result<ValidatedArgs> {
        // Handle --migrate
        if self.migrate {
            return Ok(ValidatedArgs::Migrate);
        }

        // Handle --obsidian or --quick (aliases)
        if let Some(title) = self.obsidian.or(self.quick) {
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

        // Handle --retag
        if let Some(target) = self.retag {
            return Ok(ValidatedArgs::Retag {
                target,
                no_backup: self.no_bak,
            });
        }

        // Handle --redir
        if let Some(target) = self.redir {
            return Ok(ValidatedArgs::Redir {
                target,
                no_backup: self.no_bak,
            });
        }

        // Procesar tman
        let tman_value = self.tman.or(self.tman_long);

        // Si hay tman, es incompatible con todo excepto -h
        if tman_value.is_some() {
            if self.daily
                || self.last_flag
                || self.last_num.is_some()
                || self.editor_flag
                || self.editor_cmd.is_some()
                || self.name.is_some()
                || self.title.is_some()
            {
                anyhow::bail!("--tman/-t no se puede combinar con otras opciones");
            }

            let action = match tman_value.as_deref() {
                Some("l") | Some("list") | Some("ls") => TmanAction::List,
                Some("a") | Some("list-all") | Some("la") => TmanAction::ListAll,
                Some("r") | Some("rename") | Some("rn") => TmanAction::Rename,
                Some("f") | Some("find") | Some("search") => TmanAction::Find,
                Some("v") | Some("visual") | Some("telescope") => TmanAction::Visual,
                Some("interactive") | Some("") | None => TmanAction::Interactive,
                Some(other) => anyhow::bail!(
                    "Acción de tman desconocida: '{}'. Usa: list, list-all, rename, find, visual",
                    other
                ),
            };

            return Ok(ValidatedArgs::Tman(action));
        }

        // Validar que daily, last, last_note y título sean mutuamente excluyentes
        let has_last = self.last_flag || self.last_num.is_some();
        let has_title = self.name.is_some() || self.title.is_some();

        let mode_count = [self.daily, has_last, has_title, self.last_note]
            .iter()
            .filter(|&&x| x)
            .count();

        if mode_count > 1 {
            anyhow::bail!("No se pueden combinar -d, -l/--last, --last-note, y título de nota");
        }

        // Procesar editor
        let editor = if self.editor_flag && self.editor_cmd.is_some() {
            anyhow::bail!("No se pueden usar -e y --editor al mismo tiempo");
        } else if self.editor_flag {
            EditorMode::UseConfig
        } else if let Some(cmd) = self.editor_cmd {
            EditorMode::Custom(cmd)
        } else {
            EditorMode::Default
        };

        // Determinar el modo de operación
        let skip_timestamp = self.no_id;

        if self.daily {
            Ok(ValidatedArgs::Daily {
                editor,
                skip_timestamp,
            })
        } else if let Some(count) = self.last_num {
            Ok(ValidatedArgs::Last {
                count,
                editor,
                skip_timestamp,
            })
        } else if self.last_flag {
            Ok(ValidatedArgs::Last {
                count: 5,
                editor,
                skip_timestamp,
            })
        } else if self.last_note {
            Ok(ValidatedArgs::LastNote {
                editor,
                skip_timestamp,
            })
        } else {
            // Título: prioridad a -n/--name, luego posicional
            let title = self.name.or(self.title);
            // target_dir: directorio destino para crear nota con tag auto-derivado
            let target_dir = self.target_dir.map(PathBuf::from);
            Ok(ValidatedArgs::Create {
                title,
                target_dir,
                editor,
                skip_timestamp,
            })
        }
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
    LastNote {
        editor: EditorMode,
        skip_timestamp: bool,
    },
    Tman(TmanAction),
    Retag {
        target: String,
        no_backup: bool,
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
    Migrate,
}
