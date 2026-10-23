// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import init, { WebApp } from "./pkg/sw_viewer.js";
import { SceneView } from "./scene.js";
const sidebar = document.querySelector("#sidebar"),
  input = document.querySelector("#file"),
  status = document.querySelector("#status"),
  empty = document.querySelector("#empty"),
  loading = document.querySelector("#loading"),
  loadingLabel = document.querySelector("#loading-label"),
  loadingBar = document.querySelector("#loading-progress"),
  partsSearch = document.querySelector("#parts-search"),
  searchInput = document.querySelector("#part-search"),
  clearSearch = document.querySelector("#clear-search");
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
  fontReady = false,
  reader;
function progress(label, fraction) {
  loading.hidden = false;
  loadingLabel.textContent = label;
  if (fraction === undefined) loadingBar.removeAttribute("value");
  else loadingBar.value = Math.max(0, Math.min(1, fraction)) * 100;
  loadingBar.setAttribute("aria-valuetext", label);
  document.querySelector("main").setAttribute("aria-busy", "true");
}
function finishProgress() {
  loading.hidden = true;
  document.querySelector("main").setAttribute("aria-busy", "false");
}
function stopRead() {
  reader?.abort();
  reader = null;
}
function readFile(file, token) {
  // Keep caller-provided async sources usable, without a size preflight cap.
  if (!(file instanceof Blob)) return file.arrayBuffer();
  return new Promise((resolve, reject) => {
    const active = (reader = new FileReader());
    active.onprogress = (event) => {
      if (token !== generation) return;
      const fraction = event.lengthComputable && event.total
        ? event.loaded / event.total : undefined;
      const amount = (event.loaded / (1024 * 1024)).toFixed(1);
      const percentage = fraction === undefined ? "" : ` · ${Math.round(fraction * 100)}%`;
      progress(`Reading ${file.name} · ${amount} MiB${percentage}`, fraction);
    };
    active.onload = () => {
      if (reader === active) reader = null;
      resolve(active.result);
    };
    active.onerror = () => {
      if (reader === active) reader = null;
      reject(active.error);
    };
    active.onabort = () => reject(new DOMException("Import cancelled", "AbortError"));
    active.readAsArrayBuffer(file);
  });
}
function layoutSearch() {
  const [x, y, width, height] = app.search_bounds();
  if (!width || !height) { partsSearch.hidden = true; return; }
  const rect = sidebar.getBoundingClientRect();
  partsSearch.hidden = false;
  Object.assign(partsSearch.style, {
    left: `${rect.left + x}px`, top: `${rect.top + y}px`,
    width: `${width}px`, height: `${height}px`,
  });
}
function resetSearch() {
  searchInput.value = "";
  clearSearch.hidden = true;
}
function fail(error) {
  busy = false;
  finishProgress();
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
    layoutSearch();
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
    status.dataset.state = "loading";
    progress("Decoding display meshes · " + controls.path);
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
    if (data.type === "progress") {
      const fraction = data.total > 0 ? data.completed / data.total : undefined;
      progress(data.label, fraction);
      return;
    }
    busy = false;
    if (data.type === "loaded") {
      ensureFont(data.paths.join("\n"));
      loaded = true;
      app.set_files(JSON.stringify(data.paths));
      status.textContent = `${currentFile} · ${data.paths.length} documents`;
      progress("Archive indexed · preparing the selected document…");
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
        progress("Uploading mesh to the viewport…");
        scene.setMesh(data.vertices);
        finishProgress();
        empty.hidden = data.vertices.length > 0;
        empty.textContent = data.vertices.length
          ? ""
          : data.scene.missing_references > 0
            ? "No available display meshes · referenced files are missing. See document diagnostics."
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
  const token = ++generation;
  stopRead();
  worker?.terminate();
  worker = null;
  loaded = false;
  busy = true;
  lastKey = "";
  currentFile = file.name;
  app.clear();
  resetSearch();
  scene.setMesh(new Float32Array());
  empty.hidden = false;
  empty.textContent = "Reading archive in a local worker…";
  status.textContent = "Reading " + file.name + "…";
  status.dataset.state = "loading";
  progress("Reading " + file.name + "…", 0);
  invalidate();
  try {
    const bytes = await readFile(file, token);
    if (token !== generation) return;
    progress("Opening archive and decompressing entries…");
    startWorker();
    worker.postMessage({ type: "load", name: file.name, bytes }, [bytes]);
  } catch (error) {
    if (token !== generation) return;
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
  const filterParts = () => {
    app.set_search(searchInput.value);
    clearSearch.hidden = searchInput.value.length === 0;
    invalidate();
  };
  searchInput.addEventListener("input", filterParts);
  clearSearch.addEventListener("click", () => {
    resetSearch();
    filterParts();
    searchInput.focus();
  });
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
    if (e.key === "/" && !e.ctrlKey && !e.metaKey && !e.altKey) {
      searchInput.focus();
      e.preventDefault();
      return;
    }
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
    stopRead();
    finishProgress();
    worker?.terminate();
    worker = null;
    loaded = false;
    busy = false;
    app.clear();
    resetSearch();
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
  finishProgress();
  status.textContent = "Ready · files stay in this browser";
  status.dataset.state = "ready";
  invalidate();
} catch (error) {
  finishProgress();
  status.textContent = String(error);
  status.dataset.state = "error";
  console.error(error);
}
