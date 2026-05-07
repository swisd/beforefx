mod core;
mod ui_utils;

use crate::core::*;
use crate::ui_utils::{apply_after_effects_style, draw_pro_ae_timeline, sorted_property_names};
use egui_macroquad::egui;
use macroquad::prelude::*;
use std::collections::HashMap;

fn create_default_properties() -> HashMap<String, Property> {
    [
        ("anchorX", 0.0),
        ("anchorY", 0.0),
        ("x", 400.0),
        ("y", 300.0),
        ("rotation", 0.0),
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

#[macroquad::main("BeforeFX - Pro")]
async fn main() {
    let mut comp = Composition {
        layers: vec![Layer {
            name: "creative.mov".into(),
            source: LayerSource::Solid { color: [1.0, 1.0, 1.0, 1.0] },
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
        }],
        resources: vec![],
        current_time: 0.0,
        is_playing: false,
        show_curves: false,
        settings: Settings::default(),
    };
    let mut selected_keyframe: Option<SelectedKeyframe> = None;
    let mut textures: HashMap<String, Texture2D> = HashMap::new();
    let mut to_load: Vec<String> = vec![];

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
        // Check if any layer needs texture loading
        for l in &comp.layers {
            if let LayerSource::Image { path } = &l.source {
                if !textures.contains_key(path) {
                    to_load.push(path.clone());
                }
            }
        }

        if comp.is_playing {
            comp.current_time += get_frame_time();
        }

        // --- 1. RENDER ANIMATION ---
        set_camera(&Camera2D {
            render_target: Some(render_target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0., 0., 1920., 1080.))
        });
        clear_background(Color::from_rgba(25, 25, 25, 255));
        for l in &comp.layers {
            if !l.visible { continue; }
            let ax = l.properties["anchorX"].get_value_at(comp.current_time);
            let ay = l.properties["anchorY"].get_value_at(comp.current_time);
            let x = l.properties["x"].get_value_at(comp.current_time);
            let y = l.properties["y"].get_value_at(comp.current_time);
            let rot = l.properties["rotation"].get_value_at(comp.current_time);
            let sx = l.properties["scaleX"].get_value_at(comp.current_time) / 100.0;
            let sy = l.properties["scaleY"].get_value_at(comp.current_time) / 100.0;
            let op = l.properties["opacity"].get_value_at(comp.current_time) / 100.0;

            match &l.source {
                LayerSource::Solid { color } => {
                    let c = Color::new(color[0], color[1], color[2], color[3] * op);
                    let w = 100.0 * sx;
                    let h = 100.0 * sy;
                    draw_rectangle_ex(
                        x,
                        y,
                        w,
                        h,
                        DrawRectangleParams {
                            offset: vec2(ax / 100.0, ay / 100.0), // Guessing offset is 0..1? Check docs if possible.
                            rotation: rot.to_radians(),
                            color: c,
                        },
                    );
                }
                LayerSource::Image { path } => {
                    if let Some(tex) = textures.get(path) {
                        let w = tex.width() * sx;
                        let h = tex.height() * sy;
                        draw_texture_ex(
                            tex,
                            x,
                            y,
                            Color::new(1.0, 1.0, 1.0, op),
                            DrawTextureParams {
                                dest_size: Some(vec2(w, h)),
                                rotation: rot.to_radians(),
                                pivot: Some(vec2(x + ax, y + ay)),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }
        set_default_camera();

        // --- 2. UI LAYOUT ---
        egui_macroquad::ui(|ctx| {
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
                                if let Ok(json) = std::fs::read_to_string(path) {
                                    if let Ok(new_comp) = serde_json::from_str::<Composition>(&json) {
                                        comp = new_comp;
                                        // Textures will be picked up by the loading logic in the loop
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
                    });

                    ui.menu_button("Layer", |ui| {
                        if ui.button("Import Image...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Images", &["png", "jpg", "jpeg", "bmp"])
                                .pick_file()
                            {
                                let path_str = path.to_string_lossy().to_string();
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                comp.resources.push(Resource {
                                    name: name.clone(),
                                    path: path_str.clone(),
                                });
                                comp.layers.push(Layer {
                                    name,
                                    source: LayerSource::Image { path: path_str.clone() },
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
                                });
                            }
                            ui.close_menu();
                        }
                        if ui.button("New Solid...").clicked() {
                             comp.layers.push(Layer {
                                name: "Solid 1".into(),
                                source: LayerSource::Solid { color: [1.0, 0.0, 0.0, 1.0] },
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
                            });
                            ui.close_menu();
                        }
                    });

                    ui.separator();
                    ui.menu_button("Edit", |ui| {
                        ui.menu_button("Property Colors", |ui| {
                            let mut keys: Vec<_> = comp.settings.property_colors.keys().cloned().collect();
                            keys.sort();
                            for key in keys {
                                ui.horizontal(|ui| {
                                    ui.label(&key);
                                    let color = comp.settings.property_colors.get_mut(&key).unwrap();
                                    let mut egui_color = egui::Color32::from_rgb(color[0], color[1], color[2]);
                                    if ui.color_edit_button_srgba(&mut egui_color).changed() {
                                        *color = [egui_color.r(), egui_color.g(), egui_color.b()];
                                    }
                                });
                            }
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
                                ui.label("🖼");
                                ui.label(&res.name);
                            });
                        }
                        if comp.resources.is_empty() {
                            ui.label(egui::RichText::new("No resources imported").color(egui::Color32::from_gray(100)));
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
                                            ui.label(
                                                egui::RichText::new(&prop.name)
                                                    .color(if let Some(c) = comp.settings.property_colors.get(&prop.name) {
                                                        egui::Color32::from_rgb(c[0], c[1], c[2])
                                                    } else {
                                                        egui::Color32::from_gray(165)
                                                    }),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button("⏱").on_hover_text("Add Keyframe").clicked() {
                                                    prop.keyframes.push(Keyframe {
                                                        time: comp.current_time,
                                                        value: prop.base_value,
                                                        ease: Some(BezierControl {
                                                            cp1: 0.33,
                                                            cp2: 0.67,
                                                        }),
                                                    });
                                                    prop.keyframes
                                                        .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
                                                }
                                                ui.add(
                                                    egui::DragValue::new(&mut prop.base_value)
                                                        .speed(if prop.name.contains("scale") { 1.0 } else { 0.1 }),
                                                );
                                            });
                                        });
                                    }
                                }
                            });
                        }
                    });
                });

            // TIMELINE (BOTTOM)
            egui::TopBottomPanel::bottom("timeline_panel")
                .resizable(true)
                .default_height(330.0)
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
                        let frames = (comp.current_time * 60.0) as i32 % 60;
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
                                egui::RichText::new("60 FPS").color(egui::Color32::from_gray(145)),
                            );
                        });
                    });

                    ui.separator();

                    // The professional split-view timeline
                    draw_pro_ae_timeline(ui, &mut comp, &mut selected_keyframe);
                });

            // VIEWPORT SLOT (CENTER)
            egui::CentralPanel::default().show(ctx, |ui| {
                viewport_rect = ui.available_rect_before_wrap();
            });
        });

        // --- 3. FINAL COMPOSITE ---
        egui_macroquad::draw(); // Draw egui first

        // Draw Macroquad texture directly over the egui CentralPanel "hole"
        // Maintain aspect ratio (16:9)
        let target_aspect = 1920.0 / 1080.0;
        let viewport_aspect = viewport_rect.width() / viewport_rect.height();

        let (draw_w, draw_h) = if viewport_aspect > target_aspect {
            // Viewport is wider than target
            (viewport_rect.height() * target_aspect, viewport_rect.height())
        } else {
            // Viewport is taller than target
            (viewport_rect.width(), viewport_rect.width() / target_aspect)
        };

        let draw_x = viewport_rect.min.x + (viewport_rect.width() - draw_w) / 2.0;
        let draw_y = viewport_rect.min.y + (viewport_rect.height() - draw_h) / 2.0;

        draw_texture_ex(
            &render_target.texture,
            draw_x,
            draw_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(draw_w, draw_h)),
                flip_y: true,
                ..Default::default()
            },
        );

        next_frame().await
    }
}
