// sigil: REPAIR
pub struct PhiCalibrator;

impl PhiCalibrator {
    pub fn new() -> Self {
        Self
    }

    pub fn calibrate(&self, duration_ms: u64) -> f64 {
        let phi = 1.618033988749895;
        duration_ms as f64 * phi
    }
}

impl Default for PhiCalibrator {
    fn default() -> Self {
        Self::new()
    }
}
