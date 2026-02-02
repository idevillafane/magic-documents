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

### 🏷️ Gestión de Tags (tman)
- **Selector interactivo** con búsqueda fuzzy en tiempo real
- Listado y navegación de todos los tags del vault
- Renombrado masivo de tags (con soporte para jerarquías)
- Búsqueda de archivos por tag
- Cache automático para rendimiento óptimo
- Exclusión de carpeta de templates

### 📅 Funcionalidades Especiales
- **Daily notes**: Crea o abre la nota del día con `-d`
- **Últimas notas**: Lista las últimas N notas editadas con `-l`
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

# Mapeo de directorios trabajo → documentación (para mad -o / mad -q)
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
| `tag_root` | String | Directorio raíz para derivación de tags (usado en `mad -o/-q`) | `"Notas"` |
| `notes_dir` | String | Carpeta para notas generales | `"Notas"` |
| `diary_dir` | String | Carpeta para daily notes | `"Diario"` |
| `templates_dir` | String | Carpeta para templates | `"Templates"` |
| `dir_mappings` | HashMap | Mapeo de directorios trabajo → documentación (ver `mad -o`) | `{}` |

## Uso

### Sintaxis básica

```bash
# Crear nota con título (título al final)
mad "Mi Nota"
mad -n "Mi Nota"

# Abrir la última nota editada
mad -L

# Crear o abrir daily note
mad -d

# Listar últimas 5 notas editadas
mad -l

# Listar últimas N notas editadas
mad --last 10

# Usar editor configurado (flags combinables)
mad -e "Mi Nota"              # crear con editor config
mad -de                       # daily con editor config
mad -el                       # últimas 5 con editor config
mad -eL                       # última nota con editor config

# No agregar timestamp al abrir nota existente
mad -iL                       # última nota sin timestamp
mad -eiL                      # última nota con editor y sin timestamp
mad -il                       # listar últimas 5 y abrir sin timestamp

# Especificar editor personalizado
mad --editor nvim "Mi Nota"
mad --editor code -d
mad --last 10 --editor vim

# Gestión de tags (modo interactivo)
mad -t

# Listar tags (flags cortos combinables)
mad -tl                       # listar tags
mad --tman list               # forma larga

# Otras acciones de tags
mad -ta                       # listar todos (incluye Archived)
mad -tr                       # renombrar tags
mad -tf                       # buscar por tag
mad --tman rename             # forma larga

# Integración con Obsidian desde directorio productivo
mad -o "Título"               # crear/abrir nota desde directorio mapeado
mad -q "Título"               # alias corto (quick)
```

### Opciones principales

```
USAGE:
    mad [OPCIONES] [TÍTULO]

ARGS:
    <TÍTULO>               Título de la nota (debe ir al final, después de opciones)

OPCIONES:
    -h, --help             Muestra mensaje de ayuda
    -d, --daily            Crear o abrir daily note
    -o, --obsidian         Crear/abrir nota desde directorio mapeado (ver dir_mappings)
    -q, --quick            Alias de --obsidian
    -l                     Mostrar últimas 5 notas
    --last <N>             Mostrar últimas N notas
    -e                     Usar editor configurado
    --editor <EDITOR>      Usar editor específico
    -i, --no-id            No agregar timestamp al abrir nota existante
    -L, --last-note        Abrir última nota editada
    -n, --name <TÍTULO>    Nombre de la nota (alternativa al argumento posicional)
    -t[ACCIÓN]             Gestión de tags (incompatible con otras opciones)
    --tman <ACCIÓN>        Gestión de tags (forma larga)

FLAGS DE TMAN (combinables con -t):
    -tl, --tman list       Lista todos los tags (excluye Archived)
    -ta, --tman list-all   Lista todos los tags (incluye Archived)
    -tr, --tman rename     Renombrar tags
    -tf, --tman find       Buscar archivos por tag
    -t,  --tman            Modo interactivo (por defecto)

REGLAS:
    • Las opciones van ANTES del título posicional
    • -d, -l, y título son mutuamente excluyentes
    • -t/--tman es incompatible con todas las demás opciones
    • No se puede combinar -e con --editor
    • Flags cortos son combinables: -el, -de, -tl, etc.
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

### Sistema de Tags Jerárquicos

El sistema soporta tags jerárquicos que permiten organizar tus notas en estructuras de árbol. Los tags jerárquicos se representan de varias formas equivalentes:

#### Formatos Soportados

Todas estas formas son **equivalentes** y representan el mismo tag jerárquico `experta → ia-recuperos`:

```yaml
# Forma 1: Array con niveles separados
tags:
- experta
- ia-recuperos

# Forma 2: String con slash
tags:
- experta/ia-recuperos

# Forma 3: Mixto
tags:
- experta
- ia/recuperos
```

**Importante**: El array completo representa UN SOLO tag jerárquico. El orden de los elementos define la jerarquía.

#### Ejemplos Prácticos

```yaml
# Tag jerárquico de tres niveles: padre → hijo → nieto
tags:
- padre
- hijo
- nieto

# Equivalente usando slashes
tags:
- padre/hijo/nieto

# Equivalente mixto
tags:
- padre
- hijo/nieto
```

#### Tags en el Cuerpo del Documento

También puedes usar tags inline con `#`:

```markdown
#proyecto/cliente/acme
```

Estos tags con `#` se guardan preferentemente usando el formato slash en el frontmatter.

### Selector interactivo de tags

El sistema de tags te permite:
- **Búsqueda fuzzy en tiempo real**: Filtra tags mientras escribes
- **Vista jerárquica**: Muestra primero los padres, luego los paths completos
  - Ejemplo: primero `experta`, luego `experta → ia-recuperos`
- Seleccionar tags existentes navegando por la jerarquía
- Agregar nuevos tags personalizados con `/` para crear jerarquías
- Cache automático para rendimiento

Al crear una nota, los tags se guardan automáticamente en el frontmatter como array:

### tman - Tag Manager

El gestor de tags (`tman`) es una herramienta completa para administrar tags en tu vault:

#### Modo interactivo
```bash
mad -t
```

Menú interactivo con opciones para:
- 📋 Listar tags (flat view)
- ✏️ Renombrar tags
- 🔍 Buscar archivos por tag

#### Listar tags
```bash
# Listar todos los tags (excluye Archived)
mad -t list

# Listar todos los tags (incluye Archived)
mad -t list-all
```

Muestra todos los tags con sus jerarquías y cuenta de archivos. Los tags de la carpeta de templates son excluidos automáticamente.

#### Renombrar tags
```bash
mad -t rename
```

Permite renombrar tags de forma masiva con dos modos:
1. **Solo este nivel**: Renombra únicamente el tag seleccionado
2. **Recursivo**: Renombra el tag y todos sus sub-tags

Ejemplo:
- Tag original: `proyecto/cliente/acme`
- Nuevo nombre: `work/client/acme`
- Modo recursivo actualiza todos los archivos y sub-tags automáticamente

#### Buscar por tag
```bash
mad -t find
```

Búsqueda interactiva con **filtrado fuzzy en tiempo real**. Escribe parte del nombre del tag para filtrar la lista y ver rápidamente los archivos que lo contienen.

**Ejemplo de uso:**
- Escribes "experta" → Filtra y muestra tags que contienen "experta"
- Seleccionas "experta → ia-recuperos"
- Lista todos los archivos con ese tag

### Características de tman
- ✅ Excluye automáticamente carpetas ocultas (`.obsidian`, `.trash`, etc.)
- ✅ Excluye la carpeta de templates configurada
- ✅ Excluye tags "Archived" por defecto (usa `list-all` para incluirlos)
- ✅ Búsqueda fuzzy en tiempo real para filtrado rápido
- ✅ Soporte completo para tags jerárquicos con navegación por niveles
- ✅ Cache automático para mejor rendimiento

### Regenerar Cache de Tags

El sistema mantiene un cache de tags en `~/.mad/tags_cache.json` para mejorar el rendimiento. Si cambias tags manualmente en archivos o notas comportamiento extraño:

```bash
# Eliminar el cache (se regenera automáticamente en el próximo uso)
rm ~/.mad/tags_cache.json
```

El cache se actualiza automáticamente cuando:
- Creas una nueva nota con tags
- Renombras tags desde tman
- No existe el archivo de cache

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
mad -d
# Crea o abre: vault/Diario/2025-12-18.md
```

### Nota simple
```bash
mad "Ideas para el proyecto"
# Crea: vault/Notas/ideas-para-el-proyecto.md
```

### Últimas notas editadas
```bash
# Ver últimas 5 notas
mad -l

# Ver últimas 15 notas
mad -l 15
```

### Abrir última nota
```bash
mad ..
# Abre la última nota que editaste
```

### Gestión de tags
```bash
# Modo interactivo
mad -t

# Listar todos los tags
mad -t list

# Renombrar tags masivamente
mad -t rename

# Buscar archivos por tag
mad -t find
```

### Integración con Obsidian (mad -o / mad -q)

Documenta proyectos productivos en tu vault de Obsidian manteniendo estructura de directorios espejo mediante mapeo de directorios configurados.

#### Setup inicial

Configura el mapeo de directorios en `~/.config/magic-documents/config.toml`:

```toml
vault = "/ruta/a/tu/vault"
tag_root = "Notas"

[dir_mappings]
"/Users/tu/Developer" = "developer"
"/Users/tu/Proyectos/cliente-acme" = "proyectos/acme"
"/Users/tu/Bases-de-datos" = "databases"
```

**Explicación del mapeo:**
- **Clave**: Path absoluto del directorio de trabajo
- **Valor**: Path relativo dentro de `tag_root` donde se crearán los documentos

#### Uso

```bash
# Desde cualquier subdirectorio bajo un directorio mapeado
mad -o "Título de la nota"   # forma larga
mad -q "Título de la nota"   # forma corta (quick)

# Ejemplo desde ~/Developer/proyecto/backend/
mad -q "API Documentation"
# Crea: vault/Notas/developer/proyecto/backend/api-documentation.mad
# Tag: ["developer/proyecto/backend"]
```

#### Comportamiento

1. Detecta automáticamente el directorio de trabajo actual
2. Busca el mapeo más específico (prefijo más largo) que coincida
3. Calcula el path relativo desde el directorio mapeado hasta el directorio actual
4. Construye el tag combinando: `doc_subpath` + `relative_path`
5. Crea la estructura de directorios en `vault/tag_root/doc_subpath/relative_path/`
6. Solicita confirmación antes de crear directorios nuevos

#### Ejemplo completo

**Configuración:**
```toml
[dir_mappings]
"/Users/usuario/Developer" = "developer"
```

**Uso:**
```bash
# Navegar al proyecto
cd /Users/usuario/Developer/cliente-acme/api

# Crear nota
mad -q "Endpoints REST"

# Resultado:
# - Archivo: /vault/Notas/developer/cliente-acme/api/endpoints-rest.mad
# - Tag: ["developer/cliente-acme/api"]
# - Prompt: "Crear directorio? /vault/Notas/developer/cliente-acme/api"
```

**Ventajas sobre symlinks:**
- ✅ Configuración centralizada en un solo archivo
- ✅ Múltiples mapeos simultáneos
- ✅ No requiere crear symlinks en cada proyecto
- ✅ Más fácil de compartir configuración entre máquinas
- ✅ Soporta mapeos con diferentes niveles de profundidad

## Arquitectura del Proyecto

```
src/
├── commands/       # Comandos CLI (create, daily, last, recent, tman)
├── core/          # Lógica de negocio (config, note, template, frontmatter)
├── tags/          # Sistema de tags (cache, selector, tree)
├── ui/            # Interfaz de usuario (editor, prompts)
├── utils/         # Utilidades (cli, file)
├── lib.rs         # Módulo raíz
└── main.rs        # Entry point
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
- **Filtrado en tiempo real** en `tman find`
- **Vista jerárquica** en selector: padres primero, luego paths completos
- **Exclusión inteligente**: Archived, templates, carpetas ocultas
- **Cache optimizado**: Regeneración automática cuando es necesario

#### 📝 Escritura Consistente
- Tags se guardan como arrays en YAML
- Cada elemento del array es un nivel de jerarquía
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
