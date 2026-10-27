// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { expect } from "@playwright/test";
import { fileURLToPath } from "node:url";
export const fixtures = fileURLToPath(new URL("../../test-fixtures/", import.meta.url));
export async function ready(page) {
  await page.goto("./");
  await expect(page.locator("#status")).toHaveAttribute("data-state", "ready");
  await page.waitForFunction(
    () => window.viewer?.snapshot().elements.length > 0,
  );
}
export async function clickRxui(page, key) {
  const e = await page.evaluate(
    (key) =>
      window.viewer
        .snapshot()
        .elements.find((e) => e.key?.includes('"' + key + '"')),
    key,
  );
  expect(e, `RXUI element ${key}`).toBeTruthy();
  const canvas = await page.locator("#sidebar").boundingBox();
  await page.mouse.click(
    canvas.x + e.bounds[0] + e.bounds[2] / 2,
    canvas.y + e.bounds[1] + e.bounds[3] / 2,
  );
}
