import { test, expect } from '@playwright/test'

test('create environment and see it in list', async ({ page }) => {
  await page.goto('/')

  await page.fill('#env-name', 'test-env')
  await page.fill('input[type="url"]', 'https://github.com/org/repo')
  await page.click('button[type="submit"]')

  await expect(page.getByText('test-env')).toBeVisible({ timeout: 10_000 })
})
