#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 hxyulin <hxyulin@proton.me>
"""Generate synthetic native-container fixtures, never third-party CAD."""
from pathlib import Path
import io,struct,tarfile,zipfile,zlib
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'test-fixtures';OUT.mkdir(exist_ok=True)
def u32(*values):return struct.pack('<'+'I'*len(values),*values)
def container(streams):
    out=bytearray(16);out[7]=3
    for name,data in streams:
        name=name.encode();compressor=zlib.compressobj(wbits=-15);packed=compressor.compress(data)+compressor.flush()
        encoded=bytes(((b>>3)|(b<<5))&255 for b in name)
        out.extend(b'\x14\0\x06\0\x08\0'+u32(1,zlib.crc32(data),len(packed),len(data),len(name))+encoded+packed)
    return bytes(out)
def descriptor(size,kind,count,data):return u32(size,kind,2,count)+data
def visual_class(name):
    return b'\xff\xff\x01\x00'+struct.pack('<H',len(name))+name.encode()
def visual_string(text):
    raw=text.encode('utf-16le');return b'\xff\xfe\xff'+bytes([len(raw)//2])+raw
def visual_property(rgb, name):
    return bytes([*rgb,0])+bytes(8)+visual_string('')+visual_string(name)+struct.pack('<6d',1,.7,.5,.3125,0,0)
def native_visual_state():
    return (u32(18000,0)+visual_class('visStateKeyData_c')+u32(5,1,1,0,1,1,1,1)+
            visual_string('<Default>_Inspection')+bytes(16)+u32(1)+
            visual_class('moCompHolder_c')+visual_class('moUnitComponent_c')+
            bytes(43)+bytes([255])*16+bytes(20)+u32(18000,1,1,2)+
            visual_class('visStateVisualPropertiesAttribute_c')+u32(1)+
            visual_class('moVisualProperties_c')+visual_property([34,68,102],'Saved native default'))
def part(size, material=False, visual=False):
    x,y,z=[v/2 for v in size]
    p=[[-x,-y,-z],[x,-y,-z],[x,y,-z],[-x,y,-z],[-x,-y,z],[x,-y,z],[x,y,z],[-x,y,z]]
    faces=[(0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)]
    triangles=[t for a,b,c,d in faces for t in [(a,b,c),(a,c,d)]]
    points=[p[i] for t in triangles for i in t]
    raw=b''.join(struct.pack('<fff',*v) for v in points)
    table=(descriptor(4,8,12,u32(*([3]*12)))+descriptor(12,100,36,raw)+descriptor(12,100,36,bytes(432))+
           descriptor(4,8,48,bytes(192))+descriptor(4,8,12,u32(*([4]*12)))+descriptor(1,8,48,bytes(48)))
    cls=b'uoTempFaceTessData_c';display=b'\xff\xff\x01\0'+struct.pack('<H',len(cls))+cls+u32(12,12)+table
    info=b'<swSolidWorks><swHeader><swFile id="p"/></swHeader><swModelList><swModel id="m" swFileRef="p" swConfigurationId="0" swConfigurationName="Default"/></swModelList></swSolidWorks>'
    if visual:
        display += (b'\x33\x80'+visual_property([255,0,0],'Cached face')+
                    visual_class('uoBodyPropInfo_c')+b'\x33\x80'+visual_property([0,0,255],'Cached body'))
    streams = [('swXmlContents/COMPINSTANCETREE',info),('Contents/DisplayLists',display)]
    if visual: streams.append(('ThirdPtyStore/VisualStates',native_visual_state()))
    if material:
        streams.append(('swXmlContents/MATERIALTREE', b'<mstns:materials xmlns:mstns="http://www.solidworks.com/sldmaterials"><classification name="Aluminum"><material name="test materials|6061 Alloy" matid="21"><swatchcolor RGB="151515"/><shaders><pwshader name="black low gloss plastic" path="designlibrary/appearances/plastic/low gloss/black low gloss plastic.p2m"/></shaders><physicalproperties><DENS value="2700"/></physicalproperties></material></classification><configurationslist><configuration name="Default" material="test materials|6061 Alloy|21"/></configurationslist></mstns:materials>'))
    return container(streams)
def assembly(missing=False):
    files='<swFile id="root" swPath="Demo.SLDASM"/>'
    models=''
    for i,name in enumerate(['Base.SLDPRT','Upright.SLDPRT','Cap.SLDPRT']):
        files+=f'<swFile id="p{i}" swPath="C:\\Synthetic\\{name}"/>'
        models+=f'<swModel id="m{i}" swFileRef="p{i}" swConfigurationName="Default"/>'
    refs=''
    for i,(name,index,position) in enumerate([('Base',0,[0,0,0]),('Left support',1,[-.065,0,.07]),('Right support',1,[.065,0,.07]),('Cap',2,[0,0,.14])]):
        m=[1,0,0,0,0,1,0,0,0,0,1,0,*position,1]
        refs+=f'<swReference swName="{name}" swModelRef="m{index}" swConfigurationName="Default" swSuppressed="NO" swHidden="NO" swTransform="{" ".join(map(str,m))}"/>'
    xml=f'<swSolidWorks><swHeader>{files}</swHeader><swModelList><swModel id="main" swFileRef="root" swConfigurationId="0" swConfigurationName="Default">{refs}</swModel>{models}</swModelList></swSolidWorks>'
    return container([('swXmlContents/COMPINSTANCETREE',xml.encode())])
files={'Demo.SLDASM':assembly(),'Base.SLDPRT':part([.18,.10,.02]),'Upright.SLDPRT':part([.025,.03,.12]),'Cap.SLDPRT':part([.18,.06,.02]),'README.txt':b'Synthetic parser demo. Not a SolidWorks-authored model.\n'}
for suffix in ['zip','tar','tar.gz']:
    path=OUT/f'demo.{suffix}'
    if suffix=='zip':
        with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as archive:
            for name,data in files.items():archive.writestr(name,data)
    else:
        with tarfile.open(path,'w:gz' if suffix=='tar.gz' else 'w') as archive:
            for name,data in files.items():
                entry=tarfile.TarInfo(name);entry.size=len(data);archive.addfile(entry,io.BytesIO(data))
(OUT/'single.SLDPRT').write_bytes(part([.18,.10,.02], material=True))
(OUT/'invalid.zip').write_bytes(b'PK invalid')
(ROOT/'web/demo.zip').write_bytes((OUT/'demo.zip').read_bytes())

# Partial assembly: omit Upright, keeping both Base and Cap available.
with zipfile.ZipFile(OUT/'missing-references.zip','w',zipfile.ZIP_DEFLATED) as archive:
    for name,data in files.items():
        if name != 'Upright.SLDPRT': archive.writestr(name,data)
# Unicode paths exercise the same filter used for large archive document lists.
with zipfile.ZipFile(OUT/'unicode-search.zip','w',zipfile.ZIP_DEFLATED) as archive:
    for name in ['零件/支架.SLDPRT', 'components/Base.SLDPRT']:
        archive.writestr(name, files['Base.SLDPRT'])

def linked_assembly(name, references, extra_streams=(), flags=None):
    files=f'<swFile id="self" swPath="{name}"/>'
    models=''
    children=''
    for i,reference in enumerate(references):
        files+=f'<swFile id="f{i}" swPath="{reference}"/>'
        models+=f'<swModel id="m{i}" swFileRef="f{i}" swConfigurationName="Default"/>'
        suppressed, hidden = (flags or {}).get(i, ('NO', 'NO'))
        children+=f'<swReference swName="child{i}" swModelRef="m{i}" swConfigurationName="Default" swSuppressed="{suppressed}" swHidden="{hidden}" swTransform="1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1"/>'
    xml=f'<swSolidWorks><swHeader>{files}</swHeader><swModelList><swModel id="root" swFileRef="self" swConfigurationId="0" swConfigurationName="Default">{children}</swModel>{models}</swModelList></swSolidWorks>'
    return container([('swXmlContents/COMPINSTANCETREE',xml.encode()),*extra_streams])
def mfc_string(text):
    data=text.encode('utf-16le');n=len(data)//2
    return b'\xff\xfe\xff'+(bytes([n]) if n<254 else b'\xfe'+struct.pack('<H',n))+data
old='C:/source/Old.SLDPRT';middle='C:/copy/Middle.SLDPRT';new='C:/export/New.SLDPRT'
remaps=u32(2)+b''.join(map(mfc_string,[old,middle,middle,new]))
renamed=linked_assembly('Renamed.SLDASM',[old],[('SwDocMgrTempStorage/ReplaceRef',remaps)])
inner=linked_assembly('嵌入.SLDASM',['C:/Temp/IC~~/嵌入零件.SLDPRT'])
embedded=linked_assembly('Embedded.SLDASM',['C:/Temp/IC~~/嵌入.SLDASM'],
    [('ImportedComp/嵌入.SLDASM',inner),('ImportedComp/嵌入零件.SLDPRT',files['Base.SLDPRT'])])
virtual=linked_assembly('Virtual.SLDASM',['C:/Temp/VC~~/部件^Virtual.SLDPRT'],
    [('VirtualComp/部件-sldprt',files['Base.SLDPRT'])])
for archive_name,documents in [('renamed.zip',{'Renamed.SLDASM':renamed,'New.SLDPRT':files['Base.SLDPRT']}),
    ('embedded.zip',{'Embedded.SLDASM':embedded}),('virtual.zip',{'Virtual.SLDASM':virtual})]:
    with zipfile.ZipFile(OUT/archive_name,'w',zipfile.ZIP_DEFLATED) as archive:
        for name,data in documents.items():archive.writestr(name,data)

# Nested occurrence tree, repeated subassemblies and retained excluded/error nodes.
nested = linked_assembly('Sub.SLDASM', ['Base.SLDPRT', 'Missing.SLDPRT', 'Unsupported.SLDPRT'])
# Keep statuses in the native metadata instead of constructing a UI-only tree.
root_refs = ['Sub.SLDASM', 'Sub.SLDASM', 'Suppressed.SLDPRT', 'Hidden.SLDPRT']
root = linked_assembly('Root.SLDASM', root_refs, flags={2: ('YES', 'NO'), 3: ('NO', 'YES')})
unsupported = container([('swXmlContents/COMPINSTANCETREE', b'<swSolidWorks><swHeader><swFile id="p"/></swHeader><swModelList><swModel id="m" swFileRef="p" swConfigurationId="0" swConfigurationName="Default"/></swModelList></swSolidWorks>')])
with zipfile.ZipFile(OUT/'hierarchy.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
    for name, data in {'Root.SLDASM': root, 'Sub.SLDASM': nested,
                       'Base.SLDPRT': files['Base.SLDPRT'],
                       'Hidden.SLDPRT': files['Base.SLDPRT'],
                       'Unsupported.SLDPRT': unsupported}.items():
        archive.writestr(name, data)

# Open a referenced assembly's nondefault configuration, then keep the RXUI
# configuration selector and inspector aligned with the displayed occurrence.
config_sub_xml = b'<swSolidWorks><swHeader><swFile id="self" swPath="ConfigSub.SLDASM"/><swFile id="p" swPath="Base.SLDPRT"/></swHeader><swModelList><swModel id="default" swFileRef="self" swConfigurationId="0" swConfigurationName="Default"/><swModel id="alternate" swFileRef="self" swConfigurationId="1" swConfigurationName="Alternate"><swReference swName="Alternate part" swModelRef="part" swConfigurationName="Default" swSuppressed="NO" swHidden="NO" swTransform="1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1"/></swModel><swModel id="part" swFileRef="p" swConfigurationName="Default"/></swModelList></swSolidWorks>'
config_root_xml = b'<swSolidWorks><swHeader><swFile id="self" swPath="ConfigRoot.SLDASM"/><swFile id="sub" swPath="ConfigSub.SLDASM"/></swHeader><swModelList><swModel id="root" swFileRef="self" swConfigurationId="0" swConfigurationName="Default"><swReference swName="Configured subassembly" swModelRef="submodel" swConfigurationName="Alternate" swSuppressed="NO" swHidden="NO" swTransform="1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 1"/></swModel><swModel id="submodel" swFileRef="sub" swConfigurationName="Alternate"/></swModelList></swSolidWorks>'
with zipfile.ZipFile(OUT/'configuration-hierarchy.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
    archive.writestr('ConfigRoot.SLDASM', container([('swXmlContents/COMPINSTANCETREE', config_root_xml)]))
    archive.writestr('ConfigSub.SLDASM', container([('swXmlContents/COMPINSTANCETREE', config_sub_xml), ('Contents/Config-0', b'x'), ('Contents/Config-1', b'x')]))
    archive.writestr('Base.SLDPRT', files['Base.SLDPRT'])

(OUT/'native-appearance.SLDPRT').write_bytes(part([.18,.10,.02],material=True,visual=True))
