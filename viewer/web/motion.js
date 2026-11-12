// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// One explicit joint preview. Native mate evidence remains untouched.
export class MotionPanel {
  constructor(scene, transform) {
    this.scene = scene;
    this.transform = transform;
    this.node = null;
    this.panel = document.querySelector('#motion-panel');
    this.kind = document.querySelector('#motion-kind');
    this.axis = document.querySelector('#motion-axis');
    this.pivot = ['x', 'y', 'z'].map(a => document.querySelector(`#motion-pivot-${a}`));
    this.lower = document.querySelector('#motion-lower');
    this.upper = document.querySelector('#motion-upper');
    this.range = document.querySelector('#motion-range');
    this.value = document.querySelector('#motion-value');
    this.status = document.querySelector('#motion-status');
    for (const input of [this.axis, ...this.pivot, this.lower, this.upper]) input.oninput = () => this.configure();
    this.kind.onchange = () => { this.defaults(); this.configure(); };
    this.range.oninput = () => this.apply(this.range.valueAsNumber);
    this.value.oninput = () => this.apply(this.value.valueAsNumber);
    document.querySelector('#motion-reset').onclick = () => this.apply(0);
  }
  defaults() {
    const hinge = this.kind.value === 'hinge';
    this.lower.value = hinge ? -180 : -100;
    this.upper.value = hinge ? 180 : 100;
    document.querySelector('#motion-unit').textContent = hinge ? 'Angle (degrees)' : 'Travel (mm)';
    document.querySelector('#motion-pivot').hidden = !hinge;
  }
  select(node) {
    if (this.scene.simulationActive) {
      this.scene.clearMotion(); this.node = null; this.panel.hidden = true; return;
    }
    if (node === this.node) return;
    this.scene.clearMotion();
    this.node = node;
    this.panel.hidden = !node?.vertex_count;
    if (!node?.vertex_count) return;
    this.axis.value = 'z';
    this.kind.value = 'hinge';
    this.pivot.forEach((input, i) => { input.value = (node.world_transform?.[12 + i] || 0) * 1000; });
    this.defaults();
    this.configure();
  }
  configure() {
    this.scene.clearMotion();
    this.apply(0);
  }
  apply(value) {
    if (!this.node?.vertex_count) return;
    try {
      const lower = this.lower.valueAsNumber, upper = this.upper.valueAsNumber;
      const pivot = this.pivot.map(input => input.valueAsNumber / 1000);
      if (![value, lower, upper, ...pivot].every(Number.isFinite)) throw Error('Enter finite joint values.');
      if (lower > 0 || upper < 0 || lower >= upper) throw Error('Limits must include zero, with lower less than upper.');
      const scale = this.kind.value === 'hinge' ? Math.PI / 180 : 0.001;
      const axis = ['x', 'y', 'z'].map(a => a === this.axis.value ? 1 : 0);
      const matrix = this.transform(JSON.stringify({ joint: { kind: this.kind.value, axis, pivot, lower: lower * scale, upper: upper * scale }, coordinate: value * scale }));
      if (value === 0) this.scene.clearMotion();
      else this.scene.previewMotion(this.node, matrix);
      this.range.min = lower; this.range.max = upper;
      this.range.disabled = false;
      this.range.value = value; this.value.value = value;
      this.status.textContent = 'Viewer-defined joint. The rest of the assembly stays fixed; saved mates and collisions are not enforced.';
      this.status.dataset.state = 'ready';
    } catch (error) {
      this.scene.clearMotion();
      this.range.disabled = true;
      this.status.textContent = error.message;
      this.status.dataset.state = 'error';
    }
  }
}
