use crate::domain::enums::Severity;
use crate::error::{AppError, AppResult};

pub fn severity_for(probability: u8, impact: u8) -> AppResult<Severity> {
    if probability > 9 || impact > 9 {
        return Err(AppError::Validation(
            "Probability and impact must be between 0 and 9.".into(),
        ));
    }
    Ok(match (bucket(impact), bucket(probability)) {
        (0, 0) => Severity::Informational,
        (0, 1) => Severity::Low,
        (0, 2) => Severity::Medium,
        (1, 0) => Severity::Low,
        (1, 1) => Severity::Medium,
        (1, 2) => Severity::High,
        (2, 0) => Severity::Medium,
        (2, 1) => Severity::High,
        (2, 2) => Severity::Critical,
        _ => unreachable!(),
    })
}

fn bucket(value: u8) -> u8 {
    match value {
        0..=2 => 0,
        3..=5 => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_legacy_matrix() {
        assert_eq!(severity_for(0, 0).unwrap(), Severity::Informational);
        assert_eq!(severity_for(2, 2).unwrap(), Severity::Informational);
        assert_eq!(severity_for(3, 0).unwrap(), Severity::Low);
        assert_eq!(severity_for(5, 2).unwrap(), Severity::Low);
        assert_eq!(severity_for(6, 0).unwrap(), Severity::Medium);
        assert_eq!(severity_for(9, 2).unwrap(), Severity::Medium);
        assert_eq!(severity_for(0, 3).unwrap(), Severity::Low);
        assert_eq!(severity_for(5, 5).unwrap(), Severity::Medium);
        assert_eq!(severity_for(6, 3).unwrap(), Severity::High);
        assert_eq!(severity_for(9, 5).unwrap(), Severity::High);
        assert_eq!(severity_for(0, 6).unwrap(), Severity::Medium);
        assert_eq!(severity_for(5, 9).unwrap(), Severity::High);
        assert_eq!(severity_for(6, 6).unwrap(), Severity::Critical);
        assert_eq!(severity_for(9, 9).unwrap(), Severity::Critical);
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(severity_for(10, 0).is_err());
        assert!(severity_for(0, 10).is_err());
    }
}
