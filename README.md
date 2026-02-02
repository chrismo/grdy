# grdy

A CLI tool to render JSON data as tables.

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

- `-c, --compact` - Use ASCII characters instead of Unicode box-drawing
- `-n, --no-color` - Disable colored output

## License

BSD 3-Clause
