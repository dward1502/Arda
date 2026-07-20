//! Generated gRPC types + service traits for
//! `arda.orome.health_model` and `arda.orome.route_governance`.
//!
//! Re-export the prost-generated messages alongside tonic service traits so
//! server implementations can import one surface.

#[rustfmt::skip]
mod health_model {
    tonic::include_proto!("arda.orome.health_model");
}

#[rustfmt::skip]
mod route_governance {
    tonic::include_proto!("arda.orome.route_governance");
}

pub use health_model::{
    health_model_service_client, health_model_service_server, HealthRequest, HealthResponse,
    ListModelsRequest, ListModelsResponse, ModelInfo,
};
pub use route_governance::{
    route_governance_service_client, route_governance_service_server, GovernanceVerdictRequest,
    GovernanceVerdictResponse, RouteGovernanceService, RouteRequest, RouteResponse,
};
