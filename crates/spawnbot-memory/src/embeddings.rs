use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Gemini Embeddings client for generating text embeddings via the Gemini API.
pub struct GeminiEmbeddings {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct EmbedContentRequest<'a> {
    content: ContentPart<'a>,
}

#[derive(Serialize)]
struct ContentPart<'a> {
    parts: Vec<TextPart<'a>>,
}

#[derive(Serialize)]
struct TextPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct EmbedContentResponse {
    embedding: EmbeddingValues,
}

#[derive(Deserialize)]
struct EmbeddingValues {
    values: Vec<f32>,
}

#[derive(Serialize)]
struct BatchEmbedContentsRequest<'a> {
    requests: Vec<EmbedContentRequest<'a>>,
}

#[derive(Deserialize)]
struct BatchEmbedContentsResponse {
    embeddings: Vec<EmbeddingValues>,
}

impl GeminiEmbeddings {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
        }
    }

    /// Embed a single text string, returning a vector of f32 values.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:embedContent?key={}",
            self.model, self.api_key
        );

        let request_body = EmbedContentRequest {
            content: ContentPart {
                parts: vec![TextPart { text }],
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send embed request to Gemini API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".to_string());
            anyhow::bail!("Gemini API returned status {status}: {body}");
        }

        let resp: EmbedContentResponse = response
            .json()
            .await
            .context("Failed to parse Gemini embed response")?;

        Ok(resp.embedding.values)
    }

    /// Embed multiple texts in a single batch request.
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:batchEmbedContents?key={}",
            self.model, self.api_key
        );

        let requests: Vec<EmbedContentRequest> = texts
            .iter()
            .map(|text| EmbedContentRequest {
                content: ContentPart {
                    parts: vec![TextPart { text }],
                },
            })
            .collect();

        let request_body = BatchEmbedContentsRequest { requests };

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send batch embed request to Gemini API")?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unreadable>".to_string());
            anyhow::bail!("Gemini API returned status {status}: {body}");
        }

        let resp: BatchEmbedContentsResponse = response
            .json()
            .await
            .context("Failed to parse Gemini batch embed response")?;

        Ok(resp.embeddings.into_iter().map(|e| e.values).collect())
    }
}

/// Mock embeddings for testing. Produces deterministic vectors from text content hash.
pub struct MockEmbeddings;

impl MockEmbeddings {
    /// Generate a deterministic embedding vector from a text hash.
    /// Produces a 256-dimensional vector derived from SHA-256 of the input text.
    pub fn embed(text: &str) -> Vec<f32> {
        let hash = Sha256::digest(text.as_bytes());
        // Convert 32 bytes of hash into 256 f32 values by using each byte
        // to generate 8 values (one per byte), then normalize
        let mut values: Vec<f32> = hash.iter().flat_map(|&byte| {
            // Each byte generates 8 float values from its bits
            (0..8).map(move |bit| {
                if byte & (1 << bit) != 0 {
                    1.0
                } else {
                    -1.0
                }
            })
        }).collect();

        // Normalize to unit vector
        let magnitude: f32 = values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for v in &mut values {
                *v /= magnitude;
            }
        }

        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_embeddings_deterministic() {
        let v1 = MockEmbeddings::embed("hello world");
        let v2 = MockEmbeddings::embed("hello world");
        assert_eq!(v1, v2, "Same input should produce same embedding");
    }

    #[test]
    fn test_mock_embeddings_different_inputs() {
        let v1 = MockEmbeddings::embed("hello");
        let v2 = MockEmbeddings::embed("goodbye");
        assert_ne!(v1, v2, "Different inputs should produce different embeddings");
    }

    #[test]
    fn test_mock_embeddings_dimension() {
        let v = MockEmbeddings::embed("test");
        assert_eq!(v.len(), 256, "Mock embeddings should be 256-dimensional");
    }

    #[test]
    fn test_mock_embeddings_normalized() {
        let v = MockEmbeddings::embed("test");
        let magnitude: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-5,
            "Mock embeddings should be unit vectors, got magnitude {magnitude}"
        );
    }
}
