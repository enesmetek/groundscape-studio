// E2E — plan §15: gerçek üretim build'i, gerçek Worker ve WASM, üç sabit DXF.

import { expect, test } from '@playwright/test'

test('üç ürün yüklenir, yerleştirilir ve SVG ile çizilir', async ({ page }) => {
  test.setTimeout(180_000)
  await page.addInitScript(() => {
    const originalPostMessage = Worker.prototype.postMessage
    const state = window as typeof window & { __placeMessages: number }
    state.__placeMessages = 0
    Worker.prototype.postMessage = function (message: unknown) {
      if ((message as { type?: string })?.type === 'PLACE') state.__placeMessages += 1
      return originalPostMessage.call(this, message)
    }
  })
  await page.goto('/')

  // loading → ready: üç ürünün durum bilgisi görülür
  await expect(page.getByRole('button', { name: 'Yerleştir' })).toBeVisible({ timeout: 60_000 })
  await expect(page.getByText('Alan: 5000 × 5000 mm')).toBeVisible()
  for (const id of ['product-1', 'product-2', 'product-3']) {
    await expect(page.getByText(new RegExp(`${id} — safety`))).toBeVisible()
  }

  await page.getByRole('button', { name: 'Yerleştir' }).click()

  // RESULT: üç ürün yerleşir (sentetik fixture'lar alanına sığar)
  await expect(page.getByText('Yerleştirme tamamlandı: 3/3 ürün.')).toBeVisible({ timeout: 120_000 })

  // SVG tek transform ile çizilir: 3 ürün grubu
  const canvas = page.getByRole('img', { name: 'Yerleşim alanı' })
  await expect(canvas).toBeVisible()
  expect(await canvas.locator('g g').count()).toBe(3)

  await page.setViewportSize({ width: 360, height: 740 })
  const box = await canvas.boundingBox()
  expect(box?.width).toBeCloseTo(box?.height ?? 0, 0)
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(360)

  // Tekrar deneme bozuk durum bırakmaz: düğme yeniden aktif
  await expect(page.getByRole('button', { name: 'Yerleştir' })).toBeEnabled()
  await page.getByRole('button', { name: 'Yerleştir' }).click()
  await page.waitForFunction(
    () => (window as typeof window & { __placeMessages: number }).__placeMessages === 2,
  )
  await expect(page.getByText('Yerleştirme tamamlandı: 3/3 ürün.')).toBeVisible({ timeout: 60_000 })
})
