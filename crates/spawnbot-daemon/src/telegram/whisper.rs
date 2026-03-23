use anyhow::Result;
use std::path::Path;

/// Transcribe an audio file using the OpenAI Whisper API.
///
/// Sends the audio file as multipart form data and returns the transcribed text.
pub async fn transcribe(audio_path: &Path, api_key: &str, language: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let file_bytes = tokio::fs::read(audio_path).await?;
    let file_name = audio_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let part = reqwest::multipart::Part::bytes(file_bytes)
        .file_name(file_name)
        .mime_str("audio/ogg")?;

    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .text("language", language.to_string())
        .part("file", part);

    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await?;

    let body: serde_json::Value = response.json().await?;
    Ok(body["text"].as_str().unwrap_or("").to_string())
}
