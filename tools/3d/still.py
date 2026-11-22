import bpy, sys, math, mathutils
args = sys.argv[sys.argv.index('--')+1:]
src, dst, res, yaw = args[:4]
res=int(res); yaw=float(yaw)
# optional 5th arg: camera pitch (the app's start view; default 0.28)
pitch = float(args[4]) if len(args) > 4 else 0.28
bpy.ops.wm.read_factory_settings(use_empty=True)
bpy.ops.import_scene.gltf(filepath=src)
obs=[o for o in bpy.context.scene.objects if o.type=='MESH']
# bounds
pts=[o.matrix_world @ mathutils.Vector(c) for o in obs for c in o.bound_box]
lo=mathutils.Vector([min(p[i] for p in pts) for i in range(3)]); hi=mathutils.Vector([max(p[i] for p in pts) for i in range(3)])
c=(lo+hi)/2; r=max((p-c).length for p in pts)
for o in obs: o.location -= c
piv=bpy.data.objects.new('piv',None); bpy.context.scene.collection.objects.link(piv)
for o in obs: o.parent=piv
piv.rotation_euler=(0,0,yaw)
sc=bpy.context.scene
cam=bpy.data.cameras.new('c'); cam.lens_unit='FOV'; cam.angle=2*math.atan(1/3.6)
co=bpy.data.objects.new('c',cam); sc.collection.objects.link(co); sc.camera=co
d=4.2*r
co.location=(0,-d*math.cos(pitch),d*math.sin(pitch)); co.rotation_euler=(math.pi/2-pitch,0,0)
def area(name,loc,energy,color,size):
    l=bpy.data.lights.new(name,'AREA'); l.energy=energy; l.color=color; l.size=size
    o=bpy.data.objects.new(name,l); sc.collection.objects.link(o); o.location=loc
    o.rotation_euler=(mathutils.Vector((0,0,0))-mathutils.Vector(loc)).to_track_quat('-Z','Y').to_euler()
area('key',(-2.5*r,-3*r,3.5*r),900*r*r,(1,.92,.8),2*r)
area('rim',(3.5*r,2.5*r,1.5*r),500*r*r,(.8,.88,1),1.5*r)
w=bpy.data.worlds.new('w'); sc.world=w; w.use_nodes=True
bg=w.node_tree.nodes['Background']; bg.inputs[1].default_value=0.6
tex=w.node_tree.nodes.new('ShaderNodeTexGradient')  # simple neutral-warm env
bg.inputs[0].default_value=(0.55,0.5,0.44,1)
sc.render.engine='CYCLES'; sc.cycles.samples=96; sc.cycles.device='CPU'; sc.cycles.use_denoising=True
sc.render.film_transparent=True; sc.render.resolution_x=res; sc.render.resolution_y=res
sc.view_settings.view_transform='AgX'
sc.render.image_settings.file_format='WEBP'; sc.render.image_settings.color_mode='RGBA'; sc.render.image_settings.quality=88
sc.render.filepath=dst
bpy.ops.render.render(write_still=True)
