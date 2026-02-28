export const MAX_AUDIO_BYTES = 25 * 1024 * 1024

const TARGET_RATE = 16_000

const wav = (samples: Float32Array, sampleRate: number) => {
  const bytes = samples.length * 2
  const out = new ArrayBuffer(44 + bytes)
  const view = new DataView(out)

  const write = (offset: number, value: string) => {
    for (const [index, char] of [...value].entries()) {
      view.setUint8(offset + index, char.charCodeAt(0))
    }
  }

  write(0, "RIFF")
  view.setUint32(4, 36 + bytes, true)
  write(8, "WAVE")
  write(12, "fmt ")
  view.setUint32(16, 16, true)
  view.setUint16(20, 1, true)
  view.setUint16(22, 1, true)
  view.setUint32(24, sampleRate, true)
  view.setUint32(28, sampleRate * 2, true)
  view.setUint16(32, 2, true)
  view.setUint16(34, 16, true)
  write(36, "data")
  view.setUint32(40, bytes, true)

  for (const [index, value] of samples.entries()) {
    const clamped = Math.max(-1, Math.min(1, value))
    view.setInt16(44 + index * 2, clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff, true)
  }

  return new Blob([out], { type: "audio/wav" })
}

const transcode = async (blob: Blob) => {
  if (typeof AudioContext === "undefined" || typeof OfflineAudioContext === "undefined") return blob
  const context = new AudioContext()
  const data = await blob.arrayBuffer()
  const decoded = await context.decodeAudioData(data.slice(0))
  await context.close()

  const frames = Math.ceil(decoded.duration * TARGET_RATE)
  const offline = new OfflineAudioContext(1, Math.max(1, frames), TARGET_RATE)
  const source = offline.createBufferSource()
  source.buffer = decoded
  source.connect(offline.destination)
  source.start(0)
  const rendered = await offline.startRendering()
  return wav(rendered.getChannelData(0), TARGET_RATE)
}

export async function whisperAudio(blob: Blob, mime: string) {
  if (!mime.includes("webm")) {
    return {
      blob,
      mime: mime || blob.type || "audio/ogg",
    }
  }

  const audio = await transcode(blob).catch(() => blob)
  return {
    blob: audio,
    mime: audio.type || mime || "audio/wav",
  }
}

export function blobToBase64(blob: Blob) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onerror = () => reject(new Error("VOICE_ENCODING_FAILED"))
    reader.onload = () => {
      const value = typeof reader.result === "string" ? reader.result : ""
      const base64 = value.includes(",") ? value.slice(value.indexOf(",") + 1) : value
      if (!base64) {
        reject(new Error("VOICE_ENCODING_FAILED"))
        return
      }
      resolve(base64)
    }
    reader.readAsDataURL(blob)
  })
}
