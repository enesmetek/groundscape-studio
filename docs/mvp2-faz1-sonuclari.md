# GROUNDSCAPE — MVP-2 Faz 1 Sonuçları

**Tarih:** 11 Eylül 2026
**Plan:** `docs/implementation-plans/mvp2-implemantasyon-plani.md` (P5)
**Karşılaştırma:** MVP-1 (`main`, commit `85f01a2`) ↔ Faz 1 (`mvp2-faz1`, P1–P4)

## 1. Ölçüm düzeneği

- Sentetik fixture setleri (tümünde footprint = safety'nin 100 mm içerisi; canonical merkezde):
  - `buyuk1kucuk6`: 1×1200² (auto) + 6×500²
  - `fitness7`: 7×600² (benzer boyutlu; eşitlik sahte ana ürün seçmez)
  - `peripheral`: 1×1400² (`peripheral` rolü) + 1×1200² (auto) + 4×500²
  - `ikibuyuk`: 1×1500² (auto) + 1×1450² (auto) + 4×500²
- Seed: 1..=20 (her set), `SearchConfig::default()`, `LayoutObjective::default()`.
- MVP-1, `main` dalının geçici git-worktree kopyasında aynı setlerle koşuldu;
  MVP-1 pozları Faz 1 skor fonksiyonuyla sonradan puanlandı (karşılaştırma aynı
  ölçütle).
- Build: `cargo run --release`.

## 2. Sonuçlar (20 seed ortalaması)

| Set | Tam yerleşim (M1/F1) | Toplam skor (M1/F1) | Aday sayısı (M1/F1) | Süre ms (M1/F1) |
|---|---|---|---|---|
| buyuk1kucuk6 | 20/20 | 0,714 / **0,176** | 59 / 80 | 1,3 / 0,7 |
| fitness7 | 20/20 | 0,364 / **0,128** | 57 / 76 | 1,9 / 0,6 |
| peripheral | 20/20 | 0,628 / **0,390** | 57 / 82 | 2,3 / 0,4 |
| ikibuyuk | 20/20 | 0,732 / **0,233** | 59 / 90 | 0,6 / 0,5 |

### Bileşen dökümü (ortalamalar)

| Set | Kaynak | anchor | balance | distribution | spacing | Toplam |
|---|---|---|---|---|---|---|
| buyuk1kucuk6 | MVP-1 | 0,094 | 0,236 | 0,385 | 0,000 | 0,714 |
| buyuk1kucuk6 | Faz 1 | 0,000 | 0,027 | 0,149 | 0,000 | 0,176 |
| fitness7 | MVP-1 | 0,000 | 0,126 | 0,237 | 0,000 | 0,364 |
| fitness7 | Faz 1 | 0,000 | 0,043 | 0,085 | 0,000 | 0,128 |
| peripheral | MVP-1 | 0,000 | 0,224 | 0,402 | 0,002 | 0,628 |
| peripheral | Faz 1 | 0,000 | 0,091 | 0,298 | 0,001 | 0,390 |
| ikibuyuk | MVP-1 | 0,100 | 0,232 | 0,401 | 0,000 | 0,732 |
| ikibuyuk | Faz 1 | 0,000 | 0,088 | 0,145 | 0,000 | 0,233 |

## 3. Yorum

- **Tam yerleşim başarısı değişmedi:** dört sette de MVP-1 ve Faz 1 20/20
  COMPLETE; `SEARCH_BUDGET_EXHAUSTED` / PARTIAL davranışı bütçe testlerinde
  MVP-1 ile aynı (regresyon testleri).
- **Skor her sette düştü** (düşük = iyi): en büyük iyileşme `balance` ve
  `distribution` bileşenlerinde — bölgesel aday üretimi + skor sıralaması,
  ürünleri merkeze ve 2×2 grid'e dengeli dağıtıyor.
- **Aday sayısı hafif arttı** (çeşitlilik koşulu tamponu mm-varyasyonlarıyla
  dolduramayıp yeni örnekleme yapmaya zorluyor), **süre artmadı**.
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

## 5. Uygulamadaki sapmalar (plan metnine göre)

1. **`E_distribution` metriği:** plan "maks−min sapması (örn.)" öneriyor.
   `maks−min` kısmi yerleşimlerde (boş hücre) her zaman 1,0'a sabitlenip diğer
   bileşenleri eziyor; bu yüzden hücre paylarının eşit paydan ortalama mutlak
   sapması `(Σ|sₖ−Σ/g|)/(2Σ)` ∈ [0,1] kullanıldı. "Her hücrede ürün olsun"
   hedefi yok; ürün, alanına eşit kare olarak hücrelere bölünür.
2. **`auto`→anchor çıkarımı benzersiz maksimumdur:** eşitlik (benzer boyutlu
   fitness seti) sahte ana ürün seçmez — plan §7'deki fitness kapısı bu
   davranışı gerektirir.
3. **`search_placement` imzasındaki `&LayoutObjective`** plan P4'te
   öngörülüyordu; bölgesel örnekleme merkez bölge oranını (P3) zaten
   gerektirdiği için P3'te eklendi.
4. **Çeşitlilik koşulu**, planın "aday, tampon üyelerinin en az birinden
   eşikten uzak olmalı" cümlesinin yerine daha güçlü olan "en yakınına
   eşikten uzak" olarak uygulandı: "en az bir" okuması, tek konumun
   mm-varyasyonlarının tamponu doldurmasına izin veriyordu (plan §2.7
   amacıyla çelişir).

## 6. Tekrar üretim

```powershell
# MVP-1 tarafı (main dalı geçici worktree'de):
git worktree add C:\Users\<tmp>\gs-mvp1 main
# core/examples/mvp1_bench.rs'i worktree'ye kopyala (MVP-1 imzasına uygun hali)
cargo run --release -p groundscape-core --example mvp1_bench   # RESULT/PLACE satırları

# Faz-1 tarafı:
cargo run --release -p groundscape-core --example faz1_bench > faz1_bench.txt

# MVP-1 yerleşimlerini Faz-1 skoruyla puanlama:
cargo run --release -p groundscape-core --example mvp1_score -- <mvp1_bench.txt>

# Tam doğrulama:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`mvp1_score` örneği MVP-1 çıktısındaki `PLACE` satırlarını Faz-1 skor
bileşenleriyle puanlar (`SCORE` satırları); `aggregate` hesabı bu iki çıktının
`RESULT`/`SCORE` satırlarından yapılır.
