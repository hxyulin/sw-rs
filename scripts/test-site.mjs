// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Exercise the assembled project-prefix deployment with the viewer's Playwright.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFile, stat, mkdir } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
const require = createRequire(new URL('../viewer/package.json', import.meta.url));
const { chromium, expect } = require('@playwright/test');
const root = fileURLToPath(new URL('../.site/', import.meta.url));
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
  '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml',
  '.woff2': 'font/woff2' };
const server = createServer(async (req, res) => {
  try {
    const path = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    if (!path.startsWith('/sw-rs/')) throw new Error('Outside project prefix');
    let file = resolve(root, path.slice('/sw-rs/'.length));
    if (!(file + sep).startsWith(root)) throw new Error('Outside site');
    if ((await stat(file)).isDirectory()) file = resolve(file, 'index.html');
    res.setHeader('Content-Type', types[extname(file)] || 'application/octet-stream');
    res.end(await readFile(file));
  } catch {
    res.writeHead(404).end('Not found');
  }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const base = `http://127.0.0.1:${server.address().port}/sw-rs/`;
let browser;
try {
  browser = await chromium.launch({ channel: process.env.SW_VIEWER_BROWSER_CHANNEL || undefined });
  const page = await browser.newPage({ viewport: { width: 1280, height: 900 }, colorScheme: 'light' });
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('response', (response) => {
    if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`);
  });
  await page.goto(`${base}docs/`);
  await expect(page.locator('.VPHero .text')).toHaveText('Read the saved model.');
  await page.getByRole('link', { name: 'Get started', exact: true }).click();
  await expect(page).toHaveURL(/\/docs\/getting-started\.html$/);
  await expect(page.getByRole('heading', { name: /^Getting started/ })).toBeVisible();
  await page.getByRole('link', { name: 'Containers & compression', exact: true }).click();
  await expect(page.getByRole('heading', { name: /^Containers and compression/ })).toBeVisible();
  await page.getByRole('button', { name: 'Search', exact: true }).click();
  await page.locator('#localsearch-input').fill('ZLB');
  await expect(page.locator('.results li').first()).toBeVisible();
  await expect(page.locator('.results')).toContainText(/legacy|compression/i);
  await page.keyboard.press('Escape');
  const theme = page.getByRole('switch');
  await theme.click();
  await expect(theme).toHaveAttribute('aria-checked', 'true');
  await theme.click();
  await expect(theme).toHaveAttribute('aria-checked', 'false');
  await page.setViewportSize({ width: 360, height: 800 });
  await expect(page.getByRole('button', { name: 'Menu', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Menu', exact: true }).click();
  await expect(page.locator('.VPSidebar')).toBeVisible();
  assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth), false);
  await page.goto(`${base}api/sw_rs/index.html`);
  await expect(page.getByRole('heading', { name: /Crate sw_rs/, level: 1 })).toBeVisible();
  await page.goto(`${base}api/sw_rs/struct.DocumentReader.html`);
  await expect(page.locator('[id="method.references_with_geometry"]')).toBeAttached();
  if (process.env.SW_DOCS_SCREENSHOTS) {
    await mkdir(process.env.SW_DOCS_SCREENSHOTS, { recursive: true });
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto(`${base}docs/format.html`);
    await page.screenshot({ path: `${process.env.SW_DOCS_SCREENSHOTS}/format.png`, fullPage: true });
    await page.goto(`${base}docs/`);
    await page.screenshot({ path: `${process.env.SW_DOCS_SCREENSHOTS}/home.png`, fullPage: true });
  }
  assert.deepEqual(errors, [], 'No browser errors or failed local requests');
  console.log('Site browser checks passed: prefix routes, navigation, search, themes, mobile menu and Rust API');
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
} finally {
  await browser?.close();
  await new Promise((done) => server.close(done));
}
