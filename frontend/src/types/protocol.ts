// Mesaj sözleşmesi — plan §13.2. Her mesaj schemaVersion: 1 ve requestId taşır.
// Serde tarafı camelCase (plan §13.1); DTO'lar Rust struct'larıyla eşleşir.

export type Pose = { xMm: number; yMm: number; rotationRad: number }
export type Placement = { productId: string; pose: Pose }
export type Polygon = [number, number][]

/** Serde camelCase PlacementRole ile birebir (MVP-2 plan P1). */
export type PlacementRole = 'auto' | 'anchor' | 'distributed' | 'peripheral'

/** Serde camelCase LayoutObjective ile birebir (MVP-2 plan P2). */
export type LayoutObjective = {
  weights: {
    anchor: number
    balance: number
    distribution: number
    spacing: number
    orientation: number
  }
  grid: number
  anchorRegionRatio: number
  spacingTargetRatio: number
}

export type ReadyPayload = {
  areaMm: number
  products: {
    id: string
    footprintPolygons: Polygon[]
    safetyZone: Polygon
    safetyAreaMm2: number
    /** Rust ProductPayload her zaman gönderir; UI rolleri buradan okur. */
    placementRole: PlacementRole
    tags: string[]
    ageGroup: string | null
    /** Rust ProductPayload gönderir; UI şimdilik okumuyor. */
    footprintAreaMm2?: number
    footprintCentroidLocal?: [number, number]
  }[]
}

export type SearchConfig = {
  globalSamplesPerItem: number
  localSamplesPerItem: number
  candidateBufferSize: number
  maxRestarts: number
  maxTotalCandidates: number
}

export type ResultStatus = 'COMPLETE' | 'PARTIAL' | 'NO_SOLUTION_FOUND' | 'INVALID_INPUT'

export type PlacementResult = {
  status: ResultStatus
  placements: Placement[]
  unplaced: string[]
  reasonCode: string
  stats: { candidatesTried: number; restarts: number }
}

export type ApiErrorPayload = {
  code: string
  phase: string
  productId: string | null
  description: string
}

export function toApiError(error: unknown, code: string, phase: string): ApiErrorPayload {
  if (typeof error === 'object' && error !== null) {
    const candidate = error as Partial<ApiErrorPayload>
    if (
      typeof candidate.code === 'string' &&
      typeof candidate.phase === 'string' &&
      (candidate.productId === null || typeof candidate.productId === 'string') &&
      typeof candidate.description === 'string'
    ) {
      return candidate as ApiErrorPayload
    }
  }
  return {
    code,
    phase,
    productId: null,
    description: error instanceof Error ? error.message : String(error),
  }
}

export type InitMessage = {
  type: 'INIT'
  schemaVersion: 1
  requestId: string
  products: {
    id: string
    bytes: ArrayBuffer
    placementRole?: PlacementRole
    tags?: string[]
    ageGroup?: string
  }[]
}
export type PlaceMessage = {
  type: 'PLACE'
  schemaVersion: 1
  requestId: string
  seed: number
  config: SearchConfig
  /** Yoksa WASM tarafında LayoutObjective::default() kullanılır. */
  objective?: LayoutObjective
}
export type CancelMessage = { type: 'CANCEL'; schemaVersion: 1; requestId: string }

export type ReadyMessage = {
  type: 'READY'
  schemaVersion: 1
  requestId: string
  ready: ReadyPayload
}
export type ProgressMessage = {
  type: 'PROGRESS'
  schemaVersion: 1
  requestId: string
  ranCandidates: number
  placedCount: number
  totalCandidates: number
}
export type ResultMessage = {
  type: 'RESULT'
  schemaVersion: 1
  requestId: string
  /** null: arama iptal edildi; UI ready'ye döner. */
  result: PlacementResult | null | undefined
}
export type ErrorMessage = {
  type: 'ERROR'
  schemaVersion: 1
  requestId: string
  error: ApiErrorPayload & { description: string }
}

export type UiMessage = InitMessage | PlaceMessage | CancelMessage
export type WorkerMessage = ReadyMessage | ProgressMessage | ResultMessage | ErrorMessage

export const SCHEMA_VERSION = 1

/** Birleşik mesaj tipinden şema/requestId alanlarını dağıtımlı olarak çıkarır. */
export type WorkerBody = WorkerMessage extends infer M
  ? M extends WorkerMessage
    ? Omit<M, 'schemaVersion' | 'requestId'>
    : never
  : never
