use std::io::Cursor;

use docx_rs::{AlignmentType, Docx, Paragraph, Run, Table, TableCell, TableRow, WidthType};

use crate::error::{AppError, AppResult};

use super::model::{ReportBlock, ReportDocument};

pub fn render_docx(report: &ReportDocument) -> AppResult<Vec<u8>> {
    let mut doc = Docx::new()
        .page_size(11906, 16838)
        .add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_text(&report.title).bold().size(36)),
        )
        .add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_text(&report.subtitle).size(22)),
        );

    if !report.metadata.is_empty() {
        let rows = report
            .metadata
            .iter()
            .map(|(key, value)| TableRow::new(vec![cell(key, true), cell(value, false)]))
            .collect();
        doc = doc.add_table(Table::new(rows).width(9000, WidthType::Dxa));
    }

    for section in &report.sections {
        doc = doc.add_paragraph(
            Paragraph::new().add_run(Run::new().add_text(&section.heading).bold().size(28)),
        );
        for block in &section.blocks {
            match block {
                ReportBlock::Paragraph(text) => {
                    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(text)));
                }
                ReportBlock::Bullets(items) => {
                    for item in items {
                        doc = doc.add_paragraph(
                            Paragraph::new().add_run(Run::new().add_text(format!("• {item}"))),
                        );
                    }
                }
                ReportBlock::Table { headers, rows } => {
                    let mut table_rows = vec![TableRow::new(
                        headers.iter().map(|header| cell(header, true)).collect(),
                    )];
                    for row in rows {
                        table_rows.push(TableRow::new(
                            row.iter().map(|value| cell(value, false)).collect(),
                        ));
                    }
                    doc = doc.add_table(Table::new(table_rows).width(9000, WidthType::Dxa));
                }
            }
        }
    }

    let mut cursor = Cursor::new(Vec::new());
    doc.build()
        .pack(&mut cursor)
        .map_err(|error| AppError::Internal(format!("DOCX generation failed: {error}")))?;
    Ok(cursor.into_inner())
}

fn cell(text: &str, bold: bool) -> TableCell {
    let mut run = Run::new().add_text(text);
    if bold {
        run = run.bold();
    }
    TableCell::new().add_paragraph(Paragraph::new().add_run(run))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::documents::model::{ReportBlock, ReportSection};

    #[test]
    fn renders_docx_zip() {
        let report = ReportDocument {
            title: "Synthetic report".into(),
            subtitle: "Client - 2026".into(),
            metadata: vec![("Status".into(), "Active".into())],
            sections: vec![ReportSection {
                heading: "Overview".into(),
                blocks: vec![
                    ReportBlock::Paragraph("Synthetic paragraph.".into()),
                    ReportBlock::Bullets(vec!["One".into(), "Two".into()]),
                    ReportBlock::Table {
                        headers: vec!["A".into(), "B".into()],
                        rows: vec![vec!["1".into(), "2".into()]],
                    },
                ],
            }],
        };
        let docx = render_docx(&report).expect("docx");
        assert!(docx.starts_with(b"PK"));
        assert!(docx.len() > 1000);
    }
}
