import bpy, sys, os
src, dst, tris, tex = sys.argv[sys.argv.index('--')+1:]
tris=int(tris); tex=int(tex)
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
for ob in bpy.context.scene.objects:
    if ob.type!='MESH': continue
    n=sum(len(p.vertices)-2 for p in ob.data.polygons)
    m=ob.modifiers.new('dec','DECIMATE'); m.ratio=min(1.0,tris/n)
    bpy.context.view_layer.objects.active=ob
    bpy.ops.object.modifier_apply(modifier='dec')
    print('tris',n,'->',sum(len(p.vertices)-2 for p in ob.data.polygons))
for img in bpy.data.images:
    if img.size[0]>tex:
        s = tex if 'basecolor' in img.name or 'normal' in img.name else tex//2
        img.scale(s,s)
bpy.ops.export_scene.gltf(filepath=dst, export_format='GLB', export_image_format='WEBP',
    export_image_quality=88, export_draco_mesh_compression_enable=False, export_apply=True,
    export_yup=True, export_tangents=False)
print('size',os.path.getsize(dst))
