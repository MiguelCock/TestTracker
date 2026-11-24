use std::sync::LazyLock;

use regex::Regex;

pub struct RedactionResult {
    pub text: String,
    pub count: usize,
    pub labels: Vec<String>,
}

struct Pattern {
    label: &'static str,
    regex: Regex,
}

static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(|| {
    [
        (
            "private-key",
            r"(?s)-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?-----END [A-Z0-9 ]*PRIVATE KEY-----",
        ),
        (
            "jwt",
            r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b",
        ),
        (
            "bearer-token",
            r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{12,}",
        ),
        ("aws-access-key", r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b"),
        (
            "secret",
            r#"(?i)\b(?:api[_-]?key|secret|token|password|passwd|pwd)\s*[:=]\s*["']?[A-Za-z0-9_./+=-]{8,}["']?"#,
        ),
        (
            "url-credentials",
            r"(?i)\b[a-z][a-z0-9+.-]*://[^/\s:@]+:[^/\s@]+@",
        ),
        (
            "email",
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
        ),
        (
            "ipv4",
            r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b",
        ),
        (
            "internal-host",
            r"(?i)\b[a-z0-9][a-z0-9.-]*\.(?:internal|local|corp|lan|intra)\b",
        ),
    ]
    .into_iter()
    .map(|(label, pattern)| Pattern {
        label,
        regex: Regex::new(pattern).expect("valid redaction regex"),
    })
    .collect()
});

pub fn redact(input: &str) -> RedactionResult {
    let mut text = input.to_string();
    let mut labels = Vec::new();
    let mut total = 0usize;
    for pattern in PATTERNS.iter() {
        let matches = pattern.regex.find_iter(&text).count();
        if matches == 0 {
            continue;
        }
        total += matches;
        labels.push(format!("{} x{}", pattern.label, matches));
        let replacement = format!("[REDACTED:{}]", pattern.label);
        text = pattern
            .regex
            .replace_all(&text, replacement.as_str())
            .into_owned();
    }
    RedactionResult {
        text,
        count: total,
        labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens_and_secrets() {
        let synthetic = format!(
            "Authorization: {}\n{}={}\n{}",
            concat!("Bearer ", "abcdefghijklmnopqrstuvwxyz"),
            concat!("api_", "key"),
            "synthetic-secret-value",
            concat!("AK", "IA", "SYNTHETICEXAMPLE"),
        );
        let result = redact(&synthetic);
        assert!(result.text.contains("[REDACTED:bearer-token]"));
        assert!(result.text.contains("[REDACTED:secret]"));
        assert!(result.text.contains("[REDACTED:aws-access-key]"));
        assert!(!result.text.contains("synthetic-secret-value"));
        assert_eq!(result.count, 3);
    }

    #[test]
    fn redacts_contact_and_network_data() {
        let result = redact("Contact analyst@example.test from 10.20.30.40 and app.corp.internal");
        assert!(result.text.contains("[REDACTED:email]"));
        assert!(result.text.contains("[REDACTED:ipv4]"));
        assert!(result.text.contains("[REDACTED:internal-host]"));
        assert!(!result.text.contains("analyst@example.test"));
        assert!(!result.text.contains("10.20.30.40"));
    }

    #[test]
    fn redacts_private_keys_and_jwts() {
        let synthetic = format!(
            "{}MIIEow{}{}",
            concat!("-----BEGIN ", "RSA PRIVATE KEY-----"),
            concat!("-----END ", "RSA PRIVATE KEY-----"),
            concat!(
                "eyJ",
                "hbGciOiJIUzI1NiJ9",
                ".",
                "eyJzdWIiOiIxMjM0NTY3ODkwIn0",
                ".",
                "signaturevalue123"
            ),
        );
        let result = redact(&synthetic);
        assert!(result.text.contains("[REDACTED:private-key]"));
        assert!(result.text.contains("[REDACTED:jwt]"));
        assert!(!result.text.contains("MIIEow"));
    }

    #[test]
    fn leaves_normal_text_alone() {
        let result = redact("The login form lacks rate limiting on the password reset endpoint.");
        assert_eq!(result.count, 0);
        assert_eq!(
            result.text,
            "The login form lacks rate limiting on the password reset endpoint."
        );
    }
}
