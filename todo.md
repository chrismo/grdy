# Griddy

CLI tool to render JSON as tables. Opinionated, minimal flags.

Binary name: `grdy`

## Rename?

Consider renaming to **jaat** - "JSON as a Table". More descriptive, easier to type than `grdy`.

## Future Features

- Reserved metadata key (`_grdy`) for per-invocation formatting hints (e.g. column alignment, hide columns)
- Max column width / truncation with `…`
- Colored values (nulls dim, booleans highlighted)

## Maintenance

- Check release builds on Ubuntu 26 (`ubuntu-latest` migrates starting 2026-10-19)
