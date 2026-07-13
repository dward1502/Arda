---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# annunimas-service-registry

**Blueprint crate for Annunimas service registry functionality**

This crate provides the foundational contract definitions, governance interfaces, and registry blueprint for the Annunimas service registry system. It is intentionally designed as a **blueprint** rather than a production-ready implementation.

---

## 📋 Overview

The `annunimas-service-registry` crate defines:

1. **Service Contracts** - Core data structures for service registration and discovery
2. **Governance Interfaces** - Validation traits for governance compliance
3. **Registry Blueprint** - In-memory registry structure and APIs
4. **Service Status Tracking** - Service lifecycle and health state management

This blueprint serves as the **design specification** for production service registry implementations in the Annunimas ecosystem.

---

## 🏗️ Architecture

```
annunimas-service-registry/
├── src/
│   ├── lib.rs              # Crate root and module declarations
│   ├── contract.rs         # Service contract definitions
│   ├── registry.rs         # Registry implementation blueprint
│   └── service.rs          # Service status and lifecycle
└── tests/
    └── contract_smoke.rs   # Integration test suite
```

---

## 📦 Contract Definition (`contract.rs`)

### Core Types

#### `ServiceContract`

```rust
pub struct ServiceContract {
    pub name: String,
    pub version: String,
    pub governance_policy: GovernancePolicy,
    pub dependencies: Vec<String>,
    pub metadata: HashMap<String, String>,
}
```

**Fields:**
- `name`: Unique service identifier
- `version`: Semantic version string
- `governance_policy`: Governance rules for this service
- `dependencies`: List of required services
- `metadata`: Additional service attributes

#### `GovernancePolicy`

```rust
pub enum GovernancePolicy {
    /// Service must pass all governance checks
    Strict,
    /// Service can operate with warnings
    Advisory,
    /// No governance restrictions
    None,
}
```

### Governance Validation

The `GovernanceValidator` trait defines the interface for contract validation:

```rust
pub trait GovernanceValidator {
    /// Validate a service contract against governance policies
    fn validate(&self, contract: &ServiceContract) -> Result<(), GovernanceError>;
    
    /// Check if service is compliant with current governance rules
    fn is_compliant(&self, contract: &ServiceContract) -> bool;
}
```

---

## 🗃️ Registry Blueprint (`registry.rs`)

### Registry Structure

```rust
pub struct Registry {
    services: HashMap<String, ServiceContract>,
    governance_validator: Box<dyn GovernanceValidator>,
}
```

**Methods:**
- `register(contract: ServiceContract)` - Add a service to the registry
- `deregister(name: &str)` - Remove a service from the registry
- `get(name: &str) -> Option<&ServiceContract>` - Retrieve a service contract
- `list() -> Vec<&ServiceContract>` - List all registered services
- `validate_all() -> Vec<ValidationResult>` - Validate all services

### Registry Operations

```rust
impl Registry {
    /// Create a new registry with default governance validator
    pub fn new() -> Self;
    
    /// Register a service contract
    pub fn register(&mut self, contract: ServiceContract) -> Result<(), RegistryError>;
    
    /// Check if a service exists
    pub fn contains(&self, name: &str) -> bool;
    
    /// Get service count
    pub fn len(&self) -> usize;
    
    /// Check if registry is empty
    pub fn is_empty(&self) -> bool;
}
```

---

## 🔄 Service Status (`service.rs`)

### Service State

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceState {
    /// Service is initializing
    Initializing,
    /// Service is healthy and operational
    Healthy,
    /// Service is degraded but functional
    Degraded,
    /// Service is unhealthy and should be restarted
    Unhealthy,
    /// Service is undergoing maintenance
    Maintenance,
    /// Service has been decommissioned
    Decommissioned,
}
```

### Service Lifecycle

```rust
pub struct ServiceStatus {
    pub contract: ServiceContract,
    pub state: ServiceState,
    pub last_heartbeat: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub error_count: u32,
}
```

**Methods:**
- `update_state(new_state: ServiceState)` - Update service state
- `record_heartbeat()` - Update last heartbeat timestamp
- `increment_error_count()` - Track failures
- `reset()` - Reset service state

---

## 🚀 Usage Examples

### Extending the Blueprint

This crate provides the **foundation**. To create a production service registry:

#### Step 1: Create Implementation Crate

```bash
cargo new annunimas-service-registry-impl --lib
cd annunimas-service-registry-impl
```

#### Step 2: Add Dependency

```toml
[dependencies]
annunimas-service-registry = { path = "../annunimas-service-registry" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
tracing = "0.1"
```

#### Step 3: Implement Governance Validator

```rust
use annunimas_service_registry::{GovernanceValidator, ServiceContract, GovernanceError};

pub struct ProductionGovernanceValidator;

impl GovernanceValidator for ProductionGovernanceValidator {
    fn validate(&self, contract: &ServiceContract) -> Result<(), GovernanceError> {
        // Add your governance rules here
        if contract.name.is_empty() {
            return Err(GovernanceError::InvalidName);
        }
        
        if contract.version.is_empty() {
            return Err(GovernanceError::InvalidVersion);
        }
        
        Ok(())
    }
    
    fn is_compliant(&self, contract: &ServiceContract) -> bool {
        self.validate(contract).is_ok()
    }
}
```

#### Step 4: Create Production Registry

```rust
use annunimas_service_registry::{Registry, ServiceContract};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create governance validator
    let validator = Arc::new(ProductionGovernanceValidator);
    
    // Create registry
    let mut registry = Registry::new();
    registry.set_governance_validator(validator);
    
    // Register services
    let service = ServiceContract {
        name: "example-service".to_string(),
        version: "1.0.0".to_string(),
        governance_policy: annunimas_service_registry::GovernancePolicy::Strict,
        dependencies: vec!["database".to_string(), "cache".to_string()],
        metadata: HashMap::new(),
    };
    
    registry.register(service).expect("Failed to register service");
    
    println!("Registry initialized with {} services", registry.len());
}
```

---

## 📊 Governance Rules

### Built-in Governance Policies

| Policy | Description | Validation Level |
|--------|-------------|------------------|
| `Strict` | All governance checks must pass | High |
| `Advisory` | Warnings allowed, but critical issues fail | Medium |
| `None` | No governance restrictions | Low |

### Custom Governance Extensions

To add custom governance rules:

```rust
impl GovernanceValidator for YourValidator {
    fn validate(&self, contract: &ServiceContract) -> Result<(), GovernanceError> {
        // Add custom validation logic
        
        // Example: Check for required metadata
        if !contract.metadata.contains_key("team") {
            return Err(GovernanceError::MissingMetadata("team".to_string()));
        }
        
        // Example: Check for minimum version
        if !is_version_compatible(&contract.version, "1.0.0") {
            return Err(GovernanceError::VersionMismatch);
        }
        
        Ok(())
    }
}
```

---

## 🧪 Testing

### Current Test Coverage

The crate includes **3 integration tests** in `tests/contract_smoke.rs`:

1. **Contract Validation** - Tests service contract creation and validation
2. **Governance Compliance** - Tests governance policy enforcement
3. **Registry Operations** - Tests basic registry functionality

### Running Tests

```bash
# Run all tests
cargo test -p annunimas-service-registry

# Run tests with output
cargo test -p annunimas-service-registry -- --nocapture

# Run specific test
cargo test -p annunimas-service-registry governance_validation
```

### Test Coverage Report

```
running 3 tests
test contract_creation ... ok
test governance_validation ... ok
test registry_operations ... ok

test result: ok. 3 passed; 0 failed; 0 ignored
```

---

## 📚 Module Documentation

### `lib.rs` - Crate Root

```rust
//! # annunimas-service-registry
//! 
//! Blueprint crate for Annunimas service registry functionality.
//! 
//! This crate provides foundational contract definitions, governance interfaces,
//! and registry blueprint for the Annunimas service registry system.
//! 
//! # Purpose
//! 
//! This is a **blueprint crate**, not a production implementation. It defines:
//! - Service contract interfaces
//! - Governance validation traits
//! - Registry structure blueprints
//! - Service lifecycle management
//! 
//! # Usage
//! 
//! This crate is meant to be extended. See the module documentation for:
//! - [`contract`] - Service contract definitions
//! - [`registry`] - Registry blueprint implementation
//! - [`service`] - Service status and lifecycle
//! 
//! To create a production registry, extend this blueprint in your own crate.

pub mod contract;
pub mod registry;
pub mod service;
```

---

## 🔧 Configuration

### Environment Variables

None currently required. The blueprint is designed to be flexible and configurable through the `ServiceContract` metadata field.

### Feature Flags

None currently defined. All functionality is available by default.

---

## 📈 Performance Characteristics

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Register service | O(1) avg | HashMap insertion |
| Deregister service | O(1) avg | HashMap removal |
| Get service | O(1) avg | HashMap lookup |
| Validate all | O(n) | Linear scan |
| List services | O(n) | Linear iteration |

**Memory Usage:** ~200 bytes per registered service

---

## 🔄 Migration Path

### From Blueprint to Implementation

```
Current State: Blueprint (annunimas-service-registry)
│
├─ If you need a simple in-memory registry:
│   └─ Use as-is (zero changes required)
│
├─ If you need persistence:
│   └─ Create annunimas-service-registry-persistent crate
│       └─ Add serde support and file/database backend
│
├─ If you need distributed registry:
│   └─ Create annunimas-service-registry-distributed crate
│       └─ Add network layer and consensus protocol
│
└─ If you need production governance:
    └─ Create annunimas-service-registry-impl crate
        └─ Implement GovernanceValidator trait with real rules
```

---

## 📖 Glossary

| Term | Definition |
|------|------------|
| **Service Contract** | Formal definition of a service's identity, version, dependencies, and governance requirements |
| **Governance Policy** | Rules that determine service compliance and operational restrictions |
| **Governance Validator** | Component that enforces governance policies on service contracts |
| **Registry** | Central store of service contracts and their current status |
| **Service State** | Current operational status of a service (healthy, degraded, etc.) |
| **Blueprint** | Design specification that is not yet production-ready |

---

## 🆘 Error Handling

### Error Types

```rust
pub enum GovernanceError {
    InvalidName,
    InvalidVersion,
    MissingMetadata(String),
    VersionMismatch,
    PolicyViolation(String),
}

pub enum RegistryError {
    ServiceAlreadyExists,
    ServiceNotFound,
    ValidationFailed(GovernanceError),
    StorageError(String),
}
```

### Error Conversion

```rust
impl From<GovernanceError> for RegistryError {
    fn from(err: GovernanceError) -> Self {
        RegistryError::ValidationFailed(err)
    }
}
```

---

## 🔗 Related Crates

- `annunimas-governance` - Governance policy definitions and enforcement
- `annunimas-core` - Core Annunimas system definitions
- `annunimas-systemd` - Typed `systemctl --user` client (supervision policy lives in `annunimas-prometheus`)

---

## 📝 Changelog

### v0.1.0 (Current)
- Initial blueprint release
- Service contract definitions
- Governance interface
- In-memory registry blueprint
- 3 integration tests

### Future Versions
- When converting to implementation:
  - Add persistence layer
  - Add network layer
  - Add async operations
  - Add monitoring hooks

---

## 🤝 Contributing

This is a **blueprint crate**. Contributions should focus on:

1. **Improving the blueprint** - Better documentation, clearer interfaces
2. **Adding examples** - Usage patterns and extension guides
3. **Identifying gaps** - Missing governance rules or contract fields

**Not for:** Production implementations (create separate crate for that)

---

## 📞 Support

For issues and questions:
- Check the [Annunimas documentation](https://github.com/yourorg/Annunimas)
- Review the [Governance Policy Guide](https://github.com/yourorg/Annunimas-governance)
- Open an issue in the [Annunimas repository](https://github.com/yourorg/Annunimas)

---

## 📄 License

This crate is licensed under the **MIT License**.

```
Copyright (c) 2026 Your Organization

Permission is hereby granted... (standard MIT license text)
```

---

**Last Updated:** 2026-04-29  
**Blueprint Status:** Active (not yet production-ready)  
**Next Review:** When governance requirements change or production implementation begins