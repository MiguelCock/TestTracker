# Assessment import

Assessment import creates a new engagement from a previous assessment. It never merges into or
modifies an existing engagement, and the source file is not stored.

## Supported inputs

| Format                   | Extraction                                        | Deterministic parsing    |
| ------------------------ | ------------------------------------------------- | ------------------------ |
| `.xlsx`, `.xlsm`, `.xls` | Sheet names with tab-separated cell text          | Yes, for tracker headers |
| `.docx`                  | Paragraph and table text from `word/document.xml` | No                       |
| `.pdf`                   | Best-effort embedded text extraction              | No                       |
| `.md`, `.markdown`       | UTF-8 text                                        | No                       |
| `.json`                  | UTF-8 text                                        | No                       |
| `.txt`, `.text`, `.log`  | UTF-8 text                                        | No                       |

Limits: 25 MB per file, 300,000 extracted characters, 5,000 test cases, 1,000 findings, 500 scope
assets, 200 risks, 200 assumptions, and 100 contacts. AI prompts are capped at 120,000 characters.

PDF extraction does not use OCR. Scanned PDFs without a text layer must be converted to text first.
Legacy `.doc` and `.xls` binary formats are not supported; save them as `.docx` or `.xlsx`.

## Tracker parsing

The spreadsheet parser looks for a header row containing `Test Case` and either `Approach` or
`Result`. Recognized columns include project name, test dates, application summary, attack surface,
risks, test case, test approach, category, status, result, finding, severity, and remediation.

- Status values are normalized to `not_started`, `in_progress`, `done`, `blocked`, or
  `not_applicable`.
- Severity labels map to approximate probability/impact scores; the server recomputes severity.
- A non-empty finding cell (other than `No`, `None`, `N/A`, `false`, `0`, or `-`) creates a finding.
  If the cell is `Yes`, the linked test title is used as the finding title.
- Findings whose titles match a test-case title are linked to that test.
- Imported risks, tests, and findings are drafts for review, not verified conclusions.

## AI-assisted import

For non-spreadsheet inputs, or to structure a spreadsheet differently:

1. The server extracts text and stores it in memory with a 30-minute expiry.
2. The user selects a provider and may add instructions.
3. Non-local providers require an explicit cloud-consent checkbox.
4. The server builds the import prompt, runs the same redaction pipeline as the AI assistant, and
   returns the exact redacted prompt as a five-minute consent preview.
5. Running the import makes one provider call. The response must be a single JSON object matching
   the import schema; the server validates record counts, titles, severity, and status values.
6. The validated draft is held in memory for 30 minutes. **Create engagement** is the only write.

The document text is untrusted data. The system prompt tells the model not to follow instructions
inside the document, but prompt injection remains a risk. Review every draft.

## Import semantics

- Creates a new engagement in `draft` status with new identifiers.
- The current profile is recorded as creator and test/finding owner.
- Dates are normalized to `YYYY-MM-DD` when recognizable; otherwise they are left empty rather than
  storing an invalid value.
- Findings are created as `open` with `retest_status = not_required`; probability and impact are
  bounded to 0–9.
- Evidence, snapshots, approvals, audit history, AI data, plugins, and generated documents are not
  imported.
- Import is a write operation and requires an unlocked session with a role allowed to write.
- Audit events: `import.extracted`, `import.preview`, `import.proposal`, `import.failed`, and
  `assessment.imported`. Failure/preview events store hashes and metadata, not document text.

## Security notes

- File content is validated by extension, size, and parser; errors are returned as safe validation
  messages.
- The webview receives only a short text preview and draft metadata. Full extracted text remains in
  Rust memory until it expires or the vault locks.
- In-memory extractions, previews, and drafts are cleared on lock and expiry.
- AI import does not bypass redaction. Cloud use is never implicit.
- Do not import untrusted documents on a machine where the source may contain malware; parsing is
  in-process and PDF/DOCX parsers are not a sandbox.
