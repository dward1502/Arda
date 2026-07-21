use std::net::SocketAddr;
use std::sync::Arc;

use crate::config::ManweConfig;
use arda_orome::grpc::{
    GovernanceVerdictRequest, GovernanceVerdictResponse, HealthModelService,
    HealthModelServiceServer, HealthRequest, HealthResponse, ListModelsRequest, ListModelsResponse,
    RouteGovernanceService, RouteGovernanceServiceServer, RouteRequest, RouteResponse,
};
use tonic::transport::Server;

#[derive(Clone)]
pub struct GrpcState {
    pub config: Arc<ManweConfig>,
    pub client: reqwest::Client,
}

pub async fn serve_grpc(state: GrpcState) -> anyhow::Result<()> {
    let addr: SocketAddr = std::env::var("MANWE_GRPC_PORT")
        .unwrap_or_else(|_| "0.0.0.0:50051".into())
        .parse()?;
    tracing::info!("manwe grpc: listening on {}", addr);

    let health = HealthServer {
        state: state.clone(),
    };
    let route = RouteServer { state };
    Server::builder()
        .add_service(HealthModelServiceServer::new(health))
        .add_service(RouteGovernanceServiceServer::new(route))
        .serve(addr)
        .await?;
    Ok(())
}

#[derive(Clone)]
struct HealthServer {
    state: GrpcState,
}
#[tonic::async_trait]
impl HealthModelService for HealthServer {
    async fn health(
        &self,
        _request: tonic::Request<HealthRequest>,
    ) -> Result<tonic::Response<HealthResponse>, tonic::Status> {
        Ok(tonic::Response::new(HealthResponse {
            status: "ok".into(),
        }))
    }

    async fn list_models(
        &self,
        _request: tonic::Request<ListModelsRequest>,
    ) -> Result<tonic::Response<ListModelsResponse>, tonic::Status> {
        let created = chrono::Utc::now().timestamp();
        let mut data = Vec::new();
        for (_name, p) in &self.state.config.providers {
            let models = if p.models.is_empty() {
                vec!["default".into()]
            } else {
                p.models.clone()
            };
            for m in models {
                data.push(arda_orome::grpc::ModelInfo {
                    id: m,
                    object: "model".into(),
                    created,
                    owned_by: "manwe".into(),
                });
            }
        }
        Ok(tonic::Response::new(ListModelsResponse { data }))
    }
}

#[derive(Clone)]
struct RouteServer {
    state: GrpcState,
}
#[tonic::async_trait]
impl RouteGovernanceService for RouteServer {
    async fn route_chat_completions(
        &self,
        request: tonic::Request<RouteRequest>,
    ) -> Result<tonic::Response<RouteResponse>, tonic::Status> {
        let req = request.into_inner();
        tracing::debug!(
            "route_chat_completions model={} headers={:?}",
            req.model,
            req.headers
        );
        let upstream = self
            .state
            .config
            .providers
            .values()
            .next()
            .map(|p| p.base_url.trim_end_matches('/').to_string())
            .unwrap_or_default();
        let body = if req.body.is_empty() {
            String::new()
        } else {
            String::from_utf8(req.body).unwrap_or_default()
        };
        let resp = self
            .state
            .client
            .post(format!("{}/chat/completions", upstream))
            .json(&serde_json::from_str(&body).unwrap_or(serde_json::json!({})))
            .send()
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))?;
        let status = i32::from(resp.status().as_u16());
        let body = resp.bytes().await.map(|b| b.to_vec()).unwrap_or_default();
        Ok(tonic::Response::new(RouteResponse {
            status,
            body,
            upstream,
        }))
    }

    async fn governance_verdict(
        &self,
        request: tonic::Request<GovernanceVerdictRequest>,
    ) -> Result<tonic::Response<GovernanceVerdictResponse>, tonic::Status> {
        let req = request.into_inner();
        Ok(tonic::Response::new(GovernanceVerdictResponse {
            verdict: "allow".into(),
            policy_version: "v0".into(),
            policy_hash: "dev".into(),
            reason: format!(
                "accepted from {} with {} bytes",
                req.actor,
                req.payload.len()
            ),
        }))
    }
}
