//! Factorization of observability types shared between engine and generated
//! tonic handers. This avoids leaking Rust QUERY-like literals upstream.
mod sensor_response {
    pub struct SensorResponse {
        pub status: String,
    }
}

mod route_decision {
    pub struct RouteDecision {
        pub upstream: String,
        pub status: i32,
    }
}

mod budget_status {
    pub struct BudgetStatus {
        pub used: u64,
        pub limit: u64,
    }
}
