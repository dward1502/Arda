// sigil: REPAIR
#[cfg(feature = "adaptive")]
pub use crate::adaptive::service::types::CharonService;

#[cfg(not(feature = "adaptive"))]
#[derive(Debug, Clone)]
pub struct CharonService {
    _flag: (),
}

#[cfg(not(feature = "adaptive"))]
impl CharonService {
    pub fn new(_root: impl std::path::PathBufLike) -> Self {
        Self { _flag: () }
    }
}

#[cfg(not(feature = "adaptive"))]
#[derive(Debug, Clone)]
pub struct MahaczikCharonService;

#[cfg(not(feature = "adaptive"))]
impl MahaczikCharonService {
    pub fn new() -> Self {
        Self
    }
}
