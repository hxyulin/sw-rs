// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from '@playwright/test';
import path from 'node:path';
import { fixtures, ready, clickRxui } from './support/viewer.js';

async function selectFirst(page, archive = 'demo.zip') {
  await ready(page);
  await page.locator('#file').setInputFiles(path.join(fixtures, archive));
  await expect.poll(() => page.evaluate(() => window.viewer.busy)).toBe(false);
  const rows = page.locator('#occurrence-tree > [role=treeitem] > [role=group] > [role=treeitem]');
  await rows.first().locator(':scope > .node-row .node-name').click();
  await page.locator('#motion-panel > summary').click();
  return rows;
}
const pixels = page => page.locator('#scene').evaluate(c => c.toDataURL());

test('hinge preview rotates and resets exactly, including wireframe', { tag: '@hidpi' }, async ({ page }) => {
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  const rows = await selectFirst(page);
  const saved = await pixels(page);
  await page.getByLabel('Motion value', { exact: true }).fill('60');
  await expect(page.locator('#motion-status')).toHaveAttribute('data-state', 'ready');
  expect(await pixels(page)).not.toBe(saved);
  const moved = await pixels(page);
  await page.getByLabel('Motion value', { exact: true }).fill('30');
  await page.getByLabel('Motion value', { exact: true }).fill('60');
  expect(await pixels(page)).toBe(moved);
  await page.getByRole('button', { name: 'Reset motion', exact: true }).click();
  expect(await pixels(page)).toBe(saved);
  await clickRxui(page, 'wire');
  const wire = await pixels(page);
  await page.getByLabel('Motion value', { exact: true }).fill('90');
  expect(await pixels(page)).not.toBe(wire);
  await rows.nth(1).locator(':scope > .node-row .node-name').click();
  await rows.first().locator(':scope > .node-row .node-name').click();
  expect(await pixels(page)).toBe(wire);
  await expect(page.getByLabel('Motion value', { exact: true })).toHaveValue('0');
  expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe('Demo.SLDASM');
  expect(errors).toEqual([]);
});

test('slider enforces travel limits, preserves siblings and picks the moved component', async ({ page }) => {
  const rows = await selectFirst(page);
  await page.getByLabel('Joint type', { exact: true }).selectOption('slider');
  await page.getByLabel('Joint axis', { exact: true }).selectOption('x');
  await expect(page.locator('#motion-unit')).toHaveText('Travel (mm)');
  const saved = await pixels(page);
  await page.getByLabel('Motion value', { exact: true }).fill('50');
  expect(await pixels(page)).not.toBe(saved);
  await page.getByLabel('Motion value', { exact: true }).fill('101');
  await expect(page.locator('#motion-status')).toHaveAttribute('data-state', 'error');
  expect(await pixels(page)).toBe(saved);
  await page.getByLabel('Lower travel limit').fill('10');
  await expect(page.locator('#motion-status')).toContainText('Limits must include zero');
  await expect(page.getByRole('slider', { name: 'Joint motion' })).toBeDisabled();
  await page.getByLabel('Lower travel limit').fill('-25');
  await page.getByLabel('Upper travel limit').fill('25');
  await page.getByLabel('Motion value', { exact: true }).fill('25');
  await expect(page.locator('#motion-status')).toHaveAttribute('data-state', 'ready');
  const first = await page.evaluate(() => window.viewer.components().selected);
  await page.getByRole('button', { name: 'Isolate', exact: true }).click();
  await page.getByRole('button', { name: 'Focus', exact: true }).click();
  const canvas = await page.locator('#scene').boundingBox();
  await page.mouse.click(canvas.x + canvas.width / 2, canvas.y + canvas.height / 2);
  expect(await page.evaluate(() => window.viewer.components().selected)).toBe(first);
  await expect(page.getByLabel('Motion value', { exact: true })).toHaveValue('25');
  // A sibling's isolated image is identical before and after previewing the first.
  await rows.nth(1).locator(':scope > .node-row .node-name').click();
  await page.getByRole('button', { name: 'Isolate', exact: true }).click();
  await page.getByRole('button', { name: 'Focus', exact: true }).click();
  const sibling = await pixels(page);
  await rows.first().locator(':scope > .node-row .node-name').click();
  await page.getByLabel('Motion value', { exact: true }).fill('90');
  await rows.nth(1).locator(':scope > .node-row .node-name').click();
  expect(await pixels(page)).toBe(sibling);
});

test('native-less motion API validates definitions and rotates around an explicit pivot', async ({ page }) => {
  await ready(page);
  const result = await page.evaluate(async () => {
    const { joint_transform } = await import('/pkg/sw_viewer.js');
    const joint = { kind: 'hinge', axis: [0, 0, 2], pivot: [1, 2, 3], lower: -Math.PI, upper: Math.PI };
    const m = joint_transform(JSON.stringify({ joint, coordinate: Math.PI / 2 }));
    const point = [2, 2, 3].map((_, i) => m[i]*2 + m[4+i]*2 + m[8+i]*3 + m[12+i]);
    let rejected = false;
    try { joint_transform(JSON.stringify({ joint: { ...joint, axis: [0, 0, 0] }, coordinate: 0 })); } catch { rejected = true; }
    return { point, rejected };
  });
  result.point.forEach((v, i) => expect(v).toBeCloseTo([1, 3, 3][i], 12));
  expect(result.rejected).toBe(true);
});
