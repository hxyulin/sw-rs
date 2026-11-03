// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready } from "./support/viewer.js";

test("supplied bitmap pixels render and release on document changes", { tag: "@hidpi" }, async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(String(error)));
  await page.addInitScript(() => {
    const NativeWorker = window.Worker;
    window.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args);
        this.addEventListener("message", ({ data }) => {
          if (data.type === "scene") window.bitmapScene = data.scene;
        });
      }
    };
  });
  await ready(page);
  const load = async name => {
    await page.evaluate(() => { window.bitmapScene = null; });
    await page.locator("#file").setInputFiles(path.join(fixtures, name));
    await expect.poll(() => page.evaluate(() => window.bitmapScene?.triangles)).toBe(12);
    await expect.poll(() => page.evaluate(() => window.viewer.busy)).toBe(false);
  };
  await load("bitmap.zip");
  expect(await page.evaluate(() => window.bitmapScene.textures.map(t => t.path))).toEqual(["images/grid.png"]);
  expect(await page.evaluate(() => window.bitmapScene.batches[0].texture_index)).toBe(0);
  expect(await page.evaluate(() => window.bitmapScene.diagnostics.join("\n"))).toContain("approximate Part-local projection");
  const pixels = () => page.locator("#scene").evaluate(canvas => canvas.toDataURL());
  const textured = await pixels();
  const colors = await page.locator("#scene").evaluate(canvas => {
    const copy = document.createElement("canvas");
    copy.width = canvas.width; copy.height = canvas.height;
    const context = copy.getContext("2d"); context.drawImage(canvas, 0, 0);
    const data = context.getImageData(0, 0, copy.width, copy.height).data;
    const counts = [0, 0, 0];
    for (let i = 0; i < data.length; i += 4) for (let c = 0; c < 3; c++) {
      if (data[i+c] > 60 && data[i+c] > data[i+(c+1)%3]*1.8 && data[i+c] > data[i+(c+2)%3]*1.8) counts[c]++;
    }
    return counts;
  });
  expect(colors.every(count => count > 100)).toBe(true);
  const style = page.getByRole("combobox", { name: "Appearance style", exact: true });
  await style.selectOption("family");
  expect(await pixels()).not.toBe(textured);
  await style.selectOption("saved");
  expect(await pixels()).toBe(textured);
  await load("bitmap-cache.zip");
  expect(await page.evaluate(() => window.bitmapScene.textures.map(t => t.path))).toEqual(["images/grid.png"]);
  expect(await pixels()).toBe(textured);
  await load("bitmap-override.zip");
  expect(await page.evaluate(() => window.bitmapScene.textures)).toEqual([]);
  expect(await pixels()).not.toBe(textured);
  await load("bitmap-missing.zip");
  expect(await page.evaluate(() => window.bitmapScene.textures)).toEqual([]);
  expect(await page.evaluate(() => window.bitmapScene.diagnostics.join("\n"))).toContain("not supplied");
  expect(await pixels()).not.toBe(textured);
  await load("bitmap-ambiguous.zip");
  expect(await page.evaluate(() => window.bitmapScene.textures)).toEqual([]);
  expect(await page.evaluate(() => window.bitmapScene.diagnostics.join("\n"))).toContain("multiple archive images");
  await load("bitmap-corrupt.zip");
  expect(await page.evaluate(() => window.bitmapScene.diagnostics.join("\n"))).toContain("Bitmap unavailable");
  await load("bitmap.zip");
  expect(await pixels()).toBe(textured);
  expect(errors).toEqual([]);
});
