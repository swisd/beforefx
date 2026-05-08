use crate::core::*;
use egui_macroquad::egui;

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

pub fn sorted_property_names(layer: &Layer) -> Vec<String> {
    let mut names: Vec<String> = layer.properties.keys().cloned().collect();
    // AE typical order: Anchor, Position, Scale, Rotation, Opacity
    let order = ["anchorX", "anchorY", "x", "y", "scaleX", "scaleY", "rotation", "opacity"];
    names.sort_by(|a, b| {
        let pos_a = order.iter().position(|&x| x == a).unwrap_or(99);
        let pos_b = order.iter().position(|&x| x == b).unwrap_or(99);
        pos_a.cmp(&pos_b).then(a.cmp(b))
    });
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
    Some((
        keyframe,
        format!("{} / {}", layer_name, selected.property_name),
    ))
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

fn draw_keyframe_editor(
    ui: &mut egui::Ui,
    comp: &mut Composition,
    selected: &mut Option<SelectedKeyframe>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Keyframe")
                .strong()
                .color(egui::Color32::from_gray(190)),
        );
        ui.separator();
        if ui.selectable_label(comp.show_curves, "📈").on_hover_text("Curve Editor").clicked() {
            comp.show_curves = !comp.show_curves;
        }
        ui.separator();

        if let Some(sel) = selected.clone() {
            if let Some((keyframe, label)) = selected_keyframe_mut(comp, &sel) {
                ui.label(egui::RichText::new(label).color(egui::Color32::from_rgb(145, 190, 220)));
                ui.separator();
                ui.label("Time");
                ui.add(
                    egui::DragValue::new(&mut keyframe.time)
                        .speed(0.02)
                        .range(0.0..=60.0),
                );
                ui.label("Value");
                ui.add(egui::DragValue::new(&mut keyframe.value).speed(0.5));

                let mut has_ease = keyframe.ease.is_some();
                if ui.checkbox(&mut has_ease, "Bezier").changed() {
                    keyframe.ease = if has_ease {
                        Some(BezierControl {
                            cp1: 0.33,
                            cp2: 0.67,
                        })
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
            ui.label(
                egui::RichText::new(
                    "Select a diamond to edit easing or press Delete to remove it.",
                )
                .color(egui::Color32::from_gray(125)),
            );
        }
    });

    if let Some(sel) = selected {
        if let Some(prop) = comp
            .layers
            .get_mut(sel.layer_index)
            .and_then(|layer| layer.properties.get_mut(&sel.property_name))
        {
            let edited_time = prop.keyframes.get(sel.keyframe_index).map(|kf| kf.time);
            prop.keyframes
                .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
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

pub fn draw_pro_ae_timeline(
    ui: &mut egui::Ui,
    comp: &mut Composition,
    selected: &mut Option<SelectedKeyframe>,
) {
    if selected
        .as_ref()
        .is_some_and(|sel| !selection_is_valid(comp, sel))
    {
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
    let timeline_max_width = 3000.0; // 30 seconds at 100 pps

    ui.allocate_ui(egui::vec2(ui.available_width(), total_h), |ui| {
        ui.horizontal(|ui| {
            // --- LEFT SIDE: LAYER LIST ---
            let scroll_v = egui::ScrollArea::vertical()
                .id_salt("timeline_layers_scroll")
                .vertical_scroll_offset(comp.timeline_scroll_v)
                .min_scrolled_height(total_h)
                .max_height(total_h)
                .show(ui, |ui| {
                    ui.allocate_ui(egui::vec2(label_width, 10000.0), |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(12.0);
                            for layer in &mut comp.layers {
                                ui.horizontal(|ui| {
                                    // --- LAYER SWITCHES ---
                                    ui.spacing_mut().item_spacing.x = 4.0;

                                    // Visibility (Eye icon - using "👁")
                                    let vis_text = if layer.visible { "👁" } else { " " };
                                    if ui.selectable_label(layer.visible, vis_text).clicked() {
                                        layer.visible = !layer.visible;
                                    }

                                    // Solo (S)
                                    let solo_text = if layer.solo { "S" } else { " " };
                                    let solo_color = if layer.solo { egui::Color32::from_rgb(255, 200, 0) } else { egui::Color32::from_gray(100) };
                                    if ui.add(egui::SelectableLabel::new(layer.solo, egui::RichText::new(solo_text).color(solo_color))).clicked() {
                                        layer.solo = !layer.solo;
                                    }

                                    // Lock (Padlock - using "🔒")
                                    let lock_text = if layer.locked { "🔒" } else { " " };
                                    if ui.selectable_label(layer.locked, lock_text).clicked() {
                                        layer.locked = !layer.locked;
                                    }

                                    // Shy (AE Shy icon - using "👤")
                                    let shy_text = if layer.shy { "👤" } else { " " };
                                    if ui.selectable_label(layer.shy, shy_text).clicked() {
                                        layer.shy = !layer.shy;
                                    }

                                    // Minimize/Collapse (Arrow icon)
                                    let arrow = if layer.collapsed { "▶" } else { "▼" };
                                    if ui.button(arrow).clicked() {
                                        layer.collapsed = !layer.collapsed;
                                    }

                                    ui.add_space(4.0);

                                    // --- LAYER NAME ---
                                    ui.label(
                                        egui::RichText::new(format!("{}", layer.name))
                                            .color(egui::Color32::from_rgb(165, 195, 220))
                                            .strong(),
                                    );

                                    ui.add_space(4.0);

                                    // Collapse / Continuous Rasterize (AE star icon - using "❂")
                                    let collapse_text = if layer.collapse { "*" } else { " " };
                                    if ui.selectable_label(layer.collapse, collapse_text).clicked() {
                                        layer.collapse = !layer.collapse;
                                    }

                                    // effect
                                    let fx_text = if layer.fx { "fx" } else { "  " };
                                    let fx_color = if layer.fx { egui::Color32::from_rgb(180, 140, 255) } else { egui::Color32::from_gray(100) };
                                    if ui.add(egui::SelectableLabel::new(layer.fx, egui::RichText::new(fx_text).color(fx_color))).clicked() {
                                        layer.fx = !layer.fx;
                                    }

                                    // 3d (AE Cube - using "⬚")
                                    let d3_text = if layer.d3 { "⬚" } else { " " };
                                    if ui.selectable_label(layer.d3, d3_text).clicked() {
                                        layer.d3 = !layer.d3;
                                    }

                                    // Motion Blur (AE circles - using "Ⓞ")
                                    let moblur_text = if layer.moblur { "O" } else { " " };
                                    let moblur_color = if layer.moblur { egui::Color32::from_rgb(100, 200, 255) } else { egui::Color32::from_gray(100) };
                                    if ui.add(egui::SelectableLabel::new(layer.moblur, egui::RichText::new(moblur_text).color(moblur_color))).clicked() {
                                        layer.moblur = !layer.moblur;
                                    }

                                    // Frame Blending (AE slashes - using "／")
                                    let frameblur_text = if layer.ff { "/" } else { " " };
                                    if ui.selectable_label(layer.ff, frameblur_text).clicked() {
                                        layer.ff = !layer.ff;
                                    }
                                });
                                if !layer.collapsed {
                                    for name in sorted_property_names(layer) {
                                        ui.horizontal(|ui| {
                                            ui.add_space(18.0);
                                            ui.label(
                                                egui::RichText::new(name.clone())
                                                    .size(12.0)
                                                    .color(if let Some(c) = comp.settings.property_colors.get(&name) {
                                                        egui::Color32::from_rgb(c[0], c[1], c[2])
                                                    } else {
                                                        egui::Color32::from_gray(155)
                                                    }),
                                            );
                                        });
                                    }
                                }
                            }
                        });
                    });
                });
            comp.timeline_scroll_v = scroll_v.state.offset.y;

            ui.separator();

            // --- RIGHT SIDE: KEYFRAMES ---
            let scroll_both = egui::ScrollArea::both()
                .id_salt("timeline_keyframes_scroll")
                .vertical_scroll_offset(comp.timeline_scroll_v)
                .horizontal_scroll_offset(comp.timeline_scroll_h)
                .min_scrolled_height(total_h)
                .max_height(total_h)
                .show(ui, |ui| {
                    let (res, painter) = ui.allocate_painter(
                        egui::vec2(timeline_max_width, 10000.0),
                        egui::Sense::click_and_drag(),
                    );
                    let rect = res.rect;
                    let mut clicked_selection = None;
                    let mut clicked_keyframe = false;

                    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(24, 24, 27));

                    for i in 0..300 {
                        let x = rect.left() + (i as f32 * (pps / 10.0));
                        let is_major = i % 10 == 0;
                        painter.line_segment(
                            [
                                egui::pos2(x, rect.top()),
                                egui::pos2(x, rect.top() + if is_major { 10.0 } else { 5.0 }),
                            ],
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
                        if layer.shy {
                            continue;
                        }
                        current_y += 18.0;

                        if layer.collapsed {
                            continue;
                        }

                        for property_name in sorted_property_names(layer) {
                            let Some(prop) = layer.properties.get(&property_name) else {
                                continue;
                            };

                            if comp.show_curves {
                                // --- CURVE EDITOR RENDER ---
                                let row_h = 60.0;
                                let row_center_y = current_y + row_h / 2.0;

                                painter.rect_filled(
                                    egui::Rect::from_min_max(
                                        egui::pos2(rect.left(), current_y),
                                        egui::pos2(rect.right(), current_y + row_h),
                                    ),
                                    0.0,
                                    egui::Color32::from_rgb(20, 20, 22),
                                );

                                painter.line_segment(
                                    [egui::pos2(rect.left(), row_center_y), egui::pos2(rect.right(), row_center_y)],
                                    egui::Stroke::new(0.5, egui::Color32::from_gray(40)),
                                );

                                // Find min/max values in keyframes for normalization within this row
                                let mut min_val = prop.base_value;
                                let mut max_val = prop.base_value;
                                for kf in &prop.keyframes {
                                    min_val = min_val.min(kf.value);
                                    max_val = max_val.max(kf.value);
                                }
                                let val_range = (max_val - min_val).max(1.0);

                                let to_screen_y = |val: f32| {
                                    let t = (val - min_val) / val_range;
                                    current_y + row_h - (t * (row_h - 10.0) + 5.0)
                                };

                                for (keyframe_index, curr) in prop.keyframes.iter().enumerate() {
                                    let x1 = rect.left() + (curr.time * pps);
                                    let y1 = to_screen_y(curr.value);

                                    if let Some(next) = prop.keyframes.get(keyframe_index + 1) {
                                        let x2 = rect.left() + (next.time * pps);
                                        let y2 = to_screen_y(next.value);

                                        let curve = curr.ease.unwrap_or(BezierControl { cp1: 0.33, cp2: 0.67 });

                                        // Draw the curve
                                        let p1 = egui::pos2(x1, y1);
                                        let p2 = egui::pos2(x1 + (x2 - x1) * curve.cp1, y1);
                                        let p3 = egui::pos2(x1 + (x2 - x1) * curve.cp2, y2);
                                        let p4 = egui::pos2(x2, y2);

                                        painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                                            points: [p1, p2, p3, p4],
                                            closed: false,
                                            fill: egui::Color32::TRANSPARENT,
                                            stroke: egui::epaint::PathStroke::new(1.5, egui::Color32::from_rgb(255, 210, 75)),
                                        }));

                                        // Draw Bezier handles if selected
                                        let is_selected = selected.as_ref().is_some_and(|sel| {
                                            sel.layer_index == layer_index && sel.property_name == property_name && sel.keyframe_index == keyframe_index
                                        });

                                        if is_selected {
                                            painter.line_segment([p1, p2], egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
                                            painter.circle_filled(p2, 3.0, egui::Color32::from_gray(200));

                                            let next_p1 = egui::pos2(x2, y2);
                                            let next_p2 = p3;
                                        painter.line_segment([next_p1, next_p2], egui::Stroke::new(1.0, egui::Color32::from_gray(100)));
                                            painter.circle_filled(next_p2, 3.0, egui::Color32::from_gray(200));
                                        }
                                    }

                                    // Draw keyframe point
                                    let is_selected = selected.as_ref().is_some_and(|sel| {
                                        sel.layer_index == layer_index && sel.property_name == property_name && sel.keyframe_index == keyframe_index
                                    });

                                    let dot_color = if is_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(226, 176, 45) };
                                    painter.circle_filled(egui::pos2(x1, y1), if is_selected { 4.0 } else { 3.0 }, dot_color);

                                    if res.clicked() {
                                        if let Some(pos) = res.interact_pointer_pos() {
                                            if pos.distance(egui::pos2(x1, y1)) <= 9.0 {
                                                clicked_keyframe = true;
                                                clicked_selection = Some(SelectedKeyframe {
                                                    layer_index,
                                                    property_name: property_name.clone(),
                                                    keyframe_index,
                                                });
                                            }
                                        }
                                    }
                                }

                                current_y += row_h;
                            } else {
                                // --- STANDARD KEYFRAME VIEW ---
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
                                    [
                                        egui::pos2(rect.left(), row_center_y),
                                        egui::pos2(rect.right(), row_center_y),
                                    ],
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
                                                egui::pos2(x2, row_center_y),
                                            ],
                                            closed: false,
                                            fill: egui::Color32::TRANSPARENT,
                                            stroke: egui::epaint::PathStroke::new(
                                                1.2,
                                                egui::Color32::from_gray(130),
                                            ),
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
                                                clicked_keyframe = true;
                                                clicked_selection = Some(SelectedKeyframe {
                                                    layer_index,
                                                    property_name: property_name.clone(),
                                                    keyframe_index,
                                                });
                                            }
                                        }
                                    }
                                }
                                current_y += 23.0;
                            }
                        }
                    }

                    if res.clicked() {
                        *selected = clicked_selection;
                    }

                    let cti_x = rect.left() + (comp.current_time * pps);
                    painter.line_segment(
                        [
                            egui::pos2(cti_x, rect.top()),
                            egui::pos2(cti_x, rect.bottom()),
                        ],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(72, 175, 232)),
                    );
                    painter.circle_filled(
                        egui::pos2(cti_x, rect.top() + 5.0),
                        5.0,
                        egui::Color32::from_rgb(72, 175, 232),
                    );

                    if res.dragged() && !clicked_keyframe {
                        if let Some(pos) = res.interact_pointer_pos() {
                            comp.current_time = ((pos.x - rect.left()) / pps).max(0.0);
                        }
                    }
                });
            comp.timeline_scroll_v = scroll_both.state.offset.y;
            comp.timeline_scroll_h = scroll_both.state.offset.x;
        });
    });
}
