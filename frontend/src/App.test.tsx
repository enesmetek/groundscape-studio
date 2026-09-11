// Arayüz testi — plan §15: durum makinesi, düğme kilidi, sonuç mesajı.
// Worker jsdom'da yoktur; sahte Worker ile mesaj akışı simüle edilir.

import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import App from './App'
import { SCHEMA_VERSION, type WorkerMessage } from './types/protocol'

type FakeWorker = {
  onmessage: ((event: { data: WorkerMessage }) => void) | null
  onerror: ((event: { message: string }) => void) | null
  postMessage: ReturnType<typeof vi.fn>
  terminate: ReturnType<typeof vi.fn>
}

class FakeWorkerImpl {
  static instances: FakeWorkerImpl[] = []
  onmessage: FakeWorker['onmessage'] = null
  onerror: FakeWorker['onerror'] = null
  postMessage = vi.fn()
  terminate = vi.fn()
  constructor() {
    FakeWorkerImpl.instances.push(this)
  }
}

function currentWorker() {
  const worker = FakeWorkerImpl.instances.at(-1)
  if (!worker) throw new Error('worker yok')
  return worker
}

beforeEach(() => {
  vi.stubGlobal('Worker', FakeWorkerImpl)
  vi.stubGlobal('crypto', { randomUUID: () => `req-${Math.random()}` })
  vi.stubGlobal('fetch', vi.fn(async () => ({ ok: true, arrayBuffer: async () => new ArrayBuffer(8) })))
  FakeWorkerImpl.instances = []
})

afterEach(() => {
  cleanup()
  vi.unstubAllGlobals()
})

function readyMessage(requestId: string): WorkerMessage {
  return {
    type: 'READY',
    schemaVersion: SCHEMA_VERSION,
    requestId,
    ready: {
      areaMm: 5000,
      products: [
        { id: 'product-1', footprintPolygons: [[[0, 0], [900, 0], [900, 900], [0, 900]]], safetyZone: [[0, 0], [1500, 0], [1500, 1500], [0, 1500]], safetyAreaMm2: 2_250_000 },
        { id: 'product-2', footprintPolygons: [[[0, 0], [700, 0], [700, 700], [0, 700]]], safetyZone: [[0, 0], [1200, 0], [1200, 1200], [0, 1200]], safetyAreaMm2: 1_440_000 },
        { id: 'product-3', footprintPolygons: [[[0, 0], [500, 0], [500, 500], [0, 500]]], safetyZone: [[0, 0], [900, 0], [900, 900], [0, 900]], safetyAreaMm2: 810_000 },
      ],
    },
  }
}

function resultMessage(requestId: string): WorkerMessage {
  return {
    type: 'RESULT',
    schemaVersion: SCHEMA_VERSION,
    requestId,
    result: {
      status: 'COMPLETE',
      placements: [
        { productId: 'product-1', pose: { xMm: 0, yMm: 0, rotationRad: 0 } },
        { productId: 'product-2', pose: { xMm: 1600, yMm: 0, rotationRad: 0 } },
        { productId: 'product-3', pose: { xMm: 2900, yMm: 0, rotationRad: 0 } },
      ],
      unplaced: [],
      reasonCode: '',
      stats: { candidatesTried: 512, restarts: 0 },
    },
  }
}

async function bootToReady(onLoading?: () => void) {
  render(<App />)
  onLoading?.()
  // Önce INIT mesajı, sonra READY — heading ancak bundan sonra görünür.
  await waitFor(() => expect(currentWorker().postMessage).toHaveBeenCalled())
  const initCall = currentWorker().postMessage.mock.calls.find(
    ([message]) => (message as { type: string }).type === 'INIT',
  )
  expect(initCall).toBeTruthy()
  const requestId = (initCall?.[0] as { requestId: string }).requestId
  currentWorker().onmessage?.({ data: readyMessage(requestId) })
  await waitFor(() => expect(screen.getByRole('button', { name: 'Yerleştir' })).toBeEnabled())
  return requestId
}

describe('App durum makinesi', () => {
  it('Yerleştir düğmesi yükleme sırasında yoktur; ready sonrası çalışır', async () => {
    await bootToReady(() => {
      expect(screen.queryByRole('button', { name: 'Yerleştir' })).not.toBeInTheDocument()
    })
  })

  it('placing sırasında İptal görünür, RESULT ile tamamlanır', async () => {
    const user = userEvent.setup()
    await bootToReady()

    await user.click(screen.getByRole('button', { name: 'Yerleştir' }))
    const placeCall = currentWorker().postMessage.mock.calls.find(
      ([message]) => (message as { type: string }).type === 'PLACE',
    )
    expect(placeCall).toBeTruthy()
    await waitFor(() => expect(screen.getByText(/Yerleştiriliyor/)).toBeInTheDocument())

    const requestId = (placeCall?.[0] as { requestId: string }).requestId
    currentWorker().onmessage?.({ data: resultMessage(requestId) })
    await waitFor(() =>
      expect(screen.getByText('Yerleştirme tamamlandı: 3/3 ürün.')).toBeInTheDocument(),
    )
    // SVG tek transform ile çizilir: 3 ürün grubu
    expect(screen.getByRole('img', { name: 'Yerleşim alanı' })).toBeInTheDocument()
    expect(screen.getByRole('img', { name: 'Yerleşim alanı' }).querySelectorAll('g g')).toHaveLength(3)
  })

  it('tamamlanan yerleşim yeniden başlatılabilir', async () => {
    const user = userEvent.setup()
    await bootToReady()

    await user.click(screen.getByRole('button', { name: 'Yerleştir' }))
    const firstPlace = currentWorker().postMessage.mock.calls.find(
      ([message]) => (message as { type: string }).type === 'PLACE',
    )
    const requestId = (firstPlace?.[0] as { requestId: string }).requestId
    currentWorker().onmessage?.({ data: resultMessage(requestId) })
    await screen.findByText('Yerleştirme tamamlandı: 3/3 ürün.')

    await user.click(screen.getByRole('button', { name: 'Yerleştir' }))
    expect(screen.getByText(/Yerleştiriliyor/)).toBeInTheDocument()
  })

  it('eski requestId li mesaj uygulanmaz', async () => {
    const user = userEvent.setup()
    await bootToReady()

    await user.click(screen.getByRole('button', { name: 'Yerleştir' }))
    // eski requestId ile RESULT: placing durumu bozulmamalı
    currentWorker().onmessage?.({ data: resultMessage('stale-id') })
    await waitFor(() => expect(screen.getByText(/Yerleştiriliyor/)).toBeInTheDocument())
    expect(screen.queryByText('Yerleştirme tamamlandı: 3/3 ürün.')).not.toBeInTheDocument()
  })

  it('iptal sonrası ready durumuna döner', async () => {
    const user = userEvent.setup()
    await bootToReady()

    await user.click(screen.getByRole('button', { name: 'Yerleştir' }))
    const placeCall = currentWorker().postMessage.mock.calls.find(
      ([message]) => (message as { type: string }).type === 'PLACE',
    )
    const requestId = (placeCall?.[0] as { requestId: string }).requestId
    await user.click(screen.getByRole('button', { name: 'İptal' }))
    expect(currentWorker().postMessage).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'CANCEL', requestId }),
    )

    currentWorker().onmessage?.({
      data: { type: 'RESULT', schemaVersion: SCHEMA_VERSION, requestId, result: null },
    })
    await waitFor(() => expect(screen.getByRole('button', { name: 'Yerleştir' })).toBeEnabled())
  })

  it('unmount sırasında Worker sonlandırılır', async () => {
    const view = render(<App />)
    await waitFor(() => expect(currentWorker().postMessage).toHaveBeenCalled())

    const worker = currentWorker()
    view.unmount()
    expect(worker.terminate).toHaveBeenCalledOnce()
  })

  it('hata mesajı anlaşılır kodla gösterilir', async () => {
    render(<App />)
    await waitFor(() => expect(currentWorker().postMessage).toHaveBeenCalled())
    const initCall = currentWorker().postMessage.mock.calls.find(
      ([message]) => (message as { type: string }).type === 'INIT',
    )
    const requestId = (initCall?.[0] as { requestId: string }).requestId
    currentWorker().onmessage?.({
      data: {
        type: 'ERROR',
        schemaVersion: SCHEMA_VERSION,
        requestId,
        error: { code: 'DXF_PARSE_FAILED', phase: 'import', productId: 'product-1', description: 'bozuk' },
      },
    })
    await waitFor(() => expect(screen.getByText('DXF_PARSE_FAILED')).toBeInTheDocument())
  })
})
