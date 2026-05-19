use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct BezierControl {
    pub cp1: f32,
    pub cp2: f32,
}

#[derive(Clone)]
pub struct SelectedKeyframe {
    pub layer_index: usize,
    pub property_name: String,
    pub keyframe_index: usize,
    pub handle: Option<CurveHandle>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CurveHandle {
    Out,
    In,
}

#[derive(Serialize, Deserialize)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub ease: Option<BezierControl>,
}

#[derive(Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub base_value: f32,
    pub keyframes: Vec<Keyframe>,
}

impl Property {
    pub fn get_value_at(&self, time: f32) -> f32 {
        if self.keyframes.is_empty() {
            return self.base_value;
        }
        let mut frames = self.keyframes.iter().peekable();
        while let Some(curr) = frames.next() {
            if let Some(next) = frames.peek() {
                if time >= curr.time && time <= next.time {
                    let t = (time - curr.time) / (next.time - curr.time);
                    return match curr.ease {
                        Some(e) => self.interpolate(curr.value, next.value, t, e),
                        None => curr.value + t * (next.value - curr.value),
                    };
                }
            } else if time >= curr.time {
                return curr.value;
            }
        }
        self.keyframes[0].value
    }

    fn interpolate(&self, s: f32, e: f32, t: f32, b: BezierControl) -> f32 {
        let it = 1.0 - t;
        let p1 = s + (e - s) * b.cp1;
        let p2 = s + (e - s) * b.cp2;
        it.powi(3) * s + 3.0 * it.powi(2) * t * p1 + 3.0 * it * t.powi(2) * p2 + t.powi(3) * e
    }
}

#[derive(Serialize, Deserialize)]
pub enum LayerSource {
    Solid {
        color: [f32; 4],
    },
    Image {
        path: String,
    },
    Audio {
        path: String,
    },
    Video {
        path: String,
    },
    Object3D {
        path: Option<String>,
        color: [f32; 4],
    },
    Polygon {
        points: Vec<[f32; 2]>,
        color: [f32; 4],
    },
}

#[derive(Serialize, Deserialize)]
pub struct Layer {
    pub name: String,
    pub source: LayerSource,
    pub properties: HashMap<String, Property>,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub locked: bool,
    #[serde(default)]
    pub solo: bool,
    #[serde(default)]
    pub fx: bool,
    #[serde(default)]
    pub d3: bool,
    #[serde(default)]
    pub ff: bool,
    #[serde(default)]
    pub moblur: bool,
    #[serde(default)]
    pub shy: bool,
    #[serde(default)]
    pub collapse: bool,
    #[serde(default)]
    pub collapsed: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize)]
pub struct Resource {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub kind: ResourceKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Image,
    Audio,
    Video,
    Model3D,
}

impl Default for ResourceKind {
    fn default() -> Self {
        ResourceKind::Image
    }
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub property_colors: HashMap<String, [u8; 3]>,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
    #[serde(default = "default_width")]
    pub width: u32,
    #[serde(default = "default_height")]
    pub height: u32,
    #[serde(default = "default_duration")]
    pub duration: f32,
    #[serde(default = "default_fps")]
    pub fps: u32,
}

fn default_ui_scale() -> f32 {
    1.0
}

fn default_width() -> u32 {
    1920
}

fn default_height() -> u32 {
    1080
}

fn default_duration() -> f32 {
    30.0
}

fn default_fps() -> u32 {
    60
}

impl Default for Settings {
    fn default() -> Self {
        let mut property_colors = HashMap::new();
        property_colors.insert("anchorX".to_string(), [200, 100, 100]);
        property_colors.insert("anchorY".to_string(), [200, 100, 100]);
        property_colors.insert("x".to_string(), [100, 200, 100]);
        property_colors.insert("y".to_string(), [100, 200, 100]);
        property_colors.insert("rotation".to_string(), [100, 100, 200]);
        property_colors.insert("scaleX".to_string(), [200, 200, 100]);
        property_colors.insert("scaleY".to_string(), [200, 200, 100]);
        property_colors.insert("z".to_string(), [100, 180, 230]);
        property_colors.insert("rotationX".to_string(), [120, 140, 220]);
        property_colors.insert("rotationY".to_string(), [120, 140, 220]);
        property_colors.insert("opacity".to_string(), [200, 100, 200]);
        Settings {
            property_colors,
            ui_scale: 1.0,
            width: default_width(),
            height: default_height(),
            duration: default_duration(),
            fps: default_fps(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Composition {
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub resources: Vec<Resource>,
    #[serde(default)]
    pub current_time: f32,
    #[serde(default)]
    pub is_playing: bool,
    #[serde(default)]
    pub show_curves: bool,
    #[serde(default)]
    pub timeline_scroll_v: f32,
    #[serde(default)]
    pub timeline_scroll_h: f32,
    #[serde(default)]
    pub settings: Settings,
}
