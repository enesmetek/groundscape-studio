import { describe, expect, it } from 'vitest'
import { spikePassed, type SpikeResult } from './spikeResult'

const passingResult: SpikeResult = {
  engine: 'jagua-rs 0.8.1 BPP collision query',
  dxf: { units_mm: true, closed_lwpolyline: true, vertex_count: 4 },
  overlap_detected: true,
  continuous_rotation_enabled: true,
  angle_37_accepted: true,
  exact_fit_accepted: true,
  out_of_area_rejected: true,
  unfit_item_rejected: true,
  area_size: [5000, 5000],
  bins_used: 1,
}

describe('spikePassed', () => {
  it('requires every reported proof boolean', () => {
    expect(spikePassed(passingResult)).toBe(true)

    for (const key of [
      'overlap_detected',
      'continuous_rotation_enabled',
      'angle_37_accepted',
      'exact_fit_accepted',
      'out_of_area_rejected',
      'unfit_item_rejected',
    ] as const) {
      expect(spikePassed({ ...passingResult, [key]: false })).toBe(false)
    }
    expect(spikePassed({ ...passingResult, dxf: { ...passingResult.dxf, units_mm: false } })).toBe(false)
    expect(spikePassed({ ...passingResult, dxf: { ...passingResult.dxf, closed_lwpolyline: false } })).toBe(false)
  })
})
