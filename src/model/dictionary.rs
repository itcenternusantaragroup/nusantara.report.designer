use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String, // String, Int32, Decimal, DateTime, Boolean
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub name: String,
    pub alias: String,
    pub columns: Vec<Column>,
    pub rows: Vec<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub alias: String,
    pub value: String,
    pub var_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dictionary {
    pub data_sources: Vec<DataSource>,
    pub variables: Vec<Variable>,
}

impl Default for Dictionary {
    fn default() -> Self {
        let mut orders_rows = Vec::new();
        let orders = [
            ("10248", "Vins et alcools Chevalier", "Queso Cabrales", "14", "21.00", "294.00", "2024-05-01"),
            ("10249", "Toms Spezialitäten", "Sir Rodney's Scones", "20", "10.00", "200.00", "2024-05-02"),
            ("10250", "Hanari Carnes", "Jack's New England Clam Chowder", "35", "9.65", "337.75", "2024-05-03"),
            ("10251", "Victuailles en stock", "Louisiana Fiery Hot Pepper Sauce", "15", "16.80", "252.00", "2024-05-04"),
            ("10252", "Suprêmes délices", "Chef Anton's Cajun Seasoning", "40", "22.00", "880.00", "2024-05-05"),
            ("10253", "Richter Supermarkt", "Grandma's Boysenberry Spread", "25", "25.00", "625.00", "2024-05-06"),
        ];

        for row in orders {
            let mut map = HashMap::new();
            map.insert("OrderID".to_string(), row.0.to_string());
            map.insert("Customer".to_string(), row.1.to_string());
            map.insert("Product".to_string(), row.2.to_string());
            map.insert("Quantity".to_string(), row.3.to_string());
            map.insert("UnitPrice".to_string(), row.4.to_string());
            map.insert("Total".to_string(), row.5.to_string());
            map.insert("OrderDate".to_string(), row.6.to_string());
            orders_rows.push(map);
        }

        let orders_source = DataSource {
            name: "Orders".to_string(),
            alias: "Sales Orders".to_string(),
            columns: vec![
                Column { name: "OrderID".to_string(), data_type: "Int32".to_string() },
                Column { name: "Customer".to_string(), data_type: "String".to_string() },
                Column { name: "Product".to_string(), data_type: "String".to_string() },
                Column { name: "Quantity".to_string(), data_type: "Int32".to_string() },
                Column { name: "UnitPrice".to_string(), data_type: "Decimal".to_string() },
                Column { name: "Total".to_string(), data_type: "Decimal".to_string() },
                Column { name: "OrderDate".to_string(), data_type: "DateTime".to_string() },
            ],
            rows: orders_rows,
        };

        let mut company_vars = Vec::new();
        company_vars.push(Variable {
            name: "CompanyName".to_string(),
            alias: "Company Name".to_string(),
            value: "Northwind Traders Inc.".to_string(),
            var_type: "String".to_string(),
        });
        company_vars.push(Variable {
            name: "ReportTitle".to_string(),
            alias: "Report Title".to_string(),
            value: "Monthly Sales Report".to_string(),
            var_type: "String".to_string(),
        });

        Self {
            data_sources: vec![orders_source],
            variables: company_vars,
        }
    }
}

impl Dictionary {
    /// Load data sources from an external JSON file (supports HRMS format and general JSON)
    pub fn load_from_json_file<P: AsRef<Path>>(path: P) -> Result<Vec<DataSource>, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read JSON file: {e}"))?;

        let val: Value = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON structure: {e}"))?;

        let file_stem = path.as_ref().file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Data")
            .to_string();

        let mut sources = Vec::new();

        match val {
            Value::Object(map) => {
                for (key, subval) in map {
                    if let Some(ds) = Self::parse_subval_to_datasource(&key, subval) {
                        sources.push(ds);
                    }
                }
            }
            Value::Array(arr) => {
                if let Some(ds) = Self::parse_array_to_datasource(&file_stem, &arr) {
                    sources.push(ds);
                }
            }
            _ => {
                return Err("Expected JSON Object or Array at root level".to_string());
            }
        }

        if sources.is_empty() {
            return Err("No tabular data sources found in JSON".to_string());
        }

        Ok(sources)
    }

    fn parse_subval_to_datasource(name: &str, val: Value) -> Option<DataSource> {
        match val {
            Value::Array(arr) => Self::parse_array_to_datasource(name, &arr),
            Value::Object(obj) => {
                let mut cols = Vec::new();
                let mut row = HashMap::new();
                for (k, v) in obj {
                    let s_val = match v {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "".to_string(),
                        _ => v.to_string(),
                    };
                    cols.push(Column { name: k.clone(), data_type: "String".to_string() });
                    row.insert(k, s_val);
                }
                Some(DataSource {
                    name: name.to_string(),
                    alias: name.to_string(),
                    columns: cols,
                    rows: vec![row],
                })
            }
            _ => None,
        }
    }

    fn parse_array_to_datasource(name: &str, arr: &[Value]) -> Option<DataSource> {
        let mut col_names = Vec::new();
        let mut rows = Vec::new();

        for item in arr {
            if let Some(obj) = item.as_object() {
                let mut row = HashMap::new();
                for (k, v) in obj {
                    if !col_names.contains(k) {
                        col_names.push(k.clone());
                    }
                    let s_val = match v {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "".to_string(),
                        _ => v.to_string(),
                    };
                    row.insert(k.clone(), s_val);
                }
                rows.push(row);
            }
        }

        if col_names.is_empty() {
            return None;
        }

        let columns = col_names.into_iter().map(|n| Column { name: n, data_type: "String".to_string() }).collect();

        Some(DataSource {
            name: name.to_string(),
            alias: name.to_string(),
            columns,
            rows,
        })
    }
}

#[cfg(test)]
mod data_file_tests {
    use super::*;

    /// Prints each data source loaded from the JSON file named by `NUSA_DATA_FILE`.
    #[test]
    fn summarise_data_file() {
        let Ok(path) = std::env::var("NUSA_DATA_FILE") else { return };
        let sources = Dictionary::load_from_json_file(&path).expect("load json");
        for ds in &sources {
            let cols: Vec<&str> = ds.columns.iter().map(|c| c.name.as_str()).collect();
            println!("DS {:<14} rows={:<4} cols={} [{}]", ds.name, ds.rows.len(), ds.columns.len(), cols.join(","));
            if let Some(first) = ds.rows.first() {
                let mut keys: Vec<_> = first.iter().take(4).map(|(k, v)| format!("{k}={v}")).collect();
                keys.sort();
                println!("    first row: {}", keys.join(" | "));
            }
        }
        assert!(!sources.is_empty());
    }
}
