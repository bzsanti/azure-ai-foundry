//! Blocklist management for Azure AI Content Safety.
//!
//! Create, read, update, and delete custom text blocklists and their items.
//! Blocklists can be used with text analysis to detect custom blocked terms.

use azure_ai_foundry_core::client::FoundryClient;
use azure_ai_foundry_core::error::{FoundryError, FoundryResult};
use serde::{Deserialize, Serialize};

use crate::models::{
    CONTENT_SAFETY_API_VERSION, MAX_BLOCKLIST_NAME_LENGTH, MAX_DESCRIPTION_LENGTH,
    MAX_ITEM_TEXT_LENGTH,
};

// ---------------------------------------------------------------------------
// Blocklist types
// ---------------------------------------------------------------------------

/// Request body for creating or updating a blocklist.
///
/// The blocklist name is passed as a URL path parameter to
/// [`create_or_update_blocklist`], not in this body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlocklistUpsertRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl BlocklistUpsertRequest {
    /// Creates a new builder for `BlocklistUpsertRequest`.
    pub fn builder() -> BlocklistUpsertRequestBuilder {
        BlocklistUpsertRequestBuilder::default()
    }
}

/// Builder for [`BlocklistUpsertRequest`].
#[derive(Debug, Default)]
pub struct BlocklistUpsertRequestBuilder {
    description: Option<String>,
}

impl BlocklistUpsertRequestBuilder {
    /// Sets the blocklist description (optional, max 1024 characters).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Builds the request, returning an error if validation fails.
    pub fn try_build(self) -> FoundryResult<BlocklistUpsertRequest> {
        if let Some(ref desc) = self.description {
            if desc.chars().count() > MAX_DESCRIPTION_LENGTH {
                return Err(FoundryError::validation(format!(
                    "description exceeds maximum length of {MAX_DESCRIPTION_LENGTH} characters"
                )));
            }
        }

        Ok(BlocklistUpsertRequest {
            description: self.description,
        })
    }

    /// Builds the request, panicking if validation fails.
    ///
    /// # Panics
    ///
    /// Panics if required fields are missing or invalid. Use [`try_build`](Self::try_build)
    /// for a fallible alternative.
    pub fn build(self) -> BlocklistUpsertRequest {
        self.try_build().expect("builder validation failed")
    }
}

/// A blocklist object returned by the API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BlocklistObject {
    /// The name of the blocklist.
    #[serde(rename = "blocklistName")]
    pub blocklist_name: String,

    /// The description of the blocklist.
    pub description: Option<String>,
}

/// Paginated list of blocklists.
///
/// When `next_link` is `Some`, there are more results available. Pass the
/// URL to the client's GET method to retrieve the next page.
/// When `next_link` is `None`, this is the last page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BlocklistList {
    /// The blocklist objects in the current page.
    pub value: Vec<BlocklistObject>,

    /// Link to the next page of results. `None` indicates this is the last page.
    #[serde(rename = "nextLink", default)]
    pub next_link: Option<String>,
}

// ---------------------------------------------------------------------------
// Blocklist item types
// ---------------------------------------------------------------------------

/// Input for creating or updating a blocklist item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlocklistItemInput {
    text: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    #[serde(rename = "isRegex", skip_serializing_if = "Option::is_none")]
    is_regex: Option<bool>,
}

impl BlocklistItemInput {
    /// Creates a new builder for `BlocklistItemInput`.
    pub fn builder() -> BlocklistItemInputBuilder {
        BlocklistItemInputBuilder::default()
    }
}

/// Builder for [`BlocklistItemInput`].
#[derive(Debug, Default)]
pub struct BlocklistItemInputBuilder {
    text: Option<String>,
    description: Option<String>,
    is_regex: Option<bool>,
}

impl BlocklistItemInputBuilder {
    /// Sets the text pattern to block (required, max 128 characters).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Sets the item description (optional, max 1024 characters).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets whether the text should be treated as a regular expression.
    pub fn is_regex(mut self, is_regex: bool) -> Self {
        self.is_regex = Some(is_regex);
        self
    }

    /// Builds the item, returning an error if validation fails.
    pub fn try_build(self) -> FoundryResult<BlocklistItemInput> {
        let text = self
            .text
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| FoundryError::Builder("text is required".into()))?;

        if text.chars().count() > MAX_ITEM_TEXT_LENGTH {
            return Err(FoundryError::validation(format!(
                "text exceeds maximum length of {MAX_ITEM_TEXT_LENGTH} characters"
            )));
        }

        if let Some(ref desc) = self.description {
            if desc.chars().count() > MAX_DESCRIPTION_LENGTH {
                return Err(FoundryError::validation(format!(
                    "description exceeds maximum length of {MAX_DESCRIPTION_LENGTH} characters"
                )));
            }
        }

        Ok(BlocklistItemInput {
            text,
            description: self.description,
            is_regex: self.is_regex,
        })
    }

    /// Builds the item, panicking if validation fails.
    ///
    /// # Panics
    ///
    /// Panics if required fields are missing or invalid. Use [`try_build`](Self::try_build)
    /// for a fallible alternative.
    pub fn build(self) -> BlocklistItemInput {
        self.try_build().expect("builder validation failed")
    }
}

/// Request body for adding or updating blocklist items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddOrUpdateBlocklistItemsRequest {
    #[serde(rename = "blocklistItems")]
    blocklist_items: Vec<BlocklistItemInput>,
}

impl AddOrUpdateBlocklistItemsRequest {
    /// Creates a new request from a list of items.
    ///
    /// # Errors
    ///
    /// Returns an error if the items list is empty.
    pub fn new(items: Vec<BlocklistItemInput>) -> FoundryResult<Self> {
        if items.is_empty() {
            return Err(FoundryError::Builder(
                "blocklist_items must not be empty".into(),
            ));
        }
        Ok(Self {
            blocklist_items: items,
        })
    }
}

/// A blocklist item object returned by the API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BlocklistItemObject {
    /// The unique ID of the blocklist item.
    #[serde(rename = "blocklistItemId")]
    pub blocklist_item_id: String,

    /// The text pattern of this item.
    pub text: String,

    /// The description of this item.
    pub description: Option<String>,

    /// Whether this item uses regex matching.
    #[serde(rename = "isRegex")]
    pub is_regex: bool,
}

/// Response from adding or updating blocklist items.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AddOrUpdateBlocklistItemsResponse {
    /// The created or updated blocklist items.
    #[serde(rename = "blocklistItems")]
    pub blocklist_items: Vec<BlocklistItemObject>,
}

/// Paginated list of blocklist items.
///
/// When `next_link` is `Some`, there are more results available. Pass the
/// URL to the client's GET method to retrieve the next page.
/// When `next_link` is `None`, this is the last page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BlocklistItemList {
    /// The blocklist items in the current page.
    pub value: Vec<BlocklistItemObject>,

    /// Link to the next page of results. `None` indicates this is the last page.
    #[serde(rename = "nextLink", default)]
    pub next_link: Option<String>,
}

// ---------------------------------------------------------------------------
// Blocklist CRUD functions
// ---------------------------------------------------------------------------

/// Create or update a text blocklist.
///
/// Uses HTTP PATCH with `application/merge-patch+json` content type.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `name` - The blocklist name (used in the URL path).
/// * `request` - The upsert request body.
///
/// # Example
///
/// ```rust,no_run
/// use azure_ai_foundry_core::client::FoundryClient;
/// use azure_ai_foundry_core::auth::FoundryCredential;
/// use azure_ai_foundry_safety::blocklist::{self, BlocklistUpsertRequest};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = FoundryClient::builder()
///     .endpoint("https://your-resource.cognitiveservices.azure.com")
///     .credential(FoundryCredential::api_key("your-key"))
///     .build()?;
///
/// let request = BlocklistUpsertRequest::builder()
///     .description("Common profanity terms")
///     .build();
///
/// let blocklist = blocklist::create_or_update_blocklist(&client, "profanity", &request).await?;
/// println!("Blocklist: {}", blocklist.blocklist_name);
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if the name contains path-injection characters, authentication
/// fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::create_or_update_blocklist",
    skip(client, request),
    fields(blocklist_name = %name)
)]
pub async fn create_or_update_blocklist(
    client: &FoundryClient,
    name: &str,
    request: &BlocklistUpsertRequest,
) -> FoundryResult<BlocklistObject> {
    FoundryClient::validate_resource_id(name)?;
    if name.chars().count() > MAX_BLOCKLIST_NAME_LENGTH {
        return Err(FoundryError::validation(format!(
            "blocklist name exceeds maximum length of {MAX_BLOCKLIST_NAME_LENGTH} characters"
        )));
    }
    tracing::debug!("creating or updating blocklist");

    let path = format!("/contentsafety/text/blocklists/{name}?{CONTENT_SAFETY_API_VERSION}");
    let response = client.patch(&path, request).await?;
    let result = response.json::<BlocklistObject>().await?;

    tracing::debug!("blocklist upsert complete");
    Ok(result)
}

/// Get a text blocklist by name.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `name` - The blocklist name.
///
/// # Errors
///
/// Returns an error if the name contains path-injection characters, authentication
/// fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::get_blocklist",
    skip(client),
    fields(blocklist_name = %name)
)]
pub async fn get_blocklist(client: &FoundryClient, name: &str) -> FoundryResult<BlocklistObject> {
    FoundryClient::validate_resource_id(name)?;
    tracing::debug!("getting blocklist");

    let path = format!("/contentsafety/text/blocklists/{name}?{CONTENT_SAFETY_API_VERSION}");
    let response = client.get(&path).await?;
    let result = response.json::<BlocklistObject>().await?;

    tracing::debug!("blocklist retrieved");
    Ok(result)
}

/// Delete a text blocklist by name.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `name` - The blocklist name to delete.
///
/// # Errors
///
/// Returns an error if the name contains path-injection characters, authentication
/// fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::delete_blocklist",
    skip(client),
    fields(blocklist_name = %name)
)]
pub async fn delete_blocklist(client: &FoundryClient, name: &str) -> FoundryResult<()> {
    FoundryClient::validate_resource_id(name)?;
    tracing::debug!("deleting blocklist");

    let path = format!("/contentsafety/text/blocklists/{name}?{CONTENT_SAFETY_API_VERSION}");
    let _response = client.delete(&path).await?;

    tracing::debug!("blocklist deleted");
    Ok(())
}

/// List all text blocklists.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
///
/// # Errors
///
/// Returns an error if authentication fails or the API returns an error response.
#[tracing::instrument(name = "foundry::safety::list_blocklists", skip(client))]
pub async fn list_blocklists(client: &FoundryClient) -> FoundryResult<BlocklistList> {
    tracing::debug!("listing blocklists");

    let path = format!("/contentsafety/text/blocklists?{CONTENT_SAFETY_API_VERSION}");
    let response = client.get(&path).await?;
    let result = response.json::<BlocklistList>().await?;

    tracing::debug!("blocklists listed");
    Ok(result)
}

// ---------------------------------------------------------------------------
// Blocklist item functions
// ---------------------------------------------------------------------------

/// Add or update items in a blocklist.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `blocklist_name` - The blocklist to modify.
/// * `request` - The items to add or update.
///
/// # Errors
///
/// Returns an error if the name contains path-injection characters, authentication
/// fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::add_or_update_blocklist_items",
    skip(client, request),
    fields(blocklist_name = %blocklist_name)
)]
pub async fn add_or_update_blocklist_items(
    client: &FoundryClient,
    blocklist_name: &str,
    request: &AddOrUpdateBlocklistItemsRequest,
) -> FoundryResult<AddOrUpdateBlocklistItemsResponse> {
    FoundryClient::validate_resource_id(blocklist_name)?;
    tracing::debug!("adding or updating blocklist items");

    let path = format!(
        "/contentsafety/text/blocklists/{blocklist_name}:addOrUpdateBlocklistItems?{CONTENT_SAFETY_API_VERSION}"
    );
    let response = client.post(&path, request).await?;
    let result = response.json::<AddOrUpdateBlocklistItemsResponse>().await?;

    tracing::debug!("blocklist items upserted");
    Ok(result)
}

/// Get a specific blocklist item by ID.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `blocklist_name` - The blocklist containing the item.
/// * `item_id` - The item ID to retrieve.
///
/// # Errors
///
/// Returns an error if any ID contains path-injection characters, authentication
/// fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::get_blocklist_item",
    skip(client),
    fields(blocklist_name = %blocklist_name, item_id = %item_id)
)]
pub async fn get_blocklist_item(
    client: &FoundryClient,
    blocklist_name: &str,
    item_id: &str,
) -> FoundryResult<BlocklistItemObject> {
    FoundryClient::validate_resource_id(blocklist_name)?;
    FoundryClient::validate_resource_id(item_id)?;
    tracing::debug!("getting blocklist item");

    let path = format!(
        "/contentsafety/text/blocklists/{blocklist_name}/blocklistItems/{item_id}?{CONTENT_SAFETY_API_VERSION}"
    );
    let response = client.get(&path).await?;
    let result = response.json::<BlocklistItemObject>().await?;

    tracing::debug!("blocklist item retrieved");
    Ok(result)
}

/// List all items in a blocklist.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `blocklist_name` - The blocklist to list items from.
///
/// # Errors
///
/// Returns an error if the name contains path-injection characters, authentication
/// fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::list_blocklist_items",
    skip(client),
    fields(blocklist_name = %blocklist_name)
)]
pub async fn list_blocklist_items(
    client: &FoundryClient,
    blocklist_name: &str,
) -> FoundryResult<BlocklistItemList> {
    FoundryClient::validate_resource_id(blocklist_name)?;
    tracing::debug!("listing blocklist items");

    let path = format!(
        "/contentsafety/text/blocklists/{blocklist_name}/blocklistItems?{CONTENT_SAFETY_API_VERSION}"
    );
    let response = client.get(&path).await?;
    let result = response.json::<BlocklistItemList>().await?;

    tracing::debug!("blocklist items listed");
    Ok(result)
}

/// Remove items from a blocklist by their IDs.
///
/// # Arguments
///
/// * `client` - The configured `FoundryClient`.
/// * `blocklist_name` - The blocklist to remove items from.
/// * `item_ids` - The IDs of items to remove (must not be empty).
///
/// # Errors
///
/// Returns an error if item_ids is empty, the name contains path-injection characters,
/// authentication fails, or the API returns an error response.
#[tracing::instrument(
    name = "foundry::safety::remove_blocklist_items",
    skip(client, item_ids),
    fields(blocklist_name = %blocklist_name)
)]
pub async fn remove_blocklist_items(
    client: &FoundryClient,
    blocklist_name: &str,
    item_ids: impl IntoIterator<Item = impl AsRef<str>>,
) -> FoundryResult<()> {
    FoundryClient::validate_resource_id(blocklist_name)?;

    let id_strings: Vec<String> = item_ids
        .into_iter()
        .map(|s| s.as_ref().to_string())
        .collect();
    if id_strings.is_empty() {
        return Err(FoundryError::validation("item_ids must not be empty"));
    }
    tracing::debug!("removing blocklist items");

    let path = format!(
        "/contentsafety/text/blocklists/{blocklist_name}:removeBlocklistItems?{CONTENT_SAFETY_API_VERSION}"
    );
    let body = serde_json::json!({
        "blocklistItemIds": id_strings
    });
    let _response = client.post(&path, &body).await?;

    tracing::debug!("blocklist items removed");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_mock_client;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // -----------------------------------------------------------------------
    // M8: Blocklist CRUD
    // -----------------------------------------------------------------------

    // -- BlocklistUpsertRequest builder --

    #[test]
    fn test_blocklist_upsert_body_does_not_contain_blocklist_name() {
        let request = BlocklistUpsertRequest::builder()
            .description("My filter")
            .build();
        let json = serde_json::to_value(&request).unwrap();
        assert!(
            json.get("blocklistName").is_none(),
            "blocklistName must not be in body, got: {json}"
        );
    }

    #[test]
    fn test_blocklist_upsert_accepts_description_none() {
        let result = BlocklistUpsertRequest::builder().try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocklist_upsert_accepts_description_some() {
        let result = BlocklistUpsertRequest::builder()
            .description("Profanity filter")
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocklist_object_deserialization() {
        let json = r#"{"blocklistName": "profanity", "description": "A profanity list"}"#;
        let obj: BlocklistObject = serde_json::from_str(json).unwrap();
        assert_eq!(obj.blocklist_name, "profanity");
        assert_eq!(obj.description.as_deref(), Some("A profanity list"));
    }

    // -- create_or_update_blocklist --

    #[tokio::test]
    async fn test_create_or_update_blocklist_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("PATCH"))
            .and(path("/contentsafety/text/blocklists/profanity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistName": "profanity",
                "description": "Profanity filter"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let request = BlocklistUpsertRequest::builder()
            .description("Profanity filter")
            .build();

        let result = create_or_update_blocklist(&client, "profanity", &request)
            .await
            .expect("should succeed");
        assert_eq!(result.blocklist_name, "profanity");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_create_or_update_blocklist_emits_span() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("PATCH"))
            .and(path("/contentsafety/text/blocklists/profanity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistName": "profanity",
                "description": null
            })))
            .mount(&server)
            .await;

        let request = BlocklistUpsertRequest::builder().build();
        let _ = create_or_update_blocklist(&client, "profanity", &request).await;
        assert!(logs_contain("foundry::safety::create_or_update_blocklist"));
    }

    #[tokio::test]
    async fn test_create_or_update_blocklist_sends_api_version_query_param() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("PATCH"))
            .and(path("/contentsafety/text/blocklists/test-list"))
            .and(query_param("api-version", "2024-09-01"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistName": "test-list",
                "description": null
            })))
            .expect(1)
            .mount(&server)
            .await;

        let request = BlocklistUpsertRequest::builder().build();
        let result = create_or_update_blocklist(&client, "test-list", &request).await;
        assert!(
            result.is_ok(),
            "request should match with api-version query param"
        );
    }

    #[tokio::test]
    async fn test_create_or_update_blocklist_rejects_path_traversal() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let request = BlocklistUpsertRequest::builder().build();

        let err = create_or_update_blocklist(&client, "../etc", &request)
            .await
            .expect_err("should reject path traversal");
        assert!(
            matches!(err, FoundryError::Validation { .. }),
            "expected Validation, got: {err:?}"
        );
    }

    // -- get_blocklist --

    #[tokio::test]
    async fn test_get_blocklist_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(path("/contentsafety/text/blocklists/profanity"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistName": "profanity",
                "description": null
            })))
            .expect(1)
            .mount(&server)
            .await;

        let result = get_blocklist(&client, "profanity")
            .await
            .expect("should succeed");
        assert_eq!(result.blocklist_name, "profanity");
        assert!(result.description.is_none());
    }

    #[tokio::test]
    async fn test_get_blocklist_rejects_invalid_name() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let err = get_blocklist(&client, "bad/name")
            .await
            .expect_err("should reject");
        assert!(matches!(err, FoundryError::Validation { .. }));
    }

    // -- delete_blocklist --

    #[tokio::test]
    async fn test_delete_blocklist_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("DELETE"))
            .and(path("/contentsafety/text/blocklists/profanity"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        delete_blocklist(&client, "profanity")
            .await
            .expect("should succeed");
    }

    // -- list_blocklists --

    #[tokio::test]
    async fn test_list_blocklists_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(path("/contentsafety/text/blocklists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": [
                    {"blocklistName": "profanity", "description": "Bad words"},
                    {"blocklistName": "slurs", "description": null}
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let result = list_blocklists(&client).await.expect("should succeed");
        assert_eq!(result.value.len(), 2);
        assert!(result.next_link.is_none());
    }

    #[tokio::test]
    async fn test_list_blocklists_with_next_link() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(path("/contentsafety/text/blocklists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": [{"blocklistName": "list1", "description": null}],
                "nextLink": "https://example.com/next"
            })))
            .mount(&server)
            .await;

        let result = list_blocklists(&client).await.expect("should succeed");
        assert_eq!(result.value.len(), 1);
        assert_eq!(
            result.next_link.as_deref(),
            Some("https://example.com/next")
        );
    }

    #[tokio::test]
    async fn test_list_blocklists_empty() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(path("/contentsafety/text/blocklists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": []
            })))
            .mount(&server)
            .await;

        let result = list_blocklists(&client).await.expect("should succeed");
        assert!(result.value.is_empty());
    }

    // -----------------------------------------------------------------------
    // M9: Blocklist items
    // -----------------------------------------------------------------------

    // -- BlocklistItemInput builder --

    #[test]
    fn test_blocklist_item_input_requires_text() {
        let result = BlocklistItemInput::builder().try_build();
        let err = result.expect_err("should require text");
        assert!(err.to_string().contains("text"), "error: {err}");
    }

    #[test]
    fn test_blocklist_item_input_rejects_blank_text() {
        let result = BlocklistItemInput::builder().text("  ").try_build();
        let err = result.expect_err("should reject blank text");
        assert!(err.to_string().contains("text"), "error: {err}");
    }

    #[test]
    fn test_blocklist_item_input_serialization() {
        let item = BlocklistItemInput::builder().text("badword").build();

        let json = serde_json::to_value(&item).expect("should serialize");
        assert_eq!(json["text"], "badword");
        assert!(json.get("isRegex").is_none());
        assert!(json.get("description").is_none());
    }

    #[test]
    fn test_blocklist_item_input_serialization_with_regex() {
        let item = BlocklistItemInput::builder()
            .text("bad.*word")
            .description("Regex pattern")
            .is_regex(true)
            .build();

        let json = serde_json::to_value(&item).expect("should serialize");
        assert_eq!(json["text"], "bad.*word");
        assert_eq!(json["isRegex"], true);
        assert_eq!(json["description"], "Regex pattern");
    }

    #[test]
    fn test_add_items_request_requires_non_empty_items() {
        let result = AddOrUpdateBlocklistItemsRequest::new(vec![]);
        let err = result.expect_err("should require non-empty items");
        assert!(err.to_string().contains("empty"), "error: {err}");
    }

    #[test]
    fn test_blocklist_item_object_deserialization() {
        let json = r#"{
            "blocklistItemId": "item-uuid-123",
            "text": "badword",
            "description": "A bad word",
            "isRegex": false
        }"#;
        let obj: BlocklistItemObject = serde_json::from_str(json).unwrap();
        assert_eq!(obj.blocklist_item_id, "item-uuid-123");
        assert_eq!(obj.text, "badword");
        assert_eq!(obj.description.as_deref(), Some("A bad word"));
        assert!(!obj.is_regex);
    }

    // -- add_or_update_blocklist_items --

    #[tokio::test]
    async fn test_add_items_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path(
                "/contentsafety/text/blocklists/profanity:addOrUpdateBlocklistItems",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistItems": [{
                    "blocklistItemId": "item-1",
                    "text": "badword",
                    "description": null,
                    "isRegex": false
                }]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let item = BlocklistItemInput::builder().text("badword").build();
        let request = AddOrUpdateBlocklistItemsRequest::new(vec![item]).unwrap();

        let result = add_or_update_blocklist_items(&client, "profanity", &request)
            .await
            .expect("should succeed");
        assert_eq!(result.blocklist_items.len(), 1);
        assert_eq!(result.blocklist_items[0].text, "badword");
    }

    #[tokio::test]
    async fn test_add_items_rejects_invalid_name() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let item = BlocklistItemInput::builder().text("test").build();
        let request = AddOrUpdateBlocklistItemsRequest::new(vec![item]).unwrap();

        let err = add_or_update_blocklist_items(&client, "../bad", &request)
            .await
            .expect_err("should reject");
        assert!(matches!(err, FoundryError::Validation { .. }));
    }

    // -- get_blocklist_item --

    #[tokio::test]
    async fn test_get_blocklist_item_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(path(
                "/contentsafety/text/blocklists/profanity/blocklistItems/item-1",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistItemId": "item-1",
                "text": "badword",
                "description": null,
                "isRegex": false
            })))
            .expect(1)
            .mount(&server)
            .await;

        let result = get_blocklist_item(&client, "profanity", "item-1")
            .await
            .expect("should succeed");
        assert_eq!(result.blocklist_item_id, "item-1");
    }

    #[tokio::test]
    async fn test_get_blocklist_item_rejects_invalid_item_id() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let err = get_blocklist_item(&client, "profanity", "bad/id")
            .await
            .expect_err("should reject");
        assert!(matches!(err, FoundryError::Validation { .. }));
    }

    // -- list_blocklist_items --

    #[tokio::test]
    async fn test_list_blocklist_items_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("GET"))
            .and(path(
                "/contentsafety/text/blocklists/profanity/blocklistItems",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": [
                    {"blocklistItemId": "item-1", "text": "bad", "description": null, "isRegex": false},
                    {"blocklistItemId": "item-2", "text": "worse", "description": null, "isRegex": true}
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let result = list_blocklist_items(&client, "profanity")
            .await
            .expect("should succeed");
        assert_eq!(result.value.len(), 2);
    }

    // -- remove_blocklist_items --

    #[tokio::test]
    async fn test_remove_blocklist_items_success() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        Mock::given(method("POST"))
            .and(path(
                "/contentsafety/text/blocklists/profanity:removeBlocklistItems",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&server)
            .await;

        remove_blocklist_items(&client, "profanity", &["item-1", "item-2"])
            .await
            .expect("should succeed");
    }

    #[tokio::test]
    async fn test_remove_blocklist_items_rejects_empty_ids() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;

        let empty: &[&str] = &[];
        let err = remove_blocklist_items(&client, "profanity", empty)
            .await
            .expect_err("should reject empty ids");
        assert!(
            matches!(err, FoundryError::Validation { .. }),
            "expected Validation, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn test_remove_blocklist_items_validates_name_before_empty_check() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;
        let empty: &[&str] = &[];
        // With invalid name AND empty ids, error should be about the name, not empty ids
        let err = remove_blocklist_items(&client, "bad/name", empty)
            .await
            .expect_err("should fail");
        let msg = err.to_string();
        assert!(
            !msg.contains("item_ids"),
            "should fail on name validation first, not empty ids; got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_create_or_update_blocklist_rejects_name_too_long() {
        let server = MockServer::start().await;
        let client = setup_mock_client(&server).await;
        let long_name = "a".repeat(MAX_BLOCKLIST_NAME_LENGTH + 1);
        let request = BlocklistUpsertRequest::builder().build();
        let err = create_or_update_blocklist(&client, &long_name, &request)
            .await
            .expect_err("should reject name > 64 chars");
        assert!(
            matches!(err, FoundryError::Validation { .. }),
            "expected Validation, got: {err:?}"
        );
        assert!(err.to_string().contains("maximum length"), "error: {err}");
    }

    #[tokio::test]
    async fn test_create_or_update_blocklist_accepts_name_at_boundary() {
        let server = MockServer::start().await;
        let name = "a".repeat(MAX_BLOCKLIST_NAME_LENGTH);

        Mock::given(method("PATCH"))
            .and(path(format!("/contentsafety/text/blocklists/{name}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "blocklistName": name,
                "description": null
            })))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let request = BlocklistUpsertRequest::builder().build();
        let result = create_or_update_blocklist(&client, &name, &request).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocklist_upsert_rejects_description_too_long() {
        let long_desc = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
        let err = BlocklistUpsertRequest::builder()
            .description(long_desc)
            .try_build()
            .expect_err("should reject description > 1024 chars");
        assert!(
            matches!(err, FoundryError::Validation { .. }),
            "expected Validation, got: {err:?}"
        );
    }

    #[test]
    fn test_blocklist_item_rejects_text_too_long() {
        let long_text = "a".repeat(129);
        let err = BlocklistItemInput::builder()
            .text(long_text)
            .try_build()
            .expect_err("should reject text > 128 chars");
        assert!(
            matches!(err, FoundryError::Validation { .. }),
            "expected Validation, got: {err:?}"
        );
    }

    #[test]
    fn test_blocklist_item_accepts_text_at_boundary() {
        let text = "a".repeat(128);
        let result = BlocklistItemInput::builder().text(text).try_build();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_blocklist_items_accepts_string_vec() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/contentsafety/text/blocklists/profanity:removeBlocklistItems",
            ))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let client = setup_mock_client(&server).await;
        let ids = vec!["id1".to_string(), "id2".to_string()];
        let result = remove_blocklist_items(&client, "profanity", ids).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocklist_item_description_rejects_too_long() {
        let long_desc = "a".repeat(MAX_DESCRIPTION_LENGTH + 1);
        let err = BlocklistItemInput::builder()
            .text("badword")
            .description(long_desc)
            .try_build()
            .expect_err("should reject description > 1024 chars");
        assert!(
            matches!(err, FoundryError::Validation { .. }),
            "expected Validation, got: {err:?}"
        );
        assert!(err.to_string().contains("description"), "error: {err}");
    }

    #[test]
    fn test_blocklist_item_description_accepts_boundary() {
        let boundary_desc = "a".repeat(MAX_DESCRIPTION_LENGTH);
        let result = BlocklistItemInput::builder()
            .text("badword")
            .description(boundary_desc)
            .try_build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_blocklist_object_partial_eq() {
        let json = r#"{"blocklistName": "list", "description": null}"#;
        let b1: BlocklistObject = serde_json::from_str(json).unwrap();
        let b2: BlocklistObject = serde_json::from_str(json).unwrap();
        assert_eq!(b1, b2);
    }
}
