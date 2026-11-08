pub mod build;
pub mod docx;
pub mod model;
pub mod pdf;
pub mod xlsx;

use crate::error::AppResult;

use self::build::ReportSource;
use self::model::{DocumentFormat, DocumentKind};

pub fn render(
    kind: DocumentKind,
    format: DocumentFormat,
    source: &ReportSource<'_>,
) -> AppResult<Vec<u8>> {
    match format {
        DocumentFormat::Pdf => pdf::render_pdf(&build::build_report(kind, source)),
        DocumentFormat::Docx => docx::render_docx(&build::build_report(kind, source)),
        DocumentFormat::Xlsx => xlsx::render_xlsx(kind, source),
    }
}
