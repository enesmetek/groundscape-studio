import init, { run_spike_wasm } from './wasm/groundscape_core.js'

try {
  await init()
  self.postMessage({ result: run_spike_wasm() })
} catch (error) {
  self.postMessage({ error: error instanceof Error ? error.message : String(error) })
}
