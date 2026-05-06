#[derive(Debug, Clone)]
pub struct RistUnknownEnumVariantError {
    pub message: &'static str,
}

#[derive(Debug, Clone)]
pub struct RistInvalidPointerError {
    pub message: &'static str,
}
