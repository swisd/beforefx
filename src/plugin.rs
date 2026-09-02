use crate::core::*;
use macroquad::color::Color;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    Effect,
    Functional,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginSlider {
    pub name: String,
    pub display_name: String,
    pub default_value: f32,
    pub min_value: f32,
    pub max_value: f32,
    pub step: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectPlugin {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub file_path: String,
    pub sliders: Vec<PluginSlider>,
    pub formula_lines: Vec<String>,
    pub builtin_type: Option<String>,
    pub glsl_shader: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionalPlugin {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub file_path: String,
    pub action: String,
    pub script_commands: Vec<String>,
    pub sliders: Vec<PluginSlider>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Plugin {
    Effect(EffectPlugin),
    Functional(FunctionalPlugin),
}

#[derive(Debug, Clone, Default)]
pub struct PluginRegistry {
    pub effects: Vec<EffectPlugin>,
    pub functionals: Vec<FunctionalPlugin>,
    pub load_errors: Vec<(String, String)>,
    pub plugins_dir: PathBuf,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let plugins_dir = PathBuf::from("./plugins");
        let mut reg = Self {
            effects: Vec::new(),
            functionals: Vec::new(),
            load_errors: Vec::new(),
            plugins_dir,
        };
        reg.ensure_default_plugins();
        reg.reload();
        reg
    }

    pub fn reload(&mut self) {
        self.effects.clear();
        self.functionals.clear();
        self.load_errors.clear();

        if !self.plugins_dir.exists() {
            let _ = fs::create_dir_all(&self.plugins_dir);
        }

        if let Ok(entries) = fs::read_dir(&self.plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    match parse_plugin_file(&path) {
                        Ok(Some(Plugin::Effect(eff))) => {
                            self.effects.push(eff);
                        }
                        Ok(Some(Plugin::Functional(func))) => {
                            self.functionals.push(func);
                        }
                        Ok(None) => {
                            // Not a plugin file (e.g. no .spec header)
                        }
                        Err(e) => {
                            self.load_errors.push((
                                path.file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string(),
                                e,
                            ));
                        }
                    }
                }
            }
        }

        // Sort plugins alphabetically by category then name
        self.effects.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.name.cmp(&b.name))
        });
        self.functionals.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    pub fn get_effect(&self, name: &str) -> Option<&EffectPlugin> {
        self.effects.iter().find(|e| e.name == name || e.id == name)
    }

    #[allow(dead_code)]
    pub fn get_functional(&self, name: &str) -> Option<&FunctionalPlugin> {
        self.functionals.iter().find(|f| f.name == name || f.id == name)
    }

    pub fn ensure_default_plugins(&self) {
        if !self.plugins_dir.exists() {
            let _ = fs::create_dir_all(&self.plugins_dir);
        }

        // Write sample plugins if they don't already exist
        let default_plugins = [
            (
                "sepia_tone.bfxplugin",
                r#".spec effect
name: Sepia Tone
category: Color Correction
description: Gives footage or layers a warm nostalgic sepia tint.
slider: intensity, 80.0, 0.0, 100.0, 1.0
slider: tone_r, 1.2, 0.0, 2.0, 0.05
slider: tone_g, 1.0, 0.0, 2.0, 0.05
slider: tone_b, 0.75, 0.0, 2.0, 0.05

// Color formula for Sepia
gray = r * 0.299 + g * 0.587 + b * 0.114;
mix_amt = intensity / 100.0;
r = mix(r, clamp(gray * tone_r, 0.0, 1.0), mix_amt);
g = mix(g, clamp(gray * tone_g, 0.0, 1.0), mix_amt);
b = mix(b, clamp(gray * tone_b, 0.0, 1.0), mix_amt);
"#,
            ),
            (
                "mp4_ultra_compress.bfxplugin",
                r#".spec effect
name: MP4 Ultra Compress & Corrupt
category: Glitch & Retro
description: Extreme video codec compression artifacts, macroblocking, DCT quantization, chroma bleeding, and datamosh corruption.
slider: block_size, 16.0, 2.0, 64.0, 2.0
slider: compression, 75.0, 0.0, 100.0, 1.0
slider: chroma_loss, 60.0, 0.0, 100.0, 1.0
slider: corruption, 30.0, 0.0, 100.0, 1.0
slider: temporal_jitter, 25.0, 0.0, 100.0, 1.0
slider: noise_dither, 15.0, 0.0, 100.0, 1.0
slider: mix_amount, 100.0, 0.0, 100.0, 1.0

type: mp4_ultra_compress
"#,
            ),
            (
                "rgb_channel_shift.bfxplugin",
                r#".spec effect
name: RGB Channel Shift
category: Distort & Stylize
description: Shifts red and blue color channels for a chromatic glitch look.
slider: red_shift, 15.0, -100.0, 100.0, 1.0
slider: blue_shift, -15.0, -100.0, 100.0, 1.0
slider: mix_amount, 100.0, 0.0, 100.0, 1.0

type: rgb_split
"#,
            ),
            (
                "pixelate_mosaic.bfxplugin",
                r#".spec effect
name: Pixelate Mosaic
category: Stylize
description: Creates a retro pixel art mosaic look.
slider: pixel_size, 8.0, 1.0, 64.0, 1.0
slider: blend, 100.0, 0.0, 100.0, 1.0

type: pixelate
"#,
            ),
            (
                "vignette_grain.bfxplugin",
                r#".spec effect
name: Film Vignette & Grain
category: Stylize
description: Adds film grain texture and optical edge shading.
slider: vignette_amount, 60.0, 0.0, 100.0, 1.0
slider: grain_intensity, 15.0, 0.0, 100.0, 1.0
slider: contrast_boost, 10.0, -50.0, 50.0, 1.0

type: vignette_grain
"#,
            ),
            (
                "color_grading_lut.bfxplugin",
                r#".spec effect
name: Cinematic Grade
category: Color Correction
description: Applies a cinematic teal and orange color grading look.
slider: teal_shadows, 40.0, 0.0, 100.0, 1.0
slider: orange_highlights, 45.0, 0.0, 100.0, 1.0
slider: contrast, 20.0, -50.0, 50.0, 1.0
slider: saturation, 15.0, -100.0, 100.0, 1.0

// Cinematic teal & orange grading formula
lum = r * 0.299 + g * 0.587 + b * 0.114;
teal_factor = (1.0 - lum) * (teal_shadows / 100.0);
orange_factor = lum * (orange_highlights / 100.0);
c_factor = (1.0 + contrast / 100.0);

r = ((r - 0.5) * c_factor + 0.5 + orange_factor * 0.3 - teal_factor * 0.2);
g = ((g - 0.5) * c_factor + 0.5 + orange_factor * 0.1);
b = ((b - 0.5) * c_factor + 0.5 + teal_factor * 0.4 - orange_factor * 0.2);

sat_boost = 1.0 + saturation / 100.0;
gray = r * 0.299 + g * 0.587 + b * 0.114;
r = mix(gray, r, sat_boost);
g = mix(gray, g, sat_boost);
b = mix(gray, b, sat_boost);
"#,
            ),
            (
                "create_camera_rig.bfxplugin",
                r#".spec functional
name: Create 3D Camera Rig
category: Cameras & 3D
description: Creates a 3D Camera with an Orbit Null Controller and automated motion keyframes.
action: add_camera_rig
slider: distance, 1800.0, 500.0, 5000.0, 50.0
slider: orbit_speed, 1.0, 0.1, 10.0, 0.1
"#,
            ),
            (
                "stagger_layers.bfxplugin",
                r#".spec functional
name: Stagger Layers
category: Animation Tools
description: Staggers layer in-points across the timeline in sequential steps.
action: stagger_layers
slider: offset_seconds, 0.25, 0.01, 2.0, 0.05
slider: reverse_order, 0.0, 0.0, 1.0, 1.0
"#,
            ),
            (
                "easy_ease_all.bfxplugin",
                r#".spec functional
name: Easy Ease All Keyframes
category: Animation Tools
description: Converts all keyframes in the active layer to smooth cubic Bezier Easy Ease.
action: easy_ease_all
"#,
            ),
            (
                "add_adjustment_fx.bfxplugin",
                r#".spec functional
name: Add Master Adjustment FX
category: Layers & FX
description: Creates a full-composition adjustment layer and attaches standard mastering FX.
action: add_adjustment_layer
"#,
            ),
            (
                "color_palette_solids.bfxplugin",
                r#".spec functional
name: Generate Color Palette Solids
category: Design & Utilities
description: Generates a balanced 5-color aesthetic solid palette across the composition.
action: create_palette_solids
"#,
            ),
        ];

        for (filename, content) in default_plugins {
            let p = self.plugins_dir.join(filename);
            if !p.exists() {
                let _ = fs::write(&p, content);
            }
        }
    }
}

/// Parses a plugin file based on `.spec effect` or `.spec functional` header
pub fn parse_plugin_file(path: &Path) -> Result<Option<Plugin>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    parse_plugin_content(&content, path.to_string_lossy().as_ref())
}

pub fn parse_plugin_content(content: &str, file_path: &str) -> Result<Option<Plugin>, String> {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Ok(None);
    }

    // Look for .spec directive in the first 25 non-empty lines
    let mut kind: Option<PluginKind> = None;
    for line in lines.iter().take(25) {
        let trimmed = clean_comment(line).trim();
        if trimmed.starts_with(".spec") || trimmed.starts_with("spec:") {
            let spec_val = trimmed
                .trim_start_matches(".spec")
                .trim_start_matches("spec:")
                .trim()
                .to_lowercase();
            if spec_val.contains("effect") {
                kind = Some(PluginKind::Effect);
                break;
            } else if spec_val.contains("functional") || spec_val.contains("function") || spec_val.contains("tool") {
                kind = Some(PluginKind::Functional);
                break;
            }
        }
    }

    let kind = match kind {
        Some(k) => k,
        None => return Ok(None),
    };

    let default_name = Path::new(file_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Custom Plugin".to_string())
        .replace('_', " ");

    let mut name = String::new();
    let mut category = String::new();
    let mut description = String::new();
    let mut action = String::new();
    let mut builtin_type = None;
    let mut sliders = Vec::new();
    let mut formula_lines = Vec::new();
    let mut script_commands = Vec::new();
    let mut glsl_lines = Vec::new();
    let mut in_glsl = false;

    for raw_line in &lines {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("@glsl") {
            in_glsl = true;
            continue;
        }
        if line.starts_with("@end") {
            in_glsl = false;
            continue;
        }
        if in_glsl {
            glsl_lines.push(raw_line.to_string());
            continue;
        }

        let clean = clean_comment(line).trim().to_string();
        if clean.is_empty() {
            continue;
        }

        if clean.starts_with(".spec") || clean.starts_with("spec:") {
            continue;
        }

        // Key-value parsing
        if let Some(rest) = strip_prefix_ci(&clean, "name:")
            .or_else(|| strip_prefix_ci(&clean, ".name"))
        {
            name = rest.trim().to_string();
            continue;
        }

        if let Some(rest) = strip_prefix_ci(&clean, "category:")
            .or_else(|| strip_prefix_ci(&clean, ".category"))
        {
            category = rest.trim().to_string();
            continue;
        }

        if let Some(rest) = strip_prefix_ci(&clean, "description:")
            .or_else(|| strip_prefix_ci(&clean, ".description"))
            .or_else(|| strip_prefix_ci(&clean, "desc:"))
        {
            description = rest.trim().to_string();
            continue;
        }

        if let Some(rest) = strip_prefix_ci(&clean, "action:")
            .or_else(|| strip_prefix_ci(&clean, ".action"))
        {
            action = rest.trim().to_string();
            continue;
        }

        if let Some(rest) = strip_prefix_ci(&clean, "type:")
            .or_else(|| strip_prefix_ci(&clean, ".type"))
        {
            builtin_type = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = strip_prefix_ci(&clean, "slider:")
            .or_else(|| strip_prefix_ci(&clean, "property:"))
            .or_else(|| strip_prefix_ci(&clean, "param:"))
            .or_else(|| strip_prefix_ci(&clean, ".slider"))
            .or_else(|| strip_prefix_ci(&clean, ".property"))
        {
            if let Some(slider) = parse_slider_def(rest) {
                sliders.push(slider);
            }
            continue;
        }

        if kind == PluginKind::Functional {
            script_commands.push(clean.clone());
        } else {
            if clean.contains('=') || clean.starts_with("let ") || clean.contains(';') {
                formula_lines.push(clean.clone());
            }
        }
    }

    if name.is_empty() {
        name = default_name;
    }
    if category.is_empty() {
        category = match kind {
            PluginKind::Effect => "Plugins".to_string(),
            PluginKind::Functional => "Utilities".to_string(),
        };
    }

    let id = sanitize_id(&name);

    match kind {
        PluginKind::Effect => Ok(Some(Plugin::Effect(EffectPlugin {
            id,
            name,
            category,
            description,
            file_path: file_path.to_string(),
            sliders,
            formula_lines,
            builtin_type,
            glsl_shader: if glsl_lines.is_empty() {
                None
            } else {
                Some(glsl_lines.join("\n"))
            },
        }))),
        PluginKind::Functional => Ok(Some(Plugin::Functional(FunctionalPlugin {
            id,
            name,
            category,
            description,
            file_path: file_path.to_string(),
            action,
            script_commands,
            sliders,
        }))),
    }
}

fn clean_comment(line: &str) -> &str {
    let mut s = line.trim();
    while s.starts_with("//") || s.starts_with('#') || s.starts_with("--") || s.starts_with("/*") || s.starts_with('*') {
        if s.starts_with("//") || s.starts_with("--") || s.starts_with("/*") {
            s = &s[2..];
        } else {
            s = &s[1..];
        }
        s = s.trim_start();
    }
    s
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

fn parse_slider_def(def: &str) -> Option<PluginSlider> {
    // Formats supported:
    // slider: name, default, min, max, step
    // slider: name default min max
    // slider: name = default
    let parts: Vec<&str> = if def.contains(',') {
        def.split(',').map(|s| s.trim()).collect()
    } else if def.contains('=') {
        let kv: Vec<&str> = def.split('=').map(|s| s.trim()).collect();
        return Some(PluginSlider {
            name: kv[0].to_string(),
            display_name: format_display_name(kv[0]),
            default_value: kv.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0),
            min_value: -100.0,
            max_value: 100.0,
            step: 1.0,
        });
    } else {
        def.split_whitespace().collect()
    };

    if parts.is_empty() {
        return None;
    }

    let name = parts[0].to_string();
    let display_name = format_display_name(&name);
    let default_value = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let min_value = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let max_value = parts.get(3).and_then(|v| v.parse().ok()).unwrap_or(100.0);
    let step = parts.get(4).and_then(|v| v.parse().ok()).unwrap_or(1.0);

    Some(PluginSlider {
        name,
        display_name,
        default_value,
        min_value,
        max_value,
        step,
    })
}

pub fn format_display_name(raw: &str) -> String {
    if raw.contains('_') {
        raw.split('_')
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        let mut res = String::new();
        for (i, ch) in raw.chars().enumerate() {
            if i == 0 {
                res.extend(ch.to_uppercase());
            } else if ch.is_uppercase() {
                res.push(' ');
                res.push(ch);
            } else {
                res.push(ch);
            }
        }
        res
    }
}

pub fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect()
}

// ------------------------------------------------------------------------------------------------
// Effect Plugin Evaluator
// ------------------------------------------------------------------------------------------------

pub fn apply_effect_plugin(
    mut col: Color,
    plugin: &EffectPlugin,
    properties: &HashMap<String, Property>,
    time: f32,
) -> Color {
    // Check for built-in type shortcut first
    if let Some(ref bt) = plugin.builtin_type {
        match bt.to_lowercase().as_str() {
            "sepia" => {
                let intensity = properties.get("intensity").map_or(80.0, |p| p.get_value_at(time)) / 100.0;
                let tone_r = properties.get("tone_r").map_or(1.2, |p| p.get_value_at(time));
                let tone_g = properties.get("tone_g").map_or(1.0, |p| p.get_value_at(time));
                let tone_b = properties.get("tone_b").map_or(0.75, |p| p.get_value_at(time));
                let gray = col.r * 0.299 + col.g * 0.587 + col.b * 0.114;
                col.r = (col.r * (1.0 - intensity) + (gray * tone_r).clamp(0.0, 1.0) * intensity).clamp(0.0, 1.0);
                col.g = (col.g * (1.0 - intensity) + (gray * tone_g).clamp(0.0, 1.0) * intensity).clamp(0.0, 1.0);
                col.b = (col.b * (1.0 - intensity) + (gray * tone_b).clamp(0.0, 1.0) * intensity).clamp(0.0, 1.0);
                return col;
            }
            "mp4_ultra_compress" | "mp4_corrupt" | "mp4_ultracompress" | "mp4corrupt" => {
                let comp = properties.get("compression").map_or(75.0, |p| p.get_value_at(time)) / 100.0;
                let levels = (256.0 / (1.0 + comp * 31.0)).max(2.0);
                let corrupt = properties.get("corruption").map_or(30.0, |p| p.get_value_at(time)) / 100.0;
                let mix_amt = properties.get("mix_amount").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                let noise = ((time * 37.17 + col.r * 53.21).sin() * 43758.5453).fract() - 0.5;
                let q_r = (col.r * levels).round() / levels + noise * corrupt * 0.2;
                let q_g = (col.g * levels).round() / levels;
                let q_b = (col.b * levels).round() / levels - noise * corrupt * 0.2;
                col.r = (col.r * (1.0 - mix_amt) + q_r.clamp(0.0, 1.0) * mix_amt).clamp(0.0, 1.0);
                col.g = (col.g * (1.0 - mix_amt) + q_g.clamp(0.0, 1.0) * mix_amt).clamp(0.0, 1.0);
                col.b = (col.b * (1.0 - mix_amt) + q_b.clamp(0.0, 1.0) * mix_amt).clamp(0.0, 1.0);
                return col;
            }
            "rgb_split" | "rgb_channel_shift" => {
                let r_shift = properties.get("red_shift").map_or(15.0, |p| p.get_value_at(time)) / 100.0;
                let b_shift = properties.get("blue_shift").map_or(-15.0, |p| p.get_value_at(time)) / 100.0;
                let mix_amt = properties.get("mix_amount").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                let shifted_r = (col.r * (1.0 + r_shift * 0.5)).clamp(0.0, 1.0);
                let shifted_b = (col.b * (1.0 + b_shift * 0.5)).clamp(0.0, 1.0);
                col.r = col.r * (1.0 - mix_amt) + shifted_r * mix_amt;
                col.b = col.b * (1.0 - mix_amt) + shifted_b * mix_amt;
                return col;
            }
            "pixelate" => {
                let blend = properties.get("blend").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
                let size = properties.get("pixel_size").map_or(8.0, |p| p.get_value_at(time)).max(1.0);
                let steps = (32.0 / size).max(2.0);
                let quant_r = (col.r * steps).round() / steps;
                let quant_g = (col.g * steps).round() / steps;
                let quant_b = (col.b * steps).round() / steps;
                col.r = col.r * (1.0 - blend) + quant_r * blend;
                col.g = col.g * (1.0 - blend) + quant_g * blend;
                col.b = col.b * (1.0 - blend) + quant_b * blend;
                return col;
            }
            "vignette_grain" => {
                let vig = properties.get("vignette_amount").map_or(60.0, |p| p.get_value_at(time)) / 100.0;
                let grain = properties.get("grain_intensity").map_or(15.0, |p| p.get_value_at(time)) / 100.0;
                let c_boost = properties.get("contrast_boost").map_or(10.0, |p| p.get_value_at(time)) / 100.0;
                let pseudo_noise = ((time * 123.45 + col.r * 67.89 + col.g * 43.21).sin() * 43758.5453).fract() - 0.5;
                let factor = (1.0 + c_boost).max(0.0);
                col.r = ((col.r - 0.5) * factor + 0.5 + pseudo_noise * grain * 0.3 - vig * 0.15).clamp(0.0, 1.0);
                col.g = ((col.g - 0.5) * factor + 0.5 + pseudo_noise * grain * 0.3 - vig * 0.15).clamp(0.0, 1.0);
                col.b = ((col.b - 0.5) * factor + 0.5 + pseudo_noise * grain * 0.3 - vig * 0.15).clamp(0.0, 1.0);
                return col;
            }
            _ => {}
        }
    }

    if plugin.formula_lines.is_empty() {
        return col;
    }

    // Evaluate formula lines with environment variables
    let mut vars: HashMap<String, f32> = HashMap::new();
    vars.insert("r".to_string(), col.r);
    vars.insert("g".to_string(), col.g);
    vars.insert("b".to_string(), col.b);
    vars.insert("a".to_string(), col.a);
    vars.insert("time".to_string(), time);

    for slider in &plugin.sliders {
        let val = properties
            .get(&slider.name)
            .map_or(slider.default_value, |p| p.get_value_at(time));
        vars.insert(slider.name.clone(), val);
    }

    for raw_stmt in &plugin.formula_lines {
        let stmt = raw_stmt.trim().trim_end_matches(';');
        if stmt.is_empty() {
            continue;
        }

        if let Some((target_var, expr)) = stmt.split_once('=') {
            let target = target_var.trim().trim_start_matches("let ").trim();
            let val = evaluate_expression(expr.trim(), &vars);
            vars.insert(target.to_string(), val);
        }
    }

    if let Some(&r) = vars.get("r") {
        col.r = r.clamp(0.0, 1.0);
    }
    if let Some(&g) = vars.get("g") {
        col.g = g.clamp(0.0, 1.0);
    }
    if let Some(&b) = vars.get("b") {
        col.b = b.clamp(0.0, 1.0);
    }
    if let Some(&a) = vars.get("a") {
        col.a = a.clamp(0.0, 1.0);
    }

    col
}

/// Evaluates a math expression with variables and standard math functions
pub fn evaluate_expression(expr: &str, vars: &HashMap<String, f32>) -> f32 {
    let expr = expr.trim();
    if expr.is_empty() {
        return 0.0;
    }

    // Function calls
    if let Some((fn_name, args_str)) = parse_function_call(expr) {
        let args = split_args(args_str);
        let eval_args: Vec<f32> = args.iter().map(|a| evaluate_expression(a, vars)).collect();
        return match fn_name.to_lowercase().as_str() {
            "sin" => eval_args.first().copied().unwrap_or(0.0).sin(),
            "cos" => eval_args.first().copied().unwrap_or(0.0).cos(),
            "tan" => eval_args.first().copied().unwrap_or(0.0).tan(),
            "abs" => eval_args.first().copied().unwrap_or(0.0).abs(),
            "sqrt" => eval_args.first().copied().unwrap_or(0.0).max(0.0).sqrt(),
            "fract" => eval_args.first().copied().unwrap_or(0.0).fract(),
            "floor" => eval_args.first().copied().unwrap_or(0.0).floor(),
            "ceil" => eval_args.first().copied().unwrap_or(0.0).ceil(),
            "round" => eval_args.first().copied().unwrap_or(0.0).round(),
            "min" => {
                let a = eval_args.first().copied().unwrap_or(0.0);
                let b = eval_args.get(1).copied().unwrap_or(0.0);
                a.min(b)
            }
            "max" => {
                let a = eval_args.first().copied().unwrap_or(0.0);
                let b = eval_args.get(1).copied().unwrap_or(0.0);
                a.max(b)
            }
            "clamp" => {
                let val = eval_args.first().copied().unwrap_or(0.0);
                let min = eval_args.get(1).copied().unwrap_or(0.0);
                let max = eval_args.get(2).copied().unwrap_or(1.0);
                val.clamp(min, max)
            }
            "mix" | "lerp" => {
                let a = eval_args.first().copied().unwrap_or(0.0);
                let b = eval_args.get(1).copied().unwrap_or(0.0);
                let t = eval_args.get(2).copied().unwrap_or(0.0).clamp(0.0, 1.0);
                a * (1.0 - t) + b * t
            }
            "pow" => {
                let a = eval_args.first().copied().unwrap_or(0.0);
                let b = eval_args.get(1).copied().unwrap_or(1.0);
                a.powf(b)
            }
            "step" => {
                let edge = eval_args.first().copied().unwrap_or(0.0);
                let x = eval_args.get(1).copied().unwrap_or(0.0);
                if x < edge { 0.0 } else { 1.0 }
            }
            _ => 0.0,
        };
    }

    // Binary operations with precedence (+, -, *, /, %)
    // Look for top-level + or - (outside parens)
    if let Some((left, op, right)) = find_top_level_binop(expr, &['+', '-']) {
        let l_val = evaluate_expression(left, vars);
        let r_val = evaluate_expression(right, vars);
        return if op == '+' { l_val + r_val } else { l_val - r_val };
    }

    // Look for top-level * or / or %
    if let Some((left, op, right)) = find_top_level_binop(expr, &['*', '/', '%']) {
        let l_val = evaluate_expression(left, vars);
        let r_val = evaluate_expression(right, vars);
        return match op {
            '*' => l_val * r_val,
            '/' => if r_val.abs() > 0.00001 { l_val / r_val } else { 0.0 },
            '%' => if r_val.abs() > 0.00001 { l_val % r_val } else { 0.0 },
            _ => 0.0,
        };
    }

    // Strip outer parens
    if expr.starts_with('(') && expr.ends_with(')') {
        if let Some(inner) = strip_matched_parens(expr) {
            return evaluate_expression(inner, vars);
        }
    }

    // Unary minus
    if expr.starts_with('-') {
        return -evaluate_expression(&expr[1..], vars);
    }

    // Literal number
    if let Ok(num) = expr.parse::<f32>() {
        return num;
    }

    // Variable lookup
    if let Some(&val) = vars.get(expr) {
        return val;
    }

    0.0
}

fn parse_function_call(expr: &str) -> Option<(&str, &str)> {
    if let Some(open) = expr.find('(') {
        if expr.ends_with(')') {
            let fn_name = expr[..open].trim();
            if !fn_name.is_empty() && fn_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let args_str = &expr[open + 1..expr.len() - 1];
                return Some((fn_name, args_str));
            }
        }
    }
    None
}

fn split_args(args_str: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in args_str.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(args_str[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < args_str.len() {
        args.push(args_str[start..].trim());
    }
    args
}

fn find_top_level_binop<'a>(expr: &'a str, ops: &[char]) -> Option<(&'a str, char, &'a str)> {
    let mut depth = 0;
    let chars: Vec<(usize, char)> = expr.char_indices().collect();
    // Scan from right to left for left-associativity
    for &(i, c) in chars.iter().rev() {
        match c {
            ')' => depth += 1,
            '(' => depth -= 1,
            op if depth == 0 && ops.contains(&op) => {
                // Ignore unary minus at the very start
                if op == '-' && i == 0 {
                    continue;
                }
                // Ignore operator preceded by another operator
                if i > 0 {
                    let prev_char = expr[..i].trim_end().chars().last();
                    if let Some(pc) = prev_char {
                        if pc == '+' || pc == '-' || pc == '*' || pc == '/' || pc == '%' || pc == '(' {
                            continue;
                        }
                    }
                }
                let left = expr[..i].trim();
                let right = expr[i + 1..].trim();
                return Some((left, op, right));
            }
            _ => {}
        }
    }
    None
}

fn strip_matched_parens(expr: &str) -> Option<&str> {
    let mut depth = 0;
    for (i, c) in expr.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i < expr.len() - 1 {
                    return None; // Paren closed before the end of the string
                }
            }
            _ => {}
        }
    }
    if depth == 0 && expr.len() >= 2 {
        Some(&expr[1..expr.len() - 1])
    } else {
        None
    }
}

// ------------------------------------------------------------------------------------------------
// Texture & Image Processing Engine for Plugins, Viewport, and Objects
// ------------------------------------------------------------------------------------------------

pub fn apply_image_effect_plugin(
    img: &mut macroquad::texture::Image,
    plugin: &EffectPlugin,
    properties: &HashMap<String, Property>,
    time: f32,
) {
    if let Some(ref bt) = plugin.builtin_type {
        match bt.to_lowercase().as_str() {
            "mp4_ultra_compress" | "mp4_corrupt" | "mp4_ultracompress" | "mp4corrupt" => {
                let block_size = properties.get("block_size").map_or(16.0, |p| p.get_value_at(time));
                let compression = properties.get("compression").map_or(75.0, |p| p.get_value_at(time));
                let chroma_loss = properties.get("chroma_loss").map_or(60.0, |p| p.get_value_at(time));
                let corruption = properties.get("corruption").map_or(30.0, |p| p.get_value_at(time));
                let temporal_jitter = properties.get("temporal_jitter").map_or(25.0, |p| p.get_value_at(time));
                let noise_dither = properties.get("noise_dither").map_or(15.0, |p| p.get_value_at(time));
                let mix_amount = properties.get("mix_amount").map_or(100.0, |p| p.get_value_at(time));
                process_image_mp4_compress(
                    img,
                    block_size,
                    compression,
                    chroma_loss,
                    corruption,
                    temporal_jitter,
                    noise_dither,
                    mix_amount,
                    time,
                );
                return;
            }
            "pixelate" | "pixelate_mosaic" => {
                let size = properties.get("pixel_size").map_or(8.0, |p| p.get_value_at(time));
                let blend = properties.get("blend").map_or(100.0, |p| p.get_value_at(time));
                process_image_pixelate(img, size, blend);
                return;
            }
            "rgb_split" | "rgb_channel_shift" => {
                let r_shift = properties.get("red_shift").map_or(15.0, |p| p.get_value_at(time));
                let b_shift = properties.get("blue_shift").map_or(-15.0, |p| p.get_value_at(time));
                let mix_amt = properties.get("mix_amount").map_or(100.0, |p| p.get_value_at(time));
                process_image_rgb_split(img, r_shift, b_shift, mix_amt);
                return;
            }
            "vignette_grain" => {
                let vig = properties.get("vignette_amount").map_or(60.0, |p| p.get_value_at(time));
                let grain = properties.get("grain_intensity").map_or(15.0, |p| p.get_value_at(time));
                let c_boost = properties.get("contrast_boost").map_or(10.0, |p| p.get_value_at(time));
                process_image_vignette_grain(img, vig, grain, c_boost, time);
                return;
            }
            _ => {}
        }
    }

    if !plugin.formula_lines.is_empty() {
        process_image_formula(img, plugin, properties, time);
    }
}

pub fn apply_image_builtin_effect(
    img: &mut macroquad::texture::Image,
    effect: &LayerEffect,
    time: f32,
    plugins: Option<&PluginRegistry>,
) {
    if !effect.enabled {
        return;
    }
    match &effect.effect_type {
        EffectType::Mp4UltraCompress => {
            let block_size = effect.properties.get("block_size").map_or(16.0, |p| p.get_value_at(time));
            let compression = effect.properties.get("compression").map_or(75.0, |p| p.get_value_at(time));
            let chroma_loss = effect.properties.get("chroma_loss").map_or(60.0, |p| p.get_value_at(time));
            let corruption = effect.properties.get("corruption").map_or(30.0, |p| p.get_value_at(time));
            let temporal_jitter = effect.properties.get("temporal_jitter").map_or(25.0, |p| p.get_value_at(time));
            let noise_dither = effect.properties.get("noise_dither").map_or(15.0, |p| p.get_value_at(time));
            let mix_amount = effect.properties.get("mix_amount").map_or(100.0, |p| p.get_value_at(time));
            process_image_mp4_compress(
                img,
                block_size,
                compression,
                chroma_loss,
                corruption,
                temporal_jitter,
                noise_dither,
                mix_amount,
                time,
            );
        }
        EffectType::Plugin(plugin_name) => {
            if let Some(reg) = plugins {
                if let Some(p) = reg.get_effect(plugin_name).or_else(|| reg.get_effect(&effect.name)) {
                    apply_image_effect_plugin(img, p, &effect.properties, time);
                }
            } else {
                let p_path = PathBuf::from(format!("./plugins/{}.bfxplugin", sanitize_id(plugin_name)));
                if let Ok(Some(Plugin::Effect(p))) = parse_plugin_file(&p_path) {
                    apply_image_effect_plugin(img, &p, &effect.properties, time);
                }
            }
        }
        EffectType::BrightnessContrast => {
            let br = effect.properties.get("brightness").map_or(0.0, |p| p.get_value_at(time)) / 100.0;
            let ct = effect.properties.get("contrast").map_or(0.0, |p| p.get_value_at(time)) / 100.0;
            let factor = (1.0 + ct).max(0.0);
            for chunk in img.bytes.chunks_exact_mut(4) {
                let r = (chunk[0] as f32 / 255.0 - 0.5) * factor + 0.5 + br;
                let g = (chunk[1] as f32 / 255.0 - 0.5) * factor + 0.5 + br;
                let b = (chunk[2] as f32 / 255.0 - 0.5) * factor + 0.5 + br;
                chunk[0] = (r.clamp(0.0, 1.0) * 255.0) as u8;
                chunk[1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
                chunk[2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        EffectType::Tint => {
            let amount = effect.properties.get("amount").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
            let blk_r = effect.properties.get("blackR").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
            let blk_g = effect.properties.get("blackG").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
            let blk_b = effect.properties.get("blackB").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
            let wht_r = effect.properties.get("whiteR").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
            let wht_g = effect.properties.get("whiteG").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
            let wht_b = effect.properties.get("whiteB").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
            for chunk in img.bytes.chunks_exact_mut(4) {
                let r = chunk[0] as f32 / 255.0;
                let g = chunk[1] as f32 / 255.0;
                let b = chunk[2] as f32 / 255.0;
                let lum = r * 0.299 + g * 0.587 + b * 0.114;
                let tr = blk_r + lum * (wht_r - blk_r);
                let tg = blk_g + lum * (wht_g - blk_g);
                let tb = blk_b + lum * (wht_b - blk_b);
                chunk[0] = ((r * (1.0 - amount) + tr * amount).clamp(0.0, 1.0) * 255.0) as u8;
                chunk[1] = ((g * (1.0 - amount) + tg * amount).clamp(0.0, 1.0) * 255.0) as u8;
                chunk[2] = ((b * (1.0 - amount) + tb * amount).clamp(0.0, 1.0) * 255.0) as u8;
            }
        }
        EffectType::Invert => {
            let blend = effect.properties.get("blend").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
            for chunk in img.bytes.chunks_exact_mut(4) {
                let r = chunk[0] as f32;
                let g = chunk[1] as f32;
                let b = chunk[2] as f32;
                chunk[0] = (r * (1.0 - blend) + (255.0 - r) * blend).clamp(0.0, 255.0) as u8;
                chunk[1] = (g * (1.0 - blend) + (255.0 - g) * blend).clamp(0.0, 255.0) as u8;
                chunk[2] = (b * (1.0 - blend) + (255.0 - b) * blend).clamp(0.0, 255.0) as u8;
            }
        }
        EffectType::ChromaticAberration => {
            let dist = effect.properties.get("distance").map_or(8.0, |p| p.get_value_at(time));
            let ang = effect.properties.get("angle").map_or(0.0, |p| p.get_value_at(time)).to_radians();
            let intens = effect.properties.get("intensity").map_or(100.0, |p| p.get_value_at(time));
            let r_shift = dist * ang.cos();
            let b_shift = -dist * ang.cos();
            process_image_rgb_split(img, r_shift, b_shift, intens);
        }
        EffectType::Vignette => {
            let amt = effect.properties.get("amount").map_or(50.0, |p| p.get_value_at(time));
            let feather = effect.properties.get("feather").map_or(40.0, |p| p.get_value_at(time));
            process_image_vignette_grain(img, amt, 0.0, feather * 0.2, time);
        }
        EffectType::FastBlur => {
            let radius = effect.properties.get("radius").map_or(10.0, |p| p.get_value_at(time));
            let iters = effect.properties.get("iterations").map_or(2.0, |p| p.get_value_at(time));
            process_image_fast_blur(img, radius, iters as usize);
        }
        EffectType::DirectionalBlur => {
            let length = effect.properties.get("length").map_or(15.0, |p| p.get_value_at(time));
            let direction = effect.properties.get("direction").map_or(0.0, |p| p.get_value_at(time));
            process_image_directional_blur(img, length, direction);
        }
        EffectType::HueSaturation => {
            let hue = effect.properties.get("hue").map_or(0.0, |p| p.get_value_at(time));
            let sat = effect.properties.get("saturation").map_or(0.0, |p| p.get_value_at(time));
            let lightness = effect.properties.get("lightness").map_or(0.0, |p| p.get_value_at(time));
            process_image_hue_saturation(img, hue, sat, lightness);
        }
        EffectType::Fill => {
            let cr = effect.properties.get("colorR").map_or(255.0, |p| p.get_value_at(time)) / 255.0;
            let cg = effect.properties.get("colorG").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
            let cb = effect.properties.get("colorB").map_or(0.0, |p| p.get_value_at(time)) / 255.0;
            let op = effect.properties.get("opacity").map_or(100.0, |p| p.get_value_at(time)) / 100.0;
            process_image_fill(img, cr, cg, cb, op);
        }
        EffectType::Glow => {
            let radius = effect.properties.get("radius").map_or(15.0, |p| p.get_value_at(time));
            let thresh = effect.properties.get("threshold").map_or(50.0, |p| p.get_value_at(time));
            let intens = effect.properties.get("intensity").map_or(100.0, |p| p.get_value_at(time));
            process_image_glow(img, radius, thresh, intens);
        }
        EffectType::WaveWarp => {
            let height = effect.properties.get("height").map_or(10.0, |p| p.get_value_at(time));
            let width = effect.properties.get("width").map_or(40.0, |p| p.get_value_at(time));
            let speed = effect.properties.get("speed").map_or(1.0, |p| p.get_value_at(time));
            process_image_wave_warp(img, height, width, speed, time);
        }
        _ => {}
    }
}

pub fn process_image_mp4_compress(
    img: &mut macroquad::texture::Image,
    block_size: f32,
    compression: f32,
    chroma_loss: f32,
    corruption: f32,
    temporal_jitter: f32,
    noise_dither: f32,
    mix_amount: f32,
    time: f32,
) {
    if mix_amount <= 0.001 {
        return;
    }
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }

    let bs = (block_size.round() as usize).clamp(2, 128);
    let comp_factor = (compression / 100.0).clamp(0.0, 1.0);
    let chroma_factor = (chroma_loss / 100.0).clamp(0.0, 1.0);
    let corrupt_factor = (corruption / 100.0).clamp(0.0, 1.0);
    let jitter = (temporal_jitter / 100.0).clamp(0.0, 1.0);
    let dither_factor = (noise_dither / 100.0).clamp(0.0, 1.0);
    let mix = (mix_amount / 100.0).clamp(0.0, 1.0);

    // Number of quantization levels per color channel
    let quant_levels = (256.0 / (1.0 + comp_factor * 31.0)).max(2.0);

    let orig_bytes = img.bytes.clone();

    let blocks_x = (w + bs - 1) / bs;
    let blocks_y = (h + bs - 1) / bs;

    let time_seed = (time * 12.0).floor();

    for by in 0..blocks_y {
        let y_start = by * bs;
        let y_end = (y_start + bs).min(h);

        let row_hash = ((by as f32 * 37.17 + time_seed * 91.33).sin() * 43758.5453).fract().abs();
        let is_corrupted_row = corrupt_factor > 0.05 && row_hash < (corrupt_factor * 0.4);
        let glitch_dx: isize = if is_corrupted_row {
            let shift_amount = (((row_hash * 1000.0).sin() * corrupt_factor * (bs as f32 * 4.0)) as isize)
                + ((time_seed * 7.0).sin() * jitter * 20.0) as isize;
            shift_amount
        } else {
            0
        };

        for bx in 0..blocks_x {
            let x_start = bx * bs;
            let x_end = (x_start + bs).min(w);

            let mut sum_r = 0u32;
            let mut sum_g = 0u32;
            let mut sum_b = 0u32;
            let mut count = 0u32;

            for py in y_start..y_end {
                for px in x_start..x_end {
                    let idx = (py * w + px) * 4;
                    sum_r += orig_bytes[idx] as u32;
                    sum_g += orig_bytes[idx + 1] as u32;
                    sum_b += orig_bytes[idx + 2] as u32;
                    count += 1;
                }
            }

            if count == 0 {
                continue;
            }

            let avg_r = (sum_r / count) as f32;
            let avg_g = (sum_g / count) as f32;
            let avg_b = (sum_b / count) as f32;

            let q_avg_r = ((avg_r / 255.0 * quant_levels).round() / quant_levels * 255.0).clamp(0.0, 255.0);
            let q_avg_g = ((avg_g / 255.0 * quant_levels).round() / quant_levels * 255.0).clamp(0.0, 255.0);
            let q_avg_b = ((avg_b / 255.0 * quant_levels).round() / quant_levels * 255.0).clamp(0.0, 255.0);

            for py in y_start..y_end {
                for px in x_start..x_end {
                    let src_px = if glitch_dx != 0 {
                        ((px as isize + glitch_dx).rem_euclid(w as isize)) as usize
                    } else {
                        px
                    };
                    let src_idx = (py * w + src_px) * 4;
                    let dst_idx = (py * w + px) * 4;

                    let mut r = orig_bytes[src_idx] as f32;
                    let mut g = orig_bytes[src_idx + 1] as f32;
                    let mut b = orig_bytes[src_idx + 2] as f32;
                    let a = orig_bytes[dst_idx + 3];

                    // Macroblock blend
                    r = r * (1.0 - comp_factor * 0.75) + q_avg_r * (comp_factor * 0.75);
                    g = g * (1.0 - comp_factor * 0.75) + q_avg_g * (comp_factor * 0.75);
                    b = b * (1.0 - comp_factor * 0.75) + q_avg_b * (comp_factor * 0.75);

                    // Quantization of individual pixel
                    r = ((r / 255.0 * quant_levels).round() / quant_levels * 255.0).clamp(0.0, 255.0);
                    g = ((g / 255.0 * quant_levels).round() / quant_levels * 255.0).clamp(0.0, 255.0);
                    b = ((b / 255.0 * quant_levels).round() / quant_levels * 255.0).clamp(0.0, 255.0);

                    // Chroma loss (bleed)
                    let lum = r * 0.299 + g * 0.587 + b * 0.114;
                    let cb = (b - lum) * (1.0 - chroma_factor * 0.5) + (q_avg_b - lum) * (chroma_factor * 0.5);
                    let cr = (r - lum) * (1.0 - chroma_factor * 0.5) + (q_avg_r - lum) * (chroma_factor * 0.5);
                    r = (lum + 1.402 * cr).clamp(0.0, 255.0);
                    g = (lum - 0.344136 * cb - 0.714136 * cr).clamp(0.0, 255.0);
                    b = (lum + 1.772 * cb).clamp(0.0, 255.0);

                    // High frequency dither / noise
                    if dither_factor > 0.0 {
                        let noise = ((px as f32 * 12.9898 + py as f32 * 78.233 + time_seed * 43.1).sin() * 43758.5453).fract() - 0.5;
                        r = (r + noise * dither_factor * 40.0).clamp(0.0, 255.0);
                        g = (g + noise * dither_factor * 40.0).clamp(0.0, 255.0);
                        b = (b + noise * dither_factor * 40.0).clamp(0.0, 255.0);
                    }

                    let orig_r = orig_bytes[dst_idx] as f32;
                    let orig_g = orig_bytes[dst_idx + 1] as f32;
                    let orig_b = orig_bytes[dst_idx + 2] as f32;

                    img.bytes[dst_idx] = (orig_r * (1.0 - mix) + r * mix).clamp(0.0, 255.0) as u8;
                    img.bytes[dst_idx + 1] = (orig_g * (1.0 - mix) + g * mix).clamp(0.0, 255.0) as u8;
                    img.bytes[dst_idx + 2] = (orig_b * (1.0 - mix) + b * mix).clamp(0.0, 255.0) as u8;
                    img.bytes[dst_idx + 3] = a;
                }
            }
        }
    }
}

pub fn process_image_pixelate(img: &mut macroquad::texture::Image, pixel_size: f32, blend: f32) {
    if blend <= 0.001 || pixel_size <= 1.0 {
        return;
    }
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }
    let ps = (pixel_size.round() as usize).clamp(1, 128);
    let mix = (blend / 100.0).clamp(0.0, 1.0);
    let orig = img.bytes.clone();

    for by in (0..h).step_by(ps) {
        let y_max = (by + ps).min(h);
        for bx in (0..w).step_by(ps) {
            let x_max = (bx + ps).min(w);
            let center_x = (bx + x_max) / 2;
            let center_y = (by + y_max) / 2;
            let sample_idx = (center_y * w + center_x) * 4;
            let sr = orig[sample_idx] as f32;
            let sg = orig[sample_idx + 1] as f32;
            let sb = orig[sample_idx + 2] as f32;

            for py in by..y_max {
                for px in bx..x_max {
                    let idx = (py * w + px) * 4;
                    let r = orig[idx] as f32;
                    let g = orig[idx + 1] as f32;
                    let b = orig[idx + 2] as f32;
                    img.bytes[idx] = (r * (1.0 - mix) + sr * mix).clamp(0.0, 255.0) as u8;
                    img.bytes[idx + 1] = (g * (1.0 - mix) + sg * mix).clamp(0.0, 255.0) as u8;
                    img.bytes[idx + 2] = (b * (1.0 - mix) + sb * mix).clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

pub fn process_image_rgb_split(img: &mut macroquad::texture::Image, red_shift: f32, blue_shift: f32, mix_amount: f32) {
    if mix_amount <= 0.001 {
        return;
    }
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }
    let mix = (mix_amount / 100.0).clamp(0.0, 1.0);
    let r_dx = red_shift.round() as isize;
    let b_dx = blue_shift.round() as isize;
    let orig = img.bytes.clone();

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 4;
            let rx = (x as isize + r_dx).clamp(0, (w - 1) as isize) as usize;
            let bx = (x as isize + b_dx).clamp(0, (w - 1) as isize) as usize;
            let r_idx = (y * w + rx) * 4;
            let b_idx = (y * w + bx) * 4;

            let shifted_r = orig[r_idx] as f32;
            let shifted_b = orig[b_idx + 2] as f32;
            let orig_r = orig[idx] as f32;
            let orig_b = orig[idx + 2] as f32;

            img.bytes[idx] = (orig_r * (1.0 - mix) + shifted_r * mix).clamp(0.0, 255.0) as u8;
            img.bytes[idx + 2] = (orig_b * (1.0 - mix) + shifted_b * mix).clamp(0.0, 255.0) as u8;
        }
    }
}

pub fn process_image_vignette_grain(
    img: &mut macroquad::texture::Image,
    vignette_amount: f32,
    grain_intensity: f32,
    contrast_boost: f32,
    time: f32,
) {
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }
    let vig = (vignette_amount / 100.0).clamp(0.0, 1.0);
    let grain = (grain_intensity / 100.0).clamp(0.0, 1.0);
    let factor = (1.0 + contrast_boost / 100.0).max(0.0);
    let center_x = w as f32 / 2.0;
    let center_y = h as f32 / 2.0;
    let max_dist = (center_x * center_x + center_y * center_y).sqrt();

    let time_seed = time * 123.45;

    for y in 0..h {
        let dy = y as f32 - center_y;
        for x in 0..w {
            let dx = x as f32 - center_x;
            let dist_norm = (dx * dx + dy * dy).sqrt() / max_dist.max(1.0);
            let vig_factor = (1.0 - dist_norm * dist_norm * vig).clamp(0.0, 1.0);
            let noise = ((x as f32 * 12.9898 + y as f32 * 78.233 + time_seed).sin() * 43758.5453).fract() - 0.5;
            let grain_val = noise * grain * 40.0;

            let idx = (y * w + x) * 4;
            let mut r = img.bytes[idx] as f32;
            let mut g = img.bytes[idx + 1] as f32;
            let mut b = img.bytes[idx + 2] as f32;

            r = (((r / 255.0 - 0.5) * factor + 0.5) * vig_factor * 255.0 + grain_val).clamp(0.0, 255.0);
            g = (((g / 255.0 - 0.5) * factor + 0.5) * vig_factor * 255.0 + grain_val).clamp(0.0, 255.0);
            b = (((b / 255.0 - 0.5) * factor + 0.5) * vig_factor * 255.0 + grain_val).clamp(0.0, 255.0);

            img.bytes[idx] = r as u8;
            img.bytes[idx + 1] = g as u8;
            img.bytes[idx + 2] = b as u8;
        }
    }
}

pub fn process_image_formula(
    img: &mut macroquad::texture::Image,
    plugin: &EffectPlugin,
    properties: &HashMap<String, Property>,
    time: f32,
) {
    if plugin.formula_lines.is_empty() {
        return;
    }
    let mut base_vars: HashMap<String, f32> = HashMap::new();
    base_vars.insert("time".to_string(), time);
    for slider in &plugin.sliders {
        let val = properties
            .get(&slider.name)
            .map_or(slider.default_value, |p| p.get_value_at(time));
        base_vars.insert(slider.name.clone(), val);
    }

    for chunk in img.bytes.chunks_exact_mut(4) {
        let mut vars = base_vars.clone();
        vars.insert("r".to_string(), chunk[0] as f32 / 255.0);
        vars.insert("g".to_string(), chunk[1] as f32 / 255.0);
        vars.insert("b".to_string(), chunk[2] as f32 / 255.0);
        vars.insert("a".to_string(), chunk[3] as f32 / 255.0);

        for raw_stmt in &plugin.formula_lines {
            let stmt = raw_stmt.trim().trim_end_matches(';');
            if stmt.is_empty() {
                continue;
            }
            if let Some((target_var, expr)) = stmt.split_once('=') {
                let target = target_var.trim().trim_start_matches("let ").trim();
                let val = evaluate_expression(expr.trim(), &vars);
                vars.insert(target.to_string(), val);
            }
        }

        if let Some(&r) = vars.get("r") {
            chunk[0] = (r.clamp(0.0, 1.0) * 255.0) as u8;
        }
        if let Some(&g) = vars.get("g") {
            chunk[1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
        }
        if let Some(&b) = vars.get("b") {
            chunk[2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
        }
        if let Some(&a) = vars.get("a") {
            chunk[3] = (a.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
}

pub fn process_image_fast_blur(img: &mut macroquad::texture::Image, radius: f32, iterations: usize) {
    let r = radius.round() as usize;
    if r == 0 || iterations == 0 {
        return;
    }
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }

    let iters = iterations.clamp(1, 4);
    for _ in 0..iters {
        let src = img.bytes.clone();
        // Horizontal pass
        for y in 0..h {
            for x in 0..w {
                let x_min = x.saturating_sub(r);
                let x_max = (x + r).min(w - 1);
                let count = (x_max - x_min + 1) as u32;
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut sum_a = 0u32;
                for kx in x_min..=x_max {
                    let idx = (y * w + kx) * 4;
                    sum_r += src[idx] as u32;
                    sum_g += src[idx + 1] as u32;
                    sum_b += src[idx + 2] as u32;
                    sum_a += src[idx + 3] as u32;
                }
                let dst_idx = (y * w + x) * 4;
                img.bytes[dst_idx] = (sum_r / count) as u8;
                img.bytes[dst_idx + 1] = (sum_g / count) as u8;
                img.bytes[dst_idx + 2] = (sum_b / count) as u8;
                img.bytes[dst_idx + 3] = (sum_a / count) as u8;
            }
        }
        let src2 = img.bytes.clone();
        // Vertical pass
        for y in 0..h {
            let y_min = y.saturating_sub(r);
            let y_max = (y + r).min(h - 1);
            let count = (y_max - y_min + 1) as u32;
            for x in 0..w {
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let mut sum_a = 0u32;
                for ky in y_min..=y_max {
                    let idx = (ky * w + x) * 4;
                    sum_r += src2[idx] as u32;
                    sum_g += src2[idx + 1] as u32;
                    sum_b += src2[idx + 2] as u32;
                    sum_a += src2[idx + 3] as u32;
                }
                let dst_idx = (y * w + x) * 4;
                img.bytes[dst_idx] = (sum_r / count) as u8;
                img.bytes[dst_idx + 1] = (sum_g / count) as u8;
                img.bytes[dst_idx + 2] = (sum_b / count) as u8;
                img.bytes[dst_idx + 3] = (sum_a / count) as u8;
            }
        }
    }
}

pub fn process_image_directional_blur(img: &mut macroquad::texture::Image, length: f32, direction_deg: f32) {
    if length <= 0.5 {
        return;
    }
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }

    let rad = direction_deg.to_radians();
    let dx = rad.cos();
    let dy = rad.sin();
    let steps = (length.round() as usize).clamp(2, 64);
    let half_steps = steps / 2;
    let src = img.bytes.clone();

    for y in 0..h {
        for x in 0..w {
            let mut sum_r = 0.0f32;
            let mut sum_g = 0.0f32;
            let mut sum_b = 0.0f32;
            let mut sum_a = 0.0f32;
            let mut count = 0.0f32;

            for s in 0..steps {
                let offset = (s as f32) - (half_steps as f32);
                let sx = ((x as f32) + offset * dx).round() as isize;
                let sy = ((y as f32) + offset * dy).round() as isize;
                if sx >= 0 && sx < w as isize && sy >= 0 && sy < h as isize {
                    let idx = (sy as usize * w + sx as usize) * 4;
                    sum_r += src[idx] as f32;
                    sum_g += src[idx + 1] as f32;
                    sum_b += src[idx + 2] as f32;
                    sum_a += src[idx + 3] as f32;
                    count += 1.0;
                }
            }

            if count > 0.0 {
                let dst_idx = (y * w + x) * 4;
                img.bytes[dst_idx] = (sum_r / count).clamp(0.0, 255.0) as u8;
                img.bytes[dst_idx + 1] = (sum_g / count).clamp(0.0, 255.0) as u8;
                img.bytes[dst_idx + 2] = (sum_b / count).clamp(0.0, 255.0) as u8;
                img.bytes[dst_idx + 3] = (sum_a / count).clamp(0.0, 255.0) as u8;
            }
        }
    }
}

pub fn process_image_hue_saturation(img: &mut macroquad::texture::Image, hue: f32, sat: f32, lightness: f32) {
    let hue_shift = hue / 360.0;
    let sat_mul = 1.0 + (sat / 100.0);
    let light_shift = lightness / 100.0;

    for chunk in img.bytes.chunks_exact_mut(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let mut h = 0.0f32;
        let mut s = 0.0f32;
        let mut l = (max + min) / 2.0;

        if (max - min).abs() > 0.0001 {
            let d = max - min;
            s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
            if (max - r).abs() < 0.0001 {
                h = (g - b) / d + (if g < b { 6.0 } else { 0.0 });
            } else if (max - g).abs() < 0.0001 {
                h = (b - r) / d + 2.0;
            } else {
                h = (r - g) / d + 4.0;
            }
            h /= 6.0;
        }

        h = (h + hue_shift).rem_euclid(1.0);
        s = (s * sat_mul).clamp(0.0, 1.0);
        l = (l + light_shift).clamp(0.0, 1.0);

        // Convert back to RGB
        let (nr, ng, nb) = if s <= 0.0001 {
            (l, l, l)
        } else {
            let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
            let p = 2.0 * l - q;
            let hue2rgb = |p: f32, q: f32, mut t: f32| -> f32 {
                if t < 0.0 { t += 1.0; }
                if t > 1.0 { t -= 1.0; }
                if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
                if t < 1.0 / 2.0 { return q; }
                if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
                p
            };
            (
                hue2rgb(p, q, h + 1.0 / 3.0),
                hue2rgb(p, q, h),
                hue2rgb(p, q, h - 1.0 / 3.0),
            )
        };

        chunk[0] = (nr.clamp(0.0, 1.0) * 255.0) as u8;
        chunk[1] = (ng.clamp(0.0, 1.0) * 255.0) as u8;
        chunk[2] = (nb.clamp(0.0, 1.0) * 255.0) as u8;
    }
}

pub fn process_image_fill(img: &mut macroquad::texture::Image, cr: f32, cg: f32, cb: f32, opacity: f32) {
    let op = opacity.clamp(0.0, 1.0);
    for chunk in img.bytes.chunks_exact_mut(4) {
        let r = chunk[0] as f32 / 255.0;
        let g = chunk[1] as f32 / 255.0;
        let b = chunk[2] as f32 / 255.0;
        chunk[0] = ((r * (1.0 - op) + cr * op).clamp(0.0, 1.0) * 255.0) as u8;
        chunk[1] = ((g * (1.0 - op) + cg * op).clamp(0.0, 1.0) * 255.0) as u8;
        chunk[2] = ((b * (1.0 - op) + cb * op).clamp(0.0, 1.0) * 255.0) as u8;
    }
}

pub fn process_image_glow(img: &mut macroquad::texture::Image, radius: f32, threshold: f32, intensity: f32) {
    let thresh = threshold / 100.0;
    let intens = intensity / 100.0;
    if intens <= 0.001 {
        return;
    }
    let mut bloom = img.clone();
    for chunk in bloom.bytes.chunks_exact_mut(4) {
        let lum = (chunk[0] as f32 * 0.299 + chunk[1] as f32 * 0.587 + chunk[2] as f32 * 0.114) / 255.0;
        if lum < thresh {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        }
    }
    process_image_fast_blur(&mut bloom, radius, 2);
    for (dst, src) in img.bytes.chunks_exact_mut(4).zip(bloom.bytes.chunks_exact(4)) {
        let r = (dst[0] as f32) + (src[0] as f32) * intens;
        let g = (dst[1] as f32) + (src[1] as f32) * intens;
        let b = (dst[2] as f32) + (src[2] as f32) * intens;
        dst[0] = r.clamp(0.0, 255.0) as u8;
        dst[1] = g.clamp(0.0, 255.0) as u8;
        dst[2] = b.clamp(0.0, 255.0) as u8;
    }
}

pub fn process_image_wave_warp(img: &mut macroquad::texture::Image, wave_h: f32, wave_w: f32, speed: f32, time: f32) {
    if wave_h <= 0.1 || wave_w <= 0.1 {
        return;
    }
    let w = img.width as usize;
    let h = img.height as usize;
    if w == 0 || h == 0 || img.bytes.len() < w * h * 4 {
        return;
    }
    let src = img.bytes.clone();
    let phase = time * speed * 4.0;
    let freq = std::f32::consts::TAU / wave_w;

    for y in 0..h {
        let dx = (y as f32 * freq + phase).sin() * wave_h;
        for x in 0..w {
            let src_x = ((x as f32 + dx).round() as isize).clamp(0, (w - 1) as isize) as usize;
            let src_idx = (y * w + src_x) * 4;
            let dst_idx = (y * w + x) * 4;
            img.bytes[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
        }
    }
}

// ------------------------------------------------------------------------------------------------
// Functional Plugin Execution
// ------------------------------------------------------------------------------------------------

pub fn execute_functional_plugin(
    comp: &mut Composition,
    plugin: &FunctionalPlugin,
    slider_overrides: Option<&HashMap<String, f32>>,
) -> Result<String, String> {
    let get_val = |name: &str, def: f32| -> f32 {
        if let Some(map) = slider_overrides {
            if let Some(&v) = map.get(name) {
                return v;
            }
        }
        plugin
            .sliders
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.default_value)
            .unwrap_or(def)
    };

    let action = plugin.action.to_lowercase();
    match action.as_str() {
        "add_camera_rig" | "camera_rig" => {
            let dist = get_val("distance", 1800.0);
            let speed = get_val("orbit_speed", 1.0);

            // 1. Create Null controller
            let null_idx = comp.layers.len();
            let mut null_layer = default_layer("Camera Orbit Null".to_string(), LayerSource::Null, null_idx);
            null_layer.d3 = true;
            null_layer.properties.get_mut("x").unwrap().base_value = comp.settings.width as f32 / 2.0;
            null_layer.properties.get_mut("y").unwrap().base_value = comp.settings.height as f32 / 2.0;
            null_layer.properties.get_mut("z").unwrap().base_value = 0.0;

            // Add smooth orbit rotation keyframes
            let rot_prop = null_layer.properties.get_mut("rotationY").unwrap();
            rot_prop.keyframes.push(Keyframe {
                time: 0.0,
                value: -30.0,
                ease: Some(BezierControl::easy_ease()),
            });
            rot_prop.keyframes.push(Keyframe {
                time: (10.0 / speed).max(1.0),
                value: 30.0,
                ease: Some(BezierControl::easy_ease()),
            });
            comp.layers.push(null_layer);

            // 2. Create 3D Camera layer
            let cam_idx = comp.layers.len();
            let mut cam_layer = default_layer("3D Camera 1".to_string(), LayerSource::Camera, cam_idx);
            cam_layer.d3 = true;
            cam_layer.parent_index = Some(null_idx);
            cam_layer.properties.get_mut("z").unwrap().base_value = -dist;
            cam_layer.properties.get_mut("zoom").unwrap().base_value = dist * 0.8;
            cam_layer.properties.get_mut("poiX").unwrap().base_value = comp.settings.width as f32 / 2.0;
            cam_layer.properties.get_mut("poiY").unwrap().base_value = comp.settings.height as f32 / 2.0;
            cam_layer.properties.get_mut("poiZ").unwrap().base_value = 0.0;
            comp.layers.push(cam_layer);

            comp.active_layer_index = Some(null_idx);
            Ok("Created 3D Camera Rig with Orbit Null controller and keyframes.".to_string())
        }
        "stagger_layers" | "stagger" => {
            let offset = get_val("offset_seconds", 0.25).max(0.01);
            let reverse = get_val("reverse_order", 0.0) > 0.5;

            let count = comp.layers.len();
            for i in 0..count {
                let idx = if reverse { count - 1 - i } else { i };
                if let Some(l) = comp.layers.get_mut(idx) {
                    if !l.locked {
                        let duration = (l.out_time - l.in_time).max(0.1);
                        l.in_time = i as f32 * offset;
                        l.out_time = l.in_time + duration;
                    }
                }
            }
            Ok(format!("Staggered {} layers by {:.2}s offset.", count, offset))
        }
        "easy_ease_all" | "easy_ease" => {
            let mut count = 0;
            if let Some(act_idx) = comp.active_layer_index {
                if let Some(l) = comp.layers.get_mut(act_idx) {
                    for prop in l.properties.values_mut() {
                        for kf in &mut prop.keyframes {
                            kf.ease = Some(BezierControl::easy_ease());
                            count += 1;
                        }
                    }
                }
            } else {
                for l in &mut comp.layers {
                    for prop in l.properties.values_mut() {
                        for kf in &mut prop.keyframes {
                            kf.ease = Some(BezierControl::easy_ease());
                            count += 1;
                        }
                    }
                }
            }
            Ok(format!("Applied Easy Ease (Bezier) to {} keyframes.", count))
        }
        "add_adjustment_layer" => {
            let idx = comp.layers.len();
            let mut adj = default_layer("Adjustment Layer FX".to_string(), LayerSource::Adjustment, idx);
            adj.fx = true;
            adj.effects.push(LayerEffect::new("Brightness & Contrast".to_string(), EffectType::BrightnessContrast));
            adj.effects.push(LayerEffect::new("Glow".to_string(), EffectType::Glow));
            adj.effects.push(LayerEffect::new("Vignette".to_string(), EffectType::Vignette));
            comp.layers.push(adj);
            comp.active_layer_index = Some(idx);
            Ok("Added Master Adjustment Layer with FX stack.".to_string())
        }
        "create_palette_solids" => {
            let colors: [[f32; 4]; 5] = [
                [0.12, 0.14, 0.20, 1.0], // Deep Navy
                [0.92, 0.35, 0.30, 1.0], // Coral Red
                [0.98, 0.75, 0.25, 1.0], // Gold Sun
                [0.25, 0.70, 0.65, 1.0], // Teal Aqua
                [0.95, 0.95, 0.96, 1.0], // Soft White
            ];
            let names = ["Palette Solid Navy", "Palette Solid Coral", "Palette Solid Gold", "Palette Solid Teal", "Palette Solid White"];
            let w = comp.settings.width as f32 / 5.0;
            let h = comp.settings.height as f32;

            for (i, (&col, name)) in colors.iter().zip(names.iter()).enumerate() {
                let idx = comp.layers.len();
                let mut layer = default_layer(name.to_string(), LayerSource::Solid { color: col }, idx);
                layer.properties.get_mut("x").unwrap().base_value = (i as f32 + 0.5) * w;
                layer.properties.get_mut("y").unwrap().base_value = h / 2.0;
                layer.properties.get_mut("scaleX").unwrap().base_value = (w / 200.0) * 100.0;
                layer.properties.get_mut("scaleY").unwrap().base_value = (h / 200.0) * 100.0;
                comp.layers.push(layer);
            }
            Ok("Generated 5-color palette solids across composition.".to_string())
        }
        _ => {
            // Run script commands if any
            if !plugin.script_commands.is_empty() {
                execute_script_commands(comp, &plugin.script_commands)?;
                Ok(format!("Executed script plugin '{}' successfully.", plugin.name))
            } else {
                Err(format!("Unknown functional action: {}", plugin.action))
            }
        }
    }
}

fn execute_script_commands(comp: &mut Composition, commands: &[String]) -> Result<(), String> {
    for cmd_line in commands {
        let parts: Vec<&str> = cmd_line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0].to_lowercase().as_str() {
            "add_layer" => {
                let layer_type = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_default();
                let idx = comp.layers.len();
                let name = parts.get(2).map(|s| s.trim_matches('"')).unwrap_or("New Layer");
                match layer_type.as_str() {
                    "solid" => {
                        let layer = default_layer(name.to_string(), LayerSource::Solid { color: [0.2, 0.4, 0.8, 1.0] }, idx);
                        comp.layers.push(layer);
                    }
                    "text" => {
                        let text_val = parts.get(3).map(|s| s.trim_matches('"')).unwrap_or("Sample Text");
                        let layer = default_layer(name.to_string(), LayerSource::Text {
                            text: text_val.to_string(),
                            font_size: 64.0,
                            color: [1.0, 1.0, 1.0, 1.0],
                        }, idx);
                        comp.layers.push(layer);
                    }
                    "adjustment" => {
                        let layer = default_layer(name.to_string(), LayerSource::Adjustment, idx);
                        comp.layers.push(layer);
                    }
                    "camera" => {
                        let mut layer = default_layer(name.to_string(), LayerSource::Camera, idx);
                        layer.d3 = true;
                        comp.layers.push(layer);
                    }
                    "null" => {
                        let layer = default_layer(name.to_string(), LayerSource::Null, idx);
                        comp.layers.push(layer);
                    }
                    _ => {}
                }
            }
            "add_effect" => {
                let eff_name = parts.get(1).map(|s| s.trim_matches('"')).unwrap_or("Fast Blur");
                if let Some(act_idx) = comp.active_layer_index {
                    if let Some(l) = comp.layers.get_mut(act_idx) {
                        l.fx = true;
                        let et = match eff_name.to_lowercase().as_str() {
                            "fast blur" | "blur" => EffectType::FastBlur,
                            "brightness & contrast" | "brightness" => EffectType::BrightnessContrast,
                            "glow" => EffectType::Glow,
                            "vignette" => EffectType::Vignette,
                            "tint" => EffectType::Tint,
                            "invert" => EffectType::Invert,
                            "fill" => EffectType::Fill,
                            _ => EffectType::Plugin(eff_name.to_string()),
                        };
                        l.effects.push(LayerEffect::new(eff_name.to_string(), et));
                    }
                }
            }
            "set_property" => {
                let prop_name = parts.get(1).map(|s| s.trim_matches('"')).unwrap_or("");
                let val: f32 = parts.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
                if let Some(act_idx) = comp.active_layer_index {
                    if let Some(l) = comp.layers.get_mut(act_idx) {
                        if let Some(p) = l.properties.get_mut(prop_name) {
                            p.base_value = val;
                        }
                    }
                }
            }
            "stagger" => {
                let offset: f32 = parts.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.25);
                for (i, l) in comp.layers.iter_mut().enumerate() {
                    let dur = (l.out_time - l.in_time).max(0.1);
                    l.in_time = i as f32 * offset;
                    l.out_time = l.in_time + dur;
                }
            }
            "ease_keyframes" => {
                for l in &mut comp.layers {
                    for p in l.properties.values_mut() {
                        for kf in &mut p.keyframes {
                            kf.ease = Some(BezierControl::easy_ease());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_effect_plugin() {
        let content = r#".spec effect
name: Custom Glow Booster
category: Stylize
description: Boosts brightness and glow
slider: intensity, 50.0, 0.0, 100.0, 1.0
slider: boost, 2.0, 1.0, 10.0, 0.1

// Math formula
r = r * boost;
g = g * boost;
b = b * boost;
"#;
        let plugin = parse_plugin_content(content, "test.bfxplugin").unwrap();
        assert!(plugin.is_some());
        if let Some(Plugin::Effect(eff)) = plugin {
            assert_eq!(eff.name, "Custom Glow Booster");
            assert_eq!(eff.category, "Stylize");
            assert_eq!(eff.sliders.len(), 2);
            assert_eq!(eff.sliders[0].name, "intensity");
            assert_eq!(eff.sliders[0].default_value, 50.0);
            assert_eq!(eff.formula_lines.len(), 3);
        } else {
            panic!("Expected Effect plugin");
        }
    }

    #[test]
    fn test_parse_functional_plugin() {
        let content = r#".spec functional
name: Setup Camera Rig
category: 3D Tools
description: Creates a 3D orbit camera
action: add_camera_rig
slider: distance, 2000.0, 500.0, 5000.0, 100.0
"#;
        let plugin = parse_plugin_content(content, "camera.bfxplugin").unwrap();
        assert!(plugin.is_some());
        if let Some(Plugin::Functional(func)) = plugin {
            assert_eq!(func.name, "Setup Camera Rig");
            assert_eq!(func.action, "add_camera_rig");
            assert_eq!(func.sliders.len(), 1);
            assert_eq!(func.sliders[0].name, "distance");
        } else {
            panic!("Expected Functional plugin");
        }
    }

    #[test]
    fn test_apply_effect_plugin_formula() {
        let eff = EffectPlugin {
            id: "invert_red".to_string(),
            name: "Invert Red".to_string(),
            category: "Color".to_string(),
            description: "Inverts red channel".to_string(),
            file_path: "invert_red.bfxplugin".to_string(),
            sliders: vec![PluginSlider {
                name: "mix_amt".to_string(),
                display_name: "Mix".to_string(),
                default_value: 1.0,
                min_value: 0.0,
                max_value: 1.0,
                step: 0.1,
            }],
            formula_lines: vec![
                "r = mix(r, 1.0 - r, mix_amt);".to_string(),
            ],
            builtin_type: None,
            glsl_shader: None,
        };

        let mut props = HashMap::new();
        props.insert("mix_amt".to_string(), Property {
            name: "mix_amt".to_string(),
            base_value: 1.0,
            keyframes: vec![],
            wiggle: None,
        });

        let orig = Color::new(0.8, 0.2, 0.4, 1.0);
        let res = apply_effect_plugin(orig, &eff, &props, 0.0);
        assert!((res.r - 0.2).abs() < 0.001);
        assert_eq!(res.g, 0.2);
        assert_eq!(res.b, 0.4);
    }

    #[test]
    fn test_execute_camera_rig_functional_plugin() {
        let mut comp = Composition::default();
        let func = FunctionalPlugin {
            id: "camera_rig".to_string(),
            name: "Camera Rig".to_string(),
            category: "3D".to_string(),
            description: "Add rig".to_string(),
            file_path: "camera_rig.bfxplugin".to_string(),
            action: "add_camera_rig".to_string(),
            script_commands: vec![],
            sliders: vec![],
        };

        let res = execute_functional_plugin(&mut comp, &func, None);
        assert!(res.is_ok());
        assert_eq!(comp.layers.len(), 2);
        assert!(comp.layers[0].d3);
        assert!(comp.layers[1].d3);
        assert_eq!(comp.layers[1].parent_index, Some(0));
    }

    #[test]
    fn test_execute_stagger_functional_plugin() {
        let mut comp = Composition::default();
        comp.layers.push(default_layer("Layer 1".to_string(), LayerSource::Adjustment, 0));
        comp.layers.push(default_layer("Layer 2".to_string(), LayerSource::Adjustment, 1));
        comp.layers.push(default_layer("Layer 3".to_string(), LayerSource::Adjustment, 2));

        let func = FunctionalPlugin {
            id: "stagger".to_string(),
            name: "Stagger".to_string(),
            category: "Animation".to_string(),
            description: "Stagger layers".to_string(),
            file_path: "stagger.bfxplugin".to_string(),
            action: "stagger_layers".to_string(),
            script_commands: vec![],
            sliders: vec![PluginSlider {
                name: "offset_seconds".to_string(),
                display_name: "Offset".to_string(),
                default_value: 0.5,
                min_value: 0.0,
                max_value: 5.0,
                step: 0.1,
            }],
        };

        let res = execute_functional_plugin(&mut comp, &func, None);
        assert!(res.is_ok());
        assert_eq!(comp.layers[0].in_time, 0.0);
        assert_eq!(comp.layers[1].in_time, 0.5);
        assert_eq!(comp.layers[2].in_time, 1.0);
    }

    #[test]
    fn test_math_evaluator_expressions() {
        let mut vars = HashMap::new();
        vars.insert("x".to_string(), 10.0);
        vars.insert("y".to_string(), 5.0);

        assert_eq!(evaluate_expression("x + y", &vars), 15.0);
        assert_eq!(evaluate_expression("x - y", &vars), 5.0);
        assert_eq!(evaluate_expression("x * y", &vars), 50.0);
        assert_eq!(evaluate_expression("x / y", &vars), 2.0);
        assert_eq!(evaluate_expression("x % y", &vars), 0.0);
        assert_eq!(evaluate_expression("(x + y) * 2", &vars), 30.0);
        assert_eq!(evaluate_expression("min(x, y)", &vars), 5.0);
        assert_eq!(evaluate_expression("max(x, y)", &vars), 10.0);
        assert_eq!(evaluate_expression("clamp(15.0, 0.0, 10.0)", &vars), 10.0);
        assert_eq!(evaluate_expression("mix(0.0, 100.0, 0.5)", &vars), 50.0);
        assert_eq!(evaluate_expression("pow(2.0, 3.0)", &vars), 8.0);
        assert_eq!(evaluate_expression("abs(-42.0)", &vars), 42.0);
    }

    #[test]
    fn test_plugin_registry_default_creation() {
        let temp_dir = std::env::temp_dir().join(format!("bfx_plug_test_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut reg = PluginRegistry {
            effects: Vec::new(),
            functionals: Vec::new(),
            load_errors: Vec::new(),
            plugins_dir: temp_dir.clone(),
        };
        reg.ensure_default_plugins();
        reg.reload();

        assert!(!reg.effects.is_empty(), "Should load default effect plugins");
        assert!(!reg.functionals.is_empty(), "Should load default functional plugins");
        assert!(reg.get_effect("Sepia Tone").is_some());
        assert!(reg.get_functional("Create 3D Camera Rig").is_some());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
