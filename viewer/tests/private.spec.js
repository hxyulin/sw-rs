// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import { ready } from "./support/viewer.js";

test.describe("private", () => {
  test("optional private native archive renders with CJK labels", async ({
    page,
  }, testInfo) => {
    test.skip(
      !process.env.SW_VIEWER_ARCHIVE,
      "Set SW_VIEWER_ARCHIVE to a local, privately held native archive.",
    );
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await ready(page);
    await page.locator("#file").setInputFiles(process.env.SW_VIEWER_ARCHIVE);
    await expect.poll(async () => await page.getByRole("dialog").isVisible()
      || await page.evaluate(() => window.viewer.triangles > 0), { timeout: 30000 }).toBe(true);
    if (await page.getByRole("dialog").isVisible())
      await page.locator("#assembly-options button").first().click();
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles), {
        timeout: 30000,
      })
      .toBeGreaterThan(0);
    await expect
      .poll(() => page.evaluate(() => window.viewer.fontReady), {
        timeout: 30000,
      })
      .toBe(true);
    await page.screenshot({ path: testInfo.outputPath("private-native.png") });
    expect(errors).toEqual([]);
  });
});
