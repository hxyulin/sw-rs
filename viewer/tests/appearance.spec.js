// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready, clickRxui } from "./support/viewer.js";

test.describe("appearance", () => {
  test("scene exports source presentation separately from renderer presets", async ({ page }) => {
    await page.addInitScript(() => {
      const NativeWorker = window.Worker;
      window.Worker = class extends NativeWorker {
        constructor(...args) {
          super(...args);
          this.addEventListener("message", ({data}) => {
            if (data.type === "scene") window.sourceScene = data.scene;
          });
        }
      };
    });
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "single.SLDPRT"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    const scene = await page.evaluate(() => window.sourceScene);
    const root = scene.hierarchy[0];
    const catalog = scene.presentations[root.path];
    expect(catalog.schema).toBe("sw-rs-presentation-v1");
    const physical = catalog.physical_materials[root.physical_material_index];
    expect(physical.classifications).toContain("Aluminum");
    const appearance = catalog.appearances[physical.default_appearance_index];
    expect(root.material_default_appearance_index).toBe(physical.default_appearance_index);
    expect(appearance.origin).toEqual({kind:"material_default", material_index:root.physical_material_index});
    expect(appearance.saved.shaders[0].attributes.name).toBe("black low gloss plastic");
    expect(physical.roughness).toBeUndefined();
    expect(catalog.appearance_bindings).toEqual([]);
    expect(catalog.coverage.appearance_bindings_decoded).toBe(false);
    expect(scene.materials[0].family).toBe("aluminum");
    expect(scene.materials[0].saved_surface.metallic).toBe(0);
  });

  test("native appearance catalog retains state scopes and cache properties without overriding renderer presets", async ({ page }) => {
    await page.addInitScript(() => {
      const NativeWorker = window.Worker;
      window.Worker = class extends NativeWorker {
        constructor(...args) {
          super(...args);
          this.addEventListener("message", ({data}) => {
            if (data.type === "scene") window.sourceScene = data.scene;
          });
        }
      };
    });
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures,"native-appearance.SLDPRT"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    const scene = await page.evaluate(() => window.sourceScene);
    const catalog = scene.presentations[scene.hierarchy[0].path];
    expect(catalog.appearances).toHaveLength(4);
    expect(catalog.appearance_bindings).toHaveLength(3);
    const document = catalog.appearance_bindings.find(b => b.layer === "document");
    expect(document.configuration).toEqual({kind:"named",name:"Default"});
    expect(document.display_state).toEqual({kind:"named",name:"Inspection"});
    expect(catalog.appearances[document.appearance_index].native.color_rgb).toEqual([34,68,102]);
    expect(catalog.display_states[0].key).toBe("<Default>_Inspection");
    const cache = catalog.appearance_bindings.filter(b => b.target.kind === "tessellation");
    expect(cache.map(b => b.layer).sort()).toEqual(["cached_body","cached_face"]);
    expect(cache[0].target.source_offset).toBe(cache[1].target.source_offset);
    expect(cache[0].target.source_stream).toBe("Contents/DisplayLists");
    expect(cache[0].display_state.kind).toBe("unknown");
    expect(catalog.coverage.cached_appearance_bindings_decoded).toBe(true);
    expect(catalog.coverage.appearance_bindings_decoded).toBe(false);
    expect(catalog.coverage.texture_mappings_decoded).toBe(false);
    expect(catalog.visual_diagnostics.some(d => d.code === "partial_visual_graph")).toBe(true);
    expect(scene.materials[0].family).toBe("aluminum");
    expect(scene.materials[0].saved_surface.name).toBe("black low gloss plastic");
  });
  test.beforeEach(async ({ page }) => {
    page.on("console", (message) => {
      if (message.type() === "error") console.error(message.text());
    });
    page.on("pageerror", (error) => console.error(error));
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

  test("appearance style recreates saved shader separately from physical family without decoding", { tag: "@hidpi" }, async ({ page }) => {
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
});
