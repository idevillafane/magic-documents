# mad - Magic Documents - Gestor de Notas Markdown en Rust

Una herramienta CLI en Rust para crear y gestionar notas Markdown con frontmatter YAML, tags jerárquicos, templates personalizables y un editor TUI integrado. Ideal para sistemas como Obsidian.

## Características Principales

### 📝 Gestión de Notas
- Creación rápida de notas con templates personalizables
- Sistema de tags jerárquicos con sintaxis `/` (ej: `proyecto/cliente/acme`)
- Organización por notebooks (subdirectorios)
- Soporte completo para frontmatter YAML con interpolación de variables
- Sistema de plantillas en cascada (global → notebook-specific)
- Aliases para notas
- Timestamps automáticos al editar notas existentes

### ✏️ Editor Integrado
- **Editor TUI moderno** con interfaz similar a nano/micro
- Atajos de teclado intuitivos (Ctrl+S, Ctrl+Q, Ctrl+Z/Y)
- Números de línea y barra de estado
- Soporte para editores externos (vim, nvim, nano, etc.)

### 🏷️ Gestión de Tags (`tag`)
- **Selector interactivo** con búsqueda fuzzy en tiempo real
- Listado y navegación de todos los tags del vault
- Renombrado masivo de tags (con soporte para jerarquías)
- Búsqueda de archivos por tag
- Cache automático para rendimiento óptimo
- Exclusión de carpeta de templates

### 📅 Funcionalidades Especiales
- **Dialy notes**: Crea o abre la nota del día con `mad dialy`
- **Últimas notas**: Abre la última con `mad last` o lista N con `mad last N`
- **Acceso rápido**: Abre la última nota con `md ..`
- Formatos de fecha/hora configurables

## Instalación

### Desde el código fuente

```bash
git clone <repo-url>
cd md-rust
cargo build --release
```

El binario estará disponible en `target/release/mad`.

Para instalarlo en tu sistema:

```bash
cargo install --path .
```

## Configuración

Crea el archivo de configuración en `~/.config/magic-documents/config.toml`:

```toml
vault = "/ruta/a/tu/vault"
date = "%Y-%m-%d"
time = "%H:%M"
default_nametype = "datetime"    # "date" o "datetime"
editor_mode = "integrated"       # "integrated" o "external"
editor = "nvim"                  # editor externo (Ctrl+G o -e)
timeprint = true                 # añade timestamp al abrir notas existentes
tag_root = "Notas"              # directorio raíz para tags (default: "Notas")
notes_dir = "Notas"             # carpeta para notas generales (default: "Notas")
diary_dir = "Diario"            # carpeta para daily notes (default: "Diario")
templates_dir = "Templates"      # carpeta para templates (default: "Templates")

# Mapeo de directorios trabajo → documentación (para mad -q)
[dir_mappings]
"/Users/tu/Developer" = "developer"
"/Users/tu/Proyectos" = "proyectos"
```

### Parámetros de configuración

| Parámetro | Tipo | Descripción | Default |
|-----------|------|-------------|---------|
| `vault` | String | Directorio raíz de tu vault de notas | *Requerido* |
| `date` | String | Formato de fecha (estilo strftime) | `"%Y-%m-%d"` |
| `time` | String | Formato de hora (estilo strftime) | `"%H:%M"` |
| `default_nametype` | String | Tipo de nombre por defecto: `"date"` o `"datetime"` | `"datetime"` |
| `editor_mode` | String | Modo de editor: `"integrated"` o `"external"` | `"integrated"` |
| `editor` | String | Editor externo (usado por `Ctrl+G`, `-e`, y si `editor_mode = "external"`) | `"vi"` |
| `timeprint` | Boolean | Añade timestamp al abrir notas existentes | `true` |
| `tag_root` | String | Directorio raíz para derivación de tags (usado en `mad -q`) | `"Notas"` |
| `notes_dir` | String | Carpeta para notas generales | `"Notas"` |
| `diary_dir` | String | Carpeta para daily notes | `"Diario"` |
| `templates_dir` | String | Carpeta para templates | `"Templates"` |
| `dir_mappings` | HashMap | Mapeo de directorios trabajo → documentación (ver `mad -q`) | `{}` |

## Uso

### Sintaxis básica

```bash
mad <comando> [args]
mad [-t|--title] "TITULO" [DIR]
```

Comandos: `dialy`, `last`, `tag`, `retag`, `redir`, `cache`, `tasks`, `alias`

Reglas:
- Los títulos deben ir entre comillas (simples o dobles).
- Si el título es una sola palabra, usa `--title`.
- Las palabras reservadas son los comandos y los alias definidos.

Ejemplos:
```bash
mad dialy
mad last
mad last 10
mad tag
mad tag list
mad tag rename
mad tag find
mad tag log
mad retag file.md
mad redir file.md
mad cache
mad cache dir-tags
mad tasks
mad tasks --force-check-everywhere
mad alias hoy "mad dialy"
mad -q "Título"
```

## Sistema de Templates

### Variables disponibles

Los templates pueden usar las siguientes variables:

- `{{date}}`: Fecha actual según formato `date`
- `{{time}}`: Hora actual según formato `time`
- `{{title}}`: Título de la nota

### Ubicación de templates

El sistema busca templates en:

1. **Template centralizado**: `<vault>/Templates/<nombre_carpeta>.md`
2. **Template local**: `<vault>/<nombre_carpeta>/template.txt`

Por ejemplo, para daily notes busca en:
- `<vault>/Templates/Diario.md` (centralizado)
- `<vault>/Diario/template.txt` (local)

### Ejemplo de template

**`~/vault/Templates/Notas.md`**:
```markdown
---
date: {{date}}
time: {{time}}
tags: []
aliases: []
---

# {{title}}

Creado el {{date}} {{time}}

## Notas

```

**`~/vault/Templates/Diario.md`**:
```markdown
---
date: {{date}}
time: {{time}}
tags: ["diario"]
---

# {{date}}

## 📝 Tareas del día

- [ ] 

## 💭 Reflexiones

```

## Gestión de Tags

### Sistema de Tags: Dir-Tags y Tags normales

El sistema distingue entre **dir-tags** (derivados de la estructura de directorios) y **tags normales**:

#### Dir-Tag

- **Ubicación**: Primera línea del cuerpo (después del frontmatter)
- **Formato**: `{ #tag/jerarquico/aqui }`
- **Derivación**: Se genera automáticamente desde la ruta del directorio relativa a `tag_root`
- **Propósito**: Define la ubicación estructural de la nota en el vault

**Ejemplo de archivo**:
```markdown
---
date: "2026-02-02"
time: "12:45"
aliases: []
tags:
  - tag-normal-1
  - tag-normal-2
---
{ #dev/magic-documents }

# Título de la nota

Contenido...
```

#### Tags normales

- **Ubicación**: `frontmatter.tags` (array YAML) y en el cuerpo como `#tag`
- **Formato**: Strings simples o con slash: `["tag1", "tag2/subtag"]`
- **Propósito**: Tags adicionales para categorización cruzada
- **Nota**: Los dir-tags también cuentan como tags normales (pero no al revés)

#### Comportamiento de Comandos

**`mad "título"` o `mad "título" .`**
- Sugiere dir-tag basado en directorio actual
- Inserta `{ #tag/dir }` como primera línea del cuerpo
- Permite agregar tags normales al frontmatter

**`mad retag file.md`**
- Recalcula dir-tag desde la ruta del archivo
- Actualiza la línea `{ #tag }` en el cuerpo
- Si el dir-tag cambió:
  - Agrega el viejo dir-tag a `aliases` con formato: `2026-02-02 old/tag`
- Opciones:
  - `--no-bak` - No crear archivo de backup
  - `--no-alias` - No agregar viejo tag a aliases
- Los backups se guardan en `vault/.arc/backups/` con timestamp: `filename_YYYYMMDD_HHMMSS.md.bak`

**`mad redir file.md`**
- Lee el dir-tag desde la línea `{ #tag }` del cuerpo
- Mueve el archivo al directorio correspondiente a ese tag
- Opciones:
  - `--no-bak` - No crear archivo de backup
- Los backups se guardan en `vault/.arc/backups/` con timestamp: `filename_YYYYMMDD_HHMMSS.md.bak`

#### Formato de Aliases al Cambiar Dir-Tag

Cuando se hace `retag` y el tag primario cambia:
```yaml
aliases:
  - "2026-02-02 old/tag/path"
  - "2026-01-15 another/old/tag"
```

### Selector interactivo de tags

El sistema de tags te permite:
- **Búsqueda fuzzy en tiempo real**: Filtra tags mientras escribes
- **Vista jerárquica**: Muestra primero los padres, luego los paths completos
  - Ejemplo: primero `experta`, luego `experta → ia-recuperos`
- Seleccionar tags existentes navegando por la jerarquía
- Agregar nuevos tags personalizados con `/` para crear jerarquías
- Cache automático para rendimiento

Al crear una nota, los tags se guardan automáticamente en el frontmatter como array:

### tag - Tag Manager

El gestor de tags se usa con `mad tag`:

```bash
mad tag            # list (default)
mad tag list       # lista tags
mad tag rename     # renombrar tags
mad tag find       # buscar por tag
mad tag log        # selector visual (si está implementado)
```

Incluye:
- Filtrado fuzzy en tiempo real
- Navegación jerárquica
- Exclusión de carpetas ocultas y templates
- Cache para rendimiento

### Regenerar Cache de Tags

```bash
mad cache          # all (tags + dir-tags)
mad cache dir-tags # solo dir-tags
```

## Editor Integrado

Por defecto, `md` usa un editor de texto integrado con interfaz TUI moderna.

### Atajos de teclado

| Atajo | Acción |
|-------|--------|
| `Ctrl+S` | Guardar y salir |
| `Ctrl+T` | Agregar tags |
| `Ctrl+G` | Abrir en editor externo (configurado con `editor`) |
| `Ctrl+R` | Renombrar archivo |
| `Ctrl+D` | Eliminar archivo |
| `Ctrl+Z` | Deshacer |
| `Ctrl+Y` | Rehacer |
| `ESC` | Salir sin guardar |
| `Flechas` | Navegar por el texto |
| `Inicio/Fin` | Ir al inicio/fin de línea |
| `PgUp/PgDn` | Desplazar página |

### Características
- Números de línea visibles
- Barra de estado con posición del cursor
- Resaltado de sintaxis básico
- Scroll suave

### Usar editor externo

Si prefieres tu editor favorito:

```toml
editor_mode = "external"
editor = "nvim"  # o "vim", "nano", "code", etc.
```

## Comportamiento con Notas Existentes

Si la nota ya existe:
1. Si `timeprint = true`, añade un timestamp al final del archivo
2. Abre la nota en el editor configurado
3. No sobrescribe el contenido existente

Formato del timestamp:
```
---
[2025-12-18 14:30]
```

## Ejemplos de Uso

### Daily note
```bash
mad dialy
# Crea o abre: vault/Diario/2025-12-18.md
```

### Nota simple
```bash
mad "Ideas para el proyecto"
# Crea: vault/Notas/ideas-para-el-proyecto.md
```

### Últimas notas editadas
```bash
# Abrir la última nota
mad last

# Ver últimas 15 notas
mad last 15
```

### Gestión de tags
```bash
# Listar tags
mad tag

# Listar tags (explícito)
mad tag list

# Renombrar tags masivamente
mad tag rename

# Buscar archivos por tag
mad tag find
```

### Quick (mad -q)

Atajo para crear/abrir notas desde directorios de trabajo mapeados.

```toml
[dir_mappings]
"/Users/tu/Developer" = "developer"
```

```bash
mad -q "API Documentation"
```

## Arquitectura del Proyecto

```
src/
├── commands/       # CLI (create, daily, last, tag/tman, cache, todo)
├── core/           # Lógica de negocio (config, note, template, frontmatter)
├── tags/           # Sistema de tags (cache, primary_cache, selector, tree)
├── vault/          # Scan unificado
├── ui/             # Interfaz de usuario (editor, prompts)
├── utils/          # Utilidades (cli, file, alias)
├── lib.rs          # Módulo raíz
└── main.rs         # Entry point
```

## Dependencias

| Librería | Propósito |
|----------|-----------|
| `clap` | Parsing de argumentos CLI |
| `serde`, `serde_yaml`, `serde_json` | Serialización (frontmatter, cache) |
| `toml` | Lectura de configuración |
| `chrono` | Manejo de fechas y horas |
| `dialoguer` | Menús interactivos y fuzzy select |
| `ratatui` | Interfaz TUI para el editor |
| `tui-textarea` | Widget de texto editable |
| `crossterm` | Control de terminal |
| `directories`, `dirs` | Detección de directorios del usuario |
| `slug` | Slugificación de títulos |
| `anyhow` | Manejo de errores |

## Desarrollo

```bash
# Compilar en modo debug
cargo build

# Ejecutar en modo desarrollo
cargo run -- "Prueba"
cargo run -- -d
cargo run -- -t list

# Tests
cargo test

# Linting
cargo clippy

# Formateo
cargo fmt

# Release optimizado
cargo build --release
```

## Mejoras Recientes

### ✅ Sistema de Tags Jerárquicos Completo (v0.3.0)
Se ha implementado un sistema robusto de tags jerárquicos con soporte para múltiples formatos:

#### 🏷️ Parser de Tags Unificado
- **Múltiples formatos equivalentes**: Array, slash, mixto
- **Interpretación jerárquica consistente**: Todo el array representa un tag
- **Compatibilidad con tags inline**: Soporte para `#padre/hijo/nieto`
- **Tests completos**: Verificación de todos los formatos

#### 🔍 Búsqueda Fuzzy Mejorada
- **Filtrado en tiempo real** en `tag find`
- **Vista jerárquica** en selector: padres primero, luego paths completos
- **Exclusión inteligente**: Archived, templates, carpetas ocultas
- **Cache optimizado**: Regeneración automática cuando es necesario

#### 📝 Escritura Consistente
- Tags se guardan como arrays en YAML
- Cada elemento del array es un tag independiente
- Formato limpio y fácil de leer
- Compatible con Obsidian y otros editores

### ✅ Refactorización de Arquitectura (v0.2.0)
Se han implementado mejoras significativas para reducir código duplicado y mejorar la mantenibilidad:

#### 🎯 Eliminación de Código Duplicado
- **tags/parser.rs**: Lógica unificada de extracción de tags (~400 líneas eliminadas)
  - Nuevo módulo `TagPath` para parsing consistente de tags jerárquicos
  - Elimina duplicación en `cache.rs`, `tman.rs` y `selector.rs`
  
- **ui/input.rs**: Función unificada `input_with_esc` (~80 líneas eliminadas)
  - Implementación única para captura de input con soporte ESC
  - Reutilizada en `prompts.rs`, `selector.rs` y `editor.rs`

- **utils/vault.rs**: Walker unificado para recorrer el vault (~100 líneas eliminadas)
  - Nuevo `VaultWalker` configurable con filtros
  - Soporta exclusión de carpetas ocultas y templates
  - Usado en `cache.rs`, `tman.rs` y `last.rs`

#### 🏗️ Centralización de Configuración
- **core/config.rs**: Métodos centralizados para paths
  - `Config::config_dir()` - Directorio de configuración
  - `Config::config_path()` - Path del archivo de config
  - `Config::cache_path()` - Path del cache de tags
  - `Config::last_note_path()` - Path de última nota
  - `Config::load_default()` - Carga config por defecto
  
- Eliminados hardcoded paths en 10+ archivos
- Manejo de errores consistente (no más `expect()` en producción)

#### 📊 Métricas de Mejora
- **~600 líneas de código duplicado eliminadas** (15% del código total)
- **Reducción de complejidad** en archivos principales:
  - `tman.rs`: 591 → 463 líneas (-22%)
  - `cache.rs`: 108 → 62 líneas (-43%)
  - `prompts.rs`: 115 → 58 líneas (-50%)
  
- **6 tests unitarios añadidos**:
  - 3 tests para `TagPath` (tags simples, jerárquicos, starts_with)
  - 3 tests para `VaultWalker` (básico, hidden dirs, templates)

#### 🧪 Calidad de Código
- ✅ Todos los tests pasan
- ✅ Sin warnings de compilación
- ✅ Código más mantenible y testeable
- ✅ Mejor separación de responsabilidades

## Roadmap

### Futuras mejoras
- [ ] Separar `NoteBuilder` en Service/Command layers
- [ ] Implementar error types propios (thiserror)
- [ ] CI/CD con GitHub Actions
- [ ] Dividir `tman.rs` en sub-módulos
- [ ] Añadir más tests de integración

## Contribuciones

Las contribuciones son bienvenidas. Por favor:
1. Abre un issue para discutir cambios mayores
2. Sigue el estilo de código existente (usa `cargo fmt`)
3. Añade tests para nuevas funcionalidades
4. Ejecuta `cargo clippy` antes de hacer PR

## Licencia

MIT

## Créditos

Desarrollado por [Tu Nombre]. Inspirado por herramientas como Obsidian y sistemas de conocimiento personal.
