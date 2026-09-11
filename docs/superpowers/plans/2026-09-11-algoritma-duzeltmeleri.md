# Algoritma Duzeltmeleri Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Arama butcesi, Worker dilimleri, girdi dogrulama ve skor bilesenlerindeki dogruluk hatalarini en kucuk degisikliklerle gidermek.

**Architecture:** DFS durum makinesi eklemek yerine Engine her adimda ayni seed ile kumeletif butceyi deterministik yeniden oynatir; bu, dilim boyutundan bagimsiz sonucu korurken Worker iptal noktalarini muhafaza eder. Objective ve poligonlar sinirda dogrulanir; spacing mevcut kare alan vekiliyle footprint boslugunu olcer. Grid dagilimi mevcut plan sozlesmesindeki merkez+alan vekili olarak kalir ve yaniltici “gercek footprint kesişimi” iddiasi kullanilmaz.

**Tech Stack:** Rust, jagua-rs, wasm-bindgen, React/TypeScript, Vitest, Playwright.

---

### Task 1: Arama Butcesi ve Dilim Determinizmi

**Files:**
- Modify: `core/src/search.rs`
- Modify: `core/src/engine.rs`
- Test: `core/tests/engine_tests.rs`

- [ ] `step_placement(1)` ile tek urunun sonunda COMPLETE oldugunu gosteren failing test yaz.
- [ ] Ayni toplam butcenin 1/7/256 dilimleriyle ayni sonucu verdigini gosteren failing test yaz.
- [ ] Aday uretiminde sayilan sorguyu yerlesim dongusunde ikinci kez saymayi kaldir.
- [ ] Engine adimlarini ayni seed + kumeletif butce ile deterministik yeniden oynat; seed'i aday sayisiyla degistirme.
- [ ] Yerel havuzun son buffer slotunu doldurmasina izin ver.
- [ ] `cargo test -p groundscape-core --test engine_tests` ile green dogrula.

### Task 2: Objective ve Geometri Girdi Dogrulamasi

**Files:**
- Modify: `core/src/scoring.rs`
- Modify: `core/src/search.rs`
- Modify: `core/src/geometry.rs`
- Modify: `core/src/validation.rs`
- Modify: `core/src/dxf_import.rs`
- Modify: `core/src/jagua_adapter.rs`
- Modify: `core/src/wasm_api.rs`
- Test: `core/tests/scoring_tests.rs`
- Test: `core/tests/dxf_import_tests.rs`
- Test: `core/tests/engine_tests.rs`

- [ ] `grid > 3`, NaN/negatif agirlik ve gecersiz oranlar icin failing objective testleri yaz.
- [ ] Self-intersecting DXF ve bos urun oturumu icin failing testler yaz.
- [ ] `LayoutObjective::validate` ekle; search ve WASM sinirinda gecersiz girdiyi reddet.
- [ ] Basit-poligon denetimini `geometry.rs` icinde ortaklastir; import ve validation kullansin.
- [ ] Jagua shape kurulumunda unwrap yerine `ImportError` yay.
- [ ] Engine `start_placement` sonucunu `Result` yapip bos urunu kontrollu reddet.

### Task 3: Skor Dogrulugu

**Files:**
- Modify: `core/src/scoring.rs`
- Modify: `core/src/search.rs`
- Test: `core/tests/scoring_tests.rs`

- [ ] Esit fiziksel bosluklu farkli boy urunlerin ayni spacing cezasi aldigini gosteren failing test yaz.
- [ ] Anchor ve spacing maliyetlerinin eleman/cift sayisina gore ortalandigini gosteren failing test yaz.
- [ ] Merkez mesafesinden kare-footprint yari caplarini cikararak fiziksel boslugu olc.
- [ ] Anchor maliyetini anchor sayisina, spacing maliyetini cift sayisina bol.
- [ ] Sirlama baseline'ini comparator disinda bir kez hesapla; ayni baseline tum adaylarda iptal oldugu icin yalnizca adayli toplam skoru karsilastir.

### Task 4: Protokol ve Benchmark Tutarliligi

**Files:**
- Modify: `frontend/src/types/protocol.ts`
- Modify: `frontend/src/hooks/usePlacementWorker.ts`
- Modify: `frontend/src/workers/placement.worker.ts`
- Modify: `frontend/src/components/PlacementCanvas.tsx`
- Modify: `frontend/src/App.test.tsx`
- Modify: `frontend/tests/placement.spec.ts`
- Modify: `core/examples/mvp1_score.rs`

- [ ] INIT DTO'suna `placementRole`, `tags`, `ageGroup` ekleyip unsafe cast'i kaldir.
- [ ] Hook'tan bu alanlari Worker/WASM'e aynen aktar.
- [ ] Canvas urun gruplarina test edilebilir product/role data attribute'lari ekle.
- [ ] Unit/E2E'de anchor kenarligi ve rol metadata akisini dogrula.
- [ ] `mvp1_score` fixture'ini `faz1_bench` ile ayni asimetrik centroid hesabina getir; `key_clone` helper'ini sil.

### Task 5: Tam Dogrulama

**Files:**
- Modify: `docs/mvp2-faz1-sonuclari.md`

- [ ] `cargo fmt --all -- --check` calistir.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` calistir.
- [ ] `cargo test --workspace` calistir.
- [ ] `npm run wasm:dev`, `npm run typecheck`, `npm run test:ui`, `npm run build`, `npm run test:e2e` calistir.
- [ ] Sonuc dokumanina duzeltme ve residual riskleri kaydet.
- [ ] Tek commit olusturup `mvp2-faz1` dalina push et.
