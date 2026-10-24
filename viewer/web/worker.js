// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import init, { ArchiveProject } from "./pkg/sw_viewer.js";
let project;
const ready = init();
let lastProgress = 0;
self.reportImportProgress = (completed, total, path) => {
  const now = performance.now();
  if (now - lastProgress < 40 && completed !== total) return;
  lastProgress = now;
  postMessage({ type: "progress", completed, total,
    label: `Extracting ${completed}${total ? ` / ${total}` : ""} entries · ${path}` });
};
self.onmessage = async ({ data }) => {
  try {
    await ready;
    if (data.type === "load") {
      project?.free();
      postMessage({ type: "progress", label: "Decompressing and indexing local files…" });
      lastProgress = 0;
      project = new ArchiveProject(new Uint8Array(data.bytes), data.name);
      postMessage({ type: "loaded", paths: JSON.parse(project.paths()) });
    } else if (data.type === "inventory") {
      if (!project) throw Error("No archive loaded.");
      const report = JSON.parse(project.inventory(data.path, data.stream, data.offset));
      postMessage({type:"inventory", id:data.id, path:data.path, stream:data.stream, report});
    } else if (data.type === "inventory-close") {
      project?.clear_inventory();
    } else {
      if (!project) throw Error("No archive loaded.");
      postMessage({ type: "progress", label: `Decoding metadata and display meshes · ${data.path}` });
      const scene = JSON.parse(
        project.scene(data.path, data.config, data.hidden, data.configurationName),
      );
      const vertices = project.take_vertices();
      postMessage({ type: "scene", key: data.key, scene, vertices }, [
        vertices.buffer,
      ]);
    }
  } catch (error) {
    if (data.type === "inventory") {
      postMessage({type:"inventory", id:data.id, path:data.path, message:String(error)});
    } else {
      postMessage({ type: "error", message: String(error), key: data.key });
    }
  }
};
