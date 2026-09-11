// Worker yaşam döngüsü — plan §13.5: fetch + AbortController, requestId
// uyuşmayan (eski) mesajlar uygulanmaz, unmount'ta Worker sonlandırılır.

import { useEffect, useRef, useState } from 'react'
import { PRODUCTS } from '../products'
import {
  SCHEMA_VERSION,
  type ApiErrorPayload,
  type PlacementResult,
  type ReadyPayload,
  type WorkerMessage,
} from '../types/protocol'

export type WorkerState =
  | { status: 'loading' }
  | { status: 'ready'; ready: ReadyPayload }
  | { status: 'placing'; ready: ReadyPayload; placedCount: number; totalCandidates: number }
  | { status: 'complete'; ready: ReadyPayload; result: PlacementResult }
  | { status: 'error'; ready?: ReadyPayload; error: ApiErrorPayload | { code: string; description: string } }

export function usePlacementWorker() {
  const [state, setState] = useState<WorkerState>({ status: 'loading' })
  const workerRef = useRef<Worker | null>(null)
  const requestIdRef = useRef('')

  useEffect(() => {
    const controller = new AbortController()
    const worker = new Worker(new URL('../workers/placement.worker.ts', import.meta.url), {
      type: 'module',
    })
    workerRef.current = worker

    worker.onmessage = ({ data }: MessageEvent<WorkerMessage>) => {
      // requestId uyuşmayan mesajlar uygulanmaz (plan §13.5).
      if (data.requestId !== requestIdRef.current || data.schemaVersion !== SCHEMA_VERSION) return
      switch (data.type) {
        case 'READY':
          setState({ status: 'ready', ready: data.ready })
          break
        case 'PROGRESS':
          setState((prev) =>
            prev.status === 'placing'
              ? { ...prev, placedCount: data.placedCount, totalCandidates: data.totalCandidates }
              : prev,
          )
          break
        case 'RESULT':
          if (data.result == null) {
            // İptal: bozuk durum bırakmadan ready'ye dön (plan §13.5).
            setState((prev) => (prev.status === 'placing' ? { status: 'ready', ready: prev.ready } : prev))
          } else {
            const result: PlacementResult = data.result
            setState((prev) =>
              prev.status === 'placing' || prev.status === 'ready'
                ? { status: 'complete', ready: prev.ready, result }
                : prev,
            )
          }
          break
        case 'ERROR':
          setState((prev) => ({ status: 'error', ready: prev.status === 'ready' ? prev.ready : undefined, error: data.error }))
          break
      }
    }
    worker.onerror = ({ message: text }) => {
      setState({ status: 'error', error: { code: 'WORKER_FAILED', description: text } })
    }

    ;(async () => {
      try {
        const buffers = await Promise.all(
          PRODUCTS.map(async (product) => {
            const response = await fetch(product.url, { signal: controller.signal })
            if (!response.ok) throw new Error(`${product.id}: HTTP ${response.status}`)
            const buffer = await response.arrayBuffer()
            return { id: product.id, bytes: buffer, placementRole: product.role }
          }),
        )
        requestIdRef.current = crypto.randomUUID()
        worker.postMessage({ type: 'INIT', schemaVersion: SCHEMA_VERSION, requestId: requestIdRef.current, products: buffers })
      } catch (error) {
        setState({
          status: 'error',
          error: {
            code: 'PRODUCT_FETCH_FAILED',
            description: error instanceof Error ? error.message : String(error),
          },
        })
      }
    })()

    return () => {
      controller.abort()
      worker.terminate()
    }
  }, [])

  const place = (seed: number) => {
    const worker = workerRef.current
    if (!worker) return
    requestIdRef.current = crypto.randomUUID()
    setState((prev) =>
      prev.status === 'ready' || prev.status === 'complete'
        ? { status: 'placing', ready: prev.ready, placedCount: 0, totalCandidates: 0 }
        : prev,
    )
    worker.postMessage({
      type: 'PLACE',
      schemaVersion: SCHEMA_VERSION,
      requestId: requestIdRef.current,
      seed,
      config: {
        globalSamplesPerItem: 6000,
        localSamplesPerItem: 2000,
        candidateBufferSize: 6,
        maxRestarts: 3,
        maxTotalCandidates: 200000,
      },
      // Şimdilik default objective; UI ağırlık seti buradan geçirilebilir.
      objective: undefined,
    })
  }

  const cancel = () => {
    workerRef.current?.postMessage({
      type: 'CANCEL',
      schemaVersion: SCHEMA_VERSION,
      requestId: requestIdRef.current,
    })
  }

  return { state, place, cancel }
}
