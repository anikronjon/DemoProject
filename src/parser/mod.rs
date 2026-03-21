pub mod cq;
pub mod mcq;

use serde::{Deserialize, Serialize};

/// A single parsed question from the source document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum Question {
    /// Multiple-Choice Question
    Mcq(mcq::McqQuestion),
    /// Comprehension / Creative / Short-answer Question
    Cq(cq::CqQuestion),
}

impl Question {
    pub fn number(&self) -> u32 {
        match self {
            Question::Mcq(q) => q.number,
            Question::Cq(q) => q.number,
        }
    }

    pub fn stem(&self) -> &str {
        match self {
            Question::Mcq(q) => &q.stem,
            Question::Cq(q) => &q.stem,
        }
    }
}

/// Parse raw text into a list of `Question`s (MCQ first, then CQ).
/// Questions are returned in the order they appear in the source text.
pub fn parse_questions(text: &str) -> Vec<Question> {
    let mut questions: Vec<Question> = Vec::new();

    let mcqs = mcq::parse(text);
    let cqs = cq::parse(text);

    for q in mcqs {
        questions.push(Question::Mcq(q));
    }
    for q in cqs {
        questions.push(Question::Cq(q));
    }

    // Sort by question number so the output is in document order.
    questions.sort_by_key(|q| q.number());
    questions
}
