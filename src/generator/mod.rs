pub mod llm;
pub mod prompt;

use crate::parser::mcq::{McqOption, McqQuestion};
use anyhow::{bail, Context, Result};
use std::path::Path;

// --------------------------------------------------------------------------
// Public config
// --------------------------------------------------------------------------

/// Options for MCQ generation.
pub struct GenerateConfig {
    /// Anthropic API key (read from `ANTHROPIC_API_KEY` env var if not set).
    pub api_key: String,
    /// Claude model to use (default: `claude-opus-4-6`).
    pub model: String,
    /// Number of MCQs to generate.
    pub count: usize,
    /// Optional topic/chapter hint passed to the prompt.
    pub topic: Option<String>,
    /// Max tokens the LLM may output. Increase for large batches.
    pub max_tokens: u32,
    /// Max characters of source text to feed to the LLM.
    /// Long textbooks are chunked; each chunk produces `count` MCQs.
    pub chunk_chars: usize,
}

impl Default for GenerateConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: llm::DEFAULT_MODEL.to_string(),
            count: 10,
            topic: None,
            max_tokens: 8192,
            chunk_chars: 12_000, // ~3 K tokens of source text
        }
    }
}

// --------------------------------------------------------------------------
// Entry points
// --------------------------------------------------------------------------

/// Generate MCQs by extracting text from a file first.
pub fn generate_from_file(path: &Path, cfg: &GenerateConfig) -> Result<Vec<McqQuestion>> {
    let text = crate::extract_text(path)?;
    generate_from_text(&text, cfg)
}

/// Generate MCQs from raw text.
///
/// If the text is longer than `cfg.chunk_chars`, it is split into overlapping
/// chunks so the full document is covered.
pub fn generate_from_text(text: &str, cfg: &GenerateConfig) -> Result<Vec<McqQuestion>> {
    validate_config(cfg)?;

    let chunks = chunk_text(text, cfg.chunk_chars);
    let mut all: Vec<McqQuestion> = Vec::new();
    let mut offset = all.len();

    for chunk in &chunks {
        let user_msg = prompt::user_prompt(chunk, cfg.count, cfg.topic.as_deref());

        let llm_cfg = llm::LlmConfig {
            api_key: &cfg.api_key,
            model: &cfg.model,
            max_tokens: cfg.max_tokens,
            system: prompt::system_prompt(),
            user_message: &user_msg,
        };

        let raw = llm::call(&llm_cfg).context("LLM call failed")?;
        let mut questions = parse_llm_response(&raw, offset)?;

        // Re-number so question numbers are globally unique.
        for q in &mut questions {
            q.number += offset as u32;
        }

        offset += questions.len();
        all.extend(questions);
    }

    Ok(all)
}

// --------------------------------------------------------------------------
// Internal helpers
// --------------------------------------------------------------------------

fn validate_config(cfg: &GenerateConfig) -> Result<()> {
    if cfg.api_key.is_empty() {
        bail!(
            "Anthropic API key is required. \
             Set the ANTHROPIC_API_KEY environment variable or pass --api-key."
        );
    }
    if cfg.count == 0 {
        bail!("--count must be at least 1");
    }
    Ok(())
}

/// Split `text` into overlapping chunks of at most `chunk_chars` characters.
/// A 10 % overlap is kept so questions aren't generated only from half a sentence.
fn chunk_text(text: &str, chunk_chars: usize) -> Vec<String> {
    if text.len() <= chunk_chars {
        return vec![text.to_string()];
    }

    let overlap = chunk_chars / 10;
    let step = chunk_chars - overlap;
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < chars.len() {
        let end = (start + chunk_chars).min(chars.len());
        chunks.push(chars[start..end].iter().collect());
        if end == chars.len() {
            break;
        }
        start += step;
    }

    chunks
}

/// Parse the LLM JSON response into `McqQuestion` structs.
fn parse_llm_response(raw: &str, number_offset: usize) -> Result<Vec<McqQuestion>> {
    let value = llm::parse_response(raw)?;

    let arr = value
        .get("questions")
        .and_then(|v| v.as_array())
        .with_context(|| format!("Expected a JSON object with a 'questions' array. Got:\n{raw}"))?;

    let mut questions = Vec::new();

    for (idx, item) in arr.iter().enumerate() {
        let number = item
            .get("number")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
            .unwrap_or((number_offset + idx + 1) as u32);

        let stem = item
            .get("stem")
            .and_then(|v| v.as_str())
            .unwrap_or("(missing stem)")
            .to_string();

        let answer = item
            .get("answer")
            .and_then(|v| v.as_str())
            .map(|s| s.to_uppercase());

        let options = item
            .get("options")
            .and_then(|v| v.as_array())
            .map(|opts| {
                opts.iter()
                    .filter_map(|o| {
                        let label = o.get("label")?.as_str()?.to_uppercase();
                        let text = o.get("text")?.as_str()?.to_string();
                        // Mark the correct option so the answer is embedded in the options.
                        let is_correct = answer.as_deref() == Some(label.as_str());
                        Some(McqOption { label, text, is_correct })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        questions.push(McqQuestion {
            number,
            stem,
            options,
        });
    }

    Ok(questions)
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_short_text_unchanged() {
        let text = "Hello world";
        let chunks = chunk_text(text, 1000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn chunk_long_text_produces_multiple_chunks() {
        let text = "a".repeat(10_000);
        let chunks = chunk_text(&text, 1_000);
        assert!(chunks.len() > 1);
        // Every chunk must be ≤ chunk_chars characters.
        for c in &chunks {
            assert!(c.len() <= 1_000);
        }
    }

    #[test]
    fn parse_llm_response_ok() {
        let json = r#"{
            "questions": [{
                "number": 1,
                "stem": "What is Rust?",
                "options": [
                    {"label": "A", "text": "A programming language"},
                    {"label": "B", "text": "A metal oxide"},
                    {"label": "C", "text": "A database"},
                    {"label": "D", "text": "A web browser"}
                ],
                "answer": "A",
                "difficulty": "easy"
            }]
        }"#;

        let qs = parse_llm_response(json, 0).unwrap();
        assert_eq!(qs.len(), 1);
        assert_eq!(qs[0].stem, "What is Rust?");
        // Correct answer is now embedded in the option with is_correct: true.
        let correct = qs[0].options.iter().find(|o| o.is_correct);
        assert_eq!(correct.map(|o| o.label.as_str()), Some("A"));
    }
}
