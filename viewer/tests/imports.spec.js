// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready, clickRxui } from "./support/viewer.js";

test.describe("imports", () => {
  for (const suffix of ["zip", "tar", "tar.gz"])
    test(`loads ${suffix}, composes repeated occurrences and selects a Part`, { tag: (suffix === "zip" ? "@hidpi" : []) }, async ({
      page,
    }) => {
      const errors = [];
      page.on("pageerror", (e) => errors.push(e.message));
      await ready(page);
      await page
        .locator("#file")
        .setInputFiles(path.join(fixtures, "demo." + suffix));
      await expect
        .poll(() => page.evaluate(() => window.viewer.triangles))
        .toBe(48);
      expect(
        (await page.evaluate(() => window.viewer.snapshot())).state.diagnostics,
      ).toEqual([]);
      await clickRxui(page, "file-1");
      await expect
        .poll(() => page.evaluate(() => window.viewer.triangles))
        .toBe(12);
      await expect
        .poll(() => page.evaluate(() => window.viewer.snapshot().state.selected))
        .toBe("Base.SLDPRT");
      const before = await page.locator("#scene").evaluate((c) => c.toDataURL());
      const bounds = await page.locator("#scene").boundingBox();
      await page.mouse.move(
        bounds.x + bounds.width / 2,
        bounds.y + bounds.height / 2,
      );
      await page.mouse.down();
      await page.mouse.move(
        bounds.x + bounds.width / 2 + 70,
        bounds.y + bounds.height / 2 + 30,
        { steps: 10 },
      );
      await page.mouse.up();
      expect(
        await page.locator("#scene").evaluate((c) => c.toDataURL()),
      ).not.toBe(before);
      const orbit = await page.locator("#scene").evaluate((c) => c.toDataURL());
      await clickRxui(page, "wire");
      expect(
        await page.locator("#scene").evaluate((c) => c.toDataURL()),
      ).not.toBe(orbit);
      await page.getByRole("button", { name: "Cancel / clear" }).click();
      await expect
        .poll(() => page.evaluate(() => window.viewer.triangles))
        .toBe(0);
      expect(errors).toEqual([]);
    });

  test("native Part and failed archive recovery stay responsive", async ({
    page,
  }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "invalid.zip"));
    await expect(page.locator("#status")).toHaveAttribute("data-state", "error");
    await expect(page.locator("#loading")).toBeHidden();
    await page
      .locator("#file")
      .setInputFiles(path.join(fixtures, "single.SLDPRT"));
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(12);
    await expect(page.locator("#status")).toHaveAttribute("data-state", "ready");
  });

  test("RXUI Open button invokes the native file chooser", { tag: "@hidpi" }, async ({ page }) => {
    await ready(page);
    const chooser = page.waitForEvent("filechooser");
    await clickRxui(page, "open");
    await (await chooser).setFiles(path.join(fixtures, "demo.zip"));
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(48);
  });

  test("cancellation invalidates a pending upload and the next import succeeds", async ({
    page,
  }) => {
    await ready(page);
    await page.evaluate(() => {
      window.pendingUpload = {
        size: 1,
        name: "old.zip",
        arrayBuffer: () =>
          new Promise((resolve) => (window.finishUpload = resolve)),
      };
      window.viewer.load(window.pendingUpload);
    });
    await page.getByRole("button", { name: "Cancel / clear" }).click();
    await page.evaluate(() => window.finishUpload(new ArrayBuffer(0)));
    await expect(page.locator("#status")).toHaveText("Import cancelled");
    expect(
      await page.evaluate(() => window.viewer.snapshot().state.documents),
    ).toBe(0);
    await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(48);
  });

  test("import progress tracks real worker phases and clears after rendering", async ({ page }) => {
    await ready(page);
    await page.evaluate(() => {
      window.importProgress = [];
      const loading = document.querySelector("#loading");
      new MutationObserver(() => window.importProgress.push({
        label: document.querySelector("#loading-label").textContent,
        value: document.querySelector("#loading-progress").getAttribute("value"),
        hidden: loading.hidden,
      })).observe(loading, { subtree: true, attributes: true, childList: true, characterData: true });
    });
    await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(48);
    await expect(page.locator("#loading")).toBeHidden();
    await expect(page.locator("main")).toHaveAttribute("aria-busy", "false");
    const events = await page.evaluate(() => window.importProgress);
    expect(events.some(e => e.label.startsWith("Reading ") && !e.hidden)).toBe(true);
    expect(events.some(e => e.label.startsWith("Extracting ") && e.value === "100")).toBe(true);
    expect(events.some(e => e.label.includes("Decoding") && e.value === null)).toBe(true);
  });

  test("local sources above the former upload cap reach the parser", async ({ page }) => {
    await ready(page);
    await page.evaluate(async () => {
      const bytes = await (await fetch("./demo.zip")).arrayBuffer();
      // Virtual source avoids allocating 129 MiB just to test the size preflight.
      await window.viewer.load({ name: "large.zip", size: 129 * 1024 * 1024,
        arrayBuffer: async () => bytes });
    });
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(48);
    await expect(page.locator("#loading")).toBeHidden();
  });

  test("cancelling a pending read hides progress and rejects its stale result", async ({ page }) => {
    await ready(page);
    await page.evaluate(() => {
      window.viewer.load({ name: "pending.zip", size: 1,
        arrayBuffer: () => new Promise(resolve => window.completeRead = resolve) });
    });
    await expect(page.locator("#loading")).toBeVisible();
    await expect(page.getByRole("progressbar")).toHaveAttribute("value", "0");
    await page.getByRole("button", { name: "Cancel / clear" }).click();
    await page.evaluate(() => window.completeRead(new ArrayBuffer(0)));
    await expect(page.locator("#loading")).toBeHidden();
    await expect(page.locator("main")).toHaveAttribute("aria-busy", "false");
    await expect(page.locator("#status")).toHaveText("Import cancelled");
  });

  test("missing references leave available assembly parts rendered", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "missing-references.zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(24);
    await expect(page.locator("#status")).toHaveAttribute("data-state", "ready");
    await expect(page.locator("#status")).toContainText("Partial assembly");
    await expect(page.locator("#status")).toContainText("2 missing reference(s)");
    const state = await page.evaluate(() => window.viewer.snapshot().state);
    expect(state.diagnostics).toHaveLength(2);
    expect(state.diagnostics.every(d => d.includes("Upright.SLDPRT"))).toBe(true);
    await expect(page.locator("#empty")).toBeHidden();
    await expect(page.locator("#loading")).toBeHidden();
  });

  for (const name of ["renamed", "embedded", "virtual"])
    test(`resolves ${name} native references without external files or filename guessing`, async ({ page }) => {
      await ready(page);
      await page.locator("#file").setInputFiles(path.join(fixtures, name + ".zip"));
      await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
      expect(await page.evaluate(() => window.viewer.snapshot().state.diagnostics)).toEqual([]);
      await expect(page.locator("#status")).toHaveAttribute("data-state", "ready");
    });

  test("legacy OLE imports cached geometry and exposes original compressed-stream provenance", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures,"legacy-valid.SLDPRT"));
    await expect.poll(()=>page.evaluate(()=>window.viewer.triangles)).toBe(12);
    await page.getByRole("button",{name:"Inspect source…"}).click();
    await expect(page.locator("#inventory-summary")).toContainText("legacy_ole2");
    await expect(page.locator("#inventory-status")).toContainText("Container validation available");
    await page.getByLabel("Filter source streams").fill("Contents/DisplayLists");
    await page.locator("#inventory-streams button").click();
    await expect(page.locator("#inventory-detail")).toContainText('"source_record"');
    const detail=JSON.parse(await page.locator("#inventory-detail").textContent());
    expect(detail.source_record.ole_source.path).toBe("Contents/DisplayLists__ZLB");
    expect(detail.source_record.ole_source.compression.kind).toBe("zlb");
    expect(detail.source_record.ole_source.ranges.length).toBeGreaterThan(0);
    await page.getByRole("button",{name:"Close source inspector"}).click();
    expect(await page.evaluate(()=>window.viewer.triangles)).toBe(12);
  });
});
