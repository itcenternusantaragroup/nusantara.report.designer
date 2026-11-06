//! Evaluates the expressions used in report text, for example:
//! - `{Source.Column}` and `{Variable}`, the values from the dictionary,
//! - `{Convert.ToInt32(Source.Column).ToString("#,##0")}`, conversions and formatting,
//! - `{Convert.ToDateTime(Source.Column).ToString("dd MMMM yyyy")}`, date formatting,
//! - `{Source.Column.Trim().ToUpper()}`, text methods.
//!
//! Numbers and dates are formatted with Indonesian separators (`.` for thousands, `,` for
//! decimals), matching the data in the reports. Anything that cannot be evaluated is returned
//! exactly as written, so the author can see which part is not supported.

use std::collections::HashMap;

use super::dictionary::Dictionary;

pub struct EvalContext<'a> {
    pub dictionary: &'a Dictionary,
    /// Current row index per data source name. Sources not listed use their first row.
    pub row_index: &'a HashMap<String, usize>,
    /// Variables and system values such as PageNumber and Date.
    pub vars: &'a HashMap<String, String>,
}

/// A value during evaluation.
#[derive(Debug, Clone, PartialEq)]
enum Val {
    Text(String),
    Num(f64),
    Date(DateParts),
}

impl<'a> EvalContext<'a> {
    /// Replaces every `{...}` in `text` with its value. Unknown expressions are kept as written.
    pub fn evaluate(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;

        while let Some(open) = rest.find('{') {
            let Some(close_rel) = rest[open..].find('}') else { break };
            let close = open + close_rel;
            out.push_str(&rest[..open]);

            let token = rest[open + 1..close].trim();
            match self.eval(token) {
                Some(val) => out.push_str(&to_text(&val)),
                None => out.push_str(&rest[open..=close]),
            }
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        out
    }

    /// Evaluates one expression (without the braces). `None` when it cannot be evaluated.
    fn eval(&self, input: &str) -> Option<Val> {
        let s = input.trim();
        if s.is_empty() {
            return None;
        }
        if let Some(literal) = quoted(s) {
            return Some(Val::Text(literal.to_string()));
        }

        // Whole expression in parentheses: (a + b)
        if let Some(inner) = s.strip_prefix('(') {
            if matching_close(inner) == Some(inner.len() - 1) {
                return self.eval(&inner[..inner.len() - 1]);
            }
        }

        // Number literal: 2, 0.5
        if let Ok(n) = s.parse::<f64>() {
            if s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
                return Some(Val::Num(n));
            }
        }

        // Arithmetic: a + b, a - b, a * b, a / b (numbers only)
        if let Some((left, op, right)) = split_binary(s) {
            let a = to_num(&self.eval(left)?)?;
            let b = to_num(&self.eval(right)?)?;
            let result = match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                _ => {
                    if b == 0.0 {
                        return None;
                    }
                    a / b
                }
            };
            return Some(Val::Num(result));
        }

        let (mut val, mut rest) = self.primary(s)?;
        // Method chain: .Name or .Name(args), applied left to right.
        loop {
            let Some(after_dot) = rest.trim_start().strip_prefix('.') else { break };
            let name: String = after_dot.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            if name.is_empty() {
                return None;
            }
            let after_name = &after_dot[name.len()..];
            let (args, remainder) = match after_name.strip_prefix('(') {
                Some(inner) => {
                    let close = matching_close(inner)?;
                    (&inner[..close], &inner[close + 1..])
                }
                None => ("", after_name),
            };
            val = apply_method(val, &name, args)?;
            rest = remainder;
        }

        if rest.trim().is_empty() {
            Some(val)
        } else {
            None
        }
    }

    /// The leading operand: a conversion `Convert.X(...)`, a `Source.Column`, or a variable.
    /// Returns the value and the text that follows it.
    fn primary<'s>(&self, s: &'s str) -> Option<(Val, &'s str)> {
        // Sum(Band, Source.Column): total of the column over all rows of the source.
        // The band argument is accepted but not used to pick rows.
        if let Some(args_start) = s.strip_prefix("Sum(") {
            let close = matching_close(args_start)?;
            let args = &args_start[..close];
            let last = args.rsplit(',').next()?.trim();
            let (source, column) = last.split_once('.')?;
            let total = sum_column(self.dictionary, source, column)?;
            return Some((Val::Num(total), &args_start[close + 1..]));
        }

        if let Some(rest) = s.strip_prefix("Convert.") {
            let name: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            let after = rest[name.len()..].strip_prefix('(')?;
            let close = matching_close(after)?;
            let inner = &after[..close];
            let val = self.eval(inner)?;
            return Some((convert(&name, val)?, &after[close + 1..]));
        }

        // {Line}: the line number of the current data row (1-based). Empty outside a data band.
        if s == "Line" {
            let value = self.row_index.values().next().map_or(String::new(), |i| (i + 1).to_string());
            return Some((Val::Text(value), ""));
        }

        // A whole variable name (variable names may contain characters other than letters).
        if let Some(value) = self.vars.get(s) {
            return Some((Val::Text(value.clone()), ""));
        }

        let ident: String = s.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if ident.is_empty() {
            return None;
        }
        let after_ident = &s[ident.len()..];

        if let Some(after_dot) = after_ident.strip_prefix('.') {
            let column: String = after_dot.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
            if !column.is_empty() && !after_dot[column.len()..].starts_with('(') {
                if let Some(value) = self.column(&ident, &column) {
                    return Some((Val::Text(value), &after_dot[column.len()..]));
                }
            }
        }

        let value = self.vars.get(&ident)?;
        Some((Val::Text(value.clone()), after_ident))
    }

    /// Value of `source.column` for the current row of that source.
    fn column(&self, source: &str, column: &str) -> Option<String> {
        let ds = self.dictionary.data_sources.iter().find(|d| d.name == source)?;
        let index = self.row_index.get(source).copied().unwrap_or(0);
        let row = ds.rows.get(index);
        // The schema can lag behind the data (loaded JSON adds columns), so accept either.
        let known = ds.columns.iter().any(|c| c.name == column) || row.map_or(false, |r| r.contains_key(column));
        if !known {
            return None;
        }
        Some(row.and_then(|r| r.get(column)).cloned().unwrap_or_default())
    }
}

/// Total of a numeric column over all rows of a data source.
fn sum_column(dictionary: &Dictionary, source: &str, column: &str) -> Option<f64> {
    let ds = dictionary.data_sources.iter().find(|d| d.name == source)?;
    Some(
        ds.rows
            .iter()
            .filter_map(|row| row.get(column).and_then(|v| parse_number(v)))
            .sum(),
    )
}

/// Splits `left op right` at the last top-level `+` or `-` (or `*` or `/` when there is none).
/// Operators inside parentheses or quotes are ignored, and a leading sign is not an operator.
fn split_binary(s: &str) -> Option<(&str, char, &str)> {
    let mut depth = 0usize;
    let mut in_quote = false;
    let mut additive: Option<usize> = None;
    let mut multiplicative: Option<usize> = None;
    let mut prev_significant: Option<char> = None;

    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            _ if in_quote => {}
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            '+' | '-' if depth == 0 && prev_significant.map_or(false, |p| !"+-*/(".contains(p)) => {
                additive = Some(i);
            }
            '*' | '/' if depth == 0 && prev_significant.map_or(false, |p| !"+-*/(".contains(p)) => {
                multiplicative = Some(i);
            }
            _ => {}
        }
        if !c.is_whitespace() {
            prev_significant = Some(c);
        }
    }

    let (index, op) = match (additive, multiplicative) {
        (Some(i), _) => (i, s[i..].chars().next()?),
        (None, Some(i)) => (i, s[i..].chars().next()?),
        _ => return None,
    };
    Some((s[..index].trim(), op, s[index + 1..].trim()))
}

/// Converts a value with `Convert.<name>(...)`.
fn convert(name: &str, val: Val) -> Option<Val> {
    match name {
        "ToInt16" | "ToInt32" | "ToInt64" | "ToByte" => Some(Val::Num(to_num(&val)?.round())),
        "ToDouble" | "ToDecimal" | "ToSingle" => Some(Val::Num(to_num(&val)?)),
        "ToDateTime" => Some(Val::Date(to_date(&val)?)),
        "ToString" => Some(Val::Text(to_text(&val))),
        _ => None,
    }
}

/// Applies a method such as `ToString("N2")`, `Trim()`, `ToUpper()` or `ToLower()`.
fn apply_method(val: Val, name: &str, args: &str) -> Option<Val> {
    let format = quoted(args.trim());
    match name {
        "ToString" => Some(Val::Text(match (&val, format) {
            (Val::Num(n), Some(f)) => format_number(*n, f),
            (Val::Num(n), None) => plain_number(*n),
            (Val::Date(d), Some(f)) => format_date(d, f),
            (Val::Date(d), None) => format_date(d, "dd/MM/yyyy"),
            (Val::Text(t), _) => t.clone(),
        })),
        "Trim" => Some(Val::Text(to_text(&val).trim().to_string())),
        "ToUpper" => Some(Val::Text(to_text(&val).to_uppercase())),
        "ToLower" => Some(Val::Text(to_text(&val).to_lowercase())),
        _ => None,
    }
}

fn to_text(val: &Val) -> String {
    match val {
        Val::Text(t) => t.clone(),
        Val::Num(n) => plain_number(*n),
        Val::Date(d) => format_date(d, "dd/MM/yyyy"),
    }
}

fn to_num(val: &Val) -> Option<f64> {
    match val {
        Val::Num(n) => Some(*n),
        Val::Text(t) => parse_number(t),
        Val::Date(_) => None,
    }
}

fn to_date(val: &Val) -> Option<DateParts> {
    match val {
        Val::Date(d) => Some(*d),
        Val::Text(t) => parse_date(t),
        Val::Num(_) => None,
    }
}

/// Index of the `)` that closes an already-opened `(`. Quoted text is skipped.
fn matching_close(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_quote = false;
    for (i, c) in s.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            _ if in_quote => {}
            '(' => depth += 1,
            ')' if depth == 0 => return Some(i),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// The text inside a double-quoted literal, if `s` is exactly one.
fn quoted(s: &str) -> Option<&str> {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') && !s[1..s.len() - 1].contains('"') {
        Some(&s[1..s.len() - 1])
    } else {
        None
    }
}

/// Reads a number written with Indonesian (`37.979.600,50`) or plain (`37979600.50`) separators.
pub fn parse_number(s: &str) -> Option<f64> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return None;
    }
    let normalized = match (t.contains('.'), t.contains(',')) {
        (true, true) => {
            if t.rfind(',') > t.rfind('.') {
                t.replace('.', "").replace(',', ".")
            } else {
                t.replace(',', "")
            }
        }
        (false, true) => {
            // A single comma followed by one or two digits is a decimal separator.
            let last = t.rsplit(',').next().unwrap_or("");
            if t.matches(',').count() == 1 && (1..=2).contains(&last.len()) {
                t.replace(',', ".")
            } else {
                t.replace(',', "")
            }
        }
        (true, false) => {
            if is_thousands_dots(&t) {
                t.replace('.', "")
            } else {
                t
            }
        }
        (false, false) => t,
    };
    normalized.parse::<f64>().ok()
}

/// `1.234.567` style: groups of three digits after the first group.
fn is_thousands_dots(t: &str) -> bool {
    let body = t.trim_start_matches('-');
    let groups: Vec<&str> = body.split('.').collect();
    let digits = |g: &str| !g.is_empty() && g.chars().all(|c| c.is_ascii_digit());
    groups.len() > 1
        && groups[0].len() <= 3
        && digits(groups[0])
        && groups[1..].iter().all(|g| g.len() == 3 && digits(g))
}

/// A number without formatting, with a comma as decimal separator.
fn plain_number(n: f64) -> String {
    // -0.0 compares equal to 0.0; use the positive zero so it never prints as "-0".
    let n = if n == 0.0 { 0.0 } else { n };
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{n:.0}")
    } else {
        format!("{n}").replace('.', ",")
    }
}

/// Formats a number with a .NET-style format: `N2`, `F0`, `#,##0`, `0.00`, and so on.
/// Output uses Indonesian separators.
pub fn format_number(n: f64, format: &str) -> String {
    let f = format.trim();
    let (decimals, grouping) = match f.chars().next() {
        Some(c) if "NnFf".contains(c) => {
            let digits = f[1..].parse::<usize>().unwrap_or(2);
            (digits, c == 'N' || c == 'n')
        }
        _ => {
            let decimals = f
                .split('.')
                .nth(1)
                .map_or(0, |p| p.chars().filter(|c| *c == '0' || *c == '#').count());
            (decimals, f.contains(','))
        }
    };

    let fixed = format!("{:.*}", decimals, n.abs());
    let (int_part, frac_part) = fixed.split_once('.').unwrap_or((fixed.as_str(), ""));
    let int_text = if grouping { group_thousands(int_part) } else { int_part.to_string() };
    let body = if decimals > 0 { format!("{int_text},{frac_part}") } else { int_text };

    let is_zero = body.chars().all(|c| c == '0' || c == ',' || c == '.');
    if n < 0.0 && !is_zero {
        format!("-{body}")
    } else {
        body
    }
}

fn group_thousands(digits: &str) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
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

/// Formats a date with .NET-style tokens (yyyy, MMMM, dd, HH ...). Month names are in Indonesian.
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
        row.insert("porsi".to_string(), "3983".to_string());
        row.insert("nominal".to_string(), "37.979.600".to_string());
        row.insert("rate".to_string(), "2.25".to_string());
        Dictionary {
            data_sources: vec![DataSource {
                name: "master".into(),
                alias: "master".into(),
                columns: ["nodok", "tgl", "porsi", "nominal", "rate"]
                    .iter()
                    .map(|c| Column { name: c.to_string(), data_type: "System.String".into() })
                    .collect(),
                rows: vec![row],
            }],
            variables: Vec::new(),
            relations: Vec::new(),
        }
    }

    fn render(text: &str, vars: &HashMap<String, String>) -> String {
        let dictionary = dictionary();
        let row_index = HashMap::new();
        EvalContext { dictionary: &dictionary, row_index: &row_index, vars }.evaluate(text)
    }

    #[test]
    fn resolves_source_columns_and_variables() {
        let mut vars = HashMap::new();
        vars.insert("PageNumber".to_string(), "2".to_string());
        assert_eq!(render("No {master.nodok} hal {PageNumber}", &vars), "No DOK-1 hal 2");
        assert_eq!(render("{master.unknown} {Convert.ToDateTime(x)}", &vars), "{master.unknown} {Convert.ToDateTime(x)}");
    }

    #[test]
    fn formats_dates_from_data() {
        let vars = HashMap::new();
        assert_eq!(
            render("Periode : {Convert.ToDateTime(master.tgl).ToString(\"dd MMMM yyyy\")}", &vars),
            "Periode : 21 Juli 2025"
        );
        assert_eq!(render("{Convert.ToDateTime(master.tgl).ToString(\"dd/MM/yy\")}", &vars), "21/07/25");
    }

    #[test]
    fn converts_and_formats_numbers() {
        let vars = HashMap::new();
        assert_eq!(render("{Convert.ToInt32(master.porsi).ToString(\"#,##0\")}", &vars), "3.983");
        assert_eq!(render("{Convert.ToInt64(master.nominal)}", &vars), "37979600");
        assert_eq!(render("{Convert.ToDecimal(master.nominal).ToString(\"N2\")}", &vars), "37.979.600,00");
        assert_eq!(render("{Convert.ToDouble(master.rate).ToString(\"0.00\")}", &vars), "2,25");
        assert_eq!(render("{Convert.ToDouble(master.rate).ToString(\"F0\")}", &vars), "2");
    }

    #[test]
    fn text_methods_and_strings() {
        let vars = HashMap::new();
        assert_eq!(render("{master.nodok.ToLower()}", &vars), "dok-1");
        assert_eq!(render("{Convert.ToString(master.nodok).Trim().ToUpper()}", &vars), "DOK-1");
        assert_eq!(render("{Convert.ToString(master.porsi)}", &vars), "3983");
    }

    #[test]
    fn arithmetic_and_sum() {
        let vars = HashMap::new();
        assert_eq!(render("{Convert.ToInt32(master.porsi) + Convert.ToInt32(master.porsi) * 2}", &vars), "11949");
        assert_eq!(render("{(Convert.ToDouble(master.rate) + 1) * 2}", &vars), "6,5");
        assert_eq!(render("{Convert.ToInt32(master.porsi) / 0}", &vars), "{Convert.ToInt32(master.porsi) / 0}");
        // Rounding -0.4 gives -0.0, which must print as "0".
        assert_eq!(render("{Convert.ToInt32(-0.4)}", &vars), "0");

        let mut dictionary = dictionary();
        let mut a = HashMap::new();
        a.insert("total".to_string(), "10".to_string());
        let mut b = HashMap::new();
        b.insert("total".to_string(), "5.000".to_string());
        dictionary.data_sources.push(DataSource {
            name: "detail".into(),
            alias: "detail".into(),
            columns: vec![Column { name: "total".into(), data_type: "System.String".into() }],
            rows: vec![a, b],
        });
        let row_index = HashMap::new();
        let ctx = EvalContext { dictionary: &dictionary, row_index: &row_index, vars: &vars };
        assert_eq!(ctx.evaluate("{Sum(DataBand14,detail.total)}"), "5010");
        // 5010 * (2.25 / 100) = 112.725
        assert_eq!(ctx.evaluate("{Sum(DataBand14,detail.total)*(Convert.ToDouble(master.rate)/100)}"), "112,725");
    }

    #[test]
    fn parses_indonesian_numbers() {
        assert_eq!(parse_number("37.979.600"), Some(37_979_600.0));
        assert_eq!(parse_number("2,25"), Some(2.25));
        assert_eq!(parse_number("2.25"), Some(2.25));
        assert_eq!(parse_number("1.234,50"), Some(1234.5));
        assert_eq!(parse_number("-1.500"), Some(-1500.0));
        assert_eq!(parse_number("abc"), None);
    }
}
