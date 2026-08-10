# Architecture

V0.1 is one installation for one company. Authentication remains upstream/SSO-owned;
the OpenWork control plane owns authorization. Mature upstream services are configured
or wrapped behind adapters. OpenWork-specific code focuses on installation, capability
packs, policy, approvals, audit, sandboxing, diagnosis, backup, upgrade, and rollback.

The Phase 0 repository contains only the installer CLI skeleton and governance contracts.
Service implementations arrive through the linked GitHub issues and must not bypass ADRs.
