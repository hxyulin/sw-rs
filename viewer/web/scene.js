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
      uniform mat4 camera; out vec3 n; out vec3 c; out vec3 worldPosition;
      void main(){gl_Position=camera*vec4(position,1.);n=normal;c=color;worldPosition=position;}`,
      ),
    );
    gl.attachShader(
      program,
      shader(
        gl.FRAGMENT_SHADER,
        `#version 300 es
      precision highp float;
      in vec3 n; in vec3 c; in vec3 worldPosition; out vec4 outputColor;
      uniform int surfaceMode;
      uniform vec3 baseColor; uniform float roughness; uniform float metallic;
      uniform vec3 eyePosition;
      const float PI=3.14159265359;
      // Patterns are anchored to placed WORLD coordinates in metres. They do not
      // track the camera or bounds, and are not recovered CAD UVs/fiber directions.
      // Dominant-plane projection avoids stretching on vertical planar surfaces.
      vec2 surfaceUV(vec3 p,vec3 normal){
        vec3 a=abs(normal);
        if(a.z>=a.x && a.z>=a.y)return p.xy;
        if(a.y>=a.x)return p.xz;
        return p.yz;
      }
      // Pixel-footprint filtering suppresses subpixel pattern energy. No bitmap
      // textures or unfiltered hash noise: distant views converge to the mean.
      float wave(float phase){
        float width=fwidth(phase);
        return sin(phase)*exp(-0.5*width*width);
      }
      vec3 keyLight(vec3 normal,vec3 view,vec3 light,vec3 base,float r,float metal){
        vec3 halfVector=normalize(view+light);
        float nl=max(dot(normal,light),0.);
        float nv=max(dot(normal,view),0.001);
        float nh=max(dot(normal,halfVector),0.);
        float vh=max(dot(view,halfVector),0.);
        float alpha=max(r*r,0.025),a2=alpha*alpha;
        float denominator=nh*nh*(a2-1.)+1.;
        float distribution=a2/(PI*denominator*denominator+0.0001);
        float k=(r+1.)*(r+1.)/8.;
        float geometry=nv/(nv*(1.-k)+k)*nl/(nl*(1.-k)+k);
        vec3 f0=mix(vec3(0.04),base,metal);
        vec3 fresnel=f0+(1.-f0)*pow(1.-vh,5.);
        vec3 specular=distribution*geometry*fresnel/max(4.*nv*nl,0.001);
        return ((1.-fresnel)*(1.-metal)*base/PI+specular)*nl;
      }
      void main(){
        vec3 normal=normalize(n);if(!gl_FrontFacing)normal=-normal;
        if(surfaceMode==0){
          float diffuse=max(dot(normal,normalize(vec3(0.4,-0.5,0.8))),0.);
          outputColor=vec4(pow(c*(0.35+0.65*diffuse),vec3(0.8)),1.);return;
        }
        vec2 uv=surfaceUV(worldPosition,normal);
        vec3 base=pow(clamp(baseColor,0.,1.),vec3(2.2));
        float r=clamp(roughness,0.08,1.);
        vec3 view=normalize(eyePosition-worldPosition);
        if(surfaceMode==2){
          // Alternating over/under bundles with filtered fine filaments.
          vec2 q=uv*360.;
          float checker=wave(PI*q.x)*wave(PI*q.y);
          float over=smoothstep(-0.25,0.25,checker);
          float resolved=exp(-2.*dot(fwidth(q),fwidth(q)));
          float bundle=mix(wave(2.*PI*q.y),wave(2.*PI*q.x),over);
          float filament=mix(wave(16.*PI*q.x),wave(16.*PI*q.y),over);
          base*=1.+resolved*(0.32*bundle+0.13*checker)+0.08*filament;
          r=clamp(r+0.08*resolved*bundle,0.18,0.7);
        } else if(surfaceMode==3){
          // World-aligned satin brushing, not a recovered machining direction.
          float brush=wave(2.*PI*uv.y*2800.);
          float broad=wave(2.*PI*uv.y*430.);
          base*=1.+0.07*brush+0.035*broad;
          r=clamp(r+0.045*brush,0.2,0.65);
        }
        vec3 f0=mix(vec3(0.04),base,metallic);
        float grazing=pow(1.-max(dot(normal,view),0.),4.);
        vec3 ambient=base*(0.21+0.12*max(normal.z,0.))*(1.-0.7*metallic);
        ambient+=mix(f0,vec3(0.55),grazing)*(0.18+0.16*max(normal.z,0.));
        vec3 lit=ambient+3.*keyLight(normal,view,normalize(vec3(0.4,-0.5,0.8)),base,r,metallic);
        lit+=1.2*keyLight(normal,view,normalize(vec3(-0.6,0.3,0.5)),base,min(r+0.2,1.),metallic);
        // Approximate display response, not colorimetric/material metrology.
        lit=lit/(vec3(1.)+lit);
        outputColor=vec4(pow(max(lit,vec3(0.)),vec3(1./2.2)),1.);
      }`,
      ),
    );
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS))
      throw Error(gl.getProgramInfoLog(program));
    this.matrix = gl.getUniformLocation(program, "camera");
    this.surfaceMode = gl.getUniformLocation(program, "surfaceMode");
    this.baseColor = gl.getUniformLocation(program, "baseColor");
    this.roughness = gl.getUniformLocation(program, "roughness");
    this.metallic = gl.getUniformLocation(program, "metallic");
    this.eyePosition = gl.getUniformLocation(program, "eyePosition");
    this._materialsEnabled = true;
    this.materials = [];
    this.batches = [];
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
  get materialsEnabled() { return this._materialsEnabled; }
  set materialsEnabled(value) {
    this._materialsEnabled = Boolean(value);
    if (this.gl && this.data) this.draw();
  }
  setMesh(data, materials = [], batches = []) {
    this.data = data;
    this.materials = Array.isArray(materials) ? materials : [];
    // Preserve every triangle exactly once, even if a caller omits unassigned
    // ranges. Invalid/overlapping batches are ignored; gaps remain diagnostic.
    const count = data.length / 9;
    this.batches = [];
    let cursor = 0;
    const ranges = (Array.isArray(batches) ? batches : []).filter((b) =>
      b && Number.isSafeInteger(b.first_vertex) && Number.isSafeInteger(b.vertex_count) &&
      b.first_vertex >= 0 && b.vertex_count > 0 && b.first_vertex % 3 === 0 &&
      b.vertex_count % 3 === 0 && b.first_vertex + b.vertex_count <= count
    ).slice().sort((a, b) => a.first_vertex - b.first_vertex);
    for (const batch of ranges) {
      if (batch.first_vertex < cursor) continue;
      if (batch.first_vertex > cursor) this.batches.push({ first_vertex: cursor, vertex_count: batch.first_vertex - cursor, material_index: null });
      this.batches.push({ ...batch });
      cursor = batch.first_vertex + batch.vertex_count;
    }
    if (cursor < count) this.batches.push({ first_vertex: cursor, vertex_count: count - cursor, material_index: null });
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
    gl.uniform3fv(this.eyePosition, eye);
    gl.uniform1i(this.surfaceMode, 0);
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
    } else if (this.materialsEnabled && this.batches.length) {
      for (const batch of this.batches) {
        const index = batch.material_index;
        const material = Number.isSafeInteger(index) && index >= 0 ? this.materials[index] : null;
        this.applyMaterial(material);
        gl.drawArrays(gl.TRIANGLES, batch.first_vertex, batch.vertex_count);
      }
    } else {
      gl.drawArrays(gl.TRIANGLES, 0, this.data.length / 9);
    }
  }
  applyMaterial(material) {
    const gl = this.gl;
    const color = material?.base_color;
    if (!Array.isArray(color) || color.length !== 3 || !color.every(Number.isFinite)) {
      gl.uniform1i(this.surfaceMode, 0);
      return;
    }
    const mode = material.pattern === "carbon_weave" ? 2 : material.pattern === "brushed_aluminum" ? 3 : 1;
    gl.uniform1i(this.surfaceMode, mode);
    gl.uniform3fv(this.baseColor, color.map((v) => Math.max(0, Math.min(1, v))));
    gl.uniform1f(this.roughness, Number.isFinite(material.roughness) ? Math.max(0.08, Math.min(1, material.roughness)) : 0.6);
    gl.uniform1f(this.metallic, Number.isFinite(material.metallic) ? Math.max(0, Math.min(1, material.metallic)) : 0);
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
