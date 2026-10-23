<!-- SPDX-License-Identifier: MIT -->
<!-- Copyright (c) 2026 hxyulin <hxyulin@proton.me> -->
# Viewer fonts

SourceSans3-Regular.otf is the unmodified Source Sans 3 test font from RXUI.
Original Adobe source revision: `7c691c2772570a0d3a9e1bffe8a9d6074257d985`.
SHA-256: `08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6`.
See SourceSans3-OFL.md for the original copyright, reserved names and SIL OFL.

SWViewerCJK.ttf is a derived, renamed, static weight-400 subset of Noto Sans SC,
from google/fonts revision `a85815a42757630ce188fdad368c2dfc444d4773`:
https://github.com/google/fonts/blob/a85815a42757630ce188fdad368c2dfc444d4773/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf
Original SHA-256: `a3041811a78c361b1de50f953c805e0244951c21c5bd412f7232ef0d899af0da`.
Original copyright: Adobe 2014–2021; reserved font name Source.
See NotoSansSC-OFL.txt. This font remains under SIL OFL 1.1, not the code license.
The subset retains Basic Latin, CJK unified/extension-A ideographs, punctuation
and fullwidth forms, and is lazy-loaded only for archives with CJK filenames.
It does not claim complete Unicode coverage.

Reproduce with the pinned original input and fonttools 4.67.0:

```sh
uv run --with fonttools==4.67.0 python scripts/subset-font.py \
  /path/to/NotoSansSC.ttf assets/SWViewerCJK.ttf
```

Derived SHA-256: `711dbac1196f1b2c4e593932d019b79ac875def1029a245035ae9795a86923d5`.
Font copyright belongs to the respective font authors. No CAD assets are bundled.
