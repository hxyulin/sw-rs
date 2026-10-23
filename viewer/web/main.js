// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import init, { WebApp } from "./pkg/sw_viewer.js";
import { SceneView } from "./scene.js";
const sidebar = document.querySelector("#sidebar"),
  input = document.querySelector("#file"),
  status = document.querySelector("#status"),
  empty = document.querySelector("#empty");
let app,
  scene,
  worker,
  busy = false,
  loaded = false,
  lastKey = "",
  request = 0,
  controls = {},
  currentFile = "",
  generation = 0,
  fontPromise,
  fontReady = false;
function fail(error) {
  status.textContent = String(error);
  status.dataset.state = "error";
  app?.set_status(String(error));
  invalidate();
}
function invalidate() {
  if (!request && !document.hidden) request = requestAnimationFrame(draw);
}
function draw(time) {
  request = 0;
  try {
    const rect = sidebar.getBoundingClientRect(),
      scale = Math.min(devicePixelRatio || 1, 2),
      w = Math.round(rect.width * scale),
      h = Math.round(rect.height * scale);
    if (sidebar.width !== w || sidebar.height !== h) {
      sidebar.width = w;
      sidebar.height = h;
      app.resize(w, h);
    }
    if (!app.draw(time, scale) || app.animating()) invalidate();
    poll();
  } catch (error) {
    status.textContent = String(error);
    status.dataset.state = "error";
  }
}
function poll() {
  controls = JSON.parse(app.controls());
  if (controls.open) input.click();
  if (controls.demo)
    fetch("./demo.zip")
      .then((r) => {
        if (!r.ok) throw Error("Demo unavailable; run make-fixtures.py");
        return r.blob();
      })
      .then((b) => load(new File([b], "Synthetic demo.zip")))
      .catch(fail);
  if (controls.fit) scene.fit();
  if (scene.wire !== controls.wire) {
    scene.wire = controls.wire;
    scene.draw();
  }
  const key = JSON.stringify([controls.path, controls.config, controls.hidden]);
  if (loaded && !busy && controls.path && key !== lastKey) {
    lastKey = key;
    busy = true;
    status.textContent = "Decoding " + controls.path + "…";
    worker.postMessage({ type: "scene", key, ...controls });
  }
}
function ensureFont(text) {
  if (!/[\u3400-\u9fff]/u.test(text) || fontPromise) return;
  fontPromise = fetch("./fonts/SWViewerCJK.ttf")
    .then((r) => {
      if (!r.ok) throw Error("CJK font asset missing");
      return r.arrayBuffer();
    })
    .then((bytes) => {
      app.load_font(new Uint8Array(bytes));
      fontReady = true;
      invalidate();
    })
    .catch((e) => {
      fontPromise = null;
      console.warn(e);
    });
}
function startWorker() {
  worker?.terminate();
  const active = (worker = new Worker("./worker.js", { type: "module" }));
  active.onmessage = ({ data }) => {
    if (worker !== active) return;
    busy = false;
    if (data.type === "loaded") {
      ensureFont(data.paths.join("\n"));
      loaded = true;
      app.set_files(JSON.stringify(data.paths));
      status.textContent = `${currentFile} · ${data.paths.length} documents`;
      status.dataset.state = "ready";
    } else if (data.type === "scene") {
      const current = JSON.stringify([
        controls.path,
        controls.config,
        controls.hidden,
      ]);
      if (data.key === current) {
        ensureFont(
          [
            ...data.scene.info,
            ...data.scene.configurations,
            ...data.scene.diagnostics,
          ].join("\n"),
        );
        app.show_scene(JSON.stringify(data.scene));
        scene.setMesh(data.vertices);
        empty.hidden = data.vertices.length > 0;
        empty.textContent = data.vertices.length
          ? ""
          : "No supported display mesh. See document diagnostics.";
        status.textContent = currentFile + " · " + data.scene.status;
        status.dataset.state = "ready";
      }
    } else {
      if (!loaded) {
        status.textContent = data.message;
      }
      fail(data.message);
    }
    invalidate();
  };
  active.onerror = (e) => {
    busy = false;
    fail(e.message);
  };
}
async function load(file) {
  if (!file) return;
  if (file.size > 128 * 1024 * 1024) {
    fail("Upload limit: 128 MiB.");
    return;
  }
  const token = ++generation;
  worker?.terminate();
  worker = null;
  loaded = false;
  busy = true;
  lastKey = "";
  currentFile = file.name;
  app.clear();
  scene.setMesh(new Float32Array());
  empty.hidden = false;
  empty.textContent = "Reading archive in a local worker…";
  status.textContent = "Reading " + file.name + "…";
  status.dataset.state = "loading";
  invalidate();
  try {
    const bytes = await file.arrayBuffer();
    if (token !== generation) return;
    startWorker();
    worker.postMessage({ type: "load", name: file.name, bytes }, [bytes]);
  } catch (error) {
    busy = false;
    fail(error);
  }
}
try {
  if (!navigator.gpu)
    throw Error(
      "RXUI requires WebGPU. Use a WebGPU-enabled browser on localhost or HTTPS.",
    );
  await init();
  app = await WebApp.create(sidebar);
  scene = new SceneView(document.querySelector("#scene"));
  const position = (e) => {
    const r = sidebar.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  };
  sidebar.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    sidebar.focus();
    sidebar.setPointerCapture(e.pointerId);
    app.pointer("down", ...position(e));
    invalidate();
    e.preventDefault();
  });
  sidebar.addEventListener("pointermove", (e) => {
    app.pointer("move", ...position(e));
    invalidate();
  });
  sidebar.addEventListener("pointerup", (e) => {
    app.pointer("up", ...position(e));
    if (sidebar.hasPointerCapture(e.pointerId))
      sidebar.releasePointerCapture(e.pointerId);
    poll();
    invalidate();
  });
  sidebar.addEventListener("keydown", (e) => {
    if (
      [
        "Tab",
        "Enter",
        " ",
        "Escape",
        "ArrowDown",
        "ArrowUp",
        "ArrowLeft",
        "ArrowRight",
      ].includes(e.key)
    ) {
      app.key(e.key, e.shiftKey, true);
      poll();
      invalidate();
      e.preventDefault();
    }
  });
  sidebar.addEventListener("keyup", (e) => {
    if (["Enter", " "].includes(e.key)) {
      app.key(e.key, e.shiftKey, false);
      poll();
      invalidate();
      e.preventDefault();
    }
  });
  sidebar.addEventListener("pointercancel", () => {
    app.pointer("cancel", 0, 0);
    invalidate();
  });
  sidebar.addEventListener(
    "wheel",
    (e) => {
      app.wheel(
        ...position(e),
        e.deltaY *
          (e.deltaMode === 1
            ? 20
            : e.deltaMode === 2
              ? sidebar.clientHeight
              : 1),
      );
      invalidate();
      e.preventDefault();
    },
    { passive: false },
  );
  window.addEventListener("blur", () => {
    app.pointer("cancel", 0, 0);
    invalidate();
  });
  input.addEventListener("change", () => {
    load(input.files[0]);
    input.value = "";
  });
  document.addEventListener("dragover", (e) => {
    e.preventDefault();
    document.body.classList.add("dragging");
  });
  document.addEventListener("dragleave", (e) => {
    if (!e.relatedTarget) document.body.classList.remove("dragging");
  });
  document.addEventListener("drop", (e) => {
    e.preventDefault();
    document.body.classList.remove("dragging");
    load(e.dataTransfer.files[0]);
  });
  document.querySelector("#cancel").onclick = () => {
    generation++;
    worker?.terminate();
    worker = null;
    loaded = false;
    busy = false;
    app.clear();
    app.set_status("Import cancelled. Open another archive.");
    scene.setMesh(new Float32Array());
    empty.hidden = false;
    empty.textContent = "Import cancelled";
    status.textContent = "Import cancelled";
    status.dataset.state = "ready";
    invalidate();
  };
  new ResizeObserver(invalidate).observe(sidebar);
  window.addEventListener("resize", () => {
    scene.draw();
    invalidate();
  });
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) {
      scene.draw();
      invalidate();
    } else {
      cancelAnimationFrame(request);
      request = 0;
    }
  });
  window.viewer = {
    get fontReady() {
      return fontReady;
    },
    snapshot: () => JSON.parse(app.snapshot()),
    get busy() {
      return busy;
    },
    get triangles() {
      return scene.data.length / 27;
    },
    load,
  };
  status.textContent = "Ready · files stay in this browser";
  status.dataset.state = "ready";
  invalidate();
} catch (error) {
  status.textContent = String(error);
  status.dataset.state = "error";
  console.error(error);
}
