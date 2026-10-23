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
test("search supports Unicode paths, keyboard focus and resized canvas bounds", async ({ page }) => {
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

for (const name of ["renamed", "embedded", "virtual"])
  test(`resolves ${name} native references without external files or filename guessing`, async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, name + ".zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    expect(await page.evaluate(() => window.viewer.snapshot().state.diagnostics)).toEqual([]);
    await expect(page.locator("#status")).toHaveAttribute("data-state", "ready");
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

test("occurrence tree retains repeated subassemblies and excluded states with keyboard expansion", async ({ page }) => {
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

test("native Part inspector shows saved material and raw density without inventing units", async ({ page }) => {
  await ready(page);
  await page.locator("#file").setInputFiles(path.join(fixtures, "single.SLDPRT"));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
  const info = await page.evaluate(() => window.viewer.snapshot().state.info.join("\n"));
  expect(info).toContain("Saved material: test materials|6061 Alloy");
  expect(info).toContain("Material classification: Aluminum");
  expect(info).toContain("Saved density token: 2700 (units unverified)");
  await expect(page.getByRole("dialog")).toBeHidden();
  await expect(page.getByRole("button", { name: "Material colors", exact: true })).toHaveAttribute("aria-pressed", "true");
  const materialView = await page.locator("#scene").evaluate(c => c.toDataURL());
  await page.getByRole("button", { name: "Material colors", exact: true }).click();
  await expect(page.getByRole("button", { name: "Diagnostic colors", exact: true })).toHaveAttribute("aria-pressed", "false");
  expect(await page.locator("#scene").evaluate(c => c.toDataURL())).not.toBe(materialView);
  expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
  await page.getByRole("button", { name: "Diagnostic colors", exact: true }).click();
  expect(await page.locator("#scene").evaluate(c => c.toDataURL())).toBe(materialView);
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

test("appearance style recreates saved shader separately from physical family without decoding", async ({ page }) => {
  await ready(page);
  await page.evaluate(() => {
    window.sceneRequests = 0;
    const postMessage = Worker.prototype.postMessage;
    Worker.prototype.postMessage = function(message, ...rest) {
      if (message.type === "scene") window.sceneRequests++;
      return postMessage.call(this, message, ...rest);
    };
  });
  const style = page.getByRole("combobox", { name: "Appearance style", exact: true });
  await expect(style).toHaveValue("saved");
  await expect(style.locator("option")).toHaveText(["Saved material appearance", "Material family presets"]);
  await page.locator("#file").setInputFiles(path.join(fixtures, "single.SLDPRT"));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
  const state = await page.evaluate(() => window.viewer.snapshot().state);
  expect(state.info).toContain("Saved shader: black low gloss plastic");
  expect(state.info).toContain("Material source: saved MATERIALTREE configuration reference");
  const pixels = () => page.locator("#scene").evaluate(canvas => canvas.toDataURL());
  const saved = await pixels();
  const requests = await page.evaluate(() => window.sceneRequests);
  await style.selectOption("family");
  const family = await pixels();
  expect(family).not.toBe(saved);
  expect(await page.evaluate(() => window.viewer.snapshot().state)).toEqual(state);
  expect(await page.evaluate(() => window.sceneRequests)).toBe(requests);
  expect(await page.evaluate(() => window.viewer.triangles)).toBe(12);
  await expect(page.getByRole("treeitem", { name: "single.SLDPRT · part · Default", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Material colors", exact: true }).click();
  const diagnostic = await pixels();
  await style.selectOption("saved");
  expect(await pixels()).toBe(diagnostic);
  await page.getByRole("button", { name: "Diagnostic colors", exact: true }).click();
  expect(await pixels()).toBe(saved);
  expect(await page.evaluate(() => window.sceneRequests)).toBe(requests);
  // Both document selection and a fresh import retain the chosen display style.
  await style.selectOption("family");
  await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(48);
  await clickRxui(page, "file-1");
  await expect.poll(() => page.evaluate(() => window.viewer.snapshot().state.selected)).toBe("Base.SLDPRT");
  await expect(style).toHaveValue("family");
  await page.locator("#file").setInputFiles(path.join(fixtures, "single.SLDPRT"));
  await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
  await expect(style).toHaveValue("family");
  expect(await pixels()).toBe(family);
});
