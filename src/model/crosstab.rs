//! Cross-tabs: a pivot table built from a data source.
//!
//! A cross-tab component takes the distinct values of one column as rows and of another column as
//! columns, and fills each cell with an aggregate (sum, count or average) of a value column. Row and
//! column totals and a grand total are added. The result is a grid of ordinary text cells, so the
//! preview, the PDF and the HTML export draw it like any other text.

use std::collections::HashMap;

use super::dictionary::Dictionary;
use super::expr::{format_number, parse_number};
use super::report::{BorderSides, CrossTab, CrossAgg, AlignH, Component, ComponentType, RectMm};

/// Height of a cell when the cross-tab is too short for its rows, in mm.
const MIN_CELL_H: f32 = 4.0;

#[derive(Debug, Clone, Copy, Default)]
struct Acc {
    sum: f64,
    rows: usize,
    numeric: usize,
}

impl Acc {
    fn add(&mut self, value: Option<f64>) {
        self.rows += 1;
        if let Some(v) = value {
            self.sum += v;
            self.numeric += 1;
        }
    }

    /// The aggregate, or None when there is nothing to show (an empty cell).
    fn value(&self, agg: CrossAgg) -> Option<f64> {
        match agg {
            CrossAgg::Sum => (self.rows > 0).then_some(self.sum),
            CrossAgg::Count => (self.rows > 0).then_some(self.rows as f64),
            CrossAgg::Average => (self.numeric > 0).then(|| self.sum / self.numeric as f64),
        }
    }
}

/// Builds the grid of cells for the cross-tab component `comp`. Returns cells placed on the
/// component's rectangle; the caller prints them in place of the component.
pub fn cells(comp: &Component, ct: &CrossTab, dict: &Dictionary) -> Vec<Component> {
    let Some(ds) = dict.data_sources.iter().find(|d| d.name == ct.source) else {
        return vec![cell(comp.rect, &format!("Cross-tab: data source '{}' not found", ct.source), false, false)];
    };
    let key = |row: &HashMap<String, String>, col: &str| row.get(col).map(|v| v.trim().to_string()).unwrap_or_default();

    let mut row_keys: Vec<String> = ds.rows.iter().map(|r| key(r, &ct.row_column)).collect();
    row_keys.sort();
    row_keys.dedup();
    let mut col_keys: Vec<String> = ds.rows.iter().map(|r| key(r, &ct.column_column)).collect();
    col_keys.sort();
    col_keys.dedup();
    let row_index: HashMap<&str, usize> = row_keys.iter().enumerate().map(|(i, k)| (k.as_str(), i)).collect();
    let col_index: HashMap<&str, usize> = col_keys.iter().enumerate().map(|(i, k)| (k.as_str(), i)).collect();

    let (nr, nc) = (row_keys.len(), col_keys.len());
    // cells[r][c] for the body; the extra last row and column hold the totals.
    let mut body = vec![vec![Acc::default(); nc + 1]; nr + 1];
    for row in &ds.rows {
        let r = row_index[key(row, &ct.row_column).as_str()];
        let c = col_index[key(row, &ct.column_column).as_str()];
        let value = row.get(&ct.value_column).and_then(|v| parse_number(v)).map(|v| v);
        for (rr, cc) in [(r, c), (r, nc), (nr, c), (nr, nc)] {
            body[rr][cc].add(value);
        }
    }

    let total_cols = nc + 2;
    let total_rows = nr + 2;
    let cw = comp.rect.w / total_cols as f32;
    let ch = (comp.rect.h / total_rows as f32).max(MIN_CELL_H);
    let number = |v: Option<f64>| match v {
        Some(v) if ct.aggregate == CrossAgg::Count => format_number(v, "N0"),
        Some(v) => format_number(v, "N2"),
        None => String::new(),
    };

    let mut out = Vec::new();
    // Corner: what the grid means.
    out.push(cell_at(comp.rect, cw, ch, 0, 0, &format!("{} / {}", ct.row_column, ct.column_column), true, false));
    // Column headers and the total column header.
    for (c, name) in col_keys.iter().enumerate() {
        out.push(cell_at(comp.rect, cw, ch, c + 1, 0, name, true, false));
    }
    out.push(cell_at(comp.rect, cw, ch, nc + 1, 0, "Total", true, false));
    // Body rows: row header, values, row total.
    for (r, name) in row_keys.iter().enumerate() {
        out.push(cell_at(comp.rect, cw, ch, 0, r + 1, name, true, false));
        for c in 0..=nc {
            let is_total_col = c == nc;
            let text = number(body[r][c].value(ct.aggregate));
            out.push(cell_at(comp.rect, cw, ch, c + 1, r + 1, &text, false, is_total_col));
        }
    }
    // Total row.
    out.push(cell_at(comp.rect, cw, ch, 0, nr + 1, "Total", true, false));
    for c in 0..=nc {
        let text = number(body[nr][c].value(ct.aggregate));
        out.push(cell_at(comp.rect, cw, ch, c + 1, nr + 1, &text, true, c == nc));
    }
    out
}


fn cell_at(outer: RectMm, cw: f32, ch: f32, col: usize, row: usize, text: &str, header: bool, total: bool) -> Component {
    let rect = RectMm::new(outer.x + col as f32 * cw, outer.y + row as f32 * ch, cw, ch);
    cell(rect, text, header || total, total)
}

/// One text cell: thin borders, a light fill for headers, right-aligned numbers.
fn cell(rect: RectMm, text: &str, bold: bool, shaded: bool) -> Component {
    let mut c = Component::default();
    c.comp_type = ComponentType::Text;
    c.name = "CrossCell".to_string();
    c.rect = rect;
    c.text = text.to_string();
    c.font_size = 8.0;
    c.bold = bold;
    c.word_wrap = false;
    c.border_sides = BorderSides::all();
    c.border_color = [180, 180, 180, 255];
    c.border_width = 0.2;
    c.align_h = if text.chars().next().map_or(false, |ch| ch.is_ascii_digit() || ch == '-') { AlignH::Right } else { AlignH::Left };
    if shaded {
        c.bg_color = [238, 241, 246, 255];
    }
    c
}

/// The cross-tab defaults for a new component: the first source and its first three columns.
pub fn default_for(dict: &Dictionary) -> Option<CrossTab> {
    let ds = dict.data_sources.first()?;
    let col = |i: usize| ds.columns.get(i).map(|c| c.name.clone()).unwrap_or_default();
    Some(CrossTab {
        source: ds.name.clone(),
        row_column: col(0),
        column_column: col(1),
        value_column: col(2),
        aggregate: CrossAgg::Sum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Column, DataSource};

    fn row(region: &str, month: &str, amount: &str) -> HashMap<String, String> {
        HashMap::from([
            ("region".to_string(), region.to_string()),
            ("month".to_string(), month.to_string()),
            ("amount".to_string(), amount.to_string()),
        ])
    }

    fn dict(rows: Vec<HashMap<String, String>>) -> Dictionary {
        let col = |n: &str| Column { name: n.into(), data_type: "System.String".into() };
        Dictionary {
            data_sources: vec![DataSource {
                name: "sales".into(),
                alias: "sales".into(),
                columns: vec![col("region"), col("month"), col("amount")],
                rows,
            }],
            variables: Vec::new(),
            relations: Vec::new(),
        }
    }

    fn texts(cells: &[Component], width: usize) -> Vec<Vec<String>> {
        cells
            .chunks(width)
            .map(|line| line.iter().map(|c| c.text.clone()).collect())
            .collect()
    }

    #[test]
    fn pivots_sums_with_totals() {
        let d = dict(vec![row("N", "Jan", "10"), row("N", "Feb", "5"), row("S", "Jan", "3")]);
        let ct = CrossTab { source: "sales".into(), row_column: "region".into(), column_column: "month".into(), value_column: "amount".into(), aggregate: CrossAgg::Sum };
        let comp = Component { rect: RectMm::new(10.0, 10.0, 120.0, 30.0), ..Component::default() };
        let cells = cells(&comp, &ct, &d);
        // Columns: row label + Feb/Jan (sorted) + Total = 4; rows: header + N, S + Total = 4.
        let grid = texts(&cells, 4);
        assert_eq!(grid[0], vec!["region / month", "Feb", "Jan", "Total"]);
        assert_eq!(grid[1], vec!["N", "5,00", "10,00", "15,00"]);
        assert_eq!(grid[2], vec!["S", "", "3,00", "3,00"]);
        assert_eq!(grid[3], vec!["Total", "5,00", "13,00", "18,00"]);
    }

    #[test]
    fn counts_rows_and_averages_numbers() {
        let d = dict(vec![row("N", "Jan", "10"), row("N", "Jan", "x")]);
        let mut ct = CrossTab { source: "sales".into(), row_column: "region".into(), column_column: "month".into(), value_column: "amount".into(), aggregate: CrossAgg::Count };
        let comp = Component { rect: RectMm::new(0.0, 0.0, 60.0, 20.0), ..Component::default() };
        assert_eq!(texts(&cells(&comp, &ct, &d), 3)[1][1], "2");
        ct.aggregate = CrossAgg::Average;
        assert_eq!(texts(&cells(&comp, &ct, &d), 3)[1][1], "10,00", "text values are not averaged");
    }

    #[test]
    fn missing_source_shows_a_message() {
        let d = dict(Vec::new());
        let ct = CrossTab { source: "nope".into(), row_column: "a".into(), column_column: "b".into(), value_column: "c".into(), aggregate: CrossAgg::Sum };
        let comp = Component::default();
        assert!(cells(&comp, &ct, &d)[0].text.contains("not found"));
    }
}
