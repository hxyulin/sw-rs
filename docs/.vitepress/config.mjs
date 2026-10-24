// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
import { defineConfig } from 'vitepress';
const pages = (entries) => entries.map(([text, link]) => ({ text, link }));
export default defineConfig({
  title: 'sw-rs',
  description: 'Rust and WebAssembly SolidWorks reader: usage, architecture and save-format research.',
  base: '/sw-rs/docs/',
  // Keep .html URLs: GitHub Pages does not provide SPA rewrite rules.
  cleanUrls: false,
  lastUpdated: true,
  themeConfig: {
    search: { provider: 'local' },
    nav: pages([
      ['Guide', '/getting-started'], ['Save format', '/format'],
      ['Rust API', 'https://hxyulin.github.io/sw-rs/api/sw_rs/'],
      ['SW Explorer', 'https://hxyulin.github.io/sw-rs/'],
    ]),
    sidebar: [
      { text: 'Use sw-rs', items: pages([
        ['Getting started', '/getting-started'], ['Rust & WASM API', '/api'],
        ['Command line', '/cli'], ['SW Explorer', '/viewer'], ['Measured coverage', '/parsing-coverage'],
      ]) },
      { text: 'Save-format reference', items: pages([
        ['Format overview', '/format'], ['Containers & compression', '/containers'],
        ['Metadata & configurations', '/metadata'], ['Cached geometry & coordinates', '/geometry'],
        ['Materials & presentation', '/presentation'], ['Reader & source inventory', '/reader-inventory'],
      ]) },
      { text: 'Decoding research', items: pages([
        ['Archive class/object maps', '/archive-reference-rules'], ['Saved identities & graph', '/references'],
        ['Surface-reference tuples', '/surface-reference-rules'], ['Mate relationships', '/mates'],
        ['Appearance hypotheses', '/appearance-research'], ['Legacy ZLB framing', '/legacy-zlb-rules'],
      ]) },
      { text: 'Contribute', items: pages([
        ['Architecture', '/architecture'], ['Development & deployment', '/development'],
        ['Validation history', '/validation'], ['Simulator porting history', '/porting'], ['Licensing', '/licensing'],
      ]) },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/hxyulin/sw-rs' }],
    editLink: { pattern: 'https://github.com/hxyulin/sw-rs/edit/main/docs/:path' },
    outline: [2, 3],
    footer: { message: 'Independent SolidWorks reader. MIT and Apache-2.0 notices apply; bundled fonts use SIL OFL.' },
  },
});
