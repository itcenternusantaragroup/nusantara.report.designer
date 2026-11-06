//! Real barcodes: the bar pattern of a value, as runs of black bars.
//!
//! Supported formats: `Code128` (character set B, any printable ASCII) and `Code39`
//! (upper-case letters, digits and `-. $/+%`). Anything else falls back to Code128.

use barcoders::sym::code128::Code128;
use barcoders::sym::code39::Code39;

/// One black bar: start and width as fractions (0..=1) of the full barcode width.
pub type Bar = (f32, f32);

/// The bars for `data` in `format`, or an error when the value cannot be encoded.
pub fn bars(format: &str, data: &str) -> Result<Vec<Bar>, String> {
    if data.is_empty() {
        return Err("empty value".to_string());
    }
    let modules: Vec<u8> = if format.eq_ignore_ascii_case("Code39") {
        Code39::new(data.to_uppercase()).map_err(|e| format!("{e:?}"))?.encode()
    } else {
        Code128::new(format!("\u{0181}{data}")).map_err(|e| format!("{e:?}"))?.encode()
    };
    Ok(runs(&modules))
}

/// Groups consecutive black modules (value 1) into bars, as fractions of the total width.
fn runs(modules: &[u8]) -> Vec<Bar> {
    let total = modules.len().max(1) as f32;
    let mut out = Vec::new();
    let mut i = 0;
    while i < modules.len() {
        if modules[i] == 1 {
            let start = i;
            while i < modules.len() && modules[i] == 1 {
                i += 1;
            }
            out.push((start as f32 / total, (i - start) as f32 / total));
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code128_has_bars_and_covers_the_width() {
        let b = bars("Code128", "NO-12345").expect("encodes");
        assert!(b.len() > 10);
        let (last_start, last_w) = *b.last().unwrap();
        assert!(last_start + last_w <= 1.0 + 1e-4);
        assert!(b[0].0 < 0.05, "starts near the left edge");
    }

    #[test]
    fn code39_encodes_upper_case_values() {
        assert!(bars("Code39", "abc-123").is_ok());
    }

    #[test]
    fn empty_value_is_an_error() {
        assert!(bars("Code128", "").is_err());
    }
}
