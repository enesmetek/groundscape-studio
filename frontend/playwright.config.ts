import { defineConfig } from '@playwright/test'

// Plan §15/§17: Playwright gerçek release WASM'ı çalıştırır; webServer önce
// üretim build'ini yapar, sonra preview sunar (temiz WASM çıktısıyla).
export default defineConfig({
  testDir: './tests',
  webServer: {
    command: 'npm run build && npx vite preview --port 4173 --host 127.0.0.1',
    url: 'http://127.0.0.1:4173',
    reuseExistingServer: false,
    timeout: 180_000,
  },
  use: { baseURL: 'http://127.0.0.1:4173' },
})
