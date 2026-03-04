//! Shared types for the Azure AI Foundry Agent Service.
//!
//! This module contains common types used across agents, threads, messages, and runs.

/// API version query parameter for all Agent Service requests.
///
/// # Note
///
/// This version string is hardcoded and is **not** affected by
/// [`FoundryClientBuilder::api_version()`](azure_ai_foundry_core::client::FoundryClientBuilder::api_version).
/// The Agents Service uses a separate versioning scheme from the model inference APIs.
/// Changing this value requires a crate-level change and will be exposed as a
/// configuration option in a future release.
pub(crate) const API_VERSION: &str = "api-version=v1";
