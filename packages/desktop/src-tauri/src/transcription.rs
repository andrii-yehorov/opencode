use base64::{Engine, engine::general_purpose};
use std::{env, path::PathBuf, time::Duration};
use tokio::{process::Command, time::timeout};

const TRANSCRIPTION_TIMEOUT_SECS: u64 = 30;

#[derive(serde::Deserialize, specta::Type)]
pub struct AudioTranscriptionInput {
    pub base64: String,
    pub mime: String,
    pub language: Option<String>,
}

#[derive(serde::Serialize, specta::Type)]
pub struct AudioTranscriptionResult {
    pub text: String,
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var(name).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

fn whisper_binary_path() -> Result<PathBuf, String> {
    let path = env_path("OPENCODE_WHISPER_CPP_BIN").or_else(|| env_path("OPENCODE_WHISPER_CPP_PATH"));
    let Some(path) = path else {
        return Err("Set OPENCODE_WHISPER_CPP_BIN to your whisper.cpp binary path".to_string());
    };
    if !path.exists() {
        return Err("Whisper binary not found".to_string());
    }
    Ok(path)
}

fn whisper_model_path() -> Result<PathBuf, String> {
    let path = env_path("OPENCODE_WHISPER_CPP_MODEL");
    let Some(path) = path else {
        return Err("Set OPENCODE_WHISPER_CPP_MODEL to your Whisper model path".to_string());
    };
    if !path.exists() {
        return Err("Whisper model not found".to_string());
    }
    Ok(path)
}

fn parse_transcript(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            if let Some((_, text)) = line.split_once("] ") {
                return text.trim().to_string();
            }
            line.to_string()
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[tauri::command]
#[specta::specta]
pub async fn transcribe_audio_local(
    input: AudioTranscriptionInput,
) -> Result<AudioTranscriptionResult, String> {
    let AudioTranscriptionInput {
        base64,
        mime,
        language,
    } = input;
    let whisper_bin = whisper_binary_path()?;
    let model = whisper_model_path()?;
    let audio = general_purpose::STANDARD
        .decode(base64)
        .map_err(|_| "Invalid audio payload".to_string())?;
    if !mime.starts_with("audio/") {
        return Err("Unsupported audio format".to_string());
    }
    if audio.is_empty() {
        return Err("No audio captured".to_string());
    }

    let file = env::temp_dir().join(format!("opencode-voice-{}.wav", uuid::Uuid::new_v4()));
    std::fs::write(&file, audio).map_err(|_| "Failed to prepare audio file".to_string())?;

    let lang = language.unwrap_or_else(|| "en".to_string());
    let run = Command::new(&whisper_bin)
        .arg("-m")
        .arg(&model)
        .arg("-f")
        .arg(&file)
        .arg("-l")
        .arg(lang)
        .arg("-nt")
        .arg("-np")
        .output();

    let output = timeout(Duration::from_secs(TRANSCRIPTION_TIMEOUT_SECS), run)
        .await
        .map_err(|_| "Transcription timed out".to_string())
        .and_then(|result| result.map_err(|_| "Failed to run local transcriber".to_string()));

    let _ = std::fs::remove_file(&file);

    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            return Err("Local transcription failed".to_string());
        }
        return Err(stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = parse_transcript(&stdout);
    if text.is_empty() {
        return Err("No speech detected".to_string());
    }

    Ok(AudioTranscriptionResult { text })
}
