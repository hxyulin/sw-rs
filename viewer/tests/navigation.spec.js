// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready, clickRxui } from "./support/viewer.js";

test.describe("navigation", () => {
  test("RXUI starts, draws and loads the synthetic demo through a real canvas click", { tag: "@hidpi" }, async ({
    page,
  }, testInfo) => {
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await ready(page);
    expect(
      (await page.evaluate(() => window.viewer.snapshot())).state.documents,
    ).toBe(0);
    await clickRxui(page, "demo");
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(48);
    expect(
      (await page.evaluate(() => window.viewer.snapshot())).state.documents,
    ).toBe(4);
    await page.screenshot({ path: testInfo.outputPath("demo.png") });
    expect(errors).toEqual([]);
  });

  test("sidebar keyboard focus can activate the Open action", { tag: "@hidpi" }, async ({
    page,
  }) => {
    await ready(page);
    // Install chooser interception before the direct keyboard command.
    const chooser = page.waitForEvent("filechooser");
    await page.locator("#sidebar").focus();
    await page.keyboard.press("Tab");
    await page.keyboard.press("Enter");
    await (await chooser).setFiles(path.join(fixtures, "single.SLDPRT"));
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(12);
  });

  test("resize keeps the UI canvas and pointer coordinates aligned", { tag: "@hidpi" }, async ({
    page,
  }) => {
    await ready(page);
    await page.setViewportSize({ width: 960, height: 700 });
    await clickRxui(page, "demo");
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(48);
    const box = await page.locator("#sidebar").boundingBox();
    expect(box.height).toBe(654);
    await clickRxui(page, "file-1");
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(12);
  });

  test("parts search filters filenames without changing the rendered selection", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(48);
    const search = page.getByRole("searchbox", { name: "Search parts" });
    const listFiles = () => page.evaluate(() => window.viewer.snapshot().elements
      .filter(e => e.key?.startsWith('String("file-')));
    await search.fill("uPrIgHt");
    await expect.poll(async () => (await listFiles()).map(e => e.text)).toEqual(["Upright.SLDPRT"]);
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(48);
    expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("Demo.SLDASM");
    await clickRxui(page, "file-3");
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    await search.fill("not a part");
    await expect.poll(async () => (await listFiles()).length).toBe(0);
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect((await page.evaluate(() => window.viewer.snapshot())).elements
      .some(e => e.text === "No matching documents")).toBe(true);
    await page.getByRole("button", { name: "Clear parts search" }).click();
    await expect(search).toHaveValue("");
    await expect.poll(async () => (await listFiles()).length).toBe(4);
    expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("Upright.SLDPRT");
  });

  test("search supports Unicode paths, keyboard focus and resized canvas bounds", { tag: "@hidpi" }, async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "unicode-search.zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    await page.locator("#sidebar").focus();
    await page.keyboard.press("/");
    const search = page.getByRole("searchbox", { name: "Search parts" });
    await expect(search).toBeFocused();
    await search.fill("零件/支架");
    await expect.poll(() => page.evaluate(() => window.viewer.snapshot().elements
      .filter(e => e.key?.startsWith('String("file-')).map(e => e.text))).toEqual(["支架.SLDPRT"]);
    await page.setViewportSize({ width: 960, height: 700 });
    await expect.poll(async () => {
      const input = await page.locator("#parts-search").boundingBox();
      const sidebar = await page.locator("#sidebar").boundingBox();
      const bounds = await page.evaluate(() => window.viewer.snapshot().elements
        .find(e => e.key === 'String("search-slot")').bounds);
      return Math.abs(input.y - sidebar.y - bounds[1]) + Math.abs(input.x - sidebar.x - bounds[0]);
    }).toBeLessThan(2);
    await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
    await expect(search).toHaveValue("");
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(48);
  });

  test("assembly chooser waits for selection, filters paths and reopens", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "hierarchy.zip"));
    const chooser = page.getByRole("dialog", { name: "Choose an assembly" });
    await expect(chooser).toBeVisible();
    await expect(page.locator("#loading")).toBeHidden();
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(0);
    await page.getByRole("searchbox", { name: "Search assemblies" }).fill("sUb");
    await expect(page.locator("#assembly-options button")).toHaveText(["Sub.SLDASM"]);
    await page.getByRole("button", { name: "Sub.SLDASM", exact: true }).click();
    await expect(chooser).toBeHidden();
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("Sub.SLDASM");
    await page.getByRole("button", { name: "Choose assembly…", exact: true }).click();
    await expect(page.getByRole("searchbox", { name: "Search assemblies" })).toHaveValue("");
    await expect(page.locator("#assembly-options button")).toHaveText(["Root.SLDASM", "Sub.SLDASM"]);
    await page.getByRole("button", { name: "Root.SLDASM", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(24);
  });

  test("occurrence tree retains repeated subassemblies and excluded states with keyboard expansion", { tag: "@hidpi" }, async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "hierarchy.zip"));
    await page.getByRole("button", { name: "Root.SLDASM", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(24);
    const tree = page.getByRole("tree", { name: "Component occurrences" });
    await expect(tree.getByRole("treeitem")).toHaveCount(5);
    const root = tree.getByRole("treeitem", { name: "Root.SLDASM · assembly · Default", exact: true });
    await expect(root).toHaveAttribute("aria-expanded", "true");
    const first = tree.getByRole("treeitem", { name: "child0 · assembly · Default", exact: true });
    await expect(first).toHaveAttribute("aria-expanded", "false");
    await first.focus();
    await page.keyboard.press("ArrowRight");
    await expect(first).toHaveAttribute("aria-expanded", "true");
    await expect(tree.getByRole("treeitem")).toHaveCount(8);
    await expect(tree.getByRole("treeitem", { name: "child1 · missing · Default", exact: true })).toHaveCount(1);
    await expect(tree.getByRole("treeitem", { name: "child2 · unsupported · Default", exact: true })).toHaveCount(1);
    await expect(tree.getByRole("treeitem", { name: "child2 · suppressed · Default", exact: true })).toHaveCount(1);
    await expect(tree.getByRole("treeitem", { name: "child3 · hidden · Default", exact: true })).toHaveCount(1);
    await expect(tree.getByRole("treeitem", { name: "child1 · missing · Default", exact: true }).getByRole("button")).toHaveCount(0);
    await page.keyboard.press("ArrowDown");
    await expect(tree.getByRole("treeitem", { name: "child0 · part · Default", exact: true })).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(first).toBeFocused();
    await page.keyboard.press("ArrowLeft");
    await expect(first).toHaveAttribute("aria-expanded", "false");
    // Pointer expansion returns the keyboard focus to the occurrence row.
    await page.getByRole("button", { name: "Expand child0", exact: true }).click();
    await expect(first).toBeFocused();
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("Base.SLDPRT");
  });

  test("dismissing the chooser preserves individual Part browsing and search", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "hierarchy.zip"));
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();
    await expect(page.locator("#status")).toContainText("choose a document in the sidebar");
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(0);
    await page.getByRole("searchbox", { name: "Search parts" }).fill("Base");
    const key = await page.evaluate(() => window.viewer.snapshot().elements.find(e => e.text === "Base.SLDPRT").key.match(/file-\d+/)[0]);
    await clickRxui(page, key);
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    await page.getByRole("button", { name: "Choose assembly…", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.getByRole("button", { name: "Browse individual documents", exact: true }).click();
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect(await page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("Base.SLDPRT");
  });

  test("embedded occurrence opens its worker-local path outside the archive list", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "embedded.zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    await page.getByRole("button", { name: "Open child0", exact: true }).first().click();
    await expect.poll(() => page.evaluate(() => window.viewer.snapshot().state.selected)).toContain("/@embedded/嵌入.SLDASM");
    await expect.poll(() => page.evaluate(() => window.viewer.busy)).toBe(false);
    await page.getByRole("button", { name: "Open child0", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.viewer.snapshot().state.selected)).toContain("/@embedded/嵌入零件.SLDPRT");
    await expect.poll(() => page.evaluate(() => window.viewer.busy)).toBe(false);
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect(await page.evaluate(() => window.viewer.snapshot().state.documents)).toBe(1);
  });

  test("opening a configured occurrence retains its nondefault assembly configuration", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "configuration-hierarchy.zip"));
    await page.getByRole("button", { name: "ConfigRoot.SLDASM", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    await page.getByRole("button", { name: "Open Configured subassembly", exact: true }).click();
    await expect.poll(() => page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("ConfigSub.SLDASM");
    await expect.poll(() => page.evaluate(() => window.viewer.busy)).toBe(false);
    expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
    await expect(page.getByRole("treeitem", { name: "ConfigSub.SLDASM · assembly · Alternate", exact: true })).toBeVisible();
    const elements = await page.evaluate(() => window.viewer.snapshot().elements);
    expect(elements.some(e => e.text === "Alternate")).toBe(true);
    expect(await page.evaluate(() => window.viewer.snapshot().state.diagnostics)).toEqual([]);
  });
});
