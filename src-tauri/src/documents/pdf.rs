use std::path::PathBuf;

use chrono::Datelike;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::error::{AppError, AppResult};

use super::model::{ReportBlock, ReportDocument};

pub fn render_pdf(report: &ReportDocument) -> AppResult<Vec<u8>> {
    let source = render_source(report);
    let world = TypstWorld::new(source)?;
    let warned = typst::compile::<PagedDocument>(&world);
    let document = warned.output.map_err(|errors| {
        AppError::Internal(format!(
            "PDF generation failed: {}",
            format_diagnostics(&errors)
        ))
    })?;
    typst_pdf::pdf(&document, &PdfOptions::default())
        .map_err(|errors| AppError::Internal(format!("PDF export failed: {errors:?}")))
}

fn render_source(report: &ReportDocument) -> String {
    let mut out = String::new();
    out.push_str("#set page(paper: \"a4\", margin: (x: 1.9cm, y: 2cm))\n");
    out.push_str("#set text(font: \"Libertinus Serif\", size: 10pt, fill: rgb(\"#1e293b\"))\n");
    out.push_str("#set heading(numbering: \"1.\")\n");
    out.push_str("#show heading: set text(fill: rgb(\"#0f766e\"))\n");
    out.push_str("#set par(justify: true)\n");
    out.push_str("#rect(width: 100%, height: 0.35cm, fill: rgb(\"#0f172a\"))[]\n");
    out.push_str("#v(0.8em)\n");
    out.push_str(&format!(
        "#align(center)[#text(size: 22pt, weight: \"bold\", fill: rgb(\"#0f172a\"))[{}]]\n",
        escape(&report.title)
    ));
    out.push_str(&format!(
        "#align(center)[#text(size: 11pt, fill: rgb(\"#0f766e\"))[{}]]\n",
        escape(&report.subtitle)
    ));
    out.push_str("#v(0.6em)\n#align(center)[#rect(width: 45%, height: 0.1cm, fill: rgb(\"#10b981\"))[]]\n#v(1em)\n");
    if !report.metadata.is_empty() {
        out.push_str("#table(columns: 2, inset: 5pt, stroke: 0.5pt,\n");
        for (key, value) in &report.metadata {
            out.push_str(&format!("[{}], [{}],\n", escape(key), escape(value)));
        }
        out.push_str(")\n");
    }

    for section in &report.sections {
        out.push_str("#pagebreak(weak: true)\n");
        out.push_str(&format!("= {}\n", escape(&section.heading)));
        for block in &section.blocks {
            match block {
                ReportBlock::Paragraph(text) => {
                    out.push_str(&format!("{}\n\n", escape(text)));
                }
                ReportBlock::Bullets(items) => {
                    for item in items {
                        out.push_str(&format!("- {}\n", escape(item)));
                    }
                    out.push('\n');
                }
                ReportBlock::Table { headers, rows } => {
                    out.push_str(&render_table(headers, rows));
                    out.push('\n');
                }
            }
        }
    }
    out
}

fn render_table(headers: &[String], rows: &[Vec<String>]) -> String {
    let columns = headers.len().max(1);
    let mut out = String::from("#text(size: 8.5pt)[\n#table(\n");
    out.push_str(&format!(
        "  columns: {columns},\n  inset: 4pt,\n  stroke: 0.5pt,\n"
    ));
    out.push_str("  table.header(");
    for header in headers {
        out.push_str(&format!("[{}], ", escape(header)));
    }
    out.push_str("),\n");
    for row in rows {
        for cell in row {
            out.push_str(&format!("  [{}],\n", escape(cell)));
        }
    }
    out.push_str(")\n]\n");
    out
}

fn escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' | '#' | '$' | '*' | '_' | '[' | ']' | '<' | '>' | '@' | '`' | '~' | '-' | '+' => {
                out.push('\\');
                out.push(ch);
            }
            '\n' | '\r' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

fn format_diagnostics(errors: &[SourceDiagnostic]) -> String {
    errors
        .iter()
        .map(|error| error.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

struct TypstWorld {
    library: LazyHash<Library>,
    fonts: FontStore,
    main: FileId,
    source: Source,
}

impl TypstWorld {
    fn new(text: String) -> AppResult<Self> {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::embedded());
        let vpath = VirtualPath::new("main.typ")
            .map_err(|_| AppError::Internal("invalid document path".into()))?;
        let main = FileId::unique(RootedPath::new(VirtualRoot::Project, vpath));
        let source = Source::new(main, text);
        Ok(Self {
            library: LazyHash::new(Library::default()),
            fonts,
            main,
            source,
        })
    }
}

impl World for TypstWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(PathBuf::from("missing.typ")))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main {
            Ok(Bytes::from_string(self.source.text().to_string()))
        } else {
            Err(FileError::NotFound(PathBuf::from("missing.typ")))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        let now = chrono::Utc::now();
        Datetime::from_ymd(now.year(), now.month() as u8, now.day() as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::model::{ReportBlock, ReportSection};

    #[test]
    fn escapes_typst_markup() {
        let escaped = escape("# heading [test] *bold*");
        assert!(escaped.contains("\\#"));
        assert!(escaped.contains("\\["));
        assert!(escaped.contains("\\*"));
    }

    #[test]
    fn renders_pdf_bytes() {
        let report = ReportDocument {
            title: "Synthetic report".into(),
            subtitle: "Client - 2026".into(),
            metadata: vec![("Status".into(), "Active".into())],
            sections: vec![ReportSection {
                heading: "Overview".into(),
                blocks: vec![ReportBlock::Paragraph("Synthetic paragraph.".into())],
            }],
        };
        let pdf = render_pdf(&report).expect("pdf");
        assert!(pdf.starts_with(b"%PDF"));
        assert!(pdf.len() > 1000);
    }
}
