use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use subtle::ConstantTimeEq;

#[derive(Clone)]
pub struct McpAuth {
    token_hash: [u8; 32],
    last_failure: Arc<Mutex<Option<Instant>>>,
}

impl McpAuth {
    pub fn new(token: &str) -> Self {
        Self {
            token_hash: *blake3::hash(token.as_bytes()).as_bytes(),
            last_failure: Arc::new(Mutex::new(None)),
        }
    }

    pub fn should_audit_failure(&self) -> bool {
        let mut last_failure = self
            .last_failure
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let now = Instant::now();
        if last_failure
            .map(|last| now.duration_since(last) < Duration::from_secs(60))
            .unwrap_or(false)
        {
            return false;
        }
        *last_failure = Some(now);
        true
    }

    pub fn verify_header(&self, header: Option<&str>) -> bool {
        let Some(header) = header else {
            return false;
        };
        let Some(token) = header.strip_prefix("Bearer ") else {
            return false;
        };
        if token.is_empty() || token.len() > 128 {
            return false;
        }
        let candidate = blake3::hash(token.as_bytes());
        self.token_hash.ct_eq(candidate.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_bearer_token() {
        let auth = McpAuth::new("synthetic-token");
        assert!(auth.verify_header(Some("Bearer synthetic-token")));
        assert!(!auth.verify_header(Some("Bearer wrong-token")));
        assert!(!auth.verify_header(Some("Basic synthetic-token")));
        assert!(!auth.verify_header(None));
    }
}
