use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::dictionary::Dictionary;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RectMm {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectMm {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= (self.x + self.w) && py >= self.y && py <= (self.y + self.h)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Margins {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            left: 10.0,
            right: 10.0,
            top: 10.0,
            bottom: 10.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BandType {
    ReportTitle,
    PageHeader,
    Header,
    DataBand,
    Footer,
    PageFooter,
    ReportSummary,
}

impl BandType {
    pub fn display_name(&self) -> &'static str {
        match self {
            BandType::ReportTitle => "Report Title",
            BandType::PageHeader => "Page Header",
            BandType::Header => "Header",
            BandType::DataBand => "Data Band",
            BandType::Footer => "Footer",
            BandType::PageFooter => "Page Footer",
            BandType::ReportSummary => "Report Summary",
        }
    }

    pub fn default_color(&self) -> [u8; 4] {
        match self {
            BandType::ReportTitle => [41, 128, 185, 255],  // Blue
            BandType::PageHeader => [52, 152, 219, 255],   // Sky Blue
            BandType::Header => [39, 174, 96, 255],        // Green
            BandType::DataBand => [230, 126, 34, 255],     // Orange
            BandType::Footer => [142, 68, 173, 255],       // Purple
            BandType::PageFooter => [44, 62, 80, 255],     // Dark Slate
            BandType::ReportSummary => [192, 57, 43, 255], // Red
        }
    }

    pub fn sti_ident(&self) -> &'static str {
        match self {
            BandType::ReportTitle => "StiReportTitleBand",
            BandType::PageHeader => "StiPageHeaderBand",
            BandType::Header => "StiHeaderBand",
            BandType::DataBand => "StiDataBand",
            BandType::Footer => "StiFooterBand",
            BandType::PageFooter => "StiPageFooterBand",
            BandType::ReportSummary => "StiReportSummaryBand",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Band {
    pub id: String,
    pub name: String,
    pub band_type: BandType,
    pub top: f32, // mm
    pub height: f32, // mm
    pub data_source: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentType {
    Text,
    Image,
    Shape,
    Line,
    Barcode,
    CheckBox,
    Table,
}

impl ComponentType {
    pub fn display_name(&self) -> &'static str {
        match self {
            ComponentType::Text => "Text",
            ComponentType::Image => "Image",
            ComponentType::Shape => "Shape",
            ComponentType::Line => "Line",
            ComponentType::Barcode => "Barcode",
            ComponentType::CheckBox => "CheckBox",
            ComponentType::Table => "Table",
        }
    }

    pub fn sti_ident(&self) -> &'static str {
        match self {
            ComponentType::Text => "StiText",
            ComponentType::Image => "StiImage",
            ComponentType::Shape => "StiShape",
            ComponentType::Line => "StiHorizontalLinePrimitive",
            ComponentType::Barcode => "StiBarCode",
            ComponentType::CheckBox => "StiCheckBox",
            ComponentType::Table => "StiTable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignH {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlignV {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeKind {
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Triangle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderSides {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

impl Default for BorderSides {
    fn default() -> Self {
        Self {
            left: false,
            top: false,
            right: false,
            bottom: false,
        }
    }
}

impl BorderSides {
    pub fn all() -> Self {
        Self { left: true, top: true, right: true, bottom: true }
    }
    pub fn none() -> Self {
        Self { left: false, top: false, right: false, bottom: false }
    }
    pub fn has_any(&self) -> bool {
        self.left || self.top || self.right || self.bottom
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub comp_type: ComponentType,
    pub rect: RectMm, // in mm
    pub text: String,
    pub font_family: String,
    pub font_size: f32, // in pt
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub text_color: [u8; 4],
    pub bg_color: [u8; 4],
    pub border_color: [u8; 4],
    pub border_width: f32, // in mm
    pub border_sides: BorderSides,
    pub align_h: AlignH,
    pub align_v: AlignV,
    pub word_wrap: bool,
    pub shape_kind: ShapeKind,
    pub barcode_format: String,
    pub checked: bool,
    /// Read the text as HTML (b, i, u, font, span, br ...). Off: the text is shown as written.
    pub allow_html_tags: bool,
    pub parent_band_id: Option<String>,
}

impl Default for Component {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Text1".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(10.0, 10.0, 60.0, 10.0),
            text: "Text Label".to_string(),
            font_family: "Arial".to_string(),
            font_size: 10.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [30, 30, 30, 255],
            bg_color: [0, 0, 0, 0], // transparent
            border_color: [180, 180, 180, 255],
            border_width: 0.5,
            border_sides: BorderSides::none(),
            align_h: AlignH::Left,
            align_v: AlignV::Center,
            word_wrap: true,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub name: String,
    pub width: f32, // 210.0 mm for A4
    pub height: f32, // 297.0 mm for A4
    pub margins: Margins,
    pub orientation: Orientation,
    pub bands: Vec<Band>,
    pub components: Vec<Component>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "Page1".to_string(),
            width: 210.0,
            height: 297.0,
            margins: Margins::default(),
            orientation: Orientation::Portrait,
            bands: Vec::new(),
            components: Vec::new(),
        }
    }
}

impl Page {
    pub fn printable_width(&self) -> f32 {
        let (w, _) = self.effective_dimensions();
        w - self.margins.left - self.margins.right
    }

    pub fn printable_height(&self) -> f32 {
        let (_, h) = self.effective_dimensions();
        h - self.margins.top - self.margins.bottom
    }

    pub fn effective_dimensions(&self) -> (f32, f32) {
        match self.orientation {
            Orientation::Portrait => (self.width, self.height),
            Orientation::Landscape => (self.height, self.width),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub guid: String,
    pub name: String,
    pub alias: String,
    pub version: String,
    pub file_path: Option<String>,
    pub pages: Vec<Page>,
    pub dictionary: Dictionary,
}

impl Report {
    /// An empty report: one blank A4 page and no sample data.
    pub fn blank() -> Self {
        Self {
            guid: Uuid::new_v4().to_string(),
            name: "Report".to_string(),
            alias: "New Report".to_string(),
            version: "2020.2.3".to_string(),
            file_path: None,
            pages: vec![Page::default()],
            dictionary: Dictionary {
                data_sources: Vec::new(),
                variables: Vec::new(),
            },
        }
    }
}

impl Default for Report {
    fn default() -> Self {
        let mut report = Self {
            guid: Uuid::new_v4().to_string(),
            name: "Report".to_string(),
            alias: "Sales Report".to_string(),
            version: "2020.2.3".to_string(),
            file_path: None,
            pages: Vec::new(),
            dictionary: Dictionary::default(),
        };

        // Create a default rich starter report page with realistic bands and components!
        let mut page = Page::default();

        // 1. Report Title Band
        let title_band = Band {
            id: "band_report_title".to_string(),
            name: "ReportTitleBand1".to_string(),
            band_type: BandType::ReportTitle,
            top: 10.0,
            height: 22.0,
            data_source: None,
        };

        // Component in Report Title Band: Company Title and Report Header
        let comp_title = Component {
            id: Uuid::new_v4().to_string(),
            name: "TitleText".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(10.0, 11.0, 190.0, 10.0),
            text: "{CompanyName}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 16.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [30, 70, 130, 255],
            bg_color: [0, 0, 0, 0],
            border_color: [0, 0, 0, 0],
            border_width: 0.0,
            border_sides: BorderSides::none(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: true,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_report_title".to_string()),
        };

        let comp_subtitle = Component {
            id: Uuid::new_v4().to_string(),
            name: "SubtitleText".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(10.0, 21.0, 190.0, 8.0),
            text: "{ReportTitle} - Summary of Orders".to_string(),
            font_family: "Arial".to_string(),
            font_size: 11.0,
            bold: false,
            italic: true,
            underline: false,
            text_color: [100, 100, 100, 255],
            bg_color: [0, 0, 0, 0],
            border_color: [0, 0, 0, 0],
            border_width: 0.0,
            border_sides: BorderSides::none(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: true,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_report_title".to_string()),
        };

        // 2. Header Band (Table Column Headers)
        let header_band = Band {
            id: "band_header".to_string(),
            name: "HeaderOrdersBand".to_string(),
            band_type: BandType::Header,
            top: 35.0,
            height: 10.0,
            data_source: Some("Orders".to_string()),
        };

        let h_order_id = Component {
            id: Uuid::new_v4().to_string(),
            name: "H_OrderID".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(10.0, 35.0, 20.0, 10.0),
            text: "Order #".to_string(),
            font_family: "Arial".to_string(),
            font_size: 9.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [255, 255, 255, 255],
            bg_color: [41, 128, 185, 255],
            border_color: [255, 255, 255, 255],
            border_width: 0.4,
            border_sides: BorderSides::all(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_header".to_string()),
        };

        let h_customer = Component {
            id: Uuid::new_v4().to_string(),
            name: "H_Customer".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(30.0, 35.0, 48.0, 10.0),
            text: "Customer".to_string(),
            font_family: "Arial".to_string(),
            font_size: 9.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [255, 255, 255, 255],
            bg_color: [41, 128, 185, 255],
            border_color: [255, 255, 255, 255],
            border_width: 0.4,
            border_sides: BorderSides::all(),
            align_h: AlignH::Left,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_header".to_string()),
        };

        let h_product = Component {
            id: Uuid::new_v4().to_string(),
            name: "H_Product".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(78.0, 35.0, 48.0, 10.0),
            text: "Product Description".to_string(),
            font_family: "Arial".to_string(),
            font_size: 9.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [255, 255, 255, 255],
            bg_color: [41, 128, 185, 255],
            border_color: [255, 255, 255, 255],
            border_width: 0.4,
            border_sides: BorderSides::all(),
            align_h: AlignH::Left,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_header".to_string()),
        };

        let h_qty = Component {
            id: Uuid::new_v4().to_string(),
            name: "H_Qty".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(126.0, 35.0, 18.0, 10.0),
            text: "Qty".to_string(),
            font_family: "Arial".to_string(),
            font_size: 9.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [255, 255, 255, 255],
            bg_color: [41, 128, 185, 255],
            border_color: [255, 255, 255, 255],
            border_width: 0.4,
            border_sides: BorderSides::all(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_header".to_string()),
        };

        let h_price = Component {
            id: Uuid::new_v4().to_string(),
            name: "H_Price".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(144.0, 35.0, 22.0, 10.0),
            text: "Price ($)".to_string(),
            font_family: "Arial".to_string(),
            font_size: 9.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [255, 255, 255, 255],
            bg_color: [41, 128, 185, 255],
            border_color: [255, 255, 255, 255],
            border_width: 0.4,
            border_sides: BorderSides::all(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_header".to_string()),
        };

        let h_total = Component {
            id: Uuid::new_v4().to_string(),
            name: "H_Total".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(166.0, 35.0, 24.0, 10.0),
            text: "Total ($)".to_string(),
            font_family: "Arial".to_string(),
            font_size: 9.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [255, 255, 255, 255],
            bg_color: [41, 128, 185, 255],
            border_color: [255, 255, 255, 255],
            border_width: 0.4,
            border_sides: BorderSides::all(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_header".to_string()),
        };

        // 3. Data Band (Row Template)
        let data_band = Band {
            id: "band_data".to_string(),
            name: "DataOrdersBand".to_string(),
            band_type: BandType::DataBand,
            top: 47.0,
            height: 9.0,
            data_source: Some("Orders".to_string()),
        };

        let d_order_id = Component {
            id: Uuid::new_v4().to_string(),
            name: "Data_OrderID".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(10.0, 47.0, 20.0, 9.0),
            text: "{Orders.OrderID}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [30, 30, 30, 255],
            bg_color: [248, 249, 250, 255],
            border_color: [210, 215, 220, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_data".to_string()),
        };

        let d_customer = Component {
            id: Uuid::new_v4().to_string(),
            name: "Data_Customer".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(30.0, 47.0, 48.0, 9.0),
            text: "{Orders.Customer}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [30, 30, 30, 255],
            bg_color: [248, 249, 250, 255],
            border_color: [210, 215, 220, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Left,
            align_v: AlignV::Center,
            word_wrap: true,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_data".to_string()),
        };

        let d_product = Component {
            id: Uuid::new_v4().to_string(),
            name: "Data_Product".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(78.0, 47.0, 48.0, 9.0),
            text: "{Orders.Product}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [30, 30, 30, 255],
            bg_color: [248, 249, 250, 255],
            border_color: [210, 215, 220, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Left,
            align_v: AlignV::Center,
            word_wrap: true,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_data".to_string()),
        };

        let d_qty = Component {
            id: Uuid::new_v4().to_string(),
            name: "Data_Qty".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(126.0, 47.0, 18.0, 9.0),
            text: "{Orders.Qty}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [30, 30, 30, 255],
            bg_color: [248, 249, 250, 255],
            border_color: [210, 215, 220, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_data".to_string()),
        };

        let d_price = Component {
            id: Uuid::new_v4().to_string(),
            name: "Data_Price".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(144.0, 47.0, 22.0, 9.0),
            text: "${Orders.Price}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [30, 30, 30, 255],
            bg_color: [248, 249, 250, 255],
            border_color: [210, 215, 220, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_data".to_string()),
        };

        let d_total = Component {
            id: Uuid::new_v4().to_string(),
            name: "Data_Total".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(166.0, 47.0, 24.0, 9.0),
            text: "${Orders.Total}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.5,
            bold: true,
            italic: false,
            underline: false,
            text_color: [20, 90, 40, 255],
            bg_color: [248, 249, 250, 255],
            border_color: [210, 215, 220, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_data".to_string()),
        };

        // 4. Footer Band (Totals & Signature)
        let footer_band = Band {
            id: "band_footer".to_string(),
            name: "FooterOrdersBand".to_string(),
            band_type: BandType::Footer,
            top: 60.0,
            height: 12.0,
            data_source: Some("Orders".to_string()),
        };

        let f_label = Component {
            id: Uuid::new_v4().to_string(),
            name: "F_Label".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(110.0, 60.0, 54.0, 10.0),
            text: "Grand Total:".to_string(),
            font_family: "Arial".to_string(),
            font_size: 10.0,
            bold: true,
            italic: false,
            underline: false,
            text_color: [40, 40, 40, 255],
            bg_color: [0, 0, 0, 0],
            border_color: [0, 0, 0, 0],
            border_width: 0.0,
            border_sides: BorderSides::none(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_footer".to_string()),
        };

        let f_sum = Component {
            id: Uuid::new_v4().to_string(),
            name: "F_SumTotal".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(166.0, 60.0, 24.0, 10.0),
            text: "$2,588.75".to_string(),
            font_family: "Arial".to_string(),
            font_size: 10.5,
            bold: true,
            italic: false,
            underline: true,
            text_color: [180, 20, 20, 255],
            bg_color: [255, 245, 230, 255],
            border_color: [180, 20, 20, 255],
            border_width: 0.5,
            border_sides: BorderSides::all(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_footer".to_string()),
        };

        // 5. Page Footer Band
        let page_footer_band = Band {
            id: "band_page_footer".to_string(),
            name: "PageFooterBand1".to_string(),
            band_type: BandType::PageFooter,
            top: 275.0,
            height: 12.0,
            data_source: None,
        };

        let pf_info = Component {
            id: Uuid::new_v4().to_string(),
            name: "PF_Info".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(10.0, 276.0, 95.0, 8.0),
            text: "Generated with Nusantara Report Designer".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: true,
            underline: false,
            text_color: [130, 130, 130, 255],
            bg_color: [0, 0, 0, 0],
            border_color: [0, 0, 0, 0],
            border_width: 0.0,
            border_sides: BorderSides::none(),
            align_h: AlignH::Left,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_page_footer".to_string()),
        };

        let pf_page_num = Component {
            id: Uuid::new_v4().to_string(),
            name: "PF_PageNum".to_string(),
            comp_type: ComponentType::Text,
            rect: RectMm::new(150.0, 276.0, 50.0, 8.0),
            text: "Page {PageNumber} of {TotalPageCount}".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [130, 130, 130, 255],
            bg_color: [0, 0, 0, 0],
            border_color: [0, 0, 0, 0],
            border_width: 0.0,
            border_sides: BorderSides::none(),
            align_h: AlignH::Right,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: Some("band_page_footer".to_string()),
        };

        // Extra elements: Barcode for order tracking
        let bc_tracking = Component {
            id: Uuid::new_v4().to_string(),
            name: "OrderBarcode".to_string(),
            comp_type: ComponentType::Barcode,
            rect: RectMm::new(10.0, 80.0, 48.0, 16.0),
            text: "NUSANTARA-2024-MRT".to_string(),
            font_family: "Arial".to_string(),
            font_size: 8.0,
            bold: false,
            italic: false,
            underline: false,
            text_color: [0, 0, 0, 255],
            bg_color: [255, 255, 255, 255],
            border_color: [200, 200, 200, 255],
            border_width: 0.3,
            border_sides: BorderSides::all(),
            align_h: AlignH::Center,
            align_v: AlignV::Center,
            word_wrap: false,
            shape_kind: ShapeKind::Rectangle,
            barcode_format: "Code128".to_string(),
            checked: false,
            allow_html_tags: false,
            parent_band_id: None,
        };

        page.bands = vec![title_band, header_band, data_band, footer_band, page_footer_band];
        page.components = vec![
            comp_title, comp_subtitle,
            h_order_id, h_customer, h_product, h_qty, h_price, h_total,
            d_order_id, d_customer, d_product, d_qty, d_price, d_total,
            f_label, f_sum,
            bc_tracking,
            pf_info, pf_page_num,
        ];

        report.pages.push(page);
        report
    }
}
