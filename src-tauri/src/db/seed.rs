use rusqlite::Connection;

use crate::error::AppResult;

struct Template {
    id: &'static str,
    domain: &'static str,
    category: &'static str,
    title: &'static str,
    approach: &'static str,
    expected: &'static str,
    references: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template {
        id: "tpl-web-reflected-xss",
        domain: "web",
        category: "Injection",
        title: "Reflected cross-site scripting",
        approach: "Identify reflected input, inject benign script payloads, and verify browser execution.",
        expected: "Input is encoded or rejected and the payload does not execute.",
        references: "OWASP A3, CWE-79",
    },
    Template {
        id: "tpl-web-auth",
        domain: "web",
        category: "Authentication",
        title: "Authentication and session management",
        approach: "Review login, logout, password reset, session rotation, cookie flags, and lockout behavior.",
        expected: "Sessions are rotated, cookies are protected, and brute-force controls work.",
        references: "OWASP A7, CWE-287",
    },
    Template {
        id: "tpl-web-access-control",
        domain: "web",
        category: "Authorization",
        title: "Horizontal and vertical access control",
        approach: "Compare requests across users and roles, attempting to access objects and functions outside the assigned role.",
        expected: "Server-side authorization blocks unauthorized objects and functions.",
        references: "OWASP A1, CWE-639",
    },
    Template {
        id: "tpl-api-auth",
        domain: "api",
        category: "Authentication",
        title: "API authentication and token handling",
        approach: "Inspect token issuance, expiry, revocation, audience, and storage; replay and tamper with tokens.",
        expected: "Tokens are validated and scoped, and tampered tokens are rejected.",
        references: "OWASP API2, CWE-287",
    },
    Template {
        id: "tpl-api-bola",
        domain: "api",
        category: "Authorization",
        title: "Broken object-level authorization",
        approach: "Enumerate object identifiers and attempt access across users and tenants.",
        expected: "Every object access is authorized server-side.",
        references: "OWASP API1, CWE-639",
    },
    Template {
        id: "tpl-api-input",
        domain: "api",
        category: "Injection",
        title: "API input validation and injection",
        approach: "Fuzz parameters, JSON bodies, headers, and content types for injection and parser flaws.",
        expected: "Malformed or malicious input is rejected safely.",
        references: "OWASP API8, CWE-20",
    },
    Template {
        id: "tpl-mobile-storage",
        domain: "mobile",
        category: "Data protection",
        title: "Insecure local data storage",
        approach: "Inspect app sandbox, keychain/keystore usage, caches, logs, and backups for sensitive data.",
        expected: "Sensitive data is encrypted with platform facilities and excluded from backups and logs.",
        references: "OWASP MASVS-STORAGE, CWE-312",
    },
    Template {
        id: "tpl-mobile-transport",
        domain: "mobile",
        category: "Network",
        title: "Transport security and certificate pinning",
        approach: "Proxy traffic, inspect TLS validation, and test pinning bypass resistance.",
        expected: "TLS is enforced and pinning is implemented where required.",
        references: "OWASP MASVS-NETWORK, CWE-295",
    },
    Template {
        id: "tpl-thick-client-binary",
        domain: "thick_client",
        category: "Reverse engineering",
        title: "Thick client binary and configuration review",
        approach: "Inspect binaries, configuration, embedded secrets, update mechanisms, and local privilege boundaries.",
        expected: "No secrets are embedded and updates and permissions are hardened.",
        references: "CWE-798",
    },
    Template {
        id: "tpl-container-image",
        domain: "container",
        category: "Build security",
        title: "Container image and runtime hardening",
        approach: "Review base image, user, capabilities, secrets, image signing, and runtime restrictions.",
        expected: "Images run as non-root with minimal capabilities and no embedded secrets.",
        references: "CIS Docker Benchmark, CWE-250",
    },
    Template {
        id: "tpl-cloud-identity",
        domain: "cloud",
        category: "Identity",
        title: "Cloud identity and privilege review",
        approach: "Review accounts, roles, policies, federation, MFA, and privilege escalation paths.",
        expected: "Least privilege is enforced and MFA protects privileged identities.",
        references: "CIS Benchmarks, CWE-269",
    },
    Template {
        id: "tpl-cloud-storage",
        domain: "cloud",
        category: "Data protection",
        title: "Cloud storage exposure",
        approach: "Enumerate storage resources and test public access, policies, encryption, and logging.",
        expected: "Storage is private by default, encrypted, and access-logged.",
        references: "CWE-284",
    },
    Template {
        id: "tpl-windows-baseline",
        domain: "windows",
        category: "Configuration",
        title: "Windows server security baseline",
        approach: "Review patch level, services, SMB, RDP, local admins, logging, and credential protections.",
        expected: "Baseline settings match the approved hardening standard.",
        references: "CIS Windows Benchmark, CWE-1188",
    },
    Template {
        id: "tpl-sap-parameters",
        domain: "sap",
        category: "Configuration",
        title: "SAP security parameter review",
        approach: "Compare profile parameters and user authorizations against the approved baseline.",
        expected: "Parameters and authorizations meet the baseline.",
        references: "SAP Security Baseline",
    },
    Template {
        id: "tpl-ai-prompt-injection",
        domain: "ai",
        category: "Model security",
        title: "Prompt injection and tool abuse",
        approach: "Test direct and indirect prompt injection, tool-call boundaries, data exfiltration, and output handling.",
        expected: "Untrusted content cannot alter tool permissions or exfiltrate data.",
        references: "OWASP LLM Top 10, CWE-1426",
    },
    Template {
        id: "tpl-ai-data-exposure",
        domain: "ai",
        category: "Data protection",
        title: "Sensitive data exposure in AI features",
        approach: "Probe prompts, embeddings, logs, and responses for cross-user or training-data leakage.",
        expected: "Sensitive data is isolated and excluded from unauthorized contexts.",
        references: "OWASP LLM Top 10, CWE-200",
    },
    Template {
        id: "tpl-bi-dashboard",
        domain: "bi",
        category: "Authorization",
        title: "BI dashboard access and row-level security",
        approach: "Test report sharing, dataset permissions, row-level filters, exports, and embedded credentials.",
        expected: "Dashboards and underlying data respect role and tenant boundaries.",
        references: "CWE-639",
    },
];

pub fn seed_templates(conn: &Connection) -> AppResult<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM test_case_templates", [], |row| {
        row.get(0)
    })?;
    if count > 0 {
        return Ok(());
    }
    let mut stmt = conn.prepare(
        "INSERT INTO test_case_templates (id, domain, category, title, approach, expected_result, references_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for template in TEMPLATES {
        stmt.execute(rusqlite::params![
            template.id,
            template.domain,
            template.category,
            template.title,
            template.approach,
            template.expected,
            template.references,
        ])?;
    }
    Ok(())
}
