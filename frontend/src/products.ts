// Sabit ürün kaydı — plan §9: kimlik, görünen ad, URL ve yerleşim rolü.
// ponytail: gerçek 3 ürün DXF'i gelene kadar yerini tutan sentetik mm fixture'lar;
// gerçek dosyalar bu URL'lere aynı adla konduğunda kod değişmez.
// Rol/etiket bilgisi DXF'ten gelmedigi icin burada bildirilir ve
// load_products'a ayri metadata kanaliyla gecilir.
import type { PlacementRole } from './types/protocol'

export type ProductDefinition = {
  id: string
  name: string
  url: string
  role?: PlacementRole
  tags?: readonly string[]
  ageGroup?: string
}

export const PRODUCTS: readonly ProductDefinition[] = [
  {
    id: 'product-1',
    name: 'Ürün 1',
    role: 'anchor',
    tags: ['main-equipment'],
    ageGroup: 'all',
    url: `${import.meta.env.BASE_URL}products/product-1.dxf`,
  },
  { id: 'product-2', name: 'Ürün 2', role: 'distributed', url: `${import.meta.env.BASE_URL}products/product-2.dxf` },
  { id: 'product-3', name: 'Ürün 3', role: 'peripheral', url: `${import.meta.env.BASE_URL}products/product-3.dxf` },
]
