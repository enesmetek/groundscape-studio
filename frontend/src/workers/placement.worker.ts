// Yerleşim Worker'ı — plan §13.3-13.4. wasm-pack --target web çıktısı module
// Worker içinde init edilir; arama 256 adaylık adımlara bölünür ve adımlar
// arasında setTimeout(0) macrotask sınırıyla CANCEL/iptal işlenir.

import wasmInit, {
  load_products,
  start_placement,
  step_placement,
  reset_placement,
} from '../wasm/groundscape_core.js'
import wasmUrl from '../wasm/groundscape_core_bg.wasm?url'
import {
  SCHEMA_VERSION,
  toApiError,
  type UiMessage,
  type WorkerBody,
} from '../types/protocol'

let initialised = false
let cancelRequested = false
let activeRequestId: string | null = null

function post(message: WorkerBody, requestId: string) {
  self.postMessage({ schemaVersion: SCHEMA_VERSION, requestId, ...message })
}

self.onmessage = async ({ data }: MessageEvent<UiMessage>) => {
  const message = data
  if (message.schemaVersion !== SCHEMA_VERSION) return

  switch (message.type) {
    case 'INIT': {
      activeRequestId = null
      try {
        if (!initialised) {
          await wasmInit({ module_or_path: wasmUrl })
          initialised = true
        }
        const ready = load_products(message.products.map((p) => ({ id: p.id, bytes: new Uint8Array(p.bytes) })))
        post({ type: 'READY', ready: ready as never }, message.requestId)
      } catch (error) {
        post(
          {
            type: 'ERROR',
            error: toApiError(error, 'WASM_INIT_FAILED', 'init'),
          },
          message.requestId,
        )
      }
      break
    }
    case 'PLACE': {
      if (!initialised) {
        post(
          { type: 'ERROR', error: { code: 'WASM_INIT_FAILED', phase: 'search', productId: null, description: 'worker not ready' } },
          message.requestId,
        )
        break
      }
      const requestId = message.requestId
      activeRequestId = requestId
      cancelRequested = false
      try {
        reset_placement()
        start_placement(message.config, BigInt(message.seed))
        for (;;) {
          if (activeRequestId !== requestId) break
          const step = step_placement(256)
          post(
            { type: 'PROGRESS', ranCandidates: step.ranCandidates, placedCount: step.placedCount, totalCandidates: step.totalCandidates },
            requestId,
          )
          if (step.done) {
            post({ type: 'RESULT', result: step.result ?? null }, requestId)
            activeRequestId = null
            break
          }
          // Macrotask sınırı: bekleyen CANCEL/INIT mesajlarını işleme şansı verir.
          await new Promise((resolve) => setTimeout(resolve, 0))
          if (activeRequestId === requestId && cancelRequested) {
            reset_placement()
            post({ type: 'RESULT', result: null }, requestId)
            activeRequestId = null
            break
          }
        }
      } catch (error) {
        post(
          { type: 'ERROR', error: { code: 'INTERNAL_ERROR', phase: 'search', productId: null, description: String(error) } },
          message.requestId,
        )
      }
      break
    }
    case 'CANCEL':
      if (message.requestId === activeRequestId) cancelRequested = true
      break
  }
}
