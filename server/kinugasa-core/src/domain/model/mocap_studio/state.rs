use crate::domain::model::id;

#[derive(Debug, Clone)]
pub struct MocapStudio {
    pub cameras: std::collections::HashMap<id::CameraId, Camera>,
    pub completed_takes: std::collections::HashMap<id::TakeId, Take>,
    pub ongoing_take: Option<(id::TakeId, Take)>,
}

#[derive(Debug, Clone)]
pub struct Camera {}

#[derive(Debug, Clone)]
pub struct Take {}

#[derive(Debug, Clone)]
pub struct Video {}
