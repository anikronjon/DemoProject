use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// A comprehension / creative / short-answer question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CqQuestion {
    /// Sequential number from the source text.
    pub number: u32,
    /// The full question text (may be multi-line).
    pub stem: String,
    /// Sub-parts, e.g. (a), (b), (i), (ii), if present.
    pub parts: Vec<CqPart>,
    /// Marks allocated, if stated (e.g. "[5 marks]").
    pub marks: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CqPart {
    pub label: String,
    pub text: String,
    pub marks: Option<u32>,
}

// --------------------------------------------------------------------------
// Regexes
// --------------------------------------------------------------------------

fn question_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // A numbered line that does NOT look like an MCQ stem (no A/B/C/D options follow).
        // We simply match numbered lines; the MCQ parser filters its own out first.
        Regex::new(r"(?mi)^(?:Q\.?\s*)?(\d+)[.)]\s+(.+)$").unwrap()
    })
}

/// Matches sub-part labels like (a), (b), (i), (ii), a), b)
fn part_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?mi)^\(([a-z]{1,3}|[ivxlc]+)\)\s+(.+)$").unwrap()
    })
}

/// Matches marks annotations like [5], [5 marks], (5 marks)
fn marks_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[\[(](\d+)\s*(?:marks?)?[\])]").unwrap()
    })
}

// --------------------------------------------------------------------------
// Parser
// --------------------------------------------------------------------------

/// Parse all CQs from `text`.
///
/// A question block is treated as a CQ when:
/// - It matches the numbered question pattern, AND
/// - It has fewer than 2 A–D option lines in the following 8 lines
///   (i.e. was not already claimed by the MCQ parser).
pub fn parse(text: &str) -> Vec<CqQuestion> {
    let lines: Vec<&str> = text.lines().collect();
    let mut questions = Vec::new();

    let q_re = question_re();
    let part_re = part_re();
    let marks_re = marks_re();
    // Only count bare-letter options like "A. text" or "A) text".
    // Parenthesised forms "(a) text" are CQ sub-parts, not MCQ options.
    let opt_re =
        Regex::new(r"(?mi)^([A-Da-d])[.)]\s+(.+)$").unwrap();

    let mut i = 0;
    while i < lines.len() {
        if let Some(caps) = q_re.captures(lines[i]) {
            let number: u32 = caps[1].parse().unwrap_or(0);
            let raw_stem = caps[2].trim().to_string();

            // Peek ahead to decide MCQ vs CQ.
            let mut opt_count = 0;
            let mut j = i + 1;
            while j < lines.len() && j <= i + 8 {
                let line = lines[j].trim();
                if !line.is_empty() && q_re.is_match(line) {
                    break;
                }
                if opt_re.is_match(line) {
                    opt_count += 1;
                }
                j += 1;
            }

            if opt_count >= 2 {
                // Let MCQ parser handle this block.
                i += 1;
                continue;
            }

            // It's a CQ — gather stem continuation lines + sub-parts.
            let marks = extract_marks(&raw_stem, &marks_re);
            let stem = strip_marks(&raw_stem, &marks_re);
            let mut parts = Vec::new();

            let mut k = i + 1;
            while k < lines.len() {
                let line = lines[k].trim();
                if line.is_empty() {
                    k += 1;
                    // Two consecutive blank lines → end of question block.
                    if k < lines.len() && lines[k].trim().is_empty() {
                        break;
                    }
                    continue;
                }
                // New top-level question → stop.
                if q_re.is_match(line) {
                    break;
                }
                if let Some(pcaps) = part_re.captures(line) {
                    let label = pcaps[1].to_string();
                    let part_text_raw = pcaps[2].trim().to_string();
                    let part_marks = extract_marks(&part_text_raw, &marks_re);
                    let part_text = strip_marks(&part_text_raw, &marks_re);
                    parts.push(CqPart {
                        label,
                        text: part_text,
                        marks: part_marks,
                    });
                }
                k += 1;
            }

            questions.push(CqQuestion {
                number,
                stem,
                parts,
                marks,
            });
            i = k;
            continue;
        }
        i += 1;
    }

    questions
}

fn extract_marks(text: &str, re: &Regex) -> Option<u32> {
    re.captures(text)
        .and_then(|c| c[1].parse().ok())
}

fn strip_marks(text: &str, re: &Regex) -> String {
    re.replace_all(text, "").trim().to_string()
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
1. Describe the water cycle. [10 marks]
(a) What is evaporation?
(b) Explain condensation.

2. Write a short essay on climate change. [15 marks]
"#;

    #[test]
    fn parses_two_cqs() {
        let qs = parse(SAMPLE);
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].number, 1);
        assert_eq!(qs[0].parts.len(), 2);
        assert_eq!(qs[0].marks, Some(10));
    }

    #[test]
    fn marks_stripped_from_stem() {
        let qs = parse(SAMPLE);
        assert!(!qs[0].stem.contains("[10"));
    }
}
