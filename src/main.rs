mod core;
mod plugin;
mod ui_utils;

use crate::core::*;
use crate::plugin::{
    apply_effect_plugin, execute_functional_plugin, Plugin, PluginRegistry,
};
use crate::ui_utils::{
    apply_after_effects_style, draw_pro_ae_timeline, format_timecode, get_label_color,
    property_display_name, sorted_property_names, AE_LABEL_COLORS,
};
use egui_macroquad::egui;
use macroquad::audio::{load_sound, play_sound, stop_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;
use playa_ffmpeg::format::Pixel;
use playa_ffmpeg::software::scaling::{flag::Flags, Context as ScalerContext};
use playa_ffmpeg::util::frame::Video;
use playa_ffmpeg::{self as ffmpeg, dict};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use egui_macroquad::egui::{Color32, RichText};

fn resource_icon(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Image => "🖼 IMG",
        ResourceKind::Audio => "🔊 AUD",
        ResourceKind::Video => "🎬 VID",
        ResourceKind::Model3D => "📦 3D",
    }
}

fn add_resource(comp: &mut Composition, name: String, path: String, kind: ResourceKind) {
    comp.resources.push(Resource { name, path, kind });
}

#[derive(Clone, Copy, Debug)]
pub struct ActiveCamera {
    pub position: Vec3,
    pub target: Vec3,
    pub rotation: Vec3,
    pub zoom: f32,
    pub fov: f32,
    pub is_custom: bool,
}

pub fn get_active_camera(comp: &Composition, time: f32) -> ActiveCamera {
    let width = comp.settings.width as f32;
    let height = comp.settings.height as f32;
    let default_zoom = 1500.0;

    let has_solo = comp.layers.iter().any(|l| l.solo);

    for layer in &comp.layers {
        if !layer.visible {
            continue;
        }
        if has_solo && !layer.solo {
            continue;
        }
        if time < layer.in_time || time > layer.out_time {
            continue;
        }
        if let LayerSource::Camera = &layer.source {
            let x = layer.properties.get("x").map_or(width / 2.0, |p| p.get_value_at(time));
            let y = layer.properties.get("y").map_or(height / 2.0, |p| p.get_value_at(time));
            let z = layer.properties.get("z").map_or(-default_zoom, |p| p.get_value_at(time));
            let poi_x = layer.properties.get("poiX").map_or(width / 2.0, |p| p.get_value_at(time));
            let poi_y = layer.properties.get("poiY").map_or(height / 2.0, |p| p.get_value_at(time));
            let poi_z = layer.properties.get("poiZ").map_or(0.0, |p| p.get_value_at(time));
            let rot_x = layer.properties.get("rotationX").map_or(0.0, |p| p.get_value_at(time));
            let rot_y = layer.properties.get("rotationY").map_or(0.0, |p| p.get_value_at(time));
            let rot_z = layer.properties.get("rotation").map_or(0.0, |p| p.get_value_at(time));
            let zoom = layer.properties.get("zoom").map_or(default_zoom, |p| p.get_value_at(time));
            let fov = layer.properties.get("fov").map_or(50.0, |p| p.get_value_at(time));

            return ActiveCamera {
                position: vec3(x, y, z),
                target: vec3(poi_x, poi_y, poi_z),
                rotation: vec3(rot_x, rot_y, rot_z),
                zoom,
                fov,
                is_custom: true,
            };
        }
    }

    ActiveCamera {
        position: vec3(width / 2.0, height / 2.0, -default_zoom),
        target: vec3(width / 2.0, height / 2.0, 0.0),
        rotation: vec3(0.0, 0.0, 0.0),
        zoom: default_zoom,
        fov: 50.0,
        is_custom: false,
    }
}

pub fn is_camera_inline_with_z(camera: &ActiveCamera, comp: &Composition) -> bool {
    if comp.viewport_mode != ViewportMode::ActiveCamera {
        return false;
    }
    if camera.is_custom {
        let width = comp.settings.width as f32;
        let height = comp.settings.height as f32;
        let default_target_x = width / 2.0;
        let default_target_y = height / 2.0;
        if (camera.target.x - default_target_x).abs() > 1.0 || (camera.target.y - default_target_y).abs() > 1.0 {
            return false;
        }
        if (camera.position.x - default_target_x).abs() > 1.0 || (camera.position.y - default_target_y).abs() > 1.0 {
            return false;
        }
        if camera.rotation.x.abs() > 0.1 || camera.rotation.y.abs() > 0.1 || camera.rotation.z.abs() > 0.1 {
            return false;
        }
    }
    true
}

pub fn get_viewport_camera(comp: &Composition, time: f32) -> ActiveCamera {
    let width = comp.settings.width as f32;
    let height = comp.settings.height as f32;
    let default_zoom = 1500.0;

    match comp.viewport_mode {
        ViewportMode::ActiveCamera => get_active_camera(comp, time),
        ViewportMode::CustomView => {
            let yaw_r = comp.custom_orbit_yaw.to_radians();
            let pitch_r = comp.custom_orbit_pitch.to_radians();
            let target = vec3(
                comp.custom_orbit_target[0],
                comp.custom_orbit_target[1],
                comp.custom_orbit_target[2],
            );
            let offset = vec3(
                comp.custom_orbit_distance * pitch_r.cos() * yaw_r.sin(),
                comp.custom_orbit_distance * pitch_r.sin(),
                -comp.custom_orbit_distance * pitch_r.cos() * yaw_r.cos(),
            );
            ActiveCamera {
                position: target + offset,
                target,
                rotation: vec3(0.0, 0.0, comp.custom_orbit_roll),
                zoom: default_zoom,
                fov: 50.0,
                is_custom: true,
            }
        }
        ViewportMode::Top => {
            let target = vec3(width / 2.0, height / 2.0, 0.0);
            ActiveCamera {
                position: vec3(target.x, target.y - 100000.0, target.z),
                target,
                rotation: vec3(0.0, 0.0, 0.0),
                zoom: 100000.0,
                fov: 50.0,
                is_custom: true,
            }
        }
        ViewportMode::Bottom => {
            let target = vec3(width / 2.0, height / 2.0, 0.0);
            ActiveCamera {
                position: vec3(target.x, target.y + 100000.0, target.z),
                target,
                rotation: vec3(0.0, 0.0, 0.0),
                zoom: 100000.0,
                fov: 50.0,
                is_custom: true,
            }
        }
        ViewportMode::Front => {
            let target = vec3(width / 2.0, height / 2.0, 0.0);
            ActiveCamera {
                position: vec3(target.x, target.y, target.z - 100000.0),
                target,
                rotation: vec3(0.0, 0.0, 0.0),
                zoom: 100000.0,
                fov: 50.0,
                is_custom: true,
            }
        }
        ViewportMode::Back => {
            let target = vec3(width / 2.0, height / 2.0, 0.0);
            ActiveCamera {
                position: vec3(target.x, target.y, target.z + 100000.0),
                target,
                rotation: vec3(0.0, 0.0, 0.0),
                zoom: 100000.0,
                fov: 50.0,
                is_custom: true,
            }
        }
        ViewportMode::Right => {
            let target = vec3(width / 2.0, height / 2.0, 0.0);
            ActiveCamera {
                position: vec3(target.x + 100000.0, target.y, target.z),
                target,
                rotation: vec3(0.0, 0.0, 0.0),
                zoom: 100000.0,
                fov: 50.0,
                is_custom: true,
            }
        }
        ViewportMode::Left => {
            let target = vec3(width / 2.0, height / 2.0, 0.0);
            ActiveCamera {
                position: vec3(target.x - 100000.0, target.y, target.z),
                target,
                rotation: vec3(0.0, 0.0, 0.0),
                zoom: 100000.0,
                fov: 50.0,
                is_custom: true,
            }
        }
    }
}

/// High-throughput Byte-Plane Run-Length Encoding (BP-RLE) Compressor for RGBA video frames.
/// Separates color channels into contiguous planar blocks (R plane, G plane, B plane, A plane)
/// and applies adaptive SIMD/chunk-accelerated run-length byte packing.
pub fn compress_rgba_frame_planar(raw_rgba: &[u8]) -> Vec<u8> {
    if raw_rgba.is_empty() {
        return Vec::new();
    }
    let pixel_count = raw_rgba.len() / 4;
    let mut compressed = Vec::with_capacity(raw_rgba.len() / 2);
    // Header: 4-byte magic, 4-byte raw length
    compressed.extend_from_slice(b"BFXC");
    compressed.extend_from_slice(&(raw_rgba.len() as u32).to_le_bytes());

    // Linear de-interleave into contiguous plane buffers for maximum CPU cache efficiency
    let mut planes = vec![0u8; raw_rgba.len()];
    let (r_plane, rest) = planes.split_at_mut(pixel_count);
    let (g_plane, rest) = rest.split_at_mut(pixel_count);
    let (b_plane, a_plane) = rest.split_at_mut(pixel_count);

    let chunks = raw_rgba.chunks_exact(16);
    let remainder = chunks.remainder();
    let mut p = 0;
    for chunk in chunks {
        r_plane[p] = chunk[0];
        g_plane[p] = chunk[1];
        b_plane[p] = chunk[2];
        a_plane[p] = chunk[3];

        r_plane[p + 1] = chunk[4];
        g_plane[p + 1] = chunk[5];
        b_plane[p + 1] = chunk[6];
        a_plane[p + 1] = chunk[7];

        r_plane[p + 2] = chunk[8];
        g_plane[p + 2] = chunk[9];
        b_plane[p + 2] = chunk[10];
        a_plane[p + 2] = chunk[11];

        r_plane[p + 3] = chunk[12];
        g_plane[p + 3] = chunk[13];
        b_plane[p + 3] = chunk[14];
        a_plane[p + 3] = chunk[15];
        p += 4;
    }
    for chunk in remainder.chunks_exact(4) {
        r_plane[p] = chunk[0];
        g_plane[p] = chunk[1];
        b_plane[p] = chunk[2];
        a_plane[p] = chunk[3];
        p += 1;
    }

    // Fast encode each contiguous plane
    let plane_slices = [&r_plane[..], &g_plane[..], &b_plane[..], &a_plane[..]];
    for slice in plane_slices {
        let len = slice.len();
        let mut i = 0;
        while i < len {
            let val = slice[i];
            let mut run_len = 1;
            let val8 = u64::from_ne_bytes([val; 8]);
            while i + run_len + 8 <= len && run_len + 8 <= 127 {
                let chunk = u64::from_ne_bytes(slice[i + run_len..i + run_len + 8].try_into().unwrap());
                if chunk == val8 {
                    run_len += 8;
                } else {
                    break;
                }
            }
            while i + run_len < len && run_len < 127 && slice[i + run_len] == val {
                run_len += 1;
            }

            if run_len >= 3 || val == 0 || val == 255 {
                compressed.push(0x80 | (run_len as u8));
                compressed.push(val);
                i += run_len;
            } else {
                let lit_start = i;
                let mut lit_len = 1;
                while (i + lit_len) < len && lit_len < 127 {
                    let next_val = slice[i + lit_len];
                    if i + lit_len + 2 < len
                        && slice[i + lit_len + 1] == next_val
                        && slice[i + lit_len + 2] == next_val
                    {
                        break;
                    }
                    lit_len += 1;
                }
                compressed.push(lit_len as u8);
                compressed.extend_from_slice(&slice[lit_start..lit_start + lit_len]);
                i += lit_len;
            }
        }
    }
    compressed
}

/// Decompresses Planar BP-RLE frame data into raw RGBA bytes.
pub fn decompress_rgba_frame_planar(compressed: &[u8], out_raw: &mut [u8]) -> bool {
    if compressed.len() < 8 || &compressed[0..4] != b"BFXC" {
        return false;
    }
    let expected_len = u32::from_le_bytes([compressed[4], compressed[5], compressed[6], compressed[7]]) as usize;
    if out_raw.len() < expected_len {
        return false;
    }

    let pixel_count = expected_len / 4;
    let mut planes = vec![0u8; expected_len];
    let (r_plane, rest) = planes.split_at_mut(pixel_count);
    let (g_plane, rest) = rest.split_at_mut(pixel_count);
    let (b_plane, a_plane) = rest.split_at_mut(pixel_count);

    let mut c_idx = 8;
    let plane_dests = [r_plane, g_plane, b_plane, a_plane];

    for dest in plane_dests {
        let mut p_idx = 0;
        let target_len = dest.len();
        while p_idx < target_len && c_idx < compressed.len() {
            let tag = compressed[c_idx];
            c_idx += 1;
            if (tag & 0x80) != 0 {
                let count = ((tag & 0x7F) as usize).min(target_len - p_idx);
                if c_idx >= compressed.len() {
                    break;
                }
                let val = compressed[c_idx];
                c_idx += 1;
                dest[p_idx..p_idx + count].fill(val);
                p_idx += count;
            } else {
                let count = (tag as usize).min(target_len - p_idx);
                let available = compressed.len() - c_idx;
                let actual = count.min(available);
                dest[p_idx..p_idx + actual].copy_from_slice(&compressed[c_idx..c_idx + actual]);
                c_idx += actual;
                p_idx += actual;
            }
        }
    }

    // Re-interleave 4 planes back into RGBA in contiguous chunks
    let (r_plane, rest) = planes.split_at(pixel_count);
    let (g_plane, rest) = rest.split_at(pixel_count);
    let (b_plane, a_plane) = rest.split_at(pixel_count);

    let mut chunks = out_raw.chunks_exact_mut(16);
    let mut p = 0;
    for chunk in &mut chunks {
        chunk[0] = r_plane[p];
        chunk[1] = g_plane[p];
        chunk[2] = b_plane[p];
        chunk[3] = a_plane[p];

        chunk[4] = r_plane[p + 1];
        chunk[5] = g_plane[p + 1];
        chunk[6] = b_plane[p + 1];
        chunk[7] = a_plane[p + 1];

        chunk[8] = r_plane[p + 2];
        chunk[9] = g_plane[p + 2];
        chunk[10] = b_plane[p + 2];
        chunk[11] = a_plane[p + 2];

        chunk[12] = r_plane[p + 3];
        chunk[13] = g_plane[p + 3];
        chunk[14] = b_plane[p + 3];
        chunk[15] = a_plane[p + 3];
        p += 4;
    }
    let remainder = chunks.into_remainder();
    for chunk in remainder.chunks_exact_mut(4) {
        chunk[0] = r_plane[p];
        chunk[1] = g_plane[p];
        chunk[2] = b_plane[p];
        chunk[3] = a_plane[p];
        p += 1;
    }
    true
}

/// Ultra-Fast 32-bit Pixel Run Pack Compressor.
/// Compresses RGBA frame by treating each 4-byte pixel as a single 32-bit word,
/// achieving sub-millisecond compression and instant decompression.
pub fn compress_rgba_frame_ultra(raw_rgba: &[u8]) -> Vec<u8> {
    if raw_rgba.is_empty() {
        return Vec::new();
    }
    let pixel_count = raw_rgba.len() / 4;
    let mut compressed = Vec::with_capacity(raw_rgba.len() / 2);
    compressed.extend_from_slice(b"BFXU");
    compressed.extend_from_slice(&(raw_rgba.len() as u32).to_le_bytes());

    let mut i = 0;
    while i < pixel_count {
        let p_start = i * 4;
        let px = u32::from_le_bytes([raw_rgba[p_start], raw_rgba[p_start + 1], raw_rgba[p_start + 2], raw_rgba[p_start + 3]]);
        let mut run_len = 1;
        while (i + run_len) < pixel_count && run_len < 32767 {
            let next_start = (i + run_len) * 4;
            let next_px = u32::from_le_bytes([raw_rgba[next_start], raw_rgba[next_start + 1], raw_rgba[next_start + 2], raw_rgba[next_start + 3]]);
            if next_px == px {
                run_len += 1;
            } else {
                break;
            }
        }

        if run_len >= 2 {
            let tag = 0x8000u16 | (run_len as u16);
            compressed.extend_from_slice(&tag.to_le_bytes());
            compressed.extend_from_slice(&px.to_le_bytes());
            i += run_len;
        } else {
            let lit_start = i;
            let mut lit_len = 1;
            while (i + lit_len) < pixel_count && lit_len < 32767 {
                let curr_start = (i + lit_len) * 4;
                let curr_px = u32::from_le_bytes([raw_rgba[curr_start], raw_rgba[curr_start + 1], raw_rgba[curr_start + 2], raw_rgba[curr_start + 3]]);
                if (i + lit_len + 1) < pixel_count {
                    let next_start = (i + lit_len + 1) * 4;
                    let next_px = u32::from_le_bytes([raw_rgba[next_start], raw_rgba[next_start + 1], raw_rgba[next_start + 2], raw_rgba[next_start + 3]]);
                    if curr_px == next_px {
                        break;
                    }
                }
                lit_len += 1;
            }
            let tag = lit_len as u16;
            compressed.extend_from_slice(&tag.to_le_bytes());
            compressed.extend_from_slice(&raw_rgba[lit_start * 4..(lit_start + lit_len) * 4]);
            i += lit_len;
        }
    }
    compressed
}

/// Decompresses Ultra-Fast 32-bit Pixel Run Pack into raw RGBA bytes.
pub fn decompress_rgba_frame_ultra(compressed: &[u8], out_raw: &mut [u8]) -> bool {
    if compressed.len() < 8 || &compressed[0..4] != b"BFXU" {
        return false;
    }
    let expected_len = u32::from_le_bytes([compressed[4], compressed[5], compressed[6], compressed[7]]) as usize;
    if out_raw.len() < expected_len {
        return false;
    }

    let pixel_count = expected_len / 4;
    let mut c_idx = 8;
    let mut p_idx = 0;

    while p_idx < pixel_count && c_idx + 2 <= compressed.len() {
        let tag = u16::from_le_bytes([compressed[c_idx], compressed[c_idx + 1]]);
        c_idx += 2;

        if (tag & 0x8000) != 0 {
            let count = ((tag & 0x7FFF) as usize).min(pixel_count - p_idx);
            if c_idx + 4 > compressed.len() {
                break;
            }
            let px_bytes = [compressed[c_idx], compressed[c_idx + 1], compressed[c_idx + 2], compressed[c_idx + 3]];
            c_idx += 4;
            let dest_slice = &mut out_raw[p_idx * 4..(p_idx + count) * 4];
            for chunk in dest_slice.chunks_exact_mut(4) {
                chunk.copy_from_slice(&px_bytes);
            }
            p_idx += count;
        } else {
            let count = (tag as usize).min(pixel_count - p_idx);
            let byte_count = count * 4;
            if c_idx + byte_count > compressed.len() {
                break;
            }
            out_raw[p_idx * 4..(p_idx + count) * 4].copy_from_slice(&compressed[c_idx..c_idx + byte_count]);
            c_idx += byte_count;
            p_idx += count;
        }
    }
    true
}

/// Unified frame compressor (default planar BP-RLE).
pub fn compress_rgba_frame(raw_rgba: &[u8]) -> Vec<u8> {
    compress_rgba_frame_planar(raw_rgba)
}

/// Universal frame decompressor supporting all BeforeFX compression engines.
pub fn decompress_rgba_frame(compressed: &[u8], out_raw: &mut [u8]) -> bool {
    if compressed.len() < 8 {
        if !compressed.is_empty() && out_raw.len() >= compressed.len() {
            out_raw[..compressed.len()].copy_from_slice(compressed);
            return true;
        }
        return false;
    }
    if &compressed[0..4] == b"BFXU" {
        decompress_rgba_frame_ultra(compressed, out_raw)
    } else if &compressed[0..4] == b"BFXC" {
        decompress_rgba_frame_planar(compressed, out_raw)
    } else if out_raw.len() >= compressed.len() {
        out_raw[..compressed.len()].copy_from_slice(compressed);
        true
    } else {
        false
    }
}

pub struct CachedFrame {
    pub compressed: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub raw_bytes: usize,
    pub last_used: u64,
}

pub struct RamPreviewCache {
    pub frames: HashMap<usize, CachedFrame>,
    pub decoded_texture: Option<(usize, Texture2D)>,
    pub decompress_buf: Vec<u8>,
    pub max_frames: usize,
    pub max_memory_mb: f32,
    pub comp_hash: u64,
    pub access_counter: u64,
}

impl RamPreviewCache {
    pub fn new(max_frames: usize, max_memory_mb: f32) -> Self {
        Self {
            frames: HashMap::new(),
            decoded_texture: None,
            decompress_buf: Vec::new(),
            max_frames,
            max_memory_mb,
            comp_hash: 0,
            access_counter: 0,
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.decoded_texture = None;
        self.decompress_buf.clear();
    }

    pub fn get(&mut self, frame_idx: usize) -> Option<&Texture2D> {
        self.access_counter += 1;
        let counter = self.access_counter;
        let is_match = self.decoded_texture.as_ref().map_or(false, |(idx, _)| *idx == frame_idx);
        if is_match {
            if let Some(cf) = self.frames.get_mut(&frame_idx) {
                cf.last_used = counter;
            }
            return self.decoded_texture.as_ref().map(|(_, t)| t);
        }
        if let Some(cf) = self.frames.get_mut(&frame_idx) {
            cf.last_used = counter;
            let expected_len = (cf.width * cf.height * 4) as usize;
            if self.decompress_buf.len() < expected_len {
                self.decompress_buf.resize(expected_len, 0);
            }
            let success = if cf.compressed.starts_with(b"BFXC") || cf.compressed.starts_with(b"BFXU") {
                decompress_rgba_frame(&cf.compressed, &mut self.decompress_buf[..expected_len])
            } else if cf.compressed.len() == expected_len {
                self.decompress_buf[..expected_len].copy_from_slice(&cf.compressed);
                true
            } else {
                false
            };

            if success {
                let img = Image {
                    bytes: self.decompress_buf[..expected_len].to_vec(),
                    width: cf.width as u16,
                    height: cf.height as u16,
                };
                let tex = Texture2D::from_image(&img);
                tex.set_filter(FilterMode::Linear);
                self.decoded_texture = Some((frame_idx, tex));
                return self.decoded_texture.as_ref().map(|(_, t)| t);
            }
        }
        None
    }

    pub fn insert(&mut self, frame_idx: usize, image: &Image, compress: bool, mode: CacheCompressionMode) {
        self.access_counter += 1;
        let raw_len = image.bytes.len();
        let compressed_bytes = if !compress || mode == CacheCompressionMode::Uncompressed {
            image.bytes.clone()
        } else if mode == CacheCompressionMode::UltraFastDirect {
            compress_rgba_frame_ultra(&image.bytes)
        } else {
            compress_rgba_frame_planar(&image.bytes)
        };

        self.frames.insert(
            frame_idx,
            CachedFrame {
                compressed: compressed_bytes,
                width: image.width as u32,
                height: image.height as u32,
                raw_bytes: raw_len,
                last_used: self.access_counter,
            },
        );

        // Enforce both max_frames and max_memory_mb limits via LRU eviction
        while (self.frames.len() > self.max_frames || self.memory_usage_mb() > self.max_memory_mb) && self.frames.len() > 1 {
            if let Some((&lru_key, _)) = self.frames.iter().min_by_key(|(_, cf)| cf.last_used) {
                self.frames.remove(&lru_key);
            } else {
                break;
            }
        }
    }

    pub fn memory_usage_mb(&self) -> f32 {
        let comp_bytes: usize = self.frames.values().map(|cf| cf.compressed.len()).sum();
        let active_tex_bytes: usize = if let Some((_, tex)) = &self.decoded_texture {
            (tex.width() * tex.height() * 4.0) as usize
        } else {
            0
        };
        (comp_bytes + active_tex_bytes) as f32 / (1024.0 * 1024.0)
    }

    pub fn raw_memory_usage_mb(&self) -> f32 {
        let raw_bytes: usize = self.frames.values().map(|cf| cf.raw_bytes).sum();
        raw_bytes as f32 / (1024.0 * 1024.0)
    }

    pub fn compression_ratio(&self) -> f32 {
        let used = self.memory_usage_mb();
        let raw = self.raw_memory_usage_mb();
        if used > 0.01 {
            (raw / used).max(1.0)
        } else {
            1.0
        }
    }
}

fn draw_jetbrains_ram_meter(
    ui: &mut egui::Ui,
    used_mb: f32,
    max_mb: f32,
    cached_frames: usize,
    max_frames: usize,
    compression_ratio: f32,
    history: &[f32],
    purge_requested: &mut bool,
    settings_requested: &mut bool,
) {
    let desired_size = egui::vec2(150.0, 18.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    let fraction = (used_mb / max_mb.max(1.0)).clamp(0.0, 1.0);
    let painter = ui.painter();

    // JetBrains widget background container
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(34, 36, 40));
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 55, 62)),
        egui::StrokeKind::Inside,
    );

    // Fill bar
    let fill_w = rect.width() * fraction;
    if fill_w > 1.0 {
        let fill_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.min.x + fill_w, rect.max.y),
        );
        let bar_color = if fraction < 0.60 {
            egui::Color32::from_rgb(38, 92, 148)
        } else if fraction < 0.85 {
            egui::Color32::from_rgb(175, 120, 30)
        } else {
            egui::Color32::from_rgb(175, 45, 45)
        };
        painter.rect_filled(fill_rect, 2.0, bar_color);
    }

    // Sparkline history curve
    if history.len() >= 2 {
        let max_val = max_mb.max(1.0);
        let step_x = rect.width() / (history.len() - 1) as f32;
        let mut points = Vec::with_capacity(history.len());
        for (i, &val) in history.iter().enumerate() {
            let norm_y = (val / max_val).clamp(0.0, 1.0);
            let px = rect.min.x + i as f32 * step_x;
            let py = rect.max.y - norm_y * (rect.height() - 2.0) - 1.0;
            points.push(egui::pos2(px, py));
        }

        for w in points.windows(2) {
            painter.line_segment(
                [w[0], w[1]],
                egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(135, 205, 255, 160),
                ),
            );
        }
    }

    // Centered readout text
    let text = format!("{:.0}M/{:.0}M ({:.0}x)", used_mb, max_mb, compression_ratio.max(1.0));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::monospace(10.0),
        egui::Color32::from_gray(235),
    );

    let hover_text = format!(
        "RAM Preview Cache: {:.1} MB of {:.1} MB ({:.1}%)\nCached Frames: {} / {} max\nCompression Ratio: {:.1}x\n\n• Click: Run Garbage Collection / Free Memory\n• Right-Click: Open Cache & Compression Settings",
        used_mb, max_mb, fraction * 100.0, cached_frames, max_frames, compression_ratio
    );
    let resp = response.on_hover_text(hover_text);

    if resp.clicked() {
        *purge_requested = true;
    }
    if resp.secondary_clicked() {
        *settings_requested = true;
    }
}

fn draw_large_ram_graph(
    ui: &mut egui::Ui,
    used_mb: f32,
    raw_mb: f32,
    max_mb: f32,
    cached_frames: usize,
    max_frames: usize,
    compression_ratio: f32,
    history: &[f32],
    purge_requested: &mut bool,
    settings_requested: &mut bool,
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("📊 RAM Cache Memory Monitor")
                    .strong()
                    .size(11.5)
                    .color(egui::Color32::from_rgb(100, 195, 255)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("⚙ Limits & Modes")
                    .on_hover_text("Open Cache Capacity, Limits & Compression Settings")
                    .clicked()
                {
                    *settings_requested = true;
                }
                if ui
                    .button("🗑 Free Memory")
                    .on_hover_text("Flush RAM preview frames cache and reclaim memory")
                    .clicked()
                {
                    *purge_requested = true;
                }
            });
        });

        ui.add_space(3.0);

        // Large Graph Box
        let graph_h = 60.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), graph_h),
            egui::Sense::click(),
        );

        {
            let painter = ui.painter();

            // Background
            painter.rect_filled(rect, 3.0, egui::Color32::from_rgb(22, 24, 28));
            painter.rect_stroke(
                rect,
                3.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 48, 55)),
                egui::StrokeKind::Inside,
            );

            // Grid lines (25%, 50%, 75%)
            for pct in [0.25f32, 0.5, 0.75] {
                let gy = rect.max.y - pct * rect.height();
                painter.line_segment(
                    [egui::pos2(rect.min.x, gy), egui::pos2(rect.max.x, gy)],
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(36, 38, 45)),
                );
            }

            // Filled area & Sparkline
            if history.len() >= 2 {
                let max_val = max_mb.max(1.0);
                let step_x = rect.width() / (history.len() - 1) as f32;
                let mut top_pts = Vec::with_capacity(history.len());

                for (i, &val) in history.iter().enumerate() {
                    let norm_y = (val / max_val).clamp(0.0, 1.0);
                    let px = rect.min.x + i as f32 * step_x;
                    let py = rect.max.y - norm_y * (rect.height() - 4.0) - 2.0;
                    top_pts.push(egui::pos2(px, py));
                }

                // Fill polygons
                for i in 0..top_pts.len() - 1 {
                    let p0 = top_pts[i];
                    let p1 = top_pts[i + 1];
                    let b0 = egui::pos2(p0.x, rect.max.y - 1.0);
                    let b1 = egui::pos2(p1.x, rect.max.y - 1.0);
                    painter.add(egui::Shape::convex_polygon(
                        vec![p0, p1, b1, b0],
                        egui::Color32::from_rgba_unmultiplied(40, 110, 200, 45),
                        egui::Stroke::NONE,
                    ));
                }

                // Stroke line
                for w in top_pts.windows(2) {
                    painter.line_segment(
                        [w[0], w[1]],
                        egui::Stroke::new(1.5, egui::Color32::from_rgb(65, 160, 255)),
                    );
                }
            }
        }

        // Hover & Click interaction
        let fraction = (used_mb / max_mb.max(1.0)).clamp(0.0, 1.0);
        let hover_text = format!(
            "RAM Usage: {:.1} MB (Raw: {:.1} MB, {:.1}x saved)\nCapacity: {:.1} MB ({:.1}%)\nCached Frames: {} / {} max\n\nClick to Free Memory / Run GC",
            used_mb, raw_mb, compression_ratio, max_mb, fraction * 100.0, cached_frames, max_frames
        );
        let resp = response.on_hover_text(hover_text);
        if resp.clicked() {
            *purge_requested = true;
        }

        // Stats readout under graph
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{:.1} MB used ({:.1}x saved)", used_mb, compression_ratio))
                    .color(egui::Color32::from_rgb(70, 215, 125))
                    .strong()
                    .size(11.0),
            );
            ui.label(
                egui::RichText::new(format!("/ {:.1} MB limit", max_mb))
                    .color(egui::Color32::from_gray(140))
                    .size(11.0),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1}% ({} frames)",
                        fraction * 100.0,
                        cached_frames
                    ))
                    .color(egui::Color32::from_gray(170))
                    .size(11.0),
                );
            });
        });

        // Progress Bar
        ui.add_space(2.0);
        let bar_color = if fraction < 0.60 {
            egui::Color32::from_rgb(45, 140, 220)
        } else if fraction < 0.85 {
            egui::Color32::from_rgb(220, 160, 40)
        } else {
            egui::Color32::from_rgb(220, 60, 60)
        };
        let (bar_rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 6.0),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(bar_rect, 1.5, egui::Color32::from_rgb(32, 34, 38));
        let filled_rect = egui::Rect::from_min_max(
            bar_rect.min,
            egui::pos2(bar_rect.min.x + bar_rect.width() * fraction, bar_rect.max.y),
        );
        ui.painter().rect_filled(filled_rect, 1.5, bar_color);
    });
}

pub fn compute_comp_content_hash(comp: &Composition) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut s = DefaultHasher::new();
    comp.settings.width.hash(&mut s);
    comp.settings.height.hash(&mut s);
    comp.settings.fps.hash(&mut s);
    ((comp.settings.duration * 1000.0) as i64).hash(&mut s);
    (comp.viewport_mode as u8).hash(&mut s);
    ((comp.custom_orbit_yaw * 10.0) as i32).hash(&mut s);
    ((comp.custom_orbit_pitch * 10.0) as i32).hash(&mut s);
    ((comp.custom_orbit_distance * 10.0) as i32).hash(&mut s);
    ((comp.custom_orbit_roll * 10.0) as i32).hash(&mut s);
    comp.layers.len().hash(&mut s);
    for l in &comp.layers {
        l.name.hash(&mut s);
        l.visible.hash(&mut s);
        l.locked.hash(&mut s);
        l.solo.hash(&mut s);
        l.fx.hash(&mut s);
        l.d3.hash(&mut s);
        l.shy.hash(&mut s);
        l.blend_mode.hash(&mut s);
        l.track_matte.hash(&mut s);
        l.parent_index.hash(&mut s);
        ((l.in_time * 1000.0) as i64).hash(&mut s);
        ((l.out_time * 1000.0) as i64).hash(&mut s);
        match &l.source {
            LayerSource::Solid { color } => {
                0u8.hash(&mut s);
                for c in color {
                    ((c * 1000.0) as i32).hash(&mut s);
                }
            }
            LayerSource::Image { path } => {
                1u8.hash(&mut s);
                path.hash(&mut s);
            }
            LayerSource::Audio { path } => {
                2u8.hash(&mut s);
                path.hash(&mut s);
            }
            LayerSource::Video { path } => {
                3u8.hash(&mut s);
                path.hash(&mut s);
            }
            LayerSource::Object3D { path, color } => {
                4u8.hash(&mut s);
                path.hash(&mut s);
                for c in color {
                    ((c * 1000.0) as i32).hash(&mut s);
                }
            }
            LayerSource::Polygon { points, color } => {
                5u8.hash(&mut s);
                points.len().hash(&mut s);
                for c in color {
                    ((c * 1000.0) as i32).hash(&mut s);
                }
            }
            LayerSource::Text { text, font_size, color } => {
                6u8.hash(&mut s);
                text.hash(&mut s);
                ((font_size * 10.0) as i32).hash(&mut s);
                for c in color {
                    ((c * 1000.0) as i32).hash(&mut s);
                }
            }
            LayerSource::Adjustment => {
                7u8.hash(&mut s);
            }
            LayerSource::Null => {
                8u8.hash(&mut s);
            }
            LayerSource::Camera => {
                9u8.hash(&mut s);
            }
        }
        l.effects.len().hash(&mut s);
        for eff in &l.effects {
            eff.id.hash(&mut s);
            eff.name.hash(&mut s);
            eff.effect_type.hash(&mut s);
            eff.enabled.hash(&mut s);
            for (pk, pv) in &eff.properties {
                pk.hash(&mut s);
                ((pv.base_value * 100.0) as i64).hash(&mut s);
                pv.keyframes.len().hash(&mut s);
                for kf in &pv.keyframes {
                    ((kf.time * 1000.0) as i64).hash(&mut s);
                    ((kf.value * 100.0) as i64).hash(&mut s);
                }
            }
        }
        for (pk, pv) in &l.properties {
            pk.hash(&mut s);
            ((pv.base_value * 100.0) as i64).hash(&mut s);
            if let Some(w) = &pv.wiggle {
                w.enabled.hash(&mut s);
                ((w.freq * 100.0) as i64).hash(&mut s);
                ((w.amp * 100.0) as i64).hash(&mut s);
            }
            pv.keyframes.len().hash(&mut s);
            for kf in &pv.keyframes {
                ((kf.time * 1000.0) as i64).hash(&mut s);
                ((kf.value * 100.0) as i64).hash(&mut s);
            }
        }
    }
    s.finish()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GizmoHandle {
    None,
    TranslateX,
    TranslateY,
    TranslateZ,
    RotateX,
    RotateY,
    RotateZ,
    CenterAnchor,
    CameraOrbit,
    CameraPan,
    CameraDolly,
    Translate2D,
    Rotate2D,
    ScaleTL,
    ScaleTR,
    ScaleBL,
    ScaleBR,
    ScaleT,
    ScaleB,
    ScaleL,
    ScaleR,
    PanBehindAnchor,
    HandPan,
    ZoomPan,
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = (p.x - b.x) * (a.y - b.y) - (a.x - b.x) * (p.y - b.y);
    let d2 = (p.x - c.x) * (b.y - c.y) - (b.x - c.x) * (p.y - c.y);
    let d3 = (p.x - a.x) * (c.y - a.y) - (c.x - a.x) * (p.y - a.y);
    let has_neg = (d1 < -0.001) || (d2 < -0.001) || (d3 < -0.001);
    let has_pos = (d1 > 0.001) || (d2 > 0.001) || (d3 > 0.001);
    !(has_neg && has_pos)
}

fn get_layer_2d_bounds(
    comp: &Composition,
    layer_idx: usize,
    textures: &HashMap<String, Texture2D>,
    time: f32,
) -> (Vec2, Vec2, Vec2, Vec2, Vec2) {
    let (ax, ay, x, y, _, rot, _, _, sx, sy) = layer_transform(comp, layer_idx, time);
    let layer = &comp.layers[layer_idx];
    let (w, h) = match &layer.source {
        LayerSource::Solid { .. } => (200.0, 200.0),
        LayerSource::Image { path } => {
            if let Some(tex) = textures.get(path) {
                (tex.width(), tex.height())
            } else {
                (200.0, 200.0)
            }
        }
        LayerSource::Text { text, font_size, .. } => {
            let width = (text.len() as f32 * font_size * 0.58).max(40.0);
            let height = (*font_size * 1.15).max(20.0);
            (width, height)
        }
        LayerSource::Video { .. } => (480.0, 270.0),
        LayerSource::Polygon { points, .. } => {
            if points.is_empty() {
                (100.0, 100.0)
            } else {
                let mut min_x = points[0][0];
                let mut max_x = points[0][0];
                let mut min_y = points[0][1];
                let mut max_y = points[0][1];
                for p in points {
                    min_x = min_x.min(p[0]);
                    max_x = max_x.max(p[0]);
                    min_y = min_y.min(p[1]);
                    max_y = max_y.max(p[1]);
                }
                ((max_x - min_x).max(20.0), (max_y - min_y).max(20.0))
            }
        }
        _ => (200.0, 200.0),
    };

    let rad = rot.to_radians();
    let cos_r = rad.cos();
    let sin_r = rad.sin();
    let rotate = |lx: f32, ly: f32| -> Vec2 {
        let scaled_x = (lx - ax) * sx;
        let scaled_y = (ly - ay) * sy;
        vec2(
            x + scaled_x * cos_r - scaled_y * sin_r,
            y + scaled_x * sin_r + scaled_y * cos_r,
        )
    };

    let tl = rotate(0.0, 0.0);
    let tr = rotate(w, 0.0);
    let br = rotate(w, h);
    let bl = rotate(0.0, h);
    let center = vec2(x, y);

    (center, tl, tr, br, bl)
}

fn hit_test_layers(
    comp: &Composition,
    textures: &HashMap<String, Texture2D>,
    mouse_comp_pos: Vec2,
    time: f32,
) -> Option<usize> {
    let width = comp.settings.width as f32;
    let height = comp.settings.height as f32;
    let active_camera = get_viewport_camera(comp, time);
    let has_solo = comp.layers.iter().any(|l| l.solo);

    for (layer_idx, layer) in comp.layers.iter().enumerate().rev() {
        if !layer.visible || layer.locked {
            continue;
        }
        if has_solo && !layer.solo {
            continue;
        }
        if time < layer.in_time || time > layer.out_time {
            continue;
        }

        if layer.d3 {
            // 3D Layer Hit Test
            let corners = match &layer.source {
                LayerSource::Solid { .. } => vec![
                    transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                    transform_local_to_world(comp, layer_idx, vec3(200.0, 0.0, 0.0), time),
                    transform_local_to_world(comp, layer_idx, vec3(200.0, 200.0, 0.0), time),
                    transform_local_to_world(comp, layer_idx, vec3(0.0, 200.0, 0.0), time),
                ],
                LayerSource::Image { path } => {
                    let (tw, th) = textures.get(path).map_or((200.0, 200.0), |t| (t.width(), t.height()));
                    vec![
                        transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(tw, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(tw, th, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(0.0, th, 0.0), time),
                    ]
                }
                LayerSource::Object3D { .. } => {
                    let s = 100.0;
                    vec![
                        transform_local_to_world(comp, layer_idx, vec3(-s, -s, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(s, -s, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(s, s, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(-s, s, 0.0), time),
                    ]
                }
                LayerSource::Camera => {
                    let pos_world = transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time);
                    let p = project_3d_point(pos_world, &active_camera, width, height);
                    if p.visible && (p.screen - mouse_comp_pos).length() < 30.0 {
                        return Some(layer_idx);
                    }
                    continue;
                }
                _ => vec![
                    transform_local_to_world(comp, layer_idx, vec3(-100.0, -100.0, 0.0), time),
                    transform_local_to_world(comp, layer_idx, vec3(100.0, -100.0, 0.0), time),
                    transform_local_to_world(comp, layer_idx, vec3(100.0, 100.0, 0.0), time),
                    transform_local_to_world(comp, layer_idx, vec3(-100.0, 100.0, 0.0), time),
                ],
            };

            let proj: Vec<ProjectedPoint> = corners
                .iter()
                .map(|&c| project_3d_point(c, &active_camera, width, height))
                .collect();

            if proj.len() == 4 && proj.iter().all(|p| p.visible) {
                let p0 = proj[0].screen;
                let p1 = proj[1].screen;
                let p2 = proj[2].screen;
                let p3 = proj[3].screen;
                if point_in_triangle(mouse_comp_pos, p0, p1, p2)
                    || point_in_triangle(mouse_comp_pos, p0, p2, p3)
                {
                    return Some(layer_idx);
                }
            }
        } else {
            // 2D Layer Hit Test
            let (_center, tl, tr, br, bl) = get_layer_2d_bounds(comp, layer_idx, textures, time);
            if point_in_triangle(mouse_comp_pos, tl, tr, br)
                || point_in_triangle(mouse_comp_pos, tl, br, bl)
            {
                return Some(layer_idx);
            }
            // Origin check
            let (_, _, x, y, _, _, _, _, _, _) = layer_transform(comp, layer_idx, time);
            if (vec2(x, y) - mouse_comp_pos).length() < 16.0 {
                return Some(layer_idx);
            }
        }
    }
    None
}

fn update_layer_property_val(layer: &mut Layer, prop_name: &str, new_val: f32, current_time: f32) {
    if let Some(prop) = layer.properties.get_mut(prop_name) {
        if !prop.keyframes.is_empty() {
            let mut found = false;
            for kf in &mut prop.keyframes {
                if (kf.time - current_time).abs() < 0.033 {
                    kf.value = new_val;
                    found = true;
                    break;
                }
            }
            if !found {
                prop.keyframes.push(Keyframe {
                    time: current_time,
                    value: new_val,
                    ease: None,
                });
                prop.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
            }
        } else {
            prop.base_value = new_val;
        }
    }
}

fn dist_to_segment_2d(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 0.0001 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    (p - proj).length()
}

#[derive(Clone, Copy, Debug)]
pub struct ProjectedPoint {
    pub screen: Vec2,
    pub depth: f32,
    pub visible: bool,
}

pub fn transform_local_to_world(
    comp: &Composition,
    layer_idx: usize,
    local_pt: Vec3,
    time: f32,
) -> Vec3 {
    let layer = &comp.layers[layer_idx];
    let ax = layer.properties.get("anchorX").map_or(0.0, |p| p.get_value_at(time));
    let ay = layer.properties.get("anchorY").map_or(0.0, |p| p.get_value_at(time));
    let az = layer.properties.get("anchorZ").map_or(0.0, |p| p.get_value_at(time));

    let x = layer.properties.get("x").map_or(960.0, |p| p.get_value_at(time));
    let y = layer.properties.get("y").map_or(540.0, |p| p.get_value_at(time));
    let z = layer.properties.get("z").map_or(0.0, |p| p.get_value_at(time));

    let rot_x = layer.properties.get("rotationX").map_or(0.0, |p| p.get_value_at(time));
    let rot_y = layer.properties.get("rotationY").map_or(0.0, |p| p.get_value_at(time));
    let rot_z = layer.properties.get("rotation").map_or(0.0, |p| p.get_value_at(time));

    let sx = layer.properties.get("scaleX").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
    let sy = layer.properties.get("scaleY").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
    let sz = layer.properties.get("scaleZ").map_or(100.0, |p| p.get_value_at(time)) / 100.0;

    let p0 = (local_pt - vec3(ax, ay, az)) * vec3(sx, sy, sz);

    let rx = rot_x.to_radians();
    let ry = rot_y.to_radians();
    let rz = rot_z.to_radians();

    let p_rx = vec3(
        p0.x,
        p0.y * rx.cos() - p0.z * rx.sin(),
        p0.y * rx.sin() + p0.z * rx.cos(),
    );

    let p_ry = vec3(
        p_rx.x * ry.cos() + p_rx.z * ry.sin(),
        p_rx.y,
        -p_rx.x * ry.sin() + p_rx.z * ry.cos(),
    );

    let p_rz = vec3(
        p_ry.x * rz.cos() - p_ry.y * rz.sin(),
        p_ry.x * rz.sin() + p_ry.y * rz.cos(),
        p_ry.z,
    );

    let mut world_p = p_rz + vec3(x, y, z);

    if let Some(parent_idx) = layer.parent_index {
        if parent_idx < comp.layers.len() && parent_idx != layer_idx {
            world_p = transform_local_to_world(comp, parent_idx, world_p, time);
        }
    }

    world_p
}

pub fn project_3d_point(
    world_p: Vec3,
    camera: &ActiveCamera,
    comp_width: f32,
    comp_height: f32,
) -> ProjectedPoint {
    let d = camera.target - camera.position;
    let forward = if d.length() > 0.0001 {
        d.normalize()
    } else {
        vec3(0.0, 0.0, 1.0)
    };

    let up_hint = vec3(0.0, 1.0, 0.0);
    let right = if forward.y.abs() > 0.999 {
        vec3(1.0, 0.0, 0.0)
    } else {
        forward.cross(up_hint).normalize()
    };
    let up = right.cross(forward).normalize();

    let cam_rx = camera.rotation.x.to_radians();
    let cam_ry = camera.rotation.y.to_radians();
    let cam_rz = camera.rotation.z.to_radians();

    let v = world_p - camera.position;

    let mut x_c = v.dot(right);
    let mut y_c = v.dot(up);
    let mut z_c = v.dot(forward);

    if cam_rz.abs() > 0.0001 {
        let xr = x_c * cam_rz.cos() - y_c * cam_rz.sin();
        let yr = x_c * cam_rz.sin() + y_c * cam_rz.cos();
        x_c = xr;
        y_c = yr;
    }
    if cam_rx.abs() > 0.0001 {
        let yr = y_c * cam_rx.cos() - z_c * cam_rx.sin();
        let zr = y_c * cam_rx.sin() + z_c * cam_rx.cos();
        y_c = yr;
        z_c = zr;
    }
    if cam_ry.abs() > 0.0001 {
        let xr = x_c * cam_ry.cos() + z_c * cam_ry.sin();
        let zr = -x_c * cam_ry.sin() + z_c * cam_ry.cos();
        x_c = xr;
        z_c = zr;
    }

    if z_c <= 1.0 {
        return ProjectedPoint {
            screen: vec2(comp_width / 2.0, comp_height / 2.0),
            depth: z_c,
            visible: false,
        };
    }

    let scale_factor = camera.zoom / z_c;
    let screen_x = comp_width / 2.0 + x_c * scale_factor;
    let screen_y = comp_height / 2.0 + y_c * scale_factor;

    ProjectedPoint {
        screen: vec2(screen_x, screen_y),
        depth: z_c,
        visible: true,
    }
}

fn layer_transform(
    comp: &Composition,
    layer_idx: usize,
    time: f32,
) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) {
    let layer = &comp.layers[layer_idx];
    let ax = layer.properties.get("anchorX").map_or(0.0, |p| p.get_value_at(time));
    let ay = layer.properties.get("anchorY").map_or(0.0, |p| p.get_value_at(time));
    let mut x = layer.properties.get("x").map_or(960.0, |p| p.get_value_at(time));
    let mut y = layer.properties.get("y").map_or(540.0, |p| p.get_value_at(time));
    let mut z = layer.properties.get("z").map_or(0.0, |p| p.get_value_at(time));
    let mut rot = layer.properties.get("rotation").map_or(0.0, |p| p.get_value_at(time));
    let rot_x = layer.properties.get("rotationX").map_or(0.0, |p| p.get_value_at(time));
    let rot_y = layer.properties.get("rotationY").map_or(0.0, |p| p.get_value_at(time));
    let mut sx = layer.properties.get("scaleX").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
    let mut sy = layer.properties.get("scaleY").map_or(100.0, |p| p.get_value_at(time)) / 100.0;

    // Parent transformation inheritance
    if let Some(parent_idx) = layer.parent_index {
        if parent_idx < comp.layers.len() && parent_idx != layer_idx {
            let (p_ax, p_ay, p_x, p_y, p_z, p_rot, _, _, p_sx, p_sy) =
                layer_transform(comp, parent_idx, time);
            x += p_x - p_ax;
            y += p_y - p_ay;
            z += p_z;
            rot += p_rot;
            sx *= p_sx;
            sy *= p_sy;
        }
    }

    (ax, ay, x, y, z, rot, rot_x, rot_y, sx, sy)
}

fn draw_polygon_layer(points: &[[f32; 2]], x: f32, y: f32, sx: f32, sy: f32, color: Color) {
    if points.len() < 3 {
        return;
    }
    let origin = vec2(x, y);
    let verts: Vec<Vec2> = points
        .iter()
        .map(|p| origin + vec2(p[0] * sx, p[1] * sy))
        .collect();
    for i in 1..verts.len() - 1 {
        draw_triangle(verts[0], verts[i], verts[i + 1], color);
    }
    for i in 0..verts.len() {
        draw_line(
            verts[i].x,
            verts[i].y,
            verts[(i + 1) % verts.len()].x,
            verts[(i + 1) % verts.len()].y,
            2.0,
            Color::new(1.0, 1.0, 1.0, color.a.min(0.4)),
        );
    }
}

fn draw_video_placeholder(path: &str, x: f32, y: f32, sx: f32, sy: f32, opacity: f32) {
    let w = 480.0 * sx.abs().max(0.1);
    let h = 270.0 * sy.abs().max(0.1);
    draw_rectangle(x, y, w, h, Color::new(0.04, 0.04, 0.05, opacity));
    draw_rectangle_lines(x, y, w, h, 2.5, Color::new(0.2, 0.6, 0.9, opacity));
    let label = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".to_string());
    draw_text(
        "🎬 VIDEO FOOTAGE",
        x + 20.0,
        y + 45.0,
        32.0,
        Color::new(0.65, 0.85, 1.0, opacity),
    );
    draw_text(
        &label,
        x + 20.0,
        y + h - 25.0,
        22.0,
        Color::new(0.85, 0.9, 0.95, opacity),
    );
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-5 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let mut h = if (max - r).abs() < 1e-5 {
        (g - b) / d + (if g < b { 6.0 } else { 0.0 })
    } else if (max - g).abs() < 1e-5 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h /= 6.0;
    (h * 360.0, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < 1e-5 {
        return (l, l, l);
    }
    let h = (h % 360.0 + 360.0) % 360.0 / 360.0;
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hue_to_rgb = |p: f32, q: f32, mut t: f32| -> f32 {
        if t < 0.0 { t += 1.0; }
        if t > 1.0 { t -= 1.0; }
        if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
        if t < 1.0 / 2.0 { return q; }
        if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
        p
    };
    (
        hue_to_rgb(p, q, h + 1.0 / 3.0).clamp(0.0, 1.0),
        hue_to_rgb(p, q, h).clamp(0.0, 1.0),
        hue_to_rgb(p, q, h - 1.0 / 3.0).clamp(0.0, 1.0),
    )
}

fn apply_color_effects(col: Color, layer: &Layer, time: f32) -> Color {
    apply_color_effects_with_plugins(col, layer, time, None)
}

fn apply_color_effects_with_plugins(
    mut col: Color,
    layer: &Layer,
    time: f32,
    plugins: Option<&PluginRegistry>,
) -> Color {
    if !layer.fx {
        return col;
    }
    for eff in &layer.effects {
        if !eff.enabled {
            continue;
        }
        match &eff.effect_type {
            EffectType::Fill => {
                let r = eff.properties.get("colorR").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                let g = eff.properties.get("colorG").map_or(100.0, |p| p.get_value_at(time)) / 255.0;
                let b = eff.properties.get("colorB").map_or(50.0, |p| p.get_value_at(time)) / 255.0;
                let amount = eff.properties.get("opacity").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                col.r = col.r * (1.0 - amount) + r * amount;
                col.g = col.g * (1.0 - amount) + g * amount;
                col.b = col.b * (1.0 - amount) + b * amount;
            }
            EffectType::Tint => {
                let amount = eff.properties.get("amount").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                let blk_r = eff.properties.get("blackR").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
                let blk_g = eff.properties.get("blackG").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
                let blk_b = eff.properties.get("blackB").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
                let wht_r = eff.properties.get("whiteR").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                let wht_g = eff.properties.get("whiteG").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                let wht_b = eff.properties.get("whiteB").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                let lum = col.r * 0.299 + col.g * 0.587 + col.b * 0.114;
                let tinted_r = blk_r + lum * (wht_r - blk_r);
                let tinted_g = blk_g + lum * (wht_g - blk_g);
                let tinted_b = blk_b + lum * (wht_b - blk_b);
                col.r = col.r * (1.0 - amount) + tinted_r * amount;
                col.g = col.g * (1.0 - amount) + tinted_g * amount;
                col.b = col.b * (1.0 - amount) + tinted_b * amount;
            }
            EffectType::BrightnessContrast => {
                let br = eff.properties.get("brightness").map_or(0.0, |p| p.get_value_at(time)) / 100.0;
                let ct = eff.properties.get("contrast").map_or(0.0, |p| p.get_value_at(time)) / 100.0;
                let factor = (1.0 + ct).max(0.0);
                col.r = ((col.r - 0.5) * factor + 0.5 + br).clamp(0.0, 1.0);
                col.g = ((col.g - 0.5) * factor + 0.5 + br).clamp(0.0, 1.0);
                col.b = ((col.b - 0.5) * factor + 0.5 + br).clamp(0.0, 1.0);
            }
            EffectType::Invert => {
                let amount = eff.properties.get("blend").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                let inv_r = 1.0 - col.r;
                let inv_g = 1.0 - col.g;
                let inv_b = 1.0 - col.b;
                col.r = col.r * (1.0 - amount) + inv_r * amount;
                col.g = col.g * (1.0 - amount) + inv_g * amount;
                col.b = col.b * (1.0 - amount) + inv_b * amount;
            }
            EffectType::HueSaturation => {
                let hue_deg = eff.properties.get("hue").map_or(0.0, |p| p.get_value_at(time));
                let sat = eff.properties.get("saturation").map_or(0.0, |p| p.get_value_at(time)) / 100.0;
                let light = eff.properties.get("lightness").map_or(0.0, |p| p.get_value_at(time)) / 100.0;
                let (h, s, l) = rgb_to_hsl(col.r, col.g, col.b);
                let new_h = h + hue_deg;
                let new_s = (s * (1.0 + sat)).clamp(0.0, 1.0);
                let new_l = (l + light).clamp(0.0, 1.0);
                let (r, g, b) = hsl_to_rgb(new_h, new_s, new_l);
                col.r = r;
                col.g = g;
                col.b = b;
            }
            EffectType::Glow => {
                let intens = eff.properties.get("intensity").map_or(1.0, |p| p.get_value_at(time));
                let thresh = eff.properties.get("threshold").map_or(50.0, |p| p.get_value_at(time)) / 100.0;
                let lum = col.r * 0.299 + col.g * 0.587 + col.b * 0.114;
                if lum >= thresh {
                    let boost = ((lum - thresh) / (1.0 - thresh).max(0.01)) * intens * 0.4;
                    col.r = (col.r + boost).clamp(0.0, 1.0);
                    col.g = (col.g + boost).clamp(0.0, 1.0);
                    col.b = (col.b + boost).clamp(0.0, 1.0);
                }
            }
            EffectType::Plugin(plugin_name) => {
                if let Some(reg) = plugins {
                    if let Some(plugin) = reg.get_effect(plugin_name).or_else(|| reg.get_effect(&eff.name)) {
                        col = apply_effect_plugin(col, plugin, &eff.properties, time);
                    }
                } else {
                    let p = PathBuf::from(format!("./plugins/{}.bfxplugin", crate::plugin::sanitize_id(plugin_name)));
                    if let Ok(Some(Plugin::Effect(plugin))) = crate::plugin::parse_plugin_file(&p) {
                        col = apply_effect_plugin(col, &plugin, &eff.properties, time);
                    }
                }
            }
            _ => {}
        }
    }
    col
}

fn draw_composition(
    comp: &Composition,
    textures: &HashMap<String, Texture2D>,
    time: f32,
    target: RenderTarget,
) {
    let width = comp.settings.width as f32;
    let height = comp.settings.height as f32;

    set_camera(&Camera2D {
        render_target: Some(target.clone()),
        ..Camera2D::from_display_rect(Rect::new(0., 0., width, height))
    });
    clear_background(Color::from_rgba(20, 20, 22, 255));

    let active_camera = get_viewport_camera(comp, time);
    let has_solo = comp.layers.iter().any(|l| l.solo);

    // 3D Perspective Ground Grid (in non-ActiveCamera views or when grid is enabled)
    if comp.viewport_mode != ViewportMode::ActiveCamera || comp.show_grid {
        let grid_y = comp.settings.height as f32 / 2.0;
        let step = 200.0;
        let grid_min_x = -(comp.settings.width as f32);
        let grid_max_x = comp.settings.width as f32 * 2.0;
        let grid_min_z = -3000.0;
        let grid_max_z = 3000.0;

        let mut gx = grid_min_x;
        while gx <= grid_max_x {
            let p1 = project_3d_point(vec3(gx, grid_y, grid_min_z), &active_camera, width, height);
            let p2 = project_3d_point(vec3(gx, grid_y, grid_max_z), &active_camera, width, height);
            if p1.visible && p2.visible {
                let is_axis = (gx - comp.settings.width as f32 / 2.0).abs() < 1.0;
                let col = if is_axis {
                    Color::from_rgba(220, 60, 60, 110)
                } else {
                    Color::from_rgba(65, 65, 80, 55)
                };
                draw_line(p1.screen.x, p1.screen.y, p2.screen.x, p2.screen.y, if is_axis { 1.5 } else { 0.8 }, col);
            }
            gx += step;
        }

        let mut gz = grid_min_z;
        while gz <= grid_max_z {
            let p1 = project_3d_point(vec3(grid_min_x, grid_y, gz), &active_camera, width, height);
            let p2 = project_3d_point(vec3(grid_max_x, grid_y, gz), &active_camera, width, height);
            if p1.visible && p2.visible {
                let is_axis = gz.abs() < 1.0;
                let col = if is_axis {
                    Color::from_rgba(60, 130, 240, 110)
                } else {
                    Color::from_rgba(65, 65, 80, 55)
                };
                draw_line(p1.screen.x, p1.screen.y, p2.screen.x, p2.screen.y, if is_axis { 1.5 } else { 0.8 }, col);
            }
            gz += step;
        }
    }

    for (layer_idx, layer) in comp.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        if has_solo && !layer.solo {
            continue;
        }
        // In/Out Trimming check
        if time < layer.in_time || time > layer.out_time {
            continue;
        }

        let op = layer
            .properties
            .get("opacity")
            .map_or(100.0, |p| p.get_value_at(time))
            / 100.0;

        if layer.d3 {
            // ==========================================
            // 3D LAYER PERSPECTIVE RENDERING PASS
            // ==========================================
            match &layer.source {
                LayerSource::Solid { color } => {
                    let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                    let final_col = apply_color_effects(base_col, layer, time);

                    let corners = [
                        transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(200.0, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(200.0, 200.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(0.0, 200.0, 0.0), time),
                    ];
                    let proj = [
                        project_3d_point(corners[0], &active_camera, width, height),
                        project_3d_point(corners[1], &active_camera, width, height),
                        project_3d_point(corners[2], &active_camera, width, height),
                        project_3d_point(corners[3], &active_camera, width, height),
                    ];
                    if proj.iter().all(|p| p.visible) {
                        draw_triangle(proj[0].screen, proj[1].screen, proj[2].screen, final_col);
                        draw_triangle(proj[0].screen, proj[2].screen, proj[3].screen, final_col);
                        for i in 0..4 {
                            draw_line(
                                proj[i].screen.x,
                                proj[i].screen.y,
                                proj[(i + 1) % 4].screen.x,
                                proj[(i + 1) % 4].screen.y,
                                1.5,
                                Color::new(1.0, 1.0, 1.0, op * 0.3),
                            );
                        }
                    }
                }
                LayerSource::Image { path } => {
                    if let Some(tex) = textures.get(path) {
                        let tw = tex.width();
                        let th = tex.height();
                        let base_col = Color::new(1.0, 1.0, 1.0, op);
                        let final_col = apply_color_effects(base_col, layer, time);

                        let corners = [
                            transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                            transform_local_to_world(comp, layer_idx, vec3(tw, 0.0, 0.0), time),
                            transform_local_to_world(comp, layer_idx, vec3(tw, th, 0.0), time),
                            transform_local_to_world(comp, layer_idx, vec3(0.0, th, 0.0), time),
                        ];
                        let proj = [
                            project_3d_point(corners[0], &active_camera, width, height),
                            project_3d_point(corners[1], &active_camera, width, height),
                            project_3d_point(corners[2], &active_camera, width, height),
                            project_3d_point(corners[3], &active_camera, width, height),
                        ];
                        if proj.iter().all(|p| p.visible) {
                            let mesh = Mesh {
                                vertices: vec![
                                    Vertex {
                                        position: vec3(proj[0].screen.x, proj[0].screen.y, 0.0),
                                        uv: vec2(0.0, 0.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                    Vertex {
                                        position: vec3(proj[1].screen.x, proj[1].screen.y, 0.0),
                                        uv: vec2(1.0, 0.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                    Vertex {
                                        position: vec3(proj[2].screen.x, proj[2].screen.y, 0.0),
                                        uv: vec2(1.0, 1.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                    Vertex {
                                        position: vec3(proj[3].screen.x, proj[3].screen.y, 0.0),
                                        uv: vec2(0.0, 1.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                ],
                                indices: vec![0, 1, 2, 0, 2, 3],
                                texture: Some(tex.clone()),
                            };
                            draw_mesh(&mesh);
                        }
                    }
                }
                LayerSource::Polygon { points, color } => {
                    if points.len() >= 3 {
                        let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                        let final_col = apply_color_effects(base_col, layer, time);

                        let proj_pts: Vec<ProjectedPoint> = points
                            .iter()
                            .map(|pt| {
                                let w_pt = transform_local_to_world(
                                    comp,
                                    layer_idx,
                                    vec3(pt[0], pt[1], 0.0),
                                    time,
                                );
                                project_3d_point(w_pt, &active_camera, width, height)
                            })
                            .collect();

                        if proj_pts.iter().all(|p| p.visible) {
                            for i in 1..proj_pts.len() - 1 {
                                draw_triangle(
                                    proj_pts[0].screen,
                                    proj_pts[i].screen,
                                    proj_pts[i + 1].screen,
                                    final_col,
                                );
                            }
                            for i in 0..proj_pts.len() {
                                draw_line(
                                    proj_pts[i].screen.x,
                                    proj_pts[i].screen.y,
                                    proj_pts[(i + 1) % proj_pts.len()].screen.x,
                                    proj_pts[(i + 1) % proj_pts.len()].screen.y,
                                    2.0,
                                    Color::new(1.0, 1.0, 1.0, op * 0.4),
                                );
                            }
                        }
                    }
                }
                LayerSource::Text { text, font_size, color } => {
                    let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                    let final_col = apply_color_effects(base_col, layer, time);

                    let origin_world = transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time);
                    let proj = project_3d_point(origin_world, &active_camera, width, height);

                    if proj.visible {
                        let scale_factor = (active_camera.zoom / proj.depth.max(1.0)).clamp(0.05, 10.0);
                        let size = (*font_size * scale_factor).max(6.0);
                        draw_text(text, proj.screen.x, proj.screen.y, size, final_col);
                    }
                }
                LayerSource::Object3D { color, .. } => {
                    let s = 100.0;
                    let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                    let final_col = apply_color_effects(base_col, layer, time);

                    // 8 Local Cube Vertices
                    let cube_verts = [
                        vec3(-s, -s, -s), // 0
                        vec3( s, -s, -s), // 1
                        vec3( s,  s, -s), // 2
                        vec3(-s,  s, -s), // 3
                        vec3(-s, -s,  s), // 4
                        vec3( s, -s,  s), // 5
                        vec3( s,  s,  s), // 6
                        vec3(-s,  s,  s), // 7
                    ];

                    let world_verts: Vec<Vec3> = cube_verts
                        .iter()
                        .map(|&v| transform_local_to_world(comp, layer_idx, v, time))
                        .collect();

                    let proj_verts: Vec<ProjectedPoint> = world_verts
                        .iter()
                        .map(|&wv| project_3d_point(wv, &active_camera, width, height))
                        .collect();

                    // 6 Faces: [v0, v1, v2, v3, normal_mult]
                    let faces: [([usize; 4], f32); 6] = [
                        ([0, 1, 2, 3], 0.8),  // Back face (-Z)
                        ([5, 4, 7, 6], 1.0),  // Front face (+Z)
                        ([4, 0, 3, 7], 0.65), // Left face (-X)
                        ([1, 5, 6, 2], 0.9),  // Right face (+X)
                        ([4, 5, 1, 0], 0.75), // Top face (-Y)
                        ([3, 2, 6, 7], 0.55), // Bottom face (+Y)
                    ];

                    let mut sorted_faces: Vec<_> = faces
                        .iter()
                        .map(|(idx, shade)| {
                            let avg_depth = (proj_verts[idx[0]].depth
                                + proj_verts[idx[1]].depth
                                + proj_verts[idx[2]].depth
                                + proj_verts[idx[3]].depth)
                                * 0.25;
                            (idx, *shade, avg_depth)
                        })
                        .collect();

                    sorted_faces.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

                    for (idx, shade, _) in sorted_faces {
                        let p0 = &proj_verts[idx[0]];
                        let p1 = &proj_verts[idx[1]];
                        let p2 = &proj_verts[idx[2]];
                        let p3 = &proj_verts[idx[3]];

                        if p0.visible && p1.visible && p2.visible && p3.visible {
                            let face_col = Color::new(
                                (final_col.r * shade).clamp(0.0, 1.0),
                                (final_col.g * shade).clamp(0.0, 1.0),
                                (final_col.b * shade).clamp(0.0, 1.0),
                                final_col.a,
                            );
                            draw_triangle(p0.screen, p1.screen, p2.screen, face_col);
                            draw_triangle(p0.screen, p2.screen, p3.screen, face_col);

                            for k in 0..4 {
                                let va = &proj_verts[idx[k]];
                                let vb = &proj_verts[idx[(k + 1) % 4]];
                                draw_line(
                                    va.screen.x,
                                    va.screen.y,
                                    vb.screen.x,
                                    vb.screen.y,
                                    1.2,
                                    Color::new(1.0, 1.0, 1.0, op * 0.35),
                                );
                            }
                        }
                    }
                }
                LayerSource::Camera | LayerSource::Audio { .. } | LayerSource::Adjustment | LayerSource::Null => {}
                LayerSource::Video { path } => {
                    let origin_world = transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time);
                    let proj = project_3d_point(origin_world, &active_camera, width, height);
                    if proj.visible {
                        let scale_factor = (active_camera.zoom / proj.depth.max(1.0)).clamp(0.05, 10.0);
                        draw_video_placeholder(path, proj.screen.x, proj.screen.y, scale_factor, scale_factor, op);
                    }
                }
            }
        } else if !is_camera_inline_with_z(&active_camera, comp) {
            // ==========================================
            // 2D LAYER XY-PLANE 3D PROJECTION PASS (Custom View / Angled Camera)
            // ==========================================
            match &layer.source {
                LayerSource::Solid { color } => {
                    let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                    let final_col = apply_color_effects(base_col, layer, time);

                    let corners = [
                        transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(200.0, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(200.0, 200.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(0.0, 200.0, 0.0), time),
                    ];
                    let proj = [
                        project_3d_point(corners[0], &active_camera, width, height),
                        project_3d_point(corners[1], &active_camera, width, height),
                        project_3d_point(corners[2], &active_camera, width, height),
                        project_3d_point(corners[3], &active_camera, width, height),
                    ];
                    if proj.iter().all(|p| p.visible) {
                        draw_triangle(proj[0].screen, proj[1].screen, proj[2].screen, final_col);
                        draw_triangle(proj[0].screen, proj[2].screen, proj[3].screen, final_col);
                        for i in 0..4 {
                            draw_line(
                                proj[i].screen.x,
                                proj[i].screen.y,
                                proj[(i + 1) % 4].screen.x,
                                proj[(i + 1) % 4].screen.y,
                                1.2,
                                Color::new(1.0, 1.0, 1.0, op * 0.35),
                            );
                        }
                    }
                }
                LayerSource::Image { path } => {
                    if let Some(tex) = textures.get(path) {
                        let tw = tex.width();
                        let th = tex.height();
                        let base_col = Color::new(1.0, 1.0, 1.0, op);
                        let final_col = apply_color_effects(base_col, layer, time);

                        let corners = [
                            transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                            transform_local_to_world(comp, layer_idx, vec3(tw, 0.0, 0.0), time),
                            transform_local_to_world(comp, layer_idx, vec3(tw, th, 0.0), time),
                            transform_local_to_world(comp, layer_idx, vec3(0.0, th, 0.0), time),
                        ];
                        let proj = [
                            project_3d_point(corners[0], &active_camera, width, height),
                            project_3d_point(corners[1], &active_camera, width, height),
                            project_3d_point(corners[2], &active_camera, width, height),
                            project_3d_point(corners[3], &active_camera, width, height),
                        ];
                        if proj.iter().all(|p| p.visible) {
                            let mesh = Mesh {
                                vertices: vec![
                                    Vertex {
                                        position: vec3(proj[0].screen.x, proj[0].screen.y, 0.0),
                                        uv: vec2(0.0, 0.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                    Vertex {
                                        position: vec3(proj[1].screen.x, proj[1].screen.y, 0.0),
                                        uv: vec2(1.0, 0.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                    Vertex {
                                        position: vec3(proj[2].screen.x, proj[2].screen.y, 0.0),
                                        uv: vec2(1.0, 1.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                    Vertex {
                                        position: vec3(proj[3].screen.x, proj[3].screen.y, 0.0),
                                        uv: vec2(0.0, 1.0),
                                        color: final_col.into(),
                                        normal: vec4(0.0, 0.0, 1.0, 0.0),
                                    },
                                ],
                                indices: vec![0, 1, 2, 0, 2, 3],
                                texture: Some(tex.clone()),
                            };
                            draw_mesh(&mesh);
                            for i in 0..4 {
                                draw_line(
                                    proj[i].screen.x,
                                    proj[i].screen.y,
                                    proj[(i + 1) % 4].screen.x,
                                    proj[(i + 1) % 4].screen.y,
                                    1.0,
                                    Color::new(0.4, 0.8, 1.0, op * 0.4),
                                );
                            }
                        }
                    }
                }
                LayerSource::Text { text, font_size, color } => {
                    let est_w = (text.len() as f32 * font_size * 0.55).max(40.0);
                    let est_h = *font_size;
                    let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                    let text_col = apply_color_effects(base_col, layer, time);

                    let corners = [
                        transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(est_w, 0.0, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(est_w, est_h, 0.0), time),
                        transform_local_to_world(comp, layer_idx, vec3(0.0, est_h, 0.0), time),
                    ];
                    let proj = [
                        project_3d_point(corners[0], &active_camera, width, height),
                        project_3d_point(corners[1], &active_camera, width, height),
                        project_3d_point(corners[2], &active_camera, width, height),
                        project_3d_point(corners[3], &active_camera, width, height),
                    ];
                    if proj.iter().all(|p| p.visible) {
                        let scale_factor = (active_camera.zoom / proj[0].depth.max(1.0)).clamp(0.1, 5.0);
                        draw_text(text, proj[0].screen.x, proj[0].screen.y + est_h * scale_factor, font_size * scale_factor, text_col);
                        for i in 0..4 {
                            draw_line(
                                proj[i].screen.x,
                                proj[i].screen.y,
                                proj[(i + 1) % 4].screen.x,
                                proj[(i + 1) % 4].screen.y,
                                1.0,
                                Color::new(1.0, 0.4, 0.7, op * 0.35),
                            );
                        }
                    }
                }
                LayerSource::Polygon { points, color } => {
                    if points.len() >= 3 {
                        let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                        let final_col = apply_color_effects(base_col, layer, time);

                        let proj_pts: Vec<ProjectedPoint> = points
                            .iter()
                            .map(|pt| {
                                let w_pt = transform_local_to_world(
                                    comp,
                                    layer_idx,
                                    vec3(pt[0], pt[1], 0.0),
                                    time,
                                );
                                project_3d_point(w_pt, &active_camera, width, height)
                            })
                            .collect();

                        if proj_pts.iter().all(|p| p.visible) {
                            for i in 1..proj_pts.len() - 1 {
                                draw_triangle(proj_pts[0].screen, proj_pts[i].screen, proj_pts[i + 1].screen, final_col);
                            }
                            for i in 0..proj_pts.len() {
                                draw_line(
                                    proj_pts[i].screen.x,
                                    proj_pts[i].screen.y,
                                    proj_pts[(i + 1) % proj_pts.len()].screen.x,
                                    proj_pts[(i + 1) % proj_pts.len()].screen.y,
                                    1.2,
                                    Color::new(1.0, 1.0, 1.0, op * 0.4),
                                );
                            }
                        }
                    }
                }
                LayerSource::Video { path } => {
                    let origin_world = transform_local_to_world(comp, layer_idx, vec3(0.0, 0.0, 0.0), time);
                    let proj = project_3d_point(origin_world, &active_camera, width, height);
                    if proj.visible {
                        let scale_factor = (active_camera.zoom / proj.depth.max(1.0)).clamp(0.05, 10.0);
                        draw_video_placeholder(path, proj.screen.x, proj.screen.y, scale_factor, scale_factor, op);
                    }
                }
                _ => {}
            }
        } else {
            // ==========================================
            // 2D LAYER ORTHOGRAPHIC RENDERING PASS
            // ==========================================
            let (ax, ay, x, y, z, rot, rot_x, rot_y, sx, sy) = layer_transform(comp, layer_idx, time);

            // Drop Shadow & Spatial Pre-effects Pass
            if layer.fx {
                for eff in &layer.effects {
                    if !eff.enabled {
                        continue;
                    }
                    match eff.effect_type {
                        EffectType::DropShadow => {
                            let dist = eff.properties.get("distance").map_or(10.0, |p| p.get_value_at(time));
                            let ang = eff.properties.get("angle").map_or(45.0, |p| p.get_value_at(time)).to_radians();
                            let s_op = (eff.properties.get("opacity").map_or(75.0, |p| p.get_value_at(time)) / 100.0) * op;
                            let softness = eff.properties.get("softness").map_or(5.0, |p| p.get_value_at(time));
                            let sh_x = x + dist * ang.cos();
                            let sh_y = y + dist * ang.sin();

                            let passes = if softness > 1.0 {
                                vec![
                                    (0.0, 0.0, 0.4),
                                    (-softness * 0.4, 0.0, 0.15),
                                    (softness * 0.4, 0.0, 0.15),
                                    (0.0, -softness * 0.4, 0.15),
                                    (0.0, softness * 0.4, 0.15),
                                ]
                            } else {
                                vec![(0.0, 0.0, 1.0)]
                            };

                            for (ox, oy, weight) in passes {
                                let shadow_col = Color::new(0.0, 0.0, 0.0, s_op * weight * 0.85);
                                match &layer.source {
                                    LayerSource::Solid { .. } => {
                                        draw_rectangle_ex(
                                            sh_x + ox,
                                            sh_y + oy,
                                            200.0 * sx,
                                            200.0 * sy,
                                            DrawRectangleParams {
                                                offset: vec2(ax / 200.0, ay / 200.0),
                                                rotation: rot.to_radians(),
                                                color: shadow_col,
                                            },
                                        );
                                    }
                                    LayerSource::Text { text, font_size, .. } => {
                                        let size = (*font_size * sx.abs()).max(8.0);
                                        draw_text(text, sh_x + ox - ax, sh_y + oy - ay, size, shadow_col);
                                    }
                                    LayerSource::Image { path } => {
                                        if let Some(tex) = textures.get(path) {
                                            draw_texture_ex(
                                                tex,
                                                sh_x + ox,
                                                sh_y + oy,
                                                shadow_col,
                                                DrawTextureParams {
                                                    dest_size: Some(vec2(tex.width() * sx, tex.height() * sy)),
                                                    rotation: rot.to_radians(),
                                                    pivot: Some(vec2(sh_x + ox + ax, sh_y + oy + ay)),
                                                    ..Default::default()
                                                },
                                            );
                                        }
                                    }
                                    LayerSource::Polygon { points, .. } => {
                                        draw_polygon_layer(points, sh_x + ox, sh_y + oy, sx, sy, shadow_col);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        EffectType::Glow => {
                            let radius = eff.properties.get("radius").map_or(20.0, |p| p.get_value_at(time));
                            let intens = eff.properties.get("intensity").map_or(1.0, |p| p.get_value_at(time)) * op;
                            let glow_col = Color::new(1.0, 0.95, 0.75, 0.08 * intens);
                            match &layer.source {
                                LayerSource::Solid { .. } => {
                                    for r_step in [radius * 0.4, radius * 0.8] {
                                        draw_rectangle_ex(
                                            x,
                                            y,
                                            (200.0 + r_step) * sx,
                                            (200.0 + r_step) * sy,
                                            DrawRectangleParams {
                                                offset: vec2(ax / 200.0, ay / 200.0),
                                                rotation: rot.to_radians(),
                                                color: glow_col,
                                            },
                                        );
                                    }
                                }
                                LayerSource::Text { text, font_size, .. } => {
                                    let size = (*font_size * sx.abs()).max(8.0);
                                    for (gx, gy) in [(-2.0, 0.0), (2.0, 0.0), (0.0, -2.0), (0.0, 2.0)] {
                                        draw_text(text, x - ax + gx, y - ay + gy, size, glow_col);
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Spatial Blur / Chromatic Aberration Passes
            let mut blur_samples = vec![(0.0f32, 0.0f32, 1.0f32)];
            let mut chromatic_offsets: Option<(f32, f32, f32)> = None;

            if layer.fx {
                for eff in &layer.effects {
                    if !eff.enabled {
                        continue;
                    }
                    if eff.effect_type == EffectType::FastBlur {
                        let r = eff.properties.get("blurRadius").map_or(15.0, |p| p.get_value_at(time));
                        if r > 0.5 {
                            blur_samples = vec![
                                (0.0, 0.0, 0.28),
                                (-r * 0.5, 0.0, 0.18),
                                (r * 0.5, 0.0, 0.18),
                                (0.0, -r * 0.5, 0.18),
                                (0.0, r * 0.5, 0.18),
                            ];
                        }
                    } else if eff.effect_type == EffectType::DirectionalBlur {
                        let blur_len = eff.properties.get("blurLength").map_or(20.0, |p| p.get_value_at(time));
                        let ang = eff.properties.get("angle").map_or(0.0, |p| p.get_value_at(time)).to_radians();
                        if blur_len > 0.5 {
                            let bx = blur_len * ang.cos() * 0.5;
                            let by = blur_len * ang.sin() * 0.5;
                            blur_samples = vec![
                                (0.0, 0.0, 0.34),
                                (-bx, -by, 0.33),
                                (bx, by, 0.33),
                            ];
                        }
                    } else if eff.effect_type == EffectType::ChromaticAberration {
                        let dist = eff.properties.get("distance").map_or(8.0, |p| p.get_value_at(time));
                        let ang = eff.properties.get("angle").map_or(0.0, |p| p.get_value_at(time)).to_radians();
                        let intens = eff.properties.get("intensity").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                        chromatic_offsets = Some((dist * ang.cos(), dist * ang.sin(), intens));
                    }
                }
            }

            for (bx, by, b_weight) in blur_samples {
                match &layer.source {
                    LayerSource::Solid { color } => {
                        let base_col = Color::new(color[0], color[1], color[2], color[3] * op * b_weight);
                        let final_col = apply_color_effects(base_col, layer, time);
                        if let Some((cx, cy, c_intens)) = chromatic_offsets {
                            let red_col = Color::new(1.0, 0.2, 0.2, final_col.a * 0.5 * c_intens);
                            let cyan_col = Color::new(0.2, 0.8, 1.0, final_col.a * 0.5 * c_intens);
                            draw_rectangle_ex(
                                x + bx + cx,
                                y + by + cy,
                                200.0 * sx,
                                200.0 * sy,
                                DrawRectangleParams {
                                    offset: vec2(ax / 200.0, ay / 200.0),
                                    rotation: rot.to_radians(),
                                    color: red_col,
                                },
                            );
                            draw_rectangle_ex(
                                x + bx - cx,
                                y + by - cy,
                                200.0 * sx,
                                200.0 * sy,
                                DrawRectangleParams {
                                    offset: vec2(ax / 200.0, ay / 200.0),
                                    rotation: rot.to_radians(),
                                    color: cyan_col,
                                },
                            );
                        }
                        draw_rectangle_ex(
                            x + bx,
                            y + by,
                            200.0 * sx,
                            200.0 * sy,
                            DrawRectangleParams {
                                offset: vec2(ax / 200.0, ay / 200.0),
                                rotation: rot.to_radians(),
                                color: final_col,
                            },
                        );
                    }
                    LayerSource::Text {
                        text,
                        font_size,
                        color,
                    } => {
                        let size = (*font_size * sx.abs()).max(8.0);
                        let base_col = Color::new(color[0], color[1], color[2], color[3] * op * b_weight);
                        let text_color = apply_color_effects(base_col, layer, time);
                        if let Some((cx, cy, c_intens)) = chromatic_offsets {
                            let red_col = Color::new(1.0, 0.2, 0.2, text_color.a * 0.6 * c_intens);
                            let cyan_col = Color::new(0.2, 0.8, 1.0, text_color.a * 0.6 * c_intens);
                            draw_text(text, x + bx + cx - ax, y + by + cy - ay, size, red_col);
                            draw_text(text, x + bx - cx - ax, y + by - cy - ay, size, cyan_col);
                        }
                        draw_text(text, x + bx - ax, y + by - ay, size, text_color);
                    }
                    LayerSource::Image { path } => {
                        if let Some(tex) = textures.get(path) {
                            let base_col = Color::new(1.0, 1.0, 1.0, op * b_weight);
                            let final_col = apply_color_effects(base_col, layer, time);
                            if let Some((cx, cy, c_intens)) = chromatic_offsets {
                                let red_col = Color::new(1.0, 0.3, 0.3, final_col.a * 0.5 * c_intens);
                                let cyan_col = Color::new(0.3, 0.8, 1.0, final_col.a * 0.5 * c_intens);
                                draw_texture_ex(
                                    tex,
                                    x + bx + cx,
                                    y + by + cy,
                                    red_col,
                                    DrawTextureParams {
                                        dest_size: Some(vec2(tex.width() * sx, tex.height() * sy)),
                                        rotation: rot.to_radians(),
                                        pivot: Some(vec2(x + bx + cx + ax, y + by + cy + ay)),
                                        ..Default::default()
                                    },
                                );
                                draw_texture_ex(
                                    tex,
                                    x + bx - cx,
                                    y + by - cy,
                                    cyan_col,
                                    DrawTextureParams {
                                        dest_size: Some(vec2(tex.width() * sx, tex.height() * sy)),
                                        rotation: rot.to_radians(),
                                        pivot: Some(vec2(x + bx - cx + ax, y + by - cy + ay)),
                                        ..Default::default()
                                    },
                                );
                            }
                            draw_texture_ex(
                                tex,
                                x + bx,
                                y + by,
                                final_col,
                                DrawTextureParams {
                                    dest_size: Some(vec2(tex.width() * sx, tex.height() * sy)),
                                    rotation: rot.to_radians(),
                                    pivot: Some(vec2(x + bx + ax, y + by + ay)),
                                    ..Default::default()
                                },
                            );
                        }
                    }
                    LayerSource::Video { path } => draw_video_placeholder(path, x + bx, y + by, sx, sy, op * b_weight),
                    LayerSource::Polygon { points, color } => {
                        let base_col = Color::new(color[0], color[1], color[2], color[3] * op * b_weight);
                        let final_col = apply_color_effects(base_col, layer, time);
                        draw_polygon_layer(
                            points,
                            x + bx,
                            y + by,
                            sx,
                            sy,
                            final_col,
                        );
                    }
                    _ => {}
                }
            }
            match &layer.source {
                LayerSource::Adjustment => {
                    if layer.fx && op > 0.001 {
                        for eff in &layer.effects {
                            if !eff.enabled {
                                continue;
                            }
                            match eff.effect_type {
                                EffectType::Vignette => {
                                    let amt = eff.properties.get("amount").map_or(50.0, |p| p.get_value_at(time)) / 100.0 * op;
                                    let feather = eff.properties.get("feather").map_or(40.0, |p| p.get_value_at(time)) / 100.0;
                                    let rings = 12;
                                    let center_x = width * 0.5;
                                    let center_y = height * 0.5;
                                    let max_r = (width * 0.5).hypot(height * 0.5);
                                    for r in (0..rings).rev() {
                                        let t = r as f32 / rings as f32;
                                        if t > (1.0 - feather).max(0.1) {
                                            let alpha = ((t - (1.0 - feather)) / feather.max(0.01)).powi(2) * amt * 0.7;
                                            draw_circle_lines(center_x, center_y, max_r * t, max_r / rings as f32 * 1.5, Color::new(0.0, 0.0, 0.0, alpha));
                                        }
                                    }
                                }
                                EffectType::ChromaticAberration => {
                                    let dist = eff.properties.get("distance").map_or(8.0, |p| p.get_value_at(time));
                                    let ang = eff.properties.get("angle").map_or(0.0, |p| p.get_value_at(time)).to_radians();
                                    let intensity = eff.properties.get("intensity").map_or(100.0, |p| p.get_value_at(time)) / 100.0 * op;
                                    let off_x = dist * ang.cos();
                                    let off_y = dist * ang.sin();
                                    draw_rectangle(off_x, off_y, width, height, Color::new(1.0, 0.0, 0.2, 0.04 * intensity));
                                    draw_rectangle(-off_x, -off_y, width, height, Color::new(0.0, 0.8, 1.0, 0.04 * intensity));
                                }
                                EffectType::Glow => {
                                    let rad = eff.properties.get("radius").map_or(20.0, |p| p.get_value_at(time));
                                    let intens = eff.properties.get("intensity").map_or(1.0, |p| p.get_value_at(time)) * op;
                                    draw_rectangle(0.0, 0.0, width, height, Color::new(1.0, 0.95, 0.8, 0.03 * intens * (rad / 20.0)));
                                }
                                EffectType::FastBlur => {
                                    let r = eff.properties.get("blurRadius").map_or(15.0, |p| p.get_value_at(time));
                                    if r > 0.5 {
                                        for s in 1..=4 {
                                            let step = (s as f32) * r * 0.3;
                                            draw_rectangle(step, 0.0, width, height, Color::new(1.0, 1.0, 1.0, 0.02 * op));
                                            draw_rectangle(-step, 0.0, width, height, Color::new(1.0, 1.0, 1.0, 0.02 * op));
                                            draw_rectangle(0.0, step, width, height, Color::new(1.0, 1.0, 1.0, 0.02 * op));
                                            draw_rectangle(0.0, -step, width, height, Color::new(1.0, 1.0, 1.0, 0.02 * op));
                                        }
                                    }
                                }
                                EffectType::Fill => {
                                    let r = eff.properties.get("colorR").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                                    let g = eff.properties.get("colorG").map_or(100.0, |p| p.get_value_at(time)) / 255.0;
                                    let b = eff.properties.get("colorB").map_or(50.0, |p| p.get_value_at(time)) / 255.0;
                                    let amount = eff.properties.get("opacity").map_or(100.0, |p| p.get_value_at(time)) / 100.0 * op;
                                    draw_rectangle(0.0, 0.0, width, height, Color::new(r, g, b, amount * 0.3));
                                }
                                EffectType::Tint => {
                                    let amount = eff.properties.get("amount").map_or(100.0, |p| p.get_value_at(time)) / 100.0 * op;
                                    let wht_r = eff.properties.get("whiteR").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                                    let wht_g = eff.properties.get("whiteG").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                                    let wht_b = eff.properties.get("whiteB").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
                                    draw_rectangle(0.0, 0.0, width, height, Color::new(wht_r, wht_g, wht_b, amount * 0.15));
                                }
                                EffectType::BrightnessContrast => {
                                    let br = eff.properties.get("brightness").map_or(0.0, |p| p.get_value_at(time)) / 100.0 * op;
                                    if br > 0.0 {
                                        draw_rectangle(0.0, 0.0, width, height, Color::new(1.0, 1.0, 1.0, br * 0.5));
                                    } else if br < 0.0 {
                                        draw_rectangle(0.0, 0.0, width, height, Color::new(0.0, 0.0, 0.0, (-br) * 0.5));
                                    }
                                }
                                EffectType::DirectionalBlur => {
                                    let blur_len = eff.properties.get("blurLength").map_or(20.0, |p| p.get_value_at(time));
                                    let ang = eff.properties.get("angle").map_or(0.0, |p| p.get_value_at(time)).to_radians();
                                    let bx = blur_len * ang.cos() * 0.5;
                                    let by = blur_len * ang.sin() * 0.5;
                                    draw_rectangle(bx, by, width, height, Color::new(1.0, 1.0, 1.0, 0.03 * op));
                                    draw_rectangle(-bx, -by, width, height, Color::new(1.0, 1.0, 1.0, 0.03 * op));
                                }
                                EffectType::WaveWarp => {
                                    let h = eff.properties.get("waveHeight").map_or(15.0, |p| p.get_value_at(time));
                                    let spd = eff.properties.get("speed").map_or(1.0, |p| p.get_value_at(time));
                                    let wave_off = (time * spd * 3.0).sin() * h * 0.3;
                                    draw_rectangle(0.0, wave_off, width, height, Color::new(0.7, 0.85, 1.0, 0.02 * op));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                LayerSource::Camera | LayerSource::Audio { .. } | LayerSource::Null => {}
                LayerSource::Object3D { color, .. } => {
                    set_camera(&Camera3D {
                        position: vec3(width / 2.0, height / 2.0 - 900.0, 720.0),
                        target: vec3(width / 2.0, height / 2.0, 0.0),
                        up: vec3(0.0, 0.0, 1.0),
                        aspect: Some(width / height),
                        render_target: Some(target.clone()),
                        ..Default::default()
                    });
                    let size = vec3(
                        140.0 * sx.abs().max(0.05),
                        140.0 * sy.abs().max(0.05),
                        140.0 * ((sx.abs() + sy.abs()) * 0.5).max(0.05),
                    );
                    let pos = vec3(x, y, z);
                    let base_col = Color::new(color[0], color[1], color[2], color[3] * op);
                    let final_col = apply_color_effects(base_col, layer, time);
                    draw_cube(
                        pos,
                        size,
                        None,
                        final_col,
                    );
                    draw_cube_wires(pos, size, Color::new(1.0, 1.0, 1.0, op));
                    set_camera(&Camera2D {
                        render_target: Some(target.clone()),
                        ..Camera2D::from_display_rect(Rect::new(0., 0., width, height))
                    });
                    let _ = (rot, rot_x, rot_y);
                }
                _ => {}
            }
        }
    }

    // Camera Layer visualization (Frustum & POI Line) in Custom / Orthographic views
    if comp.viewport_mode != ViewportMode::ActiveCamera {
        for l in &comp.layers {
            if let LayerSource::Camera = &l.source {
                let cam_x = l.properties.get("x").map_or(width / 2.0, |p| p.get_value_at(time));
                let cam_y = l.properties.get("y").map_or(height / 2.0, |p| p.get_value_at(time));
                let cam_z = l.properties.get("z").map_or(-1500.0, |p| p.get_value_at(time));
                let poi_x = l.properties.get("poiX").map_or(width / 2.0, |p| p.get_value_at(time));
                let poi_y = l.properties.get("poiY").map_or(height / 2.0, |p| p.get_value_at(time));
                let poi_z = l.properties.get("poiZ").map_or(0.0, |p| p.get_value_at(time));

                let p_cam = project_3d_point(vec3(cam_x, cam_y, cam_z), &active_camera, width, height);
                let p_poi = project_3d_point(vec3(poi_x, poi_y, poi_z), &active_camera, width, height);

                if p_cam.visible {
                    draw_circle(p_cam.screen.x, p_cam.screen.y, 5.0, Color::from_rgba(100, 200, 255, 230));
                    draw_rectangle_lines(p_cam.screen.x - 10.0, p_cam.screen.y - 7.0, 20.0, 14.0, 1.5, Color::from_rgba(100, 200, 255, 230));
                    draw_text(&format!("🎥 {}", l.name), p_cam.screen.x + 14.0, p_cam.screen.y + 4.0, 14.0, Color::from_rgba(100, 200, 255, 230));
                }
                if p_cam.visible && p_poi.visible {
                    draw_line(p_cam.screen.x, p_cam.screen.y, p_poi.screen.x, p_poi.screen.y, 1.0, Color::from_rgba(100, 200, 255, 120));
                    draw_circle_lines(p_poi.screen.x, p_poi.screen.y, 6.0, 1.0, Color::from_rgba(255, 220, 80, 200));
                    draw_line(p_poi.screen.x - 9.0, p_poi.screen.y, p_poi.screen.x + 9.0, p_poi.screen.y, 1.0, Color::from_rgba(255, 220, 80, 200));
                    draw_line(p_poi.screen.x, p_poi.screen.y - 9.0, p_poi.screen.x, p_poi.screen.y + 9.0, 1.0, Color::from_rgba(255, 220, 80, 200));
                }
            }
        }
    }

    // ==========================================
    // VIEWPORT / MASTER COMP EFFECTS PASS
    // ==========================================
    if comp.viewport_fx_enabled && !comp.viewport_effects.is_empty() {
        let mut img = target.texture.get_texture_data();
        for eff in &comp.viewport_effects {
            if eff.enabled {
                crate::plugin::apply_image_builtin_effect(&mut img, eff, time, None);
            }
        }
        target.texture.update(&img);
    }

    set_default_camera();
}

fn get_max_keyframe_time(comp: &Composition) -> f32 {
    let mut max_time = 0.0f32;
    for layer in &comp.layers {
        for prop in layer.properties.values() {
            for kf in &prop.keyframes {
                if kf.time > max_time {
                    max_time = kf.time;
                }
            }
        }
        for eff in &layer.effects {
            for prop in eff.properties.values() {
                for kf in &prop.keyframes {
                    if kf.time > max_time {
                        max_time = kf.time;
                    }
                }
            }
        }
    }
    for eff in &comp.viewport_effects {
        for prop in eff.properties.values() {
            for kf in &prop.keyframes {
                if kf.time > max_time {
                    max_time = kf.time;
                }
            }
        }
    }
    max_time
}

fn export_video(
    comp: &Composition,
    textures: &HashMap<String, Texture2D>,
    render_target: RenderTarget,
    output_path: &Path,
) -> String {
    let fps = comp.settings.fps.max(1);
    let mut duration = get_max_keyframe_time(comp);
    if duration <= 0.0 {
        duration = comp.settings.duration;
    }
    let frame_count = (duration.max(0.1) * fps as f32).ceil() as usize;

    let width = comp.settings.width;
    let height = comp.settings.height;

    // Initialize FFmpeg
    if let Err(e) = ffmpeg::init() {
        return format!("FFmpeg initialization failed: {}", e);
    }

    // Assemble video using playa-ffmpeg crate
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut out_ctx = ffmpeg::format::output(&output_path)?;
        let mut stream =
            out_ctx.add_stream(ffmpeg::codec::encoder::find(ffmpeg::codec::Id::H264))?;
        let codec_ctx = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        let mut encoder = codec_ctx.encoder().video()?;

        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(Pixel::YUV420P);
        encoder.set_time_base((1, fps as i32));

        let mut encoder = encoder.open_as_with(
            ffmpeg::codec::encoder::find(ffmpeg::codec::Id::H264),
            dict! {
                "preset" => "ultrafast",
                "tune" => "zerolatency",
            },
        )?;
        stream.set_parameters(&encoder);

        out_ctx.write_header()?;

        let mut scaler = ScalerContext::get(
            Pixel::RGBA,
            width,
            height,
            Pixel::YUV420P,
            width,
            height,
            Flags::BILINEAR,
        )?;

        let mut video_frame = Video::empty();
        let mut packet = ffmpeg::codec::packet::Packet::empty();

        let mut export_comp = comp.clone();
        export_comp.viewport_mode = ViewportMode::ActiveCamera;

        for frame_idx in 0..frame_count {
            let time = frame_idx as f32 / fps as f32;
            draw_composition(&export_comp, textures, time, render_target.clone());
            let image = render_target.texture.get_texture_data();
            let rgba_data = image.bytes;

            let mut input_frame = Video::new(Pixel::RGBA, width, height);
            input_frame.data_mut(0).copy_from_slice(&rgba_data);

            scaler.run(&input_frame, &mut video_frame)?;
            video_frame.set_pts(Some(frame_idx as i64));

            encoder.send_frame(&video_frame)?;
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(0);
                packet.write_interleaved(&mut out_ctx)?;
            }
        }

        encoder.send_eof()?;
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(0);
            packet.write_interleaved(&mut out_ctx)?;
        }

        out_ctx.write_trailer()?;
        Ok(())
    })();

    match result {
        Ok(_) => format!("Exported {} successfully", output_path.display()),
        Err(e) => format!("Failed to export video: {}", e),
    }
}

#[macroquad::main("BeforeFX - Professional Motion Graphics")]
async fn main() {
    let mut comp = Composition {
        layers: vec![
            default_layer(
                "Background Solid".into(),
                LayerSource::Solid {
                    color: [0.12, 0.14, 0.18, 1.0],
                },
                0,
            ),
            default_layer(
                "Title Text".into(),
                LayerSource::Text {
                    text: "BEFORE EFFECTS".to_string(),
                    font_size: 64.0,
                    color: [1.0, 0.85, 0.25, 1.0],
                },
                1,
            ),
            default_layer(
                "Graphic Polygon".into(),
                LayerSource::Polygon {
                    points: vec![
                        [0.0, -80.0],
                        [80.0, 0.0],
                        [40.0, 90.0],
                        [-60.0, 80.0],
                        [-90.0, -20.0],
                    ],
                    color: [0.95, 0.45, 0.2, 1.0],
                },
                10,
            ),
        ],
        resources: vec![],
        current_time: 0.0,
        is_playing: false,
        show_curves: false,
        timeline_scroll_v: 0.0,
        timeline_scroll_h: 0.0,
        settings: Settings::default(),
        active_layer_index: Some(1),
        work_area_in: 0.0,
        work_area_out: 30.0,
        timeline_zoom: 100.0,
        hide_shy: false,
        switches_mode: false,
        active_tool: 0,
        right_panel_tab: 0,
        left_panel_tab: 0,
        show_guides: true,
        show_grid: false,
        show_rulers: false,
        show_checkerboard: false,
        comp_zoom: 1.0,
        search_query: String::new(),
        layer_search_query: String::new(),
        markers: vec![
            Marker {
                time: 2.0,
                label: "Intro".to_string(),
                comment: "Logo entry".to_string(),
                color_index: 1,
            },
            Marker {
                time: 5.0,
                label: "Action".to_string(),
                comment: "Main motion starts".to_string(),
                color_index: 0,
            },
        ],
        show_graph_editor: false,
        snapping: true,
        solo_animated_properties: false,
        viewport_mode: ViewportMode::ActiveCamera,
        custom_orbit_yaw: -35.0,
        custom_orbit_pitch: 25.0,
        custom_orbit_distance: 2200.0,
        custom_orbit_target: [960.0, 540.0, 0.0],
        ..Composition::default()
    };

    let mut history: Vec<Composition> = vec![comp.clone()];
    let mut history_index: usize = 0;
    let mut selected_keyframe: Option<SelectedKeyframe> = None;
    let mut textures: HashMap<String, Texture2D> = HashMap::new();
    let mut sounds: HashMap<String, Sound> = HashMap::new();
    let mut to_load: Vec<String> = vec![];
    let mut to_load_audio: Vec<String> = vec![];
    let mut audio_started = false;
    let mut pending_export: Option<PathBuf> = None;
    let mut export_status = String::new();
    let mut show_shortcuts_dialog = false;
    let mut show_cache_settings_dialog = false;
    let mut show_plugin_manager = false;
    let mut plugin_registry = PluginRegistry::new();
    let _export_resolution_preset = 1; // 0=4K, 1=1080p, 2=720p, 3=Vertical 9:16, 4=Square 1:1
    let _export_range_preset = 0; // 0=Work Area, 1=Entire Comp

    let mut gizmo_drag = GizmoHandle::None;
    let mut gizmo_drag_start_mouse = vec2(0.0, 0.0);
    let mut gizmo_drag_start_val: f32 = 0.0;
    let mut gizmo_drag_start_x: f32 = 0.0;
    let mut gizmo_drag_start_y: f32 = 0.0;
    let mut gizmo_drag_start_z: f32 = 0.0;
    let mut gizmo_drag_start_ax: f32 = 0.0;
    let mut gizmo_drag_start_ay: f32 = 0.0;
    let mut gizmo_drag_start_sx: f32 = 100.0;
    let mut gizmo_drag_start_sy: f32 = 100.0;
    let mut gizmo_drag_start_rot: f32 = 0.0;
    let mut shape_drag_start: Option<Vec2> = None;
    let mut egui_wants_keyboard = false;

    let mut ram_cache = RamPreviewCache::new(comp.cache_max_frames, comp.cache_max_memory_mb);
    let mut memory_history: Vec<f32> = vec![0.0; 60];
    let mut last_mem_sample_time = 0.0f64;
    let mut measured_fps = 60.0f32;

    let render_target = render_target(1920, 1080);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut viewport_rect = egui::Rect::NOTHING;

    loop {
        // --- 0. ASSET LOADING ---
        for path in to_load.drain(..) {
            if let Ok(tex) = load_texture(&path).await {
                textures.insert(path, tex);
            }
        }
        for path in to_load_audio.drain(..) {
            if let Ok(sound) = load_sound(&path).await {
                sounds.insert(path, sound);
            }
        }
        for l in &comp.layers {
            match &l.source {
                LayerSource::Image { path } => {
                    if !textures.contains_key(path) && !to_load.contains(path) {
                        to_load.push(path.clone());
                    }
                }
                LayerSource::Audio { path } => {
                    if !sounds.contains_key(path) && !to_load_audio.contains(path) {
                        to_load_audio.push(path.clone());
                    }
                }
                _ => {}
            }
        }

        // --- RAM PREVIEW & CACHE INVALIDATION ENGINE ---
        ram_cache.max_frames = comp.cache_max_frames;
        ram_cache.max_memory_mb = comp.cache_max_memory_mb;
        let current_hash = compute_comp_content_hash(&comp);
        if current_hash != ram_cache.comp_hash || comp.ram_cache_purge_requested {
            ram_cache.clear();
            ram_cache.comp_hash = current_hash;
            comp.cached_frames.clear();
            comp.ram_cache_purge_requested = false;
        }
        comp.cached_frames = ram_cache.frames.keys().copied().collect();
        comp.cache_size_mb = ram_cache.memory_usage_mb();
        comp.cache_raw_size_mb = ram_cache.raw_memory_usage_mb();
        comp.cache_compression_ratio = ram_cache.compression_ratio();

        let now_sec = get_time();
        if now_sec - last_mem_sample_time >= 0.12 {
            last_mem_sample_time = now_sec;
            if !memory_history.is_empty() {
                memory_history.remove(0);
                memory_history.push(comp.cache_size_mb);
            }
        }

        let dt = get_frame_time();
        if dt > 0.0001 {
            let cur_fps = 1.0 / dt;
            measured_fps = measured_fps * 0.9 + cur_fps * 0.1;
            comp.playback_fps = measured_fps;
        }

        // --- PLAYBACK ENGINE ---
        if comp.is_playing {
            comp.current_time += get_frame_time();
            let max_kf = get_max_keyframe_time(&comp);
            if comp.pause_at_last_keyframe && max_kf > 0.0 {
                if comp.current_time >= max_kf {
                    comp.current_time = max_kf;
                    comp.is_playing = false;
                    comp.is_ram_previewing = false;
                }
            } else if comp.current_time >= comp.work_area_out.min(comp.settings.duration) {
                comp.current_time = comp.work_area_in;
            }
            if !audio_started {
                for layer in &comp.layers {
                    if let LayerSource::Audio { path } = &layer.source {
                        if let Some(sound) = sounds.get(path) {
                            play_sound(
                                sound,
                                PlaySoundParams {
                                    looped: false,
                                    volume: 1.0,
                                },
                            );
                        }
                    }
                }
                audio_started = true;
            }
        } else if audio_started {
            for sound in sounds.values() {
                stop_sound(sound);
            }
            audio_started = false;
        }

        // --- GLOBAL KEYBOARD SHORTCUTS ---
        if is_key_pressed(KeyCode::Space) {
            comp.is_ram_previewing = false;
            comp.is_playing = !comp.is_playing;
        }
        // RAM Preview toggle: NumPad 0 or Shift + Space
        if is_key_pressed(KeyCode::Kp0) || (is_key_down(KeyCode::LeftShift) && is_key_pressed(KeyCode::Space)) {
            comp.is_ram_previewing = !comp.is_ram_previewing;
            if comp.is_ram_previewing {
                comp.is_playing = true;
                let max_kf = get_max_keyframe_time(&comp);
                let playback_end = if comp.pause_at_last_keyframe && max_kf > 0.0 {
                    max_kf
                } else {
                    comp.work_area_out.min(comp.settings.duration)
                };
                if comp.current_time < comp.work_area_in || comp.current_time >= playback_end {
                    comp.current_time = comp.work_area_in;
                }
            } else {
                comp.is_playing = false;
            }
        }
        // Purge RAM cache: Ctrl + Alt + Key0 / Kp0
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && (is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt))
            && (is_key_pressed(KeyCode::Kp0) || is_key_pressed(KeyCode::Key0))
        {
            comp.ram_cache_purge_requested = true;
        }
        if is_key_pressed(KeyCode::Home) {
            comp.current_time = comp.work_area_in;
        }
        if is_key_pressed(KeyCode::End) {
            comp.current_time = comp.work_area_out.min(comp.settings.duration);
        }
        if is_key_pressed(KeyCode::B) {
            comp.work_area_in = comp.current_time;
        }
        if is_key_pressed(KeyCode::N) {
            comp.work_area_out = comp.current_time;
        }
        if is_key_pressed(KeyCode::U) {
            comp.solo_animated_properties = !comp.solo_animated_properties;
        }
        // Jump to Prev/Next Keyframe (J / K)
        if is_key_pressed(KeyCode::J) {
            let mut prev_time = 0.0f32;
            let mut found = false;
            for l in &comp.layers {
                for p in l.properties.values() {
                    for kf in &p.keyframes {
                        if kf.time < comp.current_time - 0.01 && (!found || kf.time > prev_time) {
                            prev_time = kf.time;
                            found = true;
                        }
                    }
                }
            }
            if found {
                comp.current_time = prev_time;
            }
        }
        if is_key_pressed(KeyCode::K) {
            let mut next_time = comp.settings.duration;
            let mut found = false;
            for l in &comp.layers {
                for p in l.properties.values() {
                    for kf in &p.keyframes {
                        if kf.time > comp.current_time + 0.01 && (!found || kf.time < next_time) {
                            next_time = kf.time;
                            found = true;
                        }
                    }
                }
            }
            if found {
                comp.current_time = next_time;
            }
        }

        // Layer in/out jumps (I / O)
        if is_key_pressed(KeyCode::I) {
            if let Some(idx) = comp.active_layer_index {
                if idx < comp.layers.len() {
                    comp.current_time = comp.layers[idx].in_time;
                }
            }
        }
        if is_key_pressed(KeyCode::O) {
            if let Some(idx) = comp.active_layer_index {
                if idx < comp.layers.len() {
                    comp.current_time = comp.layers[idx].out_time;
                }
            }
        }

        // Layer Trimming (Alt+[ and Alt+])
        if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
            if is_key_pressed(KeyCode::LeftBracket) {
                if let Some(idx) = comp.active_layer_index {
                    if idx < comp.layers.len() {
                        comp.layers[idx].in_time = comp.current_time;
                    }
                }
            }
            if is_key_pressed(KeyCode::RightBracket) {
                if let Some(idx) = comp.active_layer_index {
                    if idx < comp.layers.len() {
                        comp.layers[idx].out_time = comp.current_time;
                    }
                }
            }
        }

        // Layer Alignment ([ and ])
        if !is_key_down(KeyCode::LeftAlt) && !is_key_down(KeyCode::RightAlt) && !is_key_down(KeyCode::LeftControl) && !is_key_down(KeyCode::RightControl) {
            if is_key_pressed(KeyCode::LeftBracket) {
                if let Some(idx) = comp.active_layer_index {
                    if idx < comp.layers.len() {
                        let dur = comp.layers[idx].out_time - comp.layers[idx].in_time;
                        comp.layers[idx].in_time = comp.current_time;
                        comp.layers[idx].out_time = comp.current_time + dur;
                    }
                }
            }
            if is_key_pressed(KeyCode::RightBracket) {
                if let Some(idx) = comp.active_layer_index {
                    if idx < comp.layers.len() {
                        let dur = comp.layers[idx].out_time - comp.layers[idx].in_time;
                        comp.layers[idx].out_time = comp.current_time;
                        comp.layers[idx].in_time = (comp.current_time - dur).max(0.0);
                    }
                }
            }
        }

        // Add Marker (*)
        if is_key_pressed(KeyCode::KpMultiply) {
            let num = comp.markers.len() + 1;
            comp.markers.push(Marker {
                time: comp.current_time,
                label: format!("M{}", num),
                comment: format!("Marker {}", num),
                color_index: (num - 1) % AE_LABEL_COLORS.len(),
            });
        }

        // Graph Editor Toggle (Shift+F3)
        if (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)) && is_key_pressed(KeyCode::F3) {
            comp.show_curves = !comp.show_curves;
        }

        // Easing Shortcuts: F9 (Easy Ease), Shift+F9 (Ease In), Ctrl+Shift+F9 / Ctrl+F9 (Ease Out)
        if is_key_pressed(KeyCode::F9) {
            let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            let is_ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if let Some(sel) = &selected_keyframe {
                if let Some(layer) = comp.layers.get_mut(sel.layer_index) {
                    if let Some(prop) = layer.properties.get_mut(&sel.property_name) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            if is_ctrl {
                                kf.ease = Some(BezierControl::ease_out());
                            } else if is_shift {
                                kf.ease = Some(BezierControl::ease_in());
                            } else {
                                kf.ease = Some(BezierControl::easy_ease());
                            }
                        }
                    }
                }
            }
        }

        // Add Adjustment Layer (Ctrl+Alt+Y)
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && (is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt))
            && is_key_pressed(KeyCode::Y)
        {
            let idx = comp.layers.len();
            comp.layers.push(default_layer(
                format!("Adjustment Layer {}", idx + 1),
                LayerSource::Adjustment,
                4,
            ));
            comp.active_layer_index = Some(idx);
        }

        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            // Undo (Ctrl+Z)
            if is_key_pressed(KeyCode::Z) && !is_key_down(KeyCode::LeftShift) && !is_key_down(KeyCode::RightShift) {
                if history_index > 0 {
                    history_index -= 1;
                    comp = history[history_index].clone();
                }
            }
            // Redo (Ctrl+Y or Ctrl+Shift+Z)
            if is_key_pressed(KeyCode::Y) || (is_key_pressed(KeyCode::Z) && (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift))) {
                if history_index + 1 < history.len() {
                    history_index += 1;
                    comp = history[history_index].clone();
                }
            }

            // Split Layer (Ctrl+Shift+D)
            if is_key_pressed(KeyCode::D) && (is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)) {
                if let Some(idx) = comp.active_layer_index {
                    if idx < comp.layers.len() {
                        let curr = comp.current_time;
                        if curr > comp.layers[idx].in_time && curr < comp.layers[idx].out_time {
                            let mut second_layer = comp.layers[idx].clone();
                            comp.layers[idx].out_time = curr;
                            second_layer.in_time = curr;
                            second_layer.name = format!("{} (Split)", second_layer.name);
                            second_layer.label_color_index =
                                (second_layer.label_color_index + 2) % AE_LABEL_COLORS.len();
                            comp.layers.insert(idx + 1, second_layer);
                            comp.active_layer_index = Some(idx + 1);
                        }
                    }
                }
            } else if is_key_pressed(KeyCode::D) {
                // Duplicate Layer (Ctrl+D)
                if let Some(idx) = comp.active_layer_index {
                    if idx < comp.layers.len() {
                        let mut new_layer = comp.layers[idx].clone();
                        new_layer.name = format!("{} Copy", new_layer.name);
                        new_layer.label_color_index =
                            (new_layer.label_color_index + 1) % AE_LABEL_COLORS.len();
                        comp.layers.insert(idx + 1, new_layer);
                        comp.active_layer_index = Some(idx + 1);
                    }
                }
            }

            if is_key_pressed(KeyCode::Equal) {
                comp.settings.ui_scale = (comp.settings.ui_scale + 0.1).min(2.5);
            }
            if is_key_pressed(KeyCode::Minus) {
                comp.settings.ui_scale = (comp.settings.ui_scale - 0.1).max(0.5);
            }
            if is_key_pressed(KeyCode::Key0) {
                comp.settings.ui_scale = 1.0;
            }
        }

        // --- TOOL PALETTE KEYBOARD SHORTCUTS ---
        if !egui_wants_keyboard {
            if !is_key_down(KeyCode::LeftControl)
                && !is_key_down(KeyCode::RightControl)
                && !is_key_down(KeyCode::LeftAlt)
                && !is_key_down(KeyCode::RightAlt)
            {
                if is_key_pressed(KeyCode::V) { comp.active_tool = 0; }
                if is_key_pressed(KeyCode::H) { comp.active_tool = 1; }
                if is_key_pressed(KeyCode::Z) { comp.active_tool = 2; }
                if is_key_pressed(KeyCode::W) { comp.active_tool = 3; }
                if is_key_pressed(KeyCode::C) { comp.active_tool = 4; }
                if is_key_pressed(KeyCode::Y) { comp.active_tool = 5; }
                if is_key_pressed(KeyCode::Q) { comp.active_tool = 6; }
                if is_key_pressed(KeyCode::G) { comp.active_tool = 7; }
                if is_key_pressed(KeyCode::T) { comp.active_tool = 8; }
            }
        }

        // --- 1. RENDER ANIMATION TO TEXTURE & RAM CACHE ---
        let cur_frame_idx = (comp.current_time * comp.settings.fps.max(1) as f32).round() as usize;
        let is_cur_frame_cached = comp.ram_cache_enabled && ram_cache.frames.contains_key(&cur_frame_idx);

        if !is_cur_frame_cached {
            draw_composition(&comp, &textures, comp.current_time, render_target.clone());
            if comp.ram_cache_enabled {
                let image = render_target.texture.get_texture_data();
                ram_cache.insert(cur_frame_idx, &image, comp.cache_compression_enabled, comp.cache_compression_mode);
                comp.cached_frames.insert(cur_frame_idx);
            }
        }

        // --- AUTO FRAME CACHE ENGINE (Beginning to Final Keyframe) ---
        if comp.ram_cache_enabled && (comp.auto_frame_cache || comp.auto_cache_in_progress) {
            let fps = comp.settings.fps.max(1) as f32;
            let max_kf = get_max_keyframe_time(&comp);
            let end_time = if max_kf > 0.0 {
                max_kf
            } else {
                comp.work_area_out.min(comp.settings.duration)
            };
            let start_frame = 0usize;
            let end_frame = (end_time * fps).round() as usize;

            let batch_size = if comp.is_playing { 1 } else { 3 };
            let mut cached_count = 0;
            let mut any_missing = false;

            for f_idx in start_frame..=end_frame {
                if !ram_cache.frames.contains_key(&f_idx) {
                    any_missing = true;
                    let f_time = f_idx as f32 / fps;
                    draw_composition(&comp, &textures, f_time, render_target.clone());
                    let image = render_target.texture.get_texture_data();
                    ram_cache.insert(f_idx, &image, comp.cache_compression_enabled, comp.cache_compression_mode);
                    comp.cached_frames.insert(f_idx);
                    cached_count += 1;
                    if cached_count >= batch_size {
                        break;
                    }
                }
            }

            if !any_missing {
                comp.auto_cache_in_progress = false;
            }
        }
        clear_background(Color::from_rgba(18, 18, 20, 255));

        // --- 2. AFTER EFFECTS UI INTERFACE ---
        egui_macroquad::ui(|ctx| {
            egui_wants_keyboard = ctx.wants_keyboard_input();
            ctx.set_pixels_per_point(comp.settings.ui_scale);
            apply_after_effects_style(ctx);

            // ==========================================
            // TOP MENU BAR & APPLICATION TOOLBAR
            // ==========================================
            egui::TopBottomPanel::top("top_app_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;

                    // Menu Bar: File
                    ui.menu_button("File", |ui| {
                        if ui.button("📁 New Project").clicked() {
                            comp = Composition {
                                layers: vec![default_layer(
                                    "Solid 1".into(),
                                    LayerSource::Solid {
                                        color: [0.2, 0.2, 0.25, 1.0],
                                    },
                                    0,
                                )],
                                active_layer_index: Some(0),
                                ..Composition::default()
                            };
                            textures.clear();
                            sounds.clear();
                            ui.close_menu();
                        }
                        if ui.button("📂 Open Project... (Ctrl+O)").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("BeforeFX Project", &["bfx"])
                                .pick_file()
                            {
                                if let Ok(json) = std::fs::read_to_string(&path) {
                                    if let Ok(new_comp) = serde_json::from_str::<Composition>(&json)
                                    {
                                        comp = new_comp;
                                        textures.clear();
                                        sounds.clear();
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("💾 Save Project (Ctrl+S)").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("BeforeFX Project", &["bfx"])
                                .save_file()
                            {
                                if let Ok(json) = serde_json::to_string(&comp) {
                                    let _ = std::fs::write(path, json);
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("📥 Import Media", |ui| {
                            if ui.button("Image Asset...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                    .pick_file()
                                {
                                    let p_str = path.to_string_lossy().to_string();
                                    let n = path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    add_resource(&mut comp, n, p_str, ResourceKind::Image);
                                }
                                ui.close_menu();
                            }
                            if ui.button("Audio File...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Audio", &["wav", "ogg", "mp3"])
                                    .pick_file()
                                {
                                    let p_str = path.to_string_lossy().to_string();
                                    let n = path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    add_resource(&mut comp, n, p_str, ResourceKind::Audio);
                                }
                                ui.close_menu();
                            }
                            if ui.button("Video Footage...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Video", &["mp4", "mov", "avi"])
                                    .pick_file()
                                {
                                    let p_str = path.to_string_lossy().to_string();
                                    let n = path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string();
                                    add_resource(&mut comp, n, p_str, ResourceKind::Video);
                                }
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        if ui.button("📸 Export Frame (PNG)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG Image", &["png"])
                                .set_file_name("frame_render.png")
                                .save_file()
                            {
                                let image = render_target.texture.get_texture_data();
                                image.export_png(path.to_str().unwrap());
                            }
                            ui.close_menu();
                        }
                        if ui.button("🎬 Export Video (MP4)...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("MP4 Video", &["mp4"])
                                .set_file_name("render_output.mp4")
                                .save_file()
                            {
                                pending_export = Some(path);
                                export_status = "Encoding composition frames...".to_string();
                            }
                            ui.close_menu();
                        }
                    });

                    // Menu Bar: Edit
                    ui.menu_button("Edit", |ui| {
                        if ui.button("↩ Undo (Ctrl+Z)").clicked() {
                            if history_index > 0 {
                                history_index -= 1;
                                comp = history[history_index].clone();
                            }
                            ui.close_menu();
                        }
                        if ui.button("↪ Redo (Ctrl+Y)").clicked() {
                            if history_index + 1 < history.len() {
                                history_index += 1;
                                comp = history[history_index].clone();
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("📋 Duplicate Layer (Ctrl+D)").clicked() {
                            if let Some(idx) = comp.active_layer_index {
                                if idx < comp.layers.len() {
                                    let mut nl = comp.layers[idx].clone();
                                    nl.name = format!("{} Copy", nl.name);
                                    nl.label_color_index = (nl.label_color_index + 1) % AE_LABEL_COLORS.len();
                                    comp.layers.insert(idx + 1, nl);
                                    comp.active_layer_index = Some(idx + 1);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("✂ Split Layer (Ctrl+Shift+D)").clicked() {
                            if let Some(idx) = comp.active_layer_index {
                                if idx < comp.layers.len() {
                                    let curr = comp.current_time;
                                    if curr > comp.layers[idx].in_time && curr < comp.layers[idx].out_time {
                                        let mut second_layer = comp.layers[idx].clone();
                                        comp.layers[idx].out_time = curr;
                                        second_layer.in_time = curr;
                                        second_layer.name = format!("{} (Split)", second_layer.name);
                                        second_layer.label_color_index =
                                            (second_layer.label_color_index + 2) % AE_LABEL_COLORS.len();
                                        comp.layers.insert(idx + 1, second_layer);
                                        comp.active_layer_index = Some(idx + 1);
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("📍 Add Marker (*)").clicked() {
                            let num = comp.markers.len() + 1;
                            comp.markers.push(Marker {
                                time: comp.current_time,
                                label: format!("M{}", num),
                                comment: format!("Marker {}", num),
                                color_index: (num - 1) % AE_LABEL_COLORS.len(),
                            });
                            ui.close_menu();
                        }
                        if ui.button("🗑 Delete Selected (Del)").clicked() {
                            if selected_keyframe.is_some() {
                                ui_utils::delete_selected_keyframe(
                                    &mut comp,
                                    &mut selected_keyframe,
                                );
                            } else if let Some(idx) = comp.active_layer_index {
                                if idx < comp.layers.len() {
                                    comp.layers.remove(idx);
                                    comp.active_layer_index = if comp.layers.is_empty() {
                                        None
                                    } else {
                                        Some(idx.saturating_sub(1))
                                    };
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Select All Layers (Ctrl+A)").clicked() {
                            comp.active_layer_index = Some(0);
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("Purge", |ui| {
                            if ui.button("All Memory & Disk Cache (Ctrl+Alt+Num0)").clicked() {
                                comp.ram_cache_purge_requested = true;
                                ui.close_menu();
                            }
                        });
                        if ui.button("⚙ Cache & Compression Settings...").clicked() {
                            show_cache_settings_dialog = true;
                            ui.close_menu();
                        }
                        ui.menu_button("UI Scale", |ui| {
                            for scale in [0.75, 1.0, 1.25, 1.5, 2.0] {
                                if ui.button(format!("{}x", scale)).clicked() {
                                    comp.settings.ui_scale = scale;
                                }
                            }
                        });
                        if ui.button("⌨ Keyboard Shortcuts...").clicked() {
                            show_shortcuts_dialog = true;
                            ui.close_menu();
                        }
                    });

                    // Menu Bar: Composition
                    ui.menu_button("Composition", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Duration (s):");
                            ui.add(
                                egui::DragValue::new(&mut comp.settings.duration)
                                    .speed(0.5)
                                    .range(1.0..=600.0),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Framerate (FPS):");
                            ui.add(
                                egui::DragValue::new(&mut comp.settings.fps)
                                    .speed(1)
                                    .range(1..=120),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            ui.add(
                                egui::DragValue::new(&mut comp.settings.width)
                                    .speed(10)
                                    .range(320..=7680),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Height:");
                            ui.add(
                                egui::DragValue::new(&mut comp.settings.height)
                                    .speed(10)
                                    .range(240..=4320),
                            );
                        });
                        ui.separator();
                        if ui.button("⚙ Cache & Compression Settings...").clicked() {
                            show_cache_settings_dialog = true;
                            ui.close_menu();
                        }
                        if ui.button("⚡ RAM Preview (Num 0)").clicked() {
                            comp.is_ram_previewing = true;
                            comp.is_playing = true;
                            let max_kf = get_max_keyframe_time(&comp);
                            let playback_end = if comp.pause_at_last_keyframe && max_kf > 0.0 {
                                max_kf
                            } else {
                                comp.work_area_out.min(comp.settings.duration)
                            };
                            if comp.current_time < comp.work_area_in || comp.current_time >= playback_end {
                                comp.current_time = comp.work_area_in;
                            }
                            ui.close_menu();
                        }
                        if ui.button("⚡ Cache to Final Keyframe").clicked() {
                            comp.auto_cache_in_progress = true;
                            let fps = comp.settings.fps.max(1) as f32;
                            let max_kf = get_max_keyframe_time(&comp);
                            let end_time = if max_kf > 0.0 {
                                max_kf
                            } else {
                                comp.work_area_out.min(comp.settings.duration)
                            };
                            let start_frame = 0usize;
                            let end_frame = (end_time * fps).round() as usize;
                            for f_idx in start_frame..=end_frame {
                                if !ram_cache.frames.contains_key(&f_idx) {
                                    let f_time = f_idx as f32 / fps;
                                    draw_composition(&comp, &textures, f_time, render_target.clone());
                                    let image = render_target.texture.get_texture_data();
                                    ram_cache.insert(f_idx, &image, comp.cache_compression_enabled, comp.cache_compression_mode);
                                    comp.cached_frames.insert(f_idx);
                                }
                            }
                            ui.close_menu();
                        }
                        ui.checkbox(&mut comp.auto_frame_cache, "⚡ Auto Cache (0 -> Final Keyframe)");
                        ui.checkbox(&mut comp.pause_at_last_keyframe, "⏸ Pause at Last Keyframe");
                        ui.separator();
                        if ui.button("Set Work Area to Duration").clicked() {
                            comp.work_area_in = 0.0;
                            comp.work_area_out = comp.settings.duration;
                            ui.close_menu();
                        }
                    });

                    // Menu Bar: Layer
                    ui.menu_button("Layer", |ui| {
                        ui.menu_button("New", |ui| {
                            if ui.button("Solid Layer...").clicked() {
                                let idx = comp.layers.len();
                                comp.layers.push(default_layer(
                                    format!("Solid {}", idx + 1),
                                    LayerSource::Solid {
                                        color: [0.8, 0.25, 0.25, 1.0],
                                    },
                                    idx % AE_LABEL_COLORS.len(),
                                ));
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                            if ui.button("Text Layer").clicked() {
                                let idx = comp.layers.len();
                                comp.layers.push(default_layer(
                                    format!("Text {}", idx + 1),
                                    LayerSource::Text {
                                        text: "New Text".to_string(),
                                        font_size: 48.0,
                                        color: [1.0, 1.0, 1.0, 1.0],
                                    },
                                    (idx + 1) % AE_LABEL_COLORS.len(),
                                ));
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                            if ui.button("Polygon / Shape Layer").clicked() {
                                let idx = comp.layers.len();
                                comp.layers.push(default_layer(
                                    format!("Shape {}", idx + 1),
                                    LayerSource::Polygon {
                                        points: vec![
                                            [0.0, -70.0],
                                            [70.0, 0.0],
                                            [35.0, 80.0],
                                            [-55.0, 70.0],
                                            [-85.0, -20.0],
                                        ],
                                        color: [0.95, 0.55, 0.18, 1.0],
                                    },
                                    (idx + 2) % AE_LABEL_COLORS.len(),
                                ));
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                            if ui.button("3D Cube").clicked() {
                                let idx = comp.layers.len();
                                let mut l = default_layer(
                                    format!("3D Cube {}", idx + 1),
                                    LayerSource::Object3D {
                                        path: None,
                                        color: [0.25, 0.55, 0.95, 1.0],
                                    },
                                    (idx + 3) % AE_LABEL_COLORS.len(),
                                );
                                l.d3 = true;
                                comp.layers.push(l);
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                            if ui.button("📷 Camera Layer (3D)").clicked() {
                                let idx = comp.layers.len();
                                let mut l = default_layer(
                                    format!("Camera {}", idx + 1),
                                    LayerSource::Camera,
                                    5,
                                );
                                l.d3 = true;
                                l.properties = create_camera_properties(
                                    comp.settings.width as f32,
                                    comp.settings.height as f32,
                                );
                                comp.layers.push(l);
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                            if ui.button("Null Object").clicked() {
                                let idx = comp.layers.len();
                                comp.layers.push(default_layer(
                                    format!("Null {}", idx + 1),
                                    LayerSource::Null,
                                    11,
                                ));
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                            if ui.button("Adjustment Layer").clicked() {
                                let idx = comp.layers.len();
                                comp.layers.push(default_layer(
                                    format!("Adjustment Layer {}", idx + 1),
                                    LayerSource::Adjustment,
                                    4,
                                ));
                                comp.active_layer_index = Some(idx);
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        if ui.button("Reset Transform").clicked() {
                            if let Some(idx) = comp.active_layer_index {
                                if let Some(l) = comp.layers.get_mut(idx) {
                                    l.properties = create_default_properties();
                                }
                            }
                            ui.close_menu();
                        }
                    });

                    // Menu Bar: Animation
                    ui.menu_button("Animation", |ui| {
                        if ui.button("Easy Ease (F9)").clicked() {
                            if let Some(sel) = &selected_keyframe {
                                if let Some(prop) = comp
                                    .layers
                                    .get_mut(sel.layer_index)
                                    .and_then(|l| l.properties.get_mut(&sel.property_name))
                                {
                                    if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                                        kf.ease = Some(BezierControl::easy_ease());
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Ease In (Shift+F9)").clicked() {
                            if let Some(sel) = &selected_keyframe {
                                if let Some(prop) = comp
                                    .layers
                                    .get_mut(sel.layer_index)
                                    .and_then(|l| l.properties.get_mut(&sel.property_name))
                                {
                                    if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                                        kf.ease = Some(BezierControl::ease_in());
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Ease Out (Ctrl+Shift+F9)").clicked() {
                            if let Some(sel) = &selected_keyframe {
                                if let Some(prop) = comp
                                    .layers
                                    .get_mut(sel.layer_index)
                                    .and_then(|l| l.properties.get_mut(&sel.property_name))
                                {
                                    if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                                        kf.ease = Some(BezierControl::ease_out());
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Linear Interpolation").clicked() {
                            if let Some(sel) = &selected_keyframe {
                                if let Some(prop) = comp
                                    .layers
                                    .get_mut(sel.layer_index)
                                    .and_then(|l| l.properties.get_mut(&sel.property_name))
                                {
                                    if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                                        kf.ease = Some(BezierControl::linear());
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Anticipation / Overshoot").clicked() {
                            if let Some(sel) = &selected_keyframe {
                                if let Some(prop) = comp
                                    .layers
                                    .get_mut(sel.layer_index)
                                    .and_then(|l| l.properties.get_mut(&sel.property_name))
                                {
                                    if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                                        kf.ease = Some(BezierControl::back_out());
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Exponential / Bounce").clicked() {
                            if let Some(sel) = &selected_keyframe {
                                if let Some(prop) = comp
                                    .layers
                                    .get_mut(sel.layer_index)
                                    .and_then(|l| l.properties.get_mut(&sel.property_name))
                                {
                                    if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                                        kf.ease = Some(BezierControl::exponential());
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                    });

                    // Menu Bar: Effect
                    ui.menu_button("Effect", |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Target:").strong());
                            if ui.selectable_label(!comp.effect_target_viewport, "🖼 Layer").clicked() {
                                comp.effect_target_viewport = false;
                            }
                            if ui.selectable_label(comp.effect_target_viewport, "🖥 Viewport").clicked() {
                                comp.effect_target_viewport = true;
                            }
                        });
                        ui.separator();

                        let mut add_effect_target = |eff: LayerEffect| {
                            if comp.effect_target_viewport {
                                comp.viewport_effects.push(eff.clone());
                                export_status = format!("Added effect '{}' to Viewport.", eff.name);
                            } else if let Some(idx) = comp.active_layer_index {
                                if let Some(l) = comp.layers.get_mut(idx) {
                                    l.effects.push(eff.clone());
                                    export_status = format!("Added effect '{}' to layer.", eff.name);
                                }
                            } else {
                                comp.viewport_effects.push(eff.clone());
                                export_status = format!("Added effect '{}' to Viewport.", eff.name);
                            }
                            history.truncate(history_index + 1);
                            history.push(comp.clone());
                            history_index = history.len() - 1;
                        };

                        ui.menu_button("Glitch & Retro", |ui| {
                            if ui.button("MP4 Ultra Compress & Corrupt").clicked() {
                                add_effect_target(LayerEffect::new("MP4 Ultra Compress & Corrupt".to_string(), EffectType::Mp4UltraCompress));
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Blur & Sharpen", |ui| {
                            if ui.button("Fast Blur").clicked() {
                                add_effect_target(LayerEffect::new("Fast Blur".to_string(), EffectType::FastBlur));
                                ui.close_menu();
                            }
                            if ui.button("Directional Blur").clicked() {
                                add_effect_target(LayerEffect::new("Directional Blur".to_string(), EffectType::DirectionalBlur));
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Color Correction", |ui| {
                            if ui.button("Brightness & Contrast").clicked() {
                                add_effect_target(LayerEffect::new("Brightness & Contrast".to_string(), EffectType::BrightnessContrast));
                                ui.close_menu();
                            }
                            if ui.button("Hue/Saturation").clicked() {
                                add_effect_target(LayerEffect::new("Hue/Saturation".to_string(), EffectType::HueSaturation));
                                ui.close_menu();
                            }
                            if ui.button("Tint").clicked() {
                                add_effect_target(LayerEffect::new("Tint".to_string(), EffectType::Tint));
                                ui.close_menu();
                            }
                            if ui.button("Invert").clicked() {
                                add_effect_target(LayerEffect::new("Invert".to_string(), EffectType::Invert));
                                ui.close_menu();
                            }
                            if ui.button("Fill").clicked() {
                                add_effect_target(LayerEffect::new("Fill".to_string(), EffectType::Fill));
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Distort & Stylize", |ui| {
                            if ui.button("Chromatic Aberration").clicked() {
                                add_effect_target(LayerEffect::new("Chromatic Aberration".to_string(), EffectType::ChromaticAberration));
                                ui.close_menu();
                            }
                            if ui.button("Wave Warp").clicked() {
                                add_effect_target(LayerEffect::new("Wave Warp".to_string(), EffectType::WaveWarp));
                                ui.close_menu();
                            }
                            if ui.button("Glow").clicked() {
                                add_effect_target(LayerEffect::new("Glow".to_string(), EffectType::Glow));
                                ui.close_menu();
                            }
                            if ui.button("Vignette").clicked() {
                                add_effect_target(LayerEffect::new("Vignette".to_string(), EffectType::Vignette));
                                ui.close_menu();
                            }
                            if ui.button("Drop Shadow").clicked() {
                                add_effect_target(LayerEffect::new("Drop Shadow".to_string(), EffectType::DropShadow));
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Plugins", |ui| {
                            if plugin_registry.effects.is_empty() {
                                ui.label(
                                    egui::RichText::new("No effect plugins in /plugins")
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(130)),
                                );
                            } else {
                                for eff_plug in &plugin_registry.effects {
                                    if ui
                                        .button(format!("🎨 {}", &eff_plug.name))
                                        .on_hover_text(&eff_plug.description)
                                        .clicked()
                                    {
                                        add_effect_target(LayerEffect::new_plugin(eff_plug));
                                        ui.close_menu();
                                    }
                                }
                            }
                        });
                    });

                    // Menu Bar: Plugins
                    ui.menu_button("Plugins", |ui| {
                        if ui.button("🔄 Reload Plugins (from /plugins)").clicked() {
                            plugin_registry.reload();
                            export_status = format!(
                                "Reloaded plugins: {} effects, {} functional",
                                plugin_registry.effects.len(),
                                plugin_registry.functionals.len()
                            );
                            ui.close_menu();
                        }
                        if ui.button("📁 Open Plugins Folder").clicked() {
                            let _ = std::process::Command::new("explorer").arg("plugins").spawn();
                            ui.close_menu();
                        }
                        if ui.button("⚙ Plugin Manager...").clicked() {
                            show_plugin_manager = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("⚡ Functional Plugins")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 200, 100)),
                        );
                        if plugin_registry.functionals.is_empty() {
                            ui.label(
                                egui::RichText::new("  (No functional plugins loaded)")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(120)),
                            );
                        } else {
                            for func in &plugin_registry.functionals {
                                if ui
                                    .button(format!("⚡ {}", &func.name))
                                    .on_hover_text(&func.description)
                                    .clicked()
                                {
                                    match execute_functional_plugin(&mut comp, func, None) {
                                        Ok(msg) => {
                                            export_status = format!("Plugin: {}", msg);
                                            history.truncate(history_index + 1);
                                            history.push(comp.clone());
                                            history_index = history.len() - 1;
                                        }
                                        Err(err) => {
                                            export_status = format!("Plugin error: {}", err);
                                        }
                                    }
                                    ui.close_menu();
                                }
                            }
                        }
                        ui.separator();
                        ui.label(
                            egui::RichText::new("🎨 Effect Plugins")
                                .strong()
                                .color(egui::Color32::from_rgb(120, 200, 255)),
                        );
                        if plugin_registry.effects.is_empty() {
                            ui.label(
                                egui::RichText::new("  (No effect plugins loaded)")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(120)),
                            );
                        } else {
                            for eff_plug in &plugin_registry.effects {
                                if ui
                                    .button(format!("🎨 {}", &eff_plug.name))
                                    .on_hover_text(&eff_plug.description)
                                    .clicked()
                                {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            l.effects.push(LayerEffect::new_plugin(eff_plug));
                                            export_status = format!("Added effect '{}' to layer.", eff_plug.name);
                                            history.truncate(history_index + 1);
                                            history.push(comp.clone());
                                            history_index = history.len() - 1;
                                        }
                                    }
                                    ui.close_menu();
                                }
                            }
                        }
                    });

                    // Menu Bar: View
                    ui.menu_button("View", |ui| {
                        ui.checkbox(&mut comp.show_guides, "Title / Action Safe Guides");
                        ui.checkbox(&mut comp.show_grid, "Grid Overlay");
                        ui.checkbox(&mut comp.show_checkerboard, "Transparency Checkerboard");
                    });

                    // Menu Bar: Help
                    ui.menu_button("Help", |ui| {
                        if ui.button("⌨ Keyboard Shortcuts...").clicked() {
                            show_shortcuts_dialog = true;
                            ui.close_menu();
                        }
                        if ui.button("ℹ About BeforeFX").clicked() {
                            show_shortcuts_dialog = true;
                            ui.close_menu();
                        }
                    });

                    ui.separator();

                    // --- AFTER EFFECTS QUICK TOOL PALETTE ---
                    let tools = [
                        ("↖ V", "Selection Tool (V)", 0),
                        ("✋ H", "Hand Tool (H)", 1),
                        ("🔍 Z", "Zoom Tool (Z)", 2),
                        ("🔄 W", "Rotation Tool (W)", 3),
                        ("🎥 C", "Camera Tool (C)", 4),
                        ("⚓ Y", "Pan Behind / Anchor Point Tool (Y)", 5),
                        ("▭ Q", "Shape / Rectangle Tool (Q)", 6),
                        ("✒ G", "Pen Tool (G)", 7),
                        ("T", "Type Tool (T)", 8),
                    ];

                    for (icon, label, idx) in tools {
                        let is_active = comp.active_tool == idx;
                        if ui
                            .add(egui::SelectableLabel::new(
                                is_active,
                                egui::RichText::new(icon).strong(),
                            ))
                            .on_hover_text(label)
                            .clicked()
                        {
                            comp.active_tool = idx;
                        }
                    }

                    ui.separator();
                    ui.label(
                        egui::RichText::new("Workspace: Standard")
                            .size(11.0)
                            .color(egui::Color32::from_gray(140)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !export_status.is_empty() {
                            ui.label(
                                egui::RichText::new(&export_status)
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(140, 200, 240)),
                            );
                        }
                    });
                });
            });

            // ==========================================
            // LEFT PANEL: PROJECT & EFFECT CONTROLS
            // ==========================================
            egui::SidePanel::left("left_dock")
                .resizable(true)
                .default_width(300.0)
                .width_range(240.0..=500.0)
                .show(ctx, |ui| {
                    // Tab header
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(comp.left_panel_tab == 0, "📁 Project")
                            .clicked()
                        {
                            comp.left_panel_tab = 0;
                        }
                        if ui
                            .selectable_label(comp.left_panel_tab == 1, "⚡ Effect Controls")
                            .clicked()
                        {
                            comp.left_panel_tab = 1;
                        }
                    });
                    ui.separator();

                    match comp.left_panel_tab {
                        0 => {
                            // PROJECT TAB
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut comp.search_query)
                                        .hint_text("🔍 Filter assets...")
                                        .desired_width(ui.available_width()),
                                );
                            });

                            // Comp Info Card
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("🎬");
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new("Comp 1")
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}x{} @ {}fps, {:.1}s",
                                                comp.settings.width,
                                                comp.settings.height,
                                                comp.settings.fps,
                                                comp.settings.duration
                                            ))
                                            .size(10.5)
                                            .color(egui::Color32::from_gray(140)),
                                        );
                                    });
                                });
                            });

                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Imported Media & Items:")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(160)),
                            );

                            egui::ScrollArea::vertical()
                                .id_salt("project_resources_scroll")
                                .max_height(ui.available_height() - 42.0)
                                .show(ui, |ui| {
                                    for res in &comp.resources {
                                        if !comp.search_query.is_empty()
                                            && !res
                                                .name
                                                .to_lowercase()
                                                .contains(&comp.search_query.to_lowercase())
                                        {
                                            continue;
                                        }

                                        ui.horizontal(|ui| {
                                            ui.label(resource_icon(res.kind));
                                            ui.label(
                                                egui::RichText::new(&res.name)
                                                    .color(egui::Color32::from_rgb(180, 210, 240)),
                                            );
                                        });
                                    }
                                    if comp.resources.is_empty() {
                                        ui.label(
                                            egui::RichText::new(
                                                "No external media imported.\nImport files via File > Import.",
                                            )
                                            .size(11.0)
                                            .color(egui::Color32::from_gray(110)),
                                        );
                                    }
                                });

                            // Project bottom action bar
                            ui.with_layout(
                                egui::Layout::bottom_up(egui::Align::LEFT),
                                |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("📥 Import").clicked() {
                                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                                let p_str = path.to_string_lossy().to_string();
                                                let n = path
                                                    .file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy()
                                                    .to_string();
                                                add_resource(
                                                    &mut comp,
                                                    n,
                                                    p_str,
                                                    ResourceKind::Image,
                                                );
                                            }
                                        }
                                        if ui.button("+ Solid").clicked() {
                                            let idx = comp.layers.len();
                                            comp.layers.push(default_layer(
                                                format!("Solid {}", idx + 1),
                                                LayerSource::Solid {
                                                    color: [0.3, 0.4, 0.6, 1.0],
                                                },
                                                idx % AE_LABEL_COLORS.len(),
                                            ));
                                            comp.active_layer_index = Some(idx);
                                        }
                                        if ui.button("+ Text").clicked() {
                                            let idx = comp.layers.len();
                                            comp.layers.push(default_layer(
                                                format!("Text {}", idx + 1),
                                                LayerSource::Text {
                                                    text: "Text".to_string(),
                                                    font_size: 40.0,
                                                    color: [1.0, 1.0, 1.0, 1.0],
                                                },
                                                (idx + 1) % AE_LABEL_COLORS.len(),
                                            ));
                                            comp.active_layer_index = Some(idx);
                                        }
                                        if ui.button("+ Camera").clicked() {
                                            let idx = comp.layers.len();
                                            let mut l = default_layer(
                                                format!("Camera {}", idx + 1),
                                                LayerSource::Camera,
                                                5,
                                            );
                                            l.d3 = true;
                                            l.properties = create_camera_properties(
                                                comp.settings.width as f32,
                                                comp.settings.height as f32,
                                            );
                                            comp.layers.push(l);
                                            comp.active_layer_index = Some(idx);
                                        }
                                        if ui.button("+ 3D Cube").clicked() {
                                            let idx = comp.layers.len();
                                            let mut l = default_layer(
                                                format!("3D Cube {}", idx + 1),
                                                LayerSource::Object3D {
                                                    path: None,
                                                    color: [0.25, 0.55, 0.95, 1.0],
                                                },
                                                (idx + 3) % AE_LABEL_COLORS.len(),
                                            );
                                            l.d3 = true;
                                            comp.layers.push(l);
                                            comp.active_layer_index = Some(idx);
                                        }
                                    });
                                    ui.separator();
                                },
                            );
                        }
                        1 => {
                            // EFFECT CONTROLS TAB
                            if let Some(active_idx) = comp.active_layer_index {
                                if active_idx < comp.layers.len() {
                                    let mut new_effects = Vec::new();
                                    let mut remove_eff_idx: Option<usize> = None;
                                    let current_time = comp.current_time;

                                    {
                                        let layer = &mut comp.layers[active_idx];
                                        let label_col = get_label_color(layer.label_color_index);
                                        ui.horizontal(|ui| {
                                            let (swatch, _) = ui.allocate_exact_size(
                                                egui::vec2(10.0, 16.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(swatch, 1.0, label_col);
                                            ui.label(
                                                egui::RichText::new(&layer.name)
                                                    .strong()
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                ui.checkbox(&mut layer.fx, "Enable FX");
                                            });
                                        });

                                        ui.separator();

                                        egui::ScrollArea::vertical().show(ui, |ui| {
                                            // Transform Section
                                            ui.collapsing(egui::RichText::new("▼ Transform").strong().color(egui::Color32::from_gray(190)), |ui| {
                                                let prop_names = sorted_property_names(layer);
                                                for name in prop_names {
                                                    if let Some(prop) = layer.properties.get_mut(&name) {
                                                        ui.horizontal(|ui| {
                                                            let has_kf = !prop.keyframes.is_empty();
                                                            let sw_col = if has_kf {
                                                                egui::Color32::from_rgb(70, 180, 255)
                                                            } else {
                                                                egui::Color32::from_gray(100)
                                                            };
                                                            if ui.button(egui::RichText::new("⏱").size(10.0).color(sw_col)).on_hover_text("Toggle animation stopwatch").clicked() {
                                                                if has_kf {
                                                                    prop.keyframes.clear();
                                                                } else {
                                                                    prop.keyframes.push(Keyframe {
                                                                        time: current_time,
                                                                        value: prop.base_value,
                                                                        ease: None,
                                                                    });
                                                                }
                                                            }

                                                            ui.label(
                                                                egui::RichText::new(property_display_name(&name))
                                                                    .size(11.0)
                                                                    .color(egui::Color32::from_gray(180)),
                                                            );

                                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                let speed = if name.contains("scale") || name.contains("opacity") {
                                                                    0.5
                                                                } else {
                                                                    0.25
                                                                };
                                                                ui.add(egui::DragValue::new(&mut prop.base_value).speed(speed));
                                                            });
                                                        });
                                                    }
                                                }
                                            });

                                            ui.add_space(4.0);
                                            ui.separator();

                                            // Effects Stack Header
                                            let eff_count = layer.effects.len();
                                            let active_eff_count = layer.effects.iter().filter(|e| e.enabled).count();
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!("⚡ Effects Stack ({}/{} active)", active_eff_count, eff_count))
                                                        .color(egui::Color32::from_rgb(100, 190, 255))
                                                        .strong(),
                                                );
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    ui.menu_button("+ Add Effect", |ui| {
                                                        ui.menu_button("Blur & Sharpen", |ui| {
                                                            if ui.button("Fast Blur").clicked() {
                                                                new_effects.push(LayerEffect::new("Fast Blur".to_string(), EffectType::FastBlur));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Directional Blur").clicked() {
                                                                new_effects.push(LayerEffect::new("Directional Blur".to_string(), EffectType::DirectionalBlur));
                                                                ui.close_menu();
                                                            }
                                                        });
                                                        ui.menu_button("Color Correction", |ui| {
                                                            if ui.button("Brightness & Contrast").clicked() {
                                                                new_effects.push(LayerEffect::new("Brightness & Contrast".to_string(), EffectType::BrightnessContrast));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Hue/Saturation").clicked() {
                                                                new_effects.push(LayerEffect::new("Hue/Saturation".to_string(), EffectType::HueSaturation));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Tint").clicked() {
                                                                new_effects.push(LayerEffect::new("Tint".to_string(), EffectType::Tint));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Invert").clicked() {
                                                                new_effects.push(LayerEffect::new("Invert".to_string(), EffectType::Invert));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Fill").clicked() {
                                                                new_effects.push(LayerEffect::new("Fill".to_string(), EffectType::Fill));
                                                                ui.close_menu();
                                                            }
                                                        });
                                                        ui.menu_button("Distort & Stylize", |ui| {
                                                            if ui.button("Chromatic Aberration").clicked() {
                                                                new_effects.push(LayerEffect::new("Chromatic Aberration".to_string(), EffectType::ChromaticAberration));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Wave Warp").clicked() {
                                                                new_effects.push(LayerEffect::new("Wave Warp".to_string(), EffectType::WaveWarp));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Glow").clicked() {
                                                                new_effects.push(LayerEffect::new("Glow".to_string(), EffectType::Glow));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Vignette").clicked() {
                                                                new_effects.push(LayerEffect::new("Vignette".to_string(), EffectType::Vignette));
                                                                ui.close_menu();
                                                            }
                                                            if ui.button("Drop Shadow").clicked() {
                                                                new_effects.push(LayerEffect::new("Drop Shadow".to_string(), EffectType::DropShadow));
                                                                ui.close_menu();
                                                            }
                                                        });
                                                        ui.menu_button("Plugins", |ui| {
                                                            if plugin_registry.effects.is_empty() {
                                                                ui.label(
                                                                    egui::RichText::new("No effect plugins in /plugins")
                                                                        .size(11.0)
                                                                        .color(egui::Color32::from_gray(130)),
                                                                );
                                                            } else {
                                                                for eff_plug in &plugin_registry.effects {
                                                                    if ui
                                                                        .button(format!("🎨 {}", &eff_plug.name))
                                                                        .on_hover_text(&eff_plug.description)
                                                                        .clicked()
                                                                    {
                                                                        new_effects.push(LayerEffect::new_plugin(eff_plug));
                                                                        ui.close_menu();
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    });
                                                });
                                            });

                                            ui.add_space(2.0);

                                            if layer.effects.is_empty() {
                                                ui.label(
                                                    egui::RichText::new("No effects applied to this layer.\nUse '+ Add Effect' or double-click in Effects & Presets.")
                                                        .size(11.0)
                                                        .color(egui::Color32::from_gray(120)),
                                                );
                                            }

                                            // Effects List
                                            for (e_idx, eff) in layer.effects.iter_mut().enumerate() {
                                                ui.group(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.checkbox(&mut eff.enabled, "");
                                                        ui.label(
                                                            egui::RichText::new(format!("fx {}", &eff.name))
                                                                .strong()
                                                                .color(if eff.enabled {
                                                                    egui::Color32::from_rgb(120, 200, 255)
                                                                } else {
                                                                    egui::Color32::from_gray(120)
                                                                }),
                                                        );
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui.button("🗑").on_hover_text("Delete effect").clicked() {
                                                                remove_eff_idx = Some(e_idx);
                                                            }
                                                        });
                                                    });

                                                    if eff.enabled {
                                                        ui.indent(format!("eff_ind_{}", e_idx), |ui| {
                                                            let mut prop_keys: Vec<String> = eff.properties.keys().cloned().collect();
                                                            prop_keys.sort();
                                                            for key in prop_keys {
                                                                if let Some(prop) = eff.properties.get_mut(&key) {
                                                                    ui.horizontal(|ui| {
                                                                        let has_kf = !prop.keyframes.is_empty();
                                                                        let sw_col = if has_kf {
                                                                            egui::Color32::from_rgb(70, 180, 255)
                                                                        } else {
                                                                            egui::Color32::from_gray(100)
                                                                        };
                                                                        if ui.button(egui::RichText::new("⏱").size(10.0).color(sw_col)).on_hover_text("Toggle parameter stopwatch").clicked() {
                                                                            if has_kf {
                                                                                prop.keyframes.clear();
                                                                            } else {
                                                                                prop.keyframes.push(Keyframe {
                                                                                    time: current_time,
                                                                                    value: prop.base_value,
                                                                                    ease: None,
                                                                                });
                                                                            }
                                                                        }

                                                                        if has_kf {
                                                                            let has_kf_at_cur = prop.keyframes.iter().any(|k| (k.time - current_time).abs() < 0.05);
                                                                            let dia_col = if has_kf_at_cur {
                                                                                egui::Color32::from_rgb(255, 205, 50)
                                                                            } else {
                                                                                egui::Color32::from_gray(140)
                                                                            };
                                                                            if ui.button(egui::RichText::new("◆").size(10.0).color(dia_col)).on_hover_text("Add/remove keyframe at playhead").clicked() {
                                                                                if let Some(pos) = prop.keyframes.iter().position(|k| (k.time - current_time).abs() < 0.05) {
                                                                                    prop.keyframes.remove(pos);
                                                                                } else {
                                                                                    let cur_val = prop.get_value_at(current_time);
                                                                                    prop.keyframes.push(Keyframe {
                                                                                        time: current_time,
                                                                                        value: cur_val,
                                                                                        ease: None,
                                                                                    });
                                                                                    prop.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
                                                                                }
                                                                            }
                                                                        }

                                                                        ui.label(
                                                                            egui::RichText::new(property_display_name(&key))
                                                                                .size(11.0)
                                                                                .color(egui::Color32::from_gray(170)),
                                                                        );

                                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                            let speed = if key.contains("color") || key.contains("radius") || key.contains("distance") {
                                                                                1.0
                                                                            } else if key.contains("opacity") || key.contains("amount") || key.contains("intensity") || key.contains("feather") {
                                                                                0.5
                                                                            } else {
                                                                                0.25
                                                                            };
                                                                            ui.add(egui::DragValue::new(&mut prop.base_value).speed(speed));
                                                                        });
                                                                    });
                                                                }
                                                            }
                                                        });
                                                    }
                                                });
                                                ui.add_space(2.0);
                                            }
                                        });
                                    }

                                    if let Some(rem_idx) = remove_eff_idx {
                                        if rem_idx < comp.layers[active_idx].effects.len() {
                                            comp.layers[active_idx].effects.remove(rem_idx);
                                        }
                                    }
                                    for new_eff in new_effects {
                                        comp.layers[active_idx].effects.push(new_eff);
                                    }
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(
                                        "No layer selected.\nSelect a layer in the timeline to view effects.",
                                    )
                                    .color(egui::Color32::from_gray(120)),
                                );
                            }
                        }
                        _ => {}
                    }
                });

            // ==========================================
            // RIGHT PANEL: PREVIEW, ALIGN, AUDIO, PRESETS
            // ==========================================
            egui::SidePanel::right("right_dock")
                .resizable(true)
                .default_width(280.0)
                .width_range(220.0..=450.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let tabs = [
                            ("▶ Preview", 0),
                            ("📐 Align", 1),
                            ("📊 Audio", 2),
                            ("✨ Effects", 3),
                            ("T Text", 4),
                        ];
                        for (title, idx) in tabs {
                            if ui
                                .selectable_label(comp.right_panel_tab == idx, title)
                                .clicked()
                            {
                                comp.right_panel_tab = idx;
                            }
                        }
                    });
                    ui.separator();

                    match comp.right_panel_tab {
                        0 => {
                            // PREVIEW TAB
                            ui.label(
                                egui::RichText::new("Preview / Transport")
                                    .strong()
                                    .color(egui::Color32::from_gray(200)),
                            );
                            ui.add_space(4.0);

                            // Big Timecode Display
                            let tc = format_timecode(comp.current_time, comp.settings.fps);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(tc)
                                        .monospace()
                                        .size(22.0)
                                        .color(egui::Color32::from_rgb(80, 225, 200)),
                                );
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui.button("|◀ First").clicked() {
                                    comp.current_time = comp.work_area_in;
                                }
                                if ui.button("◀ -1f").clicked() {
                                    comp.current_time = (comp.current_time
                                        - 1.0 / comp.settings.fps.max(1) as f32)
                                        .max(0.0);
                                }
                                let play_text = if comp.is_playing && !comp.is_ram_previewing {
                                    "⏸ Pause"
                                } else {
                                    "▶ Play"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(play_text).strong(),
                                        )
                                        .min_size(egui::vec2(55.0, 24.0)),
                                    )
                                    .clicked()
                                {
                                    comp.is_ram_previewing = false;
                                    comp.is_playing = !comp.is_playing;
                                }
                                if ui.button("+1f ▶").clicked() {
                                    comp.current_time = (comp.current_time
                                        + 1.0 / comp.settings.fps.max(1) as f32)
                                        .min(comp.settings.duration);
                                }
                                if ui.button("Last ▶|").clicked() {
                                    comp.current_time =
                                        comp.work_area_out.min(comp.settings.duration);
                                }
                            });

                            ui.add_space(4.0);
                            let ram_preview_text = if comp.is_ram_previewing {
                                "⏹ Stop RAM Preview"
                            } else {
                                "⚡ RAM Preview (Num 0)"
                            };
                            let ram_preview_color = if comp.is_ram_previewing {
                                egui::Color32::from_rgb(255, 100, 100)
                            } else {
                                egui::Color32::from_rgb(45, 215, 95)
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(ram_preview_text)
                                            .strong()
                                            .color(ram_preview_color),
                                    )
                                    .min_size(egui::vec2(ui.available_width(), 24.0)),
                                )
                                .on_hover_text("Pre-renders and caches composition frames into RAM for smooth 60 FPS real-time playback looping.")
                                .clicked()
                            {
                                comp.is_ram_previewing = !comp.is_ram_previewing;
                                if comp.is_ram_previewing {
                                    comp.is_playing = true;
                                    if comp.current_time < comp.work_area_in || comp.current_time >= comp.work_area_out.min(comp.settings.duration) {
                                        comp.current_time = comp.work_area_in;
                                    }
                                } else {
                                    comp.is_playing = false;
                                }
                            }

                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // RAM Preview Performance & Stats
                            ui.label(egui::RichText::new("RAM Cache & Performance").strong().size(11.0));

                            let target_fps = comp.settings.fps.max(1) as f32;
                            let (fps_color, fps_label) = if comp.is_playing {
                                if comp.playback_fps >= target_fps * 0.92 {
                                    (egui::Color32::from_rgb(45, 215, 95), format!("● Realtime ({:.1} fps)", comp.playback_fps))
                                } else {
                                    (egui::Color32::from_rgb(255, 175, 40), format!("● Dropping ({:.1} / {:.0} fps)", comp.playback_fps, target_fps))
                                }
                            } else {
                                (egui::Color32::from_gray(140), format!("○ Paused ({:.0} fps target)", target_fps))
                            };

                            ui.horizontal(|ui| {
                                ui.label("Playback:");
                                ui.label(egui::RichText::new(fps_label).color(fps_color).strong());
                            });

                            let fps = comp.settings.fps.max(1) as f32;
                            let work_start_f = (comp.work_area_in * fps).round() as usize;
                            let work_end_f = (comp.work_area_out.min(comp.settings.duration) * fps).round() as usize;
                            let total_work_frames = (work_end_f.saturating_sub(work_start_f) + 1).max(1);
                            let cached_in_work = (work_start_f..=work_end_f).filter(|f| comp.cached_frames.contains(f)).count();
                            let percent_cached = (cached_in_work as f32 / total_work_frames as f32 * 100.0).clamp(0.0, 100.0);

                            ui.horizontal(|ui| {
                                ui.label("Work Area Cache:");
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}/{} frames ({:.0}%)",
                                        cached_in_work, total_work_frames, percent_cached
                                    ))
                                    .color(if percent_cached > 99.0 {
                                        egui::Color32::from_rgb(45, 215, 95)
                                    } else {
                                        egui::Color32::from_gray(180)
                                    }),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Cache Memory:");
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{:.1} MB ({} frames)",
                                        comp.cache_size_mb, comp.cached_frames.len()
                                    ))
                                    .color(egui::Color32::from_gray(160)),
                                );
                            });

                            let max_kf = get_max_keyframe_time(&comp);
                            if max_kf > 0.0 {
                                let end_kf_frame = (max_kf * fps).round() as usize;
                                let total_kf_frames = end_kf_frame + 1;
                                let cached_kf_count = (0..=end_kf_frame).filter(|f| comp.cached_frames.contains(f)).count();
                                let kf_cached_pct = (cached_kf_count as f32 / total_kf_frames as f32 * 100.0).clamp(0.0, 100.0);

                                ui.horizontal(|ui| {
                                    ui.label("Keyframe Span (0 -> KF):");
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}/{} frames ({:.0}%) [{:.2}s]",
                                            cached_kf_count, total_kf_frames, kf_cached_pct, max_kf
                                        ))
                                        .color(if kf_cached_pct > 99.0 {
                                            egui::Color32::from_rgb(45, 215, 95)
                                        } else {
                                            egui::Color32::from_rgb(255, 205, 50)
                                        })
                                        .strong(),
                                    );
                                });
                            }

                            ui.add_space(4.0);
                            ui.checkbox(&mut comp.pause_at_last_keyframe, "⏸ Pause at Last Keyframe")
                                .on_hover_text("Automatically pause playback when reaching the final keyframe instead of looping.");
                            ui.checkbox(&mut comp.auto_frame_cache, "⚡ Auto Cache (0 -> Final Keyframe)")
                                .on_hover_text("Continuously pre-cache frames from beginning to final keyframe in the background.");
                            ui.checkbox(&mut comp.ram_cache_enabled, "Enable Frame Caching");
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut comp.cache_compression_enabled, "🗜 Compress Cache")
                                    .on_hover_text("Compress cached frames in memory using high-speed encoding.");
                                if ui.button("⚙ Limits & Modes").on_hover_text("Configure Cache Capacity, Limits & Compression Settings").clicked() {
                                    show_cache_settings_dialog = true;
                                }
                            });

                            ui.add_space(4.0);
                            let max_mb = comp.cache_max_memory_mb;
                            draw_large_ram_graph(
                                ui,
                                comp.cache_size_mb,
                                comp.cache_raw_size_mb,
                                max_mb.max(256.0),
                                comp.cached_frames.len(),
                                ram_cache.max_frames,
                                comp.cache_compression_ratio,
                                &memory_history,
                                &mut comp.ram_cache_purge_requested,
                                &mut show_cache_settings_dialog,
                            );

                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new("Cache to Final KF Now").background_color(Color32::LIGHT_RED)).on_hover_text("(WARNING) Immediately render and cache all frames from start to final keyframe").clicked() {
                                    let fps = comp.settings.fps.max(1) as f32;
                                    let max_kf = get_max_keyframe_time(&comp);
                                    let end_time = if max_kf > 0.0 {
                                        max_kf
                                    } else {
                                        comp.work_area_out.min(comp.settings.duration)
                                    };
                                    let start_frame = 0usize;
                                    let end_frame = (end_time * fps).round() as usize;
                                    for f_idx in start_frame..=end_frame {
                                        if !ram_cache.frames.contains_key(&f_idx) {
                                            let f_time = f_idx as f32 / fps;
                                            draw_composition(&comp, &textures, f_time, render_target.clone());
                                            let image = render_target.texture.get_texture_data();
                                            ram_cache.insert(f_idx, &image, comp.cache_compression_enabled, comp.cache_compression_mode);
                                            comp.cached_frames.insert(f_idx);
                                        }
                                    }
                                }

                                if ui.button("🗑 Purge Cache (Ctrl+Alt+0)").on_hover_text("Flush all cached frames from memory").clicked() {
                                    comp.ram_cache_purge_requested = true;
                                }
                            });
                        }
                        1 => {
                            // ALIGN TAB
                            ui.label(
                                egui::RichText::new("Align & Distribute Layers")
                                    .strong()
                                    .color(egui::Color32::from_gray(200)),
                            );
                            ui.add_space(6.0);

                            ui.horizontal(|ui| {
                                if ui.button("Align Left").clicked() {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            if let Some(p) = l.properties.get_mut("x") {
                                                p.base_value = 100.0;
                                            }
                                        }
                                    }
                                }
                                if ui.button("Center H").clicked() {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            if let Some(p) = l.properties.get_mut("x") {
                                                p.base_value = comp.settings.width as f32 / 2.0;
                                            }
                                        }
                                    }
                                }
                                if ui.button("Align Right").clicked() {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            if let Some(p) = l.properties.get_mut("x") {
                                                p.base_value = comp.settings.width as f32 - 100.0;
                                            }
                                        }
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Align Top").clicked() {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            if let Some(p) = l.properties.get_mut("y") {
                                                p.base_value = 100.0;
                                            }
                                        }
                                    }
                                }
                                if ui.button("Center V").clicked() {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            if let Some(p) = l.properties.get_mut("y") {
                                                p.base_value = comp.settings.height as f32 / 2.0;
                                            }
                                        }
                                    }
                                }
                                if ui.button("Align Bottom").clicked() {
                                    if let Some(idx) = comp.active_layer_index {
                                        if let Some(l) = comp.layers.get_mut(idx) {
                                            if let Some(p) = l.properties.get_mut("y") {
                                                p.base_value = comp.settings.height as f32 - 100.0;
                                            }
                                        }
                                    }
                                }
                            });
                        }
                        2 => {
                            // AUDIO VU METERS
                            ui.label(
                                egui::RichText::new("Audio Levels & Info")
                                    .strong()
                                    .color(egui::Color32::from_gray(200)),
                            );
                            ui.add_space(8.0);

                            // Stereo VU meters
                            ui.horizontal(|ui| {
                                ui.label("L");
                                let level_l = if comp.is_playing { 0.72 } else { 0.0 };
                                let (rect_l, _) = ui.allocate_exact_size(
                                    egui::vec2(160.0, 12.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect_l,
                                    1.0,
                                    egui::Color32::from_rgb(20, 20, 22),
                                );
                                let fill_l = egui::Rect::from_min_size(
                                    rect_l.min,
                                    egui::vec2(rect_l.width() * level_l, rect_l.height()),
                                );
                                ui.painter().rect_filled(
                                    fill_l,
                                    1.0,
                                    egui::Color32::from_rgb(60, 200, 90),
                                );

                                ui.label("-6 dB");
                            });

                            ui.horizontal(|ui| {
                                ui.label("R");
                                let level_r = if comp.is_playing { 0.68 } else { 0.0 };
                                let (rect_r, _) = ui.allocate_exact_size(
                                    egui::vec2(160.0, 12.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect_r,
                                    1.0,
                                    egui::Color32::from_rgb(20, 20, 22),
                                );
                                let fill_r = egui::Rect::from_min_size(
                                    rect_r.min,
                                    egui::vec2(rect_r.width() * level_r, rect_r.height()),
                                );
                                ui.painter().rect_filled(
                                    fill_r,
                                    1.0,
                                    egui::Color32::from_rgb(60, 200, 90),
                                );

                                ui.label("-8 dB");
                            });
                        }
                        3 => {
                            // EFFECTS & PRESETS BROWSER
                            ui.label(
                                egui::RichText::new("Effects & Presets")
                                    .strong()
                                    .color(egui::Color32::from_gray(200)),
                            );
                            ui.add_space(4.0);

                            let categories = [
                                (
                                    "Blur & Sharpen",
                                    &[
                                        "Gaussian Blur",
                                        "Fast Box Blur",
                                        "Directional Blur",
                                        "Sharpen",
                                    ][..],
                                ),
                                (
                                    "Color Correction",
                                    &[
                                        "Brightness & Contrast",
                                        "Hue/Saturation",
                                        "Levels",
                                        "Curves",
                                        "Tint",
                                        "Invert",
                                    ][..],
                                ),
                                (
                                    "Distort",
                                    &["Transform", "Bulge", "Ripple", "Wave Warp"][..],
                                ),
                                (
                                    "Generate",
                                    &["Fill", "Gradient Ramp", "Grid", "Checkerboard"][..],
                                ),
                                (
                                    "Perspective",
                                    &["Drop Shadow", "Bevel Alpha", "3D Extrusion"][..],
                                ),
                                (
                                    "Stylize",
                                    &["Glow", "Posterize", "Threshold", "Find Edges"][..],
                                ),
                                (
                                    "Transition",
                                    &["Linear Wipe", "Radial Wipe", "Cross Dissolve"][..],
                                ),
                            ];

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.collapsing("Plugins", |ui| {
                                    if plugin_registry.effects.is_empty() {
                                        ui.label(
                                            egui::RichText::new("No effect plugins in /plugins")
                                                .size(11.0)
                                                .color(egui::Color32::from_gray(130)),
                                        );
                                    } else {
                                        for eff_plug in &plugin_registry.effects {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new("fx")
                                                        .color(egui::Color32::from_rgb(120, 200, 255)),
                                                );
                                                if ui
                                                    .button(&eff_plug.name)
                                                    .on_hover_text(&eff_plug.description)
                                                    .clicked()
                                                {
                                                    if let Some(idx) = comp.active_layer_index {
                                                        if let Some(l) = comp.layers.get_mut(idx) {
                                                            l.effects.push(LayerEffect::new_plugin(eff_plug));
                                                            export_status = format!("Added effect '{}' to layer.", eff_plug.name);
                                                            history.truncate(history_index + 1);
                                                            history.push(comp.clone());
                                                            history_index = history.len() - 1;
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    }
                                });

                                for (cat, effects) in categories {
                                    ui.collapsing(cat, |ui| {
                                        for eff in effects {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("fx").color(egui::Color32::from_rgb(100, 180, 255)));
                                                if ui.button(*eff).clicked() {
                                                    if let Some(idx) = comp.active_layer_index {
                                                        if let Some(l) = comp.layers.get_mut(idx) {
                                                            let eff_type = match *eff {
                                                                "Fast Box Blur" | "Gaussian Blur" => Some(EffectType::FastBlur),
                                                                "Directional Blur" => Some(EffectType::DirectionalBlur),
                                                                "Brightness & Contrast" => Some(EffectType::BrightnessContrast),
                                                                "Hue/Saturation" => Some(EffectType::HueSaturation),
                                                                "Tint" => Some(EffectType::Tint),
                                                                "Invert" => Some(EffectType::Invert),
                                                                "Fill" => Some(EffectType::Fill),
                                                                "Drop Shadow" => Some(EffectType::DropShadow),
                                                                "Glow" => Some(EffectType::Glow),
                                                                "Vignette" => Some(EffectType::Vignette),
                                                                "Chromatic Aberration" => Some(EffectType::ChromaticAberration),
                                                                "Wave Warp" => Some(EffectType::WaveWarp),
                                                                _ => None,
                                                            };
                                                            if let Some(et) = eff_type {
                                                                l.effects.push(LayerEffect::new(eff.to_string(), et));
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                        }
                        4 => {
                            // CHARACTER / TEXT TAB
                            ui.label(
                                egui::RichText::new("Character / Typography")
                                    .strong()
                                    .color(egui::Color32::from_gray(200)),
                            );
                            ui.add_space(4.0);

                            if let Some(idx) = comp.active_layer_index {
                                if let Some(l) = comp.layers.get_mut(idx) {
                                    if let LayerSource::Text {
                                        text,
                                        font_size,
                                        color,
                                    } = &mut l.source
                                    {
                                        ui.label("Text Content:");
                                        ui.add(egui::TextEdit::multiline(text).desired_rows(3));

                                        ui.horizontal(|ui| {
                                            ui.label("Font Size:");
                                            ui.add(
                                                egui::DragValue::new(font_size)
                                                    .speed(1.0)
                                                    .range(8.0..=300.0),
                                            );
                                        });

                                        ui.horizontal(|ui| {
                                            ui.label("Fill Color:");
                                            let mut egui_color = egui::Color32::from_rgba_premultiplied(
                                                (color[0] * 255.0) as u8,
                                                (color[1] * 255.0) as u8,
                                                (color[2] * 255.0) as u8,
                                                (color[3] * 255.0) as u8,
                                            );
                                            if ui.color_edit_button_srgba(&mut egui_color).changed()
                                            {
                                                color[0] = egui_color.r() as f32 / 255.0;
                                                color[1] = egui_color.g() as f32 / 255.0;
                                                color[2] = egui_color.b() as f32 / 255.0;
                                                color[3] = egui_color.a() as f32 / 255.0;
                                            }
                                        });
                                    } else {
                                        ui.label(
                                            egui::RichText::new(
                                                "Active layer is not a text layer.",
                                            )
                                            .size(11.0)
                                            .color(egui::Color32::from_gray(120)),
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                });

            // ==========================================
            // BOTTOM STATUS BAR (JetBrains Style Memory Widget & Status)
            // ==========================================
            egui::TopBottomPanel::bottom("app_status_bar")
                .exact_height(22.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        // Playback Status Indicator
                        let (status_color, status_text) = if comp.is_playing {
                            if comp.is_ram_previewing {
                                (egui::Color32::from_rgb(45, 215, 95), "⚡ RAM Previewing")
                            } else {
                                (egui::Color32::from_rgb(70, 175, 255), "▶ Playing")
                            }
                        } else {
                            (egui::Color32::from_gray(140), "⏸ Paused")
                        };
                        ui.label(egui::RichText::new(status_text).color(status_color).size(11.0));

                        ui.separator();

                        // Current Timecode
                        let tc = format_timecode(comp.current_time, comp.settings.fps);
                        ui.label(
                            egui::RichText::new(format!("⏱ {}", tc))
                                .monospace()
                                .size(11.0)
                                .color(egui::Color32::from_rgb(80, 225, 200)),
                        );

                        ui.separator();

                        // Active Tool
                        let tool_name = match comp.active_tool {
                            0 => "Selection (V)",
                            1 => "Hand (H)",
                            2 => "Zoom (Z)",
                            3 => "Rotation (W)",
                            4 => "Camera (C)",
                            5 => "Pan Behind (Y)",
                            6 => "Shape (Q)",
                            7 => "Pen (G)",
                            8 => "Type (T)",
                            _ => "Tool",
                        };
                        ui.label(egui::RichText::new(format!("🛠 {}", tool_name)).size(11.0).color(egui::Color32::from_gray(160)));

                        ui.separator();

                        // Playback FPS
                        let target_fps = comp.settings.fps.max(1) as f32;
                        let fps_txt = format!("{:.1} / {:.0} FPS", comp.playback_fps, target_fps);
                        ui.label(egui::RichText::new(fps_txt).size(11.0).color(egui::Color32::from_gray(150)));

                        // Right side: JetBrains RAM Graph & Usage Meter
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let max_mb = comp.cache_max_memory_mb;
                            draw_jetbrains_ram_meter(
                                ui,
                                comp.cache_size_mb,
                                max_mb.max(256.0),
                                comp.cached_frames.len(),
                                ram_cache.max_frames,
                                comp.cache_compression_ratio,
                                &memory_history,
                                &mut comp.ram_cache_purge_requested,
                                &mut show_cache_settings_dialog,
                            );

                            if ui.button(egui::RichText::new("⚙ RAM:").size(11.0).color(egui::Color32::from_gray(140))).on_hover_text("Open Cache & Compression Settings").clicked() {
                                show_cache_settings_dialog = true;
                            }
                        });
                    });
                });

            // ==========================================
            // BOTTOM PANEL: AFTER EFFECTS TIMELINE
            // ==========================================
            let screen_height = ctx.screen_rect().height();
            egui::TopBottomPanel::bottom("timeline_panel_root")
                .resizable(true)
                .default_height(screen_height * 0.42)
                .height_range((screen_height * 0.25)..=(screen_height * 0.8))
                .show(ctx, |ui| {
                    draw_pro_ae_timeline(ui, &mut comp, &mut selected_keyframe);
                });

            // ==========================================
            // CENTER PANEL: COMPOSITION VIEWER
            // ==========================================
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::from_rgb(16, 16, 18))
                        .stroke(egui::Stroke::NONE),
                )
                .show(ctx, |ui| {
                    // Composition Header Tab
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🎬 Comp 1")
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "({}x{})",
                                comp.settings.width, comp.settings.height
                            ))
                            .size(11.0)
                            .color(egui::Color32::from_gray(130)),
                        );

                        ui.separator();

                        // Viewport Mode Selector Dropdown
                        egui::ComboBox::from_id_salt("viewport_mode_select")
                            .selected_text(match comp.viewport_mode {
                                ViewportMode::ActiveCamera => "🎥 Active Camera",
                                ViewportMode::CustomView => "🌐 Custom View 1",
                                ViewportMode::Top => "⬆ Top (XZ)",
                                ViewportMode::Front => "🔲 Front (XY)",
                                ViewportMode::Right => "➡ Right (YZ)",
                                ViewportMode::Left => "⬅ Left (-YZ)",
                                ViewportMode::Bottom => "⬇ Bottom (-XZ)",
                                ViewportMode::Back => "🔙 Back (-XY)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::ActiveCamera, "🎥 Active Camera");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::CustomView, "🌐 Custom View 1");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::Top, "⬆ Top (XZ)");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::Front, "🔲 Front (XY)");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::Right, "➡ Right (YZ)");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::Left, "⬅ Left (-YZ)");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::Bottom, "⬇ Bottom (-XZ)");
                                ui.selectable_value(&mut comp.viewport_mode, ViewportMode::Back, "🔙 Back (-XY)");
                            });

                        if comp.viewport_mode == ViewportMode::CustomView {
                            ui.separator();
                            ui.add(egui::DragValue::new(&mut comp.custom_orbit_yaw).speed(0.5).prefix("Yaw: ").suffix("°"));
                            ui.add(egui::DragValue::new(&mut comp.custom_orbit_pitch).speed(0.5).prefix("Pitch: ").suffix("°"));
                            ui.add(egui::DragValue::new(&mut comp.custom_orbit_roll).speed(0.5).prefix("Roll: ").suffix("°"));
                            ui.add(egui::DragValue::new(&mut comp.custom_orbit_distance).speed(10.0).range(100.0..=30000.0).prefix("Dist: "));
                            if ui.button("↺ Reset").on_hover_text("Reset Custom Orbit View (Yaw, Pitch, Roll, Distance)").clicked() {
                                comp.custom_orbit_yaw = -35.0;
                                comp.custom_orbit_pitch = 25.0;
                                comp.custom_orbit_roll = 0.0;
                                comp.custom_orbit_distance = 2200.0;
                                comp.custom_orbit_target = [comp.settings.width as f32 / 2.0, comp.settings.height as f32 / 2.0, 0.0];
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut comp.show_guides, "⌗ Guides");
                            ui.checkbox(&mut comp.show_grid, "⊞ Grid");
                            ui.checkbox(&mut comp.show_checkerboard, "▦ Checkerboard");

                            if comp.is_ram_previewing {
                                ui.label(
                                    egui::RichText::new("⚡ RAM PREVIEW")
                                        .strong()
                                        .color(egui::Color32::from_rgb(45, 215, 95)),
                                );
                            } else if is_cur_frame_cached {
                                ui.label(
                                    egui::RichText::new("⚡ CACHED")
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(70, 190, 100)),
                                );
                            }
                        });
                    });

                    ui.separator();
                    viewport_rect = ui.available_rect_before_wrap();
                });

            // Shortcuts dialog
            if show_shortcuts_dialog {
                egui::Window::new("BeforeFX Keyboard Shortcuts")
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(egui::RichText::new("Tools Palette:").strong());
                        ui.label("V: Selection Tool (Move / Scale / Rotate)");
                        ui.label("H: Hand Tool (Pan Viewport)");
                        ui.label("Z: Zoom Tool (Alt: Zoom Out)");
                        ui.label("W: Rotation Tool (2D & 3D)");
                        ui.label("C: Camera Tool (3D Orbit / Pan / Dolly)");
                        ui.label("Y: Pan Behind / Anchor Point Tool");
                        ui.label("Q: Shape / Rectangle Tool");
                        ui.label("G: Pen Tool (Polygon Paths)");
                        ui.label("T: Type Tool (Create Text)");
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Playback & Navigation:").strong());
                        ui.label("Space: Play / Pause");
                        ui.label("NumPad 0 / Shift+Space: RAM Preview (Cached Loop)");
                        ui.label("Ctrl+Alt+NumPad 0: Purge All RAM Cache");
                        ui.label("Home / End: Jump to Start / End of Work Area");
                        ui.label("B / N: Set Work Area In / Out point");
                        ui.label("J / K: Jump to Previous / Next Keyframe");
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Layer & Keyframe Editing:").strong());
                        ui.label("Ctrl+D: Duplicate Selected Layer");
                        ui.label("Del / Backspace: Delete Keyframe or Layer");
                        ui.label("F9: Easy Ease keyframe");
                        ui.label("Shift+F9: Ease In keyframe");
                        ui.label("Ctrl+Shift+F9: Ease Out keyframe");
                        ui.add_space(6.0);
                        if ui.button("Close").clicked() {
                            show_shortcuts_dialog = false;
                        }
                    });
            }

            // Plugin Manager dialog
            if show_plugin_manager {
                egui::Window::new("⚙ Plugin Manager")
                    .open(&mut show_plugin_manager)
                    .default_size(egui::vec2(580.0, 460.0))
                    .resizable(true)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("🔄 Reload All Plugins").clicked() {
                                plugin_registry.reload();
                                export_status = format!(
                                    "Reloaded: {} effects, {} functional plugins",
                                    plugin_registry.effects.len(),
                                    plugin_registry.functionals.len()
                                );
                            }
                            if ui.button("📁 Open /plugins Folder").clicked() {
                                let _ = std::process::Command::new("explorer").arg("plugins").spawn();
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} Effects | {} Functional",
                                        plugin_registry.effects.len(),
                                        plugin_registry.functionals.len()
                                    ))
                                    .color(egui::Color32::from_rgb(140, 200, 255)),
                                );
                            });
                        });
                        ui.separator();

                        if !plugin_registry.load_errors.is_empty() {
                            ui.group(|ui| {
                                ui.label(
                                    egui::RichText::new("⚠️ Plugin Parse Errors:")
                                        .color(egui::Color32::from_rgb(255, 120, 120))
                                        .strong(),
                                );
                                for (file, err) in &plugin_registry.load_errors {
                                    ui.label(format!("• {}: {}", file, err));
                                }
                            });
                            ui.separator();
                        }

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("⚡ Functional Plugins (.spec functional)")
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                            );
                            if plugin_registry.functionals.is_empty() {
                                ui.label(
                                    egui::RichText::new("No functional plugins loaded.")
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(120)),
                                );
                            } else {
                                for func in &plugin_registry.functionals {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&func.name).strong());
                                            ui.label(
                                                egui::RichText::new(format!("[{}]", &func.category))
                                                    .color(egui::Color32::from_gray(140)),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("⚡ Run Action").clicked() {
                                                        match execute_functional_plugin(&mut comp, func, None) {
                                                            Ok(msg) => {
                                                                export_status = format!("Plugin: {}", msg);
                                                                history.truncate(history_index + 1);
                                                                history.push(comp.clone());
                                                                history_index = history.len() - 1;
                                                            }
                                                            Err(err) => {
                                                                export_status = format!("Error: {}", err);
                                                            }
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                        if !func.description.is_empty() {
                                            ui.label(
                                                egui::RichText::new(&func.description)
                                                    .size(11.0)
                                                    .color(egui::Color32::from_gray(180)),
                                            );
                                        }
                                        if !func.action.is_empty() {
                                            ui.label(
                                                egui::RichText::new(format!("Action: {}", func.action))
                                                    .size(10.5)
                                                    .color(egui::Color32::from_gray(140)),
                                            );
                                        }
                                        ui.label(
                                            egui::RichText::new(&func.file_path)
                                                .size(10.0)
                                                .color(egui::Color32::from_gray(120)),
                                        );
                                    });
                                    ui.add_space(2.0);
                                }
                            }

                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("🎨 Effect Plugins (.spec effect)")
                                    .strong()
                                    .color(egui::Color32::from_rgb(120, 200, 255)),
                            );
                            if plugin_registry.effects.is_empty() {
                                ui.label(
                                    egui::RichText::new("No effect plugins loaded.")
                                        .size(11.0)
                                        .color(egui::Color32::from_gray(120)),
                                );
                            } else {
                                for eff in &plugin_registry.effects {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&eff.name).strong());
                                            ui.label(
                                                egui::RichText::new(format!("[{}]", &eff.category))
                                                    .color(egui::Color32::from_gray(140)),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui.button("+ Apply to Active Layer").clicked() {
                                                        if let Some(idx) = comp.active_layer_index {
                                                            if let Some(l) = comp.layers.get_mut(idx) {
                                                                l.effects.push(LayerEffect::new_plugin(eff));
                                                                export_status = format!("Applied effect '{}' to layer.", eff.name);
                                                                history.truncate(history_index + 1);
                                                                history.push(comp.clone());
                                                                history_index = history.len() - 1;
                                                            }
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                        if !eff.description.is_empty() {
                                            ui.label(
                                                egui::RichText::new(&eff.description)
                                                    .size(11.0)
                                                    .color(egui::Color32::from_gray(180)),
                                            );
                                        }
                                        let slider_names: Vec<String> =
                                            eff.sliders.iter().map(|s| s.name.clone()).collect();
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Sliders ({}): {}",
                                                eff.sliders.len(),
                                                slider_names.join(", ")
                                            ))
                                            .size(10.5)
                                            .color(egui::Color32::from_gray(140)),
                                        );
                                        ui.label(
                                            egui::RichText::new(&eff.file_path)
                                                .size(10.0)
                                                .color(egui::Color32::from_gray(120)),
                                        );
                                    });
                                    ui.add_space(2.0);
                                }
                            }
                        });
                    });
            }

            // Cache & Compression Settings dialog
            if show_cache_settings_dialog {
                egui::Window::new("⚙ RAM Cache & Compression Settings")
                    .collapsible(false)
                    .resizable(true)
                    .default_size(egui::vec2(480.0, 440.0))
                    .show(ctx, |ui| {
                        ui.heading("RAM Preview Cache & Compression Engine");
                        ui.label(
                            egui::RichText::new("Configure memory limits, frame capacity, and real-time lossless compression.")
                                .color(egui::Color32::from_gray(160))
                                .size(11.0),
                        );
                        ui.add_space(8.0);

                        ui.group(|ui| {
                            ui.label(egui::RichText::new("🗜 Frame Compression Engine").strong().color(egui::Color32::from_rgb(100, 195, 255)));
                            ui.add_space(4.0);
                            ui.checkbox(&mut comp.cache_compression_enabled, "Enable Cache Compression")
                                .on_hover_text("Compress cached frames in RAM to dramatically reduce memory footprint and enable significantly longer real-time playback buffers.");

                            ui.add_space(3.0);
                            ui.horizontal(|ui| {
                                ui.label("Compression Mode:");
                                egui::ComboBox::from_id_salt("cache_compression_mode_dialog_select")
                                    .selected_text(match comp.cache_compression_mode {
                                        CacheCompressionMode::FastPlanarRle => "🗜 Fast Planar BP-RLE (High Compression)",
                                        CacheCompressionMode::UltraFastDirect => "⚡ Ultra-Fast 32-bit Run Pack",
                                        CacheCompressionMode::Uncompressed => "📦 Uncompressed RAW RGBA",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut comp.cache_compression_mode,
                                            CacheCompressionMode::FastPlanarRle,
                                            "🗜 Fast Planar BP-RLE (High Compression, Recommended)",
                                        );
                                        ui.selectable_value(
                                            &mut comp.cache_compression_mode,
                                            CacheCompressionMode::UltraFastDirect,
                                            "⚡ Ultra-Fast 32-bit Run Pack (Sub-millisecond)",
                                        );
                                        ui.selectable_value(
                                            &mut comp.cache_compression_mode,
                                            CacheCompressionMode::Uncompressed,
                                            "📦 Uncompressed RAW RGBA (0 CPU Overhead)",
                                        );
                                    });
                            });
                        });

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.label(egui::RichText::new("📦 Cache Capacity & Memory Allocation").strong().color(egui::Color32::from_rgb(100, 195, 255)));
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                ui.label("Max Cached Frames:");
                                ui.add(egui::DragValue::new(&mut comp.cache_max_frames).range(50..=50000).speed(10));
                                ui.label(egui::RichText::new(format!("(~{:.1}s @ {}fps)", comp.cache_max_frames as f32 / comp.settings.fps.max(1) as f32, comp.settings.fps)).color(egui::Color32::from_gray(140)));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Quick Frame Presets:");
                                if ui.button("500").clicked() { comp.cache_max_frames = 500; }
                                if ui.button("1,000").clicked() { comp.cache_max_frames = 1000; }
                                if ui.button("2,000").clicked() { comp.cache_max_frames = 2000; }
                                if ui.button("5,000").clicked() { comp.cache_max_frames = 5000; }
                                if ui.button("10,000").clicked() { comp.cache_max_frames = 10000; }
                                let fps = comp.settings.fps.max(1) as f32;
                                let dur_frames = (comp.settings.duration * fps).ceil() as usize;
                                if ui.button("Match Duration").on_hover_text(format!("Set to match duration ({} frames)", dur_frames)).clicked() {
                                    comp.cache_max_frames = dur_frames.max(100);
                                }
                            });

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Max RAM Cache Limit:");
                                ui.add(egui::DragValue::new(&mut comp.cache_max_memory_mb).range(128.0..=65536.0).speed(64.0).suffix(" MB"));
                                ui.label(egui::RichText::new(format!("({:.1} GB)", comp.cache_max_memory_mb / 1024.0)).color(egui::Color32::from_gray(140)));
                            });

                            ui.horizontal(|ui| {
                                ui.label("Quick RAM Presets:");
                                if ui.button("1 GB").clicked() { comp.cache_max_memory_mb = 1024.0; }
                                if ui.button("2 GB").clicked() { comp.cache_max_memory_mb = 2048.0; }
                                if ui.button("4 GB").clicked() { comp.cache_max_memory_mb = 4096.0; }
                                if ui.button("8 GB").clicked() { comp.cache_max_memory_mb = 8192.0; }
                                if ui.button("16 GB").clicked() { comp.cache_max_memory_mb = 16384.0; }
                                if ui.button("32 GB").clicked() { comp.cache_max_memory_mb = 32768.0; }
                            });
                        });

                        ui.add_space(6.0);

                        ui.group(|ui| {
                            ui.label(egui::RichText::new("📊 Real-time Cache Diagnostics").strong().color(egui::Color32::from_rgb(100, 195, 255)));
                            ui.add_space(4.0);
                            let saved_mb = (comp.cache_raw_size_mb - comp.cache_size_mb).max(0.0);
                            ui.label(format!("• Cached Frames in Memory: {} / {} max frames", comp.cached_frames.len(), comp.cache_max_frames));
                            ui.label(format!("• Compressed RAM Footprint: {:.1} MB / {:.0} MB allocated ({:.1}%)", comp.cache_size_mb, comp.cache_max_memory_mb, (comp.cache_size_mb / comp.cache_max_memory_mb.max(1.0)) * 100.0));
                            ui.label(format!("• Uncompressed (RAW RGBA) Size: {:.1} MB", comp.cache_raw_size_mb));
                            ui.label(format!("• Memory Saved by Compression: {:.1} MB (Effective Ratio: {:.1}x)", saved_mb, comp.cache_compression_ratio));
                        });

                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("🗑 Purge RAM Cache").color(Color32::from_rgb(255, 100, 100))).on_hover_text("Flush all cached frames from memory").clicked() {
                                comp.ram_cache_purge_requested = true;
                            }
                            if ui.button("⚡ Pre-Cache Entire Comp").clicked() {
                                let fps = comp.settings.fps.max(1) as f32;
                                let start_frame = 0usize;
                                let end_frame = (comp.settings.duration * fps).round() as usize;
                                for f_idx in start_frame..=end_frame {
                                    if !ram_cache.frames.contains_key(&f_idx) {
                                        let f_time = f_idx as f32 / fps;
                                        draw_composition(&comp, &textures, f_time, render_target.clone());
                                        let image = render_target.texture.get_texture_data();
                                        ram_cache.insert(f_idx, &image, comp.cache_compression_enabled, comp.cache_compression_mode);
                                        comp.cached_frames.insert(f_idx);
                                    }
                                }
                            }
                            if ui.button("⚡ Pre-Cache Work Area").clicked() {
                                let fps = comp.settings.fps.max(1) as f32;
                                let start_frame = (comp.work_area_in * fps).round() as usize;
                                let end_frame = (comp.work_area_out.min(comp.settings.duration) * fps).round() as usize;
                                for f_idx in start_frame..=end_frame {
                                    if !ram_cache.frames.contains_key(&f_idx) {
                                        let f_time = f_idx as f32 / fps;
                                        draw_composition(&comp, &textures, f_time, render_target.clone());
                                        let image = render_target.texture.get_texture_data();
                                        ram_cache.insert(f_idx, &image, comp.cache_compression_enabled, comp.cache_compression_mode);
                                        comp.cached_frames.insert(f_idx);
                                    }
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Close").clicked() {
                                    show_cache_settings_dialog = false;
                                }
                            });
                        });
                    });
            }
        });

        if let Some(path) = pending_export.take() {
            export_status = export_video(&comp, &textures, render_target.clone(), &path);
        }

        // --- 3. FINAL COMPOSITE VIEWPORT RENDERING ---
        egui_macroquad::draw();

        set_default_camera();

        if viewport_rect.width() > 10.0 && viewport_rect.height() > 10.0 {
            let target_aspect = comp.settings.width as f32 / comp.settings.height as f32;
            let viewport_aspect = viewport_rect.width() / viewport_rect.height();

            let (draw_w, draw_h) = if viewport_aspect > target_aspect {
                (
                    viewport_rect.height() * target_aspect,
                    viewport_rect.height(),
                )
            } else {
                (viewport_rect.width(), viewport_rect.width() / target_aspect)
            };

            let ppp = comp.settings.ui_scale;
            let draw_x = viewport_rect.min.x + (viewport_rect.width() - draw_w) / 2.0;
            let draw_y = viewport_rect.min.y + (viewport_rect.height() - draw_h) / 2.0;

            let screen_x = draw_x * ppp;
            let screen_y = draw_y * ppp;
            let screen_w = draw_w * ppp;
            let screen_h = draw_h * ppp;

            // Checkerboard pattern behind transparent elements
            if comp.show_checkerboard {
                let cell_size = 16.0;
                let cols = (screen_w / cell_size).ceil() as i32;
                let rows = (screen_h / cell_size).ceil() as i32;
                for c in 0..cols {
                    for r in 0..rows {
                        let col_color = if (c + r) % 2 == 0 {
                            Color::from_rgba(30, 30, 34, 255)
                        } else {
                            Color::from_rgba(40, 40, 45, 255)
                        };
                        draw_rectangle(
                            screen_x + c as f32 * cell_size,
                            screen_y + r as f32 * cell_size,
                            cell_size,
                            cell_size,
                            col_color,
                        );
                    }
                }
            }

            // Draw composition render (instant cache retrieval if cached)
            let tex_to_draw = if is_cur_frame_cached {
                ram_cache.get(cur_frame_idx).unwrap_or(&render_target.texture)
            } else {
                &render_target.texture
            };

            draw_texture_ex(
                tex_to_draw,
                screen_x,
                screen_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(screen_w, screen_h)),
                    flip_y: true,
                    ..Default::default()
                },
            );

            // Composition Border
            draw_rectangle_lines(
                screen_x,
                screen_y,
                screen_w,
                screen_h,
                1.5,
                Color::from_rgba(55, 55, 62, 255),
            );

            // Real-time Playback / RAM Preview Status HUD Badge
            if comp.is_playing {
                let badge_bg = if comp.is_ram_previewing {
                    Color::from_rgba(25, 85, 40, 220)
                } else if is_cur_frame_cached {
                    Color::from_rgba(30, 70, 40, 200)
                } else {
                    Color::from_rgba(45, 45, 50, 190)
                };
                let badge_text = if comp.is_ram_previewing {
                    format!("⚡ RAM PREVIEW • {:.1} FPS", comp.playback_fps)
                } else if is_cur_frame_cached {
                    format!("⚡ CACHED • {:.1} FPS", comp.playback_fps)
                } else {
                    format!("▶ PLAYING • {:.1} FPS", comp.playback_fps)
                };
                draw_rectangle(screen_x + 12.0, screen_y + screen_h - 32.0, 195.0, 20.0, badge_bg);
                draw_rectangle_lines(screen_x + 12.0, screen_y + screen_h - 32.0, 195.0, 20.0, 1.0, Color::from_rgba(60, 220, 110, 180));
                draw_text(&badge_text, screen_x + 18.0, screen_y + screen_h - 18.0, 13.0, Color::from_rgba(235, 255, 235, 255));
            }

            // Title & Action Safe Guides
            if comp.show_guides {
                // Action safe (90%)
                let as_w = screen_w * 0.9;
                let as_h = screen_h * 0.9;
                let as_x = screen_x + (screen_w - as_w) / 2.0;
                let as_y = screen_y + (screen_h - as_h) / 2.0;
                draw_rectangle_lines(
                    as_x,
                    as_y,
                    as_w,
                    as_h,
                    1.0,
                    Color::from_rgba(60, 140, 200, 100),
                );

                // Title safe (80%)
                let ts_w = screen_w * 0.8;
                let ts_h = screen_h * 0.8;
                let ts_x = screen_x + (screen_w - ts_w) / 2.0;
                let ts_y = screen_y + (screen_h - ts_h) / 2.0;
                draw_rectangle_lines(
                    ts_x,
                    ts_y,
                    ts_w,
                    ts_h,
                    1.0,
                    Color::from_rgba(60, 140, 200, 100),
                );

                // Center crosshair
                let cx = screen_x + screen_w / 2.0;
                let cy = screen_y + screen_h / 2.0;
                draw_line(
                    cx - 10.0,
                    cy,
                    cx + 10.0,
                    cy,
                    1.0,
                    Color::from_rgba(60, 140, 200, 120),
                );
                draw_line(
                    cx,
                    cy - 10.0,
                    cx,
                    cy + 10.0,
                    1.0,
                    Color::from_rgba(60, 140, 200, 120),
                );
            }

            // Grid Overlay
            if comp.show_grid {
                for i in 1..10 {
                    let gx = screen_x + (screen_w / 10.0) * i as f32;
                    let gy = screen_y + (screen_h / 10.0) * i as f32;
                    draw_line(
                        gx,
                        screen_y,
                        gx,
                        screen_y + screen_h,
                        0.5,
                        Color::from_rgba(80, 80, 90, 80),
                    );
                    draw_line(
                        screen_x,
                        gy,
                        screen_x + screen_w,
                        gy,
                        0.5,
                        Color::from_rgba(80, 80, 90, 80),
                    );
                }
            }

            let comp_w = comp.settings.width as f32;
            let comp_h = comp.settings.height as f32;
            let to_screen = |p: Vec2| -> Vec2 {
                vec2(screen_x + (p.x / comp_w) * screen_w, screen_y + (p.y / comp_h) * screen_h)
            };
            let to_comp = |p: Vec2| -> Vec2 {
                vec2((p.x - screen_x) / screen_w * comp_w, (p.y - screen_y) / screen_h * comp_h)
            };

            let (m_raw_x, m_raw_y) = mouse_position();
            let mouse_pos = vec2(m_raw_x, m_raw_y);
            let in_viewport = mouse_pos.x >= screen_x && mouse_pos.x <= screen_x + screen_w
                && mouse_pos.y >= screen_y && mouse_pos.y <= screen_y + screen_h;
            let comp_mouse = to_comp(mouse_pos);

            let mouse_down = is_mouse_button_down(MouseButton::Left);
            let mouse_pressed = is_mouse_button_pressed(MouseButton::Left);
            let mouse_released = is_mouse_button_released(MouseButton::Left);
            let mid_down = is_mouse_button_down(MouseButton::Middle);
            let mid_pressed = is_mouse_button_pressed(MouseButton::Middle);
            let right_down = is_mouse_button_down(MouseButton::Right);
            let right_pressed = is_mouse_button_pressed(MouseButton::Right);
            let alt_down = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
            let shift_down = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            let space_down = is_key_down(KeyCode::Space);
            let mouse_wheel = mouse_wheel().1;

            if mouse_wheel.abs() > 0.001 && in_viewport {
                if comp.viewport_mode == ViewportMode::CustomView {
                    comp.custom_orbit_distance = (comp.custom_orbit_distance * (1.0 - mouse_wheel * 0.08)).clamp(100.0, 30000.0);
                } else if comp.viewport_mode == ViewportMode::ActiveCamera {
                    if let Some(act) = comp.active_layer_index {
                        if act < comp.layers.len() && comp.layers[act].source == LayerSource::Camera {
                            let cur_zoom = comp.layers[act].properties.get("zoom").map_or(1500.0, |p| p.get_value_at(comp.current_time));
                            let new_zoom = (cur_zoom * (1.0 + mouse_wheel * 0.05)).clamp(100.0, 20000.0);
                            update_layer_property_val(&mut comp.layers[act], "zoom", new_zoom, comp.current_time);
                        }
                    }
                }
            }

            let viewport_cam = get_viewport_camera(&comp, comp.current_time);
            let mut hovered_handle = GizmoHandle::None;

            // Hover Outline on non-active layer under cursor
            let hovered_layer_idx = if in_viewport && gizmo_drag == GizmoHandle::None {
                hit_test_layers(&comp, &textures, comp_mouse, comp.current_time)
            } else {
                None
            };

            if let Some(h_idx) = hovered_layer_idx {
                if comp.active_layer_index != Some(h_idx) && h_idx < comp.layers.len() {
                    let h_layer = &comp.layers[h_idx];
                    if !h_layer.d3 {
                        let (_, tl, tr, br, bl) = get_layer_2d_bounds(&comp, h_idx, &textures, comp.current_time);
                        let s_tl = to_screen(tl);
                        let s_tr = to_screen(tr);
                        let s_br = to_screen(br);
                        let s_bl = to_screen(bl);
                        let h_col = Color::from_rgba(100, 200, 255, 120);
                        draw_line(s_tl.x, s_tl.y, s_tr.x, s_tr.y, 1.0, h_col);
                        draw_line(s_tr.x, s_tr.y, s_br.x, s_br.y, 1.0, h_col);
                        draw_line(s_br.x, s_br.y, s_bl.x, s_bl.y, 1.0, h_col);
                        draw_line(s_bl.x, s_bl.y, s_tl.x, s_tl.y, 1.0, h_col);
                        draw_text(&h_layer.name, s_tl.x + 4.0, s_tl.y - 4.0, 11.0, Color::from_rgba(180, 230, 255, 200));
                    }
                }
            }

            // Active layer transform bounding box & interactive gizmos in Viewport
            if let Some(active_idx) = comp.active_layer_index {
                if active_idx < comp.layers.len() {
                    let layer = &comp.layers[active_idx];
                    let is_3d_layer = layer.d3 || layer.source == LayerSource::Camera;
                    let time = comp.current_time;

                    if is_3d_layer {
                        // 3D Layer Bounding Box / Perspective Quad
                        let layer_corners = match &layer.source {
                            LayerSource::Solid { .. } => vec![
                                transform_local_to_world(&comp, active_idx, vec3(0.0, 0.0, 0.0), time),
                                transform_local_to_world(&comp, active_idx, vec3(200.0, 0.0, 0.0), time),
                                transform_local_to_world(&comp, active_idx, vec3(200.0, 200.0, 0.0), time),
                                transform_local_to_world(&comp, active_idx, vec3(0.0, 200.0, 0.0), time),
                            ],
                            LayerSource::Image { path } => {
                                let (tw, th) = textures.get(path).map_or((200.0, 200.0), |tex| (tex.width(), tex.height()));
                                vec![
                                    transform_local_to_world(&comp, active_idx, vec3(0.0, 0.0, 0.0), time),
                                    transform_local_to_world(&comp, active_idx, vec3(tw, 0.0, 0.0), time),
                                    transform_local_to_world(&comp, active_idx, vec3(tw, th, 0.0), time),
                                    transform_local_to_world(&comp, active_idx, vec3(0.0, th, 0.0), time),
                                ]
                            }
                            _ => vec![
                                transform_local_to_world(&comp, active_idx, vec3(-100.0, -100.0, 0.0), time),
                                transform_local_to_world(&comp, active_idx, vec3(100.0, -100.0, 0.0), time),
                                transform_local_to_world(&comp, active_idx, vec3(100.0, 100.0, 0.0), time),
                                transform_local_to_world(&comp, active_idx, vec3(-100.0, 100.0, 0.0), time),
                            ],
                        };

                        let proj_corners: Vec<_> = layer_corners.iter()
                            .map(|&c| project_3d_point(c, &viewport_cam, comp_w, comp_h))
                            .collect();

                        if proj_corners.iter().all(|p| p.visible) {
                            let s_corners: Vec<_> = proj_corners.iter().map(|p| to_screen(p.screen)).collect();
                            for i in 0..4 {
                                draw_line(
                                    s_corners[i].x, s_corners[i].y,
                                    s_corners[(i + 1) % 4].x, s_corners[(i + 1) % 4].y,
                                    1.2, Color::from_rgba(70, 160, 255, 180),
                                );
                                draw_rectangle(
                                    s_corners[i].x - 3.5, s_corners[i].y - 3.5,
                                    7.0, 7.0, Color::from_rgba(255, 255, 255, 220),
                                );
                            }
                        }

                        // 3D Anchor / Origin Position
                        let ax = layer.properties.get("anchorX").map_or(0.0, |p| p.get_value_at(time));
                        let ay = layer.properties.get("anchorY").map_or(0.0, |p| p.get_value_at(time));
                        let az = layer.properties.get("anchorZ").map_or(0.0, |p| p.get_value_at(time));
                        let origin_world = transform_local_to_world(&comp, active_idx, vec3(ax, ay, az), time);
                        let p_origin = project_3d_point(origin_world, &viewport_cam, comp_w, comp_h);

                        if p_origin.visible {
                            let origin_s = to_screen(p_origin.screen);
                            let gizmo_len = 110.0;

                            // Axis Tips in World Space
                            let tip_x_world = origin_world + vec3(gizmo_len, 0.0, 0.0);
                            let tip_y_world = origin_world + vec3(0.0, gizmo_len, 0.0);
                            let tip_z_world = origin_world + vec3(0.0, 0.0, gizmo_len);

                            let p_tip_x = project_3d_point(tip_x_world, &viewport_cam, comp_w, comp_h);
                            let p_tip_y = project_3d_point(tip_y_world, &viewport_cam, comp_w, comp_h);
                            let p_tip_z = project_3d_point(tip_z_world, &viewport_cam, comp_w, comp_h);

                            let tip_s_x = to_screen(p_tip_x.screen);
                            let tip_s_y = to_screen(p_tip_y.screen);
                            let tip_s_z = to_screen(p_tip_z.screen);

                            // 3D Rotation Rings
                            let radius = 80.0;
                            let mut ring_x_pts = vec![];
                            let mut ring_y_pts = vec![];
                            let mut ring_z_pts = vec![];

                            for step in 0..=36 {
                                let a = (step as f32 / 36.0) * std::f32::consts::TAU;
                                let pt_x = origin_world + vec3(0.0, radius * a.cos(), radius * a.sin());
                                let pt_y = origin_world + vec3(radius * a.cos(), 0.0, radius * a.sin());
                                let pt_z = origin_world + vec3(radius * a.cos(), radius * a.sin(), 0.0);

                                let pr_x = project_3d_point(pt_x, &viewport_cam, comp_w, comp_h);
                                let pr_y = project_3d_point(pt_y, &viewport_cam, comp_w, comp_h);
                                let pr_z = project_3d_point(pt_z, &viewport_cam, comp_w, comp_h);

                                if pr_x.visible { ring_x_pts.push(to_screen(pr_x.screen)); }
                                if pr_y.visible { ring_y_pts.push(to_screen(pr_y.screen)); }
                                if pr_z.visible { ring_z_pts.push(to_screen(pr_z.screen)); }
                            }

                            // Hit testing 3D Handles
                            if in_viewport {
                                if (mouse_pos - origin_s).length() < 12.0 {
                                    hovered_handle = GizmoHandle::CenterAnchor;
                                } else if dist_to_segment_2d(mouse_pos, origin_s, tip_s_x) < 9.0 || (mouse_pos - tip_s_x).length() < 12.0 {
                                    hovered_handle = GizmoHandle::TranslateX;
                                } else if dist_to_segment_2d(mouse_pos, origin_s, tip_s_y) < 9.0 || (mouse_pos - tip_s_y).length() < 12.0 {
                                    hovered_handle = GizmoHandle::TranslateY;
                                } else if dist_to_segment_2d(mouse_pos, origin_s, tip_s_z) < 9.0 || (mouse_pos - tip_s_z).length() < 12.0 {
                                    hovered_handle = GizmoHandle::TranslateZ;
                                } else if comp.active_tool == 3 {
                                    let mut near_rx = false;
                                    for i in 0..ring_x_pts.len().saturating_sub(1) {
                                        if dist_to_segment_2d(mouse_pos, ring_x_pts[i], ring_x_pts[i + 1]) < 8.0 {
                                            near_rx = true; break;
                                        }
                                    }
                                    let mut near_ry = false;
                                    for i in 0..ring_y_pts.len().saturating_sub(1) {
                                        if dist_to_segment_2d(mouse_pos, ring_y_pts[i], ring_y_pts[i + 1]) < 8.0 {
                                            near_ry = true; break;
                                        }
                                    }
                                    let mut near_rz = false;
                                    for i in 0..ring_z_pts.len().saturating_sub(1) {
                                        if dist_to_segment_2d(mouse_pos, ring_z_pts[i], ring_z_pts[i + 1]) < 8.0 {
                                            near_rz = true; break;
                                        }
                                    }
                                    if near_rx { hovered_handle = GizmoHandle::RotateX; }
                                    else if near_ry { hovered_handle = GizmoHandle::RotateY; }
                                    else if near_rz { hovered_handle = GizmoHandle::RotateZ; }
                                }
                            }

                            // Draw 3D Rotation Rings (when rotation tool is selected)
                            if comp.active_tool == 3 {
                                let rx_col = if hovered_handle == GizmoHandle::RotateX || gizmo_drag == GizmoHandle::RotateX {
                                    Color::from_rgba(255, 100, 100, 255)
                                } else {
                                    Color::from_rgba(230, 60, 60, 180)
                                };
                                for i in 0..ring_x_pts.len().saturating_sub(1) {
                                    draw_line(ring_x_pts[i].x, ring_x_pts[i].y, ring_x_pts[i + 1].x, ring_x_pts[i + 1].y, 2.0, rx_col);
                                }

                                let ry_col = if hovered_handle == GizmoHandle::RotateY || gizmo_drag == GizmoHandle::RotateY {
                                    Color::from_rgba(100, 255, 120, 255)
                                } else {
                                    Color::from_rgba(60, 220, 80, 180)
                                };
                                for i in 0..ring_y_pts.len().saturating_sub(1) {
                                    draw_line(ring_y_pts[i].x, ring_y_pts[i].y, ring_y_pts[i + 1].x, ring_y_pts[i + 1].y, 2.0, ry_col);
                                }

                                let rz_col = if hovered_handle == GizmoHandle::RotateZ || gizmo_drag == GizmoHandle::RotateZ {
                                    Color::from_rgba(120, 180, 255, 255)
                                } else {
                                    Color::from_rgba(60, 140, 255, 180)
                                };
                                for i in 0..ring_z_pts.len().saturating_sub(1) {
                                    draw_line(ring_z_pts[i].x, ring_z_pts[i].y, ring_z_pts[i + 1].x, ring_z_pts[i + 1].y, 2.0, rz_col);
                                }
                            }

                            // Draw Translation Gizmo (X=Red, Y=Green, Z=Blue)
                            let x_active = hovered_handle == GizmoHandle::TranslateX || gizmo_drag == GizmoHandle::TranslateX;
                            let x_col = if x_active { Color::from_rgba(255, 100, 100, 255) } else { Color::from_rgba(235, 50, 50, 220) };
                            draw_line(origin_s.x, origin_s.y, tip_s_x.x, tip_s_x.y, if x_active { 3.0 } else { 2.0 }, x_col);
                            draw_circle(tip_s_x.x, tip_s_x.y, if x_active { 5.0 } else { 4.0 }, x_col);
                            draw_text("X", tip_s_x.x + 6.0, tip_s_x.y + 4.0, 13.0, x_col);

                            let y_active = hovered_handle == GizmoHandle::TranslateY || gizmo_drag == GizmoHandle::TranslateY;
                            let y_col = if y_active { Color::from_rgba(100, 255, 120, 255) } else { Color::from_rgba(50, 220, 80, 220) };
                            draw_line(origin_s.x, origin_s.y, tip_s_y.x, tip_s_y.y, if y_active { 3.0 } else { 2.0 }, y_col);
                            draw_circle(tip_s_y.x, tip_s_y.y, if y_active { 5.0 } else { 4.0 }, y_col);
                            draw_text("Y", tip_s_y.x + 6.0, tip_s_y.y + 4.0, 13.0, y_col);

                            let z_active = hovered_handle == GizmoHandle::TranslateZ || gizmo_drag == GizmoHandle::TranslateZ;
                            let z_col = if z_active { Color::from_rgba(120, 180, 255, 255) } else { Color::from_rgba(50, 130, 255, 220) };
                            draw_line(origin_s.x, origin_s.y, tip_s_z.x, tip_s_z.y, if z_active { 3.0 } else { 2.0 }, z_col);
                            draw_circle(tip_s_z.x, tip_s_z.y, if z_active { 5.0 } else { 4.0 }, z_col);
                            draw_text("Z", tip_s_z.x + 6.0, tip_s_z.y + 4.0, 13.0, z_col);

                            let center_active = hovered_handle == GizmoHandle::CenterAnchor || gizmo_drag == GizmoHandle::CenterAnchor;
                            let center_col = if center_active { Color::from_rgba(255, 230, 80, 255) } else { Color::from_rgba(255, 200, 50, 200) };
                            draw_circle(origin_s.x, origin_s.y, 4.0, center_col);
                            draw_circle_lines(origin_s.x, origin_s.y, 7.0, 1.0, center_col);
                        }
                    } else {
                        // 2D LAYER INTERACTIVE BOUNDING BOX & 8-POINT TRANSFORM GIZMOS
                        let (anchor_pos, tl, tr, br, bl) = get_layer_2d_bounds(&comp, active_idx, &textures, comp.current_time);
                        let s_tl = to_screen(tl);
                        let s_tr = to_screen(tr);
                        let s_br = to_screen(br);
                        let s_bl = to_screen(bl);
                        let s_anchor = to_screen(anchor_pos);

                        let s_tc = (s_tl + s_tr) * 0.5;
                        let s_bc = (s_bl + s_br) * 0.5;
                        let s_lc = (s_tl + s_bl) * 0.5;
                        let s_rc = (s_tr + s_br) * 0.5;

                        // Top rotation handle stem & knob
                        let rot_dir = if (s_tc - s_bc).length() > 0.1 { (s_tc - s_bc).normalize() } else { vec2(0.0, -1.0) };
                        let s_rot = s_tc + rot_dir * 24.0;

                        // Bounding Box Rect Lines
                        let box_col = Color::from_rgba(70, 160, 255, 220);
                        draw_line(s_tl.x, s_tl.y, s_tr.x, s_tr.y, 1.5, box_col);
                        draw_line(s_tr.x, s_tr.y, s_br.x, s_br.y, 1.5, box_col);
                        draw_line(s_br.x, s_br.y, s_bl.x, s_bl.y, 1.5, box_col);
                        draw_line(s_bl.x, s_bl.y, s_tl.x, s_tl.y, 1.5, box_col);

                        // Rotation handle stem
                        draw_line(s_tc.x, s_tc.y, s_rot.x, s_rot.y, 1.2, Color::from_rgba(100, 200, 255, 180));
                        draw_circle(s_rot.x, s_rot.y, 4.5, Color::from_rgba(255, 255, 255, 240));
                        draw_circle_lines(s_rot.x, s_rot.y, 4.5, 1.5, Color::from_rgba(40, 120, 240, 255));

                        // 8 Square Resize Handles (4 corners + 4 edge centers)
                        let handle_pts = [
                            (s_tl, GizmoHandle::ScaleTL),
                            (s_tr, GizmoHandle::ScaleTR),
                            (s_br, GizmoHandle::ScaleBR),
                            (s_bl, GizmoHandle::ScaleBL),
                            (s_tc, GizmoHandle::ScaleT),
                            (s_bc, GizmoHandle::ScaleB),
                            (s_lc, GizmoHandle::ScaleL),
                            (s_rc, GizmoHandle::ScaleR),
                        ];

                        for (pt, _) in &handle_pts {
                            draw_rectangle(pt.x - 3.5, pt.y - 3.5, 7.0, 7.0, Color::from_rgba(255, 255, 255, 250));
                            draw_rectangle_lines(pt.x - 3.5, pt.y - 3.5, 7.0, 7.0, 1.2, Color::from_rgba(30, 100, 220, 255));
                        }

                        // Anchor Point with Crosshair
                        let anchor_col = Color::from_rgba(255, 215, 60, 240);
                        draw_circle(s_anchor.x, s_anchor.y, 3.0, anchor_col);
                        draw_circle_lines(s_anchor.x, s_anchor.y, 7.0, 1.2, anchor_col);
                        draw_line(s_anchor.x - 10.0, s_anchor.y, s_anchor.x + 10.0, s_anchor.y, 1.0, anchor_col);
                        draw_line(s_anchor.x, s_anchor.y - 10.0, s_anchor.x, s_anchor.y + 10.0, 1.0, anchor_col);

                        // 2D Handle Hit Testing
                        if in_viewport && gizmo_drag == GizmoHandle::None {
                            if (mouse_pos - s_rot).length() < 9.0 {
                                hovered_handle = GizmoHandle::Rotate2D;
                            } else if (mouse_pos - s_anchor).length() < 9.0 {
                                hovered_handle = if comp.active_tool == 5 { GizmoHandle::PanBehindAnchor } else { GizmoHandle::CenterAnchor };
                            } else {
                                for (pt, handle) in &handle_pts {
                                    if (mouse_pos - *pt).length() < 8.0 {
                                        hovered_handle = *handle;
                                        break;
                                    }
                                }
                                if hovered_handle == GizmoHandle::None {
                                    if point_in_triangle(mouse_pos, s_tl, s_tr, s_br) || point_in_triangle(mouse_pos, s_tl, s_br, s_bl) {
                                        hovered_handle = if comp.active_tool == 3 {
                                            GizmoHandle::Rotate2D
                                        } else if comp.active_tool == 5 {
                                            GizmoHandle::PanBehindAnchor
                                        } else {
                                            GizmoHandle::Translate2D
                                        };
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Mouse Pressed & Interaction Dispatcher
            if mouse_pressed && in_viewport {
                if comp.active_tool == 1 || space_down {
                    // Hand Tool
                    gizmo_drag = GizmoHandle::HandPan;
                    gizmo_drag_start_mouse = mouse_pos;
                } else if comp.active_tool == 2 {
                    // Zoom Tool
                    gizmo_drag = GizmoHandle::ZoomPan;
                    gizmo_drag_start_mouse = mouse_pos;
                    if alt_down {
                        if comp.viewport_mode == ViewportMode::CustomView {
                            comp.custom_orbit_distance = (comp.custom_orbit_distance * 1.25).min(30000.0);
                        } else if let Some(act) = comp.active_layer_index {
                            if act < comp.layers.len() && comp.layers[act].source == LayerSource::Camera {
                                let cur = comp.layers[act].properties.get("zoom").map_or(1500.0, |p| p.get_value_at(comp.current_time));
                                update_layer_property_val(&mut comp.layers[act], "zoom", (cur / 1.25).max(100.0), comp.current_time);
                            }
                        }
                    } else {
                        if comp.viewport_mode == ViewportMode::CustomView {
                            comp.custom_orbit_distance = (comp.custom_orbit_distance * 0.8).max(100.0);
                        } else if let Some(act) = comp.active_layer_index {
                            if act < comp.layers.len() && comp.layers[act].source == LayerSource::Camera {
                                let cur = comp.layers[act].properties.get("zoom").map_or(1500.0, |p| p.get_value_at(comp.current_time));
                                update_layer_property_val(&mut comp.layers[act], "zoom", (cur * 1.25).min(20000.0), comp.current_time);
                            }
                        }
                    }
                } else if comp.active_tool == 4 || (alt_down && (comp.viewport_mode != ViewportMode::ActiveCamera || comp.layers.iter().any(|l| l.source == LayerSource::Camera))) {
                    // Camera Tool
                    gizmo_drag = if mid_down || shift_down {
                        GizmoHandle::CameraPan
                    } else if right_down {
                        GizmoHandle::CameraDolly
                    } else {
                        GizmoHandle::CameraOrbit
                    };
                    gizmo_drag_start_mouse = mouse_pos;
                } else if comp.active_tool == 6 {
                    // Shape / Rectangle Tool
                    shape_drag_start = Some(comp_mouse);
                } else if comp.active_tool == 7 {
                    // Pen Tool
                    let mut added = false;
                    if let Some(act) = comp.active_layer_index {
                        if act < comp.layers.len() {
                            let (ax, ay, x, y, _, _, _, _, sx, sy) = layer_transform(&comp, act, comp.current_time);
                            if let LayerSource::Polygon { points, .. } = &mut comp.layers[act].source {
                                let local_x = (comp_mouse.x - x + ax) / sx.abs().max(0.01);
                                let local_y = (comp_mouse.y - y + ay) / sy.abs().max(0.01);
                                points.push([local_x, local_y]);
                                added = true;
                            }
                        }
                    }
                    if !added {
                        let idx = comp.layers.len();
                        let mut new_poly = default_layer(
                            format!("Polygon {}", idx + 1),
                            LayerSource::Polygon {
                                points: vec![[-60.0, -50.0], [60.0, -50.0], [0.0, 60.0]],
                                color: [0.95, 0.45, 0.2, 1.0],
                            },
                            idx % AE_LABEL_COLORS.len(),
                        );
                        new_poly.properties.get_mut("x").unwrap().base_value = comp_mouse.x;
                        new_poly.properties.get_mut("y").unwrap().base_value = comp_mouse.y;
                        comp.layers.push(new_poly);
                        comp.active_layer_index = Some(idx);
                    }
                } else if comp.active_tool == 8 {
                    // Type Tool
                    let idx = comp.layers.len();
                    let mut new_text = default_layer(
                        format!("Text Layer {}", idx + 1),
                        LayerSource::Text {
                            text: "Sample Text".to_string(),
                            font_size: 48.0,
                            color: [1.0, 1.0, 1.0, 1.0],
                        },
                        idx % AE_LABEL_COLORS.len(),
                    );
                    new_text.properties.get_mut("x").unwrap().base_value = comp_mouse.x;
                    new_text.properties.get_mut("y").unwrap().base_value = comp_mouse.y;
                    comp.layers.push(new_text);
                    comp.active_layer_index = Some(idx);
                    comp.active_tool = 0; // Switch to Selection tool
                } else {
                    // Selection Tool (0) / Rotation Tool (3) / Pan Behind Tool (5)
                    if hovered_handle != GizmoHandle::None {
                        gizmo_drag = hovered_handle;
                    } else {
                        let clicked_layer = hit_test_layers(&comp, &textures, comp_mouse, comp.current_time);
                        comp.active_layer_index = clicked_layer;
                        if clicked_layer.is_some() {
                            gizmo_drag = if comp.active_tool == 3 {
                                GizmoHandle::Rotate2D
                            } else if comp.active_tool == 5 {
                                GizmoHandle::PanBehindAnchor
                            } else {
                                GizmoHandle::Translate2D
                            };
                        }
                    }

                    gizmo_drag_start_mouse = mouse_pos;
                    if let Some(active_idx) = comp.active_layer_index {
                        if active_idx < comp.layers.len() {
                            let layer = &comp.layers[active_idx];
                            let time = comp.current_time;
                            gizmo_drag_start_x = layer.properties.get("x").map_or(comp_w / 2.0, |p| p.get_value_at(time));
                            gizmo_drag_start_y = layer.properties.get("y").map_or(comp_h / 2.0, |p| p.get_value_at(time));
                            gizmo_drag_start_z = layer.properties.get("z").map_or(0.0, |p| p.get_value_at(time));
                            gizmo_drag_start_ax = layer.properties.get("anchorX").map_or(0.0, |p| p.get_value_at(time));
                            gizmo_drag_start_ay = layer.properties.get("anchorY").map_or(0.0, |p| p.get_value_at(time));
                            gizmo_drag_start_sx = layer.properties.get("scaleX").map_or(100.0, |p| p.get_value_at(time));
                            gizmo_drag_start_sy = layer.properties.get("scaleY").map_or(100.0, |p| p.get_value_at(time));
                            gizmo_drag_start_rot = layer.properties.get("rotation").map_or(0.0, |p| p.get_value_at(time));

                            gizmo_drag_start_val = match gizmo_drag {
                                GizmoHandle::TranslateX => gizmo_drag_start_x,
                                GizmoHandle::TranslateY => gizmo_drag_start_y,
                                GizmoHandle::TranslateZ => gizmo_drag_start_z,
                                GizmoHandle::RotateX => layer.properties.get("rotationX").map_or(0.0, |p| p.get_value_at(time)),
                                GizmoHandle::RotateY => layer.properties.get("rotationY").map_or(0.0, |p| p.get_value_at(time)),
                                GizmoHandle::RotateZ => gizmo_drag_start_rot,
                                _ => 0.0,
                            };
                        }
                    }
                }
            }

            if (mid_pressed || (right_pressed && alt_down)) && in_viewport {
                gizmo_drag = if mid_pressed { GizmoHandle::CameraPan } else { GizmoHandle::CameraDolly };
                gizmo_drag_start_mouse = mouse_pos;
            }

            // Shape Tool Live Dragging Box & Creation on Release
            if let Some(start) = shape_drag_start {
                let p1 = to_screen(start);
                let p2 = to_screen(comp_mouse);
                let rx = p1.x.min(p2.x);
                let ry = p1.y.min(p2.y);
                let rw = (p1.x - p2.x).abs();
                let rh = (p1.y - p2.y).abs();
                draw_rectangle(rx, ry, rw, rh, Color::from_rgba(60, 140, 240, 70));
                draw_rectangle_lines(rx, ry, rw, rh, 1.5, Color::from_rgba(100, 200, 255, 230));

                let dim_text = format!("{:.0} × {:.0}", (comp_mouse.x - start.x).abs(), (comp_mouse.y - start.y).abs());
                draw_text(&dim_text, rx + 6.0, ry + rh + 16.0, 12.0, Color::from_rgba(255, 255, 255, 240));

                if mouse_released {
                    let w = (comp_mouse.x - start.x).abs();
                    let h = (comp_mouse.y - start.y).abs();
                    if w > 4.0 && h > 4.0 {
                        let cx = (start.x + comp_mouse.x) * 0.5;
                        let cy = (start.y + comp_mouse.y) * 0.5;
                        let idx = comp.layers.len();
                        let mut new_shape = default_layer(
                            format!("Shape Layer {}", idx + 1),
                            LayerSource::Solid { color: [0.28, 0.58, 0.92, 1.0] },
                            idx % AE_LABEL_COLORS.len(),
                        );
                        new_shape.properties.get_mut("x").unwrap().base_value = cx;
                        new_shape.properties.get_mut("y").unwrap().base_value = cy;
                        new_shape.properties.get_mut("anchorX").unwrap().base_value = 100.0;
                        new_shape.properties.get_mut("anchorY").unwrap().base_value = 100.0;
                        new_shape.properties.get_mut("scaleX").unwrap().base_value = (w / 200.0) * 100.0;
                        new_shape.properties.get_mut("scaleY").unwrap().base_value = (h / 200.0) * 100.0;
                        comp.layers.push(new_shape);
                        comp.active_layer_index = Some(idx);
                        comp.active_tool = 0; // Switch to Selection tool
                    }
                    shape_drag_start = None;
                }
            }

            if mouse_released || (!mouse_down && !mid_down && !right_down) {
                gizmo_drag = GizmoHandle::None;
            }

            // Active Dragging Execution
            if gizmo_drag != GizmoHandle::None {
                let mouse_delta = mouse_pos - gizmo_drag_start_mouse;
                let comp_delta = mouse_delta * vec2(comp_w / screen_w, comp_h / screen_h);

                match gizmo_drag {
                    GizmoHandle::Translate2D => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "x", gizmo_drag_start_x + comp_delta.x, comp.current_time);
                                update_layer_property_val(&mut comp.layers[active_idx], "y", gizmo_drag_start_y + comp_delta.y, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::TranslateX => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "x", gizmo_drag_start_x + comp_delta.x, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::TranslateY => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "y", gizmo_drag_start_y + comp_delta.y, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::TranslateZ => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "z", gizmo_drag_start_z - mouse_delta.y * 2.0, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::Rotate2D => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                let anchor_s = to_screen(vec2(gizmo_drag_start_x, gizmo_drag_start_y));
                                let init_ang = (gizmo_drag_start_mouse - anchor_s).y.atan2((gizmo_drag_start_mouse - anchor_s).x).to_degrees();
                                let cur_ang = (mouse_pos - anchor_s).y.atan2((mouse_pos - anchor_s).x).to_degrees();
                                let delta_ang = cur_ang - init_ang;
                                update_layer_property_val(&mut comp.layers[active_idx], "rotation", gizmo_drag_start_rot + delta_ang, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::RotateX => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "rotationX", gizmo_drag_start_val - mouse_delta.y * 0.7, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::RotateY => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "rotationY", gizmo_drag_start_val + mouse_delta.x * 0.7, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::RotateZ => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                update_layer_property_val(&mut comp.layers[active_idx], "rotation", gizmo_drag_start_val + mouse_delta.x * 0.7, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::ScaleTL | GizmoHandle::ScaleTR | GizmoHandle::ScaleBL | GizmoHandle::ScaleBR
                    | GizmoHandle::ScaleT | GizmoHandle::ScaleB | GizmoHandle::ScaleL | GizmoHandle::ScaleR => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                let rot_rad = -gizmo_drag_start_rot.to_radians();
                                let local_dx = comp_delta.x * rot_rad.cos() - comp_delta.y * rot_rad.sin();
                                let local_dy = comp_delta.x * rot_rad.sin() + comp_delta.y * rot_rad.cos();

                                let (mult_x, mult_y) = match gizmo_drag {
                                    GizmoHandle::ScaleBR => (1.0, 1.0),
                                    GizmoHandle::ScaleBL => (-1.0, 1.0),
                                    GizmoHandle::ScaleTR => (1.0, -1.0),
                                    GizmoHandle::ScaleTL => (-1.0, -1.0),
                                    GizmoHandle::ScaleR => (1.0, 0.0),
                                    GizmoHandle::ScaleL => (-1.0, 0.0),
                                    GizmoHandle::ScaleB => (0.0, 1.0),
                                    GizmoHandle::ScaleT => (0.0, -1.0),
                                    _ => (1.0, 1.0),
                                };

                                if mult_x != 0.0 {
                                    let new_sx = (gizmo_drag_start_sx + (local_dx * mult_x / 200.0) * 100.0).max(1.0);
                                    update_layer_property_val(&mut comp.layers[active_idx], "scaleX", new_sx, comp.current_time);
                                }
                                if mult_y != 0.0 {
                                    let new_sy = (gizmo_drag_start_sy + (local_dy * mult_y / 200.0) * 100.0).max(1.0);
                                    update_layer_property_val(&mut comp.layers[active_idx], "scaleY", new_sy, comp.current_time);
                                }
                            }
                        }
                    }
                    GizmoHandle::PanBehindAnchor => {
                        if let Some(active_idx) = comp.active_layer_index {
                            if active_idx < comp.layers.len() {
                                let rot_rad = -gizmo_drag_start_rot.to_radians();
                                let scale_x_factor = (gizmo_drag_start_sx / 100.0).abs().max(0.01);
                                let scale_y_factor = (gizmo_drag_start_sy / 100.0).abs().max(0.01);
                                let local_dx = (comp_delta.x * rot_rad.cos() - comp_delta.y * rot_rad.sin()) / scale_x_factor;
                                let local_dy = (comp_delta.x * rot_rad.sin() + comp_delta.y * rot_rad.cos()) / scale_y_factor;

                                update_layer_property_val(&mut comp.layers[active_idx], "anchorX", gizmo_drag_start_ax + local_dx, comp.current_time);
                                update_layer_property_val(&mut comp.layers[active_idx], "anchorY", gizmo_drag_start_ay + local_dy, comp.current_time);
                                update_layer_property_val(&mut comp.layers[active_idx], "x", gizmo_drag_start_x + comp_delta.x, comp.current_time);
                                update_layer_property_val(&mut comp.layers[active_idx], "y", gizmo_drag_start_y + comp_delta.y, comp.current_time);
                            }
                        }
                    }
                    GizmoHandle::HandPan => {
                        if comp.viewport_mode == ViewportMode::CustomView {
                            comp.custom_orbit_target[0] -= comp_delta.x;
                            comp.custom_orbit_target[1] -= comp_delta.y;
                            gizmo_drag_start_mouse = mouse_pos;
                        } else if let Some(c_idx) = comp.layers.iter().position(|l| l.source == LayerSource::Camera) {
                            let cur_x = comp.layers[c_idx].properties.get("x").map_or(comp_w / 2.0, |p| p.get_value_at(comp.current_time));
                            let cur_y = comp.layers[c_idx].properties.get("y").map_or(comp_h / 2.0, |p| p.get_value_at(comp.current_time));
                            let cur_px = comp.layers[c_idx].properties.get("poiX").map_or(comp_w / 2.0, |p| p.get_value_at(comp.current_time));
                            let cur_py = comp.layers[c_idx].properties.get("poiY").map_or(comp_h / 2.0, |p| p.get_value_at(comp.current_time));
                            update_layer_property_val(&mut comp.layers[c_idx], "x", cur_x - comp_delta.x, comp.current_time);
                            update_layer_property_val(&mut comp.layers[c_idx], "y", cur_y - comp_delta.y, comp.current_time);
                            update_layer_property_val(&mut comp.layers[c_idx], "poiX", cur_px - comp_delta.x, comp.current_time);
                            update_layer_property_val(&mut comp.layers[c_idx], "poiY", cur_py - comp_delta.y, comp.current_time);
                            gizmo_drag_start_mouse = mouse_pos;
                        }
                    }
                    GizmoHandle::ZoomPan => {
                        if comp.viewport_mode == ViewportMode::CustomView {
                            comp.custom_orbit_distance = (comp.custom_orbit_distance - mouse_delta.y * 8.0).clamp(100.0, 30000.0);
                            gizmo_drag_start_mouse = mouse_pos;
                        } else if let Some(c_idx) = comp.layers.iter().position(|l| l.source == LayerSource::Camera) {
                            let cur_z = comp.layers[c_idx].properties.get("z").map_or(-1500.0, |p| p.get_value_at(comp.current_time));
                            update_layer_property_val(&mut comp.layers[c_idx], "z", cur_z + mouse_delta.y * 6.0, comp.current_time);
                            gizmo_drag_start_mouse = mouse_pos;
                        }
                    }
                    GizmoHandle::CameraOrbit => {
                        let is_ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
                        let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                        if comp.viewport_mode == ViewportMode::CustomView {
                            if is_ctrl || (is_shift && right_down) {
                                comp.custom_orbit_roll += mouse_delta.x * 0.4;
                            } else {
                                comp.custom_orbit_yaw += mouse_delta.x * 0.4;
                                comp.custom_orbit_pitch = (comp.custom_orbit_pitch + mouse_delta.y * 0.4).clamp(-89.0, 89.0);
                            }
                            gizmo_drag_start_mouse = mouse_pos;
                        } else if comp.viewport_mode == ViewportMode::ActiveCamera {
                            let mut cam_idx = comp.active_layer_index.filter(|&act| act < comp.layers.len() && comp.layers[act].source == LayerSource::Camera);
                            if cam_idx.is_none() {
                                cam_idx = comp.layers.iter().position(|l| l.source == LayerSource::Camera);
                            }
                            if let Some(c_idx) = cam_idx {
                                if is_ctrl || is_shift {
                                    let cur_rz = comp.layers[c_idx].properties.get("rotation").map_or(0.0, |p| p.get_value_at(comp.current_time));
                                    update_layer_property_val(&mut comp.layers[c_idx], "rotation", cur_rz + mouse_delta.x * 0.4, comp.current_time);
                                } else {
                                    let cur_ry = comp.layers[c_idx].properties.get("rotationY").map_or(0.0, |p| p.get_value_at(comp.current_time));
                                    let cur_rx = comp.layers[c_idx].properties.get("rotationX").map_or(0.0, |p| p.get_value_at(comp.current_time));
                                    update_layer_property_val(&mut comp.layers[c_idx], "rotationY", cur_ry + mouse_delta.x * 0.4, comp.current_time);
                                    update_layer_property_val(&mut comp.layers[c_idx], "rotationX", (cur_rx + mouse_delta.y * 0.4).clamp(-89.0, 89.0), comp.current_time);
                                }
                                gizmo_drag_start_mouse = mouse_pos;
                            }
                        }
                    }
                    GizmoHandle::CameraPan => {
                        if comp.viewport_mode == ViewportMode::CustomView {
                            comp.custom_orbit_target[0] -= comp_delta.x;
                            comp.custom_orbit_target[1] -= comp_delta.y;
                            gizmo_drag_start_mouse = mouse_pos;
                        } else if comp.viewport_mode == ViewportMode::ActiveCamera {
                            let mut cam_idx = comp.active_layer_index.filter(|&act| act < comp.layers.len() && comp.layers[act].source == LayerSource::Camera);
                            if cam_idx.is_none() {
                                cam_idx = comp.layers.iter().position(|l| l.source == LayerSource::Camera);
                            }
                            if let Some(c_idx) = cam_idx {
                                let dx = -comp_delta.x;
                                let dy = -comp_delta.y;
                                let cur_x = comp.layers[c_idx].properties.get("x").map_or(comp_w / 2.0, |p| p.get_value_at(comp.current_time));
                                let cur_y = comp.layers[c_idx].properties.get("y").map_or(comp_h / 2.0, |p| p.get_value_at(comp.current_time));
                                let cur_px = comp.layers[c_idx].properties.get("poiX").map_or(comp_w / 2.0, |p| p.get_value_at(comp.current_time));
                                let cur_py = comp.layers[c_idx].properties.get("poiY").map_or(comp_h / 2.0, |p| p.get_value_at(comp.current_time));
                                update_layer_property_val(&mut comp.layers[c_idx], "x", cur_x + dx, comp.current_time);
                                update_layer_property_val(&mut comp.layers[c_idx], "y", cur_y + dy, comp.current_time);
                                update_layer_property_val(&mut comp.layers[c_idx], "poiX", cur_px + dx, comp.current_time);
                                update_layer_property_val(&mut comp.layers[c_idx], "poiY", cur_py + dy, comp.current_time);
                                gizmo_drag_start_mouse = mouse_pos;
                            }
                        }
                    }
                    GizmoHandle::CameraDolly => {
                        if comp.viewport_mode == ViewportMode::CustomView {
                            comp.custom_orbit_distance = (comp.custom_orbit_distance - mouse_delta.y * 5.0).clamp(100.0, 30000.0);
                            gizmo_drag_start_mouse = mouse_pos;
                        } else if comp.viewport_mode == ViewportMode::ActiveCamera {
                            let mut cam_idx = comp.active_layer_index.filter(|&act| act < comp.layers.len() && comp.layers[act].source == LayerSource::Camera);
                            if cam_idx.is_none() {
                                cam_idx = comp.layers.iter().position(|l| l.source == LayerSource::Camera);
                            }
                            if let Some(c_idx) = cam_idx {
                                let cur_z = comp.layers[c_idx].properties.get("z").map_or(-1500.0, |p| p.get_value_at(comp.current_time));
                                update_layer_property_val(&mut comp.layers[c_idx], "z", cur_z + mouse_delta.y * 4.0, comp.current_time);
                                gizmo_drag_start_mouse = mouse_pos;
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Viewport Badges: Mode & Active Tool
            let mode_title = match comp.viewport_mode {
                ViewportMode::ActiveCamera => "Active Camera",
                ViewportMode::CustomView => "Custom View 1",
                ViewportMode::Top => "Top (XZ)",
                ViewportMode::Front => "Front (XY)",
                ViewportMode::Right => "Right (YZ)",
                ViewportMode::Left => "Left (-YZ)",
                ViewportMode::Bottom => "Bottom (-XZ)",
                ViewportMode::Back => "Back (-XY)",
            };
            draw_rectangle(screen_x + 12.0, screen_y + 12.0, 115.0, 22.0, Color::from_rgba(20, 20, 26, 190));
            draw_rectangle_lines(screen_x + 12.0, screen_y + 12.0, 115.0, 22.0, 1.0, Color::from_rgba(70, 70, 85, 160));
            draw_text(mode_title, screen_x + 18.0, screen_y + 27.0, 12.0, Color::from_rgba(200, 210, 230, 240));

            let tool_badge = match comp.active_tool {
                0 => "↖ Selection (V)",
                1 => "✋ Hand (H)",
                2 => "🔍 Zoom (Z)",
                3 => "🔄 Rotation (W)",
                4 => "🎥 Camera (C)",
                5 => "⚓ Pan Behind (Y)",
                6 => "▭ Shape (Q)",
                7 => "✒ Pen (G)",
                8 => "T Type (T)",
                _ => "↖ Tool",
            };
            draw_rectangle(screen_x + 132.0, screen_y + 12.0, 115.0, 22.0, Color::from_rgba(24, 24, 32, 190));
            draw_rectangle_lines(screen_x + 132.0, screen_y + 12.0, 115.0, 22.0, 1.0, Color::from_rgba(60, 140, 220, 180));
            draw_text(tool_badge, screen_x + 138.0, screen_y + 27.0, 12.0, Color::from_rgba(140, 205, 255, 240));

            // 3D Viewport Orientation Tripod Widget (Top-Right)
            let tripod_origin = vec2(screen_x + screen_w - 36.0, screen_y + 36.0);
            draw_circle(tripod_origin.x, tripod_origin.y, 20.0, Color::from_rgba(25, 25, 30, 190));
            draw_circle_lines(tripod_origin.x, tripod_origin.y, 20.0, 1.0, Color::from_rgba(70, 70, 85, 160));

            let t_forward = if (viewport_cam.target - viewport_cam.position).length() > 0.0001 {
                (viewport_cam.target - viewport_cam.position).normalize()
            } else {
                vec3(0.0, 0.0, 1.0)
            };
            let up_hint = if t_forward.y.abs() > 0.999 { vec3(0.0, 0.0, 1.0) } else { vec3(0.0, 1.0, 0.0) };
            let t_right = t_forward.cross(up_hint).normalize();
            let t_up = t_right.cross(t_forward).normalize();

            let tx = vec2(vec3(1.0, 0.0, 0.0).dot(t_right), vec3(1.0, 0.0, 0.0).dot(t_up)) * 14.0;
            let ty = vec2(vec3(0.0, 1.0, 0.0).dot(t_right), vec3(0.0, 1.0, 0.0).dot(t_up)) * 14.0;
            let tz = vec2(vec3(0.0, 0.0, 1.0).dot(t_right), vec3(0.0, 0.0, 1.0).dot(t_up)) * 14.0;

            draw_line(tripod_origin.x, tripod_origin.y, tripod_origin.x + tx.x, tripod_origin.y + tx.y, 2.0, Color::from_rgba(240, 60, 60, 255));
            draw_text("X", tripod_origin.x + tx.x + 3.0, tripod_origin.y + tx.y + 3.0, 10.0, Color::from_rgba(240, 60, 60, 255));

            draw_line(tripod_origin.x, tripod_origin.y, tripod_origin.x + ty.x, tripod_origin.y + ty.y, 2.0, Color::from_rgba(60, 220, 80, 255));
            draw_text("Y", tripod_origin.x + ty.x + 3.0, tripod_origin.y + ty.y + 3.0, 10.0, Color::from_rgba(60, 220, 80, 255));

            draw_line(tripod_origin.x, tripod_origin.y, tripod_origin.x + tz.x, tripod_origin.y + tz.y, 2.0, Color::from_rgba(60, 140, 255, 255));
            draw_text("Z", tripod_origin.x + tz.x + 3.0, tripod_origin.y + tz.y + 3.0, 10.0, Color::from_rgba(60, 140, 255, 255));
        }

        next_frame().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bp_rle_compression_roundtrip() {
        // Test solid background frame (highly compressible)
        let mut solid_frame = vec![0u8; 1920 * 1080 * 4];
        for chunk in solid_frame.chunks_mut(4) {
            chunk[0] = 34;
            chunk[1] = 68;
            chunk[2] = 128;
            chunk[3] = 255;
        }

        let compressed = compress_rgba_frame(&solid_frame);
        assert!(!compressed.is_empty());
        assert!(compressed.len() < solid_frame.len() / 10, "Solid color should achieve >10x compression");

        let mut decompressed = vec![0u8; solid_frame.len()];
        let success = decompress_rgba_frame(&compressed, &mut decompressed);
        assert!(success);
        assert_eq!(solid_frame, decompressed);

        // Test non-trivial gradient / patterned frame
        let mut grad_frame = vec![0u8; 100 * 100 * 4];
        for y in 0..100 {
            for x in 0..100 {
                let idx = (y * 100 + x) * 4;
                grad_frame[idx] = (x * 2) as u8;
                grad_frame[idx + 1] = (y * 2) as u8;
                grad_frame[idx + 2] = ((x + y) / 2) as u8;
                grad_frame[idx + 3] = 255;
            }
        }

        let compressed_grad = compress_rgba_frame_planar(&grad_frame);
        let mut decompressed_grad = vec![0u8; grad_frame.len()];
        let ok = decompress_rgba_frame(&compressed_grad, &mut decompressed_grad);
        assert!(ok);
        assert_eq!(grad_frame, decompressed_grad);
    }

    #[test]
    fn test_ultra_fast_direct_compression_roundtrip() {
        let mut frame = vec![0u8; 256 * 256 * 4];
        for (i, chunk) in frame.chunks_mut(4).enumerate() {
            if i < 30000 {
                chunk[0] = 10;
                chunk[1] = 20;
                chunk[2] = 30;
                chunk[3] = 255;
            } else {
                chunk[0] = (i % 255) as u8;
                chunk[1] = ((i * 3) % 255) as u8;
                chunk[2] = ((i * 7) % 255) as u8;
                chunk[3] = 255;
            }
        }

        let compressed = compress_rgba_frame_ultra(&frame);
        assert!(!compressed.is_empty());
        assert!(compressed.starts_with(b"BFXU"));

        let mut decompressed = vec![0u8; frame.len()];
        let ok = decompress_rgba_frame(&compressed, &mut decompressed);
        assert!(ok);
        assert_eq!(frame, decompressed);
    }

    #[test]
    fn test_caching_exceeds_301_frames() {
        // Test that RamPreviewCache can store 500+ frames without being capped at 301
        let mut cache = RamPreviewCache::new(1000, 4096.0);
        let img = Image {
            bytes: vec![128u8; 32 * 32 * 4],
            width: 32,
            height: 32,
        };

        for f in 0..500 {
            cache.insert(f, &img, true, CacheCompressionMode::FastPlanarRle);
        }

        assert_eq!(cache.frames.len(), 500, "Cache should successfully store 500 frames");
        assert!(cache.frames.contains_key(&0));
        assert!(cache.frames.contains_key(&499));
    }

    #[test]
    fn test_ram_preview_cache_memory_limit_eviction() {
        // Set a small 1 MB limit and insert uncompressed frames to verify LRU eviction
        let mut cache = RamPreviewCache::new(100, 0.5); // 0.5 MB max
        let img = Image {
            bytes: vec![200u8; 256 * 256 * 4], // 256 KB per frame
            width: 256,
            height: 256,
        };

        for f in 0..10 {
            cache.insert(f, &img, false, CacheCompressionMode::Uncompressed);
        }

        // 10 frames * 256 KB = 2.5 MB -> should evict down to fit <= 0.5 MB (around 2 frames)
        assert!(cache.frames.len() <= 3);
        assert!(cache.memory_usage_mb() <= 1.0);
    }

    #[test]
    fn test_ram_preview_cache_compression() {
        let mut cache = RamPreviewCache::new(10, 1024.0);
        let img = Image {
            bytes: vec![42u8; 64 * 64 * 4],
            width: 64,
            height: 64,
        };

        cache.insert(0, &img, true, CacheCompressionMode::FastPlanarRle);
        assert_eq!(cache.frames.len(), 1);
        let raw_mb = cache.raw_memory_usage_mb();
        let used_mb = cache.memory_usage_mb();
        assert!(raw_mb > 0.0);
        assert!(used_mb < raw_mb);
        assert!(cache.compression_ratio() >= 1.0);

        let cached = cache.frames.get(&0).unwrap();
        let mut decomp = vec![0u8; img.bytes.len()];
        let ok = decompress_rgba_frame(&cached.compressed, &mut decomp);
        assert!(ok);
        assert_eq!(img.bytes, decomp);
    }

    #[test]
    fn test_3d_viewport_rotation_and_camera() {
        let mut comp = Composition::default();
        comp.viewport_mode = ViewportMode::CustomView;
        comp.custom_orbit_yaw = 45.0;
        comp.custom_orbit_pitch = 30.0;
        comp.custom_orbit_roll = 15.0;
        comp.custom_orbit_distance = 1500.0;
        comp.custom_orbit_target = [960.0, 540.0, 0.0];

        let cam = get_viewport_camera(&comp, 0.0);
        assert!(cam.is_custom);
        assert_eq!(cam.rotation.z, 15.0);

        // Verify 3D perspective projection with roll
        let world_pt = vec3(960.0, 540.0, 0.0);
        let proj = project_3d_point(world_pt, &cam, 1920.0, 1080.0);
        assert!(proj.visible);
        assert!((proj.screen.x - 960.0).abs() < 5.0);
        assert!((proj.screen.y - 540.0).abs() < 5.0);
    }

    #[test]
    fn test_effects_processing() {
        // Test HSL <-> RGB conversions
        let (h, s, l) = rgb_to_hsl(1.0, 0.0, 0.0);
        let (r, g, b) = hsl_to_rgb(h, s, l);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);

        // Test Color Effects (Fill, Tint, Invert, BrightnessContrast, HueSaturation)
        let mut layer = default_layer(
            "Test".into(),
            LayerSource::Solid { color: [1.0, 0.0, 0.0, 1.0] },
            0,
        );
        layer.fx = true;

        let mut eff_fill = LayerEffect::new("Fill".into(), EffectType::Fill);
        eff_fill.properties.get_mut("colorR").unwrap().base_value = 0.0;
        eff_fill.properties.get_mut("colorG").unwrap().base_value = 255.0;
        eff_fill.properties.get_mut("colorB").unwrap().base_value = 0.0;
        eff_fill.properties.get_mut("opacity").unwrap().base_value = 100.0;
        layer.effects.push(eff_fill);

        let initial_col = Color::new(1.0, 0.0, 0.0, 1.0);
        let filled_col = apply_color_effects(initial_col, &layer, 0.0);
        assert_eq!(filled_col.r, 0.0);
        assert_eq!(filled_col.g, 1.0);
        assert_eq!(filled_col.b, 0.0);
    }
}
