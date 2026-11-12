// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { test, expect } from "@playwright/test";
import fs from "node:fs/promises";
import path from "node:path";
import { fixtures, ready } from "./support/viewer.js";
import {
  simulationProfile,
  decodeSimulationProfile,
} from "../web/simulation_profile.js";

const joint = {
  axis: [0, 1, 0],
  pivot: [0, 0, 0],
  lower: -90,
  upper: 90,
  value: 30,
};
const nodes = [
  {
    id: 0,
    parent: null,
    configuration: "Default",
    source_sha256: "a".repeat(64),
  },
  {
    id: 1,
    parent: 0,
    name: "Repeated",
    configuration: "Default",
    native_id: 31,
    reference_number: 1,
    source_sha256: "b".repeat(64),
  },
  {
    id: 2,
    parent: 0,
    name: "Repeated",
    configuration: "Default",
    native_id: 32,
    reference_number: 2,
    source_sha256: "b".repeat(64),
  },
];
test("simulation profiles bind exact sources and occurrences independently of row IDs", () => {
  const profile = simulationProfile(
    nodes,
    { yaw: new Set([2]), pitch: new Set(), grounded: new Set([1]) },
    { yaw: joint, pitch: joint },
  );
  const remapped = nodes.map((n) => ({
    ...n,
    id: n.id === 0 ? 0 : n.id + 100,
  }));
  const decoded = decodeSimulationProfile(profile, remapped);
  expect([...decoded.groups.yaw]).toEqual([102]);
  expect([...decoded.groups.grounded]).toEqual([101]);
  const changed = structuredClone(profile);
  changed.source.sha256 = "c".repeat(64);
  expect(() => decodeSimulationProfile(changed, nodes)).toThrow(
    "source or configuration",
  );
  const stale = structuredClone(nodes);
  stale[2].source_sha256 = "c".repeat(64);
  expect(() => decodeSimulationProfile(profile, stale)).toThrow(
    "occurrence identity",
  );
  const unknown = structuredClone(profile);
  unknown.joints.yaw.axis = [0, 0, 0];
  expect(() => decodeSimulationProfile(unknown, nodes)).toThrow(
    "axis or pivot",
  );
  const outside = structuredClone(profile);
  outside.joints.yaw.value = 91;
  expect(() => decodeSimulationProfile(outside, nodes)).toThrow(
    "travel limits",
  );
});

test("authored setup export and import preserve groups and reject a wrong source atomically", async ({
  page,
}, testInfo) => {
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(() => {
    const Original = window.Worker;
    window.Worker = class extends Original {
      constructor(...args) {
        super(...args);
        this.addEventListener("message", ({ data }) => {
          if (data.type === "scene") window.profileMesh = data.vertices;
        });
      }
    };
  });
  await ready(page);
  await page.locator("#file").setInputFiles(path.join(fixtures, "demo.zip"));
  await expect
    .poll(() => page.evaluate(() => window.viewer.triangles))
    .toBe(48);
  await page.locator("#simulation-panel > summary").click();
  await page
    .locator(
      "#occurrence-tree > [role=treeitem] > [role=group] > [role=treeitem]",
    )
    .first()
    .locator(":scope > .node-row .node-name")
    .click();
  await page
    .getByRole("button", { name: "Add selected to Yaw", exact: true })
    .click();
  await page.getByLabel("Yaw angle (degrees)", { exact: true }).fill("30");
  await page.getByLabel("Enable simulation overrides", { exact: true }).check();
  const moved = await page.evaluate(() => Array.from(window.profileMesh));
  const downloadEvent = page.waitForEvent("download");
  await page
    .getByRole("button", { name: "Export setup…", exact: true })
    .click();
  const download = await downloadEvent,
    file = testInfo.outputPath("setup.json");
  await download.saveAs(file);
  const profile = JSON.parse(await fs.readFile(file, "utf8"));
  expect(profile.schema).toBe("sw-viewer-simulation-v1");
  expect(profile.source.sha256).toMatch(/^[0-9a-f]{64}$/);
  expect(profile.groups.yaw).toHaveLength(1);
  const incorrect = structuredClone(profile);
  incorrect.source.sha256 = "0".repeat(64);
  await page.locator("#simulation-file").setInputFiles({
    name: "wrong.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(incorrect)),
  });
  await expect(page.locator("#simulation-status")).toContainText(
    "does not match",
  );
  expect(await page.evaluate(() => Array.from(window.profileMesh))).toEqual(
    moved,
  );
  await expect(
    page.getByLabel("Enable simulation overrides", { exact: true }),
  ).toBeChecked();
  profile.name = "Authored test setup";
  profile.qualification = "User-defined joint. Native mates are not solved.";
  await page.locator("#simulation-file").setInputFiles({
    name: "valid.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(profile)),
  });
  await expect(page.locator("#simulation-status")).toContainText(
    "Authored setup imported",
  );
  await expect(page.locator("#simulation-setup-name")).toHaveText(profile.name);
  await expect(
    page.getByLabel("Enable simulation overrides", { exact: true }),
  ).not.toBeChecked();
  await expect(page.locator("#simulation-members-yaw li")).toHaveCount(1);
  await page.getByLabel("Enable simulation overrides", { exact: true }).check();
  expect(await page.evaluate(() => Array.from(window.profileMesh))).toEqual(
    moved,
  );
  await page.getByLabel("Yaw angle (degrees)", { exact: true }).fill("20");
  await expect(page.locator("#simulation-setup-name")).toHaveText(profile.name);
  await page.getByLabel("Yaw pivot X (mm)", { exact: true }).fill("1");
  await expect(page.locator("#simulation-setup-name")).toHaveText(
    profile.name + " (edited)",
  );
  await expect(page.locator("#simulation-setup-note")).toContainText(
    "Original calibration claims no longer apply",
  );
  expect(errors).toEqual([]);
});

test("pitch parallelogram follows yaw, preserves coupler orientation and resets exactly", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.addInitScript(() => {
    const Original = window.Worker;
    window.Worker = class extends Original {
      constructor(...args) {
        super(...args);
        this.addEventListener("message", ({ data }) => {
          if (data.type === "scene") {
            window.linkageNodes = data.scene.hierarchy;
            window.linkageMesh = data.vertices;
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
  const profile = await page.evaluate(async () => {
    const { simulationProfile } = await import("./simulation_profile.js");
    const nodes = window.linkageNodes,
      children = nodes.filter((n) => n.parent === 0);
    window.linkageSaved = window.linkageMesh.slice();
    const yaw = {
      axis: [0, 1, 0],
      pivot: [0, 0, 0],
      lower: -180,
      upper: 180,
      value: 90,
    };
    const pitch = {
      axis: [0, 0, 1],
      pivot: [-1, 0, 0],
      lower: -180,
      upper: 180,
      value: 90,
    };
    return simulationProfile(
      nodes,
      {
        yaw: new Set(),
        pitch: new Set([children[0].id]),
        grounded: new Set(),
        crank: new Set([children[1].id]),
        coupler: new Set([children[2].id]),
      },
      { yaw, pitch },
      { kind: "parallelogram", crank_pivot: [0, 0, 0], crank_point: [1, 0, 0] },
    );
  });
  await page.locator("#simulation-panel > summary").click();
  await page
    .locator("#simulation-file")
    .setInputFiles({
      name: "linkage.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify(profile)),
    });
  await expect(page.locator("#simulation-status")).toContainText(
    "Authored setup imported",
  );
  await page.getByLabel("Enable simulation overrides", { exact: true }).check();
  await expect(page.locator("#simulation-status")).toHaveAttribute(
    "data-state",
    "ready",
  );
  const result = await page.evaluate(() => {
    let maxError = 0,
      unassignedUnchanged = true;
    const nodes = window.linkageNodes.filter((n) => n.parent === 0);
    for (let vertex = 0; vertex < window.linkageMesh.length / 9; vertex++) {
      const group = nodes.findIndex(
          (n) =>
            vertex >= n.first_vertex &&
            vertex < n.first_vertex + n.vertex_count,
        ),
        at = vertex * 9;
      if (group > 2) {
        unassignedUnchanged &&= window.linkageSaved
          .slice(at, at + 9)
          .every((v, i) => window.linkageMesh[at + i] === v);
        continue;
      }
      for (const offset of [0, 3]) {
        const [x, y, z] = window.linkageSaved.slice(
          at + offset,
          at + offset + 3,
        );
        const expected =
          offset === 0
            ? group === 0
              ? [z, x + 1, y + 1]
              : group === 1
                ? [z, x, y]
                : [z, y + 1, 1 - x]
            : group === 2
              ? [z, y, -x]
              : [z, x, y];
        for (let i = 0; i < 3; i++)
          maxError = Math.max(
            maxError,
            Math.abs(expected[i] - window.linkageMesh[at + offset + i]),
          );
      }
    }
    return { maxError, unassignedUnchanged };
  });
  expect(result.maxError).toBeLessThan(1e-6);
  expect(result.unassignedUnchanged).toBe(true);
  const downloadEvent = page.waitForEvent("download");
  await page.locator("#simulation-export").click();
  const download = await downloadEvent;
  const saved = JSON.parse(await fs.readFile(await download.path(), "utf8"));
  expect(saved.linkage).toEqual(profile.linkage);
  await page
    .getByRole("button", { name: "Reset simulation pose", exact: true })
    .click();
  expect(
    await page.evaluate(() =>
      window.linkageSaved.every((v, i) => v === window.linkageMesh[i]),
    ),
  ).toBe(true);
  const bad = structuredClone(profile);
  bad.linkage.crank_point = [null, 0, 0];
  await page
    .locator("#simulation-file")
    .setInputFiles({
      name: "bad.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify(bad)),
    });
  await expect(page.locator("#simulation-status")).toContainText(
    "Invalid pitch linkage",
  );
  expect(
    await page.evaluate(() =>
      window.linkageSaved.every((v, i) => v === window.linkageMesh[i]),
    ),
  ).toBe(true);
  expect(errors).toEqual([]);
});
