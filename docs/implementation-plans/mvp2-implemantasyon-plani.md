# GROUNDSCAPE — MVP-2 Detaylı Implementasyon Planı

**Tarih:** 11 Eylül 2026
**Önerilen repo konumu:** `docs/implementation-plans/mvp2-implemantasyon-plani.md`
**Dayanak:** "GROUNDSCAPE — Yerleşim Algoritması Geliştirme Önerisi (v2)" tasarım belgesi. Bu plan o belgedeki tasarım kararlarını iş paketlerine çevirir; tasarım gerekçeleri orada, uygulama adımları buradadır.

> Bu belge bir geliştirme planıdır. Kod henüz değiştirilmemiştir. Mevcut kod noktaları (`search.rs`, `model.rs`, `engine.rs`, `jagua_adapter.rs`) incelenerek doğrulanmıştır. Skor ağırlıkları ve bütçe sayıları gerçek fixture'larda ölçülmeden kesin değer sayılmamalıdır.

## 1. Hedef ve kapsam

MVP-1 motoru geçerli yerleşim buluyor; MVP-2 **iyi** yerleşim bulmayı hedefler: ana ekipman merkezi bölgede, ikincil ürünler dengeli dağılmış, salıncak gibi büyük safety-zone'lu ama merkeze ait olmayan ürünler kenarda.

Tek cümlelik dönüşüm (tasarım §10): "ilk geçerli konumu kabul et" → "geçerli çözümler arasında en iyi skorluyu seç".

İş iki fazda yürür:

| Faz | İçerik | Risk | Koşul |
|---|---|---|---|
| **Faz 1** | Skor katmanı + greedy aday seçimi. Mevcut backtracking mimarisi aynen korunur. | Düşük — adaptör/Worker protokolü değişmez | Her zaman yapılır |
| **Faz 2** | İyileştirme döngüsü (move/rotate/swap), adaptöre eski-pozu-hariç sorgu, Engine'de kalıcı arama durumu, protokol aşamaları | Yüksek — adaptör sözleşmesi ve Worker protokolü değişir | Yalnızca Faz 1 ölçümleri yetersiz kalırsa |

Faz 1 bitmeden Faz 2'ye başlanmaz. Faz 2 kararı, §7'deki karşılaştırma ölçümlerine göre verilir.

## 2. Kesinleşen tasarım kararları

Tasarım belgesinden alınan, bu planda tartışılmadan uygulanacak kararlar:

1. **İki ayrı büyüklük kavramı** (§3): sıralama = `safety_area_mm2` (değişmez), skorlama = footprint alanı (yeni).
2. **`placementRole`** (§4.1): `auto | anchor | distributed | peripheral`. Güvenlik sınıfı değil, tasarım tercihi. `auto` rolünde ürün türü çıkarımı yapılmaz; yalnızca geometriye bakılır.
3. **Skor referansı footprint centroid** (§4.3): `Pose` safety-zone merkezidir; skorlamada dünya footprint merkezi ayrıca hesaplanır. Geometri/origin sözleşmesi değişmez.
4. **Merkezleme bölge tercihidir** (§4.2): tek noktaya çekme yok; birden fazla büyük ürün dengeli iki odak oluşturabilir.
5. **Beş skor bileşeni** (§4.4): ana öğe, görsel denge, bölgesel dağılım (grid), komşuluk mesafesi, yönlenme. Ağırlıklar `LayoutObjective` config'inde; tüm uzaklık terimleri alan boyutuna normalize.
6. **Bottom-left tie-break kaldırılır** (§4.5): `search.rs:235`'teki `x+y` sıralamasının yerine skor sıralaması gelir.
7. **Aday çeşitliliği** (§5): tampon farklı bölgelerden doldurulur; `perturb_around` yerel havuzu genel havuzdan ayrılır.
8. **Güvenlik asla gevşetilmez:** daha iyi skor için safety zone küçültülmez, tolerans değişmez. Skor yalnızca geçerli çözümler arasında seçim yapar.

## 3. Faz 1 — iş paketleri

### P1: Model genişletmesi (`core/src/model.rs`, `core/src/dxf_import.rs`)

`ProductGeometry`'e eklenir:

```text
footprint_area_mm2: f64            # import sırasında bir kez hesaplanır (shoelace, tüm parçaların toplamı)
footprint_centroid_local: (f64,f64) # canonical koordinatta, import sırasında bir kez hesaplanır
placement_role: PlacementRole       # default: Auto; serde ile camelCase
```

- `PlacementRole` enum'u `model.rs` içinde: `Auto | Anchor | Distributed | Peripheral`, `#[serde(rename_all = "camelCase")]`, `Default = Auto`.
- Çok footprint parçasında centroid, parçaların alan ağırlıklı ortalamasıdır.
- Hesaplama `dxf_import.rs`'te canonicalize sonrası yapılır; her adayda yeniden hesaplanmaz.
- `wasm_api.rs` DTO'suna `placementRole` eklenir (opsiyonel, yoksa `auto`).
- **Ön hazırlık (tasarım §7):** aynı commit'te `tags: Vec<String>` ve `age_group: Option<String>` alanları opsiyonel olarak eklenir; Faz 1'de hiçbir kod bunları okumaz. Amaç, gelecekte model değişikliği gerektirmemesi.

**Kapı:** import testleri yeni alanları doğruluyor; asimetrik safety-buffer'lı sentetik fixture'da centroid ≠ (0,0) olduğunu kanıtlayan test var.

### P2: Skor modülü (yeni `core/src/scoring.rs`)

```text
LayoutObjective:
  weights { anchor, balance, distribution, spacing, orientation }  # başlangıç: hepsi 1.0 dışındakiler ayarla görülür
  grid: 2                    # bölgesel dağılım grid'i (2×2 başlangıç; 3×3 opsiyonel)
  anchor_region_ratio: f64   # "merkezi bölge" yarı genişliği, alan boyutuna oranla (başlangıç önerisi 0.25)
  spacing_target_ratio: f64  # komşuluk hedef boşluğu, alan boyutuna oranla

score_layout(placed: &[ScoredItem], area, objective) -> f64   # düşük = iyi
score_layout_delta(candidate_pose, product, placed, ...) -> f64
```

- `ScoredItem = { dünya_footprint_merkezi, footprint_alanı, rol }`. `placed` listesi mevcut `state`'ten türetilir; ayrı durum saklanmaz.
- Dünya merkezi: `pose + rotate(footprint_centroid_local, rotation_rad)` (tasarım §4.3 formülü).
- Beş bileşen (tasarım §4.4 tablosu aynen):
  - `E_anchor`: yalnızca `anchor` + `auto`-büyük ürünler; merkezi bölgeye normalize uzaklık. Tüm büyükleri tek noktaya çekmez — bölge içi uzaklık sıfırdır.
  - `E_balance`: `Σ AᵢCᵢ / Σ Aᵢ` ağırlıklı merkezin alan merkezine normalize uzaklığı.
  - `E_distribution`: grid hücrelerindeki footprint alanı dağılımının dengesizliği (örn. hücre paylarının maks-min sapması). "Her hücrede ürün olsun" hedefi YOK.
  - `E_spacing`: yakın komşu çiftlerinde hedef boşluktan sapma; sınırsız uzaklaşmayı ödüllendirmez.
  - `E_orientation`: yalnızca rol/fixture tanımladığında aktif; başlangıçta ağırlığı 0 olabilir (ölçülmeden zorlanmaz).
- `auto` rol için başlangıç önerisi: footprint alanı, ürün seti içinde en büyükse anchor gibi puanlanır; `placement_role` açıkça verildiyse geometri çıkarımı yapılmaz.
- Tüm terimler `area_size`'a bölünerek normalize edilir; sabit mm eşikleri kullanılmaz.

**Kapı:** skor birim testleri: köşelere yayılmış dört ürün, dengeli merkezi yerleşimden kötü puan alıyor (tasarım §9 "dört köşe" senaryosu). Skor fonksiyonu deterministik — aynı girdi, aynı puan.

### P3: Aday üretimi çeşitlendirme (`core/src/search.rs`)

- `generate_candidates`'a rol-farkındalıklı bölge örneklemesi eklenir:
  - `anchor`/auto-büyük → merkez bölge çevresinden örnekler,
  - `peripheral` → farklı kenarlardan örnekler (mevcut kenar/köşe hizalı örnekler buraya doğal oturur),
  - diğerleri → alanın farklı kesimlerinden.
- Tampon (`candidate_buffer_size`) doldurulurken **çeşitlilik koşulu**: yeni aday, mevcut tampon üyelerinin en az birinden `scale`'e oranlı bir eşikten (örn. 0.25×√safety_area) uzak olmalı. Aynı konumun mm-varyasyonları tamponu dolduramaz.
- `perturb_around` yalnızca farklı bölge adayları denendikten sonra, tamponun kalan slotları için devreye girer (tasarım §5 — genel/yerel havuz ayrımı).
- **Ayrı düzeltme:** `perturb_around`'daki `shrink = 1/√local_left` ters çalışıyor (`search.rs:360`) — `local_left` küçüldükçe hareket büyüyor. Küçülen pertürbasyon için `shrink = √(local_left / local_total)` veya denk bir form kullanılır; yorum ile davranış eşleşir.

**Kapı:** seed sabitken tampon içeriğinin bölgelere yayıldığı test ediliyor; pertürbasyon ölçeğinin monoton küçüldüğü birim testi var.

### P4: Skor bazlı aday sıralaması (`core/src/search.rs`)

- `try_depth`'teki bottom-left sıralama (`search.rs:234-239`) silinir; yerine:

```rust
candidates.sort_by(|a, b| {
    score_layout_delta(*a, product, &placed, objective)
        .partial_cmp(&score_layout_delta(*b, product, &placed, objective))
        .unwrap_or(Ordering::Equal)
});
```

  (Düşük skor önce; `placed`, `state` + `products`'tan bu derinlikte türetilir.)
- `BestProgress` genişletilir: yerleşen ürün sayısı birincil anahtar olarak kalır (geçerlilik önce gelir), eşit sayıda **düşük toplam skor** ikincil anahtar olur. Yani `best.state` artık "en çok ürün + en iyi skor" tutar.
- `search_placement` imzasına `&LayoutObjective` eklenir; `SearchConfig`'e gömülmez (ayrı kavramlar).
- Geri izleme, yeniden başlatma, bütçe, tek kutu kuralı, `validation` kapısı **aynen korunur**. Skor hiçbir yerde geçerlilik kontrolünün önüne geçmez.

**Kapı:** §7'deki kabul senaryolarının Faz 1 regresyon testleri geçiyor; `SEARCH_BUDGET_EXHAUSTED` ve COMPLETE/PARTIAL davranışları MVP-1 ile aynı.

### P5: Faz 1 ölçüm ve ayar

- Aynı ürün setleri + aynı 20 seed ile MVP-1 ve Faz 1 karşılaştırması: tam yerleşim başarısı, skor bileşen dökümü, aday sayısı, süre, SVG üzerinde görsel değerlendirme.
- `LayoutObjective` ağırlıkları bu ölçümle ayarlanır; ayarlanmış değerler koda/test fixture'ına yazılır.
- Sonuç `docs/spike-results.md` veya yeni bir `docs/mvp2-faz1-sonuclari.md` dosyasına kaydedilir.

**Kapı:** Faz 2'ye geçip geçmeme kararı bu kayda dayanarak verilir. Ölçüm yoksa karar yok.

## 4. Faz 2 — iş paketleri (koşullu)

Yalnızca P5 ölçümleri Faz 1'i yetersiz gösterirse açılır.

### P6: Adaptörde hareket sorgusu (`core/src/jagua_adapter.rs`)

- "Bu ürünün eski pozunu hariç tutarak çakışma sorgula" desteği: `try_place`'in yanında `try_move(item, old_pose, new_pose)` veya eşdeğeri. Mevcut `restart_from` ile tüm durumu yeniden kurmak, iyileştirme döngüsünün her adımında pahalıdır; amaç tek ürünün pozunu yerinde değiştirebilmek.
- Tek kutu kuralı ve kimlik eşleme sözleşmesi değişmez.

### P7: İyileştirme döngüsü (`core/src/search.rs`)

- İlk COMPLETE çözümden sonra bütçe yettiğince move/rotate/swap denemeleri; her kabul edilen hamle skoru düşürmeli (greedy) ve `validation` kapısından geçmeli.
- Durma koşulu: bütçe biter veya N hamledir iyileşme olmaz.

### P8: Engine kalıcı durumu ve protokol (`core/src/engine.rs`, `frontend/src/types/protocol.ts`, Worker)

- Worker dilimleri arasında arama ilerlemesi sıfırlanmaz: RNG durumu, en iyi geçerli yerleşim, kalan bütçe oturumda kalır. Mevcut `Engine::step_placement`'ın her dilimde sıfırdan `search_placement` çağırması düzeltilir.
- Aşamalar: `FINDING_LAYOUT → IMPROVING_LAYOUT → FINISHED`. `done=true` yalnızca iyileştirme bütçesi bitince. UI önce geçerli sonucu gösterir, sonra iyileşmeleri günceller (yeni `PROGRESS`/`RESULT` alanları, `schemaVersion` artırımı değerlendirilir).
- Aynı seed + aynı toplam bütçe, dilim boyutundan bağımsız tutarlı sonuç üretir.

**Kapı:** §7'nin Faz 2 senaryoları; E2E'de UI iyileştirme sırasında yanıt veriyor; iptal/yeniden başlatma bozuk durum bırakmıyor.

## 5. Kapsam dışı (bilinçli, tasarım §7)

- Alan geometrisinin genelleştirilmesi (5000×5000 sabiti kalır) — ayrı iş paketi.
- Oyun/fitness için ayrı objective profilleri — `LayoutObjective`'e şimdiden opsiyonel tek bir `preset: Option<String>` alanı konabilir; profil implementasyonu YOK.
- Yaş/fonksiyon etiketi mantığı — yalnızca P1'deki boş `tags`/`age_group` alanları.
- Dolaşım/erişim bölgesi modellemesi — Faz 2 sonrası.

## 6. Repo içi değişiklik özeti

| Dosya | Değişiklik | Paket |
|---|---|---|
| `core/src/model.rs` | `PlacementRole`, `footprint_area_mm2`, `footprint_centroid_local`, opsiyonel `tags`/`age_group` | P1 |
| `core/src/dxf_import.rs` | Yeni alanların import sırasında hesabı | P1 |
| `core/src/wasm_api.rs` | DTO'ya `placementRole` (opsiyonel) | P1 |
| **Yeni:** `core/src/scoring.rs` | `LayoutObjective` + beş bileşen + `score_layout_delta` | P2 |
| `core/src/search.rs` | Bölgesel aday üretimi, çeşitlilik koşulu, perturb düzeltmesi, skor sıralaması, `BestProgress` skoru | P3, P4, P7 |
| `core/src/jagua_adapter.rs` | Eski-pozu-hariç hareket sorgusu | P6 |
| `core/src/engine.rs` | Kalıcı arama durumu, aşama ayrımı | P8 |
| `frontend/src/types/protocol.ts` + Worker | Aşama/kalite/en-iyi-sonuç güncellemeleri | P8 |
| `core/tests/` | `scoring_tests.rs` (yeni), `search_tests.rs` genişletme | P2–P4 |

## 7. Kabul senaryoları ve test planı

Tasarım §9'daki senaryolar regresyon testine dönüşür. Sentetik fixture'lar kontrollü üretilir (asimetrik safety-buffer, farklı footprint oranları).

| Senaryo | Test | Faz |
|---|---|---|
| Bir büyük + altı küçük | Büyük merkezi bölgede; küçükler tek tarafa yığılmıyor | 1 |
| İki büyük + küçükler | İki odak; ikisi de tam merkeze zorlanmıyor | 1 |
| Benzer boyutlu fitness seti | Sahte "ana ürün" seçilmiyor; dağılım bütünlüğü | 1 |
| Büyük safety'li `peripheral` ürün | Merkeze alınmıyor | 1 |
| Asimetrik footprint/safety ürün | Merkezleme dünya footprint merkezine göre; `pose ≠ centroid` fixture'ı | 1 |
| Dört köşeye yayılmış çözüm | `E_distribution` bunu dengeli saymıyor (skor birim testi) | 1 |
| Yetersiz alan | Tolerans/safety gevşetme yok; davranış MVP-1 ile aynı | 1 |
| Aynı seed, farklı Worker dilim boyutları | Tutarlı sonuç | 2 |

MVP-1'in tüm mevcut testleri koşulsuz geçmeye devam eder; özellikle tek kutu, bütçe, doğrulama ve Worker yaşam döngüsü testleri. Karşılaştırma yalnızca toplam skora bakarak yapılmaz — tam yerleşim başarısı, bileşen dökümü, süre ve görsel değerlendirme birlikte (tasarım §9 son paragraf).

Günlük akış MVP-1 ile aynı:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 8. Bağımlılık sırası

```text
P1 → P2 → P3 → P4 → P5 (karar noktası)
                      └→ gerekiyorsa: P6 → P7 → P8
```

P3 ve P4 aynı dosyaya dokunur; ayrı commit'ler ama ardışık yapılır. Her paket tek, incelenebilir değişiklik; commit açıklamasında değişen dosyalar, çalıştırılan komutlar ve geçen testler belirtilir. Testi çalıştırılmamış paket "tamamlandı" sayılmaz.

## 9. Tamamlanma kontrolü (Faz 1)

- Yeni model alanları import'da hesaplanıyor, test edilmiş.
- `scoring.rs` beş bileşenle deterministik puan üretiyor; köşe-yığılması testi geçiyor.
- Bottom-left tie-break kod tabanında yok; aday seçimi skor bazlı.
- Aday tamponu bölge çeşitliliği koşuluyla doluyor; perturb ölçeği küçülüyor.
- Tüm güvenlik kontrolleri (safety zone, tek kutu, bağımsız doğrulama) değişmeden geçiyor.
- 20 seed'lik karşılaştırma kaydı yazılı; Faz 2 kararı bu kayda bağlı.

**Faz 1'in gerçek teslim ölçütü skor değil, kabul senaryolarının görsel + sayısal olarak karşılanmasıdır.** Algoritma kendi skorunu yükseltirken tasarım açısından kötüleşebilir; bu yüzden görsel değerlendirme atlanamaz.
