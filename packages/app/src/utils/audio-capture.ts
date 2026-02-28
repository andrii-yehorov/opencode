const MIME_TYPES = ["audio/ogg;codecs=opus", "audio/ogg", "audio/webm;codecs=opus", "audio/webm", "audio/mp4"]

const pickMime = () => {
  if (typeof MediaRecorder === "undefined") return undefined
  return MIME_TYPES.find((value) => MediaRecorder.isTypeSupported(value))
}

const error = (code: string) => {
  const value = new Error(code)
  ;(value as Error & { code?: string }).code = code
  return value
}

const mapMediaError = (value: unknown) => {
  if (!(value instanceof DOMException)) return error("VOICE_RECORDING_FAILED")
  if (value.name === "NotAllowedError" || value.name === "SecurityError") return error("VOICE_PERMISSION_DENIED")
  if (value.name === "NotFoundError" || value.name === "DevicesNotFoundError") return error("VOICE_DEVICE_NOT_FOUND")
  if (value.name === "NotReadableError") return error("VOICE_DEVICE_NOT_READABLE")
  return error("VOICE_RECORDING_FAILED")
}

export type CapturedAudio = {
  blob: Blob
  mime: string
  durationMs: number
}

export function createAudioCapture(opts?: { maxDurationMs?: number }) {
  const maxDurationMs = opts?.maxDurationMs ?? 120_000
  let recorder: MediaRecorder | undefined
  let stream: MediaStream | undefined
  let timer: ReturnType<typeof setTimeout> | undefined

  const stopTracks = () => {
    if (!stream) return
    stream.getTracks().forEach((track) => track.stop())
    stream = undefined
  }

  const clearTimer = () => {
    if (!timer) return
    clearTimeout(timer)
    timer = undefined
  }

  const stop = () => {
    if (!recorder) return
    if (recorder.state === "inactive") return
    recorder.stop()
  }

  const cancel = () => {
    clearTimer()
    stopTracks()
    recorder = undefined
  }

  const start = async (): Promise<{ stop: () => void; done: Promise<CapturedAudio> }> => {
    if (recorder && recorder.state !== "inactive") throw error("VOICE_ALREADY_RECORDING")
    if (!navigator.mediaDevices?.getUserMedia) throw error("VOICE_UNSUPPORTED")

    stream = await navigator.mediaDevices.getUserMedia({ audio: true }).catch((err) => {
      throw mapMediaError(err)
    })

    const mime = pickMime()
    const next = await Promise.resolve()
      .then(() => (mime ? new MediaRecorder(stream!, { mimeType: mime }) : new MediaRecorder(stream!)))
      .catch(() => {
        stopTracks()
        throw error("VOICE_RECORDING_FAILED")
      })

    recorder = next
    const chunks: BlobPart[] = []
    const startedAt = Date.now()

    const done = new Promise<CapturedAudio>((resolve, reject) => {
      next.onerror = () => reject(error("VOICE_RECORDING_FAILED"))
      next.ondataavailable = (event) => {
        if (!event.data || event.data.size === 0) return
        chunks.push(event.data)
      }
      next.onstop = () => {
        clearTimer()
        stopTracks()
        recorder = undefined
        const blob = new Blob(chunks, { type: mime || next.mimeType || "audio/webm" })
        if (blob.size === 0) {
          reject(error("VOICE_NO_AUDIO"))
          return
        }
        resolve({
          blob,
          mime: blob.type || mime || next.mimeType || "audio/webm",
          durationMs: Date.now() - startedAt,
        })
      }
    })

    next.start()
    timer = setTimeout(() => {
      if (next.state === "recording") next.stop()
    }, maxDurationMs)

    return { stop, done }
  }

  return {
    start,
    stop,
    cancel,
  }
}
