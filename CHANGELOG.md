# Griddy Changelog

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
