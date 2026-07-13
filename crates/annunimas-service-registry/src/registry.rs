// sigil: ANKH
//! Service registry implementation and state management.
//!
//! This module provides the core registry functionality for Annunimas services,
//! including service record definitions, registry state management, and startup ordering.
//!
//! # Types
//! - [`ServiceRecord`]: Individual service definition with dependencies
//! - [`ServiceRegistry`]: Main registry structure with validation and ordering
//!
//! # Features
//! - Service validation (duplicate detection, empty IDs)
//! - Dependency graph analysis
//! - Startup order calculation (topological sort)
//! - Cycle detection in service dependencies

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceRecord {
    pub service_id: String,
    pub dependencies: Vec<String>,
    pub startup_order: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceRegistry {
    pub services: Vec<ServiceRecord>,
}

impl ServiceRegistry {
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = BTreeMap::new();
        for service in &self.services {
            if service.service_id.trim().is_empty() {
                return Err("missing service_id".into());
            }
            if seen.insert(service.service_id.clone(), true).is_some() {
                return Err(format!("duplicate service_id: {}", service.service_id));
            }
        }
        Ok(())
    }

    pub fn startup_order(&self) -> Result<Vec<String>, String> {
        self.validate()?;
        let mut indegree: BTreeMap<String, usize> = BTreeMap::new();
        let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for service in &self.services {
            indegree.entry(service.service_id.clone()).or_insert(0);
            for dep in &service.dependencies {
                edges
                    .entry(dep.clone())
                    .or_default()
                    .push(service.service_id.clone());
                *indegree.entry(service.service_id.clone()).or_insert(0) += 1;
            }
        }
        let mut queue = VecDeque::from_iter(
            self.services
                .iter()
                .filter(|service| indegree.get(&service.service_id).copied().unwrap_or(0) == 0)
                .map(|service| service.service_id.clone()),
        );
        let mut order = Vec::new();
        while let Some(service_id) = queue.pop_front() {
            order.push(service_id.clone());
            for dependent in edges.get(&service_id).cloned().unwrap_or_default() {
                if let Some(value) = indegree.get_mut(&dependent) {
                    *value -= 1;
                    if *value == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }
        if order.len() != self.services.len() {
            return Err("dependency cycle detected".into());
        }
        Ok(order)
    }
}
