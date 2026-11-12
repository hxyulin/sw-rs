// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import path from "node:path";
import { fixtures, ready } from "./support/viewer.js";
import { simulationRanges } from "../web/simulation.js";

test("simulation partitions overlapping subtrees and gives grounding priority", () => {
  const nodes = [
    { id: 0, first_vertex: 0, vertex_count: 18 },
    { id: 1, first_vertex: 0, vertex_count: 12 },
    { id: 2, first_vertex: 3, vertex_count: 3 },
    { id: 3, first_vertex: 6, vertex_count: 3 },
  ];
  expect(
    simulationRanges(nodes, new Set([0, 1]), new Set([2]), new Set([3])),
  ).toEqual([
    { first_vertex: 0, vertex_count: 3, pitch: false },
    { first_vertex: 3, vertex_count: 3, pitch: true },
    { first_vertex: 9, vertex_count: 3, pitch: false },
    { first_vertex: 12, vertex_count: 6, pitch: false },
  ]);
  expect(
    simulationRanges(nodes, new Set(), new Set([2]), new Set([1])),
  ).toEqual([]);
});

test(
  "grouped yaw and pitch compose while grounded and unassigned parts stay fixed",
  { tag: "@hidpi" },
  async ({ page }) => {
    const errors = [];
    page.on("pageerror", (e) => errors.push(e.message));
    await page.addInitScript(() => {
      const Original = window.Worker;
      window.Worker = class extends Original {
        constructor(...args) {
          super(...args);
          this.addEventListener("message", ({ data }) => {
            if (data.type === "scene") {
              window.simulationMesh = data.vertices;
              window.simulationMetadata = data.scene;
            }
          });
        }
      };
    });
    await ready(page);
    await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(48);
    await page.evaluate(() => {
      window.simulationSaved = window.simulationMesh.slice();
    });
    const rows = page.locator(
      "#occurrence-tree > [role=treeitem] > [role=group] > [role=treeitem]",
    );
    await page.locator("#simulation-panel > summary").click();
    for (const [index, group] of [
      [0, "Yaw"],
      [1, "Pitch"],
      [2, "Grounded"],
    ]) {
      await rows.nth(index).locator(":scope > .node-row .node-name").click();
      await page
        .getByRole("button", { name: `Add selected to ${group}`, exact: true })
        .click();
    }
    await page.getByLabel("Yaw angle (degrees)", { exact: true }).fill("90");
    await page.getByLabel("Pitch angle (degrees)", { exact: true }).fill("90");
    expect(
      await page.evaluate(() =>
        window.simulationSaved.every((v, i) => window.simulationMesh[i] === v),
      ),
    ).toBe(true);
    await page
      .getByLabel("Enable simulation overrides", { exact: true })
      .check();
    await expect(page.locator("#simulation-status")).toHaveAttribute(
      "data-state",
      "ready",
    );
    await expect(page.locator("#motion-panel")).toBeHidden();
    const result = await page.evaluate(() => {
      const nodes = window.simulationMetadata.hierarchy.filter(
        (n) => n.parent === 0,
      );
      let maxError = 0,
        fixedUnchanged = true,
        moved = false;
      for (
        let vertex = 0;
        vertex < window.simulationMesh.length / 9;
        vertex++
      ) {
        const at = vertex * 9,
          src = window.simulationSaved.slice(at, at + 9);
        const yaw =
          vertex >= nodes[0].first_vertex &&
          vertex < nodes[0].first_vertex + nodes[0].vertex_count;
        const pitch =
          vertex >= nodes[1].first_vertex &&
          vertex < nodes[1].first_vertex + nodes[1].vertex_count;
        if (!yaw && !pitch) {
          fixedUnchanged &&= src.every(
            (v, i) => v === window.simulationMesh[at + i],
          );
          continue;
        }
        for (const offset of [0, 3]) {
          const [x, y, z] = src.slice(offset, offset + 3),
            expected = pitch ? [y, -z, -x] : [z, y, -x];
          expected.forEach((v, a) => {
            maxError = Math.max(
              maxError,
              Math.abs(v - window.simulationMesh[at + offset + a]),
            );
          });
        }
        moved ||= src.some((v, i) => v !== window.simulationMesh[at + i]);
      }
      return { maxError, fixedUnchanged, moved };
    });
    expect(result.maxError).toBeLessThan(1e-6);
    expect(result.fixedUnchanged).toBe(true);
    expect(result.moved).toBe(true);
    const changedMesh = await page.evaluate(() =>
      Array.from(window.simulationMesh),
    );
    await rows.nth(0).locator(":scope > .node-row .node-name").click();
    expect(
      await page.evaluate(() => Array.from(window.simulationMesh)),
    ).toEqual(changedMesh);
    await page
      .getByRole("button", { name: "Add selected to Grounded", exact: true })
      .click();
    expect(
      await page.evaluate(() => {
        const node = window.simulationMetadata.hierarchy.find(
          (n) => n.parent === 0,
        );
        return window.simulationSaved
          .slice(
            node.first_vertex * 9,
            (node.first_vertex + node.vertex_count) * 9,
          )
          .every(
            (v, i) => v === window.simulationMesh[node.first_vertex * 9 + i],
          );
      }),
    ).toBe(true);
    await page
      .locator("#simulation-members-grounded li")
      .last()
      .getByRole("button")
      .click();
    expect(
      await page.evaluate(() => Array.from(window.simulationMesh)),
    ).toEqual(changedMesh);
    await page.getByLabel("Yaw angle (degrees)", { exact: true }).fill("181");
    await expect(page.locator("#simulation-status")).toHaveAttribute(
      "data-state",
      "error",
    );
    expect(
      await page.evaluate(() =>
        window.simulationSaved.every((v, i) => window.simulationMesh[i] === v),
      ),
    ).toBe(true);
    await page
      .getByRole("button", { name: "Reset simulation pose", exact: true })
      .click();
    await expect(
      page.getByLabel("Yaw angle (degrees)", { exact: true }),
    ).toHaveValue("0");
    await expect(
      page.getByLabel("Pitch angle (degrees)", { exact: true }),
    ).toHaveValue("0");
    await page.getByLabel("Yaw angle (degrees)", { exact: true }).fill("45");
    await page
      .getByLabel("Enable simulation overrides", { exact: true })
      .uncheck();
    expect(
      await page.evaluate(() =>
        window.simulationSaved.every((v, i) => window.simulationMesh[i] === v),
      ),
    ).toBe(true);
    await expect(page.locator("#motion-panel")).toBeVisible();
    await page
      .locator("#file")
      .setInputFiles(path.join(fixtures, "single.SLDPRT"));
    await expect
      .poll(() => page.evaluate(() => window.viewer.triangles))
      .toBe(12);
    await expect(
      page.getByLabel("Enable simulation overrides", { exact: true }),
    ).not.toBeChecked();
    await expect(page.locator("#simulation-members-yaw li")).toHaveCount(0);
    await expect(page.locator("#simulation-members-pitch li")).toHaveCount(0);
    expect(errors).toEqual([]);
  },
);
