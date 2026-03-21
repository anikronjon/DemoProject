use crate::ExtractionResult;
use anyhow::{Context, Result};
use std::path::Path;

/// Serialize the extraction result to a pretty-printed JSON file.
pub fn write(result: &ExtractionResult, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(result)
        .context("Failed to serialise result to JSON")?;

    std::fs::write(output_path, json)
        .with_context(|| format!("Failed to write JSON to {}", output_path.display()))?;

    Ok(())
}

/// Return the extraction result as a JSON string (for printing to stdout).
pub fn to_string(result: &ExtractionResult) -> Result<String> {
    serde_json::to_string_pretty(result).context("Failed to serialise result to JSON")
}
