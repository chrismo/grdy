# Griddy

A CLI tool to render JSON data as tables.

Binary name: `grdy`

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/chrismo/grdy/main/install.sh | bash
```

To install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/chrismo/grdy/main/install.sh | bash -s v0.1.0
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
╭─────┬───────╮
│ age │ name  │
├─────┼───────┤
│ 30  │ Alice │
│ 25  │ Bob   │
╰─────┴───────╯
```

With `--ascii`:
```
+-----+-------+
| age | name  |
+-----+-------+
| 30  | Alice |
| 25  | Bob   |
+-----+-------+
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
╭────────┬─────────┬───────┬───────────┬──────────┬───────╮
│ active │ name    │ role  │ tags      │ meta     │ score │
├────────┼─────────┼───────┼───────────┼──────────┼───────┤
│ true   │ Alice   │ admin │ [2 items] │          │       │
│ false  │ Bob     │ user  │           │ {1 keys} │       │
│ true   │ Charlie │       │           │          │ 92.5  │
╰────────┴─────────┴───────┴───────────┴──────────┴───────╯
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

Create `~/.config/grdy/config.json` to set defaults:

```json
{
  "ascii": false,
  "stripe": false
}
```

CLI flags override config file settings.

## License

BSD 3-Clause
