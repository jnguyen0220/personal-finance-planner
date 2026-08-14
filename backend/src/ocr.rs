//! Best-effort text and amount extraction from invoice attachments. Images are
//! preprocessed (grayscale + upscaling) and read with the `tesseract` CLI using
//! orientation detection so rotated phone photos are auto-corrected; PDFs use
//! `pdftotext` (poppler-utils). When a tool is missing or fails the functions
//! return empty results so ingestion continues without OCR.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The outcome of running OCR on an attachment, surfaced as a status light in
/// the review UI.
#[derive(Clone, Copy)]
pub enum OcrStatus {
    /// Extraction ran and a monetary total was detected (green).
    Success,
    /// Extraction ran but no amount could be detected (orange).
    NoDetection,
    /// The OCR tool was missing or exited with an error (red).
    Failed,
}

impl OcrStatus {
    /// The value persisted in `inbox_items.ocr_status` and sent to the client.
    pub fn as_str(self) -> &'static str {
        match self {
            OcrStatus::Success => "success",
            OcrStatus::NoDetection => "no_detection",
            OcrStatus::Failed => "failed",
        }
    }
}

/// Runs OCR/text extraction on a stored file, returning the recognized text, a
/// best-guess monetary total, and the run's outcome. Blocking — call from a
/// blocking context.
pub fn extract(path: &Path, content_type: &str) -> (String, Option<f64>, OcrStatus) {
    let (text, ran) = if content_type == "application/pdf" {
        pdf_text(path)
    } else {
        image_text(path)
    };
    if !ran {
        return (text, None, OcrStatus::Failed);
    }
    let amount = parse_amount(&text);
    let status = if amount.is_some() {
        OcrStatus::Success
    } else {
        OcrStatus::NoDetection
    };
    (text, amount, status)
}

fn image_text(path: &Path) -> (String, bool) {
    // Preprocess to a temp file when possible, falling back to the original.
    // `--psm 1` runs orientation/script detection so rotated photos are
    // auto-rotated before recognition.
    let prepared = preprocess(path);
    let target = prepared.as_ref().map(|p| p.path.as_path()).unwrap_or(path);
    run(Command::new("tesseract")
        .arg(target)
        .arg("stdout")
        .args(["-l", "eng", "--psm", "1"]))
}

/// A preprocessed copy of an image written to a temp file, removed on drop.
struct Prepared {
    path: PathBuf,
}

impl Drop for Prepared {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Produces a grayscale, size-normalized copy of an image to improve
/// recognition. Returns `None` on any failure so the caller OCRs the original
/// file instead. Tesseract reads best near ~300 DPI, so small scans are
/// enlarged; very large photos are shrunk to keep OCR (and orientation
/// detection) fast.
fn preprocess(path: &Path) -> Option<Prepared> {
    const MIN_DIM: u32 = 1000;
    const MAX_DIM: u32 = 2200;
    let mut gray = image::open(path).ok()?.into_luma8();
    let (w, h) = gray.dimensions();
    let (smallest, largest) = (w.min(h), w.max(h));
    if smallest == 0 {
        return None;
    }
    let scale = if largest > MAX_DIM {
        MAX_DIM as f32 / largest as f32
    } else if smallest < MIN_DIM {
        (MIN_DIM as f32 / smallest as f32).min(4.0)
    } else {
        1.0
    };
    if scale != 1.0 {
        let nw = (w as f32 * scale).round().max(1.0) as u32;
        let nh = (h as f32 * scale).round().max(1.0) as u32;
        gray = image::imageops::resize(&gray, nw, nh, image::imageops::FilterType::Triangle);
    }
    let out = std::env::temp_dir().join(format!("ocr-{}.png", uuid::Uuid::new_v4()));
    gray.save(&out).ok()?;
    Some(Prepared { path: out })
}

fn pdf_text(path: &Path) -> (String, bool) {
    // `-` writes extracted text to stdout. Works only for text-based PDFs.
    run(Command::new("pdftotext").arg(path).arg("-"))
}

/// Runs a command, returning its stdout and whether it completed successfully.
fn run(cmd: &mut Command) -> (String, bool) {
    match cmd.output() {
        Ok(out) if out.status.success() => {
            (String::from_utf8_lossy(&out.stdout).into_owned(), true)
        }
        Ok(out) => {
            tracing::warn!(
                "ocr: {:?} exited with {}: {}",
                cmd.get_program(),
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            (String::new(), false)
        }
        Err(e) => {
            tracing::warn!("ocr: failed to run {:?}: {e}", cmd.get_program());
            (String::new(), false)
        }
    }
}

/// Guesses the invoice total from OCR text. Prefers a monetary value on a line
/// mentioning "total" (ignoring "subtotal"), and otherwise takes the largest
/// value found. Returns `None` when no plausible amount is present.
pub fn parse_amount(text: &str) -> Option<f64> {
    let mut totals: Vec<f64> = Vec::new();
    let mut all: Vec<f64> = Vec::new();

    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        let is_total = lower.contains("total") && !lower.contains("subtotal");
        for value in amounts_in(line) {
            all.push(value);
            if is_total {
                totals.push(value);
            }
        }
    }

    let pick = |v: &[f64]| v.iter().cloned().fold(f64::NAN, f64::max);
    if !totals.is_empty() {
        Some(pick(&totals))
    } else if !all.is_empty() {
        Some(pick(&all))
    } else {
        None
    }
}

/// Extracts currency-like numbers from a line, e.g. `$1,234.56` or `89.00`.
fn amounts_in(line: &str) -> Vec<f64> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_digit() || ch == ',' || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let token: String = line[start..i].chars().filter(|c| *c != ',').collect();
            // Require a decimal point to avoid matching quantities/IDs.
            if token.contains('.') {
                if let Ok(v) = token.parse::<f64>() {
                    out.push(v);
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_amount;

    #[test]
    fn prefers_total_line() {
        let text = "Subtotal: $90.00\nTax: $10.00\nTotal Due: $100.00";
        assert_eq!(parse_amount(text), Some(100.00));
    }

    #[test]
    fn falls_back_to_largest() {
        let text = "Item A 12.50\nItem B 8.00";
        assert_eq!(parse_amount(text), Some(12.50));
    }

    #[test]
    fn none_when_no_amount() {
        assert_eq!(parse_amount("no numbers with cents here 42"), None);
    }
}
