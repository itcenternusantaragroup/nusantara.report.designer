//! Turns a Stimulsoft text value into styled runs and paragraphs.
//!
//! When a component has "allow HTML tags" on, the text is read as an HTML subset:
//! - inline: b/strong, i/em, u, s/strike/del, mark, span/font (colour, size, face, style),
//!   sup/sub, br;
//! - blocks: p/div and h1..h6 (with `align` or `text-align`), ul/ol with li (bullets and numbers);
//! - entities such as &nbsp; are decoded.
//!
//! With the flag off, the text is shown exactly as written.

use super::mrt_import::named_color;
use super::report::AlignH;

/// The marker in front of a list item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bullet {
    Dot,
    Number(u32),
}

impl Bullet {
    /// The text drawn in front of the item.
    pub fn label(self) -> String {
        match self {
            Bullet::Dot => "\u{2022}  ".to_string(),
            Bullet::Number(n) => format!("{n}. "),
        }
    }
}

/// One piece of text with a single style. `text` is empty for a bare style.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    /// Colour override (RGBA). `None` keeps the component colour.
    pub color: Option<[u8; 4]>,
    /// Background highlight (RGBA).
    pub highlight: Option<[u8; 4]>,
    /// Font family name from the tag (`face`, `font-family`).
    pub family: Option<String>,
    /// Size relative to the component font size.
    pub size_scale: f32,
    /// Absolute size in points (from CSS `font-size: 10pt`). Wins over `size_scale`.
    pub size_pt: Option<f32>,
    /// Paragraph alignment of the block this run is in.
    pub align: Option<AlignH>,
    /// List marker of the item this run is in.
    pub bullet: Option<Bullet>,
}

impl Run {
    fn plain(text: &str) -> Self {
        Self {
            text: text.to_string(),
            bold: false,
            italic: false,
            underline: false,
            strike: false,
            color: None,
            highlight: None,
            family: None,
            size_scale: 1.0,
            size_pt: None,
            align: None,
            bullet: None,
        }
    }
}

impl Default for Run {
    fn default() -> Self {
        Run::plain("")
    }
}

/// A paragraph: its styled runs (no line breaks inside), its alignment and its list marker.
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub align: Option<AlignH>,
    pub bullet: Option<Bullet>,
}

/// Parses `input` into runs. Adjacent runs with the same style are not merged; line breaks are
/// kept as `\n` inside the runs.
pub fn parse_runs(input: &str, allow_html: bool) -> Vec<Run> {
    if !allow_html {
        return vec![Run::plain(input)];
    }

    let chars: Vec<char> = input.chars().collect();
    let mut runs: Vec<Run> = Vec::new();
    let mut style = Run::plain("");
    // Each open tag remembers the style it replaces, so its close tag can restore it.
    let mut stack: Vec<(String, Run)> = Vec::new();
    // Open lists: (ordered, items so far).
    let mut lists: Vec<(bool, u32)> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c == '<' {
            if let Some(rel_end) = chars[i..].iter().position(|&x| x == '>') {
                let tag: String = chars[i + 1..i + rel_end].iter().collect();
                if looks_like_tag(&tag) {
                    flush(&mut buf, &style, &mut runs);
                    apply_tag(tag.trim(), &mut buf, &runs, &mut style, &mut stack, &mut lists);
                    i += rel_end + 1;
                    continue;
                }
            }
        }

        if c == '&' {
            if let Some(rel_end) = chars[i..].iter().take(10).position(|&x| x == ';') {
                let name: String = chars[i + 1..i + rel_end].iter().collect();
                if let Some(decoded) = decode_entity(&name) {
                    buf.push_str(&decoded);
                    i += rel_end + 1;
                    continue;
                }
            }
        }

        buf.push(c);
        i += 1;
    }
    flush(&mut buf, &style, &mut runs);
    runs
}

/// The same text split into paragraphs. Empty paragraphs at the end are dropped.
pub fn parse_paragraphs(input: &str, allow_html: bool) -> Vec<Paragraph> {
    let mut out = vec![Paragraph { runs: Vec::new(), align: None, bullet: None }];
    for run in parse_runs(input, allow_html) {
        for (k, piece) in run.text.split('\n').enumerate() {
            if k > 0 {
                out.push(Paragraph { runs: Vec::new(), align: None, bullet: None });
            }
            if piece.is_empty() {
                continue;
            }
            let para = out.last_mut().expect("at least one paragraph");
            if para.runs.is_empty() {
                para.align = run.align;
                para.bullet = run.bullet;
            }
            let mut r = run.clone();
            r.text = piece.to_string();
            para.runs.push(r);
        }
    }
    while out.len() > 1 && out.last().is_some_and(|p| p.runs.is_empty()) {
        out.pop();
    }
    out
}

/// The text with all tags removed, as the designer shows it in a plain field.
pub fn strip_tags(input: &str) -> String {
    parse_runs(input, true).into_iter().map(|r| r.text).collect()
}

fn flush(buf: &mut String, style: &Run, runs: &mut Vec<Run>) {
    if !buf.is_empty() {
        let mut run = style.clone();
        run.text = std::mem::take(buf);
        runs.push(run);
    }
}

fn looks_like_tag(tag: &str) -> bool {
    let t = tag.trim_start_matches('/').trim_start();
    t.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
}

/// Starts a new paragraph unless the text so far already ends with a line break (or is empty).
fn block_break(buf: &mut String, runs: &[Run]) {
    let last = buf.chars().last().or_else(|| runs.last().and_then(|r| r.text.chars().last()));
    if matches!(last, Some(c) if c != '\n') {
        buf.push('\n');
    }
}

fn apply_tag(
    tag: &str,
    buf: &mut String,
    runs: &[Run],
    style: &mut Run,
    stack: &mut Vec<(String, Run)>,
    lists: &mut Vec<(bool, u32)>,
) {
    let closing = tag.starts_with('/');
    let body = tag.trim_start_matches('/').trim();
    let name = body.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
    let attrs = &body[name.len().min(body.len())..];

    match name.as_str() {
        "br" => buf.push('\n'),
        "ul" | "ol" => {
            block_break(buf, runs);
            if closing {
                lists.pop();
            } else {
                lists.push((name == "ol", 0));
            }
        }
        "li" => {
            block_break(buf, runs);
            if closing {
                restore(stack, style, "li");
            } else {
                stack.push(("li".to_string(), style.clone()));
                let (ordered, n) = lists.last_mut().map(|l| {
                    l.1 += 1;
                    *l
                }).unwrap_or((false, 1));
                style.bullet = Some(if ordered { Bullet::Number(n) } else { Bullet::Dot });
            }
        }
        "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            block_break(buf, runs);
            if closing {
                restore(stack, style, &name);
            } else {
                stack.push((name.clone(), style.clone()));
                if let Some(a) = parse_align(attribute_value(attrs, "align").as_deref()) {
                    style.align = Some(a);
                }
                apply_style_attributes(attrs, style);
                match name.as_str() {
                    "h1" => heading(style, 2.0),
                    "h2" => heading(style, 1.5),
                    "h3" => heading(style, 1.17),
                    "h4" => heading(style, 1.0),
                    "h5" => heading(style, 0.83),
                    "h6" => heading(style, 0.67),
                    _ => {}
                }
            }
        }
        _ if closing => restore(stack, style, &name),
        "b" | "strong" | "i" | "em" | "u" | "s" | "strike" | "del" | "mark" | "span" | "font" | "sup" | "sub" => {
            stack.push((name.clone(), style.clone()));
            match name.as_str() {
                "b" | "strong" => style.bold = true,
                "i" | "em" => style.italic = true,
                "u" => style.underline = true,
                "s" | "strike" | "del" => style.strike = true,
                "mark" => style.highlight = Some([255, 242, 0, 255]),
                "sup" | "sub" => style.size_scale *= 0.75,
                _ => apply_style_attributes(attrs, style),
            }
        }
        _ => {}
    }
}

/// Pops the most recent open tag called `name` and restores the style it saved.
fn restore(stack: &mut Vec<(String, Run)>, style: &mut Run, name: &str) {
    if let Some(pos) = stack.iter().rposition(|(n, _)| n == name) {
        let (_, saved) = stack[pos].clone();
        stack.truncate(pos);
        *style = saved;
    }
}

fn heading(style: &mut Run, scale: f32) {
    style.bold = true;
    style.size_scale *= scale;
}

fn parse_align(value: Option<&str>) -> Option<AlignH> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "left" => Some(AlignH::Left),
        "center" | "centre" => Some(AlignH::Center),
        "right" => Some(AlignH::Right),
        "justify" => Some(AlignH::Justify),
        _ => None,
    }
}

/// Applies `color`, `size`, `face` attributes and `style="..."` declarations.
fn apply_style_attributes(attrs: &str, style: &mut Run) {
    for (key, value) in attribute_pairs(attrs) {
        match key.as_str() {
            "color" => style.color = parse_color(&value).or(style.color),
            "face" => style.family = Some(value.trim().to_string()),
            "size" => {
                // 1..7 are HTML size steps. Larger values (Stimulsoft writes e.g. size="12") are points.
                if let Ok(n) = value.trim().parse::<f32>() {
                    if (1.0..=7.0).contains(&n) {
                        style.size_scale = html_size_scale(n as u8);
                    } else if n > 7.0 {
                        style.size_pt = Some(n);
                    }
                }
            }
            "style" => {
                for decl in value.split(';') {
                    let Some((prop, val)) = decl.split_once(':') else { continue };
                    let (prop, val) = (prop.trim().to_ascii_lowercase(), val.trim());
                    match prop.as_str() {
                        "color" => style.color = parse_color(val).or(style.color),
                        "font-size" => style.size_pt = parse_pt(val).or(style.size_pt),
                        "font-family" => style.family = Some(val.trim_matches(|c| c == '\'' || c == '"').to_string()),
                        "font-weight" => style.bold = val.eq_ignore_ascii_case("bold") || val == "700",
                        "font-style" => style.italic = val.eq_ignore_ascii_case("italic"),
                        "text-decoration" | "text-decoration-line" => {
                            let v = val.to_ascii_lowercase();
                            style.underline = v.contains("underline");
                            style.strike = v.contains("line-through");
                        }
                        "background" | "background-color" => {
                            style.highlight = parse_color(val).or(style.highlight);
                        }
                        "text-align" => style.align = parse_align(Some(val)).or(style.align),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn attribute_value(attrs: &str, key: &str) -> Option<String> {
    attribute_pairs(attrs).into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Splits `key="value"` / `key='value'` / `key=value` pairs. Keys are lower-cased.
fn attribute_pairs(attrs: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = attrs.trim();
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq].trim().rsplit(char::is_whitespace).next().unwrap_or("").to_ascii_lowercase();
        let after = rest[eq + 1..].trim_start();
        let (value, remainder) = match after.chars().next() {
            Some(q) if q == '"' || q == '\'' => match after[1..].find(q) {
                Some(end) => (after[1..1 + end].to_string(), &after[end + 2..]),
                None => (after[1..].to_string(), ""),
            },
            _ => {
                let end = after.find(char::is_whitespace).unwrap_or(after.len());
                (after[..end].to_string(), &after[end..])
            }
        };
        out.push((key, value));
        rest = remainder.trim_start();
    }
    out
}

/// HTML `size` attribute (1..7) as a scale of the component font size.
fn html_size_scale(n: u8) -> f32 {
    match n {
        1 => 0.63,
        2 => 0.82,
        3 => 1.0,
        4 => 1.13,
        5 => 1.5,
        6 => 2.0,
        _ => 3.0,
    }
}

fn parse_pt(value: &str) -> Option<f32> {
    let v = value.trim().to_ascii_lowercase();
    if let Some(num) = v.strip_suffix("pt") {
        return num.trim().parse::<f32>().ok();
    }
    if let Some(num) = v.strip_suffix("px") {
        return num.trim().parse::<f32>().ok().map(|px| px * 0.75);
    }
    None
}

/// `#rgb`, `#rrggbb`, `rgb(r,g,b)` or a colour name.
fn parse_color(value: &str) -> Option<[u8; 4]> {
    let v = value.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let digits: Vec<u8> = hex.chars().filter_map(|c| c.to_digit(16).map(|d| d as u8)).collect();
        return match digits.len() {
            3 => Some([digits[0] * 17, digits[1] * 17, digits[2] * 17, 255]),
            6 => Some([
                digits[0] * 16 + digits[1],
                digits[2] * 16 + digits[3],
                digits[4] * 16 + digits[5],
                255,
            ]),
            _ => None,
        };
    }
    if let Some(inner) = v.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let p: Vec<u8> = inner.split(',').filter_map(|x| x.trim().parse::<u8>().ok()).collect();
        if p.len() == 3 {
            return Some([p[0], p[1], p[2], 255]);
        }
        return None;
    }
    named_color(v)
}

fn decode_entity(name: &str) -> Option<String> {
    let decoded = match name {
        "nbsp" => " ".to_string(),
        "amp" => "&".to_string(),
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        _ => {
            let code = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16).ok()?
            } else {
                name.strip_prefix('#')?.parse::<u32>().ok()?
            };
            char::from_u32(code)?.to_string()
        }
    };
    Some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_and_font_tags() {
        let runs = parse_runs("a<b>B</b>c<font size=\"5\" color=\"#ff0000\">d</font>", true);
        assert_eq!(runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(), vec!["a", "B", "c", "d"]);
        assert!(runs[1].bold && !runs[2].bold);
        assert_eq!(runs[3].color, Some([255, 0, 0, 255]));
        assert!(runs[3].size_scale > 1.0);
    }

    #[test]
    fn breaks_and_entities() {
        assert_eq!(strip_tags("x<br>y&nbsp;&amp;z"), "x\ny &z");
    }

    #[test]
    fn flag_off_keeps_markup() {
        assert_eq!(parse_runs("<b>x</b>", false)[0].text, "<b>x</b>");
    }

    #[test]
    fn font_size_attribute_is_points_above_seven() {
        let big = parse_runs("<font size=\"12\">a</font>", true);
        assert_eq!(big[0].size_pt, Some(12.0));
        assert_eq!(big[0].size_scale, 1.0);

        let step = parse_runs("<font size=\"3\">b</font>", true);
        assert_eq!(step[0].size_pt, None);
        assert_eq!(step[0].size_scale, 1.0);
    }

    #[test]
    fn css_style_attributes() {
        let runs = parse_runs("<span style=\"color:#00ff00; font-size:9pt; text-decoration: underline\">g</span>", true);
        assert_eq!(runs[0].color, Some([0, 255, 0, 255]));
        assert_eq!(runs[0].size_pt, Some(9.0));
        assert!(runs[0].underline);
    }

    #[test]
    fn strike_highlight_and_family() {
        let runs = parse_runs("<s>a</s><mark>b</mark><span style=\"background-color:#00ff00;font-family:'Courier New'\">c</span>", true);
        assert!(runs[0].strike);
        assert_eq!(runs[1].highlight, Some([255, 242, 0, 255]));
        assert_eq!(runs[2].highlight, Some([0, 255, 0, 255]));
        assert_eq!(runs[2].family.as_deref(), Some("Courier New"));
    }

    #[test]
    fn paragraphs_carry_alignment() {
        let paras = parse_paragraphs("<p align=\"center\">one</p><div style=\"text-align:right\">two</div>", true);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].runs[0].text, "one");
        assert_eq!(paras[0].align, Some(AlignH::Center));
        assert_eq!(paras[1].align, Some(AlignH::Right));
    }

    #[test]
    fn lists_get_bullets_and_numbers() {
        let dots = parse_paragraphs("<ul><li>a</li><li>b</li></ul>", true);
        assert_eq!(dots.len(), 2);
        assert_eq!(dots[0].bullet, Some(Bullet::Dot));
        assert_eq!(dots[1].runs[0].text, "b");

        let nums = parse_paragraphs("<ol><li>x</li><li>y</li></ol>", true);
        assert_eq!(nums[1].bullet, Some(Bullet::Number(2)));
        assert_eq!(nums[1].bullet.unwrap().label(), "2. ");
    }

    #[test]
    fn headings_are_bold_and_larger() {
        let runs = parse_runs("<h1>Title</h1>", true);
        assert!(runs[0].bold);
        assert!(runs[0].size_scale > 1.9);
    }

    #[test]
    fn plain_text_is_one_paragraph_per_line() {
        let paras = parse_paragraphs("a\nb\n", false);
        assert_eq!(paras.len(), 2, "trailing empty line is dropped");
        assert_eq!(paras[1].runs[0].text, "b");
    }
}
