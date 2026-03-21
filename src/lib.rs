pub mod extractor;
pub mod output;
pub mod parser;

use anyhow::Result;
use std::path::Path;

/// High-level result of processing a file.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExtractionResult {
    pub source: String,
    pub raw_text: String,
    pub questions: Vec<parser::Question>,
}

/// Extract raw text from a file (PDF or image).
pub fn extract_text(path: &Path) -> Result<String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "pdf" => extractor::pdf::extract(path),
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "webp" => {
            extractor::image::extract(path)
        }
        other => anyhow::bail!("Unsupported file type: .{}", other),
    }
}

/// Full pipeline: extract text → parse questions.
pub fn process_file(path: &Path) -> Result<ExtractionResult> {
    let raw_text = extract_text(path)?;
    let questions = parser::parse_questions(&raw_text);
    Ok(ExtractionResult {
        source: path.display().to_string(),
        raw_text,
        questions,
    })
}
