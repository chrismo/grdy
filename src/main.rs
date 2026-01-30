use clap::Parser;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{self, Read};
use unicode_width::UnicodeWidthStr;

#[derive(Parser)]
#[command(name = "grdy", about = "Render JSON data as beautiful tables")]
struct Args {
    /// Input file (reads from stdin if not provided)
    file: Option<String>,

    /// Use ASCII characters instead of Unicode box-drawing
    #[arg(short, long)]
    compact: bool,

    /// Disable colored output
    #[arg(short, long)]
    no_color: bool,
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

    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path).expect("Failed to read file"),
        None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf).expect("Failed to read stdin");
            buf
        }
    };

    let value: Value = serde_json::from_str(&input).expect("Invalid JSON");

    let rows = match value {
        Value::Array(arr) => arr,
        obj @ Value::Object(_) => vec![obj],
        _ => {
            eprintln!("Expected JSON array or object");
            std::process::exit(1);
        }
    };

    if rows.is_empty() {
        return;
    }

    // Extract columns from all objects (use BTreeMap for consistent ordering)
    let mut all_keys: BTreeMap<String, usize> = BTreeMap::new();
    for row in &rows {
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
        return;
    }

    // Build table data
    let mut table_data: Vec<Vec<String>> = Vec::new();
    for row in &rows {
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

    let chars = if args.compact { &ASCII_CHARS } else { &UNICODE_CHARS };

    // Print table
    print_top_border(&widths, chars);
    print_row(&columns, &widths, chars, args.no_color, true);
    print_separator(&widths, chars);
    for row in &table_data {
        print_row(row, &widths, chars, args.no_color, false);
    }
    print_bottom_border(&widths, chars);
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

fn print_top_border(widths: &[usize], chars: &TableChars) {
    print!("{}", chars.top_left);
    for (i, w) in widths.iter().enumerate() {
        print!("{}", chars.horizontal.repeat(*w + 2));
        if i < widths.len() - 1 {
            print!("{}", chars.top_tee);
        }
    }
    println!("{}", chars.top_right);
}

fn print_bottom_border(widths: &[usize], chars: &TableChars) {
    print!("{}", chars.bottom_left);
    for (i, w) in widths.iter().enumerate() {
        print!("{}", chars.horizontal.repeat(*w + 2));
        if i < widths.len() - 1 {
            print!("{}", chars.bottom_tee);
        }
    }
    println!("{}", chars.bottom_right);
}

fn print_separator(widths: &[usize], chars: &TableChars) {
    print!("{}", chars.left_tee);
    for (i, w) in widths.iter().enumerate() {
        print!("{}", chars.horizontal.repeat(*w + 2));
        if i < widths.len() - 1 {
            print!("{}", chars.cross);
        }
    }
    println!("{}", chars.right_tee);
}

fn print_row(cells: &[String], widths: &[usize], chars: &TableChars, no_color: bool, is_header: bool) {
    print!("{}", chars.vertical);
    for (i, cell) in cells.iter().enumerate() {
        let cell_width = UnicodeWidthStr::width(cell.as_str());
        let padding = widths[i] - cell_width;

        if is_header && !no_color {
            print!(" \x1b[1m{}\x1b[0m{} ", cell, " ".repeat(padding));
        } else {
            print!(" {}{} ", cell, " ".repeat(padding));
        }
        print!("{}", chars.vertical);
    }
    println!();
}
