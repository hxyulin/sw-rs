// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";
const fixtures = fileURLToPath(new URL("../test-fixtures/", import.meta.url));
test.beforeEach(async ({ page }) => {
  page.on("console", (message) => {
    if (message.type() === "error") console.error(message.text());
  });
  page.on("pageerror", (error) => console.error(error));
});
async function ready(page) {
  await page.goto("./");
  await expect(page.locator("#status")).toHaveAttribute("data-state", "ready");
  await page.waitForFunction(
    () => window.viewer?.snapshot().elements.length > 0,
  );
}
async function clickRxui(page, key) {
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
test("RXUI starts, draws and loads the synthetic demo through a real canvas click", async ({
  page,
}) => {
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
  await page.screenshot({ path: "test-results/demo.png" });
  expect(errors).toEqual([]);
});
for (const suffix of ["zip", "tar", "tar.gz"])
  test(`loads ${suffix}, composes repeated occurrences and selects a Part`, async ({
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
test("RXUI Open button invokes the native file chooser", async ({ page }) => {
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
test("sidebar keyboard focus can activate the Open action", async ({
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

test("optional private native archive renders with CJK labels", async ({
  page,
}) => {
  test.skip(
    !process.env.SW_VIEWER_ARCHIVE,
    "Set SW_VIEWER_ARCHIVE to a local, privately held native archive.",
  );
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await ready(page);
  await page.locator("#file").setInputFiles(process.env.SW_VIEWER_ARCHIVE);
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
  await page.screenshot({ path: "test-results/private-native.png" });
  expect(errors).toEqual([]);
});

test("resize keeps the UI canvas and pointer coordinates aligned", async ({
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
