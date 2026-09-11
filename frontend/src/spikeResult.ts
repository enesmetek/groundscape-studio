export type SpikeResult = {
  engine: string
  dxf: {
    units_mm: boolean
    closed_lwpolyline: boolean
    vertex_count: number
  }
  overlap_detected: boolean
  continuous_rotation_enabled: boolean
  angle_37_accepted: boolean
  exact_fit_accepted: boolean
  out_of_area_rejected: boolean
  unfit_item_rejected: boolean
  area_size: [number, number]
  bins_used: number
}

export function spikePassed(result: SpikeResult): boolean {
  return (
    result.dxf.units_mm &&
    result.dxf.closed_lwpolyline &&
    result.overlap_detected &&
    result.continuous_rotation_enabled &&
    result.angle_37_accepted &&
    result.exact_fit_accepted &&
    result.out_of_area_rejected &&
    result.unfit_item_rejected &&
    result.bins_used === 1 &&
    result.area_size[0] === 5000 &&
    result.area_size[1] === 5000
  )
}
