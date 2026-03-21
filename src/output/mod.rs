pub mod docx;
pub mod json;

use crate::ExtractionResult;
use anyhow::Result;
use std::path::Path;

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Docx,
    Txt,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Docx => write!(f, "docx"),
            OutputFormat::Txt => write!(f, "txt"),
        }
    }
}

/// Write `result` to `output_path` using the specified format.
pub fn write(result: &ExtractionResult, output_path: &Path, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => json::write(result, output_path),
        OutputFormat::Docx => docx::write(result, output_path),
        OutputFormat::Txt => txt_write(result, output_path),
    }
}

fn txt_write(result: &ExtractionResult, output_path: &Path) -> Result<()> {
    let mut out = format!("Source: {}\n\n", result.source);

    for q in &result.questions {
        use crate::parser::Question;
        match q {
            Question::Mcq(m) => {
                out.push_str(&format!("{}. [MCQ] {}\n", m.number, m.stem));
                for opt in &m.options {
                    out.push_str(&format!("   {}. {}\n", opt.label, opt.text));
                }
                if let Some(ans) = &m.answer {
                    out.push_str(&format!("   Answer: {}\n", ans));
                }
                out.push('\n');
            }
            Question::Cq(c) => {
                let marks = c
                    .marks
                    .map(|m| format!(" [{} marks]", m))
                    .unwrap_or_default();
                out.push_str(&format!("{}. [CQ] {}{}\n", c.number, c.stem, marks));
                for part in &c.parts {
                    let pm = part
                        .marks
                        .map(|m| format!(" [{} marks]", m))
                        .unwrap_or_default();
                    out.push_str(&format!("   ({}) {}{}\n", part.label, part.text, pm));
                }
                out.push('\n');
            }
        }
    }

    std::fs::write(output_path, out)?;
    Ok(())
}
