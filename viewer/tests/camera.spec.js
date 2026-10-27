// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready } from "./support/viewer.js";

test.describe("camera", () => {
  test("SolidWorks front/top/right views preserve source coordinates with explicit orbit-up override", { tag: "@hidpi" }, async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "single.SLDPRT"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    const original = await page.evaluate(() => window.viewer.snapshot().state);
    expect(original.info.join("\n")).toContain("Front: XY · +Y up · +Z toward observer");
    await expect(page.getByLabel("Orbit up axis")).toHaveValue("y");
    await expect(page.getByRole("img", {name:"Model axes: X red, Y green, Z blue"})).toBeVisible();
    await page.evaluate(() => {
      window.sceneRequests = 0;
      const original = Worker.prototype.postMessage;
      Worker.prototype.postMessage = function(data, ...args) {
        if (data.type === "scene") window.sceneRequests++;
        return original.call(this, data, ...args);
      };
    });
    const expected = {
      Front: {back:[0,0,1],up:[0,1,0],right:[1,0,0]},
      Top: {back:[0,1,0],up:[0,0,-1],right:[1,0,0]},
      Right: {back:[1,0,0],up:[0,1,0],right:[0,0,-1]},
    };
    for (const [name, basis] of Object.entries(expected)) {
      await page.getByRole("button", {name,exact:true}).click();
      const result = await page.evaluate(() => window.viewer.orientation());
      for (const [axis, vector] of Object.entries(basis))
        vector.forEach((value,i) => expect(result[axis][i]).toBeCloseTo(value, 10));
      expect(result.upAxis).toBe("y");
    }
    const right = await page.locator("#scene").evaluate(c=>c.toDataURL());
    await page.getByRole("button", {name:"Isometric",exact:true}).click();
    expect(await page.locator("#scene").evaluate(c=>c.toDataURL())).not.toBe(right);
    await page.getByLabel("Orbit up axis").selectOption("z");
    expect((await page.evaluate(() => window.viewer.orientation())).upAxis).toBe("z");
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect(await page.evaluate(() => window.sceneRequests)).toBe(0);
    expect(await page.evaluate(() => window.viewer.snapshot().state)).toEqual(original);
    await page.locator("#scene").focus();
    await page.keyboard.press("f");
    expect((await page.evaluate(() => window.viewer.orientation())).upAxis).toBe("z");
    await page.getByRole("button", {name:"Front",exact:true}).click();
    await expect(page.getByLabel("Orbit up axis")).toHaveValue("y");
  });
});
