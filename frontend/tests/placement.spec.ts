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

  // loading → ready: üç ürünün durum bilgisi görülür (rol etiketi dahil)
  await expect(page.getByRole('button', { name: 'Yerleştir' })).toBeVisible({ timeout: 60_000 })
  await expect(page.getByText('Alan: 5000 × 5000 mm')).toBeVisible()
  await expect(page.getByText('anchor', { exact: true })).toBeVisible()
  await expect(page.getByText('peripheral', { exact: true })).toBeVisible()
  for (const id of ['product-1', 'product-2', 'product-3']) {
    await expect(page.getByText(id, { exact: true })).toBeVisible()
  }

  await page.getByRole('button', { name: 'Yerleştir' }).click()

  // RESULT: üç ürün yerleşir (sentetik fixture'lar alanına sığar)
  await expect(page.getByText('Yerleştirme tamamlandı: 3/3 ürün.')).toBeVisible({ timeout: 120_000 })

  // SVG tek transform ile çizilir: 3 ürün grubu
  const canvas = page.getByRole('img', { name: 'Yerleşim alanı' })
  await expect(canvas).toBeVisible()
  expect(await canvas.locator('g g').count()).toBe(3)
  const anchor = canvas.locator('[data-product-id="product-1"]')
  await expect(anchor).toHaveAttribute('data-placement-role', 'anchor')
  await expect(anchor.locator(':scope > polygon').first()).toHaveAttribute('stroke', '#d96b43')
  await expect(anchor.locator(':scope > polygon').first()).not.toHaveAttribute('stroke-dasharray')
  const anchorTransform = await anchor.getAttribute('transform')
  const anchorPosition = anchorTransform?.match(/translate\(([-\d.]+) ([-\d.]+)\)/)
  expect(anchorPosition).not.toBeNull()
  expect(Number(anchorPosition?.[1])).toBeGreaterThanOrEqual(1250)
  expect(Number(anchorPosition?.[1])).toBeLessThanOrEqual(3750)
  expect(Number(anchorPosition?.[2])).toBeGreaterThanOrEqual(1250)
  expect(Number(anchorPosition?.[2])).toBeLessThanOrEqual(3750)

  const peripheral = canvas.locator('[data-product-id="product-3"]')
  const peripheralTransform = await peripheral.getAttribute('transform')
  const peripheralPosition = peripheralTransform?.match(/translate\(([-\d.]+) ([-\d.]+)\)/)
  expect(peripheralPosition).not.toBeNull()
  const peripheralX = Number(peripheralPosition?.[1])
  const peripheralY = Number(peripheralPosition?.[2])
  expect(
    peripheralX < 1250 || peripheralX > 3750 || peripheralY < 1250 || peripheralY > 3750,
  ).toBe(true)

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
