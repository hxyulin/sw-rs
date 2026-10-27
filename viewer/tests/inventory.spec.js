// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready } from "./support/viewer.js";

test.describe("inventory", () => {
  test("assembly metadata exports partial mate relations without changing saved placement", async ({ page }) => {
    await page.addInitScript(() => {
      const NativeWorker = window.Worker;
      window.Worker = class extends NativeWorker {
        constructor(...args) {
          super(...args);
          this.addEventListener("message", ({data}) => {
            if (data.type === "scene") window.mateScene = data.scene;
          });
        }
      };
    });
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures,"mates.zip"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(24);
    const scene = await page.evaluate(() => window.mateScene);
    const mates = scene.metadata.mates.configurations[0];
    expect(mates.configuration_index).toBe(0);
    expect(mates.declared_count).toBe(2);
    expect(mates.inventory_matches_declared_count).toBe(true);
    expect(mates.entity_ownership_decoded).toBe(false);
    expect(mates.parameters_decoded).toBe(false);
    expect(mates.records[0].kind).toBe("width");
    expect(mates.records[0].reference_candidates).toHaveLength(4);
    expect(mates.records[1].kind).toBeNull();
    expect(mates.records[1].class_reference).toBe(0x80c1);
    expect(scene.info.join("\n")).toContain("Saved mates: 2/2 headers · partial relationships");
    expect(scene.occurrences).toBe(2);
    expect(scene.missing_references).toBe(0);
  });

  test("source inventory inspects unknown XML and binary candidates locally without scene decoding", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures,"inventory.SLDPRT"));
    await expect.poll(() => page.evaluate(() => window.viewer.triangles)).toBe(12);
    const before = await page.evaluate(() => window.viewer.snapshot().state);
    await page.evaluate(() => {
      window.inspectionSceneRequests=0;
      const send=Worker.prototype.postMessage;
      Worker.prototype.postMessage=function(data,...args) {
        if(data.type==="scene") window.inspectionSceneRequests++;
        return send.call(this,data,...args);
      };
    });
    await page.getByRole("button",{name:"Inspect source…"}).click();
    const dialog=page.getByRole("dialog",{name:/Source inventory/});
    await expect(dialog).toBeVisible();
    await expect(page.locator("#inventory-summary")).toContainText("modern_inline");
    await expect(page.locator("#inventory-summary")).toContainText("unclassified bytes");
    await page.getByLabel("Filter source streams").fill("Features");
    await expect(page.locator("#inventory-streams button")).toHaveCount(1);
    await page.locator("#inventory-streams button").click();
    await expect(page.locator("#inventory-detail")).toContainText('"structure_status": "xml"');
    const xml=JSON.parse(await page.locator("#inventory-detail").textContent());
    expect(xml.stream.semantic_status).toBe("not_evaluated");
    expect(xml.stream.sha256).toBe(xml.source_record.decoded_sha256);
    expect(xml.source_record.status).toBe("selected");
    expect(xml.xml_elements[2].attributes.some(a=>a.namespace==="urn:extra"&&a.value==="keep")).toBe(true);
    expect(xml.xml_text[0].value).toBe("A&B");
    expect(xml.xml_references[0].candidate_elements).toEqual([1]);
    expect(xml.native_object_graph_decoded).toBe(false);
    await page.getByLabel("Filter source streams").fill("Unicode");
    await page.locator("#inventory-streams button").click();
    await expect(page.locator("#inventory-detail")).toContainText('"xml_encoding": "utf16_le"');
    const unicode=JSON.parse(await page.locator("#inventory-detail").textContent());
    expect(unicode.xml_text[0].value).toBe("纹理😀");
    expect(unicode.xml_elements[0].span.offset).toBe(2);
    await page.getByLabel("Filter source streams").fill("Mystery/Unknown");
    await page.locator("#inventory-streams button").click();
    await expect(page.locator("#inventory-detail")).toContainText('"structure_status": "binary_candidates"');
    const binary=JSON.parse(await page.locator("#inventory-detail").textContent());
    expect(binary.class_declarations[0].status).toBe("candidate");
    expect(binary.framed_strings[0].value).toContain("<img");
    await expect(dialog.locator("img")).toHaveCount(0);
    await page.getByLabel("Filter source streams").fill("Mystery/Many");
    await page.locator("#inventory-streams button").click();
    await expect(page.locator("#inventory-page")).toContainText("of 180");
    expect(JSON.parse(await page.locator("#inventory-detail").textContent()).framed_strings).toHaveLength(100);
    await page.getByRole("button",{name:"Next records"}).click();
    await expect(page.locator("#inventory-page")).toContainText("101–180");
    expect(JSON.parse(await page.locator("#inventory-detail").textContent()).framed_strings[0].value).toBe("entry100");
    await expect(page.getByRole("button",{name:"Next records"})).toBeDisabled();
    await page.getByRole("button",{name:"Previous records"}).click();
    await expect(page.locator("#inventory-page")).toContainText("1–100");
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await expect(page.getByRole("button",{name:"Inspect source…"})).toBeFocused();
    expect(await page.evaluate(()=>window.inspectionSceneRequests)).toBe(0);
    expect(await page.evaluate(()=>window.viewer.snapshot().state)).toEqual(before);
  });

  test("malformed OLE retains an opaque source inventory and import resets the inspector", async ({ page }) => {
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures,"legacy.SLDPRT"));
    await expect(page.getByRole("button",{name:"Inspect source…"})).toBeEnabled();
    await page.getByRole("button",{name:"Inspect source…"}).click();
    await expect(page.locator("#inventory-summary")).toContainText("legacy_ole2");
    await expect(page.locator("#inventory-status")).toContainText("Invalid CFB file");
    await expect(page.locator("#inventory-streams button")).toHaveCount(0);
    await page.locator("#file").setInputFiles(path.join(fixtures,"single.SLDPRT"));
    await expect(page.getByRole("dialog",{name:/Source inventory/})).not.toBeVisible();
    await expect.poll(()=>page.evaluate(()=>window.viewer.triangles)).toBe(12);
    await page.getByRole("button",{name:"Inspect source…"}).click();
    await expect(page.locator("#inventory-title")).toContainText("single.SLDPRT");
    await expect(page.locator("#inventory-summary")).toContainText("modern_inline");
    await page.getByRole("button",{name:"Close source inspector"}).click();
  });
});
