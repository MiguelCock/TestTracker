use crate::domain::enums::ProfileRole;

pub fn can_write(role: ProfileRole) -> bool {
    matches!(
        role,
        ProfileRole::Admin | ProfileRole::Lead | ProfileRole::Tester
    )
}

pub fn can_manage(role: ProfileRole) -> bool {
    matches!(role, ProfileRole::Admin | ProfileRole::Lead)
}

pub fn can_approve(role: ProfileRole) -> bool {
    matches!(
        role,
        ProfileRole::Admin | ProfileRole::Lead | ProfileRole::Reviewer
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewers_cannot_write() {
        assert!(!can_write(ProfileRole::Reviewer));
        assert!(can_write(ProfileRole::Tester));
        assert!(can_write(ProfileRole::Lead));
        assert!(can_write(ProfileRole::Admin));
    }

    #[test]
    fn only_leads_and_admins_manage() {
        assert!(can_manage(ProfileRole::Admin));
        assert!(can_manage(ProfileRole::Lead));
        assert!(!can_manage(ProfileRole::Reviewer));
        assert!(!can_manage(ProfileRole::Tester));
    }
}
