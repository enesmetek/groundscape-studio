import { describe, expect, it } from 'vitest'
import { toApiError } from './protocol'

describe('toApiError', () => {
  it('normal Error nesnesini yapılandırılmış Worker hatasına çevirir', () => {
    expect(toApiError(new Error('yüklenemedi'), 'WASM_INIT_FAILED', 'init')).toEqual({
      code: 'WASM_INIT_FAILED',
      phase: 'init',
      productId: null,
      description: 'yüklenemedi',
    })
  })

  it('WASM yapılandırılmış hata kodunu korur', () => {
    const error = {
      code: 'INVALID_INPUT',
      phase: 'search',
      productId: null,
      description: 'grid must be between 1 and 3',
    }
    expect(toApiError(error, 'INTERNAL_ERROR', 'search')).toEqual(error)
  })
})
