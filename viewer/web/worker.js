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
      const textureUVs = project.take_texture_uvs();
      const sources = (scene.textures || []).map(t => project.texture_bytes(t.path));
      const textureImages = await Promise.all(sources.map(async (bytes, index) => {
        try {
          if (!bytes) throw Error("image bytes unavailable");
          const image = await createImageBitmap(new Blob([bytes]), { imageOrientation: "flipY" });
          if (image.width * image.height > 16777216) {
            image.close();
            throw Error("image exceeds 16 megapixels");
          }
          return image;
        } catch (error) {
          scene.diagnostics.push(`Bitmap unavailable: ${scene.textures[index].path}: ${error}`);
          return null;
        }
      }));
      postMessage({ type: "scene", key: data.key, scene, vertices, textureUVs, textureImages }, [
        vertices.buffer, textureUVs.buffer, ...textureImages.filter(Boolean),
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
