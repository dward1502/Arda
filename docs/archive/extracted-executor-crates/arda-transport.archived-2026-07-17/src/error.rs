use std::fmt;

#[derive(Debug, Clone)]
pub struct TransportError {
    pub agent: &'static str,
    pub message: String,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} error: {}", self.agent, self.message)
    }
}

impl std::error::Error for TransportError {}

macro_rules! transport_err {
    ($agent:expr, $msg:expr) => {
        $crate::error::TransportError {
            agent: $agent,
            message: $msg.to_string(),
        }
    };
}

pub(crate) use transport_err;
