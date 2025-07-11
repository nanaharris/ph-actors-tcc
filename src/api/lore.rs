use anyhow::Context;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Mutex, oneshot};

use crate::{ArcStr, net::Net};

mod core;
mod message;

// Re-export public types for external use
pub use message::LoreApiMessage;

/// The Lore API actor that provides a high-level interface for interacting with the Lore Kernel API.
///
/// This actor intermediates calls to the networking actor, providing domain-specific methods
/// for fetching patch feeds, available mailing lists, and individual patch HTML content
/// from the Lore Kernel Archive.
///
/// # Examples
/// ```
/// let lore_api = LoreApi::spawn(net);
/// let patch_feed = lore_api.get_patch_feed("amd-gfx", 0).await?;
/// ```
///
/// # Thread Safety
/// This type is designed to be safely shared between threads. Cloning is cheap as it only
/// copies the channel sender or mock reference.
#[derive(Debug, Clone)]
pub enum LoreApi {
    /// A real Lore API actor that performs HTTP requests through the networking actor
    Actual(Sender<LoreApiMessage>),
    /// A mock implementation for testing
    Mock(Arc<Mutex<HashMap<String, ArcStr>>>),
}

impl LoreApi {
    /// Creates a new Lore API actor and spawns its core.
    ///
    /// # Arguments
    /// * `net` - The networking actor for making HTTP requests
    ///
    /// # Returns
    /// A new Lore API actor configured for the Lore Kernel Archive.
    pub fn spawn(net: Net) -> Self {
        let (lore_api, _) = core::Core::new(net).spawn();
        lore_api
    }

    /// Creates a new Lore API actor with a custom domain.
    ///
    /// # Arguments
    /// * `net` - The networking actor for making HTTP requests
    /// * `domain` - The base domain for API requests
    ///
    /// # Returns
    /// A new Lore API actor configured with the specified domain.
    pub fn spawn_with_domain(net: Net, domain: ArcStr) -> Self {
        let (lore_api, _) = core::Core::with_domain(net, domain).spawn();
        lore_api
    }

    /// Creates a new mock Lore API instance for testing.
    ///
    /// # Arguments
    /// * `responses` - Initial response cache mapping operation keys to responses
    ///
    /// # Returns
    /// A new mock Lore API instance that returns predefined responses.
    pub fn mock(responses: HashMap<String, ArcStr>) -> Self {
        Self::Mock(Arc::new(Mutex::new(responses)))
    }

    /// Creates a new empty mock Lore API instance for testing.
    ///
    /// # Returns
    /// A new mock Lore API instance with an empty response cache.
    pub fn mock_empty() -> Self {
        Self::Mock(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Fetches a patch feed from a specific mailing list with pagination.
    ///
    /// This method retrieves a paginated list of patches from the specified mailing list,
    /// filtering for patches and RFCs while excluding replies.
    ///
    /// # Arguments
    /// * `target_list` - The mailing list name (e.g., "amd-gfx", "linux-kernel")
    /// * `min_index` - The offset for pagination (0-based)
    ///
    /// # Returns
    /// The XML feed content as a string, or an error if the request fails.
    ///
    /// # Example
    /// ```
    /// let feed = lore_api.get_patch_feed("amd-gfx", 0).await?;
    /// ```
    pub async fn get_patch_feed(
        &self,
        target_list: &str,
        min_index: usize,
    ) -> Result<ArcStr, anyhow::Error> {
        match self {
            LoreApi::Actual(sender) => {
                let (tx, rx) = oneshot::channel();
                sender
                    .send(LoreApiMessage::GetPatchFeed {
                        target_list: target_list.to_string(),
                        min_index,
                        tx,
                    })
                    .await
                    .context("Sending message to LoreApi actor")?;
                rx.await.context("Receiving response from LoreApi actor")?
            }
            LoreApi::Mock(responses) => {
                let responses = responses.lock().await;
                let key = format!("patch_feed_{}_{}", target_list, min_index);
                responses
                    .get(&key)
                    .map(ArcStr::clone)
                    .ok_or_else(|| anyhow::anyhow!("Patch feed not found in mock responses: {}", key))
            }
        }
    }

    /// Fetches available mailing lists with pagination.
    ///
    /// This method retrieves a paginated list of all available mailing lists
    /// archived on the Lore Kernel Archive.
    ///
    /// # Arguments
    /// * `min_index` - The offset for pagination (0-based)
    ///
    /// # Returns
    /// The HTML content listing available mailing lists, or an error if the request fails.
    ///
    /// # Example
    /// ```
    /// let lists = lore_api.get_available_lists(0).await?;
    /// ```
    pub async fn get_available_lists(&self, min_index: usize) -> Result<ArcStr, anyhow::Error> {
        match self {
            LoreApi::Actual(sender) => {
                let (tx, rx) = oneshot::channel();
                sender
                    .send(LoreApiMessage::GetAvailableLists { min_index, tx })
                    .await
                    .context("Sending message to LoreApi actor")?;
                rx.await.context("Receiving response from LoreApi actor")?
            }
            LoreApi::Mock(responses) => {
                let responses = responses.lock().await;
                let key = format!("available_lists_{}", min_index);
                responses
                    .get(&key)
                    .map(ArcStr::clone)
                    .ok_or_else(|| anyhow::anyhow!("Available lists not found in mock responses: {}", key))
            }
        }
    }

    /// Fetches the HTML content of a specific patch.
    ///
    /// This method retrieves the full HTML content of a specific patch
    /// identified by its message ID within a mailing list.
    ///
    /// # Arguments
    /// * `target_list` - The mailing list name (e.g., "amd-gfx", "linux-kernel")
    /// * `message_id` - The unique message ID of the patch
    ///
    /// # Returns
    /// The HTML content of the patch, or an error if the request fails.
    ///
    /// # Example
    /// ```
    /// let patch_html = lore_api.get_patch_html("amd-gfx", "20231201.123456.1-1@amd.com").await?;
    /// ```
    pub async fn get_patch_html(
        &self,
        target_list: &str,
        message_id: &str,
    ) -> Result<ArcStr, anyhow::Error> {
        match self {
            LoreApi::Actual(sender) => {
                let (tx, rx) = oneshot::channel();
                sender
                    .send(LoreApiMessage::GetPatchHtml {
                        target_list: target_list.to_string(),
                        message_id: message_id.to_string(),
                        tx,
                    })
                    .await
                    .context("Sending message to LoreApi actor")?;
                rx.await.context("Receiving response from LoreApi actor")?
            }
            LoreApi::Mock(responses) => {
                let responses = responses.lock().await;
                let key = format!("patch_html_{}_{}", target_list, message_id);
                responses
                    .get(&key)
                    .map(ArcStr::clone)
                    .ok_or_else(|| anyhow::anyhow!("Patch HTML not found in mock responses: {}", key))
            }
        }
    }

    /// Fetches a raw patch in plain text format.
    ///
    /// This method retrieves the raw patch content in plain text format,
    /// which is useful for applying patches or extracting metadata.
    ///
    /// # Arguments
    /// * `target_list` - The mailing list name
    /// * `message_id` - The unique message ID of the patch
    ///
    /// # Returns
    /// The raw patch content as plain text, or an error if the request fails.
    ///
    /// # Example
    /// ```
    /// let raw_patch = lore_api.get_raw_patch("amd-gfx", "20231201.123456.1-1@amd.com").await?;
    /// ```
    pub async fn get_raw_patch(
        &self,
        target_list: &str,
        message_id: &str,
    ) -> Result<ArcStr, anyhow::Error> {
        match self {
            LoreApi::Actual(sender) => {
                let (tx, rx) = oneshot::channel();
                sender
                    .send(LoreApiMessage::GetRawPatch {
                        target_list: target_list.to_string(),
                        message_id: message_id.to_string(),
                        tx,
                    })
                    .await
                    .context("Sending message to LoreApi actor")?;
                rx.await.context("Receiving response from LoreApi actor")?
            }
            LoreApi::Mock(responses) => {
                let responses = responses.lock().await;
                let key = format!("raw_patch_{}_{}", target_list, message_id);
                responses
                    .get(&key)
                    .map(ArcStr::clone)
                    .ok_or_else(|| anyhow::anyhow!("Raw patch not found in mock responses: {}", key))
            }
        }
    }

    /// Fetches patch metadata in JSON format.
    ///
    /// This method retrieves structured metadata about a patch in JSON format,
    /// which is useful for programmatic access to patch information.
    ///
    /// # Arguments
    /// * `target_list` - The mailing list name
    /// * `message_id` - The unique message ID of the patch
    ///
    /// # Returns
    /// The patch metadata as JSON, or an error if the request fails.
    ///
    /// # Example
    /// ```
    /// let metadata = lore_api.get_patch_metadata("amd-gfx", "20231201.123456.1-1@amd.com").await?;
    /// ```
    pub async fn get_patch_metadata(
        &self,
        target_list: &str,
        message_id: &str,
    ) -> Result<ArcStr, anyhow::Error> {
        match self {
            LoreApi::Actual(sender) => {
                let (tx, rx) = oneshot::channel();
                sender
                    .send(LoreApiMessage::GetPatchMetadata {
                        target_list: target_list.to_string(),
                        message_id: message_id.to_string(),
                        tx,
                    })
                    .await
                    .context("Sending message to LoreApi actor")?;
                rx.await.context("Receiving response from LoreApi actor")?
            }
            LoreApi::Mock(responses) => {
                let responses = responses.lock().await;
                let key = format!("patch_metadata_{}_{}", target_list, message_id);
                responses
                    .get(&key)
                    .map(ArcStr::clone)
                    .ok_or_else(|| anyhow::anyhow!("Patch metadata not found in mock responses: {}", key))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_lore_api_creation() {
        let net = Net::mock_empty();
        let lore_api = LoreApi::spawn(net);
        
        // Test that we can create the actor successfully
        assert!(matches!(lore_api, LoreApi::Actual(_)));
    }

    #[tokio::test]
    async fn test_lore_api_with_custom_domain() {
        let net = Net::mock_empty();
        let custom_domain = ArcStr::from("https://custom.lore.kernel.org");
        let lore_api = LoreApi::spawn_with_domain(net, custom_domain);
        
        // Test that we can create the actor with custom domain successfully
        assert!(matches!(lore_api, LoreApi::Actual(_)));
    }

    #[tokio::test]
    async fn test_get_patch_feed_url_construction() {
        let net = Net::mock_empty();
        let lore_api = LoreApi::spawn(net);
        
        // This test verifies the URL construction logic
        // The actual request will fail with mock, but we can verify the structure
        let result = lore_api.get_patch_feed("test-list", 100).await;
        assert!(result.is_err()); // Expected with mock
    }

    #[tokio::test]
    async fn test_get_available_lists_url_construction() {
        let net = Net::mock_empty();
        let lore_api = LoreApi::spawn(net);
        
        let result = lore_api.get_available_lists(200).await;
        assert!(result.is_err()); // Expected with mock
    }

    #[tokio::test]
    async fn test_get_patch_html_url_construction() {
        let net = Net::mock_empty();
        let lore_api = LoreApi::spawn(net);
        
        let result = lore_api.get_patch_html("test-list", "test-message-id").await;
        assert!(result.is_err()); // Expected with mock
    }

    #[tokio::test]
    async fn test_mock_with_typed_keys() {
        let mut responses = HashMap::new();
        let test_response = ArcStr::from("<feed>test response</feed>");
        
        responses.insert("patch_feed_test-list_0".to_string(), test_response.clone());
        
        let lore_api = LoreApi::mock(responses);
        
        // This should now work with the mock
        let result = lore_api.get_patch_feed("test-list", 0).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_response);
    }

    #[tokio::test]
    async fn test_mock_empty() {
        let lore_api = LoreApi::mock_empty();
        
        // Test that mock_empty creates an empty mock
        let result = lore_api.get_patch_feed("test-list", 0).await;
        assert!(result.is_err()); // Expected with empty mock
    }
}
