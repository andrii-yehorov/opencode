type AudioResult = {
  base64: string
  mime: string
}

export type AudioCapture = {
  stop: () => Promise<AudioResult>
  cancel: () => Promise<void>
}

const merge = (list: Float32Array[]) => {
  const size = list.reduce((sum, item) => sum + item.length, 0)
  const out = new Float32Array(size)
  let offset = 0
  for (const item of list) {
    out.set(item, offset)
    offset += item.length
  }
  return out
}

const wav = (input: Float32Array, rate: number) => {
  const bytes = new ArrayBuffer(44 + input.length * 2)
  const view = new DataView(bytes)
  const write = (offset: number, text: string) => {
    for (let i = 0; i < text.length; i += 1) view.setUint8(offset + i, text.charCodeAt(i))
  }

  write(0, "RIFF")
  view.setUint32(4, 36 + input.length * 2, true)
  write(8, "WAVE")
  write(12, "fmt ")
  view.setUint32(16, 16, true)
  view.setUint16(20, 1, true)
  view.setUint16(22, 1, true)
  view.setUint32(24, rate, true)
  view.setUint32(28, rate * 2, true)
  view.setUint16(32, 2, true)
  view.setUint16(34, 16, true)
  write(36, "data")
  view.setUint32(40, input.length * 2, true)

  let offset = 44
  for (let i = 0; i < input.length; i += 1) {
    const sample = Math.max(-1, Math.min(1, input[i]))
    view.setInt16(offset, sample < 0 ? sample * 0x8000 : sample * 0x7fff, true)
    offset += 2
  }

  return new Blob([bytes], { type: "audio/wav" })
}

const b64 = (blob: Blob) =>
  new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const data = typeof reader.result === "string" ? reader.result : ""
      const index = data.indexOf(",")
      resolve(index === -1 ? "" : data.slice(index + 1))
    }
    reader.onerror = () => reject(reader.error ?? new Error("Failed to read audio blob"))
    reader.readAsDataURL(blob)
  })

export const startAudioCapture = async () => {
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
  const context = new AudioContext()
  const rate = context.sampleRate
  const source = context.createMediaStreamSource(stream)
  const node = context.createScriptProcessor(4096, 1, 1)
  const list: Float32Array[] = []

  node.onaudioprocess = (event) => {
    const data = event.inputBuffer.getChannelData(0)
    list.push(new Float32Array(data))
  }

  source.connect(node)
  node.connect(context.destination)

  let done = false
  const close = async () => {
    if (done) return
    done = true
    node.onaudioprocess = null
    node.disconnect()
    source.disconnect()
    for (const track of stream.getTracks()) track.stop()
    await context.close().catch(() => undefined)
  }

  const stop = async () => {
    await close()
    const blob = wav(merge(list), rate)
    const base64 = await b64(blob)
    return {
      base64,
      mime: blob.type,
    }
  }

  const cancel = async () => {
    await close()
  }

  return {
    stop,
    cancel,
  } satisfies AudioCapture
}
