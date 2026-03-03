//! Vector store management for Azure AI Foundry Agent Service.
//!
//! Vector stores are used to store file embeddings for use with the file search tool.
//! This module provides functions to create, retrieve, list, update, and delete vector
//! stores, as well as manage files within them.
//!
//! ## Example
//!
//! ```rust,no_run
//! use azure_ai_foundry_core::client::FoundryClient;
//! use azure_ai_foundry_core::auth::FoundryCredential;
//! use azure_ai_foundry_agents::vector_store::{self, VectorStoreCreateRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = FoundryClient::builder()
//!     .endpoint("https://your-resource.services.ai.azure.com")
//!     .credential(FoundryCredential::api_key("your-key"))
//!     .build()?;
//!
//! // Create a vector store
//! let request = VectorStoreCreateRequest::builder()
//!     .name("My Knowledge Base")
//!     .build();
//!
//! let store = vector_store::create(&client, &request).await?;
//! println!("Created vector store: {}", store.id);
//!
//! // Add a file to the vector store
//! let vs_file = vector_store::add_file(&client, &store.id, "file-abc123").await?;
//! println!("Added file: {}", vs_file.id);
//!
//! // Delete the vector store
//! vector_store::delete(&client, &store.id).await?;
//! # Ok(())
//! # }
//! ```

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::FoundryResult;
use serde::{Deserialize, Serialize};

use crate::models::API_VERSION;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// The status of a vector store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorStoreStatus {
    /// The vector store has expired.
    Expired,
    /// The vector store is being processed.
    InProgress,
    /// The vector store is ready for use.
    Completed,
}

impl std::fmt::Display for VectorStoreStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Expired => "expired",
        };
        f.write_str(s)
    }
}

/// File count statistics for a vector store.
#[derive(Debug, Clone, Deserialize)]
pub struct FileCounts {
    /// Number of files currently being processed.
    pub in_progress: u32,
    /// Number of files that have been processed.
    pub completed: u32,
    /// Number of files that failed processing.
    pub failed: u32,
    /// Number of files that were cancelled.
    pub cancelled: u32,
    /// Total number of files.
    pub total: u32,
}

/// The anchor point for vector store expiration.
///
/// Currently the only accepted value is `"last_active_at"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpiresAfterAnchor {
    /// Expiration is measured from the last time the vector store was accessed.
    LastActiveAt,
}

/// Expiration configuration for a vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiresAfter {
    /// The anchor point for expiration.
    pub anchor: ExpiresAfterAnchor,
    /// Number of days until expiration.
    pub days: u32,
}

/// A vector store for file embeddings.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStore {
    /// Unique identifier for the vector store.
    pub id: String,

    /// Object type, always "vector_store".
    pub object: String,

    /// Unix timestamp when the vector store was created.
    pub created_at: u64,

    /// Name of the vector store.
    pub name: Option<String>,

    /// Total bytes used by the vector store.
    pub usage_bytes: u64,

    /// File count statistics.
    pub file_counts: FileCounts,

    /// The status of the vector store.
    pub status: VectorStoreStatus,

    /// Expiration configuration.
    pub expires_after: Option<ExpiresAfter>,

    /// Unix timestamp when the vector store will expire.
    pub expires_at: Option<u64>,

    /// Unix timestamp when the vector store was last active.
    pub last_active_at: Option<u64>,

    /// Metadata attached to the vector store.
    pub metadata: Option<serde_json::Value>,
}

/// Response from listing vector stores.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreList {
    /// Object type, always "list".
    pub object: String,

    /// List of vector stores.
    pub data: Vec<VectorStore>,

    /// ID of the first vector store in the list.
    pub first_id: Option<String>,

    /// ID of the last vector store in the list.
    pub last_id: Option<String>,

    /// Whether there are more vector stores to fetch.
    pub has_more: bool,
}

/// Response from deleting a vector store.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreDeletionResponse {
    /// ID of the deleted vector store.
    pub id: String,

    /// Object type, always "vector_store.deleted".
    pub object: String,

    /// Whether the deletion was successful.
    pub deleted: bool,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// A request to create a new vector store.
///
/// All fields are optional. Use the builder pattern to construct requests:
///
/// ```rust
/// use azure_ai_foundry_agents::vector_store::VectorStoreCreateRequest;
///
/// let request = VectorStoreCreateRequest::builder()
///     .name("My Knowledge Base")
///     .build();
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct VectorStoreCreateRequest {
    /// Optional name for the vector store.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional file IDs to add to the vector store on creation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,

    /// Optional expiration configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ExpiresAfter>,

    /// Optional metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Builder for [`VectorStoreCreateRequest`].
#[derive(Debug, Default)]
pub struct VectorStoreCreateRequestBuilder {
    name: Option<String>,
    file_ids: Option<Vec<String>>,
    expires_after: Option<ExpiresAfter>,
    metadata: Option<serde_json::Value>,
}

impl VectorStoreCreateRequest {
    /// Create a new builder for `VectorStoreCreateRequest`.
    pub fn builder() -> VectorStoreCreateRequestBuilder {
        VectorStoreCreateRequestBuilder::default()
    }
}

impl VectorStoreCreateRequestBuilder {
    /// Set the name for this vector store.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the file IDs to add on creation.
    pub fn file_ids(mut self, file_ids: Vec<String>) -> Self {
        self.file_ids = Some(file_ids);
        self
    }

    /// Set the expiration configuration.
    pub fn expires_after(mut self, expires_after: ExpiresAfter) -> Self {
        self.expires_after = Some(expires_after);
        self
    }

    /// Set metadata for this vector store.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the request. All fields are optional, so this always succeeds.
    pub fn build(self) -> VectorStoreCreateRequest {
        VectorStoreCreateRequest {
            name: self.name,
            file_ids: self.file_ids,
            expires_after: self.expires_after,
            metadata: self.metadata,
        }
    }
}

/// A request to update a vector store.
///
/// All fields are optional. Only set fields will be included in the request.
///
/// ```rust
/// use azure_ai_foundry_agents::vector_store::VectorStoreUpdateRequest;
///
/// let request = VectorStoreUpdateRequest::builder()
///     .name("Updated Name")
///     .build();
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct VectorStoreUpdateRequest {
    /// Optional new name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Optional new expiration configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ExpiresAfter>,

    /// Optional new metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Builder for [`VectorStoreUpdateRequest`].
#[derive(Debug, Default)]
pub struct VectorStoreUpdateRequestBuilder {
    name: Option<String>,
    expires_after: Option<ExpiresAfter>,
    metadata: Option<serde_json::Value>,
}

impl VectorStoreUpdateRequest {
    /// Create a new builder for `VectorStoreUpdateRequest`.
    pub fn builder() -> VectorStoreUpdateRequestBuilder {
        VectorStoreUpdateRequestBuilder::default()
    }
}

impl VectorStoreUpdateRequestBuilder {
    /// Set the new name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the new expiration configuration.
    pub fn expires_after(mut self, expires_after: ExpiresAfter) -> Self {
        self.expires_after = Some(expires_after);
        self
    }

    /// Set new metadata.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the request. All fields are optional, so this always succeeds.
    pub fn build(self) -> VectorStoreUpdateRequest {
        VectorStoreUpdateRequest {
            name: self.name,
            expires_after: self.expires_after,
            metadata: self.metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// Vector Store File types
// ---------------------------------------------------------------------------

/// The processing status of a file in a vector store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorStoreFileStatus {
    /// The file is being processed.
    InProgress,
    /// The file has been processed successfully.
    Completed,
    /// The file processing was cancelled.
    Cancelled,
    /// The file processing failed.
    Failed,
}

/// Error information for a vector store file.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreFileError {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
}

/// A file within a vector store.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreFile {
    /// Unique identifier for the vector store file.
    pub id: String,

    /// Object type, always "vector_store.file".
    pub object: String,

    /// Unix timestamp when the file was added.
    pub created_at: u64,

    /// The vector store this file belongs to.
    pub vector_store_id: String,

    /// Processing status of the file.
    pub status: VectorStoreFileStatus,

    /// Error information if processing failed.
    pub last_error: Option<VectorStoreFileError>,
}

/// Response from listing files in a vector store.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreFileList {
    /// Object type, always "list".
    pub object: String,

    /// List of vector store files.
    pub data: Vec<VectorStoreFile>,

    /// ID of the first file in the list.
    pub first_id: Option<String>,

    /// ID of the last file in the list.
    pub last_id: Option<String>,

    /// Whether there are more files to fetch.
    pub has_more: bool,
}

/// Response from deleting a file from a vector store.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreFileDeletionResponse {
    /// ID of the removed file.
    pub id: String,

    /// Object type, always "vector_store.file.deleted".
    pub object: String,

    /// Whether the deletion was successful.
    pub deleted: bool,
}

// ---------------------------------------------------------------------------
// Vector Store File Batch types
// ---------------------------------------------------------------------------

/// A batch of files being added to a vector store.
#[derive(Debug, Clone, Deserialize)]
pub struct VectorStoreFileBatch {
    /// Unique identifier for the batch.
    pub id: String,

    /// Object type, always "vector_store.files_batch".
    pub object: String,

    /// Unix timestamp when the batch was created.
    pub created_at: u64,

    /// The vector store this batch belongs to.
    pub vector_store_id: String,

    /// Processing status of the batch.
    pub status: VectorStoreFileStatus,

    /// File count statistics for the batch.
    pub file_counts: FileCounts,
}

// ---------------------------------------------------------------------------
// API functions — Vector Stores
// ---------------------------------------------------------------------------

/// Create a new vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store::{self, VectorStoreCreateRequest};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = VectorStoreCreateRequest::builder()
///     .name("My Knowledge Base")
///     .build();
///
/// let store = vector_store::create(client, &request).await?;
/// println!("Created: {}", store.id);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::create`.
#[tracing::instrument(name = "foundry::vector_stores::create", skip(client, request))]
pub async fn create(
    client: &FoundryClient,
    request: &VectorStoreCreateRequest,
) -> FoundryResult<VectorStore> {
    tracing::debug!("creating vector store");

    let path = format!("/vector_stores?{}", API_VERSION);
    let response = client.post(&path, request).await?;
    let store = response.json::<VectorStore>().await?;

    tracing::debug!(vector_store_id = %store.id, "vector store created");
    Ok(store)
}

/// Get a vector store by ID.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let store = vector_store::get(client, "vs_abc123").await?;
/// println!("Status: {:?}", store.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::get` with field `vector_store_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::get",
    skip(client),
    fields(vector_store_id = %vector_store_id)
)]
pub async fn get(client: &FoundryClient, vector_store_id: &str) -> FoundryResult<VectorStore> {
    tracing::debug!("getting vector store");
    FoundryClient::validate_resource_id(vector_store_id)?;
    let path = format!("/vector_stores/{}?{}", vector_store_id, API_VERSION);
    let response = client.get(&path).await?;
    let store = response.json::<VectorStore>().await?;

    tracing::debug!(vector_store_id = %store.id, "vector store retrieved");
    Ok(store)
}

/// List all vector stores.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let stores = vector_store::list(client).await?;
/// for s in stores.data {
///     println!("{}: {:?}", s.id, s.status);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::list`.
#[tracing::instrument(name = "foundry::vector_stores::list", skip(client))]
pub async fn list(client: &FoundryClient) -> FoundryResult<VectorStoreList> {
    tracing::debug!("listing vector stores");

    let path = format!("/vector_stores?{}", API_VERSION);
    let response = client.get(&path).await?;
    let list = response.json::<VectorStoreList>().await?;

    tracing::debug!(count = list.data.len(), "vector stores listed");
    Ok(list)
}

/// Update a vector store.
///
/// Azure AI Foundry uses POST for update operations.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store::{self, VectorStoreUpdateRequest};
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let request = VectorStoreUpdateRequest::builder()
///     .name("Updated Name")
///     .build();
///
/// let store = vector_store::update(client, "vs_abc123", &request).await?;
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::update` with field `vector_store_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::update",
    skip(client, request),
    fields(vector_store_id = %vector_store_id)
)]
pub async fn update(
    client: &FoundryClient,
    vector_store_id: &str,
    request: &VectorStoreUpdateRequest,
) -> FoundryResult<VectorStore> {
    tracing::debug!("updating vector store");
    FoundryClient::validate_resource_id(vector_store_id)?;
    let path = format!("/vector_stores/{}?{}", vector_store_id, API_VERSION);
    let response = client.post(&path, request).await?;
    let store = response.json::<VectorStore>().await?;

    tracing::debug!("vector store updated");
    Ok(store)
}

/// Delete a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let result = vector_store::delete(client, "vs_abc123").await?;
/// if result.deleted {
///     println!("Vector store deleted");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::delete` with field `vector_store_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::delete",
    skip(client),
    fields(vector_store_id = %vector_store_id)
)]
pub async fn delete(
    client: &FoundryClient,
    vector_store_id: &str,
) -> FoundryResult<VectorStoreDeletionResponse> {
    tracing::debug!("deleting vector store");
    FoundryClient::validate_resource_id(vector_store_id)?;
    let path = format!("/vector_stores/{}?{}", vector_store_id, API_VERSION);
    let response = client.delete(&path).await?;
    let result = response.json::<VectorStoreDeletionResponse>().await?;

    tracing::debug!(deleted = result.deleted, "vector store deletion complete");
    Ok(result)
}

// ---------------------------------------------------------------------------
// API functions — Vector Store Files
// ---------------------------------------------------------------------------

/// Add a file to a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let vs_file = vector_store::add_file(client, "vs_abc123", "file-xyz").await?;
/// println!("Added file: {:?}", vs_file.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::add_file` with fields `vector_store_id` and `file_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::add_file",
    skip(client),
    fields(vector_store_id = %vector_store_id, file_id = %file_id)
)]
pub async fn add_file(
    client: &FoundryClient,
    vector_store_id: &str,
    file_id: &str,
) -> FoundryResult<VectorStoreFile> {
    tracing::debug!("adding file to vector store");
    FoundryClient::validate_resource_id(vector_store_id)?;
    let path = format!("/vector_stores/{}/files?{}", vector_store_id, API_VERSION);
    let body = serde_json::json!({"file_id": file_id});
    let response = client.post(&path, &body).await?;
    let vs_file = response.json::<VectorStoreFile>().await?;

    tracing::debug!(vs_file_id = %vs_file.id, "file added to vector store");
    Ok(vs_file)
}

/// List files in a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let files = vector_store::list_files(client, "vs_abc123").await?;
/// for f in files.data {
///     println!("{}: {:?}", f.id, f.status);
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::list_files` with field `vector_store_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::list_files",
    skip(client),
    fields(vector_store_id = %vector_store_id)
)]
pub async fn list_files(
    client: &FoundryClient,
    vector_store_id: &str,
) -> FoundryResult<VectorStoreFileList> {
    tracing::debug!("listing vector store files");
    FoundryClient::validate_resource_id(vector_store_id)?;
    let path = format!("/vector_stores/{}/files?{}", vector_store_id, API_VERSION);
    let response = client.get(&path).await?;
    let list = response.json::<VectorStoreFileList>().await?;

    tracing::debug!(count = list.data.len(), "vector store files listed");
    Ok(list)
}

/// Get a file from a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let vs_file = vector_store::get_file(client, "vs_abc123", "vsfile_xyz").await?;
/// println!("Status: {:?}", vs_file.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::get_file` with fields `vector_store_id` and `file_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::get_file",
    skip(client),
    fields(vector_store_id = %vector_store_id, file_id = %file_id)
)]
pub async fn get_file(
    client: &FoundryClient,
    vector_store_id: &str,
    file_id: &str,
) -> FoundryResult<VectorStoreFile> {
    tracing::debug!("getting vector store file");
    FoundryClient::validate_resource_id(vector_store_id)?;
    FoundryClient::validate_resource_id(file_id)?;
    let path = format!(
        "/vector_stores/{}/files/{}?{}",
        vector_store_id, file_id, API_VERSION
    );
    let response = client.get(&path).await?;
    let vs_file = response.json::<VectorStoreFile>().await?;

    tracing::debug!(vs_file_id = %vs_file.id, "vector store file retrieved");
    Ok(vs_file)
}

/// Delete a file from a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let result = vector_store::delete_file(client, "vs_abc123", "vsfile_xyz").await?;
/// if result.deleted {
///     println!("File removed from vector store");
/// }
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::delete_file` with fields `vector_store_id` and `file_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::delete_file",
    skip(client),
    fields(vector_store_id = %vector_store_id, file_id = %file_id)
)]
pub async fn delete_file(
    client: &FoundryClient,
    vector_store_id: &str,
    file_id: &str,
) -> FoundryResult<VectorStoreFileDeletionResponse> {
    tracing::debug!("deleting vector store file");
    FoundryClient::validate_resource_id(vector_store_id)?;
    FoundryClient::validate_resource_id(file_id)?;
    let path = format!(
        "/vector_stores/{}/files/{}?{}",
        vector_store_id, file_id, API_VERSION
    );
    let response = client.delete(&path).await?;
    let result = response.json::<VectorStoreFileDeletionResponse>().await?;

    tracing::debug!(
        deleted = result.deleted,
        "vector store file deletion complete"
    );
    Ok(result)
}

// ---------------------------------------------------------------------------
// API functions — Vector Store File Batches
// ---------------------------------------------------------------------------

/// Create a batch of files in a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let batch = vector_store::create_file_batch(client, "vs_abc123", &["file-1", "file-2"]).await?;
/// println!("Batch: {} ({:?})", batch.id, batch.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::create_file_batch` with field `vector_store_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::create_file_batch",
    skip(client, file_ids),
    fields(vector_store_id = %vector_store_id)
)]
pub async fn create_file_batch<S: AsRef<str>>(
    client: &FoundryClient,
    vector_store_id: &str,
    file_ids: &[S],
) -> FoundryResult<VectorStoreFileBatch> {
    tracing::debug!(file_count = file_ids.len(), "creating file batch");
    FoundryClient::validate_resource_id(vector_store_id)?;
    let path = format!(
        "/vector_stores/{}/file_batches?{}",
        vector_store_id, API_VERSION
    );
    let ids: Vec<&str> = file_ids.iter().map(|s| s.as_ref()).collect();
    let body = serde_json::json!({"file_ids": ids});
    let response = client.post(&path, &body).await?;
    let batch = response.json::<VectorStoreFileBatch>().await?;

    tracing::debug!(batch_id = %batch.id, "file batch created");
    Ok(batch)
}

/// Get a file batch from a vector store.
///
/// # Example
///
/// ```rust,no_run
/// # use azure_ai_foundry_core::client::FoundryClient;
/// # use azure_ai_foundry_agents::vector_store;
/// # async fn example(client: &FoundryClient) -> azure_ai_foundry_core::error::FoundryResult<()> {
/// let batch = vector_store::get_file_batch(client, "vs_abc123", "batch_xyz").await?;
/// println!("Status: {:?}", batch.status);
/// # Ok(())
/// # }
/// ```
///
/// # Tracing
///
/// Emits a span named `foundry::vector_stores::get_file_batch` with fields `vector_store_id` and `batch_id`.
#[tracing::instrument(
    name = "foundry::vector_stores::get_file_batch",
    skip(client),
    fields(vector_store_id = %vector_store_id, batch_id = %batch_id)
)]
pub async fn get_file_batch(
    client: &FoundryClient,
    vector_store_id: &str,
    batch_id: &str,
) -> FoundryResult<VectorStoreFileBatch> {
    tracing::debug!("getting file batch");
    FoundryClient::validate_resource_id(vector_store_id)?;
    FoundryClient::validate_resource_id(batch_id)?;
    let path = format!(
        "/vector_stores/{}/file_batches/{}?{}",
        vector_store_id, batch_id, API_VERSION
    );
    let response = client.get(&path).await?;
    let batch = response.json::<VectorStoreFileBatch>().await?;

    tracing::debug!(batch_id = %batch.id, "file batch retrieved");
    Ok(batch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{setup_mock_client, TEST_TIMESTAMP};
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_file_counts() -> serde_json::Value {
        serde_json::json!({
            "in_progress": 0,
            "completed": 3,
            "failed": 0,
            "cancelled": 0,
            "total": 3
        })
    }

    fn sample_vector_store_json() -> serde_json::Value {
        serde_json::json!({
            "id": "vs_abc",
            "object": "vector_store",
            "created_at": TEST_TIMESTAMP,
            "name": "My Store",
            "usage_bytes": 12345,
            "file_counts": sample_file_counts(),
            "status": "completed",
            "expires_after": {"anchor": "last_active_at", "days": 7},
            "expires_at": 1700100000,
            "last_active_at": TEST_TIMESTAMP,
            "metadata": {"key": "value"}
        })
    }

    // --- Cycle 3.1: VectorStore deserialization ---

    #[test]
    fn test_vector_store_deserialization() {
        let store: VectorStore = serde_json::from_value(sample_vector_store_json()).unwrap();

        assert_eq!(store.id, "vs_abc");
        assert_eq!(store.object, "vector_store");
        assert_eq!(store.created_at, TEST_TIMESTAMP);
        assert_eq!(store.name, Some("My Store".into()));
        assert_eq!(store.usage_bytes, 12345);
        assert_eq!(store.file_counts.completed, 3);
        assert_eq!(store.file_counts.total, 3);
        assert_eq!(store.status, VectorStoreStatus::Completed);
        assert!(store.expires_after.is_some());
        let ea = store.expires_after.unwrap();
        assert_eq!(ea.anchor, ExpiresAfterAnchor::LastActiveAt);
        assert_eq!(ea.days, 7);
        assert_eq!(store.expires_at, Some(1700100000));
        assert_eq!(store.last_active_at, Some(TEST_TIMESTAMP));
        assert!(store.metadata.is_some());
    }

    // --- Cycle 3.2: VectorStore minimal ---

    #[test]
    fn test_vector_store_deserialization_minimal() {
        let json = serde_json::json!({
            "id": "vs_abc",
            "object": "vector_store",
            "created_at": TEST_TIMESTAMP,
            "usage_bytes": 0,
            "file_counts": {
                "in_progress": 0,
                "completed": 0,
                "failed": 0,
                "cancelled": 0,
                "total": 0
            },
            "status": "in_progress"
        });

        let store: VectorStore = serde_json::from_value(json).unwrap();

        assert_eq!(store.id, "vs_abc");
        assert!(store.name.is_none());
        assert!(store.expires_after.is_none());
        assert!(store.expires_at.is_none());
        assert!(store.last_active_at.is_none());
        assert!(store.metadata.is_none());
        assert_eq!(store.status, VectorStoreStatus::InProgress);
    }

    // --- Cycle 3.3: VectorStoreCreateRequest serialization ---

    #[test]
    fn test_vector_store_create_request_serialization() {
        let request = VectorStoreCreateRequest::builder()
            .name("Test Store")
            .file_ids(vec!["file-1".into(), "file-2".into()])
            .expires_after(ExpiresAfter {
                anchor: ExpiresAfterAnchor::LastActiveAt,
                days: 30,
            })
            .metadata(serde_json::json!({"env": "test"}))
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["name"], "Test Store");
        assert_eq!(json["file_ids"][0], "file-1");
        assert_eq!(json["file_ids"][1], "file-2");
        assert_eq!(json["expires_after"]["days"], 30);
        assert_eq!(json["metadata"]["env"], "test");
    }

    #[test]
    fn expires_after_anchor_serializes_to_last_active_at() {
        let ea = ExpiresAfter {
            anchor: ExpiresAfterAnchor::LastActiveAt,
            days: 7,
        };
        let json = serde_json::to_value(&ea).unwrap();
        assert_eq!(json["anchor"], "last_active_at");
        assert_eq!(json["days"], 7);
    }

    #[test]
    fn expires_after_anchor_deserializes_from_last_active_at() {
        let json = serde_json::json!({"anchor": "last_active_at", "days": 14});
        let ea: ExpiresAfter = serde_json::from_value(json).unwrap();
        assert_eq!(ea.anchor, ExpiresAfterAnchor::LastActiveAt);
        assert_eq!(ea.days, 14);
    }

    #[test]
    fn test_vector_store_create_request_empty() {
        let request = VectorStoreCreateRequest::builder().build();

        let json = serde_json::to_value(&request).unwrap();

        assert!(json.get("name").is_none());
        assert!(json.get("file_ids").is_none());
        assert!(json.get("expires_after").is_none());
        assert!(json.get("metadata").is_none());
    }

    // --- Cycle 3.4: create() API ---

    #[tokio::test]
    async fn test_create_vector_store_success() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores"))
            .and(header("Authorization", "Bearer test-api-key"))
            .and(body_json(serde_json::json!({"name": "Test Store"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_vector_store_json()))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = VectorStoreCreateRequest::builder()
            .name("Test Store")
            .build();

        let store = create(&client, &request).await.expect("should succeed");

        assert_eq!(store.id, "vs_abc");
        assert_eq!(store.status, VectorStoreStatus::Completed);
    }

    // --- Cycle 3.5: get() API ---

    #[tokio::test]
    async fn test_get_vector_store_success() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_abc"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_vector_store_json()))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let store = get(&client, "vs_abc").await.expect("should succeed");

        assert_eq!(store.id, "vs_abc");
        assert_eq!(store.name, Some("My Store".into()));
    }

    // --- Cycle 3.6: list() API ---

    #[tokio::test]
    async fn test_list_vector_stores_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "object": "list",
            "data": [sample_vector_store_json()],
            "first_id": "vs_abc",
            "last_id": "vs_abc",
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/vector_stores"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let stores = list(&client).await.expect("should succeed");

        assert_eq!(stores.data.len(), 1);
        assert_eq!(stores.first_id, Some("vs_abc".into()));
        assert!(!stores.has_more);
    }

    #[tokio::test]
    async fn test_list_vector_stores_empty() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "object": "list",
            "data": [],
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/vector_stores"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let stores = list(&client).await.expect("should succeed");

        assert!(stores.data.is_empty());
    }

    // --- Cycle 3.7: VectorStoreUpdateRequest serialization ---

    #[test]
    fn test_vector_store_update_request_serialization() {
        let request = VectorStoreUpdateRequest::builder()
            .name("Updated Name")
            .build();

        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["name"], "Updated Name");
        assert!(json.get("expires_after").is_none());
        assert!(json.get("metadata").is_none());
    }

    // --- Cycle 3.8: update() API (POST) ---

    #[tokio::test]
    async fn test_update_vector_store_success() {
        let server = MockServer::start().await;

        let mut updated = sample_vector_store_json();
        updated["name"] = serde_json::json!("Updated Name");

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_abc"))
            .and(body_json(serde_json::json!({"name": "Updated Name"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(&updated))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = VectorStoreUpdateRequest::builder()
            .name("Updated Name")
            .build();

        let store = update(&client, "vs_abc", &request)
            .await
            .expect("should succeed");

        assert_eq!(store.name, Some("Updated Name".into()));
    }

    // --- Cycle 3.9: delete() API ---

    #[tokio::test]
    async fn test_delete_vector_store_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "id": "vs_abc",
            "object": "vector_store.deleted",
            "deleted": true
        });

        Mock::given(method("DELETE"))
            .and(path("/vector_stores/vs_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = delete(&client, "vs_abc").await.expect("should succeed");

        assert_eq!(result.id, "vs_abc");
        assert!(result.deleted);
    }

    // --- Cycle 3.10: VectorStoreStatus serde ---

    #[test]
    fn test_vector_store_status_serde() {
        assert_eq!(
            serde_json::from_str::<VectorStoreStatus>("\"expired\"").unwrap(),
            VectorStoreStatus::Expired
        );
        assert_eq!(
            serde_json::from_str::<VectorStoreStatus>("\"in_progress\"").unwrap(),
            VectorStoreStatus::InProgress
        );
        assert_eq!(
            serde_json::from_str::<VectorStoreStatus>("\"completed\"").unwrap(),
            VectorStoreStatus::Completed
        );

        assert_eq!(
            serde_json::to_string(&VectorStoreStatus::Expired).unwrap(),
            "\"expired\""
        );
        assert_eq!(
            serde_json::to_string(&VectorStoreStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
    }

    // --- Cycle 3.11: Create with file_ids ---

    #[tokio::test]
    async fn test_create_vector_store_with_file_ids() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores"))
            .and(body_json(serde_json::json!({
                "name": "With Files",
                "file_ids": ["file-1", "file-2"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_vector_store_json()))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let request = VectorStoreCreateRequest::builder()
            .name("With Files")
            .file_ids(vec!["file-1".into(), "file-2".into()])
            .build();

        let store = create(&client, &request).await.expect("should succeed");

        assert_eq!(store.id, "vs_abc");
    }

    // --- Phase 4: Vector Store Files ---

    // --- Cycle 4.1: VectorStoreFile deserialization ---

    #[test]
    fn test_vector_store_file_deserialization() {
        let json = serde_json::json!({
            "id": "vsfile_abc",
            "object": "vector_store.file",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "completed"
        });

        let vs_file: VectorStoreFile = serde_json::from_value(json).unwrap();

        assert_eq!(vs_file.id, "vsfile_abc");
        assert_eq!(vs_file.object, "vector_store.file");
        assert_eq!(vs_file.vector_store_id, "vs_abc");
        assert_eq!(vs_file.status, VectorStoreFileStatus::Completed);
        assert!(vs_file.last_error.is_none());
    }

    #[test]
    fn test_vector_store_file_with_error() {
        let json = serde_json::json!({
            "id": "vsfile_err",
            "object": "vector_store.file",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "failed",
            "last_error": {
                "code": "processing_error",
                "message": "File too large"
            }
        });

        let vs_file: VectorStoreFile = serde_json::from_value(json).unwrap();

        assert_eq!(vs_file.status, VectorStoreFileStatus::Failed);
        assert!(vs_file.last_error.is_some());
        let err = vs_file.last_error.unwrap();
        assert_eq!(err.code, "processing_error");
        assert_eq!(err.message, "File too large");
    }

    // --- Cycle 4.2: add_file() API ---

    #[tokio::test]
    async fn test_add_file_to_vector_store_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "id": "vsfile_new",
            "object": "vector_store.file",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "in_progress"
        });

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_abc/files"))
            .and(body_json(serde_json::json!({"file_id": "file-abc"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let vs_file = add_file(&client, "vs_abc", "file-abc")
            .await
            .expect("should succeed");

        assert_eq!(vs_file.id, "vsfile_new");
        assert_eq!(vs_file.status, VectorStoreFileStatus::InProgress);
    }

    // --- Cycle 4.3: list_files() API ---

    #[tokio::test]
    async fn test_list_vector_store_files_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "object": "list",
            "data": [{
                "id": "vsfile_1",
                "object": "vector_store.file",
                "created_at": TEST_TIMESTAMP,
                "vector_store_id": "vs_abc",
                "status": "completed"
            }],
            "first_id": "vsfile_1",
            "last_id": "vsfile_1",
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_abc/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let files = list_files(&client, "vs_abc").await.expect("should succeed");

        assert_eq!(files.data.len(), 1);
        assert_eq!(files.data[0].id, "vsfile_1");
    }

    // --- Cycle 4.4: get_file() API ---

    #[tokio::test]
    async fn test_get_vector_store_file_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "id": "vsfile_abc",
            "object": "vector_store.file",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "completed"
        });

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_abc/files/vsfile_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let vs_file = get_file(&client, "vs_abc", "vsfile_abc")
            .await
            .expect("should succeed");

        assert_eq!(vs_file.id, "vsfile_abc");
    }

    // --- Cycle 4.5: delete_file() API ---

    #[tokio::test]
    async fn test_delete_vector_store_file_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "id": "vsfile_abc",
            "object": "vector_store.file.deleted",
            "deleted": true
        });

        Mock::given(method("DELETE"))
            .and(path("/vector_stores/vs_abc/files/vsfile_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let result = delete_file(&client, "vs_abc", "vsfile_abc")
            .await
            .expect("should succeed");

        assert_eq!(result.id, "vsfile_abc");
        assert!(result.deleted);
    }

    // --- Cycle 4.6: VectorStoreFileBatch deserialization ---

    #[test]
    fn test_file_batch_deserialization() {
        let json = serde_json::json!({
            "id": "batch_abc",
            "object": "vector_store.files_batch",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "in_progress",
            "file_counts": sample_file_counts()
        });

        let batch: VectorStoreFileBatch = serde_json::from_value(json).unwrap();

        assert_eq!(batch.id, "batch_abc");
        assert_eq!(batch.object, "vector_store.files_batch");
        assert_eq!(batch.vector_store_id, "vs_abc");
        assert_eq!(batch.status, VectorStoreFileStatus::InProgress);
        assert_eq!(batch.file_counts.total, 3);
    }

    // --- Cycle 4.7: create_file_batch() API ---

    #[tokio::test]
    async fn test_create_file_batch_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "id": "batch_new",
            "object": "vector_store.files_batch",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "in_progress",
            "file_counts": {
                "in_progress": 2,
                "completed": 0,
                "failed": 0,
                "cancelled": 0,
                "total": 2
            }
        });

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_abc/file_batches"))
            .and(body_json(serde_json::json!({"file_ids": ["f1", "f2"]})))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let file_ids = vec!["f1".to_string(), "f2".to_string()];
        let batch = create_file_batch(&client, "vs_abc", &file_ids)
            .await
            .expect("should succeed");

        assert_eq!(batch.id, "batch_new");
        assert_eq!(batch.file_counts.total, 2);
    }

    // --- Cycle 4.8: get_file_batch() API ---

    #[tokio::test]
    async fn test_get_file_batch_success() {
        let server = MockServer::start().await;

        let response = serde_json::json!({
            "id": "batch_abc",
            "object": "vector_store.files_batch",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "completed",
            "file_counts": sample_file_counts()
        });

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_abc/file_batches/batch_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        let batch = get_file_batch(&client, "vs_abc", "batch_abc")
            .await
            .expect("should succeed");

        assert_eq!(batch.id, "batch_abc");
        assert_eq!(batch.status, VectorStoreFileStatus::Completed);
    }

    // --- VectorStoreFileStatus serde ---

    #[test]
    fn test_vector_store_file_status_serde() {
        assert_eq!(
            serde_json::from_str::<VectorStoreFileStatus>("\"in_progress\"").unwrap(),
            VectorStoreFileStatus::InProgress
        );
        assert_eq!(
            serde_json::from_str::<VectorStoreFileStatus>("\"completed\"").unwrap(),
            VectorStoreFileStatus::Completed
        );
        assert_eq!(
            serde_json::from_str::<VectorStoreFileStatus>("\"cancelled\"").unwrap(),
            VectorStoreFileStatus::Cancelled
        );
        assert_eq!(
            serde_json::from_str::<VectorStoreFileStatus>("\"failed\"").unwrap(),
            VectorStoreFileStatus::Failed
        );
    }

    // --- Quality: create_file_batch ergonomics ---

    #[tokio::test]
    async fn test_create_file_batch_with_str_slice() {
        let server = MockServer::start().await;

        let batch_response = serde_json::json!({
            "id": "batch_ergo",
            "object": "vector_store.files_batch",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "in_progress",
            "file_counts": {
                "in_progress": 2,
                "completed": 0,
                "failed": 0,
                "cancelled": 0,
                "total": 2
            }
        });

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_abc/file_batches"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&batch_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        // Ergonomic: pass &[&str] without .to_string() conversions
        let result = create_file_batch(&client, "vs_abc", &["file-1", "file-2"]).await;

        assert!(result.is_ok());
        let batch = result.unwrap();
        assert_eq!(batch.id, "batch_ergo");
    }

    #[tokio::test]
    async fn test_create_file_batch_with_string_vec() {
        let server = MockServer::start().await;

        let batch_response = serde_json::json!({
            "id": "batch_strings",
            "object": "vector_store.files_batch",
            "created_at": TEST_TIMESTAMP,
            "vector_store_id": "vs_abc",
            "status": "in_progress",
            "file_counts": {
                "in_progress": 1,
                "completed": 0,
                "failed": 0,
                "cancelled": 0,
                "total": 1
            }
        });

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_abc/file_batches"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&batch_response))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;

        // Backward compatibility: &[String] still works
        let ids: Vec<String> = vec!["file-x".to_string()];
        let result = create_file_batch(&client, "vs_abc", &ids).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_vector_store_rejects_path_traversal() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;
        let result = get(&client, "../evil").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                azure_ai_foundry_core::error::FoundryError::Validation { .. }
            ),
            "Expected Validation error, got: {:?}",
            err
        );
    }

    // --- Cycle 6.3: Display for VectorStoreStatus ---

    #[test]
    fn test_vector_store_status_display_matches_serde() {
        let pairs = [
            (VectorStoreStatus::InProgress, "in_progress"),
            (VectorStoreStatus::Completed, "completed"),
            (VectorStoreStatus::Expired, "expired"),
        ];
        for (status, expected) in pairs {
            assert_eq!(
                status.to_string(),
                expected,
                "Display mismatch for {:?}",
                status
            );
        }
    }
}
