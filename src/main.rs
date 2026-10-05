use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

#[derive(Parser)]
#[command(name = "Griddy", version, about = "Render JSON data as tables")]
struct Args {
    /// Input file (reads from stdin if not provided)
    file: Option<String>,

    /// Use ASCII instead of Unicode box-drawing
    #[arg(short, long)]
    ascii: bool,

    /// Dim alternate rows for readability
    #[arg(short, long)]
    stripe: bool,

    /// Aligned columns only, no borders or lines
    #[arg(short, long)]
    plain: bool,

    /// Hide the header row
    #[arg(short = 'H', long)]
    no_header: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    ascii: bool,
    #[serde(default)]
    stripe: bool,
    #[serde(default)]
    plain: bool,
    #[serde(default)]
    no_header: bool,
}

impl Config {
    fn load() -> Self {
        Self::config_path()
            .map(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    /// Missing file means defaults; an unreadable or invalid file warns and uses defaults.
    fn load_from(path: &Path) -> Self {
        let result = match std::fs::read_to_string(path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Self::default(),
            Err(e) => Err(e.to_string()),
            Ok(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        };
        result.unwrap_or_else(|e| {
            eprintln!("grdy: ignoring config {}: {}", path.display(), e);
            Self::default()
        })
    }

    fn config_path() -> Option<PathBuf> {
        Self::resolve_path(
            env::var_os("XDG_CONFIG_HOME"),
            dirs::home_dir(),
            dirs::config_dir(),
        )
    }

    /// Prefers `$XDG_CONFIG_HOME` (or `~/.config`), then the platform config dir
    /// (e.g. ~/Library/Application Support on macOS) if a config exists there.
    fn resolve_path(
        xdg: Option<OsString>,
        home: Option<PathBuf>,
        platform: Option<PathBuf>,
    ) -> Option<PathBuf> {
        // XDG spec: empty or relative values are ignored
        let xdg = xdg.map(PathBuf::from).filter(|p| p.is_absolute());
        let preferred = xdg
            .or_else(|| home.map(|h| h.join(".config")))
            .map(|d| d.join("grdy").join("config.json"));
        let fallback = platform.map(|d| d.join("grdy").join("config.json"));

        if preferred.as_ref().is_some_and(|p| p.exists()) {
            return preferred;
        }
        fallback.filter(|p| p.exists()).or(preferred)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Style {
    Unicode,
    Ascii,
    Plain,
}

struct TableChars {
    top_left: &'static str,
    top_right: &'static str,
    bottom_left: &'static str,
    bottom_right: &'static str,
    horizontal: &'static str,
    vertical: &'static str,
    top_tee: &'static str,
    bottom_tee: &'static str,
    left_tee: &'static str,
    right_tee: &'static str,
    cross: &'static str,
}

const UNICODE_CHARS: TableChars = TableChars {
    top_left: "╭",
    top_right: "╮",
    bottom_left: "╰",
    bottom_right: "╯",
    horizontal: "─",
    vertical: "│",
    top_tee: "┬",
    bottom_tee: "┴",
    left_tee: "├",
    right_tee: "┤",
    cross: "┼",
};

const ASCII_CHARS: TableChars = TableChars {
    top_left: "+",
    top_right: "+",
    bottom_left: "+",
    bottom_right: "+",
    horizontal: "-",
    vertical: "|",
    top_tee: "+",
    bottom_tee: "+",
    left_tee: "+",
    right_tee: "+",
    cross: "+",
};

fn no_color(value: Option<&OsStr>) -> bool {
    value.is_some()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("grdy: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();
    let config = Config::load();

    // CLI flags override config; suppress ANSI when NO_COLOR is set or stdout is not a tty
    let style = if args.plain || config.plain {
        Style::Plain
    } else if args.ascii || config.ascii {
        Style::Ascii
    } else {
        Style::Unicode
    };
    let allow_ansi = !no_color(env::var_os("NO_COLOR").as_deref()) && io::stdout().is_terminal();
    let no_header = args.no_header || config.no_header;
    let stripe = (args.stripe || config.stripe) && allow_ansi;

    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))?,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("stdin: {}", e))?;
            buf
        }
    };

    let rows = parse_json(&input)?;
    let output = render_table(&rows, style, stripe, no_header);
    match io::stdout().write_all(output.as_bytes()) {
        // Reader went away (e.g. `grdy | head`); not an error worth reporting
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(|e| format!("stdout: {}", e)),
    }
}

fn parse_json(input: &str) -> Result<Vec<Value>, String> {
    // Try parsing as a single JSON value first
    let doc_err = match serde_json::from_str::<Value>(input) {
        Err(e) => e,
        Ok(value) => {
            return match value {
                Value::Array(arr) => match arr.iter().position(|v| !v.is_object()) {
                    Some(i) => Err(format!(
                        "Expected JSON objects in array, found non-object at index {}",
                        i
                    )),
                    None => Ok(arr),
                },
                obj @ Value::Object(_) => Ok(vec![obj]),
                _ => Err("Expected JSON array or object".to_string()),
            };
        }
    };

    // Try parsing as JSONL (newline-delimited JSON)
    let mut rows = Vec::new();
    for (i, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(obj @ Value::Object(_)) => rows.push(obj),
            Ok(_) => {
                return Err(format!(
                    "Expected JSON objects in JSONL input at line {}",
                    i + 1
                ));
            }
            // Nothing parsed as JSONL yet: it was likely one (pretty-printed) document, so its error is the useful one
            Err(_) if rows.is_empty() => return Err(format!("Invalid JSON: {}", doc_err)),
            Err(e) => {
                // serde reports positions relative to the single line; rewrite them for the whole input
                let msg = e.to_string();
                let msg = msg
                    .rsplit_once(" at line ")
                    .map_or(msg.as_str(), |(m, _)| m);
                return Err(format!(
                    "Invalid JSON: {} at line {} column {}",
                    msg,
                    i + 1,
                    e.column()
                ));
            }
        }
    }
    Ok(rows)
}

fn render_table(rows: &[Value], style: Style, stripe: bool, no_header: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }

    // Extract columns from all objects, preserving insertion order
    let mut seen: HashSet<String> = HashSet::new();
    let mut columns: Vec<String> = Vec::new();
    for row in rows {
        if let Value::Object(obj) = row {
            for key in obj.keys() {
                if seen.insert(key.clone()) {
                    columns.push(key.clone());
                }
            }
        }
    }

    if columns.is_empty() {
        return String::new();
    }

    // Build cells, widths, and numeric detection in one pass. A column is numeric
    // when it has at least one number and every other present value is null.
    let mut widths: Vec<usize> = if no_header {
        vec![0; columns.len()]
    } else {
        columns.iter().map(|c| display_width(c)).collect()
    };
    let mut has_number = vec![false; columns.len()];
    let mut has_non_number = vec![false; columns.len()];
    let mut table_data: Vec<Vec<String>> = Vec::with_capacity(rows.len());
    for row in rows {
        let row_data: Vec<String> = columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let value = row.get(col);
                match value {
                    Some(Value::Number(_)) => has_number[i] = true,
                    Some(Value::Null) | None => {}
                    Some(_) => has_non_number[i] = true,
                }
                let cell = value.map(format_value).unwrap_or_default();
                widths[i] = widths[i].max(display_width(&cell));
                cell
            })
            .collect();
        table_data.push(row_data);
    }
    let numeric: Vec<bool> = has_number
        .iter()
        .zip(&has_non_number)
        .map(|(n, x)| *n && !x)
        .collect();

    let chars = match style {
        Style::Unicode => Some(&UNICODE_CHARS),
        Style::Ascii => Some(&ASCII_CHARS),
        Style::Plain => None,
    };

    let mut output = String::new();
    if let Some(chars) = chars {
        output.push_str(&render_rule(
            &widths,
            chars,
            chars.top_left,
            chars.top_tee,
            chars.top_right,
        ));
    }
    if !no_header {
        output.push_str(&render_row(
            &columns, &widths, &numeric, chars, stripe, None,
        ));
        if let Some(chars) = chars {
            output.push_str(&render_rule(
                &widths,
                chars,
                chars.left_tee,
                chars.cross,
                chars.right_tee,
            ));
        }
    }
    for (i, row) in table_data.iter().enumerate() {
        output.push_str(&render_row(row, &widths, &numeric, chars, stripe, Some(i)));
    }
    if let Some(chars) = chars {
        output.push_str(&render_rule(
            &widths,
            chars,
            chars.bottom_left,
            chars.bottom_tee,
            chars.bottom_right,
        ));
    }
    output
}

/// Returns the display width of a string, ignoring ANSI escape sequences.
/// Width is measured on the whole visible text so emoji sequences (ZWJ, VS16) count correctly.
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(strip_ansi(s).as_str())
}

/// Removes CSI (`ESC [ ... final`) and OSC (`ESC ] ... BEL|ESC \`) sequences.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for c in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn format_value(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => escape_control_chars(s),
        Value::Array(arr) => format!("[{}]", plural(arr.len(), "item")),
        Value::Object(obj) => format!("{{{}}}", plural(obj.len(), "key")),
    }
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{} {}", n, noun)
    } else {
        format!("{} {}s", n, noun)
    }
}

/// Escapes control characters that would break table layout, keeping ESC for ANSI styling.
fn escape_control_chars(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_control() && c != '\x1b' {
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Renders a horizontal rule: `left`, then each column's span joined by `mid`, then `right`.
fn render_rule(widths: &[usize], chars: &TableChars, left: &str, mid: &str, right: &str) -> String {
    let spans: Vec<String> = widths
        .iter()
        .map(|w| chars.horizontal.repeat(w + 2))
        .collect();
    format!("{}{}{}\n", left, spans.join(mid), right)
}

fn render_row(
    cells: &[String],
    widths: &[usize],
    numeric: &[bool],
    chars: Option<&TableChars>,
    stripe: bool,
    row_index: Option<usize>,
) -> String {
    let is_header = row_index.is_none();
    let dim = stripe && row_index.is_some_and(|i| i % 2 == 1);

    let padded: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            let padding = " ".repeat(widths[i] - display_width(cell));
            if is_header && stripe {
                format!("\x1b[1m{}\x1b[0m{}", cell, padding)
            } else if numeric[i] && !is_header {
                format!("{}{}", padding, cell)
            } else {
                format!("{}{}", cell, padding)
            }
        })
        .collect();

    let mut s = String::new();

    if dim {
        s.push_str("\x1b[2m");
    }

    match chars {
        Some(chars) => {
            s.push_str(chars.vertical);
            for cell in &padded {
                s.push_str(&format!(" {} ", cell));
                s.push_str(chars.vertical);
            }
        }
        // Plain: two-space gutter, no trailing whitespace
        None => s.push_str(padded.join("  ").trim_end()),
    }

    if dim {
        s.push_str("\x1b[0m");
    }
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_json {
        use super::*;

        #[test]
        fn array_of_objects() {
            let input = r#"[{"a": 1}, {"a": 2}]"#;
            let rows = parse_json(input).unwrap();
            assert_eq!(rows.len(), 2);
        }

        #[test]
        fn single_object() {
            let input = r#"{"a": 1, "b": 2}"#;
            let rows = parse_json(input).unwrap();
            assert_eq!(rows.len(), 1);
        }

        #[test]
        fn jsonl() {
            let input = "{\"a\": 1}\n{\"a\": 2}\n{\"a\": 3}";
            let rows = parse_json(input).unwrap();
            assert_eq!(rows.len(), 3);
        }

        #[test]
        fn jsonl_with_blank_lines() {
            let input = "{\"a\": 1}\n\n{\"a\": 2}\n";
            let rows = parse_json(input).unwrap();
            assert_eq!(rows.len(), 2);
        }

        #[test]
        fn empty_array() {
            assert_eq!(parse_json("[]").unwrap().len(), 0);
        }

        #[test]
        fn array_with_non_object_is_an_error() {
            let err = parse_json(r#"[{"a": 1}, 5]"#).unwrap_err();
            assert!(err.contains("index 1"), "{}", err);
        }

        #[test]
        fn pretty_printed_syntax_error_reports_real_location() {
            let err = parse_json("[\n  {\"a\": 1},\n]\n").unwrap_err();
            assert!(err.contains("line 3"), "{}", err);
        }

        #[test]
        fn jsonl_syntax_error_reports_line_number() {
            let err = parse_json("{\"a\": 1}\n{\"a\": 2}\n{\"a\": }\n").unwrap_err();
            assert!(err.contains("line 3"), "{}", err);
        }

        #[test]
        fn array_of_scalars_is_an_error() {
            assert!(parse_json("[1, 2, 3]").is_err());
        }
    }

    mod format_value {
        use super::*;

        #[test]
        fn null() {
            assert_eq!(format_value(&Value::Null), "null");
        }

        #[test]
        fn bool_true() {
            assert_eq!(format_value(&Value::Bool(true)), "true");
        }

        #[test]
        fn bool_false() {
            assert_eq!(format_value(&Value::Bool(false)), "false");
        }

        #[test]
        fn number_int() {
            assert_eq!(format_value(&serde_json::json!(42)), "42");
        }

        #[test]
        fn number_float() {
            assert_eq!(format_value(&serde_json::json!(2.5)), "2.5");
        }

        #[test]
        fn string() {
            assert_eq!(format_value(&serde_json::json!("hello")), "hello");
        }

        #[test]
        fn string_control_chars_escaped() {
            assert_eq!(
                format_value(&serde_json::json!("a\nb\tc\rd\u{1}")),
                r"a\nb\tc\rd\u{1}"
            );
        }

        #[test]
        fn string_ansi_escape_preserved() {
            assert_eq!(
                format_value(&serde_json::json!("\x1b[31mred\x1b[0m")),
                "\x1b[31mred\x1b[0m"
            );
        }

        #[test]
        fn array() {
            assert_eq!(format_value(&serde_json::json!([1, 2, 3])), "[3 items]");
        }

        #[test]
        fn object() {
            assert_eq!(
                format_value(&serde_json::json!({"a": 1, "b": 2})),
                "{2 keys}"
            );
        }

        #[test]
        fn singular_counts() {
            assert_eq!(format_value(&serde_json::json!([1])), "[1 item]");
            assert_eq!(format_value(&serde_json::json!({"a": 1})), "{1 key}");
        }
    }

    mod display_width {
        use super::*;

        #[test]
        fn plain_ascii() {
            assert_eq!(display_width("hello"), 5);
        }

        #[test]
        fn ignores_csi_sequences() {
            assert_eq!(display_width("\x1b[31mred\x1b[0m"), 3);
        }

        #[test]
        fn ignores_osc_hyperlinks() {
            assert_eq!(
                display_width("\x1b]8;;http://x.io\x1b\\click\x1b]8;;\x1b\\"),
                5
            );
            assert_eq!(display_width("\x1b]8;;http://x.io\x07click\x1b]8;;\x07"), 5);
        }

        #[test]
        fn emoji_with_variation_selector() {
            assert_eq!(display_width("❤️"), 2);
        }

        #[test]
        fn zwj_emoji_sequence() {
            assert_eq!(display_width("👨‍👩‍👧"), 2);
        }
    }

    mod render_table {
        use super::*;
        use insta::assert_snapshot;

        fn assert_aligned(output: &str) {
            let widths: Vec<usize> = output.lines().map(display_width).collect();
            assert!(
                widths.windows(2).all(|w| w[0] == w[1]),
                "misaligned: {:?}\n{}",
                widths,
                output
            );
        }

        #[test]
        fn all_null_column_is_not_numeric() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"value": null}),
                serde_json::json!({"value": null}),
            ];
            assert_eq!(
                render_table(&rows, Style::Plain, false, false),
                "value\nnull\nnull\n"
            );
        }

        #[test]
        fn zwj_emoji_header() {
            let rows: Vec<Value> = vec![serde_json::json!({"👨‍👩‍👧": 1})];
            assert_aligned(&render_table(&rows, Style::Unicode, false, false));
        }

        #[test]
        fn emoji_cells() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"a": "❤️", "b": 1}),
                serde_json::json!({"a": "xx", "b": 2}),
            ];
            assert_aligned(&render_table(&rows, Style::Unicode, false, false));
        }

        #[test]
        fn empty_input() {
            let rows: Vec<Value> = vec![];
            assert_eq!(render_table(&rows, Style::Unicode, false, false), "");
        }

        #[test]
        fn single_row() {
            let rows: Vec<Value> = vec![serde_json::json!({"name": "Alice", "age": 30})];
            assert_snapshot!(render_table(&rows, Style::Unicode, false, false));
        }

        #[test]
        fn single_row_ascii() {
            let rows: Vec<Value> = vec![serde_json::json!({"name": "Alice", "age": 30})];
            assert_snapshot!(render_table(&rows, Style::Ascii, false, false));
        }

        #[test]
        fn multiple_rows() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 25}),
                serde_json::json!({"name": "Charlie", "age": 35}),
            ];
            assert_snapshot!(render_table(&rows, Style::Unicode, false, false));
        }

        #[test]
        fn multiple_rows_with_stripe() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 25}),
                serde_json::json!({"name": "Charlie", "age": 35}),
                serde_json::json!({"name": "Diana", "age": 28}),
            ];
            assert_snapshot!(render_table(&rows, Style::Unicode, true, false));
        }

        #[test]
        fn sparse_data() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"a": 1}),
                serde_json::json!({"b": 2}),
                serde_json::json!({"a": 3, "b": 4}),
            ];
            assert_snapshot!(render_table(&rows, Style::Unicode, false, false));
        }

        #[test]
        fn preserves_key_order() {
            // Keys are in non-alphabetical order in the JSON; columns should match input order
            let rows = parse_json(r#"[{"zebra": 1, "apple": 2, "mango": 3}]"#).unwrap();
            let output = render_table(&rows, Style::Ascii, false, false);
            let header_line = output.lines().nth(1).unwrap();
            let headers: Vec<&str> = header_line
                .split('|')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            assert_eq!(headers, vec!["zebra", "apple", "mango"]);
        }

        #[test]
        fn multiple_rows_plain() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30, "city": "Austin"}),
                serde_json::json!({"name": "Bob", "age": 5, "city": "NYC"}),
            ];
            assert_eq!(
                render_table(&rows, Style::Plain, false, false),
                "name   age  city\nAlice   30  Austin\nBob      5  NYC\n"
            );
        }

        #[test]
        fn multiple_rows_plain_with_stripe() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 25}),
            ];
            assert_snapshot!(render_table(&rows, Style::Plain, true, false));
        }

        #[test]
        fn no_header() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 5}),
            ];
            assert_eq!(
                render_table(&rows, Style::Unicode, false, true),
                "╭───────┬────╮\n│ Alice │ 30 │\n│ Bob   │  5 │\n╰───────┴────╯\n"
            );
        }

        #[test]
        fn no_header_plain() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 5}),
            ];
            assert_eq!(
                render_table(&rows, Style::Plain, false, true),
                "Alice  30\nBob     5\n"
            );
        }

        #[test]
        fn nested_values() {
            let rows: Vec<Value> = vec![serde_json::json!({"data": [1, 2, 3], "meta": {"x": 1}})];
            assert_snapshot!(render_table(&rows, Style::Unicode, false, false));
        }

        #[test]
        fn ansi_colored_values() {
            let green = "\x1b[32m●\x1b[0m";
            let red = "\x1b[31m●\x1b[0m";
            let rows: Vec<Value> = vec![
                serde_json::json!({"status": green, "name": "api-server"}),
                serde_json::json!({"status": red, "name": "db-worker"}),
            ];
            assert_snapshot!(render_table(&rows, Style::Ascii, false, false));
        }
    }

    mod config {
        use super::*;
        use std::fs;

        fn write_config(dir: &std::path::Path, body: &str) -> PathBuf {
            let path = dir.join("grdy").join("config.json");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, body).unwrap();
            path
        }

        #[test]
        fn prefers_xdg_config_home() {
            let xdg = tempfile::tempdir().unwrap();
            let platform = tempfile::tempdir().unwrap();
            let expected = write_config(xdg.path(), "{}");
            write_config(platform.path(), "{}");
            let path =
                Config::resolve_path(Some(xdg.path().into()), None, Some(platform.path().into()));
            assert_eq!(path, Some(expected));
        }

        #[test]
        fn ignores_empty_xdg_config_home() {
            let home = tempfile::tempdir().unwrap();
            let path = Config::resolve_path(Some("".into()), Some(home.path().into()), None);
            assert_eq!(
                path,
                Some(home.path().join(".config").join("grdy").join("config.json"))
            );
        }

        #[test]
        fn ignores_relative_xdg_config_home() {
            let home = tempfile::tempdir().unwrap();
            let path = Config::resolve_path(Some("rel".into()), Some(home.path().into()), None);
            assert_eq!(
                path,
                Some(home.path().join(".config").join("grdy").join("config.json"))
            );
        }

        #[test]
        fn falls_back_to_platform_dir_when_it_exists() {
            let xdg = tempfile::tempdir().unwrap();
            let platform = tempfile::tempdir().unwrap();
            let expected = write_config(platform.path(), "{}");
            let path =
                Config::resolve_path(Some(xdg.path().into()), None, Some(platform.path().into()));
            assert_eq!(path, Some(expected));
        }

        #[test]
        fn defaults_to_xdg_path_when_nothing_exists() {
            let xdg = tempfile::tempdir().unwrap();
            let platform = tempfile::tempdir().unwrap();
            let path =
                Config::resolve_path(Some(xdg.path().into()), None, Some(platform.path().into()));
            assert_eq!(path, Some(xdg.path().join("grdy").join("config.json")));
        }

        #[test]
        fn load_from_reads_settings() {
            let tmp = tempfile::tempdir().unwrap();
            let path = write_config(tmp.path(), r#"{"ascii": true, "stripe": true}"#);
            let config = Config::load_from(&path);
            assert!(config.ascii);
            assert!(config.stripe);
        }

        #[test]
        fn load_from_missing_file_returns_defaults() {
            let tmp = tempfile::tempdir().unwrap();
            let config = Config::load_from(&tmp.path().join("nope.json"));
            assert!(!config.ascii);
            assert!(!config.stripe);
        }

        #[test]
        fn load_from_invalid_file_returns_defaults() {
            let tmp = tempfile::tempdir().unwrap();
            let path = write_config(tmp.path(), r#"{"ascii": tru}"#);
            assert!(!Config::load_from(&path).ascii);
        }

        #[test]
        fn partial_config() {
            let config: Config = serde_json::from_str(r#"{"ascii": true}"#).unwrap();
            assert!(config.ascii);
            assert!(!config.stripe);
        }
    }

    mod no_color {
        use super::*;
        #[test]
        fn returns_true_when_set() {
            assert!(no_color(Some(OsStr::new("1"))));
        }

        #[test]
        fn returns_true_when_empty() {
            assert!(no_color(Some(OsStr::new(""))));
        }

        #[test]
        fn returns_false_when_unset() {
            assert!(!no_color(None));
        }
    }
}
