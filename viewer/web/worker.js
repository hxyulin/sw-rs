// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import init, { ArchiveProject } from "./pkg/sw_viewer.js";
let project;
const ready = init();
self.onmessage = async ({ data }) => {
  try {
    await ready;
    if (data.type === "load") {
      project?.free();
      project = new ArchiveProject(new Uint8Array(data.bytes), data.name);
      postMessage({ type: "loaded", paths: JSON.parse(project.paths()) });
    } else {
      if (!project) throw Error("No archive loaded.");
      const scene = JSON.parse(
        project.scene(data.path, data.config, data.hidden),
      );
      const vertices = project.take_vertices();
      postMessage({ type: "scene", key: data.key, scene, vertices }, [
        vertices.buffer,
      ]);
    }
  } catch (error) {
    postMessage({ type: "error", message: String(error), key: data.key });
  }
};
