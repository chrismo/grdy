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

### Options

- `-a, --ascii` - Use ASCII instead of Unicode box-drawing
- `-s, --stripe` - Dim alternate rows for readability

### Input formats

- JSON array of objects
- Single JSON object
- JSONL (newline-delimited JSON)

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
