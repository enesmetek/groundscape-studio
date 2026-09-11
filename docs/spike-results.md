# Spike sonuçları — Aşama A (10 Eylül 2026)

## Doğrulanan sürümler

| Bileşen | Sürüm |
|---|---|
| rustc/cargo | 1.98.1 (stable-x86_64-pc-windows-gnu) |
| jagua-rs | 0.8.1 (crates.io, `bpp` feature) |
| dxf | 0.6.1 |
| geo | 0.33.1 |
| wasm-pack | 0.15.0 (önceden derlenmiş binary) |
| Node | 24.13.0 |
| wasm-bindgen | 0.2.128 |

## Host toolchain kararı

VS C++ iş yükü kullanıcı kararıyla kurulmadı; `x86_64-pc-windows-gnu` host kullanılıyor.
`rustup` gnu toolchain'i `dlltool.exe` ile gelmediği için MinGW binutils gerekti.
WinLibs GCC 16.1.0 (r4) zip'inden `cc1.exe` güvenlik yazılımınca sessizce silindi
(0xC0000402 çökmesi, dosya klasörde yok); **GCC 14.2.0 (msvcrt r3) çalışıyor**:
`C:\dev\winlibs14\bin` PATH'e ekli (user scope). Cargo/komutlar bunu gerektirir.

## Kanıt edilen davranışlar (native + tarayıcı)

- Sabit 5000×5000 jagua `Bin`; `area_size` ve `bins_used` `BPProblem` durumundan türetilir (hardcode değil).
- Ayrık poligonlar çakışmaz; kapsama çakışır; bbox kesişen konkav şekiller ayrık çıkabilir.
- `candidate_fits`: yerleştirme ÖNCE tam poligon çakışma sorgusu (bbox + `collect_poly_collisions`, yalnız `Exterior` kabul).
- `RotationRange::Continuous` + 37° jagua placement'ı geçerli.
- Tam 5000 sınır teması kabul; alan dışı poz reddedilir; örnekleyici paniklemiyor.
- Sığmayan öğe reddedilir; ikinci kutu açılmaz (`layouts.len() == 1`).
- Kontrolü DXF byte fixture: `$INSUNITS = 4` (mm), kapalı 4 köşeli `LWPOLYLINE` `Drawing::load(Cursor<&[u8]>)` ile okunur.
- Worker: `wasm-pack --target web` çıktısı module Worker'da init edilir; React `new URL(..., import.meta.url)` ile Worker kurar.
- UI "passed" başlığı yalnızca tüm kanıt boolean'ları + `bins_used == 1` + `[5000, 5000]` iken görünür; Playwright gerçek build'de tüm değerleri assert eder.

## Komutlar (PATH: `C:\dev\winlibs14\bin;~\.cargo\bin`)

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                  # 8 native test
cargo check -p groundscape-core --target wasm32-unknown-unknown --features wasm
npm run wasm:dev && npm run typecheck && npm run build && npm run test   # Playwright 1 passed
```

## Bilinen sınırlar

- Gerçek üç ürün DXF'i yok; sadece sentetik poligonlar + kontrollü byte fixture test edildi.
- Arama algoritması, importer sözleşmesi, protokol/iptal ve bağımsız doğrulama sonraki aşamalarda.
- Release WASM ~1.2 MB (sıkıştırılmamış); production'da tekrar ölçülecek.
- `getrandom 0.4.3`: `wasm_js` feature backend'i otomatik seçer; RUSTFLAGS cfg gerekmez.
- `dxf`'in uuid bağımlılığı için `uuid = { features = ["js"] }` feature-unification ile wasm32'de çözüldü.
