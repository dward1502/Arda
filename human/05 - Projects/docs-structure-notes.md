---
title: "Docs Structure Notes"
last_updated: 2026-05-14
soterion:
  type: project_summary
  category: summaries
  project: annunimas
  agent_access: public
  mnemosyne_priority: high
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Annunimas Documentation Structure
# Created: 2026-03-21
# Author: Hermes Agent (arandur)
# Session: audit-session-001

## Overview
This document describes the complete documentation structure for the Annunimas project,
organized by directory and purpose.

---

## Documentation Directories

### 1. docs/ - Public Documentation
**Purpose:** User-facing documentation and guides  
**Audience:** Developers, users, stakeholders

#### Structure:
```
docs/
├── architecture.md          # System architecture overview
├── (additional guides)
└── governance/
    └── AGENTS.md           # Agent roles and responsibilities
```

#### Contents:
- **Architecture Guide:** High-level system design
- **Agent Guides:** Individual agent documentation
- **Integration Guides:** How to use Annunimas
- **API Reference:** Public interfaces

---

### 2. human/ - Human-Readable Notes
**Purpose:** Session notes, audit logs, and working documents  
**Audience:** Development team, auditors

#### Structure:
```
human/
├── audit-session-notes.md     # Current session audit
├── config-notes.md            # Configuration documentation
├── library-crates-notes.md    # Library crate documentation
├── scripts-notes.md           # Script documentation
├── tests-notes.md             # Test documentation
└── (additional notes)
```

#### Contents:
- **Audit Notes:** System audits and findings
- **Configuration Notes:** Config file explanations
- **Implementation Notes:** Code documentation
- **Session Logs:** Development session records

---

### 3. scripts/ - Operational Scripts
**Purpose:** System operation and management  
**Audience:** Operators, administrators

#### Structure:
```
scripts/
└── agent_supervisor.sh        # Main orchestrator
```

#### Contents:
- **Orchestrator Scripts:** System startup/shutdown
- **Management Scripts:** Agent management
- **Monitoring Scripts:** Health checks and alerts

---

### 4. tests/ - Test Suite
**Purpose:** System validation and quality assurance  
**Audience:** Developers, QA team

#### Structure:
```
tests/
├── integration/              # Integration tests
├── unit/                     # Unit tests
├── e2e/                      # End-to-end tests
└── fixtures/                 # Test data
```

#### Contents:
- **Unit Tests:** Component-level validation
- **Integration Tests:** Interaction validation
- **E2E Tests:** System-level validation
- **Test Data:** Fixtures and scenarios

---

### 5. crates/ - Library Crates
**Purpose:** Agent implementation libraries  
**Audience:** Developers

#### Structure:
```
crates/
├── annunimas-apollo/         # Context and memory
├── annunimas-athena/         # Knowledge base
├── annunimas-charon/         # Task orchestration
├── annunimas-ceo/            # Executive decisions
├── annunimas-council/        # Collective decisions
├── annunimas-governance/     # Policy enforcement
├── annunimas-hades/          # Security monitoring
├── annunimas-hermes/         # Communication
├── annunimas-mnemosyne/      # Memory storage
└── annunimas-core/           # Runtime infrastructure
```

#### Contents:
- **Agent Implementations:** Individual agent logic
- **Support Functions:** Shared utilities
- **Types and Traits:** Shared abstractions

---

### 6. core/ - Core Configuration
**Purpose:** System configuration and registry  
**Audience:** System administrators

#### Structure:
```
core/
├── realm/                    # Realm configuration
│   ├── agents.toml          # Agent definitions
│   ├── annunimas.toml       # System config
│   └── boot.toml            # Bootstrap config
├── clients/                  # Client registry
│   └── _registry.toml       # Client definitions
├── edge/                     # Edge configuration
│   ├── model_profiles.toml  # Model configurations
│   └── targets.toml         # Target definitions
└── personal/                 # Personal configuration
    └── personal-identity.toml
```

#### Contents:
- **Agent Configurations:** Agent definitions and settings
- **Client Registry:** External client definitions
- **Edge Config:** Model and target configurations
- **System Config:** Global system settings

---

### 7. apps/ - Application Binaries
**Purpose:** Deployable applications  
**Audience:** Users, operators

#### Structure:
```
apps/
├── arda-hud/                 # ARDA HUD application
│   ├── ENHANCEMENT_PLAN.md
│   ├── HUD_EVENT_SCHEMA.md
│   └── MYTHOS_SPEC.md
└── (additional apps)
```

#### Contents:
- **Application Code:** Deployable binaries
- **Application Docs:** Per-app documentation
- **Configuration:** App-specific configs

---

## Documentation Flow

### Development Workflow
1. **Implementation** → crates/
2. **Tests** → tests/
3. **Notes** → human/
4. **Public Docs** → docs/
5. **Scripts** → scripts/

### Audit Workflow
1. **Explore** → Read existing docs
2. **Audit** → Analyze implementation
3. **Document** → Add to human/
4. **Update** → Refresh public docs

---

## Documentation Standards

### human/ Notes
- Markdown format
- Clear section headers
- Code examples where applicable
- Session timestamps
- Author attribution

### docs/ Guides
- ReStructuredText or Markdown
- Cross-references
- Table of contents
- Versioning

### tests/ Documentation
- Test purpose and scope
- Expected behavior
- Setup requirements
- Execution instructions

---

## Summary

The Annunimas documentation follows a clear separation:

1. **Public docs (docs/):** User-facing guides and references
2. **Working notes (human/):** Development session records
3. **Implementation (crates/):** Code-based implementation
4. **Configuration (core/):** System configuration files
5. **Operations (scripts/):** Operational scripts
6. **Quality (tests/):** Test suite and validation

This structure ensures:
- Clear separation of concerns
- Easy navigation and discovery
- Comprehensive coverage
- Maintainability over time

The documentation grows organically with the system, capturing both the technical implementation and the evolution of the multi-agent architecture.
