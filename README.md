# GROUNDSCAPE

MVP-1: yerleşim motoru. Rust/WASM çekirdeği (`core/`), Web Worker üzerinden React/Vite arayüzü (`frontend/`).

Plan: `docs/implementation-plans/mvp1-implementasyon-plani.md`

## Günlük akış

```powershell
# Rust testleri
cargo test --workspace

# Frontend geliştirme (Rust değişince wasm:dev tekrar çalıştırılır)
cd frontend
npm run wasm:dev
npm run dev
```

## Araç gereksinimleri

- Node 24 LTS
- Rust stable (`x86_64-pc-windows-gnu` host; MSVC C++ iş yükü gerekmez)
- `wasm-pack 0.15.0`
- `wasm32-unknown-unknown` hedefi (`rustup target add wasm32-unknown-unknown`)
