const find = (value: string, list: string[]) => list.some((item) => value.includes(item))

export function voiceErrorKey(error: unknown) {
  const code =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? ((error as Error & { code?: string }).code ?? error.message)
        : ""
  const normalized = code.toLowerCase()

  if (
    find(normalized, [
      "voice_permission_denied",
      "notallowederror",
      "securityerror",
      "microphone permission denied",
    ])
  ) {
    return "prompt.voice.error.microphonePermissionDenied"
  }

  if (find(normalized, ["voice_no_audio", "no audio captured"])) {
    return "prompt.voice.error.noAudioCaptured"
  }

  if (find(normalized, ["audio too large"])) {
    return "prompt.voice.error.audioTooLarge"
  }

  if (find(normalized, ["unsupported audio format"])) {
    return "prompt.voice.error.unsupportedAudioFormat"
  }

  if (
    find(normalized, [
      "whisper binary not found",
      "whisper model not found",
      "local transcription unavailable",
      "voice_unsupported",
    ])
  ) {
    return "prompt.voice.error.localTranscriptionUnavailable"
  }

  return "prompt.voice.error.transcriptionFailed"
}
