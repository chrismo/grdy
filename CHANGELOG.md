# Griddy Changelog

## [0.8.0] - 2026-10-10

### Changed
- Arrays containing non-object values are now an error (previously rendered broken rows)
- Control characters in string values (newlines, tabs) are shown escaped instead of breaking the table
- Nested value counts use singular forms (`[1 item]`, `{1 key}`)
- Columns containing only `null` are no longer right-aligned as numeric
- Invalid config files now print a warning instead of being silently ignored
- Releases include static Linux builds (`*-unknown-linux-musl`); `install.sh` prefers them

### Fixed
- Crash on column names containing multi-codepoint emoji (e.g. ZWJ sequences)
- Misaligned columns for emoji with variation selectors and OSC 8 hyperlinks
- Missing files, non-UTF-8 input, and closed pipes (`grdy | head`) now exit cleanly instead of panicking
- Syntax errors in pretty-printed JSON report the real line and column
- Empty or relative `$XDG_CONFIG_HOME` is ignored (previously could load `./grdy/config.json`)
- `install.sh` accepts versions with or without a leading `v`

## [0.7.0] - 2026-09-29

### Added
- `-p, --plain` option: aligned columns with no borders or lines
- `-H, --no-header` option: hide the header row (works with any style)

## [0.6.0] - 2026-03-26

### Added
- `NO_COLOR` environment variable support (suppresses ANSI formatting)
- Automatic tty detection (ANSI formatting suppressed when stdout is piped)
- ANSI-aware column width calculation (input containing escape codes no longer inflates column widths)

## [0.5.0] - 2026-03-24

### Added
- Right-align numeric columns (JSON `Number` type auto-detected)

### Fixed
- Config file now found via `$XDG_CONFIG_HOME` / `~/.config` on all platforms

## [0.4.0] - 2026-02-03

### Fixed
- Columns now preserve JSON key order instead of being alphabetically sorted

## [0.3.0] - 2026-02-02

### Added
- "Griddy" branding (binary remains `grdy`)

### Changed
- Renamed `--compact` to `--ascii`
- Renamed `--plain` to `--stripe` (now opt-in for styling)
- Plain output is now the default (no bold headers, no dim rows)
- `--stripe` enables bold headers and dim alternate rows

## [0.2.0] - 2026-02-02

### Added
- Config file support (`~/.config/grdy/config.json`)
- JSONL (newline-delimited JSON) input support
- `--plain` flag to disable styling
- `-V` / `--version` flag
- Unit and snapshot tests

### Changed
- Removed `--no-color` flag

## [0.1.0] - 2026-02-01

### Added
- Initial release
- Render JSON arrays/objects as tables
- Unicode box-drawing characters (default)
- `--compact` flag for ASCII output
- Bold headers
