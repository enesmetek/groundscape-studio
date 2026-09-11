import { usePlacementWorker } from './hooks/usePlacementWorker'
import PlacementCanvas from './components/PlacementCanvas'
import { PRODUCTS } from './products'
import './App.css'

// Durum makinesi — plan §14:
// loading → ready → placing → complete/partial/error; Yerleştir düğmesi
// loading ve placing sırasında devre dışıdır.
function App() {
  const { state, place, cancel } = usePlacementWorker()

  if (state.status === 'loading')
    return (
      <main>
        <h1>Ürünler yükleniyor...</h1>
      </main>
    )

  if (state.status === 'error')
    return (
      <main>
        <h1>Hata</h1>
        <p>
          <strong>{state.error.code}</strong>
        </p>
        <p>{state.error.description}</p>
      </main>
    )

  const { ready } = state

  return (
    <main>
      <h1>Groundscape — Yerleştirme</h1>
      <p>
        Alan: {ready.areaMm} × {ready.areaMm} mm · {PRODUCTS.length} ürün
      </p>
      <ul>
        {ready.products.map((product) => (
          <li key={product.id}>
            {product.id} — safety {Math.round(product.safetyAreaMm2).toLocaleString('tr-TR')} mm²
          </li>
        ))}
      </ul>

      {(state.status === 'ready' || state.status === 'complete') && (
        <button type="button" onClick={() => place(Date.now() % 100000)}>
          Yerleştir
        </button>
      )}
      {state.status === 'placing' && (
        <>
          <button type="button" onClick={cancel}>
            İptal
          </button>
          <p>Yerleştiriliyor... {state.placedCount}/3 ürün · {state.totalCandidates} aday</p>
        </>
      )}

      {state.status === 'placing' && <p>Motor çalışıyor...</p>}

      {state.status === 'complete' && (
        <>
          {state.result.status === 'COMPLETE' && <p>Yerleştirme tamamlandı: 3/3 ürün.</p>}
          {state.result.status === 'PARTIAL' && (
            <p>
              Kısmi sonuç: {state.result.placements.length}/3 ürün yerleşti; yerleşmeyen:{' '}
              {state.result.unplaced.join(', ') || '—'} ({state.result.reasonCode}).
            </p>
          )}
          {state.result.status === 'NO_SOLUTION_FOUND' && (
            <p>Bu denemede yer bulunamadı ({state.result.reasonCode}); yeni denemeyi deneyin.</p>
          )}
          {state.result.status === 'INVALID_INPUT' && (
            <p>
              Girdi geçersiz: {state.result.reasonCode}. Yerleşmeyen:{' '}
              {state.result.unplaced.join(', ') || '—'}.
            </p>
          )}
          {state.result.reasonCode === 'FINAL_VALIDATION_FAILED' && (
            <p>Bulunan sonuç geometri kontrolünden geçmedi.</p>
          )}
          <PlacementCanvas ready={ready} result={state.result} />
        </>
      )}
    </main>
  )
}

export default App
