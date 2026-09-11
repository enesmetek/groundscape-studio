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
      <main className="app app--centered">
        <h1>Groundscape</h1>
        <p role="status">Ürünler yükleniyor...</p>
        <button type="button" disabled>
          Yerleştir
        </button>
      </main>
    )

  if (state.status === 'error')
    return (
      <main className="app app--centered">
        <div role="alert">
          <h1>Hata</h1>
          <p>
            <strong>{state.error.code}</strong>
          </p>
          <p>{state.error.description}</p>
          <p>Ürün dosyalarını kontrol edip sayfayı yenileyin.</p>
        </div>
      </main>
    )

  const { ready } = state

  return (
    <main className="app">
      <header className="app__header">
        <p className="eyebrow">Tek alan · sürekli rotasyon</p>
        <h1>Groundscape — Yerleştirme</h1>
        <p className="app__summary">
          Alan: {ready.areaMm} × {ready.areaMm} mm · {PRODUCTS.length} ürün
        </p>
      </header>

      <section className="product-panel" aria-label="Ürünler">
        <ul className="product-list">
          {ready.products.map((product) => (
            <li key={product.id}>
              <strong>{product.id}</strong>
              {' '}
              <span className={`product-role product-role--${product.placementRole}`}>
                {product.placementRole}
              </span>
              {' '}
              <span>— safety {Math.round(product.safetyAreaMm2).toLocaleString('tr-TR')} mm²</span>
            </li>
          ))}
        </ul>

        <div className="actions">
          <button
            type="button"
            disabled={state.status === 'placing'}
            onClick={() => place(Date.now() % 100000)}
          >
            Yerleştir
          </button>
          {state.status === 'placing' && (
            <button type="button" className="button--secondary" onClick={cancel}>
              İptal
            </button>
          )}
        </div>
        {state.status === 'placing' && (
          <p className="state-message" role="status">
            Yerleştiriliyor... {state.placedCount}/3 ürün · {state.totalCandidates} aday
          </p>
        )}
      </section>

      {state.status === 'complete' && (
        <section className="result-panel" aria-label="Yerleştirme sonucu">
          {state.result.status === 'COMPLETE' && <p role="status">Yerleştirme tamamlandı: 3/3 ürün.</p>}
          {state.result.status === 'PARTIAL' && (
            <p role="status">
              Kısmi sonuç: {state.result.placements.length}/3 ürün yerleşti; yerleşmeyen:{' '}
              {state.result.unplaced.join(', ') || '—'} ({state.result.reasonCode}).
            </p>
          )}
          {state.result.status === 'NO_SOLUTION_FOUND' && (
            <p role="status">Bu denemede yer bulunamadı ({state.result.reasonCode}); yeni denemeyi deneyin.</p>
          )}
          {state.result.status === 'INVALID_INPUT' && (
            <p role="status">
              Girdi geçersiz: {state.result.reasonCode}. Yerleşmeyen:{' '}
              {state.result.unplaced.join(', ') || '—'}.
            </p>
          )}
          {state.result.reasonCode === 'FINAL_VALIDATION_FAILED' && (
            <p role="alert">Bulunan sonuç geometri kontrolünden geçmedi.</p>
          )}
          <PlacementCanvas ready={ready} result={state.result} />
        </section>
      )}
    </main>
  )
}

export default App
