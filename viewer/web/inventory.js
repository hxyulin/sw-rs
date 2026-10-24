// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
// On-demand structural inspection; saved text is always rendered with textContent.
export class InventoryPanel {
  constructor(send) {
    this.send = send;
    this.button = document.querySelector("#inspect-source");
    this.dialog = document.querySelector("#source-inventory");
    this.title = document.querySelector("#inventory-title");
    this.summary = document.querySelector("#inventory-summary");
    this.filter = document.querySelector("#inventory-filter");
    this.list = document.querySelector("#inventory-streams");
    this.detail = document.querySelector("#inventory-detail");
    this.message = document.querySelector("#inventory-status");
    this.previous = document.querySelector("#inventory-previous");
    this.next = document.querySelector("#inventory-next");
    this.pageLabel = document.querySelector("#inventory-page");
    this.previous.onclick = () => this.ask(this.selectedStream, Math.max(0, this.offset - 100));
    this.next.onclick = () => this.ask(this.selectedStream, this.offset + 100);
    this.path = "";
    this.request = 0;
    this.streams = [];
    this.button.onclick = () => this.open();
    this.filter.oninput = () => this.renderStreams();
    document.querySelector("#inventory-close").onclick = () => this.dialog.close();
    this.dialog.addEventListener("close", () => {
      this.request++;
      this.send({type:"inventory-close"});
      this.streams = [];
      this.list.replaceChildren();
      this.detail.textContent = "";
    });
  }
  setDocument(path) {
    if (path !== this.path && this.dialog.open) this.dialog.close();
    this.path = path || "";
    this.button.disabled = !this.path;
  }
  reset() { this.setDocument(""); }
  open() {
    if (!this.path) return;
    this.title.textContent = `Source inventory · ${this.path}`;
    this.summary.textContent = "";
    this.detail.textContent = "Select a stream to inspect XML or binary candidates.";
    this.filter.value = "";
    this.streams = [];
    this.list.replaceChildren();
    this.dialog.showModal();
    this.filter.focus();
    this.ask();
  }
  ask(stream, offset = 0) {
    this.selectedStream = stream;
    this.offset = offset;
    this.previous.disabled = true;
    this.next.disabled = true;
    this.pageLabel.textContent = "Loading records…";
    this.message.textContent = stream ? `Inspecting ${stream}…` : "Validating and indexing source records…";
    this.send({type:"inventory", id:++this.request, path:this.path, stream, offset});
  }
  receive(data) {
    if (!this.dialog.open || data.id !== this.request || data.path !== this.path) return;
    if (data.message) {
      this.message.textContent = data.message;
      return;
    }
    const {inventory, inspection} = data.report;
    this.streams = inventory.streams;
    const opaque = inventory.source_bytes - inventory.verified_container_bytes;
    this.summary.textContent = `${inventory.format} · ${inventory.source_bytes.toLocaleString()} source bytes · ` +
      `${inventory.streams.length} selected streams · ${data.report.container_record_count} record candidates\n` +
      `${inventory.verified_container_bytes.toLocaleString()} bytes in verified container ranges · ` +
      `${opaque.toLocaleString()} unclassified bytes\nSHA-256: ${inventory.source_sha256}`;
    this.message.textContent = inventory.container_error || "Container validation available; semantic coverage has not been evaluated.";
    this.renderStreams();
    const {offset, size, total} = data.report.page;
    this.offset = offset;
    this.previous.disabled = offset === 0;
    this.next.disabled = offset + size >= total;
    this.pageLabel.textContent = `Record window ${total ? offset + 1 : 0}–${Math.min(offset + size, total)} of ${total} per collection`;
    if (inspection) {
      this.detail.textContent = JSON.stringify({
        stream:this.streams.find(s => s.name === inspection.source_stream),
        source_record:data.report.source_record, ...inspection,
      }, null, 2);
    } else if (data.stream) {
      this.detail.textContent = "Selected stream is unavailable.";
    } else {
      this.detail.textContent = JSON.stringify({records:data.report.container_records,
        unclassified_ranges:data.report.unclassified_ranges}, null, 2);
    }
  }
  renderStreams() {
    const query = this.filter.value.toLocaleLowerCase();
    this.list.replaceChildren();
    for (const stream of this.streams.filter(s => s.name.toLocaleLowerCase().includes(query))) {
      const button = document.createElement("button");
      button.textContent = `${stream.name} · ${stream.decoded_bytes.toLocaleString()} bytes · ${stream.family_hint}`;
      button.onclick = () => this.ask(stream.name);
      this.list.append(button);
    }
  }
}
