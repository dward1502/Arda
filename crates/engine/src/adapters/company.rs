use arda_core::company_ops::{AdapterProvenance, ApprovalReceipt, CommercialAuthority};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const COMPANY_ADAPTER_SCHEMA_VERSION: &str = "arda.company-adapter.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyAdapterCapability {
    OrganizationsRead,
    ContactsRead,
    OpportunitiesRead,
    ActivitiesRead,
    CalendarActivitiesRead,
    EmailContextRead,
    ProjectIssuesRead,
    AccountingExportWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyAdapterOperation {
    OrganizationsRead,
    ContactsRead,
    OpportunitiesRead,
    ActivitiesRead,
    CalendarActivitiesRead,
    EmailContextRead,
    ProjectIssuesRead,
    AccountingExportWrite,
}

impl CompanyAdapterOperation {
    pub fn capability(self) -> CompanyAdapterCapability {
        match self {
            Self::OrganizationsRead => CompanyAdapterCapability::OrganizationsRead,
            Self::ContactsRead => CompanyAdapterCapability::ContactsRead,
            Self::OpportunitiesRead => CompanyAdapterCapability::OpportunitiesRead,
            Self::ActivitiesRead => CompanyAdapterCapability::ActivitiesRead,
            Self::CalendarActivitiesRead => CompanyAdapterCapability::CalendarActivitiesRead,
            Self::EmailContextRead => CompanyAdapterCapability::EmailContextRead,
            Self::ProjectIssuesRead => CompanyAdapterCapability::ProjectIssuesRead,
            Self::AccountingExportWrite => CompanyAdapterCapability::AccountingExportWrite,
        }
    }
    pub fn is_write(self) -> bool {
        matches!(self, Self::AccountingExportWrite)
    }

    fn wire_name(self) -> &'static str {
        match self {
            Self::OrganizationsRead => "organizations_read",
            Self::ContactsRead => "contacts_read",
            Self::OpportunitiesRead => "opportunities_read",
            Self::ActivitiesRead => "activities_read",
            Self::CalendarActivitiesRead => "calendar_activities_read",
            Self::EmailContextRead => "email_context_read",
            Self::ProjectIssuesRead => "project_issues_read",
            Self::AccountingExportWrite => "accounting_export_write",
        }
    }

    fn approval_scope(self, resource_id: &str) -> String {
        format!("{}:{resource_id}", self.wire_name())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanyAdapterRequest {
    pub schema_version: String,
    pub request_id: String,
    pub operation: CompanyAdapterOperation,
    pub resource_id: String,
    pub idempotency_key: String,
    pub authority: CommercialAuthority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval: Option<ApprovalReceipt>,
}

impl CompanyAdapterRequest {
    pub fn validate(
        &self,
        allowlist: &BTreeSet<CompanyAdapterCapability>,
        now: DateTime<Utc>,
    ) -> Result<(), CompanyAdapterError> {
        if self.schema_version != COMPANY_ADAPTER_SCHEMA_VERSION
            || self.request_id.trim().is_empty()
            || self.resource_id.trim().is_empty()
            || self.idempotency_key.trim().is_empty()
        {
            return Err(CompanyAdapterError::InvalidRequest);
        }
        if !allowlist.contains(&self.operation.capability()) {
            return Err(CompanyAdapterError::CapabilityDenied);
        }
        if self.operation.is_write() {
            let approval = self
                .approval
                .as_ref()
                .ok_or(CompanyAdapterError::ApprovalRequired)?;
            if self.authority != CommercialAuthority::ExplicitOperatorApproval
                || approval.expires_at < now
            {
                return Err(CompanyAdapterError::ApprovalRequired);
            }
            if approval.approved_scope != self.operation.approval_scope(&self.resource_id) {
                return Err(CompanyAdapterError::ApprovalScopeMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyResource {
    pub resource_type: String,
    pub external_id: String,
    pub display_name: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
    pub provenance: AdapterProvenance,
}

#[derive(Debug, Clone)]
pub struct ReferenceCrmAdapter {
    adapter_id: String,
    adapter_version: String,
}

impl ReferenceCrmAdapter {
    pub fn read_only(adapter_id: impl Into<String>, adapter_version: impl Into<String>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            adapter_version: adapter_version.into(),
        }
    }

    pub fn normalize(
        &self,
        resource_type: &str,
        observed_at: DateTime<Utc>,
        rows: Vec<(String, String, BTreeMap<String, String>)>,
    ) -> Result<Vec<CompanyResource>, CompanyAdapterError> {
        let mut stable_ids = BTreeSet::new();
        let mut resources = Vec::with_capacity(rows.len());
        for (external_id, display_name, attributes) in rows {
            if external_id.trim().is_empty() || !stable_ids.insert(external_id.clone()) {
                return Err(CompanyAdapterError::DuplicateOrMissingExternalId);
            }
            resources.push(CompanyResource {
                resource_type: resource_type.into(),
                external_id: external_id.clone(),
                display_name,
                attributes,
                provenance: AdapterProvenance {
                    adapter_id: self.adapter_id.clone(),
                    adapter_version: self.adapter_version.clone(),
                    external_id,
                    observed_at,
                    read_only: true,
                },
            });
        }
        resources.sort_by(|a, b| a.external_id.cmp(&b.external_id));
        Ok(resources)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CompanyAdapterError {
    #[error("invalid company adapter request")]
    InvalidRequest,
    #[error("company adapter capability denied")]
    CapabilityDenied,
    #[error("write operation requires explicit unexpired operator approval")]
    ApprovalRequired,
    #[error("operator approval does not cover the requested operation and resource")]
    ApprovalScopeMismatch,
    #[error("CRM rows require unique stable external IDs")]
    DuplicateOrMissingExternalId,
}
