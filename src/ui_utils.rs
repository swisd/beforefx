use crate::core::*;
use egui_macroquad::egui;

pub const AE_LABEL_COLORS: &[(&str, [u8; 3])] = &[
    ("Red", [220, 60, 60]),
    ("Yellow", [230, 185, 40]),
    ("Aqua", [50, 180, 160]),
    ("Pink", [235, 75, 125]),
    ("Lavender", [160, 100, 220]),
    ("Peach", [235, 130, 60]),
    ("Seafoam", [70, 180, 130]),
    ("Blue", [50, 120, 220]),
    ("Green", [60, 175, 75]),
    ("Purple", [120, 80, 200]),
    ("Orange", [230, 100, 30]),
    ("Brown", [150, 110, 80]),
    ("Magenta", [210, 50, 140]),
    ("Cyan", [40, 170, 220]),
    ("Sand", [210, 180, 120]),
    ("Dark Green", [30, 110, 55]),
];

pub fn get_label_color(index: usize) -> egui::Color32 {
    let color = AE_LABEL_COLORS[index % AE_LABEL_COLORS.len()].1;
    egui::Color32::from_rgb(color[0], color[1], color[2])
}

pub fn apply_after_effects_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(6.0, 3.0);
    style.spacing.window_margin = egui::Margin::same(4);
    
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = egui::Color32::from_rgb(33, 33, 36);
    visuals.panel_fill = egui::Color32::from_rgb(28, 28, 31);
    visuals.extreme_bg_color = egui::Color32::from_rgb(18, 18, 20);
    visuals.faint_bg_color = egui::Color32::from_rgb(38, 38, 42);
    
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(28, 28, 31);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 50));
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(185));
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(2);

    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(42, 42, 46);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 52, 58));
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(205));
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(2);

    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(55, 55, 62);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(75, 75, 85));
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(2);

    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(38, 112, 180);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 150, 230));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(2);

    visuals.selection.bg_fill = egui::Color32::from_rgb(30, 95, 165);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(70, 150, 230));

    style.visuals = visuals;
    ctx.set_style(style);
}

pub fn format_timecode(time: f32, fps: u32) -> String {
    let safe_fps = fps.max(1) as f32;
    let total_frames = (time.max(0.0) * safe_fps).round() as u64;
    let frames = total_frames % (fps.max(1) as u64);
    let total_seconds = total_frames / (fps.max(1) as u64);
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;
    format!("{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, frames)
}

pub fn property_display_name(prop_name: &str) -> String {
    match prop_name {
        "anchorX" => "Anchor Point X".to_string(),
        "anchorY" => "Anchor Point Y".to_string(),
        "anchorZ" => "Anchor Point Z".to_string(),
        "x" => "Position X".to_string(),
        "y" => "Position Y".to_string(),
        "z" => "Position Z".to_string(),
        "scaleX" => "Scale X".to_string(),
        "scaleY" => "Scale Y".to_string(),
        "scaleZ" => "Scale Z".to_string(),
        "rotation" => "Rotation (Z)".to_string(),
        "rotationX" => "Rotation X".to_string(),
        "rotationY" => "Rotation Y".to_string(),
        "opacity" => "Opacity".to_string(),
        "poiX" => "Point of Interest X".to_string(),
        "poiY" => "Point of Interest Y".to_string(),
        "poiZ" => "Point of Interest Z".to_string(),
        "zoom" => "Zoom".to_string(),
        "fov" => "Field of View".to_string(),
        "audioVolume" => "Audio Level (%)".to_string(),
        "audioPan" => "Audio Pan".to_string(),
        "colorR" => "Color R".to_string(),
        "colorG" => "Color G".to_string(),
        "colorB" => "Color B".to_string(),
        "blackR" => "Black R".to_string(),
        "blackG" => "Black G".to_string(),
        "blackB" => "Black B".to_string(),
        "whiteR" => "White R".to_string(),
        "whiteG" => "White G".to_string(),
        "whiteB" => "White B".to_string(),
        "amount" => "Amount".to_string(),
        "brightness" => "Brightness".to_string(),
        "contrast" => "Contrast".to_string(),
        "blurRadius" => "Blur Radius".to_string(),
        "blurLength" => "Blur Length".to_string(),
        "threshold" => "Threshold".to_string(),
        "radius" => "Radius".to_string(),
        "intensity" => "Intensity".to_string(),
        "distance" => "Distance".to_string(),
        "angle" => "Angle".to_string(),
        "softness" => "Softness".to_string(),
        "blend" => "Blend".to_string(),
        "feather" => "Feather".to_string(),
        "hue" => "Hue".to_string(),
        "saturation" => "Saturation".to_string(),
        "lightness" => "Lightness".to_string(),
        "waveHeight" => "Wave Height".to_string(),
        "waveWidth" => "Wave Width".to_string(),
        "speed" => "Speed".to_string(),
        "direction" => "Direction".to_string(),
        other => crate::plugin::format_display_name(other),
    }
}

pub fn sorted_property_names(layer: &Layer) -> Vec<String> {
    let mut names: Vec<String> = layer.properties.keys().cloned().collect();
    let order = [
        "anchorX",
        "anchorY",
        "anchorZ",
        "x",
        "y",
        "z",
        "scaleX",
        "scaleY",
        "scaleZ",
        "rotation",
        "rotationX",
        "rotationY",
        "opacity",
        "poiX",
        "poiY",
        "poiZ",
        "zoom",
        "fov",
        "audioVolume",
        "audioPan",
    ];
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
        format!("{} / {}", layer_name, property_display_name(&selected.property_name)),
    ))
}

fn selection_is_valid(comp: &Composition, selected: &SelectedKeyframe) -> bool {
    comp.layers
        .get(selected.layer_index)
        .and_then(|layer| layer.properties.get(&selected.property_name))
        .and_then(|prop| prop.keyframes.get(selected.keyframe_index))
        .is_some()
}

pub fn delete_selected_keyframe(comp: &mut Composition, selected: &mut Option<SelectedKeyframe>) {
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

pub struct VisibleTimelineRow {
    pub is_layer: bool,
    pub layer_index: usize,
    pub property_name: Option<String>,
    pub row_height: f32,
    pub y_top: f32,
}

pub const LAYER_ROW_HEIGHT: f32 = 24.0;
pub const PROPERTY_ROW_HEIGHT: f32 = 22.0;
pub const CURVE_ROW_HEIGHT: f32 = 90.0;
pub const TIMELINE_HEADER_HEIGHT: f32 = 26.0;

pub fn snap_time(comp: &Composition, target: f32, snap_range_time: f32) -> f32 {
    if !comp.snapping {
        return target;
    }
    let mut closest = target;
    let mut min_diff = snap_range_time;
    let candidates = [0.0, comp.settings.duration, comp.work_area_in, comp.work_area_out];
    for &cand in &candidates {
        let diff = (cand - target).abs();
        if diff < min_diff {
            min_diff = diff;
            closest = cand;
        }
    }
    for m in &comp.markers {
        let diff = (m.time - target).abs();
        if diff < min_diff {
            min_diff = diff;
            closest = m.time;
        }
    }
    for l in &comp.layers {
        for &t in &[l.in_time, l.out_time] {
            let diff = (t - target).abs();
            if diff < min_diff {
                min_diff = diff;
                closest = t;
            }
        }
        for m in &l.markers {
            let diff = (m.time - target).abs();
            if diff < min_diff {
                min_diff = diff;
                closest = m.time;
            }
        }
        for (_, p) in &l.properties {
            for kf in &p.keyframes {
                let diff = (kf.time - target).abs();
                if diff < min_diff {
                    min_diff = diff;
                    closest = kf.time;
                }
            }
        }
    }
    closest
}

pub fn calculate_visible_rows(comp: &Composition) -> (Vec<VisibleTimelineRow>, f32) {
    let mut rows = Vec::new();
    let mut current_y = 0.0;

    for (layer_index, layer) in comp.layers.iter().enumerate() {
        if comp.hide_shy && layer.shy {
            continue;
        }
        if !comp.layer_search_query.is_empty()
            && !layer
                .name
                .to_lowercase()
                .contains(&comp.layer_search_query.to_lowercase())
        {
            continue;
        }

        let row_h = LAYER_ROW_HEIGHT;
        rows.push(VisibleTimelineRow {
            is_layer: true,
            layer_index,
            property_name: None,
            row_height: row_h,
            y_top: current_y,
        });
        current_y += row_h;

        if !layer.collapsed {
            for prop_name in sorted_property_names(layer) {
                if let Some(prop) = layer.properties.get(&prop_name) {
                    if comp.solo_animated_properties && prop.keyframes.is_empty() {
                        continue;
                    }
                    let p_h = if comp.show_curves {
                        CURVE_ROW_HEIGHT
                    } else {
                        PROPERTY_ROW_HEIGHT
                    };
                    rows.push(VisibleTimelineRow {
                        is_layer: false,
                        layer_index,
                        property_name: Some(prop_name),
                        row_height: p_h,
                        y_top: current_y,
                    });
                    current_y += p_h;
                }
            }
        }
    }

    (rows, current_y)
}

fn draw_keyframe_editor(
    ui: &mut egui::Ui,
    comp: &mut Composition,
    selected: &mut Option<SelectedKeyframe>,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.label(
            egui::RichText::new("Keyframe:")
                .strong()
                .color(egui::Color32::from_gray(190)),
        );

        if let Some(sel) = selected.clone() {
            if let Some((keyframe, label)) = selected_keyframe_mut(comp, &sel) {
                ui.label(egui::RichText::new(label).color(egui::Color32::from_rgb(145, 190, 220)));
                ui.separator();
                
                ui.label("Time:");
                ui.add(
                    egui::DragValue::new(&mut keyframe.time)
                        .speed(0.01)
                        .range(0.0..=600.0)
                        .suffix("s"),
                );
                
                ui.label("Value:");
                ui.add(egui::DragValue::new(&mut keyframe.value).speed(0.5));

                let mut has_ease = keyframe.ease.is_some();
                if ui.checkbox(&mut has_ease, "Bezier Ease").changed() {
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
                    if ui.button("Easy Ease (F9)").clicked() {
                        ease.cp1 = 0.33;
                        ease.cp2 = 0.67;
                    }
                    if ui.button("Ease In (Shift+F9)").clicked() {
                        ease.cp1 = 0.15;
                        ease.cp2 = 1.0;
                    }
                    if ui.button("Ease Out (Ctrl+Shift+F9)").clicked() {
                        ease.cp1 = 0.85;
                        ease.cp2 = 0.35;
                    }
                    ui.label("In:");
                    ui.add(egui::Slider::new(&mut ease.cp1, 0.0..=1.0).show_value(false));
                    ui.label("Out:");
                    ui.add(egui::Slider::new(&mut ease.cp2, 0.0..=1.0).show_value(false));
                }

                if ui.button("Linear").on_hover_text("Reset to Linear Keyframe").clicked() {
                    keyframe.ease = None;
                }

                if ui
                    .button(egui::RichText::new("🗑 Delete").color(egui::Color32::from_rgb(240, 90, 90)))
                    .clicked()
                {
                    delete_selected_keyframe(comp, selected);
                }
            } else {
                *selected = None;
                ui.label(
                    egui::RichText::new("Select a keyframe diamond to edit values & curve handles.")
                        .color(egui::Color32::from_gray(120)),
                );
            }
        } else {
            ui.label(
                egui::RichText::new("No keyframe selected. Click a diamond ◆ or add keyframes with ⏱.")
                    .color(egui::Color32::from_gray(120)),
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
                .sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
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

    // Keyboard Shortcuts
    if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
        if selected.is_some() {
            delete_selected_keyframe(comp, selected);
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
    }

    if ui.input(|i| i.key_pressed(egui::Key::F9)) {
        if let Some(sel) = selected {
            if let Some((kf, _)) = selected_keyframe_mut(comp, sel) {
                kf.ease = Some(BezierControl {
                    cp1: 0.33,
                    cp2: 0.67,
                });
            }
        }
    }

    // Keyframe Editor Toolbar Strip
    draw_keyframe_editor(ui, comp, selected);
    ui.add_space(2.0);

    let (rows, total_content_height) = calculate_visible_rows(comp);
    let pps = comp.timeline_zoom.clamp(20.0, 500.0);
    let timeline_duration = comp.settings.duration.max(10.0);
    let timeline_track_width = (timeline_duration * pps).max(ui.available_width() - 360.0);
    let label_width = 370.0;
    let available_h = ui.available_height();

    // Work area and global switches bar
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;

        // Switches / Modes toggle button
        let mode_btn_text = if comp.switches_mode {
            "🗂 Switches"
        } else {
            "🗂 Modes"
        };
        if ui
            .button(egui::RichText::new(mode_btn_text).size(11.0))
            .on_hover_text("Toggle between Layer Switches and Blend Modes / Track Mattes")
            .clicked()
        {
            comp.switches_mode = !comp.switches_mode;
        }

        ui.separator();

        // Shy Layers Global Toggle
        if ui
            .selectable_label(
                comp.hide_shy,
                egui::RichText::new("👤 Hide Shy").size(11.0),
            )
            .on_hover_text("Hide all layers marked with Shy")
            .clicked()
        {
            comp.hide_shy = !comp.hide_shy;
        }

        // Curve Graph Editor Global Toggle
        if ui
            .selectable_label(
                comp.show_curves,
                egui::RichText::new("📈 Graph Editor").size(11.0),
            )
            .on_hover_text("Toggle Graph Editor (Curve keyframe interpolations)")
            .clicked()
        {
            comp.show_curves = !comp.show_curves;
        }

        ui.separator();

        // Snapping Toggle
        if ui
            .selectable_label(
                comp.snapping,
                egui::RichText::new("🧲 Snap").size(11.0),
            )
            .on_hover_text("Toggle Timeline Snapping to Keyframes, Markers & Boundaries")
            .clicked()
        {
            comp.snapping = !comp.snapping;
        }

        // Solo Animated Properties Toggle (U)
        if ui
            .selectable_label(
                comp.solo_animated_properties,
                egui::RichText::new("⚡ Solo Anim (U)").size(11.0),
            )
            .on_hover_text("Show only animated properties with keyframes (U)")
            .clicked()
        {
            comp.solo_animated_properties = !comp.solo_animated_properties;
        }

        // Auto Frame Cache Toggle
        if ui
            .selectable_label(
                comp.auto_frame_cache,
                egui::RichText::new("⚡ Auto Cache").size(11.0),
            )
            .on_hover_text("Automatically cache frames in the background from beginning to the final keyframe")
            .clicked()
        {
            comp.auto_frame_cache = !comp.auto_frame_cache;
        }

        // Pause at Last Keyframe Toggle
        if ui
            .selectable_label(
                comp.pause_at_last_keyframe,
                egui::RichText::new("⏸ Pause @ Last KF").size(11.0),
            )
            .on_hover_text("Automatically pause playback when reaching the final keyframe instead of looping")
            .clicked()
        {
            comp.pause_at_last_keyframe = !comp.pause_at_last_keyframe;
        }

        // Add Marker Button
        if ui
            .button(egui::RichText::new("📍 +Marker (*)").size(11.0))
            .on_hover_text("Add Composition Marker at current playhead time (*)")
            .clicked()
        {
            let num = comp.markers.len() + 1;
            comp.markers.push(Marker {
                time: comp.current_time,
                label: format!("M{}", num),
                comment: format!("Marker {}", num),
                color_index: (num - 1) % AE_LABEL_COLORS.len(),
            });
        }

        ui.separator();

        // Timeline Zoom Controls
        ui.label(egui::RichText::new("Zoom:").size(11.0).color(egui::Color32::from_gray(140)));
        if ui.button("1s").clicked() {
            comp.timeline_zoom = 250.0;
        }
        if ui.button("5s").clicked() {
            comp.timeline_zoom = 120.0;
        }
        if ui.button("10s").clicked() {
            comp.timeline_zoom = 70.0;
        }
        if ui.button("Fit").clicked() {
            let avail_w = (ui.available_width() - label_width - 40.0).max(300.0);
            comp.timeline_zoom = (avail_w / comp.settings.duration).clamp(20.0, 400.0);
        }
        ui.add(
            egui::Slider::new(&mut comp.timeline_zoom, 25.0..=400.0)
                .show_value(false)
                .logarithmic(true),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut comp.layer_search_query)
                    .hint_text("🔍 Search layers...")
                    .desired_width(120.0),
            );
        });
    });

    if comp.show_curves {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📊 Graph Editor:").size(11.0).color(egui::Color32::from_rgb(255, 215, 75)).strong());

            let is_val = comp.graph_mode == GraphMode::ValueGraph;
            if ui.selectable_label(is_val, egui::RichText::new("📈 Value Graph").size(11.0)).on_hover_text("Show absolute property values and trajectory over time").clicked() {
                comp.graph_mode = GraphMode::ValueGraph;
            }
            let is_spd = comp.graph_mode == GraphMode::SpeedGraph;
            if ui.selectable_label(is_spd, egui::RichText::new("⚡ Speed Graph").size(11.0)).on_hover_text("Show rate of change / velocity in units per second").clicked() {
                comp.graph_mode = GraphMode::SpeedGraph;
            }

            ui.separator();
            ui.label(egui::RichText::new("Easing Presets:").size(11.0).color(egui::Color32::from_gray(140)));

            if ui.button(egui::RichText::new("Easy Ease (F9)").size(10.5)).on_hover_text("Apply smooth symmetric cubic bezier easing (F9)").clicked() {
                if let Some(sel) = selected.as_ref() {
                    if let Some(prop) = comp.layers.get_mut(sel.layer_index).and_then(|l| l.properties.get_mut(&sel.property_name)) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            kf.ease = Some(BezierControl::easy_ease());
                        }
                    }
                }
            }
            if ui.button(egui::RichText::new("Ease In (Shift+F9)").size(10.5)).on_hover_text("Apply smooth incoming deceleration (Shift+F9)").clicked() {
                if let Some(sel) = selected.as_ref() {
                    if let Some(prop) = comp.layers.get_mut(sel.layer_index).and_then(|l| l.properties.get_mut(&sel.property_name)) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            kf.ease = Some(BezierControl::ease_in());
                        }
                    }
                }
            }
            if ui.button(egui::RichText::new("Ease Out (Ctrl+Shift+F9)").size(10.5)).on_hover_text("Apply smooth outgoing acceleration (Ctrl+Shift+F9)").clicked() {
                if let Some(sel) = selected.as_ref() {
                    if let Some(prop) = comp.layers.get_mut(sel.layer_index).and_then(|l| l.properties.get_mut(&sel.property_name)) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            kf.ease = Some(BezierControl::ease_out());
                        }
                    }
                }
            }
            if ui.button(egui::RichText::new("Linear").size(10.5)).on_hover_text("Constant linear interpolation").clicked() {
                if let Some(sel) = selected.as_ref() {
                    if let Some(prop) = comp.layers.get_mut(sel.layer_index).and_then(|l| l.properties.get_mut(&sel.property_name)) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            kf.ease = Some(BezierControl::linear());
                        }
                    }
                }
            }
            if ui.button(egui::RichText::new("Overshoot").size(10.5)).on_hover_text("Dynamic anticipation & overshoot bounce").clicked() {
                if let Some(sel) = selected.as_ref() {
                    if let Some(prop) = comp.layers.get_mut(sel.layer_index).and_then(|l| l.properties.get_mut(&sel.property_name)) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            kf.ease = Some(BezierControl::back_out());
                        }
                    }
                }
            }
            if ui.button(egui::RichText::new("Bounce").size(10.5)).on_hover_text("Exponential elastic easing curve").clicked() {
                if let Some(sel) = selected.as_ref() {
                    if let Some(prop) = comp.layers.get_mut(sel.layer_index).and_then(|l| l.properties.get_mut(&sel.property_name)) {
                        if let Some(kf) = prop.keyframes.get_mut(sel.keyframe_index) {
                            kf.ease = Some(BezierControl::exponential());
                        }
                    }
                }
            }
        });
    }

    ui.add_space(2.0);

    // Zoom with Ctrl + Mouse Wheel
    let smooth_scroll = ui.input(|i| i.smooth_scroll_delta);
    if ui.input(|i| i.modifiers.ctrl) && smooth_scroll.y.abs() > 0.5 {
        comp.timeline_zoom = (comp.timeline_zoom + smooth_scroll.y * 0.25).clamp(20.0, 500.0);
    }

    // Synchronized Timeline Layout
    // Outer scroll area handles VERTICAL scrolling for both Left and Right columns in exact lockstep!
    let scroll_v = egui::ScrollArea::vertical()
        .id_salt("timeline_unified_vertical_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let total_h = (total_content_height + TIMELINE_HEADER_HEIGHT + 40.0).max(available_h);
            ui.allocate_ui(egui::vec2(ui.available_width(), total_h), |ui| {
                ui.horizontal(|ui| {
                    // ==========================================
                    // LEFT COLUMN: LAYER HEADERS & PROPERTIES
                    // ==========================================
                    ui.allocate_ui(egui::vec2(label_width, total_h), |ui| {
                        let (left_col_resp, left_painter) = ui.allocate_painter(
                            egui::vec2(label_width, total_h),
                            egui::Sense::hover(),
                        );
                        let left_col_rect = left_col_resp.rect;

                        // Draw Left Header
                        let header_rect = egui::Rect::from_min_size(
                            left_col_rect.min,
                            egui::vec2(label_width, TIMELINE_HEADER_HEIGHT),
                        );
                        left_painter.rect_filled(
                            header_rect,
                            0.0,
                            egui::Color32::from_rgb(24, 24, 26),
                        );
                        left_painter.line_segment(
                            [header_rect.left_bottom(), header_rect.right_bottom()],
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 50)),
                        );

                        // Header Column Labels
                        left_painter.text(
                            egui::pos2(header_rect.left() + 6.0, header_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "#",
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_gray(140),
                        );
                        left_painter.text(
                            egui::pos2(header_rect.left() + 28.0, header_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "Layer Name",
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_gray(170),
                        );

                        if comp.switches_mode {
                            left_painter.text(
                                egui::pos2(header_rect.right() - 110.0, header_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Mode / Parent",
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_gray(140),
                            );
                        } else {
                            left_painter.text(
                                egui::pos2(header_rect.right() - 125.0, header_rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Switches (* fx 3D B /)",
                                egui::FontId::proportional(11.0),
                                egui::Color32::from_gray(140),
                            );
                        }

                        // Render Each Row on the Left Side
                        let mut layer_to_toggle_collapse = None;
                        let mut layer_to_select = None;
                        let layer_names: Vec<String> =
                            comp.layers.iter().map(|l| l.name.clone()).collect();

                        for (row_idx, row) in rows.iter().enumerate() {
                            let row_y = left_col_rect.min.y + TIMELINE_HEADER_HEIGHT + row.y_top;
                            let row_rect = egui::Rect::from_min_size(
                                egui::pos2(left_col_rect.min.x, row_y),
                                egui::vec2(label_width, row.row_height),
                            );

                            let is_active_layer = comp.active_layer_index == Some(row.layer_index);

                            // Background zebra & active state
                            let bg_color = if is_active_layer {
                                egui::Color32::from_rgb(34, 60, 92)
                            } else if row_idx % 2 == 0 {
                                egui::Color32::from_rgb(28, 28, 31)
                            } else {
                                egui::Color32::from_rgb(32, 32, 35)
                            };
                            left_painter.rect_filled(row_rect, 0.0, bg_color);
                            left_painter.line_segment(
                                [row_rect.left_bottom(), row_rect.right_bottom()],
                                egui::Stroke::new(0.5, egui::Color32::from_rgb(42, 42, 46)),
                            );

                            // Put interactive widgets inside the allocated row rect
                            let mut row_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(row_rect)
                                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            );
                            row_ui.spacing_mut().item_spacing = egui::vec2(3.0, 0.0);

                            if row.is_layer {
                                if let Some(layer) = comp.layers.get_mut(row.layer_index) {
                                    // Layer index
                                    row_ui.add_space(2.0);
                                    row_ui.label(
                                        egui::RichText::new(format!("{}", row.layer_index + 1))
                                            .size(10.5)
                                            .color(egui::Color32::from_gray(140)),
                                    );

                                    // Color swatch badge
                                    let label_col = get_label_color(layer.label_color_index);
                                    let (swatch_rect, swatch_resp) = row_ui.allocate_exact_size(
                                        egui::vec2(8.0, 14.0),
                                        egui::Sense::click(),
                                    );
                                    row_ui.painter().rect_filled(swatch_rect, 1.0, label_col);
                                    if swatch_resp.clicked() {
                                        layer.label_color_index =
                                            (layer.label_color_index + 1) % AE_LABEL_COLORS.len();
                                    }

                                    // Visibility Eye
                                    let vis_icon = if layer.visible { "👁" } else { "  " };
                                    if row_ui
                                        .selectable_label(layer.visible, vis_icon)
                                        .on_hover_text("Video Visibility (Eye)")
                                        .clicked()
                                    {
                                        layer.visible = !layer.visible;
                                    }

                                    // Audio
                                    let is_audio = matches!(layer.source, LayerSource::Audio { .. });
                                    if is_audio {
                                        let aud_icon = if layer.visible { "🔊" } else { "🔇" };
                                        if row_ui.button(aud_icon).clicked() {
                                            layer.visible = !layer.visible;
                                        }
                                    }

                                    // Solo
                                    let solo_color = if layer.solo {
                                        egui::Color32::from_rgb(255, 200, 0)
                                    } else {
                                        egui::Color32::from_gray(100)
                                    };
                                    if row_ui
                                        .add(egui::SelectableLabel::new(
                                            layer.solo,
                                            egui::RichText::new("S").color(solo_color).size(10.0),
                                        ))
                                        .on_hover_text("Solo Layer")
                                        .clicked()
                                    {
                                        layer.solo = !layer.solo;
                                    }

                                    // Lock
                                    let lock_icon = if layer.locked { "L" } else { "  " };
                                    if row_ui
                                        .selectable_label(layer.locked, lock_icon)
                                        .on_hover_text("Lock Layer")
                                        .clicked()
                                    {
                                        layer.locked = !layer.locked;
                                    }

                                    // Shy
                                    let shy_icon = if layer.shy { "S" } else { "  " };
                                    if row_ui
                                        .selectable_label(layer.shy, shy_icon)
                                        .on_hover_text("Shy Layer (Hides when Shy is enabled globally)")
                                        .clicked()
                                    {
                                        layer.shy = !layer.shy;
                                    }

                                    // Expand / Collapse arrow
                                    let arrow_icon = if layer.collapsed { "▶" } else { "▼" };
                                    if row_ui
                                        .button(egui::RichText::new(arrow_icon).size(9.0))
                                        .on_hover_text("Twirl Down Transform / Properties")
                                        .clicked()
                                    {
                                        layer_to_toggle_collapse = Some(row.layer_index);
                                    }

                                    // Layer Name button
                                    let name_color = if is_active_layer {
                                        egui::Color32::WHITE
                                    } else {
                                        egui::Color32::from_rgb(175, 205, 235)
                                    };
                                    if row_ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(&layer.name)
                                                    .strong()
                                                    .size(11.5)
                                                    .color(name_color),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        layer_to_select = Some(row.layer_index);
                                    }

                                    // Right Switches or Modes
                                    row_ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |row_ui| {
                                            row_ui.spacing_mut().item_spacing.x = 2.0;

                                            if comp.switches_mode {
                                                // Parent & Link Picker
                                                egui::ComboBox::from_id_salt(format!("parent_{}", row.layer_index))
                                                    .width(65.0)
                                                    .selected_text(
                                                        layer
                                                            .parent_index
                                                            .map_or("None".to_string(), |p| format!("{}. Parent", p + 1)),
                                                    )
                                                    .show_ui(row_ui, |ui| {
                                                        ui.selectable_value(&mut layer.parent_index, None, "None");
                                                        for (p_idx, p_name) in layer_names.iter().enumerate() {
                                                            if p_idx != row.layer_index {
                                                                ui.selectable_value(
                                                                    &mut layer.parent_index,
                                                                    Some(p_idx),
                                                                    format!("{}. {}", p_idx + 1, p_name),
                                                                );
                                                            }
                                                        }
                                                    });

                                                // Blend Mode
                                                egui::ComboBox::from_id_salt(format!("blend_{}", row.layer_index))
                                                    .width(65.0)
                                                    .selected_text(&layer.blend_mode)
                                                    .show_ui(row_ui, |ui| {
                                                        for m in ["Normal", "Multiply", "Screen", "Overlay", "Add", "Darken", "Lighten", "Difference"] {
                                                            ui.selectable_value(&mut layer.blend_mode, m.to_string(), m);
                                                        }
                                                    });
                                            } else {
                                                // Frame Blending
                                                let ff_text = if layer.ff { "/" } else { " " };
                                                if row_ui.selectable_label(layer.ff, ff_text).on_hover_text("Frame Blending").clicked() {
                                                    layer.ff = !layer.ff;
                                                }

                                                // Motion Blur
                                                let moblur_text = if layer.moblur { "B" } else { "  " };
                                                if row_ui.selectable_label(layer.moblur, moblur_text).on_hover_text("Motion Blur").clicked() {
                                                    layer.moblur = !layer.moblur;
                                                }

                                                // 3D Layer
                                                let d3_text = if layer.d3 { "3D" } else { " " };
                                                if row_ui.selectable_label(layer.d3, d3_text).on_hover_text("3D Layer").clicked() {
                                                    layer.d3 = !layer.d3;
                                                }

                                                // Effect Switch
                                                let fx_text = if layer.fx { "fx" } else { "  " };
                                                if row_ui.selectable_label(layer.fx, fx_text).on_hover_text("Effects Switch").clicked() {
                                                    layer.fx = !layer.fx;
                                                }

                                                // Collapse Transformations
                                                let collapse_text = if layer.collapse { "*" } else { " " };
                                                if row_ui.selectable_label(layer.collapse, collapse_text).on_hover_text("Collapse Transformations / Continuously Rasterize").clicked() {
                                                    layer.collapse = !layer.collapse;
                                                }
                                            }
                                        },
                                    );
                                }
                            } else if let Some(prop_name) = &row.property_name {
                                if let Some(layer) = comp.layers.get_mut(row.layer_index) {
                                    let current_time = comp.current_time;
                                    if let Some(prop) = layer.properties.get_mut(prop_name) {
                                        row_ui.add_space(28.0);

                                        // Property Color Dot
                                        let prop_color = if let Some(c) = comp.settings.property_colors.get(prop_name) {
                                            egui::Color32::from_rgb(c[0], c[1], c[2])
                                        } else {
                                            egui::Color32::from_gray(140)
                                        };
                                        row_ui.painter().circle_filled(
                                            egui::pos2(row_rect.left() + 20.0, row_rect.center().y),
                                            2.5,
                                            prop_color,
                                        );

                                        // Stopwatch toggle (keyframe activation)
                                        let has_keyframes = !prop.keyframes.is_empty();
                                        let stopwatch_color = if has_keyframes {
                                            egui::Color32::from_rgb(70, 165, 245)
                                        } else {
                                            egui::Color32::from_gray(120)
                                        };
                                        if row_ui
                                            .add(egui::Button::new(
                                                egui::RichText::new("⏱").size(11.0).color(stopwatch_color),
                                            ).frame(false))
                                            .on_hover_text("Stopwatch: Toggle Keyframing & Animation")
                                            .clicked()
                                        {
                                            if prop.keyframes.is_empty() {
                                                prop.keyframes.push(Keyframe {
                                                    time: current_time,
                                                    value: prop.base_value,
                                                    ease: Some(BezierControl {
                                                        cp1: 0.33,
                                                        cp2: 0.67,
                                                    }),
                                                });
                                            } else {
                                                prop.keyframes.clear();
                                            }
                                        }

                                        // Keyframe Navigation: Previous Keyframe ◀
                                        let prev_kf = prop
                                            .keyframes
                                            .iter()
                                            .filter(|kf| kf.time < current_time - 0.001)
                                            .last()
                                            .map(|kf| kf.time);
                                        if row_ui
                                            .add_enabled(
                                                prev_kf.is_some(),
                                                egui::Button::new(egui::RichText::new("◀").size(9.0)).frame(false),
                                            )
                                            .on_hover_text("Jump to Previous Keyframe (J)")
                                            .clicked()
                                        {
                                            if let Some(t) = prev_kf {
                                                comp.current_time = t;
                                            }
                                        }

                                        // Add/Remove Keyframe Diamond ◆
                                        let has_kf_at_current = prop
                                            .keyframes
                                            .iter()
                                            .position(|kf| (kf.time - current_time).abs() < 0.02);
                                        let diamond_color = if has_kf_at_current.is_some() {
                                            egui::Color32::from_rgb(255, 205, 50)
                                        } else {
                                            egui::Color32::from_gray(100)
                                        };
                                        if row_ui
                                            .add(egui::Button::new(
                                                egui::RichText::new("◆").size(10.0).color(diamond_color),
                                            ).frame(false))
                                            .on_hover_text("Add or Remove Keyframe at Current Time")
                                            .clicked()
                                        {
                                            if let Some(idx) = has_kf_at_current {
                                                prop.keyframes.remove(idx);
                                            } else {
                                                prop.keyframes.push(Keyframe {
                                                    time: current_time,
                                                    value: prop.get_value_at(current_time),
                                                    ease: Some(BezierControl {
                                                        cp1: 0.33,
                                                        cp2: 0.67,
                                                    }),
                                                });
                                                prop.keyframes.sort_by(|a, b| {
                                                    a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
                                                });
                                            }
                                        }

                                        // Keyframe Navigation: Next Keyframe ▶
                                        let next_kf = prop
                                            .keyframes
                                            .iter()
                                            .find(|kf| kf.time > current_time + 0.001)
                                            .map(|kf| kf.time);
                                        if row_ui
                                            .add_enabled(
                                                next_kf.is_some(),
                                                egui::Button::new(egui::RichText::new("▶").size(9.0)).frame(false),
                                            )
                                            .on_hover_text("Jump to Next Keyframe (K)")
                                            .clicked()
                                        {
                                            if let Some(t) = next_kf {
                                                comp.current_time = t;
                                            }
                                        }

                                        // Property Name
                                        row_ui.label(
                                            egui::RichText::new(property_display_name(prop_name))
                                                .size(11.0)
                                                .color(egui::Color32::from_gray(190)),
                                        );

                                        // Value Scrubber right in timeline
                                        row_ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |row_ui| {
                                                // Wiggle Expression toggle
                                                let has_wiggle = prop.wiggle.as_ref().is_some_and(|w| w.enabled);
                                                let wiggle_color = if has_wiggle {
                                                    egui::Color32::from_rgb(250, 150, 50)
                                                } else {
                                                    egui::Color32::from_gray(90)
                                                };
                                                if row_ui
                                                    .add(egui::Button::new(egui::RichText::new("~").size(10.0).color(wiggle_color)).frame(false))
                                                    .on_hover_text("Toggle Wiggle (Organic Noise Expression)")
                                                    .clicked()
                                                {
                                                    if let Some(w) = &mut prop.wiggle {
                                                        w.enabled = !w.enabled;
                                                    } else {
                                                        prop.wiggle = Some(WiggleSettings {
                                                            enabled: true,
                                                            freq: 2.0,
                                                            amp: 20.0,
                                                        });
                                                    }
                                                }

                                                let speed = if prop_name.contains("scale") || prop_name.contains("opacity") {
                                                    0.5
                                                } else if prop_name.contains("rotation") {
                                                    0.25
                                                } else {
                                                    0.5
                                                };
                                                
                                                let mut val = if prop.keyframes.is_empty() {
                                                    prop.base_value
                                                } else {
                                                    prop.get_value_at(current_time)
                                                };

                                                if row_ui.add(egui::DragValue::new(&mut val).speed(speed)).changed() {
                                                    if prop.keyframes.is_empty() {
                                                        prop.base_value = val;
                                                    } else if let Some(idx) = has_kf_at_current {
                                                        prop.keyframes[idx].value = val;
                                                    } else {
                                                        prop.keyframes.push(Keyframe {
                                                            time: current_time,
                                                            value: val,
                                                            ease: Some(BezierControl {
                                                                cp1: 0.33,
                                                                cp2: 0.67,
                                                            }),
                                                        });
                                                        prop.keyframes.sort_by(|a, b| {
                                                            a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
                                                        });
                                                    }
                                                }
                                            },
                                        );
                                    }
                                }
                            }
                        }

                        if let Some(idx) = layer_to_toggle_collapse {
                            if let Some(l) = comp.layers.get_mut(idx) {
                                l.collapsed = !l.collapsed;
                            }
                        }
                        if let Some(idx) = layer_to_select {
                            comp.active_layer_index = Some(idx);
                        }
                    });

                    ui.separator();

                    // ==========================================
                    // RIGHT COLUMN: TIME RULER & TRACKS CANVAS
                    // ==========================================
                    let scroll_h = egui::ScrollArea::horizontal()
                        .id_salt("timeline_tracks_scroll_h")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let (track_res, painter) = ui.allocate_painter(
                                egui::vec2(timeline_track_width, total_h),
                                egui::Sense::click_and_drag(),
                            );
                            let track_rect = track_res.rect;

                            // Fill background
                            painter.rect_filled(track_rect, 0.0, egui::Color32::from_rgb(20, 20, 22));

                            // ------------------------------------------
                            // 1. TIME RULER (Top)
                            // ------------------------------------------
                            let ruler_rect = egui::Rect::from_min_size(
                                track_rect.min,
                                egui::vec2(timeline_track_width, TIMELINE_HEADER_HEIGHT),
                            );
                            painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_rgb(24, 24, 26));
                            painter.line_segment(
                                [ruler_rect.left_bottom(), ruler_rect.right_bottom()],
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 45, 50)),
                            );

                            // Work Area Bar in Ruler
                            let work_in_x = track_rect.left() + (comp.work_area_in * pps);
                            let work_out_x = track_rect.left() + (comp.work_area_out.min(timeline_duration) * pps);
                            let work_area_rect = egui::Rect::from_min_max(
                                egui::pos2(work_in_x, ruler_rect.top() + 1.0),
                                egui::pos2(work_out_x, ruler_rect.top() + 5.0),
                            );
                            painter.rect_filled(work_area_rect, 1.0, egui::Color32::from_rgb(65, 140, 215));

                            // Work Area In/Out Handles
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(work_in_x - 3.0, ruler_rect.top()),
                                    egui::pos2(work_in_x + 1.0, ruler_rect.top() + 8.0),
                                ),
                                0.0,
                                egui::Color32::from_rgb(100, 180, 255),
                            );
                            painter.rect_filled(
                                egui::Rect::from_min_max(
                                    egui::pos2(work_out_x - 1.0, ruler_rect.top()),
                                    egui::pos2(work_out_x + 3.0, ruler_rect.top() + 8.0),
                                ),
                                0.0,
                                egui::Color32::from_rgb(100, 180, 255),
                            );

                            // RAM Preview Cached Frames Indicator Bar (Bright Green)
                            if comp.ram_cache_enabled && !comp.cached_frames.is_empty() {
                                let frame_w = pps / comp.settings.fps.max(1) as f32;
                                let cache_y_top = ruler_rect.top() + 5.0;
                                let cache_y_bottom = ruler_rect.top() + 7.5;

                                let mut cached_vec: Vec<usize> = comp.cached_frames.iter().copied().collect();
                                cached_vec.sort_unstable();

                                if !cached_vec.is_empty() {
                                    let mut span_start = cached_vec[0];
                                    let mut span_end = cached_vec[0];

                                    for &f in &cached_vec[1..] {
                                        if f == span_end + 1 {
                                            span_end = f;
                                        } else {
                                            let x_start = track_rect.left() + (span_start as f32 * frame_w);
                                            let x_end = track_rect.left() + ((span_end + 1) as f32 * frame_w);
                                            let cache_rect = egui::Rect::from_min_max(
                                                egui::pos2(x_start, cache_y_top),
                                                egui::pos2(x_end, cache_y_bottom),
                                            );
                                            painter.rect_filled(cache_rect, 0.5, egui::Color32::from_rgb(45, 215, 95));
                                            span_start = f;
                                            span_end = f;
                                        }
                                    }
                                    let x_start = track_rect.left() + (span_start as f32 * frame_w);
                                    let x_end = track_rect.left() + ((span_end + 1) as f32 * frame_w);
                                    let cache_rect = egui::Rect::from_min_max(
                                        egui::pos2(x_start, cache_y_top),
                                        egui::pos2(x_end, cache_y_bottom),
                                    );
                                    painter.rect_filled(cache_rect, 0.5, egui::Color32::from_rgb(45, 215, 95));
                                }
                            }

                            // Ruler Time Markers
                            let total_seconds = timeline_duration.ceil() as i32;
                            for s in 0..=total_seconds {
                                let x = track_rect.left() + (s as f32 * pps);
                                painter.line_segment(
                                    [egui::pos2(x, ruler_rect.top() + 6.0), egui::pos2(x, ruler_rect.bottom())],
                                    egui::Stroke::new(1.0, egui::Color32::from_gray(75)),
                                );

                                // Major frame ticks between seconds
                                for f in 1..comp.settings.fps.min(30) {
                                    let sub_x = x + (f as f32 / comp.settings.fps as f32) * pps;
                                    painter.line_segment(
                                        [egui::pos2(sub_x, ruler_rect.bottom() - 4.0), egui::pos2(sub_x, ruler_rect.bottom())],
                                        egui::Stroke::new(0.5, egui::Color32::from_gray(45)),
                                    );
                                }

                                painter.text(
                                    egui::pos2(x + 3.0, ruler_rect.top() + 8.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{}s", s),
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::from_gray(160),
                                );

                                // Vertical grid line down the canvas
                                painter.line_segment(
                                    [egui::pos2(x, ruler_rect.bottom()), egui::pos2(x, track_rect.bottom())],
                                    egui::Stroke::new(0.5, egui::Color32::from_rgb(32, 32, 36)),
                                );
                            }

                            // Render Composition Markers on Ruler
                            for (_m_idx, marker) in comp.markers.iter().enumerate() {
                                let mx = track_rect.left() + (marker.time * pps);
                                let m_color = get_label_color(marker.color_index);
                                let marker_pts = vec![
                                    egui::pos2(mx - 4.5, ruler_rect.top() + 2.0),
                                    egui::pos2(mx + 4.5, ruler_rect.top() + 2.0),
                                    egui::pos2(mx + 4.5, ruler_rect.top() + 10.0),
                                    egui::pos2(mx, ruler_rect.top() + 15.0),
                                    egui::pos2(mx - 4.5, ruler_rect.top() + 10.0),
                                ];
                                painter.add(egui::Shape::convex_polygon(
                                    marker_pts,
                                    m_color,
                                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                                ));
                                if !marker.label.is_empty() {
                                    painter.text(
                                        egui::pos2(mx + 6.0, ruler_rect.top() + 3.0),
                                        egui::Align2::LEFT_TOP,
                                        &marker.label,
                                        egui::FontId::proportional(9.0),
                                        egui::Color32::from_rgb(230, 230, 240),
                                    );
                                }
                            }

                            // ------------------------------------------
                            // 2. TRACK ROWS (Exact match with Left Rows)
                            // ------------------------------------------
                            let mut clicked_selection = None;
                            let mut clicked_keyframe = false;
                            let mut curve_handle_edit: Option<(usize, String, usize, CurveHandle, f32)> = None;

                            for (row_idx, row) in rows.iter().enumerate() {
                                let row_y = track_rect.min.y + TIMELINE_HEADER_HEIGHT + row.y_top;
                                let row_rect = egui::Rect::from_min_size(
                                    egui::pos2(track_rect.min.x, row_y),
                                    egui::vec2(timeline_track_width, row.row_height),
                                );

                                let is_active_layer = comp.active_layer_index == Some(row.layer_index);

                                // Background zebra matching left side
                                let bg_color = if is_active_layer {
                                    egui::Color32::from_rgb(28, 48, 72)
                                } else if row_idx % 2 == 0 {
                                    egui::Color32::from_rgb(22, 22, 25)
                                } else {
                                    egui::Color32::from_rgb(25, 25, 28)
                                };
                                painter.rect_filled(row_rect, 0.0, bg_color);
                                painter.line_segment(
                                    [row_rect.left_bottom(), row_rect.right_bottom()],
                                    egui::Stroke::new(0.5, egui::Color32::from_rgb(38, 38, 42)),
                                );

                                if row.is_layer {
                                    if let Some(layer) = comp.layers.get(row.layer_index) {
                                        // Draw Layer Duration Span Bar
                                        let in_x = track_rect.left() + (layer.in_time * pps);
                                        let out_x = track_rect.left() + (layer.out_time.min(timeline_duration) * pps);
                                        let bar_rect = egui::Rect::from_min_max(
                                            egui::pos2(in_x, row_rect.top() + 3.0),
                                            egui::pos2(out_x.max(in_x + 6.0), row_rect.bottom() - 3.0),
                                        );

                                        let label_col = get_label_color(layer.label_color_index);
                                        let bar_fill = egui::Color32::from_rgba_unmultiplied(
                                            label_col.r(),
                                            label_col.g(),
                                            label_col.b(),
                                            160,
                                        );

                                        painter.rect_filled(bar_rect, 2.0, bar_fill);
                                        painter.rect_stroke(
                                            bar_rect,
                                            2.0,
                                            egui::Stroke::new(1.0, label_col),
                                            egui::StrokeKind::Inside,
                                        );

                                        // Left Trim Handle
                                        painter.rect_filled(
                                            egui::Rect::from_min_max(
                                                egui::pos2(bar_rect.left(), bar_rect.top()),
                                                egui::pos2(bar_rect.left() + 3.0, bar_rect.bottom()),
                                            ),
                                            1.0,
                                            label_col,
                                        );

                                        // Right Trim Handle
                                        painter.rect_filled(
                                            egui::Rect::from_min_max(
                                                egui::pos2(bar_rect.right() - 3.0, bar_rect.top()),
                                                egui::pos2(bar_rect.right(), bar_rect.bottom()),
                                            ),
                                            1.0,
                                            label_col,
                                        );

                                        // Layer Name in Bar
                                        if bar_rect.width() > 30.0 {
                                            painter.text(
                                                egui::pos2(bar_rect.left() + 6.0, bar_rect.center().y),
                                                egui::Align2::LEFT_CENTER,
                                                &layer.name,
                                                egui::FontId::proportional(10.5),
                                                egui::Color32::WHITE,
                                            );
                                        }

                                        // Audio Waveform Visualization
                                        if let LayerSource::Audio { .. } = &layer.source {
                                            let vol = layer.properties.get("audioVolume").map(|p| p.base_value).unwrap_or(100.0).clamp(0.0, 200.0) / 100.0;
                                            let num_bars = ((bar_rect.width() / 3.5).floor() as usize).max(2);
                                            let mid_y = bar_rect.center().y;
                                            for b in 0..num_bars {
                                                let bx = bar_rect.left() + b as f32 * 3.5 + 2.0;
                                                let t = layer.in_time + (b as f32 / num_bars as f32) * (layer.out_time - layer.in_time);
                                                let s1 = (t * 31.0).sin().abs();
                                                let s2 = (t * 73.0 + 0.6).sin().abs();
                                                let s3 = (t * 142.0 + 1.4).cos().abs();
                                                let s4 = (t * 220.0 + 2.1).sin().abs();
                                                let amp = (s1 * 0.35 + s2 * 0.35 + s3 * 0.2 + s4 * 0.1) * (bar_rect.height() * 0.42) * vol;
                                                let bar_color = if b % 4 == 0 {
                                                    egui::Color32::from_rgba_unmultiplied(80, 240, 220, 220)
                                                } else {
                                                    egui::Color32::from_rgba_unmultiplied(60, 200, 180, 160)
                                                };
                                                painter.line_segment(
                                                    [egui::pos2(bx, mid_y - amp), egui::pos2(bx, mid_y + amp)],
                                                    egui::Stroke::new(1.5, bar_color),
                                                );
                                            }
                                            // Center baseline
                                            painter.line_segment(
                                                [egui::pos2(bar_rect.left(), mid_y), egui::pos2(bar_rect.right(), mid_y)],
                                                egui::Stroke::new(0.5, egui::Color32::from_rgba_unmultiplied(40, 180, 160, 100)),
                                            );
                                        }

                                        // Layer Markers
                                        for lm in &layer.markers {
                                            let lmx = track_rect.left() + (lm.time * pps);
                                            if lmx >= bar_rect.left() && lmx <= bar_rect.right() {
                                                let lm_col = get_label_color(lm.color_index);
                                                painter.circle_filled(egui::pos2(lmx, bar_rect.top() + 3.0), 2.5, lm_col);
                                                painter.line_segment(
                                                    [egui::pos2(lmx, bar_rect.top()), egui::pos2(lmx, bar_rect.bottom())],
                                                    egui::Stroke::new(1.0, lm_col),
                                                );
                                            }
                                        }

                                        // Summary keyframe diamonds on collapsed layer bar
                                        if layer.collapsed {
                                            for (prop_name, prop) in &layer.properties {
                                                for (kf_idx, kf) in prop.keyframes.iter().enumerate() {
                                                    let kf_x = track_rect.left() + (kf.time * pps);
                                                    let kf_y = row_rect.center().y;
                                                    let is_sel = selected.as_ref().is_some_and(|s| {
                                                        s.layer_index == row.layer_index
                                                            && s.property_name == *prop_name
                                                            && s.keyframe_index == kf_idx
                                                    });
                                                    let d_size = if is_sel { 4.5 } else { 3.5 };
                                                    let diamond = vec![
                                                        egui::pos2(kf_x, kf_y - d_size),
                                                        egui::pos2(kf_x + d_size, kf_y),
                                                        egui::pos2(kf_x, kf_y + d_size),
                                                        egui::pos2(kf_x - d_size, kf_y),
                                                    ];
                                                    painter.add(egui::Shape::convex_polygon(
                                                        diamond,
                                                        if is_sel { egui::Color32::WHITE } else { egui::Color32::from_rgb(255, 215, 60) },
                                                        egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 40, 10)),
                                                    ));

                                                    if track_res.clicked() {
                                                        if let Some(pos) = track_res.interact_pointer_pos() {
                                                            if pos.distance(egui::pos2(kf_x, kf_y)) <= 8.0 {
                                                                clicked_keyframe = true;
                                                                clicked_selection = Some(SelectedKeyframe {
                                                                    layer_index: row.layer_index,
                                                                    property_name: prop_name.clone(),
                                                                    keyframe_index: kf_idx,
                                                                    handle: None,
                                                                });
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else if let Some(prop_name) = &row.property_name {
                                    if let Some(layer) = comp.layers.get(row.layer_index) {
                                        if let Some(prop) = layer.properties.get(prop_name) {
                                            if comp.show_curves {
                                                // ==========================================
                                                // CURVE / GRAPH EDITOR TRACK
                                                // ==========================================
                                                let row_center_y = row_rect.center().y;

                                                match comp.graph_mode {
                                                    GraphMode::ValueGraph => {
                                                        painter.line_segment(
                                                            [egui::pos2(track_rect.left(), row_center_y), egui::pos2(track_rect.right(), row_center_y)],
                                                            egui::Stroke::new(0.5, egui::Color32::from_gray(38)),
                                                        );

                                                        let mut min_val = prop.base_value;
                                                        let mut max_val = prop.base_value;
                                                        for kf in &prop.keyframes {
                                                            min_val = min_val.min(kf.value);
                                                            max_val = max_val.max(kf.value);
                                                        }
                                                        let val_range = (max_val - min_val).max(1.0);

                                                        let to_screen_y = |val: f32| {
                                                            let t = (val - min_val) / val_range;
                                                            row_rect.bottom() - (t * (row.row_height - 14.0) + 7.0)
                                                        };

                                                        // Value labels along the track left
                                                        painter.text(
                                                            egui::pos2(track_rect.left() + 4.0, row_rect.top() + 8.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            format!("max: {:.1}", max_val),
                                                            egui::FontId::proportional(9.0),
                                                            egui::Color32::from_gray(100),
                                                        );
                                                        painter.text(
                                                            egui::pos2(track_rect.left() + 4.0, row_rect.bottom() - 8.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            format!("min: {:.1}", min_val),
                                                            egui::FontId::proportional(9.0),
                                                            egui::Color32::from_gray(100),
                                                        );

                                                        for (keyframe_index, curr) in prop.keyframes.iter().enumerate() {
                                                            let x1 = track_rect.left() + (curr.time * pps);
                                                            let y1 = to_screen_y(curr.value);

                                                            if let Some(next) = prop.keyframes.get(keyframe_index + 1) {
                                                                let x2 = track_rect.left() + (next.time * pps);
                                                                let y2 = to_screen_y(next.value);

                                                                let curve = curr.ease.unwrap_or(BezierControl {
                                                                    cp1: 0.33,
                                                                    cp2: 0.67,
                                                                });

                                                                let p1 = egui::pos2(x1, y1);
                                                                let p2 = egui::pos2(x1 + (x2 - x1) * curve.cp1, y1);
                                                                let p3 = egui::pos2(x1 + (x2 - x1) * curve.cp2, y2);
                                                                let p4 = egui::pos2(x2, y2);

                                                                painter.add(egui::Shape::CubicBezier(
                                                                    egui::epaint::CubicBezierShape {
                                                                        points: [p1, p2, p3, p4],
                                                                        closed: false,
                                                                        fill: egui::Color32::TRANSPARENT,
                                                                        stroke: egui::epaint::PathStroke::new(
                                                                            2.0,
                                                                            egui::Color32::from_rgb(255, 215, 75),
                                                                        ),
                                                                    },
                                                                ));

                                                                let is_selected = selected.as_ref().is_some_and(|sel| {
                                                                    sel.layer_index == row.layer_index
                                                                        && sel.property_name == *prop_name
                                                                        && sel.keyframe_index == keyframe_index
                                                                });

                                                                if is_selected {
                                                                    painter.line_segment([p1, p2], egui::Stroke::new(1.2, egui::Color32::from_rgb(100, 180, 255)));
                                                                    painter.circle_filled(p2, 4.0, egui::Color32::from_rgb(0, 190, 255));
                                                                    painter.circle_stroke(p2, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE));

                                                                    let next_p1 = egui::pos2(x2, y2);
                                                                    let next_p2 = p3;
                                                                    painter.line_segment([next_p1, next_p2], egui::Stroke::new(1.2, egui::Color32::from_rgb(100, 180, 255)));
                                                                    painter.circle_filled(next_p2, 4.0, egui::Color32::from_rgb(0, 190, 255));
                                                                    painter.circle_stroke(next_p2, 4.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
                                                                }

                                                                if track_res.clicked() {
                                                                    if let Some(pos) = track_res.interact_pointer_pos() {
                                                                        if pos.distance(p2) <= 8.0 {
                                                                            clicked_keyframe = true;
                                                                            clicked_selection = Some(SelectedKeyframe {
                                                                                layer_index: row.layer_index,
                                                                                property_name: prop_name.clone(),
                                                                                keyframe_index,
                                                                                handle: Some(CurveHandle::Out),
                                                                            });
                                                                        } else if pos.distance(p3) <= 8.0 {
                                                                            clicked_keyframe = true;
                                                                            clicked_selection = Some(SelectedKeyframe {
                                                                                layer_index: row.layer_index,
                                                                                property_name: prop_name.clone(),
                                                                                keyframe_index,
                                                                                handle: Some(CurveHandle::In),
                                                                            });
                                                                        }
                                                                    }
                                                                }

                                                                if track_res.dragged() {
                                                                    if let (Some(sel), Some(pos)) = (selected.as_ref(), track_res.interact_pointer_pos()) {
                                                                        if sel.layer_index == row.layer_index
                                                                            && sel.property_name == *prop_name
                                                                            && sel.keyframe_index == keyframe_index
                                                                        {
                                                                            let denom = (x2 - x1).abs().max(1.0);
                                                                            match sel.handle {
                                                                                Some(CurveHandle::Out) => {
                                                                                    curve_handle_edit = Some((
                                                                                        row.layer_index,
                                                                                        prop_name.clone(),
                                                                                        keyframe_index,
                                                                                        CurveHandle::Out,
                                                                                        ((pos.x - x1) / denom).clamp(0.0, 1.0),
                                                                                    ));
                                                                                }
                                                                                Some(CurveHandle::In) => {
                                                                                    curve_handle_edit = Some((
                                                                                        row.layer_index,
                                                                                        prop_name.clone(),
                                                                                        keyframe_index,
                                                                                        CurveHandle::In,
                                                                                        ((pos.x - x1) / denom).clamp(0.0, 1.0),
                                                                                    ));
                                                                                }
                                                                                None => {}
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }

                                                            // Draw keyframe vertex point
                                                            let is_selected = selected.as_ref().is_some_and(|sel| {
                                                                sel.layer_index == row.layer_index
                                                                    && sel.property_name == *prop_name
                                                                    && sel.keyframe_index == keyframe_index
                                                            });

                                                            painter.circle_filled(
                                                                egui::pos2(x1, y1),
                                                                if is_selected { 5.0 } else { 3.5 },
                                                                if is_selected { egui::Color32::WHITE } else { egui::Color32::from_rgb(255, 215, 75) },
                                                            );
                                                            if is_selected {
                                                                painter.circle_stroke(
                                                                    egui::pos2(x1, y1),
                                                                    6.5,
                                                                    egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 180, 255)),
                                                                );
                                                            }

                                                            if track_res.clicked() {
                                                                if let Some(pos) = track_res.interact_pointer_pos() {
                                                                    if pos.distance(egui::pos2(x1, y1)) <= 9.0 {
                                                                        clicked_keyframe = true;
                                                                        clicked_selection = Some(SelectedKeyframe {
                                                                            layer_index: row.layer_index,
                                                                            property_name: prop_name.clone(),
                                                                            keyframe_index,
                                                                            handle: None,
                                                                        });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                    GraphMode::SpeedGraph => {
                                                        let base_y = row_rect.bottom() - 6.0;
                                                        painter.line_segment(
                                                            [egui::pos2(track_rect.left(), base_y), egui::pos2(track_rect.right(), base_y)],
                                                            egui::Stroke::new(1.0, egui::Color32::from_gray(50)),
                                                        );

                                                        let mut max_speed = 10.0f32;
                                                        for (keyframe_index, curr) in prop.keyframes.iter().enumerate() {
                                                            if let Some(next) = prop.keyframes.get(keyframe_index + 1) {
                                                                let dt = (next.time - curr.time).abs().max(0.001);
                                                                let dv = (next.value - curr.value).abs();
                                                                let avg_speed = dv / dt;
                                                                max_speed = max_speed.max(avg_speed * 2.2);
                                                            }
                                                        }

                                                        painter.text(
                                                            egui::pos2(track_rect.left() + 4.0, row_rect.top() + 8.0),
                                                            egui::Align2::LEFT_CENTER,
                                                            format!("peak: {:.1} /s", max_speed),
                                                            egui::FontId::proportional(9.0),
                                                            egui::Color32::from_rgb(255, 160, 40),
                                                        );

                                                        let to_speed_y = |spd: f32| {
                                                            let t = (spd / max_speed).clamp(0.0, 1.0);
                                                            row_rect.bottom() - (t * (row.row_height - 14.0) + 7.0)
                                                        };

                                                        for (keyframe_index, curr) in prop.keyframes.iter().enumerate() {
                                                            let x1 = track_rect.left() + (curr.time * pps);

                                                            if let Some(next) = prop.keyframes.get(keyframe_index + 1) {
                                                                let x2 = track_rect.left() + (next.time * pps);
                                                                let dt = (next.time - curr.time).abs().max(0.001);
                                                                let dv = (next.value - curr.value).abs();
                                                                let avg_speed = dv / dt;

                                                                let curve = curr.ease.unwrap_or(BezierControl {
                                                                    cp1: 0.33,
                                                                    cp2: 0.67,
                                                                });

                                                                let steps = 24;
                                                                let mut speed_pts = Vec::with_capacity(steps + 1);

                                                                for s in 0..=steps {
                                                                    let u = s as f32 / steps as f32;
                                                                    let px = x1 + u * (x2 - x1);
                                                                    let shape = (u * std::f32::consts::PI).sin().powf(1.0 + (curve.cp1 - curve.cp2).abs() * 2.0);
                                                                    let speed_val = avg_speed * shape * (1.5 + (curve.cp2 - curve.cp1).abs());
                                                                    let py = to_speed_y(speed_val);
                                                                    speed_pts.push(egui::pos2(px, py));
                                                                }

                                                                for window in speed_pts.windows(2) {
                                                                    painter.line_segment(
                                                                        [window[0], window[1]],
                                                                        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 160, 40)),
                                                                    );
                                                                }
                                                            }

                                                            painter.circle_filled(
                                                                egui::pos2(x1, base_y),
                                                                3.5,
                                                                egui::Color32::from_rgb(255, 160, 40),
                                                            );
                                                        }
                                                    }
                                                }
                                            } else {
                                                // ==========================================
                                                // STANDARD KEYFRAME TRACK
                                                // ==========================================
                                                let row_center_y = row_rect.center().y;
                                                painter.line_segment(
                                                    [egui::pos2(track_rect.left(), row_center_y), egui::pos2(track_rect.right(), row_center_y)],
                                                    egui::Stroke::new(0.5, egui::Color32::from_gray(38)),
                                                );

                                                // Interpolation line between keyframes
                                                for (keyframe_index, curr) in prop.keyframes.iter().enumerate() {
                                                    let x1 = track_rect.left() + (curr.time * pps);
                                                    if let Some(next) = prop.keyframes.get(keyframe_index + 1) {
                                                        let x2 = track_rect.left() + (next.time * pps);
                                                        painter.line_segment(
                                                            [egui::pos2(x1, row_center_y), egui::pos2(x2, row_center_y)],
                                                            egui::Stroke::new(1.2, egui::Color32::from_rgb(180, 150, 60)),
                                                        );
                                                    }

                                                    let is_selected = selected.as_ref().is_some_and(|sel| {
                                                        sel.layer_index == row.layer_index
                                                            && sel.property_name == *prop_name
                                                            && sel.keyframe_index == keyframe_index
                                                    });

                                                    let diamond_size = if is_selected { 5.5 } else { 4.0 };
                                                    let color = if is_selected {
                                                        egui::Color32::from_rgb(255, 255, 255)
                                                    } else {
                                                        egui::Color32::from_rgb(235, 185, 45)
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
                                                        egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 40, 10)),
                                                    ));

                                                    if track_res.clicked() {
                                                        if let Some(pos) = track_res.interact_pointer_pos() {
                                                            if pos.distance(egui::pos2(x1, row_center_y)) <= 8.0 {
                                                                clicked_keyframe = true;
                                                                clicked_selection = Some(SelectedKeyframe {
                                                                    layer_index: row.layer_index,
                                                                    property_name: prop_name.clone(),
                                                                    keyframe_index,
                                                                    handle: None,
                                                                });
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if track_res.clicked() {
                                *selected = clicked_selection;
                            }

                            if let Some((layer_index, property_name, keyframe_index, handle, value)) =
                                curve_handle_edit
                            {
                                if let Some(keyframe) = comp
                                    .layers
                                    .get_mut(layer_index)
                                    .and_then(|layer| layer.properties.get_mut(&property_name))
                                    .and_then(|prop| prop.keyframes.get_mut(keyframe_index))
                                {
                                    let ease = keyframe.ease.get_or_insert(BezierControl {
                                        cp1: 0.33,
                                        cp2: 0.67,
                                    });
                                    match handle {
                                        CurveHandle::Out => ease.cp1 = value,
                                        CurveHandle::In => ease.cp2 = value,
                                    }
                                }
                            }

                            // ------------------------------------------
                            // 3. PLAYHEAD / CTI SCRUBBER
                            // ------------------------------------------
                            let cti_x = track_rect.left() + (comp.current_time * pps);
                            painter.line_segment(
                                [egui::pos2(cti_x, track_rect.top()), egui::pos2(cti_x, track_rect.bottom())],
                                egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 210, 255)),
                            );

                            // Playhead CTI Head (Adobe blue triangle marker)
                            let head_pts = vec![
                                egui::pos2(cti_x - 5.0, track_rect.top()),
                                egui::pos2(cti_x + 5.0, track_rect.top()),
                                egui::pos2(cti_x + 5.0, track_rect.top() + 7.0),
                                egui::pos2(cti_x, track_rect.top() + 13.0),
                                egui::pos2(cti_x - 5.0, track_rect.top() + 7.0),
                            ];
                            painter.add(egui::Shape::convex_polygon(
                                head_pts,
                                egui::Color32::from_rgb(0, 185, 255),
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(20, 80, 120)),
                            ));

                            // Drag playhead
                            if (track_res.dragged() || track_res.clicked()) && !clicked_keyframe {
                                if let Some(pos) = track_res.interact_pointer_pos() {
                                    let new_time = ((pos.x - track_rect.left()) / pps).clamp(0.0, timeline_duration);
                                    comp.current_time = snap_time(comp, new_time, 10.0 / pps);
                                }
                            }
                        });

                    comp.timeline_scroll_h = scroll_h.state.offset.x;
                });
            });
        });

    comp.timeline_scroll_v = scroll_v.state.offset.y;
}
