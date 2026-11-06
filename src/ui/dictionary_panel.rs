//! The Data tab: edit data sources, their columns and the report variables.

use eframe::egui::{self, Id, TextEdit};

use crate::model::*;
use super::designer::ReportDesignerApp;
use super::theme;

pub fn show(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
    if ui
        .button("Load JSON...")
        .on_hover_text("Load a JSON file. Sources with the same name are replaced.")
        .clicked()
    {
        app.load_json_datasource_dialog();
    }

    ui.add_space(8.0);
    add_source_row(app, ui);

    ui.add_space(10.0);
    ui.label(theme::heading("Data sources"));
    if app.report.dictionary.data_sources.is_empty() {
        ui.label(theme::caption("No data sources yet. Load a JSON file or add a source above."));
    }

    let mut changed = false;
    let mut remove_source: Option<usize> = None;
    let mut remove_column: Option<(usize, usize)> = None;
    let mut add_column: Option<(usize, String)> = None;
    let mut insert_field: Option<String> = None;

    for (si, ds) in app.report.dictionary.data_sources.iter_mut().enumerate() {
        let header = format!("{} ({} rows)", ds.name, ds.rows.len());
        egui::CollapsingHeader::new(header)
            .id_salt(("ds", si))
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    changed |= ui.add(TextEdit::singleline(&mut ds.name).desired_width(140.0)).changed();
                    if ui.small_button("Delete source").clicked() {
                        remove_source = Some(si);
                    }
                });

                for (ci, col) in ds.columns.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        changed |= ui.add(TextEdit::singleline(&mut col.name).desired_width(130.0)).changed();
                        if ui.small_button("Add").on_hover_text("Place this field on the page").clicked() {
                            insert_field = Some(format!("{{{}.{}}}", ds.name, col.name));
                        }
                        if ui.small_button("Remove").clicked() {
                            remove_column = Some((si, ci));
                        }
                    });
                }

                let key = Id::new(("new_column", si));
                let mut name = ui.data(|d| d.get_temp::<String>(key)).unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.add(TextEdit::singleline(&mut name).hint_text("column").desired_width(130.0));
                    if ui.small_button("Add column").clicked() && !name.trim().is_empty() {
                        add_column = Some((si, name.trim().to_string()));
                        name.clear();
                    }
                });
                ui.data_mut(|d| d.insert_temp(key, name));
            });
    }

    // Variables
    ui.add_space(10.0);
    ui.label(theme::heading("Variables"));
    let mut remove_variable: Option<usize> = None;
    for (vi, var) in app.report.dictionary.variables.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            changed |= ui.add(TextEdit::singleline(&mut var.name).desired_width(110.0)).changed();
            changed |= ui
                .add(TextEdit::singleline(&mut var.value).hint_text("value").desired_width(90.0))
                .changed();
            if ui.small_button("Add").on_hover_text("Place this variable on the page").clicked() {
                insert_field = Some(format!("{{{}}}", var.name));
            }
            if ui.small_button("Remove").clicked() {
                remove_variable = Some(vi);
            }
        });
    }
    add_variable_row(app, ui);

    // Apply the actions collected above.
    if let Some(si) = remove_source {
        app.report.dictionary.data_sources.remove(si);
        changed = true;
    }
    if let Some((si, ci)) = remove_column {
        if let Some(ds) = app.report.dictionary.data_sources.get_mut(si) {
            ds.columns.remove(ci);
            changed = true;
        }
    }
    if let Some((si, name)) = add_column {
        if let Some(ds) = app.report.dictionary.data_sources.get_mut(si) {
            ds.columns.push(Column { name, data_type: "System.String".into() });
            changed = true;
        }
    }
    if let Some(vi) = remove_variable {
        app.report.dictionary.variables.remove(vi);
        changed = true;
    }
    if let Some(expr) = insert_field {
        insert_on_page(app, expr);
    }
    if changed {
        app.refresh_code_buffer();
    }
}

/// Row to create a new data source (name typed by the user).
fn add_source_row(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
    let key = Id::new("new_source_name");
    let mut name = ui.data(|d| d.get_temp::<String>(key)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut name).hint_text("source name").desired_width(140.0));
        if ui.button("Add source").clicked() && !name.trim().is_empty() {
            let source = name.trim().to_string();
            name.clear();
            app.report.dictionary.data_sources.push(DataSource {
                alias: source.clone(),
                name: source,
                columns: Vec::new(),
                rows: Vec::new(),
            });
            app.refresh_code_buffer();
        }
    });
    ui.data_mut(|d| d.insert_temp(key, name));
}

/// Row to create a new variable.
fn add_variable_row(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
    let key = Id::new("new_variable_name");
    let mut name = ui.data(|d| d.get_temp::<String>(key)).unwrap_or_default();
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(&mut name).hint_text("variable name").desired_width(140.0));
        if ui.button("Add variable").clicked() && !name.trim().is_empty() {
            let variable = name.trim().to_string();
            name.clear();
            app.report.dictionary.variables.push(Variable {
                alias: variable.clone(),
                name: variable,
                value: String::new(),
                var_type: "System.String".into(),
            });
            app.refresh_code_buffer();
        }
    });
    ui.data_mut(|d| d.insert_temp(key, name));
}

/// Puts a text component with `expr` on the current page, and selects it.
fn insert_on_page(app: &mut ReportDesignerApp, expr: String) {
    app.push_undo();
    let mut comp = Component::default();
    comp.name = format!("Field_{}", app.current_page().components.len() + 1);
    comp.text = expr;
    comp.rect = RectMm::new(20.0, 50.0, 45.0, 8.0);
    comp.border_sides = BorderSides::all();
    let new_id = comp.id.clone();
    app.current_page_mut().components.push(comp);
    app.selected_component_id = Some(new_id);
    app.status_message = "Added field to the page.".to_string();
    app.refresh_code_buffer();
}
