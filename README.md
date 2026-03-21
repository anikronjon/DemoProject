# OCR Question Extractor (`oqe`)

A Rust CLI tool that extracts **MCQ** (Multiple-Choice Questions) and **CQ**
(Comprehension / Short-answer Questions) from PDF and image files, then outputs
the structured data as **JSON**, **DOCX**, or plain **TXT**.

---

## Features

| Capability | Status |
|---|---|
| PDF text extraction | ✅ |
| Image OCR (Tesseract) | ✅ *(requires `ocr` feature + system libs)* |
| MCQ detection & parsing | ✅ |
| CQ detection & sub-part parsing | ✅ |
| JSON output | ✅ |
| DOCX output | ✅ |
| TXT output | ✅ |
| Generate MCQs from PDF textbooks | 🔜 planned |

---

## Installation

### System requirements

```bash
# PDF support only (no OCR)
cargo build --release

# PDF + Image OCR support
sudo apt install libtesseract-dev libleptonica-dev tesseract-ocr
cargo build --release --features ocr
```

### Run from source

```bash
cargo run --bin oqe -- --help
```

---

## Usage

### Extract questions from a PDF → JSON (default)

```bash
oqe extract exam.pdf
# Output: exam.json
```

### Extract to DOCX

```bash
oqe extract exam.pdf --format docx --output result.docx
```

### Extract from an image (OCR)

```bash
# Requires --features ocr at build time
oqe extract scan.png --format json
```

### Print raw extracted text

```bash
oqe text exam.pdf
# or
oqe extract exam.pdf --raw
```

### Print JSON to stdout

```bash
oqe extract exam.pdf --stdout
```

---

## JSON output format

```json
{
  "source": "exam.pdf",
  "raw_text": "...",
  "questions": [
    {
      "type": "MCQ",
      "number": 1,
      "stem": "What is the capital of France?",
      "options": [
        { "label": "A", "text": "Berlin" },
        { "label": "B", "text": "Paris" }
      ],
      "answer": null
    },
    {
      "type": "CQ",
      "number": 2,
      "stem": "Describe the water cycle.",
      "marks": 10,
      "parts": [
        { "label": "a", "text": "What is evaporation?", "marks": 5 },
        { "label": "b", "text": "Explain condensation.", "marks": 5 }
      ]
    }
  ]
}
```

---

## Project structure

```
src/
├── main.rs              # CLI entry point (oqe binary)
├── lib.rs               # Public API
├── extractor/
│   ├── pdf.rs           # PDF text extraction (pdf-extract)
│   └── image.rs         # Image OCR via Tesseract (feature: ocr)
├── parser/
│   ├── mcq.rs           # MCQ detection & parsing
│   └── cq.rs            # CQ detection & parsing
└── output/
    ├── json.rs           # JSON formatter
    ├── docx.rs           # DOCX formatter
    └── mod.rs            # TXT formatter + format dispatch
```

---

## Roadmap

- [ ] Auto-generate MCQs from PDF textbook chapters (LLM integration)
- [ ] Answer key extraction from answer sheets
- [ ] Batch processing of multiple files
- [ ] Confidence scores for parsed questions
- [ ] Web API / REST server mode
