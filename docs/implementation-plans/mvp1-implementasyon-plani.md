# GROUNDSCAPE — MVP-1 Detaylı Implementasyon Planı

**Tarih:** 10 Eylül 2026  
**Önerilen repo konumu:** `docs/implementation-plan.md`  
**Dayanak:** Kullanıcının paylaştığı “GROUNDSCAPE — MVP-1: YERLEŞİM MOTORU” dokümanı ve bu dokümandaki dört açık karara verdiği yanıtlar.

> Bu belge bir geliştirme planıdır; uygulamanın yazıldığı, derlendiği veya gerçek ürünlerle test edildiği anlamına gelmez. Referans depoların dokümantasyonu ve ilgili kaynak dosyaları incelenmiştir. Üç gerçek ürün DXF’i bu görüşmede paylaşılmadığından bunların birimi, geometrisi, DXF entity türleri ve 5000 × 5000 mm alana birlikte sığması henüz doğrulanmamıştır.

## 1. Hedef ve kapsam

Kullanıcı tek bir web sayfası açar. Sayfada 5000 × 5000 mm sabit alan ve üç sabit ürün bulunur. “Yerleştir” düğmesi, tarayıcıdaki Web Worker içinde çalışan Rust/WASM motorunu çalıştırır. Ürünlerin footprint ve safety zone geometrileri birlikte taşınır ve döndürülür. Üç ürün de sınır içinde ve güvenlik alanları birbirinin iç bölgesiyle çakışmadan yerleştiğinde sonuç başarılıdır.

Bu MVP’de backend, veritabanı, kullanıcı hesabı, şirket/rol yönetimi, ürün yükleme ekranı, sürükle-bırak, pan/zoom, teklif veya 3D bulunmayacak. Vite’nin geliştirme sunucusu bir iş uygulaması backend’i değildir; yalnızca geliştirme dosyalarını sunar. Üretim çıktısı statik dosyalardır.

Önceki geniş kapsamlı .NET/ERP planları bu işin bağımlılığı değildir. Rust çekirdeği gelecekte başka bir host tarafından kullanılabilecek şekilde ayrıştırılacak; fakat şimdi Axum veya başka bir sunucu oluşturulmayacak.

## 2. Kesinleşen kararlar ve planın ek teknik önerileri

### 2.1. Kullanıcı tarafından kesinleşen kararlar

| Konu | Uygulanacak karar |
|---|---|
| DXF katmanları | `FOOTPRINT` ve `SAFETY_ZONE` |
| Alan | Kodda sabit, 5000 mm × 5000 mm |
| Rotasyon | Sürekli açı uzayından örneklemeli arama |
| Yerleştirme sırası | Büyükten küçüğe |

“Büyük” için bu planda kullanılacak ölçüt **SAFETY_ZONE poligonunun alanıdır**. Yerleşimde engel olarak kullanılan geometri bu olduğu için footprint alanı veya bounding box alanı yerine bu değer esas alınacaktır. Eşit alanlı ürünlerde ürün kimliğiyle kararlı sıralama uygulanacaktır. Bu, kullanıcı kararının teknik yorumudur.

### 2.2. Bu planın önerdiği mühendislik tercihleri

Aşağıdakiler kaynak dokümanda kesinleştirilmiş kullanıcı kararları değil, uygulanabilir bir MVP için önerilen ayrıntılardır:

- İlk sürüm tek Web Worker ve seri WASM kullanır; çok iş parçacıklı WASM kurulmaz.
- Safety zone’nun footprint’i kapsadığı geometri sözleşmesi kabul edilir ve gerçek dosyalarda denetlenir. Uymayan dosya sessizce değiştirilmez.
- Kenarların yalnızca birbirine veya alan sınırına değmesi kabul edilir; pozitif iç bölge örtüşmesi kabul edilmez. Sayısal toleranslar geometri testleriyle sabitlenir; fiziksel güvenlik mesafesi olarak yorumlanmaz.
- Üç temiz fixture için ilk DXF sözleşmesi düz kenarlı, kapalı, iki boyutlu konturlarla sınırlanır. Gerçek DXF’ler eğri içeriyorsa bu fark önce belirlenir ve açıkça ele alınır.
- Arama bütçesi sınırlıdır; başarısız örneklemeli arama “matematiksel olarak imkânsız” demek değildir.
- İlk uygun konumlara körlemesine bağlı kalmamak için sınırlı geri izleme ve aynı sırayı koruyan yeniden başlatmalar kullanılır.
- Sonuç, aramanın kullandığı geometriye ek olarak orijinal geometri üzerinde bağımsız bir doğrulamadan geçirilir.

Temas politikası, sayısal toleranslar ve arama bütçeleri kodun çeşitli yerlerine dağılmış sabitler olmayacak. Toleranslar geometri politikası altında, arama ayarları `SearchConfig` içinde tutulacak. Daha kolay sonuç almak için sessizce değiştirilmeyecek.

## 3. Referans depolardan ne alınacak?

| Kaynak | Doğrulanan rol | Bu projede kullanım |
|---|---|---|
| `sparrow-studio` | React/TypeScript arayüzü; Worker içinde Rust/WASM; tarayıcıda yerel çalışma | Worker/WASM yükleme ve statik uygulama deseni için referans |
| `jagua-rs` | Sürekli dönüşümlerde poligon çakışma kontrolü; `bpp` modeli; örnek LBF araması | Çakışma motoru ve tek kutulu problem modeli |
| `sparrow` | Şerit paketleme, yani `spp` optimizasyonu | Arama yaklaşımları için inceleme kaynağı; doğrudan kare alan çözücüsü olarak alınmayacak |

**Kritik ayrım:** `jagua-rs` kullanmak, otomatik yerleşim algoritmasının tamamının hazır olması demek değildir. Ürün sırasını, aday konum/açı üretimini, arama bütçesini, geri izlemeyi ve kullanıcıya dönen sonuçları Groundscape yazacak.

İncelenen `jagua-rs` LBF kaynak kodu ürünleri alana değil çapa göre sıralıyor. Ayrıca mevcut kutulardan sonra stoktaki başka kutulara geçebiliyor. Bu davranışlar aynen taşınmayacak: bizim sıramız safety zone alanına göre ve kutu sayımız daima bir olacak.

LBF’nin rastgele örnekleyicisinde sıfır genişlikli aralıkların ayrıca ele alınması gerekiyor. Güncel `sparrow-studio/web/README.md`, crates.io `jagua-rs 0.8.1` paketinin değiştirilmeden kullanıldığını ve kendi uygulamasının seri/paylaşımlı bellek ile SIMD/uyumluluk kombinasyonlarından oluşan dört WASM varyantını sabit bir nightly araç zinciriyle derlediğini belirtiyor. Groundscape bu build matrisini kopyalamayacak; stable Rust ile kendi sınırlı `bpp` kullanımını ayrı bir spike içinde doğrulayacak.

Tam sığma ve temas davranışı bu spike’ın zorunlu testidir. Demo uygulamasının bütün davranışları yalnızca aynı collision paketini eklemekle elde edilmiş sayılmayacak. Kesin 5000 mm sınırına uyum problemi çıkarsa alanı büyütmek yerine adaptör/örnekleyici düzeltilip regresyon testi eklenecek.

Bu bölümün kaynakları: K1–K9.

Başlangıç bağımlılığı `jagua-rs = 0.8.1`, `bpp` özelliği açık olacak. Kaynakların hareketli `main` dalı üretim bağımlılığı yapılmayacak. Kopyalanan kod olursa kaynak ve lisans kaydı tutulacak.

## 4. Bilgisayar kurulumu — Windows / PowerShell

Bu bölüm Windows üzerinde geliştirme içindir. Mevcut programları kaldırıp yeniden kurmak yerine önce sürümler kontrol edilmelidir. İşletim sistemi sürümü bu planın dayanak dokümanında belirtilmemiştir.

### 4.1. Gerekli araçlar

| Araç | Tercih / görev | Kurulumdan sonra kontrol |
|---|---|---|
| Git | Kaynak kodu ve değişiklik yönetimi | `git --version` |
| Node.js | **24 LTS**; araştırma anındaki 24.x sürümü 24.21.0 | `node --version`, `npm --version` |
| VS Code veya mevcut editör | Rust ve TypeScript geliştirme | Proje klasörünün açılması |
| Visual Studio C++ araçları | Windows MSVC derleyicisi, linker ve Windows SDK | Native Rust derlemesinin geçmesi |
| Rustup | Stable Rust, Cargo, hedef yönetimi | `rustc --version`, `cargo --version` |
| wasm-pack | Başlangıçta **0.15.0** ile sabitleme | `wasm-pack --version` |
| Chromium tabanlı tarayıcı | Worker/WASM ve üretim çıktısı kontrolü | DevTools Console/Network |

10 Eylül 2026 için Node 24 LTS seçiliyor. Node 26 Current olduğu için “en yeni” diye özellikle seçilmeyecek. Node ve Rust’ın gerçekten test edilen tam sürümleri daha sonra repo içinde sabitlenecek.

Visual Studio kuruluysa **Visual Studio Installer → Modify → Desktop development with C++** iş yükünü ekle. MSVC araçları ile Windows SDK bulunmalı. Yalnızca .NET iş yükünün kurulu olması Rust’ın Windows linker ihtiyacını karşılamayabilir. Visual Studio arayüzünü editör olarak kullanmak zorunlu değildir.

VS Code kullanılıyorsa Rust için `rust-analyzer` yeterlidir. Frontend için Vite şablonunun lint altyapısı korunur; çok sayıda ek araçla başlangıç karmaşıklaştırılmaz.

Bu planın başlangıç kurulumu Docker, PostgreSQL, PostGIS, .NET backend, Redis, Python çalışma ortamı, WSL, Rust nightly veya çok iş parçacıklı WASM içermiyor. Stable Rust ile seçilen bağımlılıkların derlendiği teknik spike içinde doğrulanacak.

Kurulum ve sürüm dayanakları: K10–K14, Playwright ortam desteği için K19.

### 4.2. Kurulum komutları

Git yoksa PowerShell’de:

```powershell
winget install --id Git.Git -e --source winget
```

Node.js 24 LTS’yi resmi Windows yükleyicisiyle kur. Rust’ı resmi Rustup Windows yükleyicisiyle, varsayılan MSVC araç zinciriyle kur. Kurulumlardan sonra terminali kapatıp yeniden aç.

```powershell
rustup update stable
rustup default stable
rustup component add rustfmt clippy
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
```

Kontrol:

```powershell
git --version
node --version
npm --version
rustc --version
cargo --version
wasm-pack --version
rustup target list --installed
```

`node --version` çıktısı `v24...` olmalı. Yüklü Rust hedefleri içinde `wasm32-unknown-unknown` görünmeli.

**Playwright notu:** Güncel resmi Windows desteği Windows 11 ve üzeridir. Bilgisayar Windows 10 ise E2E testleri için desteklenen bir Linux CI ortamı seç; yerel test komutlarının resmi destek kapsamında olduğunu varsayma. Bu durum bütün uygulama mimarisini değiştirmeyi gerektirmez.

## 5. Repo ve başlangıç iskeleti

Mevcut Groundscape reposu varsa yeni Vite projesiyle dosyaların üzerine yazma. Ayrı bir çalışma dalı aç ve aşağıdaki yapıyı mevcut dosyalara uyarlayarak ilerle. Aşağıdaki komutlar **boş bir repo** içindir.

```powershell
New-Item -ItemType Directory -Force -Path "C:\dev\groundscape" | Out-Null
Set-Location "C:\dev\groundscape"

git init
cargo new core --lib --name groundscape-core --vcs none
npm create vite@latest frontend -- --template react-ts
New-Item -ItemType Directory -Force -Path "docs","frontend/public/products" | Out-Null

Set-Location frontend
npm install
npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom @playwright/test
npx playwright install chromium
Set-Location ..
```

Bu paket kurulumu test dosyalarını ve yapılandırmalarını kendiliğinden hazırlamaz. İlgili geliştirme aşamasında `vitest.config.ts` içinde React testleri için jsdom ve test setup dosyası; `playwright.config.ts` içinde `e2e` test dizini, preview sunucusu ve base URL tanımlanır.

İlk iskeleti oluştururken `@latest` kullanılabilir. Sonraki kurulumlar `package-lock.json` ve `npm ci` ile tekrarlanmalıdır. Playwright komutunu desteklenen ortamda çalıştır.

Önerilen yapı:

```text
groundscape/
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  .gitignore
  README.md
  core/
    Cargo.toml
    src/
      lib.rs
      model.rs
      error.rs
      area.rs
      geometry.rs
      dxf_import.rs
      jagua_adapter.rs
      placement.rs
      validation.rs
      wasm_api.rs
    tests/
      geometry_tests.rs
      dxf_import_tests.rs
      placement_tests.rs
      fixtures/
  frontend/
    public/products/
      product-1.dxf
      product-2.dxf
      product-3.dxf
    src/
      wasm/                     # wasm-pack tarafindan uretilir
      types/protocol.ts
      workers/placement.worker.ts
      hooks/usePlacementWorker.ts
      components/PlacementCanvas.tsx
      products.ts
      App.tsx
    e2e/placement.spec.ts
    package.json
    package-lock.json
    vite.config.ts
    vitest.config.ts
    playwright.config.ts
  docs/
    mvp-plan.md
    implementation-plan.md
    geometry-contract.md
    spike-results.md
```

Klasörün adı `core` kalsın; Cargo paketinin adı `groundscape-core` olsun. Rust’ın kendi `core` kütüphanesiyle ad karışıklığı yaratılmasın.

Kök `Cargo.toml`:

```toml
[workspace]
members = ["core"]
resolver = "3"
```

İlk `rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = ["wasm32-unknown-unknown"]
```

İlk teknik doğrulama geçtiğinde `channel`, test edilen tam Rust sürümüne çevrilir. Node’un test edilen tam sürümü `.node-version` gibi bir dosyaya yazılır.

`Cargo.lock` ve `package-lock.json` Git’e alınır. `target/`, `frontend/node_modules/`, `frontend/dist/` ve `frontend/src/wasm/` alınmaz. WASM çıktısı CI ve yerel build sırasında üretilir. Ürün DXF’leri dağıtıma giren sabit uygulama varlıklarıdır; bunları repoya ve statik yayına koyma yetkisi ayrıca kontrol edilmelidir.

## 6. Rust bağımlılıkları ve ilk derleme kapısı

Aşağıdaki manifest başlangıç önerisidir. “Bütün transitive bağımlılıklar tarayıcıda test edilmiştir” anlamına gelmez; bu doğrulama bir sonraki aşamanın teslimidir.

`core/Cargo.toml`:

```toml
[package]
name = "groundscape-core"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["rlib", "cdylib"]

[features]
default = []
wasm = [
  "dep:wasm-bindgen",
  "dep:serde-wasm-bindgen",
  "dep:console_error_panic_hook",
]

[dependencies]
jagua-rs = { version = "=0.8.1", features = ["bpp"] }
dxf = "=0.6.1"
geo = "=0.33.1"
serde = { version = "1", features = ["derive"] }
thiserror = "2"
rand = "0.10"

wasm-bindgen = { version = "0.2", optional = true }
serde-wasm-bindgen = { version = "0.6", optional = true }
console_error_panic_hook = { version = "0.1", optional = true }

[target.'cfg(all(target_arch = "wasm32", target_os = "unknown"))'.dependencies]
getrandom = { version = "0.4", default-features = false, features = ["wasm_js"] }
```

`geo`, kaynak dokümanın zorunlu bağımlılığı değil; orijinal poligonların fark/kesişim işlemleriyle bağımsız doğrulanması için bu planın önerisidir. Onun da native ve WASM derlemesi doğrulanacaktır.

`getrandom` ayarı incelenen güncel upstream bağımlılık düzeniyle uyumludur; farklı sürümde ek bir `getrandom` dalı gelirse bu satırın onu da düzelttiği varsayılmayacak. `cargo tree -i getrandom` ile sürüm ağacı incelenecek. Çakışan sürümlerde komutun istediği açık sürüm belirtilecek.

`lib.rs`, geometri ve yerleşim modüllerini host bağımsız sunar. `wasm_api.rs` yalnızca `feature = "wasm"` ve `target_arch = "wasm32"` altında açılır. Çekirdek tiplerde `JsValue`, `window`, DOM veya Worker kavramı bulunmaz.

İlk derleme:

```powershell
cargo check --workspace
cargo test --workspace
cargo check -p groundscape-core --target wasm32-unknown-unknown --features wasm
```

Bu aşamada basit bir WASM export’u oluşturulduktan sonra:

```powershell
Set-Location core
wasm-pack build --target web --dev --out-dir ../frontend/src/wasm --out-name groundscape_core -- --features wasm
Set-Location ../frontend
npm run dev
```

`--target web` seçilir; WASM’ın üretilen JavaScript başlatıcısı bir module Worker içinde çağrılır. İlk başlatma için sırf alışkanlıkla nightly, shared memory, atomics veya Rayon thread-pool eklenmez.

## 7. Aşama A — En küçük uçtan uca teknik doğrulama

**Amaç:** Arayüze yatırım yapmadan Rust → WASM → Worker → React hattının ve temel jagua/DXF bağımlılıklarının gerçekten çalıştığını kanıtlamak.

İlk dikey kesit gerçek ürünlerle başlamaz. Rust içinde sentetik iki veya üç kapalı poligon oluşturulur. Bir adet 5000 × 5000 mm kutu kurulur. Çakışmalı ve çakışmasız birkaç poz denenir. Aynı çağrı native testte ve tarayıcıdaki Worker’da çalıştırılır. Ayrıca küçük, kontrollü bir DXF byte fixture’ı `dxf` üzerinden okunur; DXF bağımlılığının WASM’a uygunluğu sona bırakılmaz.

`jagua-rs` kaynak incelemesi şu API yapısını doğruluyor: `BPInstance`, `Bin`, `BPProblem`, `BPPlacement` ve sürekli dönüşler için `RotationRange::Continuous`. Somut nesne kurulumu `jagua_adapter.rs` içinde toplanır. Uygulamanın diğer bölümleri bu sürüme özgü tipleri bilmez.

Spike kapsamında doğrulanacak durumlar:

| Deney | Beklenen |
|---|---|
| Ayrık iki safety poligonu | Çakışma yok |
| Bir poligon diğerinin tamamen içinde | Çakışma var; yalnızca kenar kesişimi testiyle kaçırılmıyor |
| Bounding box’ları kesişen ama gerçek poligonları ayrık iki konkav şekil | Çakışma yok |
| Alanın dışına çıkan poz | Reddediliyor |
| 37° poz | Geçerli bir sürekli açı olarak değerlendiriliyor |
| 5000 mm’ye tam sığan şekil | Temas politikasıyla uyumlu; örnekleyici paniklemiyor |
| İlk ürün eklendikten sonra ikinci ürün | Aynı açık kutuya ekleniyor |
| Bir ürün sığmıyor | İkinci kutu açılmıyor |

`place_item` çağrısı bir otomatik arama veya kendiliğinden tam çakışma doğrulaması yerine kullanılmayacak. Öncesindeki aday üretimi ve tam poligon çakışma sorgusu uygulamanın sorumluluğudur.

**Çıkış koşulu:** Native test, WASM build ve gerçek Worker çalışması geçiyor; tam sürümler, gerekli özellikler, sınır/temas davranışı ve test çıktıları `docs/spike-results.md` içinde kayıtlı. Bu geçmeden gerçek DXF importer ve ekran ayrıntıları büyütülmez.

## 8. Aşama B — Geometri sözleşmesi

### 8.1. Birimler ve koordinatlar

İş mantığında birim daima milimetredir. Sabit alan `0 ≤ x ≤ 5000`, `0 ≤ y ≤ 5000` aralığıdır. Alanın iş kuralı `area.rs` içindedir; arayüzde bağımsız ikinci bir 5000 hesabı oluşturmak yerine alan bilgisi Worker’dan alınır.

Uygulamanın kaynak geometrisi `f64` koordinatlar taşır. `jagua-rs` geometri tiplerine dönüşüm adaptörün sınırında yapılır. Bu, motorun sayısal hassasiyetinin “tam matematiksel kesinlik” olduğu anlamına gelmez.

Poz sözleşmesi:

```text
Pose:
  xMm: number
  yMm: number
  rotationRad: number
```

Rust, arama ve Worker mesajlarında radyan kullanılır. Derece dönüşümü yalnızca SVG ve kullanıcıya gösterilen metinde yapılır. Veri saklanmadan veya doğrulanmadan önce x/y/açı yuvarlanmaz; yalnızca ekrandaki metin kısaltılır.

### 8.2. Ortak origin

Bir ürünün footprint ve safety zone katmanları aynı DXF koordinat sisteminde bulunmalıdır. Örneğin safety zone bounding box merkezini ortak referans kabul et ve bu merkezi **iki katmandan da aynı şekilde** çıkar. Footprint’i ve safety zone’u birbirinden bağımsız merkezleme; bu ürünün geometrik ilişkisini bozar.

`jagua-rs` `OriginalShape` üzerinde bir `pre_transform` taşıyabilir (K21). Motor içi koordinat değişimi varsa dışarı dönen poz bunu da kapsamalıdır:

```text
T_world = T_solver_placement × T_pre_transform
```

Kendi canonical geometri hazırlığında motorun ön dönüşümü bilinçli olarak identity bırakılabiliyorsa bu daha basittir. Aksi durumda dönüşümler `jagua_adapter.rs` içinde birleştirilir. Render ve bağımsız doğrulama aynı birleşik pozu kullanır.

İlk sürümde safety zone’u küçültebilecek sadeleştirme, offset veya dar girintileri kapatma gibi dönüşümler varsayılan olarak kullanılmaz. Motorun şekil değiştirme ayarları bilinçli biçimde seçilir ve test edilir.

### 8.3. Model

```text
ProductGeometry:
  id
  footprintPolygons[]
  safetyZone
  safetyAreaMm2
  sourceMetadata

Placement:
  productId
  pose

PlacementResult:
  status
  placements[]
  unplaced[]
  stats
  validation
```

Bir ürünün birden fazla footprint parçası bulunabilir; ancak ilk safety sözleşmesi tek, kapalı, deliksiz, basit bir poligondur. Çok parçalı veya delikli safety zone ihtiyacı gerçek fixture’larda çıkarsa sessizce tek dış kontura çevrilmez; sözleşme genişletilir veya fixture kontrollü hazırlanır.

`linearEpsilonMm` ile `areaEpsilonMm2` ayrı kavramlardır. Bunlar sayısal regresyon testinden sonra kayda geçirilerek sabitlenir. Ek güvenlik tamponu, yasal mesafe veya tasarım payı değildir. Uygulama footprint’ten tahmini safety zone üretmez.

**Çıkış koşulu:** Ortak origin, radyan/derece, alan hesabı ve dönüşüm round-trip testleri geçiyor; geometri sözleşmesi yazılı.

## 9. Aşama C — Sabit ürün DXF importer’ı

Üç gerçek dosya geliştirici tarafından `frontend/public/products/` altına konur. `products.ts` yalnızca sabit ürün kimliği, görünen adı, URL ve gerekirse doğrulanmış kaynak birimi bilgisi taşır. Dosya seçme veya yükleme ekranı yapılmaz.

### 9.1. Okuma akışı

Frontend `fetch` ve `arrayBuffer()` kullanır. Worker byte verisini Rust’a iletir. Rust tarafında dosya sistemi API’si değil, `Cursor<&[u8]>` ve `Drawing::load` kullanılır (K16).

Importer sırası:

```text
Byte okuma
→ DXF parse
→ birim denetimi
→ FOOTPRINT / SAFETY_ZONE katman seçimi
→ desteklenen entity'leri kontura dönüştürme
→ geometri geçerliliği
→ ortak origin
→ footprint ⊆ safety zone kontrolü
→ alan / bounding box hesaplama
```

### 9.2. Birim politikası

Başlangıç sözleşmesi `$INSUNITS = 4`, yani milimetredir (K17). `$INSUNITS = 0` veya eksik değer “kesin mm” kabul edilmez. Sabit fixture üreticisi tarafından doğrulanmış birim manifestte açıkça tanımlanabilir; bunun dışında belirsiz dosya reddedilir.

Farklı birim açıkça belirtilmişse kontrollü dönüştürme eklenebilir; ilk MVP için dosyaları önceden mm olarak hazırlamak daha basittir. “Alana sığdırmak için ölçekle” uygulanmaz. Her dosyanın beklenen genişlik/yüksekliği geliştirici tarafından en az bir kez kaynak CAD ölçüsüyle karşılaştırılır.

### 9.3. Desteklenen DXF alt kümesi

İlk hedef, kapalı ve düz kenarlı 2D `LWPOLYLINE` ve uygun 2D `POLYLINE` konturlarıdır. `LWPOLYLINE` içinde `bulge` değerleri sıfır olmalıdır. Diğer katmanlar bilgi notuyla yok sayılabilir; seçilen iki katmandaki desteklenmeyen entity’ler sessizce atlanamaz.

Açık kontur, öz-kesişim, sıfıra yakın alan, sonlu olmayan koordinat, üçten az farklı köşe, yanlış safety parça sayısı veya footprint’in safety dışına çıkması ürün bazlı hata üretir. Küçük tekrar eden ardışık noktalar kontrollü temizlenebilir; büyük boşluklar otomatik birleştirilmez.

`ARC`, `CIRCLE`, eğrili `bulge`, `SPLINE`, `INSERT` ve 3D entity ihtiyaçları gerçek dosyalar üzerinden belirlenir. DXF parser’ın entity okuyabilmesi, uygulamanın onu güvenli biçimde poligona dönüştürdüğü anlamına gelmez.

Eğri zorunluysa iki yol vardır: geliştirici tarafından kontrollü poligonlaştırılmış fixture hazırlamak veya hata sınırı belirlenmiş eğri açma modülü eklemek. Safety zone poligonlaştırması gerçek güvenlik bölgesini küçültemez. Yalnızca “her bir derecede nokta al” gibi gerekçesiz bir yaklaşım kullanılmaz.

Footprint’in safety içinde olması yalnızca köşelerin içeride olmasına bakılarak doğrulanmaz; konkav poligonda kenar dışarı çıkabilir. Poligon farkı/kapsama kontrolü yapılır.

**Çıkış koşulu:** Üç gerçek DXF doğru boyutlarda okunuyor; iki katman doğru ayrılıyor; hatalı fixture’lar açıklayıcı hata veriyor. Gerçek dosyalar gelene kadar yalnızca sentetik fixture testleri tamamlanmış sayılabilir.

## 10. Aşama D — Tek kutulu jagua adaptörü

`jagua_adapter.rs`, aşağıdaki sorumlulukları üstlenir:

1. Bir tane 5000 × 5000 mm container/bin oluşturur; `stock = 1`.
2. Üç ürün için birer adet talep oluşturur; collision geometrisi safety zone’dur.
3. Uygulama ürün kimliği ile jagua item indeksini eşler.
4. Sürekli rotasyon iznini tanımlar.
5. Adayın tam poligon ve alan sınırı çakışma sorgusunu yapar.
6. Geçerli adayı aynı yerleşime ekler; uygulama pozu olarak dışarı çevirir.

`BPInstance` içindeki item kimlikleri/indeksleri kararlı kalır. Büyükten küçüğe sıralama instance vektörünün kimlik eşleşmesini bozarak değil, ayrı bir yerleştirme sırası vektörü üzerinde yapılır.

İlk ürün için kapalı kutu açılır; sonraki ürünler aynı açık layout anahtarı üzerinden eklenir. Mevcut arama uygun yer bulamayınca upstream örnekteki gibi başka kutu açılmaz. Tek kutu kuralı yalnızca debug assertion’a veya stok ayarına bırakılmaz; adaptörde çalışma zamanı kontrolü ve release testi bulunur.

Arama hızlandırıcıları hızlı eleme için kullanılabilir. “Surrogate çakışmadı” bilgisi nihai geçerlilik değildir; tam poligon sorgusu gerekir.

Geri izleme sırasında üç ürün için en basit güvenilir yaklaşım, seçilmiş önceki pozlardan problem durumunu yeniden kurmaktır. Kütüphanede boş layout kapanınca değişebilecek anahtarları yanlışlıkla yeniden kullanmaktan kaçınılır.

**Çıkış koşulu:** Motor ikinci kutu açamıyor; ürün kimlikleri doğru; native ve Worker testlerinde aynı sözleşme çalışıyor.

## 11. Aşama E — Sürekli örneklemeli yerleşim araması

### 11.1. Sıra ve ön kontroller

Ürünler `abs(area(SAFETY_ZONE))` azalan sırada yerleştirilir. Eşitlikte ürün kimliği kullanılır. Yeniden başlatma ve geri izleme bu sırayı değiştirmez.

Safety alanları toplamı 25.000.000 mm²’yi aşıyorsa, bu gerekli alan koşulu nedeniyle tam yerleşim mümkün değildir. Toplam alanın küçük olması ise yerleşimin varlığını kanıtlamaz.

Bir ürünün **döndürülmemiş** bounding box’ının 5000 mm’yi aşması doğrudan ret nedeni değildir. Örneğin 6000 × 500 mm bir dikdörtgen bu kareye çapraz yerleşebilir.

### 11.2. Aday üretimi

Açı uzayı `[0, 2π)` aralığıdır. 0/90/180/270 derece yalnızca faydalı başlangıç adaylarıdır; arama bunlarla sınırlanmaz. Sonraki adaylar seed’li sürekli rastgele açı örnekleri ve yerel açı pertürbasyonları içerir.

Bir açı için döndürülmüş safety poligonunun sınırları hesaplanır:

```text
minX, maxX, minY, maxY

xTranslation ∈ [-minX, 5000 - maxX]
yTranslation ∈ [-minY, 5000 - maxY]
```

Aralık tersse o açı geçersizdir. Alt ve üst sınır aynıysa tek sabit değer kullanılır; sıfır genişlikli aralık rastgele dağılıma verilmez. Sayısal yakınlık konusu kayıtlı geometri politikasıyla ele alınır; alanı büyütme veya ürünü küçültme yapılmaz.

Adayların bir bölümü kenar/köşe hizalı, bir bölümü iç bölgeden örneklenir. Geçerli adaylar çevresinde giderek küçülen x/y/açı pertürbasyonları denenir. Açı değişince boyut sınırları yeniden hesaplanır.

Her aday önce ucuz kontrollerden, sonra tam poligon çakışma sorgusundan geçer. Uygun adayın puanı öncelikle yerleştirilmiş ürün sayısına katkı, sonrasında boş alanı parçalamama/kompaktlık ve bottom-left benzeri tie-break ölçütleriyle değerlendirilir. Hedef şerit genişliği veya uzunluğu optimize etmek değil, aynı kareye üçünü yerleştirmektir.

### 11.3. Geri izleme ve yeniden başlatma

Bir ürün için tek bir ilk uygun pozu saklama. Konum ve açı açısından farklı birkaç iyi aday sakla. Sonraki ürün yerleşmezse önceki ürünün alternatif adayını dene. Alternatifler bittiğinde aynı büyükten küçüğe sırayla yeni seed akışından yeniden başlat.

MVP’de tüm ürün sıralarını permüte etmek gerekmez; bu, kullanıcının sıra kararını da değiştirir. Üç ürün için sınırlı geri izleme yeterince küçük ve denetlenebilir bir başlangıçtır.

İlk deneysel profil:

```text
globalSamplesPerItem = 6000
localSamplesPerItem  = 2000
candidateBufferSize = 6
maxRestarts         = 3
maxTotalCandidates  = 200000
workerStepCandidates = 256
```

Bu sayılar araştırma sonucu kanıtlanmış performans değerleri değil, benchmark için başlangıç önerileridir. Gerçek fixture’lar ve kullanıcının bilgisayarı üzerinde ölçülüp ayarlanır. Global üst sınır geri izleme ve tüm yeniden başlatmaları da kapsar.

Aday sayacı yalnızca pahalı çakışma testlerini değil, sınır nedeniyle erken elenen denemeleri de kapsar. Böylece Worker adımı kötü bir geometride sınırsız dönmez. Üretim araması süre yerine aday bütçesiyle sınırlandığında aynı build ve seed için daha tekrarlanabilir olur. Farklı platformlarda bit düzeyinde aynı kayan nokta sonuçları garanti edilmez.

### 11.4. Sonuç durumları

```text
COMPLETE          Üç ürün de yerleşti ve son doğrulama geçti.
PARTIAL           Bazı ürünler yerleşti; kalanlar ürün bazında bildiriliyor.
NO_SOLUTION_FOUND Verilen arama bütçesinde tam çözüm bulunamadı.
INVALID_INPUT     Geometri, birim veya DXF sözleşmesi geçersiz.
INTERNAL_ERROR    Beklenmeyen motor/entegrasyon hatası.
```

Gerekli alan koşulu gibi ispatlanabilen bir ret ile örnekleme bütçesinin tükenmesi farklı reason code taşır. Ekran “bu denemede yer bulunamadı” der; örneklemeli arama başarısızlığını “bu ürünler kesinlikle sığmaz” diye sunmaz.

Kısmi sonuç kullanılabilir ancak MVP’nin gerçek üç ürün için başarı kriterini karşılamaz. Tek bir ürün bile sessizce atlanamaz.

**Çıkış koşulu:** Sıra testi, keyfi açı testi, geri izleme testi, bütçe testi ve tek kutu testi geçiyor. Gerçek ürünlerde üçlü başarılı sonuç bağımsız doğrulanıyor.

## 12. Aşama F — Bağımsız son doğrulama

`validation.rs`, motorun döndürdüğü uygulama pozlarını **orijinal canonical f64 geometriye** uygular. Motorun sadeleştirilmiş/surrogate geometrisini nihai doğrulama verisi olarak kullanmaz.

Doğrulama ölçütleri:

| Kural | Kontrol |
|---|---|
| Tamlık | Beklenen üç ürün kimliği tam birer kez mevcut |
| Tek alan | Bütün yerleşimler aynı sabit alan için |
| Poz geçerliliği | Sonlu x/y/açı; yalnızca rijit taşıma ve döndürme |
| Alan içinde kalma | Footprint ve safety zone, kayıtlı sayısal politika içinde alan sınırlarında |
| Zone çakışması | Her safety çifti için iç bölge kesişimi yok |
| Katman ilişkisi | Footprint, ilgili safety zone içinde |
| Ölçek | İthalattaki mm boyutları değişmemiş |
| Dönüşüm | Render’a dönen poz ile kontrol edilen poz aynı |

`geo` poligon farkı/kesişimi ve alan işlemleri için önerilen yardımcıdır (K18). Önce poligon geçerliliği kontrol edilir; geçersiz geometri üzerinde Boolean işlemlerine güvenilmez. Bu bağımsız kontrol de kayan nokta hesabıdır; sınırsız matematiksel kesinlik iddiası taşımaz.

Doğrulama başarısızsa aday sonuç kullanıcıya “başarılı” diye gönderilmez. Bütçe varsa arama devam eder; yoksa açık hata veya geçersiz sonuç durumu döner. Hata SVG’de clip/mask kullanılarak gizlenmez.

**Çıkış koşulu:** Özellikle tam kapsama, konkav şekil, tam sınır teması, motor ön dönüşümü ve yanlışlıkla iki kez transform uygulama regresyonları geçiyor.

## 13. Aşama G — WASM API ve Worker

### 13.1. API sınırı

Rust domain katmanı host bağımsız kalır. WASM katmanı byte ve DTO dönüştürme, hata serileştirme ve motor oturumunun yaşam döngüsünü yönetir.

Önerilen uygulama API’si — bunlar upstream hazır API isimleri değildir:

```text
load_products(inputs)        → geometriler + alan bilgisi
start_placement(config)      → arama oturumu
step_placement(maxAttempts)  → ilerleme veya sonuç
reset_placement()            → oturum temizliği
```

Native testler aynı arama oturumunu sonuçlanana kadar adımlayabilir. Tarayıcıdaki Worker her adım arasında mesaj döngüsüne fırsat verir.

Serde alan adları TypeScript ile uyuşacak şekilde `camelCase` yapılır. Hatalar serbest metinden ibaret değil, `code`, `phase`, `productId` ve açıklama alanları taşır.

### 13.2. Mesaj sözleşmesi

Her mesaj `schemaVersion: 1` ve `requestId` taşır.

| Yön | Mesaj | İçerik |
|---|---|---|
| UI → Worker | `INIT` | Ürün kimlikleri, DXF byte buffer’ları, doğrulanmış metadata |
| Worker → UI | `READY` | Alan, canonical geometriler, import özeti |
| UI → Worker | `PLACE` | Seed ve arama ayarları |
| Worker → UI | `PROGRESS` | Aşama, aday sayısı, yerleşen ürün sayısı |
| Worker → UI | `RESULT` | Pozlar, durum, yerleşmeyen ürünler, doğrulama özeti |
| Worker → UI | `ERROR` | Ürün/faz/hata kodu |
| UI → Worker | `CANCEL` | Adımlı arama sırasında iptal isteği |

Sadece işlem hacminden tahmin edilen yanıltıcı bir “başarı yüzdesi” gösterilmez. Aday bütçesi ilerlemesi, çözüm bulunma olasılığı olarak sunulmaz.

### 13.3. Worker oluşturma ve WASM yükleme

`src/App.tsx` veya aynı seviyedeki bir factory içinde:

```typescript
const worker = new Worker(
  new URL("./workers/placement.worker.ts", import.meta.url),
  { type: "module" }
);
```

Vite yapılandırmasında üretim Worker formatı `es` olacak şekilde ayarlanır. `wasm-pack --target web` çıktısının JavaScript başlatıcısı Worker’da import edilir. WASM asset adresi Vite’nin `?url` importuyla çözülebilir. `init` çağrısının imzasında üretilen `.d.ts` esas alınır; internetten eski başlatıcı imzası körlemesine kopyalanmaz.

WASM hazır olmadan `READY` gönderilmez. Hata durumunda UI bekleme ekranında takılmaz.

### 13.4. Donma ve iptal

Tek bir uzun, senkron WASM çağrısı Worker’ı meşgul ederken Worker’ın gelen `CANCEL` mesajını okuyacağı varsayılmaz. Arama, örneğin 256 adaylık `step` çağrılarına bölünür; aralarda `setTimeout(..., 0)` gibi bir macrotask sınırıyla mesajlar işlenir. Yalnızca sonsuz `Promise.resolve()` zinciri kullanmak yeterli tasarım değildir.

Ana thread gerektiğinde `worker.terminate()` ile sert iptal yapabilir; bu Worker’ı hemen sonlandırır (K20). Sonrasında yeni Worker oluşturulur ve statik DXF’ler yeniden yüklenir. WASM nesne handle’ları reset/unmount sırasında serbest bırakılır.

### 13.5. Veri ve yaşam döngüsü

Frontend `response.ok` kontrolünden sonra ArrayBuffer alır. Buffer transfer edilirse ana thread’deki kopyanın detached olacağı dikkate alınır. Yeniden başlatmada yeniden fetch veya bilinçli tutulan bir kopya kullanılır.

React StrictMode, HMR, unmount, hızlı yeniden tıklama ve eski istekten geç gelen mesajlar test edilir. `requestId` uyuşmayan mesajlar uygulanmaz. Effect cleanup içinde Worker sonlandırılır; devam eden fetch çağrıları AbortController ile iptal edilir.

**Çıkış koşulu:** UI arama sırasında yanıt veriyor; iptal/yeniden başlatma bozuk durum bırakmıyor; Worker sonucu orijinal geometri doğrulamasından geçmiş.

## 14. Aşama H — Minimal React ve SVG ekranı

Ekran yalnızca alan, üç ürünün durum bilgisi, “Yerleştir” düğmesi ve anlaşılır sonuç/hata mesajından oluşur. React state/reducer yeterlidir; başlangıç için büyük bir global state altyapısı gerekmez.

Durum makinesi:

```text
loading → ready → placing → complete
                        ↘ partial
                        ↘ error
```

Yükleme sırasında ve aktif aramada Yerleştir düğmesi devre dışıdır. Hata durumunda neden gösterilir; yeni denemenin koşulları nettir.

### 14.1. Render sözleşmesi: çift dönüşüm yok

Kaynak MVP metni hem transform uygulanmış geometri hem de SVG group transform’dan söz ediyor. Bu iki yöntem aynı anda uygulanırsa ürün iki kez taşınır/döner.

Bu planın tercihi:

```text
READY  → Yerel/canonical footprint ve safety geometrileri
RESULT → Yalnızca bu geometrilerin uygulanacak pozları
SVG    → Pozu bir kez uygular
```

Dünya koordinatlı geometri debug amacıyla ayrıca verilirse yeniden transform edilmez.

### 14.2. SVG koordinat sistemi

`viewBox="0 0 5000 5000"` kullanılır. Aspect ratio korunur; alan ekranda kare kalır. Matematikte +Y yukarı olacaksa dış grup:

```xml
<g transform="translate(0 5000) scale(1 -1)">
  <!-- product groups -->
</g>
```

Ürün grubu:

```text
translate(xMm yMm) rotate(rotationRad × 180 / π)
```

Footprint ve safety aynı ürün grubunun içindedir. Safety yarı saydam/kesikli sınırla, footprint belirgin yüzeyle gösterilebilir. Metinler Y ters çevrilmesinden etkilenmemesi için ayrı UI bölümünde tutulabilir.

5000 mm modelde 5000 SVG birimidir. Bu, monitörde fiziksel olarak 1:1 milimetre gösterildiği anlamına gelmez. Örneğin 1000 mm genişlik, ekran boyutundan bağımsız olarak alan genişliğinin beşte biridir.

SVG clip/mask dışarı taşan bir ürünü gizleyerek geçerliymiş gibi göstermeyecek. Görsel düzgünlük geometrik doğrulamanın yerine geçmez.

**Çıkış koşulu:** Üç ürün aynı poz/geometri sözleşmesiyle çiziliyor, ölçek oranları korunuyor, eksik ürün veya hata sessizce gizlenmiyor.

## 15. Test planı

Test fixture’ları iki gruptur: geliştirici tarafından küçük ve kontrollü üretilmiş sentetik testler ile gerçek üç ürün. Sentetik testlerden geçmek gerçek ürünlerin hazır olduğu anlamına gelmez.

| Test grubu | Örnekler | Araç |
|---|---|---|
| Geometri | Alan, yön, ortak origin, 37° dönüş, dönüşüm geri kontrolü | Rust unit test |
| Geçersiz input | Açık/öz-kesişen kontur, eksik layer, yanlış/eksik birim, desteklenmeyen entity | Rust importer test |
| Çakışma | Konkav şekiller, tam kapsama, sadece bbox kesişimi, footprint küçükken safety çakışması | Rust + jagua adaptör testi |
| Sınır | 5000 mm tam sığma, 5000.1 mm taşma, köşe/kenar teması, sıfır aralık | Native ve WASM test |
| Rotasyon | 0/90 ile sığmayan, eğik açıyla sığan 6000 × 500 mm fixture | Arama regresyonu |
| Sıralama | Alan azalan sıra; eşit alanda kararlı ürün kimliği; item-index eşlemesi | Rust unit test |
| Arama | Geri izleme, seed tekrarı, aday bütçesi, sıfır bütçe, kısmi sonuç | Rust integration |
| Tamlık | Üç ürün, bir kutu, her ürün bir kez; release modunda ikinci kutu yok | Rust integration |
| Worker | Gerçek WASM yükleme, progress, cancel/restart, eski requestId, handle cleanup | Browser integration |
| Arayüz | Loading/ready/error, düğme kilidi, sonuç mesajı, SVG transform | Vitest / Testing Library |
| E2E | Gerçek üretim build’i, gerçek Worker ve WASM, üç DXF, üç ürün | Playwright |
| Dağıtım | Base path, DXF/WASM 404, doğru asset URL’leri, statik çalışma | Preview / Playwright |

6000 × 500 mm eğik dikdörtgen, sadece dört açıyla çalışan sahte bir “serbest rotasyon” implementasyonunu yakalamak için özellikle yararlıdır.

Gerçek fixture regresyonunda önce bir kayıtlı seed ile beklenen başarılı sonuç oluşturulur; sonrasında örneğin 20 sabit seed ile başarı oranı ve aday sayıları raporlanır. Bütün seed’lerde başarı bir hedef olabilir, ancak ölçülmeden iddia edilmez. Farklı tarayıcı/platformlarda aynılık, gerektiğinde geometrik toleranslarla değerlendirilir.

Yerleşim 100 kez art arda çalıştırılarak eski sonuçların birikmediği, Worker sayısının artmadığı ve serbest bırakılmamış WASM handle’ları bulunmadığı kontrol edilir. Performans hedefleri gerçek bilgisayarda ölçüm sonrasında yazılır; ölçülmemiş saniye garantisi verilmez.

## 16. Geliştirme ve build komutları

Frontend `package.json` içine, diğer mevcut alanları koruyarak önerilen script’ler:

```json
{
  "scripts": {
    "wasm:dev": "cd ../core && wasm-pack build --target web --dev --out-dir ../frontend/src/wasm --out-name groundscape_core -- --features wasm --locked",
    "wasm:build": "cd ../core && wasm-pack build --target web --release --out-dir ../frontend/src/wasm --out-name groundscape_core -- --features wasm --locked",
    "dev": "vite",
    "typecheck": "tsc -b",
    "test": "vitest run",
    "test:e2e": "playwright test",
    "build": "npm run wasm:build && tsc -b && vite build",
    "preview": "vite preview"
  }
}
```

`--locked` kullanan bu script’lerden önce başlangıç `Cargo.lock` dosyası oluşturulmuş ve doğrulanmış olmalıdır.

Günlük akış, repo kökünden:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

Set-Location frontend
npm run wasm:dev
npm run dev
```

Rust kodu değişince `npm run dev` kendiliğinden yeniden WASM üretmez. `wasm:dev` yeniden çalıştırılır. Otomatik Rust watch süreci daha sonra eklenebilir; ilk MVP’nin zorunlu bağımlılığı değildir.

Üretim doğrulaması:

```powershell
cargo test --workspace --release

Set-Location frontend
npm ci
npm run test
npm run build
npm run preview
```

E2E komutu Playwright yapılandırmasındaki `webServer` üzerinden preview’ı başlatabilir; bu durumda ayrı terminalde ikinci preview açılmaz.

Vite TypeScript’i dönüştürse de bu tek başına typecheck değildir. `tsc -b` build kapısında kalır. Son kullanıcıya `frontend/dist/` dağıtılır. `index.html` dosyasına çift tıklamak yerine HTTP üzerinden statik sunum kullanılır.

## 17. CI ve dağıtım

CI, aynı kilitli Node/Rust/wasm-pack sürümlerini kullanır. Önerilen sıra:

```text
Checkout
→ Rust / Node / wasm-pack kurulumu
→ cargo fmt + clippy
→ cargo test (native, gerekli kritik testler release)
→ wasm target kontrolü
→ npm ci
→ Vitest
→ release WASM + TypeScript + Vite build
→ desteklenen ortamda Playwright
→ dist artifact
```

Desteklenen Linux CI üzerinde tarayıcı ve sistem bağımlılıkları `npx playwright install --with-deps chromium` ile hazırlanabilir (K19).

Playwright gerçek release WASM’ı çalıştırır; yalnızca mock Worker ile yeşil test yeterli değildir. `frontend/src/wasm/` Git’te olmadığı için temiz bir checkout’tan üretim build’inin geçmesi ayrıca kontrol edilir.

Statik hosting DXF ve WASM dosyalarını sunabilmelidir. Alt dizinde yayınlanacaksa Vite `base` ve `import.meta.env.BASE_URL` ile ürün URL’leri uyumlu kurulur; kök `/products/...` adresine körlemesine bağlanılmaz.

Bu seri Worker tasarımında sırf WASM kullanılıyor diye `SharedArrayBuffer`, COOP/COEP veya özel thread header’ları kurulmaz. Çok iş parçacıklı WASM ileride eklenirse ayrı bir tasarım kararı olur.

Sunucuya geometri gönderen bir solver isteği yoktur. Ancak statik `public/products` içeriği, uygulamaya erişebilen biri tarafından indirilebilir. “Hesaplama yerel” demek “ürün çizimleri gizlidir” demek değildir.

Üçüncü taraf paket ve lisans bilgileri kayıt altına alınır. Özellikle upstream’den kod veya yama taşınırsa kaynak dosya/lisans kayıtları korunur ve dağıtım incelemesine dahil edilir.

## 18. İş paketleri ve bağımlılık sırası

| Paket | Teslim | Bağımlılık | Tamamlanma kapısı |
|---|---|---|---|
| P0 | Bilgisayar kurulumu ve repo iskeleti | Yok | Native Rust + Vite çalışıyor |
| P1 | Worker/WASM ve jagua/DXF teknik spike | P0 | Native ve tarayıcı testleri; gerçek sürüm kaydı |
| P2 | Model, ortak origin ve geometri sözleşmesi | P1 | Dönüşüm ve geometri testleri |
| P3 | DXF importer ve gerçek fixture hazırlığı | P2 | Katmanlar/birimler/boyutlar doğrulanmış |
| P4 | Tek kutulu jagua adaptörü | P1, P2 | Tam çakışma sorgusu; ikinci kutu açılamıyor |
| P5 | Sürekli örnekleme + büyükten küçüğe + geri izleme | P3, P4 | Bütçeli ve test edilmiş yerleşim |
| P6 | Orijinal geometride bağımsız son doğrulama | P2, P5 | Geçersiz sonuç COMPLETE olamıyor |
| P7 | Worker protokolü, progress, iptal ve yaşam döngüsü | P1, P5, P6 | UI donmuyor; tekrar çalıştırma güvenilir |
| P8 | Minimal React/SVG ekranı | P3, P7 | Ortak transform; üç ürün durumu |
| P9 | Gerçek DXF E2E, CI ve statik dağıtım | P8 | Temiz checkout’tan çalışan üretim build’i |

Bağımsız validator’ın temel testleri P2 ile birlikte yazılır; P6 motor sonucu ile entegrasyon kapısıdır. Böylece doğrulama bütünüyle sona ertelenmez.

Her paket ayrı, incelenebilir bir değişiklik olarak tamamlanabilir. Her paketin açıklamasında değişen dosyalar, çalıştırılan komutlar, geçen/başarısız testler ve bilinen sınırlamalar bulunmalı. Testi çalıştırmadan “tamamlandı” yazılmamalı.

## 19. Hata sözlüğü

| Kod | Kullanıcıya gösterilecek anlam |
|---|---|
| `PRODUCT_FETCH_FAILED` | Ürün dosyası yüklenemedi |
| `DXF_PARSE_FAILED` | Dosya DXF olarak okunamadı |
| `UNIT_UNSPECIFIED` | Çizim birimi doğrulanamadı |
| `UNIT_NOT_SUPPORTED` | Çizim birimi beklenen sözleşmeye uymuyor |
| `MISSING_LAYER` | FOOTPRINT veya SAFETY_ZONE katmanı eksik |
| `UNSUPPORTED_ENTITY` | Seçili katmanda henüz desteklenmeyen çizim öğesi var |
| `OPEN_CONTOUR` | Ürün konturu kapalı değil |
| `INVALID_POLYGON` | Poligon geometrisi geçersiz |
| `FOOTPRINT_OUTSIDE_SAFETY` | Ürün gövdesi tanımlı güvenlik alanının dışına çıkıyor |
| `NECESSARY_AREA_EXCEEDED` | Güvenlik alanları toplamı kullanılabilir alanı aşıyor |
| `SEARCH_BUDGET_EXHAUSTED` | Bu arama bütçesinde tam yerleşim bulunamadı |
| `FINAL_VALIDATION_FAILED` | Bulunan sonuç geometri kontrolünden geçmedi |
| `WASM_INIT_FAILED` | Yerleşim motoru başlatılamadı |
| `WORKER_FAILED` | Hesaplama işlemi beklenmedik şekilde kesildi |

İlgili hatalarda ürün kimliği belirtilir. Teknik ayrıntı geliştirici logunda tutulur; arayüzde ham stack trace yerine anlaşılır açıklama gösterilir.

## 20. MVP’nin tamamlanma kontrolü

Gerçek üç DXF’in iki katmanı doğru okunmuş, mm ölçüleri doğrulanmış ve ortak geometrik ilişkileri korunmuş olmalı. Tek bir 5000 × 5000 mm alana, safety zone alanına göre büyükten küçüğe sırada, sürekli örneklemeli açı aramasıyla üç ürün yerleştirilmeli.

Aynı sonuç orijinal geometri üzerinde sınır, çakışma, tamlık ve ölçek kontrollerini geçmeli. SVG aynı pozları yalnızca bir kez uygulamalı. UI Worker hesabı sırasında yanıt vermeli; arka arkaya denemeler bozuk oturum veya yanlış eski sonuç üretmemeli.

Temiz checkout’tan native testler, WASM build, TypeScript kontrolü, frontend testleri ve üretim E2E testi geçmeli. `dist/` statik sunulduğunda backend gerektirmeden çalışmalı.

**İlk kodlama teslimi:** Ürün yönetimi ekranı değil; tek kutu içinde sentetik poligonlara native ve Worker üzerinden aynı çakışma sorgusunu yapan, küçük DXF byte fixture’ını okuyabilen ve sonucu sayfada gösteren teknik spike.

**Gerçek üç DXF gelmeden “MVP tamamlandı” denemez.** Bu dosyalar olmadan iskelet, sentetik testler ve motor entegrasyonu tamamlanabilir; ürün uyumluluğu ve gerçek yerleşim kabulü açık kalır.

## 21. Kaynaklar ve doğrulama sınırı

Aşağıdaki bağlantılar 10 Eylül 2026 tarihli plan için incelenen birincil kaynaklardır. Hareketli `main` bağlantıları implementasyon sırasında commit SHA ile kayıt altına alınmalıdır. Bu metindeki arama bütçeleri, dosya ayrımı, API adları ve kabul kapıları Groundscape için öneridir; upstream’in hazır sunduğu özellikler olarak yorumlanmamalıdır.

**K0 — Kullanıcı dokümanı:** “GROUNDSCAPE — MVP-1: YERLEŞİM MOTORU”; yüklenen `Yapıştırılan markdown(1).md`. Özellikle hedef/mimari, veri akışı, açık kararlar ve kapsam dışı bölümleri.

**K1 — sparrow-studio mimari:**  
`https://github.com/JeroenGar/sparrow-studio`

**K2 — sparrow-studio güncel web build düzeni:**  
`https://github.com/JeroenGar/sparrow-studio/blob/main/web/README.md`

**K3 — jagua-rs kapsam ve örnek LBF:**  
`https://github.com/JeroenGar/jagua-rs`

**K4 — jagua sürüm/özellikleri:**  
`https://github.com/JeroenGar/jagua-rs/blob/main/jagua-rs/Cargo.toml`  
Not: Repo içindeki crate konumu hareketli sürümde değişebilir; uygulama için crates.io 0.8.1 ve Cargo.lock esas alınır.

**K5 — LBF tek/çok kutu davranışı:**  
`https://github.com/JeroenGar/jagua-rs/blob/main/lbf/src/opt/lbf_bpp.rs`

**K6 — LBF arama ve çap sıralaması:**  
`https://github.com/JeroenGar/jagua-rs/blob/main/lbf/src/opt/search.rs`

**K7 — Sürekli rotasyon örnekleme:**  
`https://github.com/JeroenGar/jagua-rs/blob/main/lbf/src/samplers/rotation_distr.rs`

**K8 — Örnekleme aralıkları:**  
`https://github.com/JeroenGar/jagua-rs/blob/main/lbf/src/samplers/uniform_rect_sampler.rs`

**K9 — sparrow problem türü / Cargo:**  
`https://github.com/JeroenGar/sparrow`  
`https://github.com/JeroenGar/sparrow/blob/main/Cargo.toml`

**K10 — Node resmi sürümleri:**  
`https://nodejs.org/en/about/previous-releases`  
`https://nodejs.org/download/release/latest-v24.x/`

**K11 — Rust resmi kurulum ve MSVC:**  
`https://www.rust-lang.org/tools/install`  
`https://rust-lang.github.io/rustup/installation/windows-msvc.html`

**K12 — Git Windows kurulumu:**  
`https://git-scm.com/install/windows`

**K13 — Vite:**  
`https://vite.dev/guide/`  
`https://vite.dev/guide/features.html`

**K14 — wasm-pack:**  
`https://rustwasm.github.io/wasm-pack/book/commands/build.html`  
`https://rustwasm.github.io/wasm-pack/installer/`

**K15 — wasm-bindgen web başlatma örneği:**  
`https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html`

**K16 — DXF Drawing API:**  
`https://docs.rs/dxf/0.6.1/dxf/struct.Drawing.html`

**K17 — DXF birimleri:** Autodesk AutoCAD resmi `$INSUNITS` referansı; mm kodu 4, unspecified kodu 0.

**K18 — geo BooleanOps:**  
`https://docs.rs/geo/0.33.1/geo/algorithm/bool_ops/trait.BooleanOps.html`

**K19 — Test araçları:**  
`https://playwright.dev/docs/intro`  
`https://vitest.dev/guide/`

**K20 — Worker sonlandırma:**  
`https://developer.mozilla.org/en-US/docs/Web/API/Worker/terminate`

**K21 — jagua kaynak geometri ve ön dönüşüm:**  
`https://github.com/JeroenGar/jagua-rs/blob/main/jagua-rs/src/geometry/original_shape.rs`

**Doğrulama sınırı:** Referans kaynak okuması yapılmıştır. Bu plan kapsamında repo derlenmemiş, örnek uygulama oluşturulmamış ve gerçek üç DXF üzerinde yerleşim çalıştırılmamıştır.
