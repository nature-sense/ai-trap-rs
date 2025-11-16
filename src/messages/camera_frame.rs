use std::sync::Arc;
use nokhwa::Buffer;

#[derive(Clone)]
pub struct CameraFrame {
    timestamp : i64,
    buffer : Arc<Buffer>
}

impl CameraFrame {

    pub(crate) fn new(timestamp : i64, buffer : Buffer) -> Self {
        Self {
            timestamp,
            buffer: Arc::new(buffer)
        }
    }
}