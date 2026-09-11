import { expect, test } from '@playwright/test'

test('shows a result produced by the real module worker and wasm', async ({ page }) => {
  await page.goto('/')

  await expect(page.getByRole('heading', { name: 'Technical spike passed' })).toBeVisible()
  await expect(page.getByText('jagua-rs 0.8.1 BPP collision query')).toBeVisible()
  await expect(page.getByText('4 closed vertices in mm')).toBeVisible()
  await expect(page.getByText('containment detected')).toBeVisible()
  await expect(page.getByText('continuous; 37 degrees accepted')).toBeVisible()
  await expect(page.getByText('accepted at fixed translation')).toBeVisible()
  await expect(page.getByText('outside area rejected')).toBeVisible()
  await expect(page.getByText('unfit item rejected; no fallback bin')).toBeVisible()
  await expect(page.getByText('Bins used: 1')).toBeVisible()
})
