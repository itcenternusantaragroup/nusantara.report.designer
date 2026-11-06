use std::path::PathBuf;
use eframe::egui::{self, Key};

use crate::model::*;
use super::canvas::CanvasView;
use super::code_view::CodeView;
use super::dialogs::{NewReportDialog, PageSetupDialog};
use super::preview::PreviewView;
use super::properties::PropertiesPanel;
use super::ribbon::RibbonBar;
use super::theme;
use super::toolbox::ToolboxPanel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Design,
    Preview,
    CodeView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonTab {
    Home,
    Insert,
    PageLayout,
    View,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveTool {
    Pointer,
    InsertComponent(ComponentType),
    InsertBand(BandType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

#[derive(Debug, Clone)]
pub enum DragState {
    None,
    Moving {
        comp_id: String,
        initial_rect: RectMm,
        mouse_start_mm: (f32, f32),
    },
    Resizing {
        comp_id: String,
        handle: ResizeHandle,
        initial_rect: RectMm,
        mouse_start_mm: (f32, f32),
    },
    ResizingBand {
        band_id: String,
        initial_height: f32,
        mouse_start_y_mm: f32,
    },
}

pub struct ReportDesignerApp {
    pub report: Report,
    pub current_page_idx: usize,
    pub selected_component_id: Option<String>,
    pub selected_band_id: Option<String>,
    pub active_tool: ActiveTool,
    pub active_ribbon_tab: RibbonTab,
    pub zoom: f32, // 1.0 = 100%
    /// When true the canvas picks a zoom that shows the whole page.
    pub fit_to_window: bool,
    pub snap_to_grid: bool,
    pub grid_size_mm: f32,
    pub show_grid: bool,
    pub show_rulers: bool,
    pub app_mode: AppMode,
    pub status_message: String,

    pub undo_stack: Vec<Report>,
    pub redo_stack: Vec<Report>,

    pub drag_state: DragState,
    pub clipboard_component: Option<Component>,

    // Dialog states
    pub show_new_report_dialog: bool,
    pub show_page_setup: bool,
    pub show_about: bool,

    // Code view buffer
    pub code_buffer: String,
    pub code_is_dirty: bool,

    // Automation / Testing
    pub screenshot_path: Option<String>,
    pub frame_count: u32,
}

impl ReportDesignerApp {
    pub fn new(screenshot_path: Option<String>, open_path: Option<String>) -> Self {
        let (report, status) = match open_path {
            Some(path) => match MrtHandler::load_from_file(&path) {
                Ok(r) => {
                    let status = format!("Opened {path}");
                    (r, status)
                }
                Err(e) => (Report::blank(), format!("Could not open {path}: {e}")),
            },
            None => (Report::blank(), "Nusantara Report Designer ready.".to_string()),
        };
        let code_buffer = serde_json::to_string_pretty(
            &MrtHandler::to_stimulsoft_json(&report).unwrap_or_default()
        ).unwrap_or_default();

        Self {
            report,
            current_page_idx: 0,
            selected_component_id: None,
            selected_band_id: None,
            active_tool: ActiveTool::Pointer,
            active_ribbon_tab: RibbonTab::Home,
            zoom: 1.0,
            fit_to_window: true,
            snap_to_grid: true,
            grid_size_mm: 2.5,
            show_grid: true,
            show_rulers: true,
            app_mode: AppMode::Design,
            status_message: status,

            undo_stack: Vec::new(),
            redo_stack: Vec::new(),

            drag_state: DragState::None,
            clipboard_component: None,

            show_new_report_dialog: false,
            show_page_setup: false,
            show_about: false,

            code_buffer,
            code_is_dirty: false,

            screenshot_path,
            frame_count: 0,
        }
    }

    pub fn push_undo(&mut self) {
        self.undo_stack.push(self.report.clone());
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.report.clone());
            self.report = prev;
            self.selected_component_id = None;
            self.selected_band_id = None;
            self.status_message = "Undo performed.".to_string();
            self.refresh_code_buffer();
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.report.clone());
            self.report = next;
            self.selected_component_id = None;
            self.selected_band_id = None;
            self.status_message = "Redo performed.".to_string();
            self.refresh_code_buffer();
        }
    }

    pub fn current_page(&self) -> &Page {
        &self.report.pages[self.current_page_idx.min(self.report.pages.len().saturating_sub(1))]
    }

    pub fn current_page_mut(&mut self) -> &mut Page {
        let idx = self.current_page_idx.min(self.report.pages.len().saturating_sub(1));
        &mut self.report.pages[idx]
    }

    pub fn selected_component(&self) -> Option<&Component> {
        let sel_id = self.selected_component_id.as_ref()?;
        self.current_page().components.iter().find(|c| &c.id == sel_id)
    }

    pub fn selected_component_mut(&mut self) -> Option<&mut Component> {
        let sel_id = self.selected_component_id.clone()?;
        self.current_page_mut().components.iter_mut().find(|c| c.id == sel_id)
    }

    pub fn selected_band(&self) -> Option<&Band> {
        let sel_id = self.selected_band_id.as_ref()?;
        self.current_page().bands.iter().find(|b| &b.id == sel_id)
    }

    pub fn selected_band_mut(&mut self) -> Option<&mut Band> {
        let sel_id = self.selected_band_id.clone()?;
        self.current_page_mut().bands.iter_mut().find(|b| b.id == sel_id)
    }

    pub fn create_blank_report(&mut self, landscape: bool) {
        self.push_undo();
        let mut report = Report {
            guid: uuid::Uuid::new_v4().to_string(),
            name: "NewReport".to_string(),
            alias: "New Report".to_string(),
            version: "2020.2.3".to_string(),
            file_path: None,
            pages: Vec::new(),
            dictionary: self.report.dictionary.clone(),
        };

        let mut page = Page::default();
        if landscape {
            page.orientation = Orientation::Landscape;
        }
        report.pages.push(page);

        self.report = report;
        self.current_page_idx = 0;
        self.selected_component_id = None;
        self.selected_band_id = None;
        self.status_message = "Created blank report page.".to_string();
        self.refresh_code_buffer();
    }

    pub fn create_template_report(&mut self, template_kind: &str) {
        self.push_undo();
        match template_kind {
            "letter" => {
                // HRMS Letter template (like assignment_letter.mrt / surat_jalan.mrt)
                let mut report = Report {
                    guid: uuid::Uuid::new_v4().to_string(),
                    name: "AssignmentLetter".to_string(),
                    alias: "Surat Tugas".to_string(),
                    version: "2020.2.3".to_string(),
                    file_path: None,
                    pages: Vec::new(),
                    dictionary: self.report.dictionary.clone(),
                };

                let mut page = Page::default();
                let title_band = Band {
                    id: "band_letter_head".to_string(),
                    name: "ReportTitleBand1".to_string(),
                    band_type: BandType::ReportTitle,
                    top: 10.0,
                    height: 30.0,
                    data_source: None,
                };

                let comp_org = Component {
                    name: "TxtOrg".to_string(),
                    text: "PT. NUSA BOARD INDONESIA".to_string(),
                    rect: RectMm::new(10.0, 10.0, 190.0, 8.0),
                    font_size: 14.0,
                    bold: true,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_head".to_string()),
                    ..Default::default()
                };

                let comp_title = Component {
                    name: "TxtTitle".to_string(),
                    text: "SURAT TUGAS / ASSIGNMENT LETTER".to_string(),
                    rect: RectMm::new(10.0, 20.0, 190.0, 8.0),
                    font_size: 12.0,
                    bold: true,
                    underline: true,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_head".to_string()),
                    ..Default::default()
                };

                let comp_num = Component {
                    name: "TxtNumber".to_string(),
                    text: "Nomor: {employee.nodok}".to_string(),
                    rect: RectMm::new(10.0, 28.0, 190.0, 6.0),
                    font_size: 9.5,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_head".to_string()),
                    ..Default::default()
                };

                let body_band = Band {
                    id: "band_letter_body".to_string(),
                    name: "DataBand1".to_string(),
                    band_type: BandType::DataBand,
                    top: 45.0,
                    height: 50.0,
                    data_source: Some("employee".to_string()),
                };

                let comp_body1 = Component {
                    name: "TxtBody1".to_string(),
                    text: "Yang bertanda tangan di bawah ini menerangkan bahwa karyawan berikut:".to_string(),
                    rect: RectMm::new(10.0, 46.0, 190.0, 6.0),
                    font_size: 10.0,
                    parent_band_id: Some("band_letter_body".to_string()),
                    ..Default::default()
                };

                let comp_name = Component {
                    name: "TxtEmpName".to_string(),
                    text: "Nama Lengkap: {employee.nmlengkap}".to_string(),
                    rect: RectMm::new(15.0, 54.0, 180.0, 6.0),
                    font_size: 10.0,
                    bold: true,
                    parent_band_id: Some("band_letter_body".to_string()),
                    ..Default::default()
                };

                let comp_nik = Component {
                    name: "TxtEmpNik".to_string(),
                    text: "NIK: {employee.nik}".to_string(),
                    rect: RectMm::new(15.0, 61.0, 180.0, 6.0),
                    font_size: 10.0,
                    parent_band_id: Some("band_letter_body".to_string()),
                    ..Default::default()
                };

                let comp_dept = Component {
                    name: "TxtEmpDept".to_string(),
                    text: "Departemen / Jabatan: {employee.nmdept} - {employee.nmjabatan}".to_string(),
                    rect: RectMm::new(15.0, 68.0, 180.0, 6.0),
                    font_size: 10.0,
                    parent_band_id: Some("band_letter_body".to_string()),
                    ..Default::default()
                };

                let comp_body2 = Component {
                    name: "TxtBody2".to_string(),
                    text: "Diberikan tugas kerja terhitung sejak tanggal: {employee.tgl_kerja1}.".to_string(),
                    rect: RectMm::new(10.0, 78.0, 190.0, 6.0),
                    font_size: 10.0,
                    parent_band_id: Some("band_letter_body".to_string()),
                    ..Default::default()
                };

                let footer_band = Band {
                    id: "band_letter_sign".to_string(),
                    name: "FooterBand1".to_string(),
                    band_type: BandType::Footer,
                    top: 105.0,
                    height: 35.0,
                    data_source: None,
                };

                let comp_sign1 = Component {
                    name: "TxtSignLabel".to_string(),
                    text: "Penerima Tugas,".to_string(),
                    rect: RectMm::new(15.0, 106.0, 60.0, 6.0),
                    font_size: 10.0,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_sign".to_string()),
                    ..Default::default()
                };

                let comp_sign2 = Component {
                    name: "TxtSignLeader".to_string(),
                    text: "Mengetahui Atasan,".to_string(),
                    rect: RectMm::new(135.0, 106.0, 60.0, 6.0),
                    font_size: 10.0,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_sign".to_string()),
                    ..Default::default()
                };

                let comp_emp_sign = Component {
                    name: "TxtSignEmpName".to_string(),
                    text: "({employee.nmlengkap})".to_string(),
                    rect: RectMm::new(15.0, 130.0, 60.0, 6.0),
                    font_size: 10.0,
                    bold: true,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_sign".to_string()),
                    ..Default::default()
                };

                let comp_lead_sign = Component {
                    name: "TxtSignLeadName".to_string(),
                    text: "({employee.nmatasan1})".to_string(),
                    rect: RectMm::new(135.0, 130.0, 60.0, 6.0),
                    font_size: 10.0,
                    bold: true,
                    align_h: AlignH::Center,
                    parent_band_id: Some("band_letter_sign".to_string()),
                    ..Default::default()
                };

                page.bands = vec![title_band, body_band, footer_band];
                page.components = vec![
                    comp_org, comp_title, comp_num,
                    comp_body1, comp_name, comp_nik, comp_dept, comp_body2,
                    comp_sign1, comp_sign2, comp_emp_sign, comp_lead_sign,
                ];

                report.pages.push(page);
                self.report = report;
                self.status_message = "Created Official Document / Letter report.".to_string();
            }
            _ => {
                // Default Sales Report
                self.report = Report::default();
                self.status_message = "Created Standard Sales Report.".to_string();
            }
        }

        self.current_page_idx = 0;
        self.selected_component_id = None;
        self.selected_band_id = None;
        self.refresh_code_buffer();
    }

    pub fn delete_selected(&mut self) {
        if let Some(sel_id) = self.selected_component_id.take() {
            self.push_undo();
            self.current_page_mut().components.retain(|c| c.id != sel_id);
            self.status_message = "Component deleted.".to_string();
            self.refresh_code_buffer();
        } else if let Some(band_id) = self.selected_band_id.take() {
            self.push_undo();
            let page = self.current_page_mut();
            page.bands.retain(|b| b.id != band_id);
            for c in &mut page.components {
                if c.parent_band_id.as_deref() == Some(&band_id) {
                    c.parent_band_id = None;
                }
            }
            self.status_message = "Band deleted.".to_string();
            self.refresh_code_buffer();
        }
    }

    pub fn copy_selected(&mut self) {
        let comp_clone = self.selected_component().cloned();
        if let Some(comp) = comp_clone {
            self.status_message = format!("Copied '{}' to clipboard.", comp.name);
            self.clipboard_component = Some(comp);
        }
    }

    pub fn paste_selected(&mut self) {
        if let Some(mut comp) = self.clipboard_component.clone() {
            self.push_undo();
            comp.id = uuid::Uuid::new_v4().to_string();
            comp.name = format!("{}_copy", comp.name);
            comp.rect.x += 5.0;
            comp.rect.y += 5.0;
            let id = comp.id.clone();
            self.current_page_mut().components.push(comp);
            self.selected_component_id = Some(id);
            self.selected_band_id = None;
            self.status_message = "Component pasted.".to_string();
            self.refresh_code_buffer();
        }
    }

    pub fn duplicate_selected(&mut self) {
        self.copy_selected();
        self.paste_selected();
    }

    pub fn refresh_code_buffer(&mut self) {
        if let Ok(v) = MrtHandler::to_stimulsoft_json(&self.report) {
            if let Ok(s) = serde_json::to_string_pretty(&v) {
                self.code_buffer = s;
                self.code_is_dirty = false;
            }
        }
    }

    pub fn apply_code_buffer(&mut self) -> Result<(), String> {
        let new_report = MrtHandler::from_mrt_content(&self.code_buffer)?;
        self.push_undo();
        self.report = new_report;
        self.selected_component_id = None;
        self.selected_band_id = None;
        self.status_message = "Applied changes from MRT definition.".to_string();
        self.code_is_dirty = false;
        Ok(())
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Report file (*.mrt)", &["mrt"])
            .add_filter("JSON File (*.json)", &["json"])
            .add_filter("All Files (*.*)", &["*"])
            .pick_file()
        {
            match MrtHandler::load_from_file(&path) {
                Ok(loaded) => {
                    self.push_undo();
                    self.report = loaded;
                    self.current_page_idx = 0;
                    self.selected_component_id = None;
                    self.selected_band_id = None;
                    self.status_message = format!("Loaded report from {}", path.display());
                    self.refresh_code_buffer();
                }
                Err(e) => {
                    self.status_message = format!("Error loading file: {e}");
                }
            }
        }
    }

    pub fn save_file_dialog(&mut self) {
        let existing = self.report.file_path.clone();
        let target_path = if let Some(p) = existing {
            Some(PathBuf::from(p))
        } else {
            rfd::FileDialog::new()
                .add_filter("Report file (*.mrt)", &["mrt"])
                .set_file_name("report.mrt")
                .save_file()
        };

        if let Some(path) = target_path {
            match MrtHandler::save_to_file(&self.report, &path) {
                Ok(_) => {
                    self.report.file_path = Some(path.to_string_lossy().to_string());
                    self.status_message = format!("Saved report to {}", path.display());
                }
                Err(e) => {
                    self.status_message = format!("Error saving file: {e}");
                }
            }
        }
    }

    pub fn save_as_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Report file (*.mrt)", &["mrt"])
            .set_file_name(self.report.file_path.as_deref().unwrap_or("report.mrt"))
            .save_file()
        {
            match MrtHandler::save_to_file(&self.report, &path) {
                Ok(_) => {
                    self.report.file_path = Some(path.to_string_lossy().to_string());
                    self.status_message = format!("Saved report to {}", path.display());
                }
                Err(e) => {
                    self.status_message = format!("Error saving file: {e}");
                }
            }
        }
    }

    pub fn load_json_datasource_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("JSON Data File (*.json)", &["json"])
            .set_title("Select JSON Data Source")
            .pick_file()
        {
            match Dictionary::load_from_json_file(&path) {
                Ok(sources) => {
                    self.push_undo();
                    let count = sources.len();
                    self.report.dictionary.data_sources.extend(sources);
                    self.status_message = format!("Loaded {count} data sources from {}", path.file_name().unwrap_or_default().to_string_lossy());
                }
                Err(e) => {
                    self.status_message = format!("Error loading JSON: {e}");
                }
            }
        }
    }

    fn handle_global_keyboard(&mut self, ctx: &egui::Context) {
        let (z_pressed, y_pressed, c_pressed, v_pressed, d_pressed, s_pressed, o_pressed, n_pressed, del_pressed) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command;
            (
                ctrl && i.key_pressed(Key::Z),
                ctrl && i.key_pressed(Key::Y),
                ctrl && i.key_pressed(Key::C),
                ctrl && i.key_pressed(Key::V),
                ctrl && i.key_pressed(Key::D),
                ctrl && i.key_pressed(Key::S),
                ctrl && i.key_pressed(Key::O),
                ctrl && i.key_pressed(Key::N),
                i.key_pressed(Key::Delete),
            )
        });

        if z_pressed {
            self.undo();
        } else if y_pressed {
            self.redo();
        } else if c_pressed {
            self.copy_selected();
        } else if v_pressed {
            self.paste_selected();
        } else if d_pressed {
            self.duplicate_selected();
        } else if s_pressed {
            self.save_file_dialog();
        } else if o_pressed {
            self.open_file_dialog();
        } else if n_pressed {
            self.show_new_report_dialog = true;
        } else if del_pressed {
            self.delete_selected();
        }
    }
}

impl eframe::App for ReportDesignerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;

        if self.screenshot_path.is_some() {
            ctx.request_repaint();
        }

        if self.frame_count == 120 && self.screenshot_path.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }

        for event in &ctx.input(|i| i.raw.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                if let Some(path) = &self.screenshot_path {
                    let p = std::path::Path::new(path);
                    if let Some(parent) = p.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = image::save_buffer(
                        path,
                        image.as_raw(),
                        image.width() as u32,
                        image.height() as u32,
                        image::ExtendedColorType::Rgba8,
                    ) {
                        eprintln!("Failed to save screenshot: {e}");
                    } else {
                        println!("Screenshot saved to {path}");
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }

        self.handle_global_keyboard(ctx);

        // 1. Top Ribbon / Menu Bar
        RibbonBar::show(self, ctx);

        // 2. Bottom Status Bar
        egui::TopBottomPanel::bottom("status_bar")
            .min_height(24.0)
            .frame(egui::Frame::side_top_panel(&ctx.style())
                .fill(theme::CHROME)
                .inner_margin(egui::Margin::symmetric(12.0, 4.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&self.status_message).color(theme::MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let page = self.current_page();
                        let (pw, ph) = page.effective_dimensions();
                        let orientation = match page.orientation {
                            Orientation::Portrait => "Portrait",
                            Orientation::Landscape => "Landscape",
                        };
                        ui.label(egui::RichText::new(format!("{:.0} × {:.0} mm, {orientation}", pw, ph)).color(theme::MUTED));
                        ui.label(egui::RichText::new("·").color(theme::RULE));
                        ui.label(egui::RichText::new(format!("{:.0}%", self.zoom * 100.0)).color(theme::MUTED));
                        ui.label(egui::RichText::new("·").color(theme::RULE));
                        if let Some(comp) = self.selected_component() {
                            ui.label(format!("{}   x {:.1}  y {:.1}  w {:.1}  h {:.1} mm", comp.name, comp.rect.x, comp.rect.y, comp.rect.w, comp.rect.h));
                        } else if let Some(band) = self.selected_band() {
                            ui.label(format!("{}   top {:.1}  height {:.1} mm", band.name, band.top, band.height));
                        } else {
                            ui.label(egui::RichText::new("Nothing selected").color(theme::MUTED));
                        }
                    });
                });
            });

        // Main content depends on mode
        match self.app_mode {
            AppMode::Design => {
                // 3. Left Toolbox Dock
                ToolboxPanel::show(self, ctx);

                // 4. Right Properties Inspector
                PropertiesPanel::show(self, ctx);

                // 5. Central Interactive Visual Canvas
                CanvasView::show(self, ctx);
            }
            AppMode::Preview => {
                PreviewView::show(self, ctx);
            }
            AppMode::CodeView => {
                CodeView::show(self, ctx);
            }
        }

        // Dialogs
        NewReportDialog::show(self, ctx);
        PageSetupDialog::show(self, ctx);
    }
}
