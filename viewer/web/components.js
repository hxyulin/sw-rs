// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// Occurrence identity is independent of document paths and material batches.
export class ComponentState {
  constructor(nodes = [], count = 0) {
    this.nodes = nodes;
    this.count = count;
    this.selected = null;
    this.hidden = new Set();
    this.isolated = null;
  }
  contains(node, vertex) {
    return node && vertex >= node.first_vertex && vertex < node.first_vertex + node.vertex_count;
  }
  ranges() {
    const selected = this.nodes.find(n => n.id === this.selected);
    const isolated = this.nodes.find(n => n.id === this.isolated);
    const hidden = this.nodes.filter(n => this.hidden.has(n.id));
    const cuts = new Set([0, this.count]);
    for (const n of [selected, isolated, ...hidden]) {
      if (n?.vertex_count) { cuts.add(n.first_vertex); cuts.add(n.first_vertex + n.vertex_count); }
    }
    const points = [...cuts].sort((a, b) => a - b), ranges = [];
    for (let i = 1; i < points.length; i++) {
      const first = points[i - 1];
      if ((isolated && !this.contains(isolated, first)) || hidden.some(n => this.contains(n, first))) continue;
      ranges.push({ first_vertex: first, vertex_count: points[i] - first, selected: this.contains(selected, first) });
    }
    return ranges;
  }
  owner(vertex) {
    // Preorder puts descendants after their parents, even for equal ranges.
    return this.nodes.findLast(n => this.contains(n, vertex));
  }
}

export class ComponentPanel {
  constructor(scene, tree, motion, simulation) {
    this.motion = motion;
    this.simulation = simulation;
    this.scene = scene;
    this.tree = tree;
    this.panel = document.querySelector('#component-inspector');
    scene.onSelect = id => {
      this.select(id);
      const item = this.tree.querySelector(`[data-occurrence="${id}"]`);
      for (let parent = item?.parentElement.closest('[role="treeitem"]'); parent; parent = parent.parentElement.closest('[role="treeitem"]')) parent.expand?.(true);
      item?.scrollIntoView({ block: 'nearest' });
    };
    document.querySelector('#component-restore').onclick = () => {
      scene.components.hidden.clear(); scene.components.isolated = null;
      this.select(scene.components.selected);
    };
  }
  reset(metadata = {}) {
    this.metadata = metadata;
    this.scene.components = new ComponentState(metadata.hierarchy || [], this.scene.data.length / 9);
    this.select(null);
    this.renderMates();
  }
  select(id) {
    const state = this.scene.components;
    state.selected = id;
    this.scene.updateComponents();
    document.querySelector('#component-restore').disabled = !state.hidden.size && state.isolated === null;
    for (const item of this.tree.querySelectorAll('[data-occurrence]')) {
      item.setAttribute('aria-selected', String(Number(item.dataset.occurrence) === id));
    }
    const node = state.nodes.find(n => n.id === id);
    this.motion?.select(node);
    this.simulation?.select(node);
    this.panel.hidden = !node;
    if (!node) return;
    document.querySelector('#component-name').textContent = node.name;
    const props = document.querySelector('#component-properties');
    props.replaceChildren();
    const add = (name, value) => {
      if (value === undefined || value === null || value === '') return;
      const term = document.createElement('dt'), detail = document.createElement('dd');
      term.textContent = name; detail.textContent = value; props.append(term, detail);
    };
    add('Document', node.path || node.reference);
    add('Configuration', node.configuration);
    add('Saved component name', node.component_name);
    add('Occurrence number', node.reference_number);
    add('Native component ID', node.native_id);
    if (node.kind === 'assembly') add('Subassembly mode', node.is_flexible === true ? 'Flexible' : node.is_flexible === false ? 'Rigid' : 'Unknown');
    add('Status', node.status);
    add('Triangles', (node.vertex_count || 0) / 3);
    const material = this.metadata.presentations?.[node.path]?.physical_materials?.[node.physical_material_index];
    add('Material', material?.name?.value || material?.name);
    if (node.world_transform) add('Assembly position (mm)', node.world_transform.slice(12, 15).map(v => (v * 1000).toFixed(3)).join(', '));
    const transform = document.createElement('details'), summary = document.createElement('summary'), raw = document.createElement('pre');
    summary.textContent = 'Saved transform';
    raw.textContent = node.world_transform ? JSON.stringify(node.world_transform, null, 2) : 'World transform unavailable.';
    transform.append(summary, raw); props.append(transform);
    const hasGeometry = node.vertex_count > 0;
    document.querySelector('#component-focus').disabled = !hasGeometry;
    document.querySelector('#component-isolate').disabled = !hasGeometry;
    const hide = document.querySelector('#component-hide');
    hide.disabled = !hasGeometry;
    hide.textContent = state.hidden.has(id) ? 'Show' : 'Hide';
    document.querySelector('#component-focus').onclick = () => this.scene.focusComponent(node);
    document.querySelector('#component-isolate').onclick = () => { state.isolated = id; state.hidden.clear(); this.select(id); };
    hide.onclick = () => { state.hidden.has(id) ? state.hidden.delete(id) : state.hidden.add(id); this.select(id); };

  }
  renderMates() {
    const panel = document.querySelector('#mate-inspector'), list = document.querySelector('#mate-list');
    list.replaceChildren();
    panel.hidden = !this.metadata.hierarchy?.length;
    const index = this.metadata.metadata?.configurations?.[this.metadata.selectedConfiguration]?.index;
    const streams = (this.metadata.metadata?.mates?.configurations || []).filter(s => s.configuration_index === index);
    document.querySelector('#mate-status').textContent = streams.length
      ? 'Saved assembly evidence. Component ownership, parameters and active state are unresolved.'
      : 'No mate evidence available for this configuration.';
    for (const stream of streams) {
      const status = document.createElement('p');
      status.textContent = `${stream.source_stream}: ${stream.records.length}/${stream.declared_count ?? 'unknown'} saved headers`;
      list.append(status);
      for (const record of stream.records) {
        const details = document.createElement('details'), summary = document.createElement('summary');
        summary.textContent = `${record.name.value} · ${record.kind || 'Unknown type'}`;
        details.append(summary);
        const raw = document.createElement('pre');
        const resolution = record.class_reference_resolution;
        raw.textContent = [record.class_name || `Reused class tag: ${record.class_reference ?? 'unknown'}`, `Source offset: ${record.source_offset}`,
          ...(resolution ? [`Class declaration: ${resolution.class_name} at ${resolution.declaration_offset}`,
            `Class index: ${resolution.class_index} (leading word at ${resolution.index_source_offset})`] : []),
          ...record.reference_candidates.map(c => c.reference.value)].join('\n');
        details.append(raw); list.append(details);
      }
    }
  }
}
