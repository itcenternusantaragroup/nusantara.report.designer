//! Evaluates the placeholders used in report text:
//! - `{Source.Column}` and `{Variable}` (values from the dictionary),
//! - `{Convert.ToDateTime(Source.Column).ToString("dd MMMM yyyy")}` (date formatting).
//!
//! Anything else (other function calls, arithmetic) is left as written, so the author can see
//! which part is not supported yet.

use std::collections::HashMap;

use super::dictionary::Dictionary;

pub struct EvalContext<'a> {
    pub dictionary: &'a Dictionary,
    /// Current row index per data source name. Sources not listed use their first row.
    pub row_index: &'a HashMap<String, usize>,
    /// Variables and system values such as PageNumber and Date.
    pub vars: &'a HashMap<String, String>,
}

impl<'a> EvalContext<'a> {
    pub fn evaluate(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;

        while let Some(open) = rest.find('{') {
            let Some(close_rel) = rest[open..].find('}') else { break };
            let close = open + close_rel;
            out.push_str(&rest[..open]);

            let token = rest[open + 1..close].trim();
            match self.resolve(token) {
                Some(value) => out.push_str(&value),
                None => out.push_str(&rest[open..=close]),
            }
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        out
    }

    fn resolve(&self, token: &str) -> Option<String> {
        if let Some(date) = self.resolve_date(token) {
            return Some(date);
        }
        self.lookup(token)
    }

    /// `Source.Column` or a variable name, without any function call.
    fn lookup(&self, token: &str) -> Option<String> {
        if token.is_empty() || token.contains('(') {
            return None;
        }
        if let Some((source, column)) = token.split_once('.') {
            let ds = self.dictionary.data_sources.iter().find(|d| d.name == source)?;
            let index = self.row_index.get(source).copied().unwrap_or(0);
            let row = ds.rows.get(index);
            // The schema can lag behind the data (loaded JSON adds columns), so accept either.
            let known = ds.columns.iter().any(|c| c.name == column) || row.map_or(false, |r| r.contains_key(column));
            if !known {
                return None;
            }
            return Some(row.and_then(|r| r.get(column)).cloned().unwrap_or_default());
        }
        self.vars.get(token).cloned()
    }

    /// `Convert.ToDateTime(<value>).ToString("<format>")`, or without the `.ToString` part.
    fn resolve_date(&self, token: &str) -> Option<String> {
        let rest = token.strip_prefix("Convert.ToDateTime(")?;
        let close = rest.find(')')?;
        let inner = rest[..close].trim();
        let tail = rest[close + 1..].trim();

        let raw = self.lookup(inner)?;
        let date = parse_date(&raw)?;
        let format = match tail.strip_prefix(".ToString(") {
            Some(f) => f.trim_end_matches(')').trim().trim_matches('"').to_string(),
            None => "dd/MM/yyyy".to_string(),
        };
        Some(format_date(&date, &format))
    }
}

/// A date and time read from report data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DateParts {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Reads `YYYY-MM-DD[ HH:mm[:ss]]` and `DD-MM-YYYY` / `DD/MM/YYYY`.
pub fn parse_date(s: &str) -> Option<DateParts> {
    let s = s.trim();
    let b = s.as_bytes();
    let num = |from: usize, to: usize| -> Option<u32> { s.get(from..to)?.parse::<u32>().ok() };

    let (year, month, day, time_from) = if b.len() >= 10 && b[4] == b'-' {
        (s.get(0..4)?.parse::<i32>().ok()?, num(5, 7)?, num(8, 10)?, 10)
    } else if b.len() >= 10 && (b[2] == b'-' || b[2] == b'/') {
        (s.get(6..10)?.parse::<i32>().ok()?, num(3, 5)?, num(0, 2)?, 10)
    } else {
        return None;
    };

    let rest = s.get(time_from..).unwrap_or("").trim();
    let (hour, minute, second) = if rest.len() >= 5 {
        let h = rest.get(0..2)?.parse::<u32>().ok()?;
        let m = rest.get(3..5)?.parse::<u32>().ok()?;
        let sec = rest.get(6..8).and_then(|x| x.parse::<u32>().ok()).unwrap_or(0);
        (h, m, sec)
    } else {
        (0, 0, 0)
    };
    Some(DateParts { year, month, day, hour, minute, second })
}

const MONTHS: [&str; 12] = [
    "Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September", "Oktober", "November", "Desember",
];

/// Formats with .NET-style tokens (yyyy, MMMM, dd, HH ...). Month names are in Indonesian.
pub fn format_date(d: &DateParts, format: &str) -> String {
    const TOKENS: [&str; 11] = ["yyyy", "yy", "MMMM", "MMM", "MM", "M", "dd", "d", "HH", "mm", "ss"];
    let chars: Vec<char> = format.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    'outer: while i < chars.len() {
        for tok in TOKENS {
            let len = tok.len();
            if i + len <= chars.len() && chars[i..i + len].iter().copied().eq(tok.chars()) {
                out.push_str(&token_value(d, tok));
                i += len;
                continue 'outer;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn token_value(d: &DateParts, tok: &str) -> String {
    let month_name = MONTHS[(d.month.clamp(1, 12) - 1) as usize];
    match tok {
        "yyyy" => format!("{:04}", d.year),
        "yy" => format!("{:02}", d.year.rem_euclid(100)),
        "MMMM" => month_name.to_string(),
        "MMM" => month_name.chars().take(3).collect(),
        "MM" => format!("{:02}", d.month),
        "M" => d.month.to_string(),
        "dd" => format!("{:02}", d.day),
        "d" => d.day.to_string(),
        "HH" => format!("{:02}", d.hour),
        "mm" => format!("{:02}", d.minute),
        "ss" => format!("{:02}", d.second),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dictionary::{Column, DataSource};

    fn dictionary() -> Dictionary {
        let mut row = HashMap::new();
        row.insert("nodok".to_string(), "DOK-1".to_string());
        row.insert("tgl".to_string(), "2025-07-21".to_string());
        Dictionary {
            data_sources: vec![DataSource {
                name: "master".into(),
                alias: "master".into(),
                columns: vec![
                    Column { name: "nodok".into(), data_type: "System.String".into() },
                    Column { name: "tgl".into(), data_type: "System.String".into() },
                ],
                rows: vec![row],
            }],
            variables: Vec::new(),
        }
    }

    #[test]
    fn resolves_source_columns_and_variables() {
        let dictionary = dictionary();
        let row_index = HashMap::new();
        let mut vars = HashMap::new();
        vars.insert("PageNumber".to_string(), "2".to_string());
        let ctx = EvalContext { dictionary: &dictionary, row_index: &row_index, vars: &vars };

        assert_eq!(ctx.evaluate("No {master.nodok} hal {PageNumber}"), "No DOK-1 hal 2");
        assert_eq!(ctx.evaluate("{master.unknown} {Convert.ToDateTime(x)}"), "{master.unknown} {Convert.ToDateTime(x)}");
    }

    #[test]
    fn formats_dates_from_data() {
        let dictionary = dictionary();
        let row_index = HashMap::new();
        let vars = HashMap::new();
        let ctx = EvalContext { dictionary: &dictionary, row_index: &row_index, vars: &vars };

        assert_eq!(
            ctx.evaluate("Periode : {Convert.ToDateTime(master.tgl).ToString(\"dd MMMM yyyy\")}"),
            "Periode : 21 Juli 2025"
        );
        assert_eq!(ctx.evaluate("{Convert.ToDateTime(master.tgl).ToString(\"dd/MM/yy\")}"), "21/07/25");
    }
}
