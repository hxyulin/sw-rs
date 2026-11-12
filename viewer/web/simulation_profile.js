// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Profiles identify source occurrences, not transient renderer row numbers.
const fields = [
  "name",
  "configuration",
  "native_id",
  "reference_number",
  "source_sha256",
];
const signature = (node) =>
  Object.fromEntries(fields.map((key) => [key, node[key] ?? null]));

export function occurrenceSelector(nodes, id) {
  const chain = [];
  let node = nodes.find((n) => n.id === id);
  if (!node || node.parent === null)
    throw Error("Select a component occurrence.");
  while (node.parent !== null) {
    if (chain.length >= nodes.length)
      throw Error("Invalid occurrence ancestry.");
    const siblings = nodes.filter((n) => n.parent === node.parent);
    chain.unshift({
      ordinal: siblings.findIndex((n) => n.id === node.id),
      ...signature(node),
    });
    node = nodes.find((n) => n.id === node.parent);
    if (!node) throw Error("Missing occurrence parent.");
  }
  return { chain };
}

export function simulationProfile(nodes, groups, joints, linkage = null) {
  const root = nodes.find((n) => n.parent === null);
  if (!root?.source_sha256)
    throw Error("Source identity is unavailable for this scene.");
  return {
    schema: "sw-viewer-simulation-v1",
    kind: "authored",
    units: { length: "metres", angle: "degrees" },
    source: { sha256: root.source_sha256, configuration: root.configuration },
    groups: Object.fromEntries(
      ["yaw", "pitch", "grounded"].map((key) => [
        key,
        [...groups[key]].map((id) => occurrenceSelector(nodes, id)),
      ]),
    ),
    joints,
    ...(linkage
      ? {
          linkage: {
            ...linkage,
            groups: Object.fromEntries(
              ["crank", "coupler"].map((key) => [
                key,
                [...groups[key]].map((id) => occurrenceSelector(nodes, id)),
              ]),
            ),
          },
        }
      : {}),
  };
}

export function decodeSimulationProfile(profile, nodes) {
  const root = nodes.find((n) => n.parent === null);
  if (
    profile?.schema !== "sw-viewer-simulation-v1" ||
    profile.kind !== "authored"
  )
    throw Error("Unsupported simulation setup.");
  if (profile.units?.length !== "metres" || profile.units?.angle !== "degrees")
    throw Error("Unsupported simulation setup units.");
  if (
    !root?.source_sha256 ||
    profile.source?.sha256 !== root.source_sha256 ||
    profile.source.configuration !== root.configuration
  )
    throw Error(
      "Setup source or configuration does not match the loaded document.",
    );
  const groups = {};
  for (const key of ["yaw", "pitch", "grounded", "crank", "coupler"]) {
    const selectors =
      key === "crank" || key === "coupler"
        ? profile.linkage
          ? profile.linkage.groups?.[key]
          : []
        : profile.groups?.[key];
    if (!Array.isArray(selectors)) throw Error("Invalid simulation groups.");
    groups[key] = new Set(
      selectors.map((selector) => {
        if (!Array.isArray(selector?.chain) || !selector.chain.length)
          throw Error("Invalid occurrence selector.");
        let node = root;
        for (const step of selector.chain) {
          if (!Number.isSafeInteger(step?.ordinal) || step.ordinal < 0)
            throw Error("Invalid occurrence index.");
          node = nodes.filter((n) => n.parent === node.id)[step.ordinal];
          if (
            !node ||
            fields.some((field) => (node[field] ?? null) !== step[field])
          )
            throw Error("Setup occurrence identity does not match the scene.");
        }
        return node.id;
      }),
    );
  }
  const joints = {};
  for (const key of ["yaw", "pitch"]) {
    const joint = profile.joints?.[key];
    if (
      !joint ||
      ![joint.axis, joint.pivot].every(
        (v) => Array.isArray(v) && v.length === 3 && v.every(Number.isFinite),
      ) ||
      ![joint.lower, joint.upper, joint.value].every(Number.isFinite)
    )
      throw Error("Invalid simulation joint values.");
    const length = Math.hypot(...joint.axis);
    if (
      !Number.isFinite(length) ||
      length === 0 ||
      joint.pivot.some((v) => !Number.isFinite(v * 1000))
    )
      throw Error("Invalid simulation axis or pivot.");
    if (
      joint.lower > 0 ||
      joint.upper < 0 ||
      joint.lower >= joint.upper ||
      joint.value < joint.lower ||
      joint.value > joint.upper
    )
      throw Error("Invalid simulation travel limits.");
    joints[key] = {
      axis: [...joint.axis],
      pivot: [...joint.pivot],
      lower: joint.lower,
      upper: joint.upper,
      value: joint.value,
    };
  }
  let linkage = null;
  if (profile.linkage !== undefined) {
    const data = profile.linkage;
    if (
      data?.kind !== "parallelogram" ||
      ![data.crank_pivot, data.crank_point].every(
        (v) =>
          Array.isArray(v) &&
          v.length === 3 &&
          v.every((x) => Number.isFinite(x) && Number.isFinite(x * 1000)),
      )
    )
      throw Error("Invalid pitch linkage.");
    linkage = {
      kind: "parallelogram",
      crank_pivot: [...data.crank_pivot],
      crank_point: [...data.crank_point],
    };
  }
  return { groups, joints, linkage };
}
