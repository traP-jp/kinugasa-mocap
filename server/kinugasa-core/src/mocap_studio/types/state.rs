#[derive(Debug, Clone)]
pub struct MocapStudio {
    pub cameras: std::collections::HashMap<crate::id::CameraId, Camera>,
    pub completed_takes: std::collections::HashMap<crate::id::TakeId, Take>,
    pub ongoing_take: Option<(crate::id::TakeId, Take)>,
}

#[derive(Debug, Clone)]
pub struct Camera {}

#[derive(Debug, Clone)]
pub struct Take {}

#[derive(Debug, Clone)]
pub struct Video {}
