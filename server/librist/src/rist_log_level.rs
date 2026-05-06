use crate::RistUnknownEnumVariantError;

#[derive(Default, Copy, Clone)]
pub enum RistLogLevel {
    #[default]
    Disable = 0,
    Error = 1,
    Warn = 2,
    Notice = 3,
    Info = 4,
    Debug = 5,
    Simulate = 6,
}

impl RistLogLevel {
    pub fn is_important_than_or_equal_to(&self, other: &RistLogLevel) -> bool {
        *self as u32 >= *other as u32
    }
}

impl std::fmt::Display for RistLogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RistLogLevel::Disable => write!(f, "Disable"),
            RistLogLevel::Error => write!(f, "Error"),
            RistLogLevel::Warn => write!(f, "Warn"),
            RistLogLevel::Notice => write!(f, "Notice"),
            RistLogLevel::Info => write!(f, "Info"),
            RistLogLevel::Debug => write!(f, "Debug"),
            RistLogLevel::Simulate => write!(f, "Simulate"),
        }
    }
}

impl Into<crate::binding::rist_log_level> for &RistLogLevel {
    fn into(self) -> crate::binding::rist_log_level {
        match self {
            RistLogLevel::Disable => crate::binding::rist_log_level_RIST_LOG_DISABLE,
            RistLogLevel::Error => crate::binding::rist_log_level_RIST_LOG_ERROR,
            RistLogLevel::Warn => crate::binding::rist_log_level_RIST_LOG_WARN,
            RistLogLevel::Notice => crate::binding::rist_log_level_RIST_LOG_NOTICE,
            RistLogLevel::Info => crate::binding::rist_log_level_RIST_LOG_INFO,
            RistLogLevel::Debug => crate::binding::rist_log_level_RIST_LOG_DEBUG,
            RistLogLevel::Simulate => crate::binding::rist_log_level_RIST_LOG_SIMULATE,
        }
    }
}

impl TryFrom<crate::binding::rist_log_level> for RistLogLevel {
    type Error = RistUnknownEnumVariantError;

    fn try_from(
        raw: crate::binding::rist_log_level,
    ) -> Result<Self, crate::RistUnknownEnumVariantError> {
        match raw {
            crate::binding::rist_log_level_RIST_LOG_DISABLE => Ok(RistLogLevel::Disable),
            crate::binding::rist_log_level_RIST_LOG_ERROR => Ok(RistLogLevel::Error),
            crate::binding::rist_log_level_RIST_LOG_WARN => Ok(RistLogLevel::Warn),
            crate::binding::rist_log_level_RIST_LOG_NOTICE => Ok(RistLogLevel::Notice),
            crate::binding::rist_log_level_RIST_LOG_INFO => Ok(RistLogLevel::Info),
            crate::binding::rist_log_level_RIST_LOG_DEBUG => Ok(RistLogLevel::Debug),
            crate::binding::rist_log_level_RIST_LOG_SIMULATE => Ok(RistLogLevel::Simulate),
            _ => Err(crate::RistUnknownEnumVariantError {
                message: "Unknown rist_log_level variant",
            }),
        }
    }
}
