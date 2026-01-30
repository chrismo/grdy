# grdy

CLI tool to render JSON as tables. Opinionated, minimal flags.

## Name

~~Rename from `sqr` to `grdy` (crate + binary).~~ Done.

## Future Features

- Right-align numbers
- Max column width / truncation with `…`
- CSV/TSV input support
- YAML input support
- Colored values (nulls dim, booleans highlighted)
- `--headers` to specify/reorder columns
- `--select` to pick specific fields (like jq)
- Streaming/JSONL support
- Auto-detect if piped vs TTY (disable color when piped)
