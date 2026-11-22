//! Minimal binary glTF (GLB) reader for the embedded ritual-object models:
//! one or more triangle primitives sharing the first material, with
//! POSITION / NORMAL / TEXCOORD_0, u8/u16/u32 indices and WebP (or PNG/JPEG
//! when the `image` crate can decode it) base-colour, metallic-roughness and
//! normal textures. Textures are decoded to RGBA8 with a CPU mip chain.

use serde_json::Value;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub nrm: [f32; 3],
    pub uv: [f32; 2],
}

/// RGBA8 texture with its mip chain (level 0 first).
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub mips: Vec<Vec<u8>>,
}

/// Decoded textures; handed over (taken) on GPU upload so the ~40 MB of
/// RGBA mips do not stay in RAM next to their GPU copies.
pub struct Textures {
    pub base_color: Option<Texture>,
    pub metal_rough: Option<Texture>,
    pub normal: Option<Texture>,
}

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub textures: std::sync::Mutex<Option<Textures>>,
    pub has_normal_map: bool,
    pub base_factor: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    /// Bounding-sphere centre and radius (model units).
    pub center: [f32; 3],
    pub radius: f32,
    /// Lowest point (model Y) — where the contact shadow goes.
    pub min_y: f32,
}

fn err<T>(m: &str) -> Result<T, String> {
    Err(m.to_string())
}

pub fn parse(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return err("not a GLB");
    }
    let u32_at =
        |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    let json_len = u32_at(12) as usize;
    if u32_at(16) != 0x4E4F_534A {
        return err("first chunk is not JSON");
    }
    let json: Value =
        serde_json::from_slice(&bytes[20..20 + json_len]).map_err(|e| e.to_string())?;
    let bin_at = 20 + json_len;
    let bin: &[u8] = if bytes.len() >= bin_at + 8 && u32_at(bin_at + 4) == 0x004E_4942 {
        let len = u32_at(bin_at) as usize;
        &bytes[bin_at + 8..bin_at + 8 + len]
    } else {
        return err("no BIN chunk");
    };

    let arr = |k: &str| {
        json.get(k)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };
    let views = arr("bufferViews");
    let accessors = arr("accessors");

    let view_bytes = |i: usize| -> Result<(&[u8], usize), String> {
        let v = views.get(i).ok_or("bad bufferView")?;
        let off = v.get("byteOffset").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let len = v
            .get("byteLength")
            .and_then(|x| x.as_u64())
            .ok_or("no byteLength")? as usize;
        let stride = v.get("byteStride").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        bin.get(off..off + len)
            .map(|s| (s, stride))
            .ok_or_else(|| "view out of range".into())
    };

    // Float accessor → Vec of N-component arrays.
    let floats = |i: usize, n: usize| -> Result<Vec<f32>, String> {
        let a = accessors.get(i).ok_or("bad accessor")?;
        if a.get("componentType").and_then(|x| x.as_u64()) != Some(5126) {
            return err("only float vertex attributes are supported");
        }
        let count = a.get("count").and_then(|x| x.as_u64()).ok_or("no count")? as usize;
        let view = a
            .get("bufferView")
            .and_then(|x| x.as_u64())
            .ok_or("sparse accessor")? as usize;
        let off = a.get("byteOffset").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let (data, stride) = view_bytes(view)?;
        let stride = if stride == 0 { n * 4 } else { stride };
        let mut out = Vec::with_capacity(count * n);
        for k in 0..count {
            for c in 0..n {
                let o = off + k * stride + c * 4;
                let b = data.get(o..o + 4).ok_or("attribute out of range")?;
                out.push(f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            }
        }
        Ok(out)
    };

    let indices_of = |i: usize| -> Result<Vec<u32>, String> {
        let a = accessors.get(i).ok_or("bad accessor")?;
        let count = a.get("count").and_then(|x| x.as_u64()).ok_or("no count")? as usize;
        let view = a
            .get("bufferView")
            .and_then(|x| x.as_u64())
            .ok_or("sparse accessor")? as usize;
        let off = a.get("byteOffset").and_then(|x| x.as_u64()).unwrap_or(0) as usize;
        let (data, _) = view_bytes(view)?;
        let size = match a.get("componentType").and_then(|x| x.as_u64()) {
            Some(5121) => 1,
            Some(5123) => 2,
            Some(5125) => 4,
            _ => return err("bad index type"),
        };
        let data = data
            .get(off..off + count * size)
            .ok_or("indices out of range")?;
        Ok(match size {
            1 => data.iter().map(|&b| b as u32).collect(),
            2 => data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes(*b) as u32)
                .collect(),
            _ => data
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| u32::from_le_bytes(*b))
                .collect(),
        })
    };

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut material = None;
    for mesh in arr("meshes") {
        for prim in mesh
            .get("primitives")
            .and_then(|p| p.as_array())
            .into_iter()
            .flatten()
        {
            if prim.get("mode").and_then(|m| m.as_u64()).unwrap_or(4) != 4 {
                continue;
            }
            let at = &prim["attributes"];
            let Some(pi) = at.get("POSITION").and_then(|x| x.as_u64()) else {
                continue;
            };
            let pos = floats(pi as usize, 3)?;
            let n = pos.len() / 3;
            let nrm = match at.get("NORMAL").and_then(|x| x.as_u64()) {
                Some(i) => floats(i as usize, 3)?,
                None => [0.0, 1.0, 0.0].repeat(n),
            };
            let uv = match at.get("TEXCOORD_0").and_then(|x| x.as_u64()) {
                Some(i) => floats(i as usize, 2)?,
                None => vec![0.0; n * 2],
            };
            let base = vertices.len() as u32;
            for k in 0..n {
                vertices.push(Vertex {
                    pos: [pos[k * 3], pos[k * 3 + 1], pos[k * 3 + 2]],
                    nrm: [nrm[k * 3], nrm[k * 3 + 1], nrm[k * 3 + 2]],
                    uv: [uv[k * 2], uv[k * 2 + 1]],
                });
            }
            match prim.get("indices").and_then(|x| x.as_u64()) {
                Some(i) => indices.extend(indices_of(i as usize)?.into_iter().map(|x| x + base)),
                None => indices.extend(base..base + n as u32),
            }
            if material.is_none() {
                material = prim.get("material").and_then(|m| m.as_u64());
            }
        }
    }
    if vertices.is_empty() {
        return err("no triangles");
    }

    // Texture index → decoded image.
    let images = arr("images");
    let textures = arr("textures");
    let load_tex = |tex_ref: Option<&Value>, srgb_hint: &str| -> Option<Texture> {
        let ti = tex_ref?.get("index")?.as_u64()? as usize;
        let t = textures.get(ti)?;
        let src = t
            .pointer("/extensions/EXT_texture_webp/source")
            .or_else(|| t.get("source"))?
            .as_u64()? as usize;
        let view = images.get(src)?.get("bufferView")?.as_u64()? as usize;
        let (data, _) = view_bytes(view).ok()?;
        let img = match image::load_from_memory(data) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                eprintln!("model3d: {srgb_hint} texture: {e}");
                return None;
            }
        };
        Some(with_mips(img))
    };
    let mat = material
        .and_then(|m| json.get("materials")?.get(m as usize).cloned())
        .unwrap_or(Value::Null);
    let pbr = mat
        .get("pbrMetallicRoughness")
        .cloned()
        .unwrap_or(Value::Null);
    let base_factor = pbr
        .get("baseColorFactor")
        .and_then(|v| v.as_array())
        .map(|a| {
            let f = |i: usize| a.get(i).and_then(|x| x.as_f64()).unwrap_or(1.0) as f32;
            [f(0), f(1), f(2), f(3)]
        })
        .unwrap_or([1.0; 4]);
    let num = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_f64()).map(|x| x as f32);

    let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for v in &vertices {
        for c in 0..3 {
            lo[c] = lo[c].min(v.pos[c]);
            hi[c] = hi[c].max(v.pos[c]);
        }
    }
    let center = [
        (lo[0] + hi[0]) / 2.0,
        (lo[1] + hi[1]) / 2.0,
        (lo[2] + hi[2]) / 2.0,
    ];
    let radius = vertices
        .iter()
        .map(|v| {
            let d = [
                v.pos[0] - center[0],
                v.pos[1] - center[1],
                v.pos[2] - center[2],
            ];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        })
        .fold(0.0f32, f32::max)
        .max(1e-6);

    let textures = Textures {
        base_color: load_tex(pbr.get("baseColorTexture"), "base colour"),
        metal_rough: load_tex(pbr.get("metallicRoughnessTexture"), "metallic-roughness"),
        normal: load_tex(mat.get("normalTexture"), "normal"),
    };
    Ok(Mesh {
        has_normal_map: textures.normal.is_some(),
        textures: std::sync::Mutex::new(Some(textures)),
        vertices,
        indices,
        base_factor,
        metallic: num(&pbr, "metallicFactor").unwrap_or(1.0),
        roughness: num(&pbr, "roughnessFactor").unwrap_or(1.0),
        center,
        radius,
        min_y: lo[1],
    })
}

/// Box-filtered mip chain down to 1×1.
fn with_mips(img: image::RgbaImage) -> Texture {
    let (width, height) = img.dimensions();
    let mut mips = vec![img.into_raw()];
    let (mut w, mut h) = (width, height);
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let prev = mips.last().unwrap();
        let mut next = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut s = 0u32;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (x * 2 + dx).min(w - 1);
                        let sy = (y * 2 + dy).min(h - 1);
                        s += prev[((sy * w + sx) * 4 + c) as usize] as u32;
                    }
                    next[((y * nw + x) * 4 + c) as usize] = ((s + 2) / 4) as u8;
                }
            }
        }
        mips.push(next);
        w = nw;
        h = nh;
    }
    Texture {
        width,
        height,
        mips,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn embedded_kiddush_cup_parses() {
        let m = super::parse(super::super::KIDDUSH_CUP_GLB).expect("parse");
        assert!(m.vertices.len() > 1000);
        assert_eq!(m.indices.len() % 3, 0);
        assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
        let t = m.textures.lock().unwrap().take().expect("textures");
        let bc = t.base_color.expect("base colour texture");
        assert!(bc.width >= 512 && bc.mips.len() > 5);
        assert!(t.metal_rough.is_some() && t.normal.is_some() && m.has_normal_map);
        assert!(m.radius > 0.0);
    }
}
