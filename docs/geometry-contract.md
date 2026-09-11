# Geometri sözleşmesi — Aşama B

Tarih: 11 Eylül 2026. Plan: `docs/implementation-plans/mvp1-implementasyon-plani.md` §8.

## Birimler ve koordinatlar

- İş mantığında birim daima **milimetre**. Alan sabit: `0 ≤ x ≤ 5000`, `0 ≤ y ≤ 5000` (`core/src/area.rs`).
- Arayüzde ikinci bağımsız 5000 hesabı yok; alan bilgisi çekirdeğten alınır.
- Uygulama kaynak geometrisi **f64** taşır. jagua-rs f32 geometri kullanır; dönüşüm yalnızca `jagua_adapter.rs` sınırında yapılır.

## Poz (Pose)

```
x_mm, y_mm      : f64, mm
rotation_rad    : f64, radyan
```

- Rust/Worker/arama **radyan** kullanır. Derece dönüşümü yalnızca SVG ve kullanıcı metninde yapılır.
- Pozlar saklanmadan/doğrulanmadan önce yuvarlanmaz; ekrandaki metin kısaltılır.

## Ortak origin

- Bir ürünün FOOTPRINT ve SAFETY_ZONE katmanları aynı DXF koordinat sisteminde bulunur.
- Canonical hazırlık: SAFETY_ZONE bbox merkezi referans alınır ve **iki katmandan da aynı şekilde** çıkarılır (`geometry::canonicalize`). Bağımsız merkezleme yasaktır.
- Çıkarılan ofset geri dönüşte kullanılır: `T_world = T_canonical`, original = canonical + offset.
- jagua `pre_transform` bilinçli olarak identity bırakılmıştır; motor içi ek koordinat değişimi yoktur.
- Safety zone küçültebilecek sadeleştirme/offset/dar girinti dönüşümleri kullanılmaz.

## Tolerans politikası

| Sabit | Değer | Kullanım |
|---|---|---|
| `LINEAR_EPSILON_MM` | 0.01 mm | Kenar teması, sınır kontrolü, nokta-arka plan ilişkisi |
| `AREA_EPSILON_MM2` | 1.0 mm² | Poligon alan karşılaştırmaları |

Bunlar sayısal toleranstır; fiziksel güvenlik mesafesi veya tasarım payı değildir. Uygulama footprint'ten safety zone üretmez.

## Model

```
ProductGeometry { id, footprint_polygons[], safety_zone, safety_area_mm2, source_metadata? }
Placement       { product_id, pose }
```

- Birden fazla footprint parçası olabilir; **safety zone tek, kapalı, deliksiz, basit poligondur**.
- Çok parçalı/delikli safety zone ihtiyacı çıkarsa sessizce dış kontura çevrilmez; sözleşme açıkça genişletilir.

## Doğrulanan testler (native)

- Shoelace alan (100², 5000²), alan toplamı 25.000.000 mm².
- 37° dönüş: alan korunur, bbox genişliği `size·(|cosθ|+|sinθ|)`, ters dönüş nokta bazında kimliğe döner (±0.01 mm).
- Canonicalize: safety bbox merkezi sıfıra iner; footprint-safety göreli konumu korunur; ofset geri uygulaması orijinali verir.
- Alan sınırı: sınır noktaları dahil; `LINEAR_EPSILON_MM` dışındaki taşmalar reddedilir.
