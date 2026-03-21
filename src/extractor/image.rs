use anyhow::Result;
use std::path::Path;

/// Extract text from an image file.
///
/// When compiled with the `ocr` feature, Tesseract is used (requires system
/// packages `libtesseract-dev`, `libleptonica-dev`, and `tesseract-ocr`).
///
/// Without the `ocr` feature this function always returns an error explaining
/// how to enable it, so the rest of the codebase still compiles cleanly.
pub fn extract(path: &Path) -> Result<String> {
    #[cfg(feature = "ocr")]
    {
        extract_with_tesseract(path)
    }
    #[cfg(not(feature = "ocr"))]
    {
        let _ = path;
        anyhow::bail!(
            "Image OCR is disabled. Recompile with `--features ocr` \
             (requires libtesseract-dev + libleptonica-dev on the host system)."
        )
    }
}

// --------------------------------------------------------------------------
// Tesseract implementation (feature = "ocr")
// --------------------------------------------------------------------------

#[cfg(feature = "ocr")]
fn extract_with_tesseract(path: &Path) -> Result<String> {
    use leptess::LepTess;

    // Pre-process: convert to greyscale for better OCR accuracy.
    let grey = preprocess(path)
        .with_context(|| format!("Image pre-processing failed: {}", path.display()))?;

    let tmp = save_temp_png(&grey)?;

    let mut lt = LepTess::new(None, "eng")
        .map_err(|e| anyhow::anyhow!("Tesseract init failed: {}", e))?;

    lt.set_image(tmp.path())
        .map_err(|e| anyhow::anyhow!("Tesseract set_image failed: {}", e))?;

    let text = lt
        .get_utf8_text()
        .map_err(|e| anyhow::anyhow!("Tesseract OCR failed: {}", e))?;

    Ok(text.trim().to_string())
}

#[cfg(feature = "ocr")]
fn preprocess(path: &Path) -> Result<image::GrayImage> {
    let img = image::open(path)
        .with_context(|| format!("Cannot open image: {}", path.display()))?;
    Ok(img.into_luma8())
}

#[cfg(feature = "ocr")]
fn save_temp_png(img: &image::GrayImage) -> Result<TempPng> {
    TempPng::from_gray_image(img)
}

/// Scoped wrapper keeping a NamedTempFile alive while Tesseract reads it.
#[cfg(feature = "ocr")]
struct TempPng {
    _file: tempfile::NamedTempFile,
    path: std::path::PathBuf,
}

#[cfg(feature = "ocr")]
impl TempPng {
    fn from_gray_image(img: &image::GrayImage) -> Result<Self> {
        let file = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(|e| anyhow::anyhow!("Cannot create temp file: {}", e))?;
        img.save(file.path())
            .map_err(|e| anyhow::anyhow!("Cannot save temp PNG: {}", e))?;
        let path = file.path().to_path_buf();
        Ok(Self { _file: file, path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}
