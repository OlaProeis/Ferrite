# Ferrite Command Line Interface

Ferrite supports command-line arguments for opening files and directories directly from the terminal.

## Usage

```bash
ferrite [OPTIONS] [PATH]...
```

## Arguments

### `[PATH]...` - Files or directories to open

Pass one or more file paths to open them as tabs, or a directory to open it as a workspace.

**Examples:**

```bash
# Open a single file
ferrite README.md

# Open multiple files as tabs
ferrite file1.md file2.md notes.md

# Open current directory as workspace
ferrite .

# Open a specific directory as workspace
ferrite ~/projects/my-project

# Mix of files (first file gets focus)
ferrite README.md CHANGELOG.md docs/getting-started.md
```

## Options

### `-h, --help`

Print help information and exit.

```bash
ferrite --help
ferrite -h
```

### `-V, --version`

Print version information and exit.

```bash
ferrite --version
ferrite -V
```

### `--log-level <LEVEL>`

Set the log level for debugging. This overrides the `log_level` setting in `config.json`.

Valid values: `debug`, `info`, `warn`, `error`, `off`

```bash
# Enable debug logging
ferrite --log-level debug

# Show only errors
ferrite --log-level error

# Disable all logging
ferrite --log-level off

# Combine with file arguments
ferrite --log-level debug README.md
```

**Log Level Precedence:**
1. CLI flag (`--log-level`)
2. Config file (`config.json`)
3. Built-in default (`warn`)

### `--new-instance`

*(v0.3.1)* Start a new Ferrite process even if another instance is already running. Without this flag, a second launch forwards its paths to the running instance and exits (unless **Settings → Files → Allow multiple Ferrite windows/instances** is on).

```bash
# Open a file in a separate Ferrite process
ferrite --new-instance notes.md
```

A `--new-instance` process does not write the single-instance lock/pid files and keeps its session and config writes separate from the primary instance. See [`technical/platform/single-instance.md`](technical/platform/single-instance.md) and [`technical/files/session-persistence.md`](technical/files/session-persistence.md).

## Behavior

### Opening Files

- Each file path is opened as a separate tab
- The first successfully opened file gets focus
- Non-existent files are logged as warnings and skipped
- Relative paths are resolved to absolute paths

### Opening Directories

- A single directory can be passed to open it as a workspace
- The workspace mode enables:
  - File tree navigation
  - Quick switcher (Ctrl+P)
  - Search in files (Ctrl+Shift+F)
  - Git status indicators
- If multiple directories are provided, only the first one is used

### Mixed Arguments

When both files and a directory are provided:

- The directory becomes the workspace root
- Files are opened as tabs within that workspace context

### Error Handling

- Non-existent paths produce a warning in the log but don't crash the app
- Permission errors are logged and the file is skipped
- Invalid paths (neither file nor directory) are skipped with a warning

## Examples

### Basic Usage

```bash
# Start with empty editor
ferrite

# Edit a markdown file
ferrite ~/Documents/notes.md

# Open project as workspace
ferrite ~/projects/my-rust-project
```

### Shell Integration

```bash
# Open file from pipe (not directly supported, use temp file)
# Instead, use shell redirection:
cat file.md | ferrite /dev/stdin  # Linux/macOS

# Edit git commit message
GIT_EDITOR="ferrite" git commit

# Open all markdown files in current directory
ferrite *.md
```

### Scripting

```bash
# Check version in scripts
VERSION=$(ferrite --version)
echo "Using $VERSION"

# Conditional launch
if [ -f "README.md" ]; then
    ferrite README.md
fi
```

## Environment Variables

Ferrite reads log configuration from `config.json` and supports CLI overrides. The `RUST_LOG` environment variable is no longer used; use `--log-level` instead.

| Method | Example | Priority |
|--------|---------|----------|
| CLI flag | `ferrite --log-level debug` | Highest |
| Config file | `"log_level": "debug"` in config.json | Medium |
| Built-in default | `warn` | Lowest |

## Platform Notes

### Windows

```powershell
# PowerShell
.\ferrite.exe README.md

# CMD
ferrite.exe README.md

# Open current directory
ferrite.exe .
```

### Linux/macOS

```bash
# Standard usage
./ferrite README.md

# If installed to PATH
ferrite README.md
```

## Related

- [Building from Source](building.md)
- [Configuration](../README.md#configuration)
