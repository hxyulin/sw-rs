// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// App-owned WebGL2 viewport. All CAD parsing and assembly placement is Rust/WASM.
export class SceneView {
  constructor(canvas) {
    this.canvas = canvas;
    const gl = (this.gl = canvas.getContext("webgl2", {
      antialias: true,
      alpha: false,
      preserveDrawingBuffer: true,
    }));
    if (!gl) throw Error("The 3D viewport requires WebGL 2.");
    const shader = (kind, code) => {
      const s = gl.createShader(kind);
      gl.shaderSource(s, code);
      gl.compileShader(s);
      if (!gl.getShaderParameter(s, gl.COMPILE_STATUS))
        throw Error(gl.getShaderInfoLog(s));
      return s;
    };
    const program = (this.program = gl.createProgram());
    gl.attachShader(
      program,
      shader(
        gl.VERTEX_SHADER,
        `#version 300 es
      precision highp float;
      layout(location=0) in vec3 position; layout(location=1) in vec3 normal; layout(location=2) in vec3 color;
      uniform mat4 camera; out vec3 n; out vec3 c;
      void main(){gl_Position=camera*vec4(position,1.);n=normal;c=color;}`,
      ),
    );
    gl.attachShader(
      program,
      shader(
        gl.FRAGMENT_SHADER,
        `#version 300 es
      precision highp float; in vec3 n; in vec3 c; out vec4 outputColor;
      void main(){vec3 normal=normalize(n);if(!gl_FrontFacing)normal=-normal;
      float diffuse=max(dot(normal,normalize(vec3(0.4,-0.5,0.8))),0.);
      outputColor=vec4(pow(c*(0.35+0.65*diffuse),vec3(0.8)),1.);}`,
      ),
    );
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS))
      throw Error(gl.getProgramInfoLog(program));
    this.matrix = gl.getUniformLocation(program, "camera");
    this.buffer = gl.createBuffer();
    this.lines = gl.createBuffer();
    this.lineCount = 0;
    this.data = new Float32Array();
    this.wire = false;
    this.yaw = -0.85;
    this.pitch = 0.45;
    this.radius = 1;
    this.distance = 3;
    this.center = [0, 0, 0];
    let drag = null;
    canvas.addEventListener("pointerdown", (e) => {
      if (e.button > 2) return;
      drag = { x: e.clientX, y: e.clientY, pan: e.button !== 0 || e.shiftKey };
      canvas.setPointerCapture(e.pointerId);
      canvas.focus();
      e.preventDefault();
    });
    canvas.addEventListener("pointermove", (e) => {
      if (!drag) return;
      const dx = e.clientX - drag.x,
        dy = e.clientY - drag.y;
      drag.x = e.clientX;
      drag.y = e.clientY;
      if (drag.pan) {
        const right = [-Math.sin(this.yaw), Math.cos(this.yaw), 0];
        const up = [
          -Math.sin(this.pitch) * Math.cos(this.yaw),
          -Math.sin(this.pitch) * Math.sin(this.yaw),
          Math.cos(this.pitch),
        ];
        for (let i = 0; i < 3; i++)
          this.center[i] +=
            ((-right[i] * dx + up[i] * dy) * this.distance) /
            Math.max(canvas.clientHeight, 1);
      } else {
        this.yaw -= dx * 0.008;
        this.pitch = Math.max(-1.45, Math.min(1.45, this.pitch + dy * 0.008));
      }
      this.draw();
    });
    const end = () => (drag = null);
    canvas.addEventListener("pointerup", end);
    canvas.addEventListener("pointercancel", end);
    canvas.addEventListener("lostpointercapture", end);
    canvas.addEventListener(
      "wheel",
      (e) => {
        this.distance = Math.max(
          this.radius * 0.03,
          Math.min(
            this.radius * 100,
            this.distance *
              Math.exp(Math.max(-300, Math.min(300, e.deltaY)) * 0.001),
          ),
        );
        this.draw();
        e.preventDefault();
      },
      { passive: false },
    );
    canvas.addEventListener("dblclick", () => this.fit());
    canvas.addEventListener("contextmenu", (e) => e.preventDefault());
    canvas.addEventListener("keydown", (e) => {
      if (e.key.toLowerCase() === "f") {
        this.fit();
        e.preventDefault();
      }
    });
    canvas.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      document.querySelector("#status").textContent =
        "3D graphics context lost. Reload to recreate the viewer.";
    });
    new ResizeObserver(() => this.draw()).observe(canvas);
  }
  setMesh(data) {
    this.data = data;
    this.gl.bindBuffer(this.gl.ARRAY_BUFFER, this.buffer);
    this.gl.bufferData(this.gl.ARRAY_BUFFER, data, this.gl.STATIC_DRAW);
    // Wireframe shares the supplied vertex buffer and builds only indices on demand.
    this.lineCount = 0;
    this.gl.bindBuffer(this.gl.ELEMENT_ARRAY_BUFFER, this.lines);
    this.gl.bufferData(this.gl.ELEMENT_ARRAY_BUFFER, 0, this.gl.STATIC_DRAW);
    if (this.gl.getError() !== this.gl.NO_ERROR)
      throw Error("Unable to upload mesh to the GPU. Available GPU memory may be exhausted.");
    this.fit();
  }
  prepareWireframe() {
    if (this.lineCount) return;
    const count = this.data.length / 9;
    const indices = new Uint32Array(count * 2);
    for (let vertex = 0, at = 0; vertex < count; vertex += 3) {
      indices.set([vertex, vertex + 1, vertex + 1, vertex + 2, vertex + 2, vertex], at);
      at += 6;
    }
    const gl = this.gl;
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.lines);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
    if (gl.getError() !== gl.NO_ERROR)
      throw Error("Unable to build wireframe. Available GPU memory may be exhausted.");
    this.lineCount = indices.length;
  }
  fit() {
    const lo = [Infinity, Infinity, Infinity],
      hi = [-Infinity, -Infinity, -Infinity];
    for (let i = 0; i < this.data.length; i += 9)
      for (let a = 0; a < 3; a++) {
        lo[a] = Math.min(lo[a], this.data[i + a]);
        hi[a] = Math.max(hi[a], this.data[i + a]);
      }
    this.center = this.data.length
      ? lo.map((v, i) => (v + hi[i]) / 2)
      : [0, 0, 0];
    this.radius = this.data.length
      ? Math.max(Math.hypot(...hi.map((v, i) => v - lo[i])) / 2, 0.001)
      : 1;
    this.distance = this.radius * 3.2;
    this.yaw = -0.85;
    this.pitch = 0.45;
    this.draw();
  }
  draw() {
    const gl = this.gl,
      c = this.canvas,
      dpr = Math.min(devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.round(c.clientWidth * dpr)),
      h = Math.max(1, Math.round(c.clientHeight * dpr));
    if (c.width !== w || c.height !== h) {
      c.width = w;
      c.height = h;
    }
    gl.viewport(0, 0, w, h);
    gl.clearColor(0.045, 0.06, 0.075, 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    if (!this.data.length) return;
    const cp = Math.cos(this.pitch),
      eye = this.center.map(
        (v, i) =>
          v +
          this.distance *
            [
              cp * Math.cos(this.yaw),
              cp * Math.sin(this.yaw),
              Math.sin(this.pitch),
            ][i],
      );
    const camera = multiply(
      perspective(Math.PI / 4, w / h, this.radius * 0.005, this.radius * 300),
      lookAt(eye, this.center),
    );
    gl.useProgram(this.program);
    gl.uniformMatrix4fv(this.matrix, false, camera);
    gl.enable(gl.DEPTH_TEST);
    gl.disable(gl.CULL_FACE);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    for (let i = 0; i < 3; i++) {
      gl.enableVertexAttribArray(i);
      gl.vertexAttribPointer(i, 3, gl.FLOAT, false, 36, i * 12);
    }
    if (this.wire) {
      this.prepareWireframe();
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.lines);
      gl.drawElements(gl.LINES, this.lineCount, gl.UNSIGNED_INT, 0);
    } else {
      gl.drawArrays(gl.TRIANGLES, 0, this.data.length / 9);
    }
  }
}
function multiply(a, b) {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++)
    for (let r = 0; r < 4; r++)
      for (let k = 0; k < 4; k++) out[c * 4 + r] += a[k * 4 + r] * b[c * 4 + k];
  return out;
}
function perspective(fov, aspect, near, far) {
  const f = 1 / Math.tan(fov / 2);
  return [
    f / aspect,
    0,
    0,
    0,
    0,
    f,
    0,
    0,
    0,
    0,
    (far + near) / (near - far),
    -1,
    0,
    0,
    (2 * far * near) / (near - far),
    0,
  ];
}
function lookAt(eye, target) {
  const normalize = (a) => {
    const n = Math.hypot(...a);
    return a.map((x) => x / n);
  };
  const cross = (a, b) => [
    a[1] * b[2] - a[2] * b[1],
    a[2] * b[0] - a[0] * b[2],
    a[0] * b[1] - a[1] * b[0],
  ];
  const z = normalize(eye.map((v, i) => v - target[i])),
    x = normalize(cross([0, 0, 1], z)),
    y = cross(z, x),
    dot = (a) => -a.reduce((s, v, i) => s + v * eye[i], 0);
  return [
    x[0],
    y[0],
    z[0],
    0,
    x[1],
    y[1],
    z[1],
    0,
    x[2],
    y[2],
    z[2],
    0,
    dot(x),
    dot(y),
    dot(z),
    1,
  ];
}
