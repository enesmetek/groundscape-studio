// Sabit ürün kaydı — plan §9: kimlik, görünen ad, URL ve yerleşim rolü.
// ponytail: gerçek 3 ürün DXF'i gelene kadar yerini tutan sentetik mm fixture'lar;
// gerçek dosyalar bu URL'lere aynı adla konduğunda kod değişmez.
// Rol bilgisi DXF'ten gelmediği için burada bildirilir (MVP-2 plan P5.5) ve
// load_products'a opsiyonel placementRole olarak geçilir.
export const PRODUCTS = [
  { id: 'product-1', name: 'Ürün 1', role: 'anchor', url: `${import.meta.env.BASE_URL}products/product-1.dxf` },
  { id: 'product-2', name: 'Ürün 2', role: 'distributed', url: `${import.meta.env.BASE_URL}products/product-2.dxf` },
  { id: 'product-3', name: 'Ürün 3', role: 'peripheral', url: `${import.meta.env.BASE_URL}products/product-3.dxf` },
] as const
