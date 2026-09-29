use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::env;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;
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
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn config_path() -> Option<PathBuf> {
        // Check XDG_CONFIG_HOME first, then fall back to dirs::config_dir()
        let xdg = env::var("XDG_CONFIG_HOME").ok().map(PathBuf::from);
        let base = xdg.or_else(|| dirs::home_dir().map(|h| h.join(".config")));
        let xdg_path = base.map(|d| d.join("grdy").join("config.json"));

        if let Some(ref p) = xdg_path {
            if p.exists() {
                return xdg_path;
            }
        }

        // Fall back to platform default (e.g. ~/Library/Application Support on macOS)
        let platform_path = dirs::config_dir().map(|d| d.join("grdy").join("config.json"));
        if let Some(ref p) = platform_path {
            if p.exists() {
                return platform_path;
            }
        }

        // Return XDG path as the preferred default even if nothing exists yet
        xdg_path
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

fn no_color() -> bool {
    env::var_os("NO_COLOR").is_some()
}

fn main() {
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
    let allow_ansi = !no_color() && io::stdout().is_terminal();
    let no_header = args.no_header || config.no_header;
    let stripe = (args.stripe || config.stripe) && allow_ansi;

    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path).expect("Failed to read file"),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).expect("Failed to read stdin");
            buf
        }
    };

    let rows = parse_json(&input);
    let output = render_table(&rows, style, stripe, no_header);
    print!("{}", output);
}

fn parse_json(input: &str) -> Vec<Value> {
    // Try parsing as a single JSON value first
    if let Ok(value) = serde_json::from_str::<Value>(input) {
        return match value {
            Value::Array(arr) => arr,
            obj @ Value::Object(_) => vec![obj],
            _ => {
                eprintln!("Expected JSON array or object");
                std::process::exit(1);
            }
        };
    }

    // Try parsing as JSONL (newline-delimited JSON)
    let mut rows = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(obj @ Value::Object(_)) => rows.push(obj),
            Ok(_) => {
                eprintln!("Expected JSON objects in JSONL input");
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Invalid JSON: {}", e);
                std::process::exit(1);
            }
        }
    }
    rows
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

    // Build table data
    let mut table_data: Vec<Vec<String>> = Vec::new();
    for row in rows {
        let mut row_data = Vec::new();
        if let Value::Object(obj) = row {
            for col in &columns {
                let cell = obj.get(col).map(|v| format_value(v)).unwrap_or_default();
                row_data.push(cell);
            }
        }
        table_data.push(row_data);
    }

    // Detect which columns are numeric (all non-null, non-missing values are numbers)
    let mut numeric: Vec<bool> = vec![true; columns.len()];
    for row in rows {
        if let Value::Object(obj) = row {
            for (i, col) in columns.iter().enumerate() {
                if let Some(v) = obj.get(col) {
                    if !v.is_number() && !v.is_null() {
                        numeric[i] = false;
                    }
                }
            }
        }
    }

    // Calculate column widths
    let mut widths: Vec<usize> = if no_header {
        vec![0; columns.len()]
    } else {
        columns.iter().map(|c| UnicodeWidthStr::width(c.as_str())).collect()
    };
    for row in &table_data {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }

    let chars = match style {
        Style::Unicode => Some(&UNICODE_CHARS),
        Style::Ascii => Some(&ASCII_CHARS),
        Style::Plain => None,
    };

    let mut output = String::new();
    if let Some(chars) = chars {
        output.push_str(&render_top_border(&widths, chars));
    }
    if !no_header {
        output.push_str(&render_row(&columns, &widths, &numeric, chars, stripe, None));
        if let Some(chars) = chars {
            output.push_str(&render_separator(&widths, chars));
        }
    }
    for (i, row) in table_data.iter().enumerate() {
        output.push_str(&render_row(row, &widths, &numeric, chars, stripe, Some(i)));
    }
    if let Some(chars) = chars {
        output.push_str(&render_bottom_border(&widths, chars));
    }
    output
}

/// Returns the display width of a string, ignoring ANSI escape sequences.
fn display_width(s: &str) -> usize {
    let mut width = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if c == '\x1b' {
            in_escape = true;
        } else {
            width += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        }
    }
    width
}

fn format_value(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => format!("[{} items]", arr.len()),
        Value::Object(obj) => format!("{{{} keys}}", obj.len()),
    }
}

fn render_top_border(widths: &[usize], chars: &TableChars) -> String {
    let mut s = String::new();
    s.push_str(chars.top_left);
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&chars.horizontal.repeat(*w + 2));
        if i < widths.len() - 1 {
            s.push_str(chars.top_tee);
        }
    }
    s.push_str(chars.top_right);
    s.push('\n');
    s
}

fn render_bottom_border(widths: &[usize], chars: &TableChars) -> String {
    let mut s = String::new();
    s.push_str(chars.bottom_left);
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&chars.horizontal.repeat(*w + 2));
        if i < widths.len() - 1 {
            s.push_str(chars.bottom_tee);
        }
    }
    s.push_str(chars.bottom_right);
    s.push('\n');
    s
}

fn render_separator(widths: &[usize], chars: &TableChars) -> String {
    let mut s = String::new();
    s.push_str(chars.left_tee);
    for (i, w) in widths.iter().enumerate() {
        s.push_str(&chars.horizontal.repeat(*w + 2));
        if i < widths.len() - 1 {
            s.push_str(chars.cross);
        }
    }
    s.push_str(chars.right_tee);
    s.push('\n');
    s
}

fn render_row(cells: &[String], widths: &[usize], numeric: &[bool], chars: Option<&TableChars>, stripe: bool, row_index: Option<usize>) -> String {
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
            let rows = parse_json(input);
            assert_eq!(rows.len(), 2);
        }

        #[test]
        fn single_object() {
            let input = r#"{"a": 1, "b": 2}"#;
            let rows = parse_json(input);
            assert_eq!(rows.len(), 1);
        }

        #[test]
        fn jsonl() {
            let input = "{\"a\": 1}\n{\"a\": 2}\n{\"a\": 3}";
            let rows = parse_json(input);
            assert_eq!(rows.len(), 3);
        }

        #[test]
        fn jsonl_with_blank_lines() {
            let input = "{\"a\": 1}\n\n{\"a\": 2}\n";
            let rows = parse_json(input);
            assert_eq!(rows.len(), 2);
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
            assert_eq!(format_value(&serde_json::json!(3.14)), "3.14");
        }

        #[test]
        fn string() {
            assert_eq!(format_value(&serde_json::json!("hello")), "hello");
        }

        #[test]
        fn array() {
            assert_eq!(format_value(&serde_json::json!([1, 2, 3])), "[3 items]");
        }

        #[test]
        fn object() {
            assert_eq!(format_value(&serde_json::json!({"a": 1, "b": 2})), "{2 keys}");
        }
    }

    mod render_table {
        use super::*;
        use insta::assert_snapshot;

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
            let rows = parse_json(r#"[{"zebra": 1, "apple": 2, "mango": 3}]"#);
            let output = render_table(&rows, Style::Ascii, false, false);
            let header_line = output.lines().nth(1).unwrap();
            let headers: Vec<&str> = header_line.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
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
            assert_eq!(render_table(&rows, Style::Plain, false, true), "Alice  30\nBob     5\n");
        }

        #[test]
        fn nested_values() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"data": [1, 2, 3], "meta": {"x": 1}}),
            ];
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

        #[test]
        fn loads_from_xdg_config_home() {
            let tmp = tempfile::tempdir().unwrap();
            let config_dir = tmp.path().join("grdy");
            fs::create_dir_all(&config_dir).unwrap();
            let config_file = config_dir.join("config.json");
            fs::write(&config_file, r#"{"ascii": true, "stripe": true}"#).unwrap();

            // SAFETY: test is single-threaded; no other threads reading this env var
            unsafe { env::set_var("XDG_CONFIG_HOME", tmp.path()) };
            let path = Config::config_path().unwrap();
            assert_eq!(path, config_file);

            let config: Config =
                serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
            assert!(config.ascii);
            assert!(config.stripe);
            unsafe { env::remove_var("XDG_CONFIG_HOME") };
        }

        #[test]
        fn missing_file_returns_defaults() {
            let tmp = tempfile::tempdir().unwrap();
            // Point XDG at an empty dir with no config file
            unsafe { env::set_var("XDG_CONFIG_HOME", tmp.path()) };
            let config = Config::load();
            assert!(!config.ascii);
            assert!(!config.stripe);
            unsafe { env::remove_var("XDG_CONFIG_HOME") };
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
            unsafe { env::set_var("NO_COLOR", "1") };
            assert!(no_color());
            unsafe { env::remove_var("NO_COLOR") };
        }

        #[test]
        fn returns_true_when_empty() {
            unsafe { env::set_var("NO_COLOR", "") };
            assert!(no_color());
            unsafe { env::remove_var("NO_COLOR") };
        }

        #[test]
        fn returns_false_when_unset() {
            unsafe { env::remove_var("NO_COLOR") };
            assert!(!no_color());
        }
    }
}
