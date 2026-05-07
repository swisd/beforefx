use egui_macroquad::egui;
use crate::core::*;

pub fn apply_after_effects_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 5.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = egui::Color32::from_rgb(35, 35, 38);
    style.visuals.panel_fill = egui::Color32::from_rgb(32, 32, 35);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(18, 18, 20);
    style.visuals.faint_bg_color = egui::Color32::from_rgb(43, 43, 47);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(32, 32, 35);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(48, 48, 52);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(61, 61, 66);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(75, 75, 82);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(42, 114, 170);
    ctx.set_style(style);
}

fn sorted_property_names(layer: &Layer) -> Vec<String> {
    let mut names: Vec<String> = layer.properties.keys().cloned().collect();
    names.sort();
    names
}

fn selected_keyframe_mut<'a>(
    comp: &'a mut Composition,
    selected: &SelectedKeyframe,
) -> Option<(&'a mut Keyframe, String)> {
    let layer = comp.layers.get_mut(selected.layer_index)?;
    let layer_name = layer.name.clone();
    let prop = layer.properties.get_mut(&selected.property_name)?;
    let keyframe = prop.keyframes.get_mut(selected.keyframe_index)?;
    Some((keyframe, format!("{} / {}", layer_name, selected.property_name)))
}

fn selection_is_valid(comp: &Composition, selected: &SelectedKeyframe) -> bool {
    comp.layers
        .get(selected.layer_index)
        .and_then(|layer| layer.properties.get(&selected.property_name))
        .and_then(|prop| prop.keyframes.get(selected.keyframe_index))
        .is_some()
}

fn delete_selected_keyframe(comp: &mut Composition, selected: &mut Option<SelectedKeyframe>) {
    if let Some(sel) = selected.clone() {
        if let Some(prop) = comp
            .layers
            .get_mut(sel.layer_index)
            .and_then(|layer| layer.properties.get_mut(&sel.property_name))
        {
            if sel.keyframe_index < prop.keyframes.len() {
                prop.keyframes.remove(sel.keyframe_index);
            }
        }
    }
    *selected = None;
}

fn draw_keyframe_editor(ui: &mut egui::Ui, comp: &mut Composition, selected: &mut Option<SelectedKeyframe>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Keyframe").strong().color(egui::Color32::from_gray(190)));

        if let Some(sel) = selected.clone() {
            if let Some((keyframe, label)) = selected_keyframe_mut(comp, &sel) {
                ui.label(egui::RichText::new(label).color(egui::Color32::from_rgb(145, 190, 220)));
                ui.separator();
                ui.label("Time");
                ui.add(egui::DragValue::new(&mut keyframe.time).speed(0.02).range(0.0..=60.0));
                ui.label("Value");
                ui.add(egui::DragValue::new(&mut keyframe.value).speed(0.5));

                let mut has_ease = keyframe.ease.is_some();
                if ui.checkbox(&mut has_ease, "Bezier").changed() {
                    keyframe.ease = if has_ease {
                        Some(BezierControl { cp1: 0.33, cp2: 0.67 })
                    } else {
                        None
                    };
                }

                if let Some(ease) = &mut keyframe.ease {
                    ui.label("In");
                    ui.add(egui::Slider::new(&mut ease.cp1, 0.0..=1.0).show_value(true));
                    ui.label("Out");
                    ui.add(egui::Slider::new(&mut ease.cp2, 0.0..=1.0).show_value(true));
                }

                if ui.button("Delete").clicked() {
                    delete_selected_keyframe(comp, selected);
                }
            } else {
                *selected = None;
                ui.label("No keyframe selected");
            }
        } else {
            ui.label(egui::RichText::new("Select a diamond to edit easing or press Delete to remove it.").color(egui::Color32::from_gray(125)));
        }
    });

    if let Some(sel) = selected {
        if let Some(prop) = comp
            .layers
            .get_mut(sel.layer_index)
            .and_then(|layer| layer.properties.get_mut(&sel.property_name))
        {
            let edited_time = prop
                .keyframes
                .get(sel.keyframe_index)
                .map(|kf| kf.time);
            prop.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
            if let Some(edited_time) = edited_time {
                if let Some(new_index) = prop
                    .keyframes
                    .iter()
                    .position(|kf| (kf.time - edited_time).abs() < f32::EPSILON)
                {
                    sel.keyframe_index = new_index;
                }
            }
        }
    }
}

pub fn draw_pro_ae_timeline(ui: &mut egui::Ui, comp: &mut Composition, selected: &mut Option<SelectedKeyframe>) {
    if selected.as_ref().is_some_and(|sel| !selection_is_valid(comp, sel)) {
        *selected = None;
    }

    if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
        delete_selected_keyframe(comp, selected);
    }

    draw_keyframe_editor(ui, comp, selected);
    ui.add_space(6.0);

    let total_h = ui.available_height();
    let label_width = 220.0;
    let pps = 100.0;

    ui.horizontal(|ui| {
        ui.allocate_ui(egui::vec2(label_width, total_h), |ui| {
            ui.vertical(|ui| {
                ui.add_space(20.0);
                for layer in &comp.layers {
                    ui.label(egui::RichText::new(format!("▼ {}", layer.name)).color(egui::Color32::from_rgb(165, 195, 220)).strong());
                    for name in sorted_property_names(layer) {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(egui::RichText::new(name).size(12.0).color(egui::Color32::from_gray(155)));
                        });
                    }
                }
            });
        });

        ui.separator();

        let (res, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());
        let rect = res.rect;
        let mut clicked_selection = None;

        painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 24, 27));

        for i in 0..100 {
            let x = rect.left() + (i as f32 * (pps / 10.0));
            let is_major = i % 10 == 0;
            painter.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.top() + if is_major { 10.0 } else { 5.0 })],
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            );
            if is_major {
                painter.line_segment(
                    [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                    egui::Stroke::new(0.5, egui::Color32::from_gray(35)),
                );
                painter.text(
                    egui::pos2(x + 2.0, rect.top() + 10.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}s", i / 10),
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_gray(120),
                );
            }
        }

        let mut current_y = rect.top() + 20.0;
        for (layer_index, layer) in comp.layers.iter().enumerate() {
            current_y += 18.0;
            for property_name in sorted_property_names(layer) {
                let Some(prop) = layer.properties.get(&property_name) else { continue };
                let row_center_y = current_y + 8.0;

                painter.rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.left(), row_center_y - 9.0),
                        egui::pos2(rect.right(), row_center_y + 9.0),
                    ),
                    0.0,
                    if (current_y as i32 / 18) % 2 == 0 {
                        egui::Color32::from_rgb(27, 27, 30)
                    } else {
                        egui::Color32::from_rgb(30, 30, 33)
                    },
                );
                painter.line_segment(
                    [egui::pos2(rect.left(), row_center_y), egui::pos2(rect.right(), row_center_y)],
                    egui::Stroke::new(0.5, egui::Color32::from_gray(48)),
                );

                for (keyframe_index, curr) in prop.keyframes.iter().enumerate() {
                    let x1 = rect.left() + (curr.time * pps);
                    if let Some(next) = prop.keyframes.get(keyframe_index + 1) {
                        let x2 = rect.left() + (next.time * pps);
                        let curve = curr.ease.unwrap_or(BezierControl { cp1: 0.4, cp2: 0.6 });
                        painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                            points: [
                                egui::pos2(x1, row_center_y),
                                egui::pos2(x1 + (x2 - x1) * curve.cp1, row_center_y - 12.0),
                                egui::pos2(x1 + (x2 - x1) * curve.cp2, row_center_y + 12.0),
                                egui::pos2(x2, row_center_y)
                            ],
                            closed: false,
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::epaint::PathStroke::new(1.2, egui::Color32::from_gray(130)),
                        }));
                    }

                    let is_selected = selected.as_ref().is_some_and(|sel| {
                        sel.layer_index == layer_index
                            && sel.property_name == property_name
                            && sel.keyframe_index == keyframe_index
                    });
                    let diamond_size = if is_selected { 6.0 } else { 4.5 };
                    let color = if is_selected {
                        egui::Color32::from_rgb(255, 210, 75)
                    } else {
                        egui::Color32::from_rgb(226, 176, 45)
                    };
                    let diamond = vec![
                        egui::pos2(x1, row_center_y - diamond_size),
                        egui::pos2(x1 + diamond_size, row_center_y),
                        egui::pos2(x1, row_center_y + diamond_size),
                        egui::pos2(x1 - diamond_size, row_center_y),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        diamond,
                        color,
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 60, 20)),
                    ));

                    if res.clicked() {
                        if let Some(pos) = res.interact_pointer_pos() {
                            if pos.distance(egui::pos2(x1, row_center_y)) <= 9.0 {
                                clicked_selection = Some(SelectedKeyframe {
                                    layer_index,
                                    property_name: property_name.clone(),
                                    keyframe_index,
                                });
                            }
                        }
                    }
                }
                current_y += 18.0;
            }
        }

        if res.clicked() {
            *selected = clicked_selection;
        }

        let cti_x = rect.left() + (comp.current_time * pps);
        painter.line_segment(
            [egui::pos2(cti_x, rect.top()), egui::pos2(cti_x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(72, 175, 232)),
        );
        painter.circle_filled(egui::pos2(cti_x, rect.top() + 5.0), 5.0, egui::Color32::from_rgb(72, 175, 232));

        if res.dragged() && clicked_selection.is_none() {
            if let Some(pos) = res.interact_pointer_pos() {
                comp.current_time = ((pos.x - rect.left()) / pps).max(0.0);
            }
        }
    });
}
