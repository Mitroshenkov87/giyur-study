# Add a larger stone altar (mizbeach) with ramp, horns and top step in the
# Court of the Priests, textured from a patch of the model's own wall stone.
import bpy, bmesh, sys, os, mathutils
from mathutils.bvhtree import BVHTree
src, dst = sys.argv[sys.argv.index('--')+1:]
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
ob=[o for o in bpy.context.scene.objects if o.type=='MESH'][0]
me=ob.data; mat=me.materials[0]
# --- stone UV: hit the front wall facade horizontally, interpolate its UV
bm0=bmesh.new(); bm0.from_mesh(me); bm0.faces.ensure_lookup_table()
uvl=bm0.loops.layers.uv.active
tree=BVHTree.FromBMesh(bm0)
loc,nor,fi,d=tree.ray_cast(mathutils.Vector((-0.30,-1.0,0.09)),mathutils.Vector((0,1,0)))
f=bm0.faces[fi]
from mathutils.interpolate import poly_3d_calc
w=poly_3d_calc([l.vert.co for l in f.loops], loc)
UV0=sum((l[uvl].uv*wi for l,wi in zip(f.loops,w)), mathutils.Vector((0,0)))
print('stone hit',loc,'uv',UV0)
bm0.free()
FLOOR=0.1675
# Remove the small Tripo altar by the court wall (it read as a second,
# tiny altar) and patch the floor under it with the court's own floor UV.
OLD=(-0.172,0.072,-0.347,-0.258)
bmo=bmesh.new(); bmo.from_mesh(me)
dead=[f for f in bmo.faces if OLD[0]<f.calc_center_median().x<OLD[1] and OLD[2]<f.calc_center_median().y<OLD[3] and f.calc_center_median().z>FLOOR+0.002]
print('old altar faces',len(dead))
bmesh.ops.delete(bmo,geom=dead,context='FACES')
bmo.to_mesh(me); bmo.free()
bmf=bmesh.new(); bmf.from_mesh(me); bmf.faces.ensure_lookup_table(); uvf=bmf.loops.layers.uv.active
tf=BVHTree.FromBMesh(bmf)
l2,n2,fi2,d2=tf.ray_cast(mathutils.Vector((-0.03,-0.24,1.0)),mathutils.Vector((0,0,-1)))
f2=bmf.faces[fi2]; w2=poly_3d_calc([l.vert.co for l in f2.loops], l2)
UVF=sum((l[uvf].uv*wi for l,wi in zip(f2.loops,w2)), mathutils.Vector((0,0)))
bmf.free()
print('floor uv',UVF,'z',l2.z)
CX,CY=-0.035,-0.208
parts=[]
def box(name,cx,cy,z0,sx,sy,sz):
    bm=bmesh.new(); bmesh.ops.create_cube(bm,size=1.0)
    for v in bm.verts: v.co=mathutils.Vector((cx+v.co.x*sx, cy+v.co.y*sy, z0+(v.co.z+0.5)*sz))
    return bm
def wedge(x0,x1,cy,w,z0,h):
    # ramp rising from x0 (floor) to x1 (top)
    bm=bmesh.new()
    V=[bm.verts.new(p) for p in [(x0,cy-w/2,z0),(x0,cy+w/2,z0),(x1,cy+w/2,z0),(x1,cy-w/2,z0),(x1,cy-w/2,z0+h),(x1,cy+w/2,z0+h)]]
    for idx in [(0,3,2,1),(0,1,5,4)[::-1],(3,4,5,2),(0,4,3),(1,2,5)]:
        bm.faces.new([V[i] for i in idx])
    bmesh.ops.recalc_face_normals(bm,faces=bm.faces)
    return bm
S=0.11; H=0.034
geoms=[box('base',CX,CY,FLOOR,S,S,H),
       box('top',CX,CY,FLOOR+H,S*0.86,S*0.86,0.012)]
hz=FLOOR+H+0.012; k=S*0.86/2-0.010
for dx in (-k,k):
    for dy in (-k,k):
        geoms.append(box("horn",CX+dx,CY+dy,hz,0.018,0.018,0.016))
floor=bmesh.new(); fv=[floor.verts.new((x,y,FLOOR+0.0004)) for x,y in [(OLD[0],OLD[2]),(OLD[1],OLD[2]),(OLD[1],OLD[3]),(OLD[0],OLD[3])]]; floor.faces.new(fv)
geoms.append(wedge(CX-S/2-0.095,CX-S/2,CY,0.055,FLOOR,H+0.012))
geoms.append(floor)
# merge into one bmesh, box-project UVs into a small stone patch
bm=bmesh.new()
for g in geoms:
    tmp=bpy.data.meshes.new('t'); g.to_mesh(tmp); bm.from_mesh(tmp); bpy.data.meshes.remove(tmp); g.free()
uv=bm.loops.layers.uv.new('UVMap')
PATCH=0.012
for fc in bm.faces:
    n=fc.normal; ax=max(range(3),key=lambda i:abs(n[i]))
    a,b=[i for i in range(3) if i!=ax]
    if abs(fc.normal.z)>0.9 and abs(fc.calc_center_median().z-FLOOR)<0.002:
        # small planar window (never degenerate: the normal map needs UV derivatives)
        for l in fc.loops: l[uv].uv=(UVF.x+(l.vert.co.x-OLD[0])*0.02, UVF.y+(l.vert.co.y-OLD[2])*0.02)
        fc.smooth=False; continue
    for l in fc.loops:
        c=l.vert.co
        l[uv].uv=(UV0.x+((c[a]*7.0)%1.0-0.5)*PATCH, UV0.y+((c[b]*7.0)%1.0-0.5)*PATCH)
    fc.smooth=False
am=bpy.data.meshes.new('altar'); bm.to_mesh(am); bm.free()
am.materials.append(mat)
ao=bpy.data.objects.new('altar',am); bpy.context.scene.collection.objects.link(ao)
# uv layer name must match the main mesh for join
am.uv_layers[0].name=me.uv_layers.active.name
bpy.ops.object.select_all(action='DESELECT'); ao.select_set(True); ob.select_set(True)
bpy.context.view_layer.objects.active=ob; bpy.ops.object.join()
print('tris', sum(len(p.vertices)-2 for p in ob.data.polygons))
bpy.ops.export_scene.gltf(filepath=dst, export_format='GLB', export_image_format='WEBP',
    export_image_quality=88, export_draco_mesh_compression_enable=False, export_apply=True, export_yup=True, export_tangents=False)
print('size',os.path.getsize(dst))
