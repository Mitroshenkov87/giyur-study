# 3D models

Tripo AI exports (~65–75 MB GLB, 8K PBR textures) are shrunk for embedding with Blender headless:

    blender -b --factory-startup --python tools/3d/optimize.py -- in.glb assets/3d/out.glb 60000 2048

Decimates to ~60k triangles and re-exports with WebP textures (`EXT_texture_webp`):
base colour and normal at 2048 px, metallic-roughness at 1024 px. That gives 2–3 MB per model.

The static fallback picture (tiny-skia renderer, or «Объёмные модели» off) is rendered with Cycles from the same
camera the app uses:

    blender -b --factory-startup --python tools/3d/still.py -- assets/3d/out.glb assets/3d/out.webp 512 0.7

Runtime: `src/ui/model3d` (GLB reader, wgpu shader widget). Debug hooks: `SHORASHIM_3D_STATS=1` (fps and 3D-pass
time to stderr), `SHORASHIM_3D_YAW=<rad>` (fixed angle, no idle spin), `SHORASHIM_3D_MSAA=1` (no MSAA),
`SHORASHIM_3D_SIZE=<px>` (hero size on the lessons map).
