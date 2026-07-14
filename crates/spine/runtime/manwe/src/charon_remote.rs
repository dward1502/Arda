//! `public-bridge` — charon → gateway port.
//!
//! Provides `CharonRemote`, adapters, and `GatewayDependencyInjection`
//! so callers can start with a local transport and swap to manwe when
//! upstream connectors are ready.

use std::future::Future;

use anyhow::{anyhow, Context as _, Result};
use reqwest::Client;
use serde_json::Value;

use crate::{CharonTransport, ProviderRecord, SpannedManweGateway};

pub struct CharonRemote {
    client: Client,
    endpoint: String,
}

impl CharonRemote {
    pub const DEFAULT_ENDPOINT: &'static str = "http://localhost:7171/v1/chat/completions";

    pub fn new(client: Client, gateway_endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = gateway_endpoint.into();
        if !endpoint.contains("chat/completions") {
            return Err(anyhow!(
                "invalid manwe gateway endpoint: {endpoint:?} (must include chat/completions)"
            ));
        }
        Ok(Self { client, endpoint })
    }

    pub fn with_client(client: Client) -> Self {
        Self { client, endpoint: Self::DEFAULT_ENDPOINT.to_string() }
    }
}

impl CharonTransport for CharonRemote {
    fn complete(
        &self,
        request: Value,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Value>> + Send + '_>> {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        Box::pin(async move {
            let response = client
                .post(&endpoint)
                .json(&request)
                .send()
                .await
                .context(format!("failed to post to manwe gateway at {endpoint:?}"))?;
            let json = response.json::<Value>().await?;
            Ok(json)
        })
    }
}

pub trait GatewayDependencyInjection: Send + Sync {
    fn gateway(&self) -> SpannedManweGateway;
}

impl GatewayDependencyInjection for SpannedManweGateway {
    fn gateway(&self) -> SpannedManweGateway {
        self.clone()
    }
}

pub struct GatewayProviders {
    pub providers: Vec<ProviderRecord>,
}

impl GatewayProviders {
    pub fn from_records(providers: Vec<ProviderRecord>) -> Result<Self> {
        if providers.is_empty() {
            return Err(anyhow!("GatewayProviders requires at least one record"));
        }
        Ok(Self { providers })
    }
}
