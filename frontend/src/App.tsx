import { useEffect, useState } from 'react'
import { spikePassed, type SpikeResult } from './spikeResult'
import './App.css'

type State =
  | { status: 'loading' }
  | { status: 'success'; result: SpikeResult }
  | { status: 'error'; message: string }

function App() {
  const [state, setState] = useState<State>({ status: 'loading' })

  useEffect(() => {
    const worker = new Worker(new URL('./spike.worker.ts', import.meta.url), {
      type: 'module',
    })
    worker.onmessage = ({ data }: MessageEvent<{ result?: SpikeResult; error?: string }>) => {
      if (data.result) setState({ status: 'success', result: data.result })
      else setState({ status: 'error', message: data.error ?? 'Worker failed' })
    }
    worker.onerror = ({ message }) => setState({ status: 'error', message })
    return () => worker.terminate()
  }, [])

  if (state.status === 'loading') return <main><h1>Running technical spike...</h1></main>
  if (state.status === 'error') return <main><h1>Technical spike failed</h1><pre>{state.message}</pre></main>

  const { result } = state
  const passed = spikePassed(result)
  return (
    <main>
      <h1>{passed ? 'Technical spike passed' : 'Technical spike failed'}</h1>
      <p>{result.engine}</p>
      <dl>
        <dt>DXF</dt><dd>{result.dxf.units_mm && result.dxf.closed_lwpolyline ? `${result.dxf.vertex_count} closed vertices in mm` : 'failed'}</dd>
        <dt>Collision</dt><dd>{result.overlap_detected ? 'containment detected' : 'failed'}</dd>
        <dt>37 degree pose</dt><dd>{result.continuous_rotation_enabled && result.angle_37_accepted ? 'continuous; 37 degrees accepted' : 'failed'}</dd>
        <dt>Exact 5000 fit</dt><dd>{result.exact_fit_accepted ? 'accepted at fixed translation' : 'failed'}</dd>
        <dt>Outside pose</dt><dd>{result.out_of_area_rejected ? 'outside area rejected' : 'failed'}</dd>
        <dt>Unfit item</dt><dd>{result.unfit_item_rejected ? 'unfit item rejected; no fallback bin' : 'failed'}</dd>
      </dl>
      <p>Area size: {result.area_size[0]} x {result.area_size[1]}</p>
      <p>Bins used: {result.bins_used}</p>
    </main>
  )
}

export default App
