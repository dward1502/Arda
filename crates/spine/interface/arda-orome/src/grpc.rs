//! Re-export the prost-generated messages alongside tonic service traits so
//! server implementations can import one surface.

#[rustfmt::skip]
mod health_model {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/grpc/arda.orome.health_model.rs"));
}

#[rustfmt::skip]
mod route_governance {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/grpc/arda.orome.route_governance.rs"));
}

pub use health_model::{
    health_model_service_server::{HealthModelService, HealthModelServiceServer},
    health_model_service_client::HealthModelServiceClient,
    HealthRequest, HealthResponse, ListModelsRequest, ListModelsResponse, ModelInfo,
};
pub use route_governance::{
    route_governance_service_server::{RouteGovernanceService, RouteGovernanceServiceServer},
    route_governance_service_client::RouteGovernanceServiceClient,
    GovernanceVerdictRequest, GovernanceVerdictResponse, RouteRequest, RouteResponse,
};
