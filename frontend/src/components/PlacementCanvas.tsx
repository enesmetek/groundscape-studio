// Render sözleşmesi — plan §14.1: canonical geometri + poz, SVG pozu bir kez
// uygular; çift dönüşüm yok. viewBox 0 0 5000 5000; matematik +Y yukarı:
// dış grup translate(0 5000) scale(1 -1). Derece yalnızca SVG metninde.

import type { PlacementResult, ReadyPayload } from '../types/protocol'

type Props = {
  ready: ReadyPayload
  result: PlacementResult
}

function PlacementCanvas({ ready, result }: Props) {
  const byId = new Map(ready.products.map((product) => [product.id, product]))
  return (
    <svg viewBox="0 0 5000 5000" width="min(90vmin, 42rem)" role="img" aria-label="Yerleşim alanı">
      <rect x="0" y="0" width="5000" height="5000" fill="#f4f7f2" stroke="#17221b" strokeWidth="10" />
      <g transform="translate(0 5000) scale(1 -1)">
        {result.placements.map(({ productId, pose }) => {
          const product = byId.get(productId)
          if (!product) return null // hata sessizce gizlenmez; durum metninde görünür
          const deg = (pose.rotationRad * 180) / Math.PI
          return (
            <g key={productId} transform={`translate(${pose.xMm} ${pose.yMm}) rotate(${deg})`}>
              <polygon
                points={product.safetyZone.map(([x, y]) => `${x},${y}`).join(' ')}
                fill="#4c9f70"
                fillOpacity="0.15"
                stroke="#4c9f70"
                strokeWidth="8"
                strokeDasharray="40 25"
              />
              {product.footprintPolygons.map((polygon, index) => (
                <polygon key={index} points={polygon.map(([x, y]) => `${x},${y}`).join(' ')} fill="#17221b" />
              ))}
            </g>
          )
        })}
      </g>
    </svg>
  )
}

export default PlacementCanvas
