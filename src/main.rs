use anyhow::Result;
use clap::{Parser, Subcommand};
use ocr_question_extractor::output::{self, OutputFormat};
use std::path::PathBuf;

// --------------------------------------------------------------------------
// CLI definition
// --------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "oqe",
    version,
    about = "OCR Question Extractor — pull MCQ & CQ from PDFs and images",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract questions from a PDF or image file.
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

            if stdout {
                match format {
                    OutputFormat::Json => {
                        let s = output::json::to_string(&result)?;
                        println!("{}", s);
                    }
                    OutputFormat::Txt => {
                        // Write to a temp path then read back.
                        let tmp = tempfile_path(&input, "txt");
                        output::write(&result, &tmp, format)?;
                        print!("{}", std::fs::read_to_string(&tmp)?);
                        let _ = std::fs::remove_file(&tmp);
                    }
                    OutputFormat::Docx => {
                        anyhow::bail!(
                            "DOCX output cannot be printed to stdout. \
                             Use --output to specify a file."
                        );
                    }
                }
            } else {
                let out_path = output.unwrap_or_else(|| tempfile_path(&input, &format.to_string()));
                output::write(&result, &out_path, format)?;
                eprintln!("Output written to '{}'", out_path.display());
            }
        }
    }

    Ok(())
}

/// Derive a sibling output path from the input path by replacing its extension.
fn tempfile_path(input: &PathBuf, ext: &str) -> PathBuf {
    input.with_extension(ext)
}
