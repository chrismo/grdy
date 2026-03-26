# Griddy

A CLI tool to render JSON data as tables.

Binary name: `grdy`

## Installation

### Homebrew (macOS)

```bash
brew install chrismo/grdy/grdy
```

### Shell installer

```bash
curl -fsSL https://raw.githubusercontent.com/chrismo/grdy/main/install.sh | bash
```

This installs to `~/.local/bin`. Make sure it's in your PATH:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

To install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/chrismo/grdy/main/install.sh | bash -s v0.6.0
```

## Usage

```bash
# From stdin
echo '[{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}]' | grdy

# From file
grdy data.json
```

Example output:
```
╭───────┬─────╮
│ name  │ age │
├───────┼─────┤
│ Alice │ 30  │
│ Bob   │ 25  │
╰───────┴─────╯
```

With `--ascii`:
```
+-------+-----+
| name  | age |
+-------+-----+
| Alice | 30  |
| Bob   | 25  |
+-------+-----+
```

Handles sparse keys, nested structures, and mixed types:
```bash
echo '[
  {"name": "Alice", "role": "admin", "active": true, "tags": ["go", "rust"]},
  {"name": "Bob", "role": "user", "active": false, "meta": {"level": 5}},
  {"name": "Charlie", "active": true, "score": 92.5}
]' | grdy
```
```
╭─────────┬───────┬────────┬───────────┬──────────┬───────╮
│ name    │ role  │ active │ tags      │ meta     │ score │
├─────────┼───────┼────────┼───────────┼──────────┼───────┤
│ Alice   │ admin │ true   │ [2 items] │          │       │
│ Bob     │ user  │ false  │           │ {1 keys} │       │
│ Charlie │       │ true   │           │          │ 92.5  │
╰─────────┴───────┴────────┴───────────┴──────────┴───────╯
```

### Options

- `-a, --ascii` - Use ASCII instead of Unicode box-drawing
- `-s, --stripe` - Dim alternate rows for readability

### Input formats

- JSON array of objects
- Single JSON object
- JSONL (newline-delimited JSON)

Nested arrays display as `[N items]` and nested objects as `{N keys}`.

### Configuration

Create `~/.config/grdy/config.json` (or `$XDG_CONFIG_HOME/grdy/config.json`) to set defaults:

```json
{
  "ascii": false,
  "stripe": false
}
```

CLI flags override config file settings.

## License

BSD 3-Clause
