# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Run with fonttools==4.67.0 on the pinned Noto source recorded in assets/README.md."""
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools import subset
import sys
font=TTFont(sys.argv[1],recalcTimestamp=False)
font=instantiateVariableFont(font,{"wght":400},inplace=True)
options=subset.Options();options.name_IDs=["*"];options.name_legacy=True;options.name_languages=["*"]
subsetter=subset.Subsetter(options=options)
subsetter.populate(unicodes=list(range(0x20,0x100))+list(range(0x2000,0x3100))+list(range(0x3400,0xa000))+list(range(0xff00,0xfff0)))
subsetter.subset(font)
for record in font["name"].names:
 if record.nameID in (1,4,6,16,17):
  text="SWViewerCJK-Regular" if record.nameID==6 else ("Regular" if record.nameID==17 else "SW Viewer CJK")
  record.string=text.encode(record.getEncoding())
font.save(sys.argv[2])
