import bpy, bmesh, sys, os
src, dst = sys.argv[sys.argv.index('--')+1:]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
ob=[o for o in bpy.context.scene.objects if o.type=='MESH'][0]
bpy.context.view_layer.objects.active=ob; ob.select_set(True)
# world == local here (check) ; planes keep the walls + a small even margin
# Planes (point, outward normal) for the Second Temple base; edit per model.
cuts=[((0.40,0,0),(1,0,0)), ((0,-0.45,0),(0,-1,0))]
for co,no in cuts:
    bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_all(action='SELECT')
    bpy.ops.mesh.bisect(plane_co=co, plane_no=no, use_fill=True, clear_inner=False, clear_outer=True)
    bpy.ops.object.mode_set(mode='OBJECT')
    me=ob.data; uv=me.uv_layers.active.data
    sel=set(p.index for p in me.polygons if p.select)
    # UV for the new caps: borrow each vertex's UV from a non-cap face
    vuv={}
    for p in me.polygons:
        if p.index in sel: continue
        for li in p.loop_indices: vuv.setdefault(me.loops[li].vertex_index, uv[li].uv.copy())
    n=0
    for i in sel:
        p=me.polygons[i]
        for li in p.loop_indices:
            v=me.loops[li].vertex_index
            if v in vuv: uv[li].uv=vuv[v]; n+=1
    print('cut',co,'sel faces',len(sel),'uv fixed loops',n)
bpy.ops.object.mode_set(mode='EDIT'); bpy.ops.mesh.select_all(action='SELECT'); bpy.ops.mesh.remove_doubles(threshold=1e-5); bpy.ops.object.mode_set(mode='OBJECT')
print('tris', sum(len(p.vertices)-2 for p in ob.data.polygons))
bpy.ops.export_scene.gltf(filepath=dst, export_format='GLB', export_image_format='WEBP',
    export_image_quality=88, export_draco_mesh_compression_enable=False, export_apply=True, export_yup=True, export_tangents=False)
print('size',os.path.getsize(dst))
