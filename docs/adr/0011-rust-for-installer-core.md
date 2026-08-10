# Rust for the installer core

Status: Accepted

## Context

The bootstrap must ship as a small native binary across macOS, Linux, Windows,
and WSL, with typed errors and no prerequisite language runtime.

## Decision

Use a Rust Cargo workspace with responsibility-focused crates and exactly one
product binary named `openwork`. Keep async code at I/O boundaries only. The
existing TypeScript prototype remains temporarily until command parity is proven.

## Consequences

CI must enforce formatting, clippy with warnings denied, workspace tests, and a
committed lockfile. Platform-specific behavior stays behind injectable traits.

## Security and license implications

Unsafe Rust is forbidden at workspace level. Dependencies require compatible
licenses and provenance review.

## Revisit trigger

Revisit only if a Tier 1 target cannot be supported by the Rust toolchain.
