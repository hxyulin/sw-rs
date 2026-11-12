// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Explicit simulation groups replace saved constraints only for assigned geometry.
import {
  simulationProfile,
  decodeSimulationProfile,
} from "./simulation_profile.js";
export function simulationRanges(
  nodes,
  yaw,
  pitch,
  grounded,
  crank = new Set(),
  coupler = new Set(),
) {
  const selected = (ids) =>
    nodes.filter((n) => ids.has(n.id) && n.vertex_count > 0);
  const y = selected(yaw),
    p = selected(pitch),
    g = selected(grounded),
    c = selected(crank),
    l = selected(coupler);
  const boundaries = [
    ...new Set(
      [...y, ...p, ...g, ...c, ...l].flatMap((n) => [
        n.first_vertex,
        n.first_vertex + n.vertex_count,
      ]),
    ),
  ].sort((a, b) => a - b);
  const contains = (group, vertex) =>
    group.some(
      (n) =>
        vertex >= n.first_vertex && vertex < n.first_vertex + n.vertex_count,
    );
  const ranges = [];
  for (let i = 1; i < boundaries.length; i++) {
    const start = boundaries[i - 1],
      isPitch = contains(p, start);
    const linkage = contains(c, start)
      ? "crank"
      : contains(l, start)
        ? "coupler"
        : null;
    if (contains(g, start) || (!isPitch && !linkage && !contains(y, start)))
      continue;
    ranges.push({
      first_vertex: start,
      vertex_count: boundaries[i] - start,
      pitch: isPitch,
      ...(linkage ? { linkage } : {}),
    });
  }
  return ranges;
}

export class SimulationPanel {
  constructor(scene, transform, linkageTransform) {
    this.scene = scene;
    this.transform = transform;
    this.linkageTransform = linkageTransform;
    this.panel = document.querySelector("#simulation-panel");
    this.enable = document.querySelector("#simulation-enable");
    this.status = document.querySelector("#simulation-status");
    this.groups = Object.fromEntries(
      ["yaw", "pitch", "grounded", "crank", "coupler"].map((key) => [
        key,
        new Set(),
      ]),
    );
    this.joints = {};
    for (const key of ["yaw", "pitch"]) this.buildJoint(key);
    this.buildLinkage();
    this.importGeneration = 0;
    this.setupName = "";
    this.qualification = "";
    const input = document.querySelector("#simulation-file");
    document.querySelector("#simulation-import").onclick = () => input.click();
    input.onchange = async () => {
      const file = input.files[0],
        token = ++this.importGeneration;
      input.value = "";
      if (!file) return;
      try {
        const profile = JSON.parse(await file.text());
        if (token !== this.importGeneration) return;
        this.loadProfile(profile);
        this.status.textContent =
          "Authored setup imported. Enable simulation overrides to apply it.";
      } catch (error) {
        if (token !== this.importGeneration) return;
        this.status.textContent = error.message;
        this.status.dataset.state = "error";
      }
    };
    document.querySelector("#simulation-export").onclick = () => {
      try {
        const profile = this.profile();
        decodeSimulationProfile(profile, this.nodes);
        const url = URL.createObjectURL(
          new Blob([JSON.stringify(profile, null, 2)], {
            type: "application/json",
          }),
        );
        const link = document.createElement("a");
        link.href = url;
        link.download = "simulation-setup.json";
        link.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
      } catch (error) {
        this.status.textContent = error.message;
        this.status.dataset.state = "error";
      }
    };
    for (const key of Object.keys(this.groups)) {
      document.querySelector(`#simulation-add-${key}`).onclick = () => {
        if (!this.selected?.vertex_count || this.selected.parent === null)
          return;
        this.scene.clearMotion();
        this.scene.clearSimulation();
        this.groups[key].add(this.selected.id);
        this.markEdited();
        this.renderGroups();
        this.apply();
      };
    }
    this.enable.onchange = () => {
      this.scene.clearMotion();
      this.scene.clearSimulation();
      this.scene.simulationActive = this.enable.checked;
      this.onActiveChange?.();
      this.apply();
    };
    document.querySelector("#simulation-reset").onclick = () => {
      for (const joint of Object.values(this.joints))
        joint.value.value = joint.range.value = "0";
      this.scene.clearSimulation();
      this.apply();
    };
  }
  buildLinkage() {
    this.linkageEnabled = document.querySelector("#simulation-linkage-enable");
    this.linkageInputs = {};
    const container = document.querySelector("#simulation-linkage-inputs");
    for (const [key, title] of [
      ["crank_pivot", "Crank pivot"],
      ["crank_point", "Crank connection"],
    ]) {
      this.linkageInputs[key] = ["X", "Y", "Z"].map((axis, i) => {
        const label = document.createElement("label"),
          input = document.createElement("input");
        label.textContent = `${title} ${axis} (mm)`;
        input.type = "number";
        input.step = "any";
        input.value = 0;
        input.id = `simulation-${key}-${i}`;
        input.setAttribute("aria-label", label.textContent);
        input.oninput = () => {
          this.markEdited();
          this.apply();
        };
        label.append(input);
        container.append(label);
        return input;
      });
    }
    this.linkageEnabled.onchange = () => {
      this.markEdited();
      this.scene.clearSimulation();
      this.apply();
    };
  }
  linkageData() {
    return this.linkageEnabled.checked
      ? {
          kind: "parallelogram",
          ...Object.fromEntries(
            Object.entries(this.linkageInputs).map(([key, inputs]) => [
              key,
              inputs.map((v) => v.valueAsNumber / 1000),
            ]),
          ),
        }
      : null;
  }
  linkageMatrices(data, pitch) {
    const matrices = this.linkageTransform(
      JSON.stringify({
        linkage: {
          axis: pitch.axis,
          crank_pivot: data.crank_pivot,
          crank_point: data.crank_point,
          lower: (pitch.lower * Math.PI) / 180,
          upper: (pitch.upper * Math.PI) / 180,
        },
        coordinate: (pitch.value * Math.PI) / 180,
      }),
    );
    return { crank: matrices.slice(0, 16), coupler: matrices.slice(16, 32) };
  }
  buildJoint(key) {
    const title = key === "yaw" ? "Yaw" : "Pitch";
    const fieldset = document.createElement("fieldset");
    const legend = document.createElement("legend");
    legend.textContent = title;
    fieldset.append(legend);
    const input = (labelText, id, value, type = "number") => {
      const label = document.createElement("label");
      label.textContent = labelText;
      const element = document.createElement("input");
      element.id = id;
      element.type = type;
      element.value = value;
      element.step = "any";
      element.setAttribute("aria-label", `${title} ${labelText}`);
      label.append(element);
      fieldset.append(label);
      return element;
    };
    const joint = { axis: [], pivot: [] };
    for (const [i, axis] of ["X", "Y", "Z"].entries())
      joint.axis.push(
        input(
          `axis ${axis}`,
          `simulation-${key}-axis-${i}`,
          i === (key === "yaw" ? 1 : 0) ? 1 : 0,
        ),
      );
    for (const [i, axis] of ["X", "Y", "Z"].entries())
      joint.pivot.push(
        input(`pivot ${axis} (mm)`, `simulation-${key}-pivot-${i}`, 0),
      );
    const origin = document.createElement("button");
    origin.textContent = "Use selected origin";
    origin.setAttribute("aria-label", `${title} use selected origin`);
    origin.onclick = () => {
      if (!this.selected?.world_transform) return;
      joint.pivot.forEach((v, i) => {
        v.value = this.selected.world_transform[12 + i] * 1000;
      });
      this.markEdited();
      this.apply();
    };
    fieldset.append(origin);
    joint.lower = input(
      "lower limit (degrees)",
      `simulation-${key}-lower`,
      -180,
    );
    joint.upper = input(
      "upper limit (degrees)",
      `simulation-${key}-upper`,
      180,
    );
    joint.value = input("angle (degrees)", `simulation-${key}-value`, 0);
    joint.range = input("motion", `simulation-${key}-range`, 0, "range");
    joint.range.min = -180;
    joint.range.max = 180;
    joint.range.oninput = () => {
      joint.value.value = joint.range.value;
      this.apply();
    };
    for (const element of [
      ...joint.axis,
      ...joint.pivot,
      joint.lower,
      joint.upper,
      joint.value,
    ])
      element.oninput = () => {
        if (element !== joint.value) this.markEdited();
        this.apply();
      };
    const add = document.createElement("button");
    add.id = `simulation-add-${key}`;
    add.textContent = "Add selected";
    add.setAttribute("aria-label", `Add selected to ${title}`);
    fieldset.append(add);
    const list = document.createElement("ul");
    list.id = `simulation-members-${key}`;
    fieldset.append(list);
    document.querySelector("#simulation-joints").append(fieldset);
    this.joints[key] = joint;
  }
  reset(metadata = {}) {
    this.importGeneration++;
    this.scene.clearSimulation();
    this.scene.simulationActive = false;
    this.nodes = metadata.hierarchy || [];
    this.setupName = "";
    this.qualification = "";
    document.querySelector("#simulation-setup-name").hidden = true;
    document.querySelector("#simulation-setup-note").hidden = true;
    this.selected = null;
    this.enable.checked = false;
    this.linkageEnabled.checked = false;
    for (const inputs of Object.values(this.linkageInputs))
      for (const v of inputs) v.value = 0;
    for (const set of Object.values(this.groups)) set.clear();
    for (const [key, joint] of Object.entries(this.joints)) {
      joint.axis.forEach((v, i) => {
        v.value = i === (key === "yaw" ? 1 : 0) ? 1 : 0;
      });
      joint.pivot.forEach((v) => {
        v.value = 0;
      });
      joint.lower.value = -180;
      joint.upper.value = 180;
      joint.range.min = -180;
      joint.range.max = 180;
      joint.value.value = joint.range.value = 0;
    }
    this.panel.hidden = !this.nodes.length;
    this.renderGroups();
    this.select(null);
    this.apply();
  }
  profile() {
    const joints = Object.fromEntries(
      Object.entries(this.joints).map(([key, j]) => [key, this.jointValues(j)]),
    );
    return {
      ...simulationProfile(this.nodes, this.groups, joints, this.linkageData()),
      name: this.setupName,
      qualification: this.qualification,
    };
  }
  jointValues(joint) {
    return {
      axis: joint.axis.map((v) => v.valueAsNumber),
      pivot: joint.pivot.map((v) => v.valueAsNumber / 1000),
      lower: joint.lower.valueAsNumber,
      upper: joint.upper.valueAsNumber,
      value: joint.value.valueAsNumber,
    };
  }
  markEdited() {
    if (!this.setupName && !this.qualification) return;
    if (!this.setupName.endsWith(" (edited)"))
      this.setupName = (this.setupName || "Imported setup") + " (edited)";
    this.qualification =
      "Edited authored setup. Original calibration claims no longer apply.";
    document.querySelector("#simulation-setup-name").textContent =
      this.setupName;
    document.querySelector("#simulation-setup-name").hidden = false;
    document.querySelector("#simulation-setup-note").textContent =
      this.qualification;
    document.querySelector("#simulation-setup-note").hidden = false;
  }
  loadProfile(profile) {
    const decoded = decodeSimulationProfile(profile, this.nodes);
    // Validate through the shared Rust evaluator before altering an active setup.
    for (const joint of Object.values(decoded.joints))
      this.transform(
        JSON.stringify({
          joint: {
            kind: "hinge",
            axis: joint.axis,
            pivot: joint.pivot,
            lower: (joint.lower * Math.PI) / 180,
            upper: (joint.upper * Math.PI) / 180,
          },
          coordinate: (joint.value * Math.PI) / 180,
        }),
      );
    if (decoded.linkage)
      this.linkageMatrices(decoded.linkage, decoded.joints.pitch);
    this.scene.clearMotion();
    this.scene.clearSimulation();
    this.enable.checked = false;
    this.scene.simulationActive = false;
    this.groups = decoded.groups;
    this.linkageEnabled.checked = !!decoded.linkage;
    for (const [key, inputs] of Object.entries(this.linkageInputs))
      inputs.forEach((v, i) => {
        v.value = (decoded.linkage?.[key][i] || 0) * 1000;
      });
    this.setupName = typeof profile.name === "string" ? profile.name : "";
    this.qualification =
      typeof profile.qualification === "string" ? profile.qualification : "";
    const name = document.querySelector("#simulation-setup-name"),
      note = document.querySelector("#simulation-setup-note");
    name.textContent = this.setupName;
    name.hidden = !this.setupName;
    note.textContent = this.qualification;
    note.hidden = !this.qualification;
    for (const [key, data] of Object.entries(decoded.joints)) {
      const joint = this.joints[key];
      joint.axis.forEach((v, i) => {
        v.value = data.axis[i];
      });
      joint.pivot.forEach((v, i) => {
        v.value = data.pivot[i] * 1000;
      });
      joint.lower.value = data.lower;
      joint.upper.value = data.upper;
      joint.range.min = data.lower;
      joint.range.max = data.upper;
      joint.value.value = joint.range.value = data.value;
    }
    this.renderGroups();
    this.onActiveChange?.();
    this.apply();
  }
  select(node) {
    this.selected = node;
    for (const key of Object.keys(this.groups))
      document.querySelector(`#simulation-add-${key}`).disabled =
        !node?.vertex_count || node.parent === null;
  }
  renderGroups() {
    for (const [key, ids] of Object.entries(this.groups)) {
      const list = document.querySelector(`#simulation-members-${key}`);
      list.replaceChildren();
      for (const id of ids) {
        const node = this.nodes.find((n) => n.id === id),
          row = document.createElement("li"),
          remove = document.createElement("button");
        row.append(document.createTextNode(node.name + " "));
        remove.textContent = "Remove";
        remove.setAttribute("aria-label", `Remove ${node.name} from ${key}`);
        remove.onclick = () => {
          this.scene.clearSimulation();
          ids.delete(id);
          this.markEdited();
          this.renderGroups();
          this.apply();
        };
        row.append(remove);
        list.append(row);
      }
    }
  }
  matrix(joint) {
    const lower = joint.lower.valueAsNumber,
      upper = joint.upper.valueAsNumber,
      value = joint.value.valueAsNumber;
    if (
      ![lower, upper, value].every(Number.isFinite) ||
      lower > 0 ||
      upper < 0 ||
      lower >= upper
    )
      throw Error(
        "Limits must be finite and include zero, with lower less than upper.",
      );
    const matrix = this.transform(
      JSON.stringify({
        joint: {
          kind: "hinge",
          axis: joint.axis.map((v) => v.valueAsNumber),
          pivot: joint.pivot.map((v) => v.valueAsNumber / 1000),
          lower: (lower * Math.PI) / 180,
          upper: (upper * Math.PI) / 180,
        },
        coordinate: (value * Math.PI) / 180,
      }),
    );
    joint.range.min = lower;
    joint.range.max = upper;
    joint.range.value = value;
    return matrix;
  }
  apply() {
    try {
      if (!this.enable.checked) {
        this.status.textContent =
          "Assign driven components, then enable. Unassigned components stay in their saved pose.";
        this.status.dataset.state = "ready";
        return;
      }
      const yaw = this.matrix(this.joints.yaw),
        pitch = this.matrix(this.joints.pitch);
      const linkage = this.linkageData();
      const followers = linkage
        ? this.linkageMatrices(linkage, this.jointValues(this.joints.pitch))
        : null;
      const ranges = simulationRanges(
        this.nodes,
        this.groups.yaw,
        this.groups.pitch,
        this.groups.grounded,
        linkage ? this.groups.crank : new Set(),
        linkage ? this.groups.coupler : new Set(),
      );
      if (!ranges.length)
        throw Error(
          "Assign at least one rendered component to Yaw or Pitch outside the grounded group.",
        );
      if (Object.values(this.joints).every((j) => j.value.valueAsNumber === 0))
        this.scene.clearSimulation();
      else this.scene.previewSimulation(ranges, yaw, pitch, followers);
      this.status.textContent =
        "Simulation overrides active. Pitch follows yaw; grounded and unassigned geometry stays fixed. Saved mates and collisions are not enforced for driven groups.";
      this.status.dataset.state = "ready";
    } catch (error) {
      this.scene.clearSimulation();
      this.status.textContent = error.message;
      this.status.dataset.state = "error";
    }
  }
}
