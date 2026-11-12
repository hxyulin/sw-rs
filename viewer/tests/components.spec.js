// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from '@playwright/test';
import path from 'node:path';
import { fixtures, ready } from './support/viewer.js';
import { ComponentState } from '../web/components.js';

test('nested repeated occurrences retain identity through isolate, hide and restore', () => {
  const state = new ComponentState([
    { id: 0, first_vertex: 0, vertex_count: 12 },
    { id: 1, first_vertex: 0, vertex_count: 6 },
    { id: 2, first_vertex: 0, vertex_count: 3 },
    { id: 3, first_vertex: 3, vertex_count: 3 },
    { id: 4, first_vertex: 6, vertex_count: 6 },
  ], 12);
  state.selected = 3;
  expect(state.owner(3).id).toBe(3);
  expect(state.ranges().filter(r => r.selected)).toEqual([{ first_vertex: 3, vertex_count: 3, selected: true }]);
  state.isolated = 1;
  state.hidden.add(2);
  expect(state.ranges()).toEqual([{ first_vertex: 3, vertex_count: 3, selected: true }]);
  state.hidden.add(1);
  expect(state.ranges()).toEqual([]);
  state.hidden.clear(); state.isolated = null;
  expect(state.ranges().reduce((sum, r) => sum + r.vertex_count, 0)).toBe(12);
});

test('browser inspects and controls individual instances without reopening the assembly', { tag: '@hidpi' }, async ({ page }) => {
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await ready(page);
  await page.locator('#file').setInputFiles(path.join(fixtures, 'demo.zip'));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(48);
  const rows = page.locator('#occurrence-tree > [role=treeitem] > [role=group] > [role=treeitem]');
  const before = await page.locator('#scene').evaluate(c => c.toDataURL());
  await rows.nth(0).locator(':scope > .node-row .node-name').click();
  await expect(rows.nth(0)).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('region', { name: 'Selected component' })).toBeVisible();
  await expect(page.locator('#component-properties')).toContainText('Assembly position (mm)');
  expect(await page.locator('#scene').evaluate(c => c.toDataURL())).not.toBe(before);
  const first = await page.evaluate(() => window.viewer.components().selected);
  await page.getByRole('button', { name: 'Isolate', exact: true }).click();
  expect(await page.evaluate(() => window.viewer.components().isolated)).toBe(first);
  await page.getByRole('button', { name: 'Focus', exact: true }).click();
  // The focused instance fills the center. A real viewport click picks it again.
  const canvas = await page.locator('#scene').boundingBox();
  await page.mouse.click(canvas.x + canvas.width / 2, canvas.y + canvas.height / 2);
  expect(await page.evaluate(() => window.viewer.components().selected)).toBe(first);
  await page.getByRole('button', { name: 'Hide', exact: true }).click();
  expect(await page.evaluate(() => window.viewer.components().ranges)).toEqual([]);
  // Clearing selection must leave Restore all reachable.
  await page.locator('#scene').click({ position: { x: 40, y: 150 } });
  await expect(page.locator('#component-inspector')).toBeHidden();
  await page.getByRole('button', { name: 'Restore all', exact: true }).click();
  expect(await page.evaluate(() => window.viewer.components().ranges.reduce((s, r) => s + r.vertex_count, 0))).toBe(144);
  await rows.nth(1).locator(':scope > .node-row .node-name').click();
  expect(await page.evaluate(() => window.viewer.components().selected)).not.toBe(first);
  expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe('Demo.SLDASM');
  await page.locator('#file').setInputFiles(path.join(fixtures, 'single.SLDPRT'));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
  expect(await page.evaluate(() => window.viewer.components().selected)).toBeNull();
  await expect(page.locator('#component-restore')).toBeDisabled();
  expect(errors).toEqual([]);
});

test('saved mates preserve class-resolution provenance and unresolved reference evidence', async ({ page }) => {
  await ready(page);
  await page.locator('#file').setInputFiles(path.join(fixtures, 'mates.zip'));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(24);
  await page.locator('#mate-inspector > summary').click();
  await expect(page.locator('#mate-status')).toContainText('unresolved');
  await expect(page.locator('#mate-list')).toContainText('3/3 saved headers');
  await page.locator('#mate-list summary').first().click();
  await expect(page.locator('#mate-list pre').first()).toContainText('A-2@Mated');
  await expect(page.locator('#mate-list summary').nth(1)).toContainText('Renamed · width');
  await page.locator('#mate-list summary').nth(1).click();
  await expect(page.locator('#mate-list pre').nth(1)).toContainText('Reused class tag: 32961');
  await expect(page.locator('#mate-list pre').nth(1)).toContainText('Class declaration: moMateWidth at 6');
  await expect(page.locator('#mate-list pre').nth(1)).toContainText('Class index: 193 (leading word at 0)');
  await expect(page.locator('#mate-list summary').nth(2)).toContainText('Unknown type');
});
