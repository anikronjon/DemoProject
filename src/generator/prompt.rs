/// Build the system prompt for MCQ generation.
pub fn system_prompt() -> &'static str {
    r#"You are an expert educator and assessment designer. Your task is to generate
high-quality Multiple-Choice Questions (MCQs) from textbook content.

Rules:
- Each question must have exactly 4 options labeled A, B, C, D.
- Exactly one option must be correct.
- Distractors (wrong options) must be plausible but clearly incorrect to someone who
  understands the material.
- Questions should test comprehension and reasoning, not just recall of exact sentences.
- Vary difficulty across the set: approximately 40% easy, 40% medium, 20% hard.
- Do NOT include the answer rationale in the question text.
- Return ONLY valid JSON — no markdown fences, no commentary outside the JSON."#
}

/// Build the user prompt for MCQ generation from raw text.
///
/// Parameters:
/// - `text`  : the extracted textbook content
/// - `count` : number of MCQs to generate
/// - `topic` : optional topic hint (e.g. "photosynthesis", "chapter 3")
pub fn user_prompt(text: &str, count: usize, topic: Option<&str>) -> String {
    let topic_hint = topic
        .map(|t| format!(" Focus specifically on the topic: \"{t}\"."))
        .unwrap_or_default();

    format!(
        r#"Generate exactly {count} MCQs from the following textbook content.{topic_hint}

Return a JSON object with a single key "questions" whose value is an array of objects.
Each object must follow this exact schema:
{{
  "number": <integer, 1-based>,
  "stem": "<question text>",
  "options": [
    {{"label": "A", "text": "<option text>", "is_correct": false}},
    {{"label": "B", "text": "<option text>", "is_correct": true}},
    {{"label": "C", "text": "<option text>", "is_correct": false}},
    {{"label": "D", "text": "<option text>", "is_correct": false}}
  ],
  "answer": "<correct label: A | B | C | D>",
  "difficulty": "<easy | medium | hard>"
}}

TEXTBOOK CONTENT:
---
{text}
---

Respond with the JSON object only."#
    )
}
