use crate::{
    parser::{
        cq::CqQuestion,
        mcq::McqQuestion,
        Question,
    },
    ExtractionResult,
};
use anyhow::{Context, Result};
use docx_rs::{
    Docx, Paragraph, Run, RunFonts,
    AlignmentType,
};
use std::path::Path;

// --------------------------------------------------------------------------
// Public entry point
// --------------------------------------------------------------------------

/// Render the extraction result as a `.docx` file.
pub fn write(result: &ExtractionResult, output_path: &Path) -> Result<()> {
    let file = std::fs::File::create(output_path)
        .with_context(|| format!("Cannot create DOCX file: {}", output_path.display()))?;

    let mut doc = Docx::new();

    // Title
    doc = doc.add_paragraph(
        Paragraph::new()
            .add_run(
                Run::new()
                    .add_text(format!("Extracted Questions — {}", result.source))
                    .bold()
                    .size(28),
            )
            .align(AlignmentType::Center),
    );

    doc = doc.add_paragraph(Paragraph::new()); // spacer

    for question in &result.questions {
        doc = match question {
            Question::Mcq(q) => add_mcq(doc, q),
            Question::Cq(q) => add_cq(doc, q),
        };
    }

    doc.build()
        .pack(file)
        .map_err(|e| anyhow::anyhow!("DOCX build error: {}", e))?;

    Ok(())
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn heading_run(text: String) -> Run {
    Run::new()
        .add_text(text)
        .bold()
        .size(24)
        .fonts(RunFonts::new().ascii("Calibri"))
}

fn body_run(text: String) -> Run {
    Run::new()
        .add_text(text)
        .size(22)
        .fonts(RunFonts::new().ascii("Calibri"))
}

fn add_mcq(mut doc: Docx, q: &McqQuestion) -> Docx {
    // Question stem
    doc = doc.add_paragraph(
        Paragraph::new().add_run(
            heading_run(format!("{}. [MCQ]  {}", q.number, q.stem)),
        ),
    );

    // Options
    for opt in &q.options {
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(body_run(format!("    ({})  {}", opt.label, opt.text)))
                .indent(Some(720), None, None, None),
        );
    }

    // Answer key (if present)
    if let Some(ans) = &q.answer {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(
                body_run(format!("    ✓ Answer: {}", ans))
                    .color("2E7D32"), // dark green
            ),
        );
    }

    // Spacer
    doc.add_paragraph(Paragraph::new())
}

fn add_cq(mut doc: Docx, q: &CqQuestion) -> Docx {
    let marks_label = q
        .marks
        .map(|m| format!("  [{} marks]", m))
        .unwrap_or_default();

    doc = doc.add_paragraph(
        Paragraph::new().add_run(
            heading_run(format!("{}. [CQ]  {}{}", q.number, q.stem, marks_label)),
        ),
    );

    for part in &q.parts {
        let pm = part
            .marks
            .map(|m| format!("  [{} marks]", m))
            .unwrap_or_default();
        doc = doc.add_paragraph(
            Paragraph::new()
                .add_run(body_run(format!(
                    "    ({})  {}{}",
                    part.label, part.text, pm
                )))
                .indent(Some(720), None, None, None),
        );
    }

    doc.add_paragraph(Paragraph::new())
}
