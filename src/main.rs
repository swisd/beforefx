mod core;
mod ui_utils;

use crate::core::*;
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

fn create_default_properties() -> HashMap<String, Property> {
    [
        ("anchorX", 0.0),
        ("anchorY", 0.0),
        ("x", 960.0),
        ("y", 540.0),
        ("rotation", 0.0),
        ("rotationX", 0.0),
        ("rotationY", 0.0),
        ("z", 0.0),
        ("scaleX", 100.0),
        ("scaleY", 100.0),
        ("opacity", 100.0),
    ]
    .iter()
    .map(|(name, val)| {
        (
            name.to_string(),
            Property {
                name: name.to_string(),
                base_value: *val,
                keyframes: vec![],
            },
        )
    })
    .collect()
}

fn default_layer(name: String, source: LayerSource, label_color_index: usize) -> Layer {
    Layer {
        name,
        source,
        properties: create_default_properties(),
        visible: true,
        locked: false,
        solo: false,
        fx: true,
        d3: false,
        ff: false,
        moblur: false,
        shy: false,
        collapse: false,
        collapsed: false,
        in_time: 0.0,
        out_time: 30.0,
        label_color_index,
        blend_mode: "Normal".to_string(),
        parent_index: None,
        track_matte: "None".to_string(),
    }
}

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

    // Has any solo layer?
    let has_solo = comp.layers.iter().any(|l| l.solo);

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

        let (ax, ay, x, y, z, rot, rot_x, rot_y, sx, sy) = layer_transform(comp, layer_idx, time);
        let op = layer
            .properties
            .get("opacity")
            .map_or(100.0, |p| p.get_value_at(time))
            / 100.0;

        match &layer.source {
            LayerSource::Solid { color } => {
                draw_rectangle_ex(
                    x,
                    y,
                    200.0 * sx,
                    200.0 * sy,
                    DrawRectangleParams {
                        offset: vec2(ax / 200.0, ay / 200.0),
                        rotation: rot.to_radians(),
                        color: Color::new(color[0], color[1], color[2], color[3] * op),
                    },
                );
            }
            LayerSource::Text {
                text,
                font_size,
                color,
            } => {
                let size = (*font_size * sx.abs()).max(8.0);
                let text_color = Color::new(color[0], color[1], color[2], color[3] * op);
                draw_text(text, x - ax, y - ay, size, text_color);
            }
            LayerSource::Image { path } => {
                if let Some(tex) = textures.get(path) {
                    draw_texture_ex(
                        tex,
                        x,
                        y,
                        Color::new(1.0, 1.0, 1.0, op),
                        DrawTextureParams {
                            dest_size: Some(vec2(tex.width() * sx, tex.height() * sy)),
                            rotation: rot.to_radians(),
                            pivot: Some(vec2(x + ax, y + ay)),
                            ..Default::default()
                        },
                    );
                }
            }
            LayerSource::Video { path } => draw_video_placeholder(path, x, y, sx, sy, op),
            LayerSource::Polygon { points, color } => {
                draw_polygon_layer(
                    points,
                    x,
                    y,
                    sx,
                    sy,
                    Color::new(color[0], color[1], color[2], color[3] * op),
                );
            }
            LayerSource::Audio { .. } | LayerSource::Adjustment | LayerSource::Null => {}
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
                draw_cube(
                    pos,
                    size,
                    None,
                    Color::new(color[0], color[1], color[2], color[3] * op),
                );
                draw_cube_wires(pos, size, Color::new(1.0, 1.0, 1.0, op));
                set_camera(&Camera2D {
                    render_target: Some(target.clone()),
                    ..Camera2D::from_display_rect(Rect::new(0., 0., width, height))
                });
                let _ = (rot, rot_x, rot_y);
            }
        }
    }
    set_default_camera();
}

fn get_max_keyframe_time(comp: &Composition) -> f32 {
    let mut max_time = 0.0f32;
    for layer in &comp.layers {
        for prop in layer.properties.values() {
            if let Some(last_kf) = prop.keyframes.last() {
                if last_kf.time > max_time {
                    max_time = last_kf.time;
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

        for frame_idx in 0..frame_count {
            let time = frame_idx as f32 / fps as f32;
            draw_composition(comp, textures, time, render_target.clone());
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
    };

    let mut selected_keyframe: Option<SelectedKeyframe> = None;
    let mut textures: HashMap<String, Texture2D> = HashMap::new();
    let mut sounds: HashMap<String, Sound> = HashMap::new();
    let mut to_load: Vec<String> = vec![];
    let mut to_load_audio: Vec<String> = vec![];
    let mut audio_started = false;
    let mut pending_export: Option<PathBuf> = None;
    let mut export_status = String::new();
    let mut show_shortcuts_dialog = false;

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

        // --- PLAYBACK ENGINE ---
        if comp.is_playing {
            comp.current_time += get_frame_time();
            if comp.current_time >= comp.work_area_out.min(comp.settings.duration) {
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
            comp.is_playing = !comp.is_playing;
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
        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            if is_key_pressed(KeyCode::D) {
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

        // --- 1. RENDER ANIMATION TO TEXTURE ---
        draw_composition(&comp, &textures, comp.current_time, render_target.clone());
        clear_background(Color::from_rgba(18, 18, 20, 255));

        // --- 2. AFTER EFFECTS UI INTERFACE ---
        egui_macroquad::ui(|ctx| {
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
                                resources: vec![],
                                current_time: 0.0,
                                is_playing: false,
                                show_curves: false,
                                timeline_scroll_v: 0.0,
                                timeline_scroll_h: 0.0,
                                settings: Settings::default(),
                                active_layer_index: Some(0),
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
                        if ui.button("Duplicate Layer (Ctrl+D)").clicked() {
                            if let Some(idx) = comp.active_layer_index {
                                if idx < comp.layers.len() {
                                    let mut nl = comp.layers[idx].clone();
                                    nl.name = format!("{} Copy", nl.name);
                                    comp.layers.insert(idx + 1, nl);
                                    comp.active_layer_index = Some(idx + 1);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Delete Selected (Del)").clicked() {
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
                        ui.menu_button("UI Scale", |ui| {
                            for scale in [0.75, 1.0, 1.25, 1.5, 2.0] {
                                if ui.button(format!("{}x", scale)).clicked() {
                                    comp.settings.ui_scale = scale;
                                }
                            }
                        });
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
                                        kf.ease = Some(BezierControl {
                                            cp1: 0.33,
                                            cp2: 0.67,
                                        });
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
                                        kf.ease = Some(BezierControl {
                                            cp1: 0.15,
                                            cp2: 1.0,
                                        });
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
                                        kf.ease = Some(BezierControl {
                                            cp1: 0.85,
                                            cp2: 0.35,
                                        });
                                    }
                                }
                            }
                            ui.close_menu();
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
                                    });
                                    ui.separator();
                                },
                            );
                        }
                        1 => {
                            // EFFECT CONTROLS TAB
                            if let Some(active_idx) = comp.active_layer_index {
                                if let Some(layer) = comp.layers.get_mut(active_idx) {
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
                                    });

                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new("▼ Transform")
                                            .strong()
                                            .color(egui::Color32::from_gray(190)),
                                    );

                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        let prop_names = sorted_property_names(layer);
                                        for name in prop_names {
                                            if let Some(prop) = layer.properties.get_mut(&name) {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new(property_display_name(
                                                            &name,
                                                        ))
                                                        .size(11.0)
                                                        .color(egui::Color32::from_gray(180)),
                                                    );

                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(
                                                            egui::Align::Center,
                                                        ),
                                                        |ui| {
                                                            let speed =
                                                                if name.contains("scale")
                                                                    || name.contains("opacity")
                                                                {
                                                                    0.5
                                                                } else {
                                                                    0.25
                                                                };
                                                            ui.add(
                                                                egui::DragValue::new(
                                                                    &mut prop.base_value,
                                                                )
                                                                .speed(speed),
                                                            );
                                                        },
                                                    );
                                                });
                                            }
                                        }

                                        ui.separator();
                                        ui.label(
                                            egui::RichText::new("⚡ Effects Stack (0 active)")
                                                .color(egui::Color32::from_gray(140)),
                                        );
                                        if ui.button("+ Add Effect...").clicked() {
                                            // Effect selector
                                        }
                                    });
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
                                        - 1.0 / comp.settings.fps as f32)
                                        .max(0.0);
                                }
                                let play_text = if comp.is_playing {
                                    "⏸ Pause"
                                } else {
                                    "▶ Play"
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(play_text).strong(),
                                        )
                                        .min_size(egui::vec2(60.0, 24.0)),
                                    )
                                    .clicked()
                                {
                                    comp.is_playing = !comp.is_playing;
                                }
                                if ui.button("+1f ▶").clicked() {
                                    comp.current_time = (comp.current_time
                                        + 1.0 / comp.settings.fps as f32)
                                        .min(comp.settings.duration);
                                }
                                if ui.button("Last ▶|").clicked() {
                                    comp.current_time =
                                        comp.work_area_out.min(comp.settings.duration);
                                }
                            });

                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Frame: {} / {}",
                                        (comp.current_time * comp.settings.fps as f32).round()
                                            as u64,
                                        (comp.settings.duration * comp.settings.fps as f32).round()
                                            as u64
                                    ))
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(160)),
                                );
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
                                for (cat, effects) in categories {
                                    ui.collapsing(cat, |ui| {
                                        for eff in effects {
                                            ui.horizontal(|ui| {
                                                ui.label("fx");
                                                if ui.button(*eff).clicked() {
                                                    // Add effect to active layer
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

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut comp.show_guides, "⌗ Guides");
                            ui.checkbox(&mut comp.show_grid, "⊞ Grid");
                            ui.checkbox(&mut comp.show_checkerboard, "▦ Checkerboard");
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
                        ui.label(egui::RichText::new("Playback & Navigation:").strong());
                        ui.label("Space: Play / Pause");
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

            // Draw composition render
            draw_texture_ex(
                &render_target.texture,
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

            // Active layer transform bounding box in Viewport
            if let Some(active_idx) = comp.active_layer_index {
                if active_idx < comp.layers.len() {
                    let (ax, ay, x, y, _, _, _, _, sx, sy) =
                        layer_transform(&comp, active_idx, comp.current_time);
                    let scale_factor = screen_w / comp.settings.width as f32;
                    let layer_screen_x = screen_x + (x - ax) * scale_factor;
                    let layer_screen_y = screen_y + (y - ay) * scale_factor;
                    let box_w = 200.0 * sx * scale_factor;
                    let box_h = 200.0 * sy * scale_factor;

                    if box_w.abs() > 2.0 && box_h.abs() > 2.0 {
                        draw_rectangle_lines(
                            layer_screen_x,
                            layer_screen_y,
                            box_w,
                            box_h,
                            1.0,
                            Color::from_rgba(70, 150, 240, 180),
                        );

                        // Anchor Point Crosshair ⊕
                        let anchor_screen_x = screen_x + x * scale_factor;
                        let anchor_screen_y = screen_y + y * scale_factor;
                        draw_circle_lines(
                            anchor_screen_x,
                            anchor_screen_y,
                            5.0,
                            1.0,
                            Color::from_rgba(255, 200, 50, 220),
                        );
                        draw_line(
                            anchor_screen_x - 8.0,
                            anchor_screen_y,
                            anchor_screen_x + 8.0,
                            anchor_screen_y,
                            1.0,
                            Color::from_rgba(255, 200, 50, 220),
                        );
                        draw_line(
                            anchor_screen_x,
                            anchor_screen_y - 8.0,
                            anchor_screen_x,
                            anchor_screen_y + 8.0,
                            1.0,
                            Color::from_rgba(255, 200, 50, 220),
                        );
                    }
                }
            }
        }

        next_frame().await
    }
}
