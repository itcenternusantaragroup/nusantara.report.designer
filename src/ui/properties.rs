use eframe::egui::{self, Color32};
use crate::model::*;
use super::designer::ReportDesignerApp;
use super::theme;

pub struct PropertiesPanel;

impl PropertiesPanel {
    pub fn show(app: &mut ReportDesignerApp, ctx: &egui::Context) {
        egui::SidePanel::right("properties_panel")
            .resizable(true)
            .default_width(260.0)
            .min_width(220.0)
            .max_width(380.0)
            .frame(egui::Frame::side_top_panel(&ctx.style())
                .fill(theme::PANEL)
                .inner_margin(egui::Margin::symmetric(10.0, 10.0)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Properties").size(14.0).strong().color(theme::INK));
                ui.add_space(4.0);
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if app.selected_component_id.is_some() {
                        Self::show_component_properties(app, ui);
                    } else if app.selected_band_id.is_some() {
                        Self::show_band_properties(app, ui);
                    } else {
                        Self::show_page_properties(app, ui);
                    }
                });
            });
    }

    fn show_component_properties(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        let comp_id = app.selected_component_id.clone().unwrap();
        let (mut changed, mut new_bind_expr) = (false, None);

        let ds_columns: Vec<(String, String)> = app.report.dictionary.data_sources.iter()
            .flat_map(|ds| ds.columns.iter().map(move |col| (ds.name.clone(), col.name.clone())))
            .collect();

        {
            let comp = match app.current_page_mut().components.iter_mut().find(|c| c.id == comp_id) {
                Some(c) => c,
                None => return,
            };

            ui.label(egui::RichText::new(format!("Component: {} ({})", comp.name, comp.comp_type.display_name())).strong());
            ui.label(egui::RichText::new(format!("MRT Ident: {}", comp.comp_type.sti_ident())).size(11.0).color(Color32::GRAY));
            ui.separator();

            // Section 1: Identification
            egui::CollapsingHeader::new("Identification").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    if ui.text_edit_singleline(&mut comp.name).changed() {
                        changed = true;
                    }
                });
            });

            // Section 2: Text / Expression
            if comp.comp_type == ComponentType::Text || comp.comp_type == ComponentType::Barcode {
                egui::CollapsingHeader::new("Value & Expression").default_open(true).show(ui, |ui| {
                    if comp.comp_type == ComponentType::Text && comp.allow_html_tags {
                        ui.label("Text (HTML):");
                        if super::html_editor::show(ui, comp) {
                            changed = true;
                        }
                    } else {
                        ui.label("Text / Expression:");
                        if ui.text_edit_multiline(&mut comp.text).changed() {
                            changed = true;
                        }
                    }

                    // Quick Bind to Data Field dropdown
                    ui.horizontal(|ui| {
                        ui.label("Bind field:");
                        egui::ComboBox::from_id_salt("bind_field_combo")
                            .selected_text("Select Field...")
                            .show_ui(ui, |ui| {
                                for (ds_name, col_name) in &ds_columns {
                                    let expr = format!("{{{ds_name}.{col_name}}}");
                                    if ui.button(&expr).clicked() {
                                        new_bind_expr = Some(expr);
                                    }
                                }
                            });
                    });
                });
            }

            // Section 3: Layout & Geometry (mm)
            egui::CollapsingHeader::new("Layout (mm)").default_open(true).show(ui, |ui| {
                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut comp.rect.x).speed(0.5).suffix(" mm")).changed() {
                            changed = true;
                        }
                    });
                    cols[1].horizontal(|ui| {
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut comp.rect.y).speed(0.5).suffix(" mm")).changed() {
                            changed = true;
                        }
                    });
                });

                ui.columns(2, |cols| {
                    cols[0].horizontal(|ui| {
                        ui.label("Width:");
                        if ui.add(egui::DragValue::new(&mut comp.rect.w).speed(0.5).range(1.0..=500.0).suffix(" mm")).changed() {
                            changed = true;
                        }
                    });
                    cols[1].horizontal(|ui| {
                        ui.label("Height:");
                        if ui.add(egui::DragValue::new(&mut comp.rect.h).speed(0.5).range(1.0..=500.0).suffix(" mm")).changed() {
                            changed = true;
                        }
                    });
                });
            });

            // Section 4: Typography
            if comp.comp_type == ComponentType::Text {
                egui::CollapsingHeader::new("Typography").default_open(true).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Font:");
                        egui::ComboBox::from_id_salt("prop_font_combo")
                            .selected_text(&comp.font_family)
                            .show_ui(ui, |ui| {
                                for f in ["Arial", "Segoe UI", "Times New Roman", "Courier New", "Verdana"] {
                                    if ui.selectable_label(comp.font_family == f, f).clicked() {
                                        comp.font_family = f.to_string();
                                        changed = true;
                                    }
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Size:");
                        if ui.add(egui::DragValue::new(&mut comp.font_size).speed(0.5).range(6.0..=72.0).suffix(" pt")).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut comp.bold, "Bold").changed() { changed = true; }
                        if ui.checkbox(&mut comp.italic, "Italic").changed() { changed = true; }
                        if ui.checkbox(&mut comp.underline, "Underline").changed() { changed = true; }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Align H:");
                        if ui.selectable_label(comp.align_h == AlignH::Left, "Left").clicked() {
                            comp.align_h = AlignH::Left;
                            changed = true;
                        }
                        if ui.selectable_label(comp.align_h == AlignH::Center, "Center").clicked() {
                            comp.align_h = AlignH::Center;
                            changed = true;
                        }
                        if ui.selectable_label(comp.align_h == AlignH::Right, "Right").clicked() {
                            comp.align_h = AlignH::Right;
                            changed = true;
                        }
                    });

                    if ui.checkbox(&mut comp.word_wrap, "Word Wrap").changed() {
                        changed = true;
                    }
                    if ui
                        .checkbox(&mut comp.allow_html_tags, "Allow HTML tags")
                        .on_hover_text("Read the text as HTML: <b>, <i>, <u>, <font>, <span>, <br>")
                        .changed()
                    {
                        changed = true;
                    }
                });
            }

            // Conditional formatting: rules that recolour the component when its value matches.
            egui::CollapsingHeader::new("Conditional formatting").id_salt("cond_fmt").default_open(false).show(ui, |ui| {
                let mut remove: Option<usize> = None;
                for (i, rule) in comp.conditions.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(("cond_op", i))
                            .selected_text(rule.op.symbol())
                            .width(70.0)
                            .show_ui(ui, |ui| {
                                for op in CondOp::ALL {
                                    if ui.selectable_value(&mut rule.op, op, op.symbol()).changed() {
                                        changed = true;
                                    }
                                }
                            });
                        if ui.add(egui::TextEdit::singleline(&mut rule.value).desired_width(80.0)).changed() {
                            changed = true;
                        }
                        let mut bg = Color32::from_rgba_premultiplied(rule.bg_color[0], rule.bg_color[1], rule.bg_color[2], rule.bg_color[3]);
                        if ui.color_edit_button_srgba(&mut bg).on_hover_text("Fill colour").changed() {
                            rule.bg_color = [bg.r(), bg.g(), bg.b(), bg.a()];
                            changed = true;
                        }
                        let mut tc = Color32::from_rgba_premultiplied(rule.text_color[0], rule.text_color[1], rule.text_color[2], rule.text_color[3]);
                        if ui.color_edit_button_srgba(&mut tc).on_hover_text("Text colour").changed() {
                            rule.text_color = [tc.r(), tc.g(), tc.b(), tc.a()];
                            changed = true;
                        }
                        if ui.small_button("X").on_hover_text("Remove rule").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    comp.conditions.remove(i);
                    changed = true;
                }
                if ui.button("Add rule").clicked() {
                    comp.conditions.push(CondRule {
                        op: CondOp::Greater,
                        value: String::new(),
                        bg_color: [255, 235, 156, 255],
                        text_color: [156, 0, 6, 255],
                    });
                    changed = true;
                }
                ui.label(theme::caption("The first matching rule sets the colours. Numbers compare as numbers."));
            });
            // Section 5: Appearance & Colors
            egui::CollapsingHeader::new("Appearance").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut tc = Color32::from_rgba_premultiplied(
                        comp.text_color[0], comp.text_color[1], comp.text_color[2], comp.text_color[3]
                    );
                    ui.label("Text Color:");
                    if ui.color_edit_button_srgba(&mut tc).changed() {
                        comp.text_color = [tc.r(), tc.g(), tc.b(), tc.a()];
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    let mut bg = Color32::from_rgba_premultiplied(
                        comp.bg_color[0], comp.bg_color[1], comp.bg_color[2], comp.bg_color[3]
                    );
                    ui.label("Fill Color:");
                    if ui.color_edit_button_srgba(&mut bg).changed() {
                        comp.bg_color = [bg.r(), bg.g(), bg.b(), bg.a()];
                        changed = true;
                    }
                    if ui.button("Clear").clicked() {
                        comp.bg_color = [0, 0, 0, 0];
                        changed = true;
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    let mut bc = Color32::from_rgba_premultiplied(
                        comp.border_color[0], comp.border_color[1], comp.border_color[2], comp.border_color[3]
                    );
                    ui.label("Border Color:");
                    if ui.color_edit_button_srgba(&mut bc).changed() {
                        comp.border_color = [bc.r(), bc.g(), bc.b(), bc.a()];
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Border Width:");
                    if ui.add(egui::DragValue::new(&mut comp.border_width).speed(0.1).range(0.1..=10.0).suffix(" mm")).changed() {
                        changed = true;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Borders:");
                    if ui.checkbox(&mut comp.border_sides.left, "Left").changed() { changed = true; }
                    if ui.checkbox(&mut comp.border_sides.top, "Top").changed() { changed = true; }
                    if ui.checkbox(&mut comp.border_sides.right, "Right").changed() { changed = true; }
                    if ui.checkbox(&mut comp.border_sides.bottom, "Bottom").changed() { changed = true; }
                });

                ui.horizontal(|ui| {
                    if ui.button("All Borders").clicked() {
                        comp.border_sides = BorderSides::all();
                        changed = true;
                    }
                    if ui.button("No Borders").clicked() {
                        comp.border_sides = BorderSides::none();
                        changed = true;
                    }
                });
            });

            // Section 6: Specific Component Settings
            if comp.comp_type == ComponentType::Barcode {
                egui::CollapsingHeader::new("Barcode Settings").default_open(true).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Format:");
                        egui::ComboBox::from_id_salt("bc_format_combo")
                            .selected_text(&comp.barcode_format)
                            .show_ui(ui, |ui| {
                                for fmt in ["Code128", "EAN13", "QR Code", "Code39", "PDF417"] {
                                    if ui.selectable_label(comp.barcode_format == fmt, fmt).clicked() {
                                        comp.barcode_format = fmt.to_string();
                                        changed = true;
                                    }
                                }
                            });
                    });
                });
            }

            if comp.comp_type == ComponentType::Shape {
                egui::CollapsingHeader::new("Shape Settings").default_open(true).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Shape Kind:");
                        if ui.selectable_label(comp.shape_kind == ShapeKind::Rectangle, "Rectangle").clicked() {
                            comp.shape_kind = ShapeKind::Rectangle;
                            changed = true;
                        }
                        if ui.selectable_label(comp.shape_kind == ShapeKind::RoundedRectangle, "Rounded").clicked() {
                            comp.shape_kind = ShapeKind::RoundedRectangle;
                            changed = true;
                        }
                        if ui.selectable_label(comp.shape_kind == ShapeKind::Ellipse, "Ellipse").clicked() {
                            comp.shape_kind = ShapeKind::Ellipse;
                            changed = true;
                        }
                    });
                });
            }

            ui.add_space(8.0);
            if ui.button("Delete component").clicked() {
                app.delete_selected();
                return;
            }
        }

        if let Some(expr) = new_bind_expr {
            if let Some(comp) = app.current_page_mut().components.iter_mut().find(|c| c.id == comp_id) {
                comp.text = expr;
                changed = true;
            }
        }

        if changed {
            app.refresh_code_buffer();
        }
    }

    fn show_band_properties(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        let band_id = app.selected_band_id.clone().unwrap();
        let mut changed = false;

        let ds_names: Vec<String> = app.report.dictionary.data_sources.iter().map(|d| d.name.clone()).collect();
        let rel_names: Vec<String> = app.report.dictionary.relations.iter().map(|r| r.name.clone()).collect();

        {
            let band = match app.current_page_mut().bands.iter_mut().find(|b| b.id == band_id) {
                Some(b) => b,
                None => return,
            };

            ui.label(egui::RichText::new(format!("Band: {} ({})", band.name, band.band_type.display_name())).strong());
            ui.label(egui::RichText::new(format!("MRT Ident: {}", band.band_type.sti_ident())).size(11.0).color(Color32::GRAY));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui.text_edit_singleline(&mut band.name).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Top Position (mm):");
                if ui.add(egui::DragValue::new(&mut band.top).speed(0.5).suffix(" mm")).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Height (mm):");
                if ui.add(egui::DragValue::new(&mut band.height).speed(0.5).range(2.0..=300.0).suffix(" mm")).changed() {
                    changed = true;
                }
            });

                        if band.band_type == BandType::GroupHeader {
                ui.horizontal(|ui| {
                    ui.label("Group by:");
                    let mut group = band.group_by.clone().unwrap_or_default();
                    if ui
                        .add(egui::TextEdit::singleline(&mut group).hint_text("Source.Column").desired_width(150.0))
                        .on_hover_text("Rows with the same value form one group, e.g. detail.tgl_kantin")
                        .changed()
                    {
                        let group = group.trim().to_string();
                        band.group_by = if group.is_empty() { None } else { Some(group) };
                        changed = true;
                    }
                });
                if ui
                    .checkbox(&mut band.keep_together, "Keep whole group on one sheet")
                    .on_hover_text("When the group fits on a sheet, move it to the next sheet instead of splitting it. The header always stays with its first row.")
                    .changed()
                {
                    changed = true;
                }
            }
            if band.band_type == BandType::DataBand || band.band_type == BandType::Header || band.band_type == BandType::Footer {
                ui.horizontal(|ui| {
                    ui.label("Data Source:");
                    let current_ds = band.data_source.clone().unwrap_or_else(|| "None".to_string());
                    egui::ComboBox::from_id_salt("band_ds_combo")
                        .selected_text(&current_ds)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(band.data_source.is_none(), "None").clicked() {
                                band.data_source = None;
                                changed = true;
                            }
                            for ds_name in &ds_names {
                                if ui.selectable_label(band.data_source.as_deref() == Some(ds_name), ds_name).clicked() {
                                    band.data_source = Some(ds_name.clone());
                                    changed = true;
                                }
                            }
                        });
                });
            }

            ui.add_space(8.0);
            if band.band_type == BandType::DataBand && !rel_names.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Detail of:");
                    let current = band.detail_of.clone().unwrap_or_else(|| "None".to_string());
                    egui::ComboBox::from_id_salt("band_detail_combo").selected_text(&current).show_ui(ui, |ui| {
                        if ui.selectable_label(band.detail_of.is_none(), "None").clicked() {
                            band.detail_of = None;
                            changed = true;
                        }
                        for name in &rel_names {
                            if ui.selectable_label(band.detail_of.as_deref() == Some(name), name).clicked() {
                                band.detail_of = Some(name.clone());
                                changed = true;
                            }
                        }
                    });
                })
                .response
                .on_hover_text("Print this band once for every child row of the relation, under its parent row.");
            }
            if ui.button("Delete band").clicked() {
                app.delete_selected();
                return;
            }
        }

        if changed {
            app.refresh_code_buffer();
        }
    }

    fn show_page_properties(app: &mut ReportDesignerApp, ui: &mut egui::Ui) {
        let mut changed = false;
        {
            let page = app.current_page_mut();

            ui.label(egui::RichText::new(format!("Page: {}", page.name)).strong());
            ui.label(egui::RichText::new("MRT Ident: StiPage").size(11.0).color(Color32::GRAY));
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Name:");
                if ui.text_edit_singleline(&mut page.name).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Orientation:");
                if ui.selectable_label(page.orientation == Orientation::Portrait, "Portrait").clicked() {
                    page.orientation = Orientation::Portrait;
                    changed = true;
                }
                if ui.selectable_label(page.orientation == Orientation::Landscape, "Landscape").clicked() {
                    page.orientation = Orientation::Landscape;
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Page Width:");
                if ui.add(egui::DragValue::new(&mut page.width).speed(1.0).range(50.0..=1000.0).suffix(" mm")).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label("Page Height:");
                if ui.add(egui::DragValue::new(&mut page.height).speed(1.0).range(50.0..=1000.0).suffix(" mm")).changed() {
                    changed = true;
                }
            });

            ui.separator();
            ui.label(egui::RichText::new("Margins (mm)").strong());

            ui.columns(2, |cols| {
                cols[0].horizontal(|ui| {
                    ui.label("Left:");
                    if ui.add(egui::DragValue::new(&mut page.margins.left).speed(0.5).range(0.0..=50.0).suffix(" mm")).changed() {
                        changed = true;
                    }
                });
                cols[1].horizontal(|ui| {
                    ui.label("Right:");
                    if ui.add(egui::DragValue::new(&mut page.margins.right).speed(0.5).range(0.0..=50.0).suffix(" mm")).changed() {
                        changed = true;
                    }
                });
            });

            ui.columns(2, |cols| {
                cols[0].horizontal(|ui| {
                    ui.label("Top:");
                    if ui.add(egui::DragValue::new(&mut page.margins.top).speed(0.5).range(0.0..=50.0).suffix(" mm")).changed() {
                        changed = true;
                    }
                });
                cols[1].horizontal(|ui| {
                    ui.label("Bottom:");
                    if ui.add(egui::DragValue::new(&mut page.margins.bottom).speed(0.5).range(0.0..=50.0).suffix(" mm")).changed() {
                        changed = true;
                    }
                });
            });
        }

        if changed {
            app.refresh_code_buffer();
        }
    }
}
