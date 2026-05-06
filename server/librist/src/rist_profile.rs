pub enum RistProfile {
    Simple,
    Main,
    Advanced,
}

impl Into<crate::binding::rist_profile> for RistProfile {
    fn into(self) -> crate::binding::rist_profile {
        match self {
            RistProfile::Simple => crate::binding::rist_profile_RIST_PROFILE_SIMPLE,
            RistProfile::Main => crate::binding::rist_profile_RIST_PROFILE_MAIN,
            RistProfile::Advanced => crate::binding::rist_profile_RIST_PROFILE_ADVANCED,
        }
    }
}
