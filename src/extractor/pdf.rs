use anyhow::{Context, Result};
use std::path::Path;

/// Extract all text content from a PDF file.
///
/// Uses the `pdf-extract` crate which handles most standard PDFs.
/// Scanned PDFs (image-only) will return empty or minimal text —
/// in that case you should run the PDF pages through the image OCR extractor.
pub fn extract(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read PDF: {}", path.display()))?;

    let text = pdf_extract::extract_text_from_mem(&bytes)
        .with_context(|| format!("Failed to extract text from PDF: {}", path.display()))?;

    Ok(normalise_whitespace(&text))
}

/// Collapse excessive blank lines and trim trailing whitespace per line.
fn normalise_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut blank_count = 0u32;

    for line in s.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            blank_count += 1;
            // Allow at most one consecutive blank line.
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalise_removes_excess_blanks() {
        let input = "Line 1\n\n\n\nLine 2\n";
        let output = normalise_whitespace(input);
        assert_eq!(output, "Line 1\n\nLine 2");
    }
}
