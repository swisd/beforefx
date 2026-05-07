mod core;
mod ui_utils;

use crate::core::*;
use macroquad::prelude::*;
use egui_macroquad::egui;
use crate::ui_utils::{apply_after_effects_style, draw_pro_ae_timeline};

#[macroquad::main("BeforeFX - Pro")]
async fn main() {
    let mut comp = Composition {
        layers: vec![Layer {
            name: "creative.mov".into(),
            properties: [
                ("x".into(), Property { name: "x".into(), base_value: 400.0, keyframes: vec![] }),
                ("y".into(), Property { name: "y".into(), base_value: 300.0, keyframes: vec![] }),
            ].into(),
        }],
        current_time: 0.0,
        is_playing: false,
    };
    let mut selected_keyframe: Option<SelectedKeyframe> = None;

    let render_target = render_target(1920, 1080);
    render_target.texture.set_filter(FilterMode::Linear);
    let mut viewport_rect = egui::Rect::NOTHING;

    loop {
        if comp.is_playing { comp.current_time += get_frame_time(); }

        // --- 1. RENDER ANIMATION ---
        set_camera(&Camera2D {
            render_target: Some(render_target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0., 0., 1920., 1080.))
        });
        clear_background(Color::from_rgba(25, 25, 25, 255));
        for l in &comp.layers {
            let x = l.properties["x"].get_value_at(comp.current_time);
            let y = l.properties["y"].get_value_at(comp.current_time);
            draw_rectangle(x - 50.0, y - 50.0, 100.0, 100.0, WHITE);
        }
        set_default_camera();

        // --- 2. UI LAYOUT ---
        egui_macroquad::ui(|ctx| {
            apply_after_effects_style(ctx);

            // TOOLBAR (TOP)
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("BeforeFX").strong().color(egui::Color32::from_gray(210)));
                    ui.separator();
                    ui.selectable_label(true, "Selection");
                    ui.selectable_label(false, "Hand");
                    ui.selectable_label(false, "Pen");
                    ui.separator();
                    ui.label(egui::RichText::new("Workspace: Standard").color(egui::Color32::from_gray(150)));
                });
            });

            // INSPECTOR (LEFT)
            egui::SidePanel::left("inspector").resizable(true).default_width(360.0).show(ctx, |ui| {
                ui.label(egui::RichText::new("Effect Controls").strong().color(egui::Color32::from_gray(205)));
                ui.separator();
                for layer in &mut comp.layers {
                    ui.collapsing(&layer.name, |ui| {
                        for prop in layer.properties.values_mut() {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&prop.name).color(egui::Color32::from_gray(165)));
                                ui.add(egui::DragValue::new(&mut prop.base_value));
                                if ui.button("◆").clicked() {
                                    prop.keyframes.push(Keyframe {
                                        time: comp.current_time,
                                        value: prop.base_value,
                                        ease: Some(BezierControl { cp1: 0.33, cp2: 0.67 })
                                    });
                                    prop.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
                                }
                            });
                        }
                    });
                }
            });

            // TIMELINE (BOTTOM)
            egui::TopBottomPanel::bottom("timeline_panel")
                .resizable(true)
                .default_height(330.0)
                .show(ctx, |ui| {
                    // Panel A: Timecode & Transport
                    ui.horizontal(|ui| {
                        if ui.button(if comp.is_playing { "Pause" } else { "Play" }).clicked() {
                            comp.is_playing = !comp.is_playing;
                        }
                        ui.separator();

                        // Pro Timecode Format (00:00:00:00)
                        let frames = (comp.current_time * 60.0) as i32 % 60;
                        let secs = comp.current_time as i32 % 60;
                        let mins = (comp.current_time / 60.0) as i32;
                        let timecode = format!("{:02}:{:02}:{:02}", mins, secs, frames);

                        ui.label(egui::RichText::new(timecode).monospace().size(18.0).color(egui::Color32::from_rgb(100, 235, 180)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("60 FPS").color(egui::Color32::from_gray(145)));
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
        draw_texture_ex(
            &render_target.texture,
            viewport_rect.min.x,
            viewport_rect.min.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(viewport_rect.width(), viewport_rect.height())),
                ..Default::default()
            },
        );

        next_frame().await
    }
}
