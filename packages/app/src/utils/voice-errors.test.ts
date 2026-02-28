import { describe, expect, test } from "bun:test"
import { voiceErrorKey } from "./voice-errors"

describe("voiceErrorKey", () => {
  test("maps microphone permission errors", () => {
    expect(voiceErrorKey("VOICE_PERMISSION_DENIED")).toBe("prompt.voice.error.microphonePermissionDenied")
    expect(voiceErrorKey(new Error("NotAllowedError"))).toBe("prompt.voice.error.microphonePermissionDenied")
  })

  test("maps no-audio and size guards", () => {
    expect(voiceErrorKey("VOICE_NO_AUDIO")).toBe("prompt.voice.error.noAudioCaptured")
    expect(voiceErrorKey("Audio too large")).toBe("prompt.voice.error.audioTooLarge")
    expect(voiceErrorKey("Unsupported audio format")).toBe("prompt.voice.error.unsupportedAudioFormat")
  })

  test("maps local transcription availability", () => {
    expect(voiceErrorKey("Whisper binary not found")).toBe("prompt.voice.error.localTranscriptionUnavailable")
    expect(voiceErrorKey("Local transcription unavailable")).toBe("prompt.voice.error.localTranscriptionUnavailable")
  })

  test("falls back to generic failure", () => {
    expect(voiceErrorKey("some unknown issue")).toBe("prompt.voice.error.transcriptionFailed")
  })
})
