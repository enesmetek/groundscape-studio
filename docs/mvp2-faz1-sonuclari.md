# GROUNDSCAPE — MVP-2 Faz 1 Sonuçları

**Tarih:** 11 Eylül 2026
**Plan:** `docs/implementation-plans/mvp2-implemantasyon-plani.md` (P5)
**Karşılaştırma:** MVP-1 (`main`, commit `85f01a2`) ↔ Faz 1 (`mvp2-faz1`, P1–P4)

## 1. Ölçüm düzeneği

- Sentetik fixture setleri (tümünde asimetrik footprint; centroid geometriden hesaplanır):
  - `buyuk1kucuk6`: 1×1200² (auto) + 6×500²
  - `fitness7`: 7×600² (benzer boyutlu; eşitlik sahte ana ürün seçmez)
  - `peripheral`: 1×1400² (`peripheral` rolü) + 1×1200² (auto) + 4×500²
  - `ikibuyuk`: 1×1500² (auto) + 1×1450² (auto) + 4×500²
- Seed: 1..=20 (her set), `SearchConfig::default()`, `LayoutObjective::default()`.
- MVP-1, `main` dalının geçici git-worktree kopyasında aynı setlerle koşuldu;
  MVP-1 pozları Faz 1 skor fonksiyonuyla sonradan puanlandı (karşılaştırma aynı
  ölçütle).
- Build: `cargo run --release`.

## 2. Sonuçlar (20 seed ortalaması, doğruluk düzeltmeleri sonrası)

| Set | Tam yerleşim (M1/F1) | Toplam skor (M1/F1) | Aday sayısı (M1/F1) |
|---|---|---|---|
| buyuk1kucuk6 | 20/20 | 0,617 / **0,165** | 59 / 83 |
| fitness7 | 20/20 | 0,351 / **0,129** | 62 / 78 |
| peripheral | 20/20 | 0,508 / **0,409** | 59 / 88 |
| ikibuyuk | 20/20 | 0,512 / **0,231** | 58 / 115 |

Duvar saati ölçümü aynı makinede arka plan yükünden belirgin etkilendiği için
karar metriğinden çıkarıldı; aday sayısı deterministik maliyet vekili olarak
korundu.

### Bileşen dökümü (ortalamalar)

| Set | Kaynak | anchor | balance | distribution | spacing | Toplam |
|---|---|---|---|---|---|---|
| buyuk1kucuk6 | MVP-1 | 0,053 | 0,171 | 0,392 | 0,000 | 0,617 |
| buyuk1kucuk6 | Faz 1 | 0,000 | 0,024 | 0,141 | 0,000 | 0,165 |
| fitness7 | MVP-1 | 0,000 | 0,127 | 0,223 | 0,000 | 0,351 |
| fitness7 | Faz 1 | 0,000 | 0,039 | 0,090 | 0,000 | 0,129 |
| peripheral | MVP-1 | 0,000 | 0,147 | 0,361 | 0,000 | 0,508 |
| peripheral | Faz 1 | 0,000 | 0,094 | 0,316 | 0,000 | 0,409 |
| ikibuyuk | MVP-1 | 0,037 | 0,130 | 0,345 | 0,000 | 0,512 |
| ikibuyuk | Faz 1 | 0,000 | 0,067 | 0,164 | 0,000 | 0,231 |

## 3. Yorum

- **Tam yerleşim başarısı değişmedi:** dört sette de MVP-1 ve Faz 1 20/20
  COMPLETE; `SEARCH_BUDGET_EXHAUSTED` / PARTIAL davranışı bütçe testlerinde
  MVP-1 ile aynı (regresyon testleri).
- **Skor her sette düştü** (düşük = iyi): en büyük iyileşme `balance` ve
  `distribution` bileşenlerinde — bölgesel aday üretimi + skor sıralaması,
  ürünleri merkeze ve 2×2 grid'e dengeli dağıtıyor.
- **Aday sayısı hafif arttı** (çeşitlilik koşulu tamponu mm-varyasyonlarıyla
  dolduramayıp yeni örnekleme yapmaya zorluyor).
- `spacing` bileşeni bu fixture'larda ~0: hedef boşluk (0,05·5000 = 250 mm)
  ürün merkezleri arasında kolayca sağlanıyor. Ağırlık ayarı P5 için gereken
  ek kalibrasyon ihtiyacını göstermiyor; başlangıç ağırlıkları
  (anchor/balance/distribution/spacing = 1, orientation = 0) korundu.

## 4. Faz 2 kararı

Sayısal ölçüt: Faz 1, MVP-1'in tam yerleşim başarısını kaybetmeden tüm
setlerde skoru anlamlı biçimde düşürüyor → **Faz 2 (P6–P8) açılmadı.**

**Uyarı:** Plan §9'a göre Faz 1'in gerçek teslim ölçütü skor değil, kabul
senaryolarının görsel + sayısal olarak karşılanmasıdır; görsel değerlendirme
otomatik ölçümle atlanamaz. Görsel onay kullanıcıya aittir; onay gelirse Faz 1
tamamlanmış sayılır. Görsel değerlendirme olumsuz çıkarsa Faz 2 karar bu
belgenin güncellenmesiyle yeniden alınır.

## 4b. P5.5 — Görünürlük zinciri (tamamlama)

İlk ölçüm turunda rol/objective kanalları yalnızca Rust içindeydi; tarayıcıda
roller kullanılamıyor ve görsel kapı geçilemiyordu. Tamamlama adımları:

1. `wasm_api.rs::start_placement` üçüncü (opsiyonel) parametre olarak
   `objective` aldı; undefined/null → `LayoutObjective::default()`.
   `Engine::set_objective` artık canlı yolda besleniyor.
2. `LoadProductInput`'a opsiyonel `placementRole`/`tags`/`ageGroup` eklendi;
   import sonrası `Engine::override_product_metadata` override uygular
   (DXF bu bilgiyi taşımıyor).
3. `products.ts` gerçek 3 DXF için rol bildirir: product-1 → `anchor`
   (en büyük footprint 900²/1500²), product-2 → `distributed`,
   product-3 → `peripheral`. UI ürün listesi rolü etiketle gösterir;
   PlacementCanvas anchor ürünün safety zone'unu düz turuncu kenarlıkla
   işaretler.
4. Kapı: `npm run build` (gerçek WASM + gerçek 3 DXF) ile Playwright e2e
   geçer; `frontend/tests/placement.spec.ts` rol etiketleriyle birlikte
   anchor'ın merkez bölgede, peripheral'ın merkez dışında olduğunu doğrular.
   Ekran görüntüsü: tarayıcıda `npm run dev` / `npm run preview` ile
   doğrulama kullanıcıya aittir (aşağıdaki uyarıyla aynı kapı).

## 5. Uygulamadaki sapmalar (plan metnine göre)

1. **`E_distribution` metriği:** plan "maks−min sapması (örn.)" öneriyor.
   `maks−min` kısmi yerleşimlerde (boş hücre) her zaman 1,0'a sabitlenip diğer
   bileşenleri eziyor; bu yüzden hücre paylarının eşit paydan ortalama mutlak
   sapması `(Σ|sₖ−Σ/g|)/(2Σ)` ∈ [0,1] kullanıldı. "Her hücrede ürün olsun"
   hedefi yok; ürün, alanına eşit kare olarak hücrelere bölünür.
2. **`auto`→anchor çıkarımı baskın maksimumdur:** en büyük footprint ikinci
   büyükten %1'den fazla büyük değilse `auto` kalır. Eşit veya sayısal gürültü
   düzeyinde farklı fitness ürünleri sahte ana ürün seçmez.
3. **`search_placement` imzasındaki `&LayoutObjective`** plan P4'te
   öngörülüyordu; bölgesel örnekleme merkez bölge oranını (P3) zaten
   gerektirdiği için P3'te eklendi.
4. **Çeşitlilik koşulu**, planın "aday, tampon üyelerinin en az birinden
   eşikten uzak olmalı" cümlesinin yerine daha güçlü olan "en yakınına
   eşikten uzak" olarak uygulandı: "en az bir" okuması, tek konumun
   mm-varyasyonlarının tamponu doldurmasına izin veriyordu (plan §2.7
   amacıyla çelişir).
5. **Dağılım footprint'i alan-eşdeğer kare vekilidir:** gerçek döndürülmüş
   footprint hücre kesişimi hesaplanmaz. Bu, Faz 1 için bilinçli ve belgelenmiş
   yaklaşımdır; konkav/çok parçalı ürünlerde hücre payı yaklaşık değerdir.

## 6. Doğruluk ve dayanıklılık düzeltmeleri

- Aday bütçesi her örnek için yalnız bir kez sayılır; küçük worker dilimleri
  aynı seed ve kümülatif bütçeyle tek çağrının sonucunu üretir.
- **Aday tamponu atomiktir:** bütçe ortasında kalan kısmi tampon sıralamaya
  katılmaz ve replay'de yeniden kurulur; bu prefix-stabil davranış worker
  dilim boyutundan bağımsız sonucu garanti eder (`INVALID_SEARCH_CONFIG`
  için `SearchConfig::validate` sınır denetimine bakın).
- Geçersiz objective/arama yapılandırması, boş ürün kümesi ve
  self-intersecting DXF kontrollü hata döndürür; Jagua shape üretimi bu
  girdilerde panic etmez. Worker PLACE hataları da yapılandırılmış API kodu
  taşır (`toApiError`).
- `SEARCH_BUDGET_EXHAUSTED`, yalnız örnekleme bütçesi kullanıldığında döner;
  sıfır adaylı yapılandırma `SEARCH_SPACE_EXHAUSTED` olarak ayrılır.
- Spacing merkez uzaklığı yerine alan-eşdeğer kare footprint kenar boşluğunu
  ölçer (diagonal komşular dahil); anchor ve spacing terimleri ürün/çift
  sayısına normalize edilir.
- Frontend INIT mesajı `placementRole`, `tags` ve `ageGroup` metadata'sını
  tip güvenli biçimde worker/WASM sınırına taşır ve READY yükünde geri döner.
- Ölçüm fixture'ları (`faz1_bench`, `mvp1_score`, arama testleri) canonical
  geometriye taşındı: safety merkezi origin'de, footprint centroid'i
  geometriden hesaplanır; MVP-1/Faz-1 karşılaştırması üretim geometrisiyle
  aynı sözleşmeyi kullanır.

Doğrulama: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `npm run wasm:dev`, `npm run typecheck`,
`npm run test:ui`, `npm run build` ve `npm run test:e2e`.

## 7. Tekrar üretim

```powershell
# MVP-1 tarafı (main dalı geçici worktree'de):
git worktree add C:\Users\<tmp>\gs-mvp1 main
# core/examples/mvp1_bench.rs'i worktree'ye kopyala (MVP-1 imzasına uygun hali)
cargo run --release -p groundscape-core --example mvp1_bench   # RESULT/PLACE satırları

# Faz-1 tarafı:
cargo run --release -p groundscape-core --example faz1_bench > faz1_bench.txt

# MVP-1 yerleşimlerini Faz-1 skoruyla puanlama:
cargo run --release -p groundscape-core --example mvp1_score -- <mvp1_bench.txt>
# veya pipeline icin: ...mvp1_bench | ...mvp1_score -- -

# Tam doğrulama:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Frontend/WASM:
cd frontend
npm run wasm:dev
npm run typecheck
npm run test:ui
npm run build
npm run test:e2e
```

`mvp1_score` örneği MVP-1 çıktısındaki `PLACE` satırlarını Faz-1 skor
bileşenleriyle puanlar (`SCORE` satırları); `aggregate` hesabı bu iki çıktının
`RESULT`/`SCORE` satırlarından yapılır.
