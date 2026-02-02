use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{self, Read};
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
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    ascii: bool,
    #[serde(default)]
    stripe: bool,
}

impl Config {
    fn load() -> Self {
        Self::config_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("grdy").join("config.json"))
    }
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

fn main() {
    let args = Args::parse();
    let config = Config::load();

    // CLI flags override config
    let ascii = args.ascii || config.ascii;
    let stripe = args.stripe || config.stripe;

    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path).expect("Failed to read file"),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).expect("Failed to read stdin");
            buf
        }
    };

    let rows = parse_json(&input);
    let output = render_table(&rows, ascii, stripe);
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

fn render_table(rows: &[Value], ascii: bool, stripe: bool) -> String {
    if rows.is_empty() {
        return String::new();
    }

    // Extract columns from all objects (use BTreeMap for consistent ordering)
    let mut all_keys: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        if let Value::Object(obj) = row {
            for key in obj.keys() {
                let len = all_keys.len();
                all_keys.entry(key.clone()).or_insert(len);
            }
        }
    }

    let columns: Vec<String> = {
        let mut cols: Vec<(String, usize)> = all_keys.into_iter().collect();
        cols.sort_by_key(|(_, idx)| *idx);
        cols.into_iter().map(|(k, _)| k).collect()
    };

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

    // Calculate column widths
    let mut widths: Vec<usize> = columns.iter().map(|c| UnicodeWidthStr::width(c.as_str())).collect();
    for row in &table_data {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(UnicodeWidthStr::width(cell.as_str()));
        }
    }

    let chars = if ascii { &ASCII_CHARS } else { &UNICODE_CHARS };

    let mut output = String::new();
    output.push_str(&render_top_border(&widths, chars));
    output.push_str(&render_row(&columns, &widths, chars, stripe, None));
    output.push_str(&render_separator(&widths, chars));
    for (i, row) in table_data.iter().enumerate() {
        output.push_str(&render_row(row, &widths, chars, stripe, Some(i)));
    }
    output.push_str(&render_bottom_border(&widths, chars));
    output
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

fn render_row(cells: &[String], widths: &[usize], chars: &TableChars, stripe: bool, row_index: Option<usize>) -> String {
    let is_header = row_index.is_none();
    let dim = stripe && row_index.is_some_and(|i| i % 2 == 1);

    let mut s = String::new();

    if dim {
        s.push_str("\x1b[2m");
    }

    s.push_str(chars.vertical);
    for (i, cell) in cells.iter().enumerate() {
        let cell_width = UnicodeWidthStr::width(cell.as_str());
        let padding = widths[i] - cell_width;

        if is_header && stripe {
            s.push_str(&format!(" \x1b[1m{}\x1b[0m{} ", cell, " ".repeat(padding)));
        } else {
            s.push_str(&format!(" {}{} ", cell, " ".repeat(padding)));
        }
        s.push_str(chars.vertical);
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
            assert_eq!(render_table(&rows, false, false), "");
        }

        #[test]
        fn single_row() {
            let rows: Vec<Value> = vec![serde_json::json!({"name": "Alice", "age": 30})];
            assert_snapshot!(render_table(&rows, false, false));
        }

        #[test]
        fn single_row_ascii() {
            let rows: Vec<Value> = vec![serde_json::json!({"name": "Alice", "age": 30})];
            assert_snapshot!(render_table(&rows, true, false));
        }

        #[test]
        fn multiple_rows() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 25}),
                serde_json::json!({"name": "Charlie", "age": 35}),
            ];
            assert_snapshot!(render_table(&rows, false, false));
        }

        #[test]
        fn multiple_rows_with_stripe() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"name": "Alice", "age": 30}),
                serde_json::json!({"name": "Bob", "age": 25}),
                serde_json::json!({"name": "Charlie", "age": 35}),
                serde_json::json!({"name": "Diana", "age": 28}),
            ];
            assert_snapshot!(render_table(&rows, false, true));
        }

        #[test]
        fn sparse_data() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"a": 1}),
                serde_json::json!({"b": 2}),
                serde_json::json!({"a": 3, "b": 4}),
            ];
            assert_snapshot!(render_table(&rows, false, false));
        }

        #[test]
        fn nested_values() {
            let rows: Vec<Value> = vec![
                serde_json::json!({"data": [1, 2, 3], "meta": {"x": 1}}),
            ];
            assert_snapshot!(render_table(&rows, false, false));
        }
    }
}
