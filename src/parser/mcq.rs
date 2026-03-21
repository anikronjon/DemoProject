use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A multiple-choice question with up to 4 options and an optional answer key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McqQuestion {
    /// Sequential number extracted from the source text.
    pub number: u32,
    /// The question stem (the actual question text).
    pub stem: String,
    /// Answer options keyed by label (A, B, C, D).
    pub options: Vec<McqOption>,
    /// Correct answer label, if present in the source (e.g. from an answer key).
    pub answer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McqOption {
    /// Option label: "A", "B", "C", or "D".
    pub label: String,
    /// Option text.
    pub text: String,
}

// --------------------------------------------------------------------------
// Regexes
// --------------------------------------------------------------------------

/// Matches question stems like:
///   "1. Which of the following..."
///   "1) Which of the following..."
///   "Q1. Which of the following..."
fn question_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?mi)^(?:Q\.?\s*)?(\d+)[.)]\s+(.+)$").unwrap()
    })
}

/// Matches option lines like:
///   "A. option text"
///   "(A) option text"
///   "A) option text"
///   "a. option text"
fn option_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?mi)^\(?([A-Da-d])[.)]\)?\s+(.+)$").unwrap()
    })
}

// --------------------------------------------------------------------------
// Parser
// --------------------------------------------------------------------------

/// Parse all MCQs from `text`.
///
/// Strategy:
/// 1. Find every line that looks like a numbered question stem.
/// 2. Collect the lines immediately following it for option detection.
/// 3. A block is considered an MCQ if it contains at least two A–D options.
pub fn parse(text: &str) -> Vec<McqQuestion> {
    let lines: Vec<&str> = text.lines().collect();
    let mut questions = Vec::new();

    let q_re = question_re();
    let opt_re = option_re();

    let mut i = 0;
    while i < lines.len() {
        if let Some(caps) = q_re.captures(lines[i]) {
            let number: u32 = caps[1].parse().unwrap_or(0);
            let stem = caps[2].trim().to_string();

            // Collect up to 8 following lines as candidate option lines.
            let mut options = Vec::new();
            let mut j = i + 1;
            while j < lines.len() && j <= i + 8 {
                let line = lines[j].trim();
                if line.is_empty() {
                    j += 1;
                    continue;
                }
                // Stop if we hit another question stem.
                if q_re.is_match(line) {
                    break;
                }
                if let Some(ocaps) = opt_re.captures(line) {
                    let label = ocaps[1].to_uppercase();
                    let text = ocaps[2].trim().to_string();
                    options.push(McqOption { label, text });
                }
                j += 1;
            }

            // Only treat as MCQ if there are at least 2 options.
            if options.len() >= 2 {
                questions.push(McqQuestion {
                    number,
                    stem,
                    options,
                    answer: None,
                });
                i = j;
                continue;
            }
        }
        i += 1;
    }

    questions
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
1. What is the capital of France?
A. Berlin
B. Madrid
C. Paris
D. Rome

2. Which planet is closest to the Sun?
A. Earth
B. Mercury
C. Venus
D. Mars
"#;

    #[test]
    fn parses_two_mcqs() {
        let qs = parse(SAMPLE);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].number, 1);
        assert_eq!(qs[0].options.len(), 4);
        assert_eq!(qs[1].stem, "Which planet is closest to the Sun?");
    }

    #[test]
    fn option_labels_are_uppercase() {
        let text = "1. Question?\na. opt1\nb. opt2\nc. opt3\nd. opt4\n";
        let qs = parse(text);
        assert_eq!(qs[0].options[0].label, "A");
    }
}
