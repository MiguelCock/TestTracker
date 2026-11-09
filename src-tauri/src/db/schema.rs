pub const MIGRATION_1: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE profiles (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL,
    public_key TEXT NOT NULL,
    encrypted_private_key TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE engagements (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    client_code TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    start_date TEXT NOT NULL DEFAULT '',
    end_date TEXT NOT NULL DEFAULT '',
    application_summary TEXT NOT NULL DEFAULT '',
    attack_surface TEXT NOT NULL DEFAULT '',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (created_by) REFERENCES profiles(id)
);

CREATE TABLE engagement_members (
    engagement_id TEXT NOT NULL,
    profile_id TEXT NOT NULL,
    role TEXT NOT NULL,
    PRIMARY KEY (engagement_id, profile_id),
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE TABLE scope_assets (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    value TEXT NOT NULL,
    in_scope INTEGER NOT NULL DEFAULT 1,
    notes TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE risks (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    likelihood INTEGER NOT NULL,
    impact INTEGER NOT NULL,
    severity TEXT NOT NULL,
    notes TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE assumptions (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    text TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE contacts (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT '',
    email TEXT NOT NULL DEFAULT '',
    phone TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE test_case_templates (
    id TEXT PRIMARY KEY,
    domain TEXT NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    approach TEXT NOT NULL,
    expected_result TEXT NOT NULL DEFAULT '',
    references_text TEXT NOT NULL DEFAULT ''
);

CREATE TABLE test_cases (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    template_id TEXT,
    title TEXT NOT NULL,
    approach TEXT NOT NULL DEFAULT '',
    category TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    result TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    assignee TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE,
    FOREIGN KEY (template_id) REFERENCES test_case_templates(id) ON DELETE SET NULL
);

CREATE TABLE findings (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    impact TEXT NOT NULL DEFAULT '',
    remediation TEXT NOT NULL DEFAULT '',
    severity TEXT NOT NULL,
    probability INTEGER NOT NULL,
    impact_score INTEGER NOT NULL,
    owasp TEXT NOT NULL DEFAULT '',
    cwe TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    retest_status TEXT NOT NULL,
    owner TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE finding_test_cases (
    finding_id TEXT NOT NULL,
    test_case_id TEXT NOT NULL,
    PRIMARY KEY (finding_id, test_case_id),
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE,
    FOREIGN KEY (test_case_id) REFERENCES test_cases(id) ON DELETE CASCADE
);

CREATE TABLE finding_assets (
    finding_id TEXT NOT NULL,
    asset_id TEXT NOT NULL,
    PRIMARY KEY (finding_id, asset_id),
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES scope_assets(id) ON DELETE CASCADE
);

CREATE TABLE evidence (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    finding_id TEXT,
    test_case_id TEXT,
    filename TEXT NOT NULL,
    mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    size INTEGER NOT NULL,
    blob_id TEXT NOT NULL,
    plaintext_hash TEXT NOT NULL,
    caption TEXT NOT NULL DEFAULT '',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE,
    FOREIGN KEY (finding_id) REFERENCES findings(id) ON DELETE SET NULL,
    FOREIGN KEY (test_case_id) REFERENCES test_cases(id) ON DELETE SET NULL
);

CREATE TABLE evidence_blobs (
    blob_id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    nonce TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    content_json TEXT NOT NULL DEFAULT '{}',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE TABLE document_versions (
    id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    content_json TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE TABLE approvals (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    document_id TEXT,
    reviewer_id TEXT NOT NULL,
    decision TEXT NOT NULL,
    comment TEXT NOT NULL DEFAULT '',
    signature TEXT NOT NULL,
    signed_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE,
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE TABLE audit_events (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    id TEXT NOT NULL UNIQUE,
    engagement_id TEXT,
    actor_id TEXT,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}',
    prev_hash TEXT NOT NULL,
    hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX idx_engagements_status ON engagements(status);
CREATE INDEX idx_test_cases_engagement ON test_cases(engagement_id);
CREATE INDEX idx_findings_engagement ON findings(engagement_id);
CREATE INDEX idx_findings_severity ON findings(engagement_id, severity);
CREATE INDEX idx_evidence_engagement ON evidence(engagement_id);
CREATE INDEX idx_audit_engagement ON audit_events(engagement_id, seq);
"#;

pub const MIGRATION_2: &str = r#"
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    snapshot_json TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE INDEX idx_snapshots_engagement ON snapshots(engagement_id, version);
CREATE INDEX idx_snapshots_hash ON snapshots(content_hash);

ALTER TABLE approvals ADD COLUMN snapshot_id TEXT REFERENCES snapshots(id) ON DELETE CASCADE;
CREATE INDEX idx_approvals_engagement ON approvals(engagement_id);
"#;

pub const MIGRATION_3: &str = r#"
CREATE TABLE generated_documents (
    id TEXT PRIMARY KEY,
    engagement_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    format TEXT NOT NULL,
    title TEXT NOT NULL,
    filename TEXT NOT NULL,
    size INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    nonce TEXT NOT NULL,
    path TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);

CREATE INDEX idx_generated_documents_engagement
    ON generated_documents(engagement_id, created_at);
"#;

pub const MIGRATION_4: &str = r#"
CREATE TABLE ai_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model TEXT NOT NULL,
    encrypted_api_key TEXT NOT NULL DEFAULT '',
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE ai_policies (
    engagement_id TEXT PRIMARY KEY,
    policy TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (engagement_id) REFERENCES engagements(id) ON DELETE CASCADE
);
"#;

pub const MIGRATION_5: &str = r#"
CREATE TABLE plugin_publishers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    public_key TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE plugins (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    name TEXT NOT NULL,
    publisher_id TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    capabilities_json TEXT NOT NULL DEFAULT '[]',
    manifest_json TEXT NOT NULL,
    wasm_path TEXT NOT NULL,
    wasm_hash TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    installed_by TEXT NOT NULL,
    installed_at TEXT NOT NULL
);
"#;

pub const MIGRATION_6: &str = r#"
ALTER TABLE ai_providers ADD COLUMN supports_vision INTEGER NOT NULL DEFAULT 0;
"#;

pub const MIGRATION_7: &str = r#"
ALTER TABLE test_case_templates ADD COLUMN source TEXT NOT NULL DEFAULT 'builtin';
ALTER TABLE test_case_templates ADD COLUMN origin_id TEXT;

CREATE TABLE test_template_bundles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    version TEXT NOT NULL DEFAULT '1.0.0',
    source TEXT NOT NULL DEFAULT 'imported',
    origin_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE test_template_bundle_items (
    bundle_id TEXT NOT NULL,
    template_id TEXT NOT NULL,
    origin_template_id TEXT NOT NULL DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (bundle_id, template_id),
    FOREIGN KEY (bundle_id) REFERENCES test_template_bundles(id) ON DELETE CASCADE,
    FOREIGN KEY (template_id) REFERENCES test_case_templates(id) ON DELETE CASCADE
);

CREATE INDEX idx_template_bundle_items_bundle
    ON test_template_bundle_items(bundle_id, sort_order);
"#;
