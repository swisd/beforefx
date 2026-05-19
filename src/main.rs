mod core;
mod ui_utils;

use crate::core::*;
use crate::ui_utils::{apply_after_effects_style, draw_pro_ae_timeline, sorted_property_names};
use egui_macroquad::egui;
use macroquad::audio::{PlaySoundParams, Sound, load_sound, play_sound, stop_sound};
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use playa_ffmpeg::{self as ffmpeg, dict};
use playa_ffmpeg::format::Pixel;
use playa_ffmpeg::util::frame::Video;
use playa_ffmpeg::software::scaling::{Context as ScalerContext, flag::Flags};

fn create_default_properties() -> HashMap<String, Property> {
    [
        ("anchorX", 0.0),
        ("anchorY", 0.0),
        ("x", 400.0),
        ("y", 300.0),
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

fn default_layer(name: String, source: LayerSource) -> Layer {
    Layer {
        name,
        source,
        properties: create_default_properties(),
        visible: true,
        locked: false,
        solo: false,
        fx: false,
        d3: false,
        ff: false,
        moblur: false,
        shy: false,
        collapse: false,
        collapsed: false,
    }
}

fn resource_icon(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Image => "IMG",
        ResourceKind::Audio => "AUD",
        ResourceKind::Video => "VID",
        ResourceKind::Model3D => "3D",
    }
}

fn add_resource(comp: &mut Composition, name: String, path: String, kind: ResourceKind) {
    comp.resources.push(Resource { name, path, kind });
}

fn layer_transform(layer: &Layer, time: f32) -> (f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) {
    let ax = layer.properties["anchorX"].get_value_at(time);
    let ay = layer.properties["anchorY"].get_value_at(time);
    let x = layer.properties["x"].get_value_at(time);
    let y = layer.properties["y"].get_value_at(time);
    let z = layer
        .properties
        .get("z")
        .map_or(0.0, |p| p.get_value_at(time));
    let rot = layer.properties["rotation"].get_value_at(time);
    let rot_x = layer
        .properties
        .get("rotationX")
        .map_or(0.0, |p| p.get_value_at(time));
    let rot_y = layer
        .properties
        .get("rotationY")
        .map_or(0.0, |p| p.get_value_at(time));
    let sx = layer.properties["scaleX"].get_value_at(time) / 100.0;
    let sy = layer.properties["scaleY"].get_value_at(time) / 100.0;
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
            Color::new(1.0, 1.0, 1.0, color.a.min(0.35)),
        );
    }
}

fn draw_video_placeholder(path: &str, x: f32, y: f32, sx: f32, sy: f32, opacity: f32) {
    let w = 360.0 * sx.abs().max(0.1);
    let h = 202.5 * sy.abs().max(0.1);
    draw_rectangle(x, y, w, h, Color::new(0.03, 0.035, 0.04, opacity));
    draw_rectangle_lines(x, y, w, h, 3.0, Color::new(0.25, 0.55, 0.75, opacity));
    let label = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "video".to_string());
    draw_text(
        "VIDEO",
        x + 18.0,
        y + 42.0,
        34.0,
        Color::new(0.65, 0.85, 1.0, opacity),
    );
    draw_text(
        &label,
        x + 18.0,
        y + h - 24.0,
        24.0,
        Color::new(0.8, 0.85, 0.9, opacity),
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
    clear_background(Color::from_rgba(25, 25, 25, 255));

    for layer in &comp.layers {
        if !layer.visible {
            continue;
        }
        let (ax, ay, x, y, z, rot, rot_x, rot_y, sx, sy) = layer_transform(layer, time);
        let op = layer.properties["opacity"].get_value_at(time) / 100.0;

        match &layer.source {
            LayerSource::Solid { color } => {
                draw_rectangle_ex(
                    x,
                    y,
                    100.0 * sx,
                    100.0 * sy,
                    DrawRectangleParams {
                        offset: vec2(ax / 100.0, ay / 100.0),
                        rotation: rot.to_radians(),
                        color: Color::new(color[0], color[1], color[2], color[3] * op),
                    },
                );
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
            LayerSource::Audio { .. } => {}
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
                    120.0 * sx.abs().max(0.05),
                    120.0 * sy.abs().max(0.05),
                    120.0 * ((sx.abs() + sy.abs()) * 0.5).max(0.05),
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

fn first_audio_path(comp: &Composition) -> Option<String> {
    comp.layers.iter().find_map(|layer| match &layer.source {
        LayerSource::Audio { path } => Some(path.clone()),
        _ => None,
    })
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
        let mut stream = out_ctx.add_stream(ffmpeg::codec::encoder::find(ffmpeg::codec::Id::H264))?;
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

        // Flush encoder
        encoder.send_eof()?;
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(0);
            packet.write_interleaved(&mut out_ctx)?;
        }

        out_ctx.write_trailer()?;
        Ok(())
    })();

    match result {
        Ok(_) => format!("Exported {} using playa-ffmpeg", output_path.display()),
        Err(e) => format!("Failed to export video via crate: {}", e),
    }
}

#[macroquad::main("BeforeFX - Pro")]
async fn main() {
    let mut comp = Composition {
        layers: vec![default_layer(
            "Solid 0".into(),
            LayerSource::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
        )],
        resources: vec![],
        current_time: 0.0,
        is_playing: false,
        show_curves: false,
        timeline_scroll_v: 0.0,
        timeline_scroll_h: 0.0,
        settings: Settings::default(),
    };
    let mut selected_keyframe: Option<SelectedKeyframe> = None;
    let mut textures: HashMap<String, Texture2D> = HashMap::new();
    let mut sounds: HashMap<String, Sound> = HashMap::new();
    let mut to_load: Vec<String> = vec![];
    let mut to_load_audio: Vec<String> = vec![];
    let mut audio_started = false;
    let mut pending_export: Option<PathBuf> = None;
    let mut export_status = String::new();

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
        // Check if any layer needs texture loading
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

        if comp.is_playing {
            comp.current_time += get_frame_time();
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

        // --- Keyboard Shortcuts for Scaling ---
        if is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl) {
            if is_key_pressed(KeyCode::Equal) {
                // Plus key
                comp.settings.ui_scale = (comp.settings.ui_scale + 0.1).min(2.5);
            }
            if is_key_pressed(KeyCode::Minus) {
                comp.settings.ui_scale = (comp.settings.ui_scale - 0.1).max(0.5);
            }
            if is_key_pressed(KeyCode::Key0) {
                comp.settings.ui_scale = 1.0;
            }
        }

        // --- 1. RENDER ANIMATION ---
        draw_composition(&comp, &textures, comp.current_time, render_target.clone());
        clear_background(Color::from_rgba(20, 20, 20, 255));

        // --- 2. UI LAYOUT ---
        egui_macroquad::ui(|ctx| {
            ctx.set_pixels_per_point(comp.settings.ui_scale);
            apply_after_effects_style(ctx);

            // TOOLBAR (TOP)
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Save Project").clicked() {
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
                        if ui.button("Open Project").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("BeforeFX Project", &["bfx"])
                                .pick_file()
                            {
                                match std::fs::read_to_string(&path) {
                                    Ok(json) => {
                                        match serde_json::from_str::<Composition>(&json) {
                                            Ok(new_comp) => {
                                                comp = new_comp;
                                                textures.clear(); // Clear existing textures to avoid memory bloat
                                                sounds.clear();
                                                println!("Project loaded: {:?}", path);
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to parse project: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to read project file: {}", e);
                                    }
                                }
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Export Frame...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("PNG Image", &["png"])
                                .set_file_name("render.png")
                                .save_file()
                            {
                                let image = render_target.texture.get_texture_data();
                                image.export_png(path.to_str().unwrap());
                            }
                            ui.close_menu();
                        }
                        if ui.button("Export Video...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("MP4 Video", &["mp4"])
                                .set_file_name("render.mp4")
                                .save_file()
                            {
                                pending_export = Some(path);
                                export_status = "Rendering video frames...".to_string();
                            }
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Layer", |ui| {
                        if ui.button("Import Image...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                .pick_file()
                            {
                                let path_str = path.to_string_lossy().to_string();
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                println!("Importing image: {}", path_str);
                                add_resource(
                                    &mut comp,
                                    name.clone(),
                                    path_str.clone(),
                                    ResourceKind::Image,
                                );
                                comp.layers.push(default_layer(
                                    name,
                                    LayerSource::Image {
                                        path: path_str.clone(),
                                    },
                                ));
                            }
                            ui.close_menu();
                        }
                        if ui.button("Import Audio...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Audio", &["wav", "ogg", "mp3"])
                                .pick_file()
                            {
                                let path_str = path.to_string_lossy().to_string();
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                add_resource(
                                    &mut comp,
                                    name.clone(),
                                    path_str.clone(),
                                    ResourceKind::Audio,
                                );
                                comp.layers.push(default_layer(
                                    name,
                                    LayerSource::Audio { path: path_str },
                                ));
                            }
                            ui.close_menu();
                        }
                        if ui.button("Import Video...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Video", &["mp4", "mov", "webm", "avi", "mkv"])
                                .pick_file()
                            {
                                let path_str = path.to_string_lossy().to_string();
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                add_resource(
                                    &mut comp,
                                    name.clone(),
                                    path_str.clone(),
                                    ResourceKind::Video,
                                );
                                comp.layers.push(default_layer(
                                    name,
                                    LayerSource::Video { path: path_str },
                                ));
                            }
                            ui.close_menu();
                        }
                        if ui.button("Import 3D Object...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("3D Objects", &["obj", "gltf", "glb"])
                                .pick_file()
                            {
                                let path_str = path.to_string_lossy().to_string();
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                add_resource(
                                    &mut comp,
                                    name.clone(),
                                    path_str.clone(),
                                    ResourceKind::Model3D,
                                );
                                let mut layer = default_layer(
                                    name,
                                    LayerSource::Object3D {
                                        path: Some(path_str),
                                        color: [0.25, 0.55, 0.95, 1.0],
                                    },
                                );
                                layer.d3 = true;
                                comp.layers.push(layer);
                            }
                            ui.close_menu();
                        }
                        if ui.button("New Solid...").clicked() {
                            comp.layers.push(default_layer(
                                format!("Solid {}", comp.layers.len()),
                                LayerSource::Solid {
                                    color: [1.0, 0.0, 0.0, 1.0],
                                },
                            ));
                            ui.close_menu();
                        }
                        if ui.button("New Polygon").clicked() {
                            comp.layers.push(default_layer(
                                format!("Polygon {}", comp.layers.len()),
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
                            ));
                            ui.close_menu();
                        }
                        if ui.button("New 3D Cube").clicked() {
                            let mut layer = default_layer(
                                format!("Cube {}", comp.layers.len()),
                                LayerSource::Object3D {
                                    path: None,
                                    color: [0.25, 0.55, 0.95, 1.0],
                                },
                            );
                            layer.d3 = true;
                            comp.layers.push(layer);
                            ui.close_menu();
                        }
                    });

                    ui.separator();
                    ui.menu_button("Edit", |ui| {
                        ui.menu_button("UI Scale", |ui| {
                            if ui.button("0.5x").clicked() {
                                comp.settings.ui_scale = 0.5;
                            }
                            if ui.button("0.75x").clicked() {
                                comp.settings.ui_scale = 0.75;
                            }
                            if ui.button("1.0x").clicked() {
                                comp.settings.ui_scale = 1.0;
                            }
                            if ui.button("1.25x").clicked() {
                                comp.settings.ui_scale = 1.25;
                            }
                            if ui.button("1.5x").clicked() {
                                comp.settings.ui_scale = 1.5;
                            }
                            if ui.button("2.0x").clicked() {
                                comp.settings.ui_scale = 2.0;
                            }
                            ui.separator();
                            ui.add(
                                egui::Slider::new(&mut comp.settings.ui_scale, 0.5..=2.5)
                                    .text("Custom"),
                            );
                        });
                        ui.menu_button("Property Colors", |ui| {
                            let mut keys: Vec<_> =
                                comp.settings.property_colors.keys().cloned().collect();
                            keys.sort();
                            for key in keys {
                                ui.horizontal(|ui| {
                                    ui.label(&key);
                                    let color =
                                        comp.settings.property_colors.get_mut(&key).unwrap();
                                    let mut egui_color =
                                        egui::Color32::from_rgb(color[0], color[1], color[2]);
                                    if ui.color_edit_button_srgba(&mut egui_color).changed() {
                                        *color = [egui_color.r(), egui_color.g(), egui_color.b()];
                                    }
                                });
                            }
                        });
                        ui.separator();
                        ui.label("Composition");
                        ui.horizontal(|ui| {
                            ui.label("Duration");
                            ui.add(
                                egui::DragValue::new(&mut comp.settings.duration)
                                    .speed(0.25)
                                    .range(0.1..=600.0),
                            );
                            ui.label("FPS");
                            ui.add(
                                egui::DragValue::new(&mut comp.settings.fps)
                                    .speed(1)
                                    .range(1..=120),
                            );
                        });
                    });
                    ui.separator();
                    ui.label(
                        egui::RichText::new("BeforeFX")
                            .strong()
                            .color(egui::Color32::from_gray(210)),
                    );
                    ui.separator();
                    let _ = ui.selectable_label(true, "Selection");
                    let _ = ui.selectable_label(false, "Hand");
                    let _ = ui.selectable_label(false, "Pen");
                    ui.separator();
                    ui.label(
                        egui::RichText::new("Workspace: Standard")
                            .color(egui::Color32::from_gray(150)),
                    );
                });
            });

            // PROJECT / RESOURCE EXPLORER (LEFT)
            egui::SidePanel::left("project_explorer")
                .resizable(true)
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Project")
                            .strong()
                            .color(egui::Color32::from_gray(205)),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for res in &comp.resources {
                            ui.horizontal(|ui| {
                                ui.label(resource_icon(res.kind));
                                ui.label(&res.name);
                            });
                        }
                        if comp.resources.is_empty() {
                            ui.label(
                                egui::RichText::new("No resources imported")
                                    .color(egui::Color32::from_gray(100)),
                            );
                        }
                    });
                });

            // INSPECTOR (RIGHT)
            egui::SidePanel::right("inspector")
                .resizable(true)
                .default_width(320.0)
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Effect Controls / Properties")
                            .strong()
                            .color(egui::Color32::from_gray(205)),
                    );
                    ui.separator();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for layer in &mut comp.layers {
                            let layer_name = layer.name.clone();
                            let names = sorted_property_names(layer);
                            ui.collapsing(layer_name, |ui| {
                                // AE style properties formatting
                                for name in names {
                                    if let Some(prop) = layer.properties.get_mut(&name) {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&prop.name).color(
                                                if let Some(c) =
                                                    comp.settings.property_colors.get(&prop.name)
                                                {
                                                    egui::Color32::from_rgb(c[0], c[1], c[2])
                                                } else {
                                                    egui::Color32::from_gray(165)
                                                },
                                            ));
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .button("⏱")
                                                        .on_hover_text("Add Keyframe")
                                                        .clicked()
                                                    {
                                                        prop.keyframes.push(Keyframe {
                                                            time: comp.current_time,
                                                            value: prop.base_value,
                                                            ease: Some(BezierControl {
                                                                cp1: 0.33,
                                                                cp2: 0.67,
                                                            }),
                                                        });
                                                        prop.keyframes.sort_by(|a, b| {
                                                            a.time.partial_cmp(&b.time).unwrap()
                                                        });
                                                    }
                                                    ui.add(
                                                        egui::DragValue::new(&mut prop.base_value)
                                                            .speed(
                                                                if prop.name.contains("scale") {
                                                                    1.0
                                                                } else {
                                                                    0.1
                                                                },
                                                            ),
                                                    );
                                                },
                                            );
                                        });
                                    }
                                }
                            });
                        }
                    });
                });

            // TIMELINE (BOTTOM)
            let screen_height = ctx.screen_rect().height();
            egui::TopBottomPanel::bottom("timeline_panel_v4")
                .resizable(true)
                .default_height(screen_height / 3.0)
                .height_range(screen_height / 3.0..=1000.0)
                .show(ctx, |ui| {
                    // Panel A: Timecode & Transport
                    ui.horizontal(|ui| {
                        if ui
                            .button(if comp.is_playing { "Pause" } else { "Play" })
                            .clicked()
                        {
                            comp.is_playing = !comp.is_playing;
                        }
                        ui.separator();

                        // Pro Timecode Format (00:00:00:00)
                        let frames = (comp.current_time * comp.settings.fps as f32) as i32
                            % comp.settings.fps as i32;
                        let secs = comp.current_time as i32 % 60;
                        let mins = (comp.current_time / 60.0) as i32;
                        let timecode = format!("{:02}:{:02}:{:02}", mins, secs, frames);

                        ui.label(
                            egui::RichText::new(timecode)
                                .monospace()
                                .size(18.0)
                                .color(egui::Color32::from_rgb(100, 235, 180)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("{} FPS", comp.settings.fps))
                                    .color(egui::Color32::from_gray(145)),
                            );
                            if !export_status.is_empty() {
                                ui.label(
                                    egui::RichText::new(&export_status)
                                        .color(egui::Color32::from_rgb(145, 190, 220)),
                                );
                            }
                        });
                    });

                    ui.separator();

                    // The professional split-view timeline
                    draw_pro_ae_timeline(ui, &mut comp, &mut selected_keyframe);
                });

            // VIEWPORT SLOT (CENTER)
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::default()
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::NONE),
                )
                .show(ctx, |ui| {
                    viewport_rect = ui.max_rect();
                });
        });

        if let Some(path) = pending_export.take() {
            export_status = export_video(&comp, &textures, render_target.clone(), &path);
        }

        // --- 3. FINAL COMPOSITE ---
        egui_macroquad::draw(); // Draw egui first

        // Ensure we are using the default camera for screen-space drawing
        set_default_camera();

        // Draw Macroquad texture directly over the egui CentralPanel "hole"
        if viewport_rect.width() > 0.0 && viewport_rect.height() > 0.0 {
            // Maintain aspect ratio (16:9)
            let target_aspect = comp.settings.width as f32 / comp.settings.height as f32;
            let viewport_aspect = viewport_rect.width() / viewport_rect.height();

            let (draw_w, draw_h) = if viewport_aspect > target_aspect {
                // Viewport is wider than target
                (
                    viewport_rect.height() * target_aspect,
                    viewport_rect.height(),
                )
            } else {
                // Viewport is taller than target
                (viewport_rect.width(), viewport_rect.width() / target_aspect)
            };

            let ppp = comp.settings.ui_scale;
            let draw_x = viewport_rect.min.x + (viewport_rect.width() - draw_w) / 2.0;
            let draw_y = viewport_rect.min.y + (viewport_rect.height() - draw_h) / 2.0;

            draw_texture_ex(
                &render_target.texture,
                draw_x * ppp,
                draw_y * ppp,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(draw_w * ppp, draw_h * ppp)),
                    flip_y: true,
                    ..Default::default()
                },
            );
        }

        next_frame().await
    }
}
