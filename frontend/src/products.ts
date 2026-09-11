// Sabit ürün kaydı — plan §9: yalnızca kimlik, görünen ad, URL.
// ponytail: gerçek 3 ürün DXF'i gelene kadar yerini tutan sentetik mm fixture'lar;
// gerçek dosyalar bu URL'lere aynı adla konduğunda kod değişmez.
export const PRODUCTS = [
  { id: 'product-1', name: 'Ürün 1', url: `${import.meta.env.BASE_URL}products/product-1.dxf` },
  { id: 'product-2', name: 'Ürün 2', url: `${import.meta.env.BASE_URL}products/product-2.dxf` },
  { id: 'product-3', name: 'Ürün 3', url: `${import.meta.env.BASE_URL}products/product-3.dxf` },
] as const
