use anyhow::Result;
use clap::{Parser, Subcommand};
use ocr_question_extractor::{
    generator::{GenerateConfig, generate_from_file, generate_from_text},
    output::{self, OutputFormat},
    parser::{mcq::McqQuestion, Question},
    ExtractionResult,
};
use std::path::PathBuf;

// --------------------------------------------------------------------------
// CLI definition
// --------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "oqe",
    version,
    about = "OCR Question Extractor — pull MCQ & CQ from PDFs and images, and auto-generate MCQs",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract questions from a PDF or image file (regex-based parser).
    Extract {
        /// Input file (PDF, PNG, JPG, TIFF, …).
        #[arg(value_name = "FILE")]
        input: PathBuf,

        /// Output file path (default: <input>.<format>).
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Print raw extracted text and exit (no parsing).
        #[arg(long)]
        raw: bool,

        /// Print result to stdout instead of writing a file.
        #[arg(long)]
        stdout: bool,
    },

    /// Auto-generate MCQs from a PDF or image using Claude AI.
    Generate {
        /// Input file (PDF, PNG, JPG, …) or pass --text for raw text.
        #[arg(value_name = "FILE", conflicts_with = "text")]
        input: Option<PathBuf>,

        /// Raw text to generate MCQs from (alternative to FILE).
        #[arg(long, value_name = "TEXT", conflicts_with = "input")]
        text: Option<String>,

        /// Number of MCQs to generate (per chunk for long documents).
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,

        /// Topic or chapter hint (e.g. "photosynthesis", "chapter 3").
        #[arg(short, long, value_name = "TOPIC")]
        topic: Option<String>,

        /// Claude model to use.
        #[arg(long, default_value = "claude-opus-4-6")]
        model: String,

        /// Max tokens for the LLM response.
        #[arg(long, default_value = "8192")]
        max_tokens: u32,

        /// Anthropic API key (defaults to ANTHROPIC_API_KEY env var).
        #[arg(long, env = "ANTHROPIC_API_KEY", value_name = "KEY")]
        api_key: String,

        /// Output file path (default: <input>.json or generated.json).
        #[arg(short, long, value_name = "OUTPUT")]
        output: Option<PathBuf>,

        /// Output format.
        #[arg(short, long, value_enum, default_value = "json")]
        format: OutputFormat,

        /// Print result to stdout instead of writing a file.
        #[arg(long)]
        stdout: bool,
    },

    /// Print raw extracted text from a file (shortcut for `extract --raw`).
    Text {
        /// Input file.
        #[arg(value_name = "FILE")]
        input: PathBuf,
    },
}

// --------------------------------------------------------------------------
// Entry point
// --------------------------------------------------------------------------

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Text { input } => {
            let text = ocr_question_extractor::extract_text(&input)?;
            println!("{}", text);
        }

        Commands::Extract {
            input,
            output,
            format,
            raw,
            stdout,
        } => {
            if raw {
                let text = ocr_question_extractor::extract_text(&input)?;
                println!("{}", text);
                return Ok(());
            }

            let result = ocr_question_extractor::process_file(&input)?;
            eprintln!(
                "Extracted {} question(s) from '{}'",
                result.questions.len(),
                result.source
            );
            write_or_print(result, output, format, stdout, &input)?;
        }

        Commands::Generate {
            input,
            text,
            count,
            topic,
            model,
            max_tokens,
            api_key,
            output,
            format,
            stdout,
        } => {
            let cfg = GenerateConfig {
                api_key,
                model,
                count,
                topic,
                max_tokens,
                ..Default::default()
            };

            eprintln!("Calling Claude ({}) to generate {} MCQ(s)…", cfg.model, cfg.count);

            let questions: Vec<McqQuestion> = if let Some(path) = &input {
                generate_from_file(path, &cfg)?
            } else if let Some(raw_text) = &text {
                generate_from_text(raw_text, &cfg)?
            } else {
                anyhow::bail!("Provide either a FILE or --text");
            };

            eprintln!("Generated {} MCQ(s).", questions.len());

            // Wrap generated MCQs in an ExtractionResult for uniform output.
            let source = input
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<inline text>".into());

            let result = ExtractionResult {
                source,
                raw_text: text.unwrap_or_default(),
                questions: questions.into_iter().map(Question::Mcq).collect(),
            };

            let default_out = input
                .as_ref()
                .map(|p| output_path(p, &format.to_string()))
                .unwrap_or_else(|| PathBuf::from(format!("generated.{format}")));

            write_or_print(result, output.or(Some(default_out)), format, stdout, &PathBuf::from("generated"))?;
        }
    }

    Ok(())
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn write_or_print(
    result: ExtractionResult,
    out: Option<PathBuf>,
    format: OutputFormat,
    stdout: bool,
    input: &PathBuf,
) -> Result<()> {
    if stdout {
        match format {
            OutputFormat::Json => {
                println!("{}", output::json::to_string(&result)?);
            }
            OutputFormat::Txt => {
                let tmp = output_path(input, "txt");
                output::write(&result, &tmp, format)?;
                print!("{}", std::fs::read_to_string(&tmp)?);
                let _ = std::fs::remove_file(&tmp);
            }
            OutputFormat::Docx => {
                anyhow::bail!(
                    "DOCX cannot be printed to stdout. Use --output to specify a file."
                );
            }
        }
    } else {
        let out_path = out.unwrap_or_else(|| output_path(input, &format.to_string()));
        output::write(&result, &out_path, format)?;
        eprintln!("Output written to '{}'", out_path.display());
    }
    Ok(())
}

fn output_path(input: &PathBuf, ext: &str) -> PathBuf {
    input.with_extension(ext)
}
