# Griddy

CLI tool to render JSON as tables. Opinionated, minimal flags.

Binary name: `grdy`

## Rename?

Consider renaming to **jaat** - "JSON as a Table". More descriptive, easier to type than `grdy`.

## Distribution

- Homebrew tap/formula

## Future Features

- Right-align numbers (JSON `Number` type)
- Reserved metadata key (`_grdy`) for per-invocation formatting hints (e.g. column alignment, hide columns)
- Max column width / truncation with `…`
- Colored values (nulls dim, booleans highlighted)
- Auto-detect if piped vs TTY (disable styling when piped)
