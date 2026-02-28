use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};
use tauri::{AppHandle, Manager};

const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;

#[derive(Serialize, specta::Type)]
pub struct TranscriptionResult {
    pub text: String,
}

#[tauri::command]
#[specta::specta]
pub async fn transcribe_audio_local(
    app: AppHandle,
    base64: String,
    mime: String,
    language: Option<String>,
) -> Result<TranscriptionResult, String> {
    if base64.trim().is_empty() {
        return Err("No audio captured".to_string());
    }
    if !mime.trim().starts_with("audio/") {
        return Err("Local transcription unavailable".to_string());
    }

    let bytes = STANDARD
        .decode(base64.trim())
        .map_err(|_| "No audio captured".to_string())?;
    if bytes.is_empty() {
        return Err("No audio captured".to_string());
    }
    if bytes.len() > MAX_AUDIO_BYTES {
        return Err("Audio too large".to_string());
    }

    tokio::task::spawn_blocking(move || run_transcription(&app, bytes, mime, language))
        .await
        .map_err(|_| "Transcription failed".to_string())?
}

fn run_transcription(
    app: &AppHandle,
    bytes: Vec<u8>,
    mime: String,
    language: Option<String>,
) -> Result<TranscriptionResult, String> {
    let started = Instant::now();
    let size = bytes.len();
    let requested_language = language.clone();
    tracing::debug!(size, mime = %mime, language = ?requested_language, "voice transcription start");

    let binary = resolve_binary().ok_or_else(|| "Whisper binary not found".to_string())?;
    let model = resolve_model(app).ok_or_else(|| "Whisper model not found".to_string())?;
    tracing::debug!(binary = %binary.display(), model = %model.display(), "voice transcription paths");

    let token = uuid::Uuid::new_v4().to_string();
    let ext = extension_for_mime(&mime);
    let input = std::env::temp_dir().join(format!("opencode-voice-{token}.{ext}"));
    let output_prefix = std::env::temp_dir().join(format!("opencode-voice-{token}"));
    let output_txt = output_prefix.with_extension("txt");

    fs::write(&input, bytes).map_err(|_| "Transcription failed".to_string())?;

    let mut cmd = Command::new(binary);
    cmd.arg("-m")
        .arg(model)
        .arg("-f")
        .arg(&input)
        .arg("-np")
        .arg("-otxt")
        .arg("-of")
        .arg(&output_prefix);

    let language = whisper_language(language);
    if let Some(language) = language {
        tracing::debug!(language = %language, "voice transcription normalized language");
        cmd.arg("-l").arg(language);
    }

    let output = cmd.output().map_err(|error| {
        tracing::warn!(error = %error, "voice transcription command spawn failed");
        "Transcription failed".to_string()
    })?;

    let _ = fs::remove_file(&input);

    if !output.status.success() {
        tracing::warn!(
            code = ?output.status.code(),
            stderr = %preview(&output.stderr),
            stdout = %preview(&output.stdout),
            "voice transcription command failed"
        );
        let _ = fs::remove_file(&output_txt);
        return Err("Transcription failed".to_string());
    }

    let text = fs::read_to_string(&output_txt)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if value.is_empty() { None } else { Some(value) }
        })
        .ok_or_else(|| {
            tracing::warn!(
                mime = %mime,
                stderr = %preview(&output.stderr),
                stdout = %preview(&output.stdout),
                "voice transcription returned empty output"
            );
            if mime.contains("webm") {
                "Unsupported audio format".to_string()
            } else {
                "Transcription failed".to_string()
            }
        })?;

    let _ = fs::remove_file(&output_txt);
    tracing::debug!(duration_ms = started.elapsed().as_millis(), text_len = text.len(), "voice transcription success");

    Ok(TranscriptionResult { text })
}

fn whisper_language(value: Option<String>) -> Option<String> {
    let value = value
        .map(|value| value.trim().to_lowercase().replace('_', "-"))
        .filter(|value| !value.is_empty())?;
    if value == "auto" {
        return Some(value);
    }
    let head = value.split('-').next().unwrap_or_default();
    if !(2..=3).contains(&head.len()) {
        return None;
    }
    if !head.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    Some(head.to_string())
}

fn preview(value: &[u8]) -> String {
    let text = String::from_utf8_lossy(value).trim().replace('\n', " | ");
    if text.chars().count() <= 280 {
        return text;
    }
    let snippet = text.chars().take(280).collect::<String>();
    format!("{snippet}...")
}

fn extension_for_mime(mime: &str) -> &'static str {
    if mime.contains("ogg") {
        return "ogg";
    }
    if mime.contains("wav") {
        return "wav";
    }
    if mime.contains("mp4") || mime.contains("m4a") {
        return "m4a";
    }
    "webm"
}

fn resolve_binary() -> Option<PathBuf> {
    ["OPENCODE_WHISPER_CPP_BIN", "OPENCODE_WHISPER_BIN"]
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| resolve_on_path("whisper-cli"))
        .or_else(|| resolve_on_path("whisper"))
        .or_else(|| resolve_on_path("main"))
}

fn resolve_model(app: &AppHandle) -> Option<PathBuf> {
    ["OPENCODE_WHISPER_CPP_MODEL", "OPENCODE_WHISPER_MODEL"]
        .iter()
        .find_map(|key| std::env::var(key).ok())
        .map(PathBuf::from)
        .filter(|path| path.exists())
        .or_else(|| {
            app.path()
                .app_data_dir()
                .ok()
                .map(|path| path.join("whisper").join("model.gguf"))
                .filter(|path| path.exists())
        })
        .or_else(|| {
            dirs::home_dir()
                .map(|path| path.join(".opencode").join("whisper").join("model.gguf"))
                .filter(|path| path.exists())
        })
        .or_else(|| {
            dirs::home_dir()
                .map(|path| path.join(".cache").join("whisper").join("model.gguf"))
                .filter(|path| path.exists())
        })
}

fn resolve_on_path(name: &str) -> Option<PathBuf> {
    let output = if cfg!(windows) {
        Command::new("where").arg(name).output().ok()?
    } else {
        Command::new("which").arg(name).output().ok()?
    };

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(Path::new)
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::{extension_for_mime, preview, whisper_language};

    #[test]
    fn extension_matches_common_audio_types() {
        assert_eq!(extension_for_mime("audio/wav"), "wav");
        assert_eq!(extension_for_mime("audio/ogg;codecs=opus"), "ogg");
        assert_eq!(extension_for_mime("audio/mp4"), "m4a");
        assert_eq!(extension_for_mime("audio/webm;codecs=opus"), "webm");
    }

    #[test]
    fn language_normalizes_locale_values() {
        assert_eq!(whisper_language(Some("en-US".to_string())), Some("en".to_string()));
        assert_eq!(whisper_language(Some("pt_BR".to_string())), Some("pt".to_string()));
        assert_eq!(whisper_language(Some("auto".to_string())), Some("auto".to_string()));
    }

    #[test]
    fn language_rejects_invalid_values() {
        assert_eq!(whisper_language(Some("".to_string())), None);
        assert_eq!(whisper_language(Some("english".to_string())), None);
        assert_eq!(whisper_language(Some("12".to_string())), None);
    }

    #[test]
    fn preview_limits_length() {
        let long = vec![b'a'; 400];
        let short = preview(&long);
        assert!(short.len() < 320);
        assert!(short.ends_with("..."));
    }
}
