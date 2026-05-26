/// State for the master output bus.
#[derive(Debug, Clone)]
pub struct MasterState {
    #[allow(dead_code)]
    pub volume: f32, // linear 0..2
}

impl Default for MasterState {
    fn default() -> Self {
        Self { volume: 1.0 }
    }
}
