use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub struct BezierControl {
    pub cp1: f32,
    pub cp2: f32,
}

impl BezierControl {
    pub fn easy_ease() -> Self {
        Self { cp1: 0.333, cp2: 0.667 }
    }
    pub fn ease_in() -> Self {
        Self { cp1: 0.667, cp2: 1.0 }
    }
    pub fn ease_out() -> Self {
        Self { cp1: 0.0, cp2: 0.333 }
    }
    pub fn linear() -> Self {
        Self { cp1: 0.0, cp2: 1.0 }
    }
    pub fn back_out() -> Self {
        Self { cp1: 0.175, cp2: 0.885 }
    }
    pub fn exponential() -> Self {
        Self { cp1: 0.95, cp2: 0.05 }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GraphMode {
    ValueGraph,
    SpeedGraph,
}

impl Default for GraphMode {
    fn default() -> Self {
        GraphMode::ValueGraph
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedKeyframe {
    pub layer_index: usize,
    pub property_name: String,
    pub keyframe_index: usize,
    pub handle: Option<CurveHandle>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CurveHandle {
    Out,
    In,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct WiggleSettings {
    pub enabled: bool,
    pub freq: f32,
    pub amp: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Marker {
    pub time: f32,
    pub label: String,
    pub comment: String,
    #[serde(default)]
    pub color_index: usize,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Hash)]
pub enum EffectType {
    Tint,
    BrightnessContrast,
    FastBlur,
    Glow,
    DropShadow,
    Invert,
    Vignette,
    HueSaturation,
    Fill,
    ChromaticAberration,
    DirectionalBlur,
    WaveWarp,
    #[serde(rename = "Plugin")]
    Plugin(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LayerEffect {
    pub id: String,
    pub name: String,
    pub effect_type: EffectType,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub properties: HashMap<String, Property>,
}

impl LayerEffect {
    pub fn new(name: String, effect_type: EffectType) -> Self {
        let mut properties = HashMap::new();
        match effect_type {
            EffectType::Tint => {
                properties.insert("amount".to_string(), Property { name: "amount".to_string(), base_value: 100.0, keyframes: vec![], wiggle: None });
                properties.insert("blackR".to_string(), Property { name: "blackR".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("blackG".to_string(), Property { name: "blackG".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("blackB".to_string(), Property { name: "blackB".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("whiteR".to_string(), Property { name: "whiteR".to_string(), base_value: 255.0, keyframes: vec![], wiggle: None });
                properties.insert("whiteG".to_string(), Property { name: "whiteG".to_string(), base_value: 255.0, keyframes: vec![], wiggle: None });
                properties.insert("whiteB".to_string(), Property { name: "whiteB".to_string(), base_value: 255.0, keyframes: vec![], wiggle: None });
            }
            EffectType::BrightnessContrast => {
                properties.insert("brightness".to_string(), Property { name: "brightness".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("contrast".to_string(), Property { name: "contrast".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
            }
            EffectType::FastBlur => {
                properties.insert("blurRadius".to_string(), Property { name: "blurRadius".to_string(), base_value: 15.0, keyframes: vec![], wiggle: None });
            }
            EffectType::Glow => {
                properties.insert("threshold".to_string(), Property { name: "threshold".to_string(), base_value: 50.0, keyframes: vec![], wiggle: None });
                properties.insert("radius".to_string(), Property { name: "radius".to_string(), base_value: 20.0, keyframes: vec![], wiggle: None });
                properties.insert("intensity".to_string(), Property { name: "intensity".to_string(), base_value: 1.0, keyframes: vec![], wiggle: None });
            }
            EffectType::DropShadow => {
                properties.insert("distance".to_string(), Property { name: "distance".to_string(), base_value: 10.0, keyframes: vec![], wiggle: None });
                properties.insert("angle".to_string(), Property { name: "angle".to_string(), base_value: 45.0, keyframes: vec![], wiggle: None });
                properties.insert("opacity".to_string(), Property { name: "opacity".to_string(), base_value: 75.0, keyframes: vec![], wiggle: None });
                properties.insert("softness".to_string(), Property { name: "softness".to_string(), base_value: 5.0, keyframes: vec![], wiggle: None });
            }
            EffectType::Invert => {
                properties.insert("blend".to_string(), Property { name: "blend".to_string(), base_value: 100.0, keyframes: vec![], wiggle: None });
            }
            EffectType::Vignette => {
                properties.insert("amount".to_string(), Property { name: "amount".to_string(), base_value: 50.0, keyframes: vec![], wiggle: None });
                properties.insert("feather".to_string(), Property { name: "feather".to_string(), base_value: 40.0, keyframes: vec![], wiggle: None });
            }
            EffectType::HueSaturation => {
                properties.insert("hue".to_string(), Property { name: "hue".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("saturation".to_string(), Property { name: "saturation".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("lightness".to_string(), Property { name: "lightness".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
            }
            EffectType::Fill => {
                properties.insert("colorR".to_string(), Property { name: "colorR".to_string(), base_value: 255.0, keyframes: vec![], wiggle: None });
                properties.insert("colorG".to_string(), Property { name: "colorG".to_string(), base_value: 100.0, keyframes: vec![], wiggle: None });
                properties.insert("colorB".to_string(), Property { name: "colorB".to_string(), base_value: 50.0, keyframes: vec![], wiggle: None });
                properties.insert("opacity".to_string(), Property { name: "opacity".to_string(), base_value: 100.0, keyframes: vec![], wiggle: None });
            }
            EffectType::ChromaticAberration => {
                properties.insert("distance".to_string(), Property { name: "distance".to_string(), base_value: 8.0, keyframes: vec![], wiggle: None });
                properties.insert("angle".to_string(), Property { name: "angle".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
                properties.insert("intensity".to_string(), Property { name: "intensity".to_string(), base_value: 100.0, keyframes: vec![], wiggle: None });
            }
            EffectType::DirectionalBlur => {
                properties.insert("blurLength".to_string(), Property { name: "blurLength".to_string(), base_value: 20.0, keyframes: vec![], wiggle: None });
                properties.insert("angle".to_string(), Property { name: "angle".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
            }
            EffectType::WaveWarp => {
                properties.insert("waveHeight".to_string(), Property { name: "waveHeight".to_string(), base_value: 15.0, keyframes: vec![], wiggle: None });
                properties.insert("waveWidth".to_string(), Property { name: "waveWidth".to_string(), base_value: 40.0, keyframes: vec![], wiggle: None });
                properties.insert("speed".to_string(), Property { name: "speed".to_string(), base_value: 1.0, keyframes: vec![], wiggle: None });
                properties.insert("direction".to_string(), Property { name: "direction".to_string(), base_value: 0.0, keyframes: vec![], wiggle: None });
            }
            EffectType::Plugin(_) => {}
        }
        LayerEffect {
            id: format!("{:x}", (time_now_seed() ^ rand_pseudo())),
            name,
            effect_type,
            enabled: true,
            properties,
        }
    }

    pub fn new_plugin(plugin: &crate::plugin::EffectPlugin) -> Self {
        let mut properties = HashMap::new();
        for slider in &plugin.sliders {
            properties.insert(
                slider.name.clone(),
                Property {
                    name: slider.name.clone(),
                    base_value: slider.default_value,
                    keyframes: vec![],
                    wiggle: None,
                },
            );
        }
        LayerEffect {
            id: format!("{:x}", (time_now_seed() ^ rand_pseudo())),
            name: plugin.name.clone(),
            effect_type: EffectType::Plugin(plugin.name.clone()),
            enabled: true,
            properties,
        }
    }
}

fn time_now_seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(123456789)
}

static mut PSEUDO_RAND: u64 = 987654321;
fn rand_pseudo() -> u64 {
    unsafe {
        PSEUDO_RAND = PSEUDO_RAND.wrapping_mul(6364136223846793005).wrapping_add(1);
        PSEUDO_RAND
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub ease: Option<BezierControl>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Property {
    pub name: String,
    pub base_value: f32,
    pub keyframes: Vec<Keyframe>,
    #[serde(default)]
    pub wiggle: Option<WiggleSettings>,
}

impl Property {
    pub fn get_value_at(&self, time: f32) -> f32 {
        let base = if self.keyframes.is_empty() {
            self.base_value
        } else {
            let mut frames = self.keyframes.iter().peekable();
            let mut val = self.keyframes[0].value;
            while let Some(curr) = frames.next() {
                if let Some(next) = frames.peek() {
                    if time >= curr.time && time <= next.time {
                        let t = (time - curr.time) / (next.time - curr.time);
                        val = match curr.ease {
                            Some(e) => self.interpolate(curr.value, next.value, t, e),
                            None => curr.value + t * (next.value - curr.value),
                        };
                        break;
                    }
                } else if time >= curr.time {
                    val = curr.value;
                    break;
                }
            }
            val
        };

        if let Some(w) = &self.wiggle {
            if w.enabled && w.freq > 0.0 && w.amp > 0.0 {
                let s1 = (time * w.freq * 6.2831853).sin();
                let s2 = (time * w.freq * 1.618034 * 6.2831853 + 1.25).sin();
                let noise = (s1 * 0.7 + s2 * 0.3) * w.amp;
                return base + noise;
            }
        }
        base
    }

    fn interpolate(&self, s: f32, e: f32, t: f32, b: BezierControl) -> f32 {
        let it = 1.0 - t;
        let p1 = s + (e - s) * b.cp1;
        let p2 = s + (e - s) * b.cp2;
        it.powi(3) * s + 3.0 * it.powi(2) * t * p1 + 3.0 * it * t.powi(2) * p2 + t.powi(3) * e
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
    Text {
        text: String,
        font_size: f32,
        color: [f32; 4],
    },
    Adjustment,
    Null,
    Camera,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
    #[serde(default = "default_true")]
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
    #[serde(default)]
    pub in_time: f32,
    #[serde(default = "default_out_time")]
    pub out_time: f32,
    #[serde(default)]
    pub label_color_index: usize,
    #[serde(default = "default_blend_mode")]
    pub blend_mode: String,
    #[serde(default)]
    pub parent_index: Option<usize>,
    #[serde(default = "default_track_matte")]
    pub track_matte: String,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub effects: Vec<LayerEffect>,
}

fn default_out_time() -> f32 {
    30.0
}

fn default_blend_mode() -> String {
    "Normal".to_string()
}

fn default_track_matte() -> String {
    "None".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Resource {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub kind: ResourceKind,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
        property_colors.insert("anchorZ".to_string(), [200, 100, 100]);
        property_colors.insert("x".to_string(), [100, 200, 100]);
        property_colors.insert("y".to_string(), [100, 200, 100]);
        property_colors.insert("z".to_string(), [100, 180, 230]);
        property_colors.insert("rotation".to_string(), [100, 100, 200]);
        property_colors.insert("rotationX".to_string(), [120, 140, 220]);
        property_colors.insert("rotationY".to_string(), [120, 140, 220]);
        property_colors.insert("scaleX".to_string(), [200, 200, 100]);
        property_colors.insert("scaleY".to_string(), [200, 200, 100]);
        property_colors.insert("scaleZ".to_string(), [200, 200, 100]);
        property_colors.insert("opacity".to_string(), [200, 100, 200]);
        property_colors.insert("poiX".to_string(), [100, 220, 180]);
        property_colors.insert("poiY".to_string(), [100, 220, 180]);
        property_colors.insert("poiZ".to_string(), [100, 220, 180]);
        property_colors.insert("zoom".to_string(), [230, 160, 70]);
        property_colors.insert("fov".to_string(), [230, 160, 70]);
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewportMode {
    ActiveCamera,
    CustomView,
    Top,
    Front,
    Right,
    Left,
    Bottom,
    Back,
}

impl Default for ViewportMode {
    fn default() -> Self {
        ViewportMode::ActiveCamera
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheCompressionMode {
    FastPlanarRle,
    UltraFastDirect,
    Uncompressed,
}

impl Default for CacheCompressionMode {
    fn default() -> Self {
        CacheCompressionMode::FastPlanarRle
    }
}

fn default_cache_max_frames() -> usize {
    2000
}

fn default_cache_max_memory_mb() -> f32 {
    2048.0
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
    #[serde(default)]
    pub active_layer_index: Option<usize>,
    #[serde(default)]
    pub work_area_in: f32,
    #[serde(default = "default_duration")]
    pub work_area_out: f32,
    #[serde(default = "default_zoom")]
    pub timeline_zoom: f32,
    #[serde(default)]
    pub hide_shy: bool,
    #[serde(default)]
    pub switches_mode: bool,
    #[serde(default)]
    pub active_tool: usize,
    #[serde(default)]
    pub right_panel_tab: usize,
    #[serde(default)]
    pub left_panel_tab: usize,
    #[serde(default = "default_true")]
    pub show_guides: bool,
    #[serde(default)]
    pub show_grid: bool,
    #[serde(default)]
    pub show_rulers: bool,
    #[serde(default)]
    pub show_checkerboard: bool,
    #[serde(default = "default_comp_zoom")]
    pub comp_zoom: f32,
    #[serde(default)]
    pub search_query: String,
    #[serde(default)]
    pub layer_search_query: String,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub show_graph_editor: bool,
    #[serde(default)]
    pub graph_mode: GraphMode,
    #[serde(default = "default_true")]
    pub snapping: bool,
    #[serde(default)]
    pub solo_animated_properties: bool,
    #[serde(default)]
    pub viewport_mode: ViewportMode,
    #[serde(default = "default_custom_yaw")]
    pub custom_orbit_yaw: f32,
    #[serde(default = "default_custom_pitch")]
    pub custom_orbit_pitch: f32,
    #[serde(default = "default_custom_distance")]
    pub custom_orbit_distance: f32,
    #[serde(default = "default_custom_target")]
    pub custom_orbit_target: [f32; 3],
    #[serde(default)]
    pub custom_orbit_roll: f32,
    #[serde(default = "default_true")]
    pub cache_compression_enabled: bool,
    #[serde(default)]
    pub cache_compression_mode: CacheCompressionMode,
    #[serde(default = "default_cache_max_frames")]
    pub cache_max_frames: usize,
    #[serde(default = "default_cache_max_memory_mb")]
    pub cache_max_memory_mb: f32,
    #[serde(skip)]
    pub cache_raw_size_mb: f32,
    #[serde(skip)]
    pub cache_compression_ratio: f32,
    #[serde(default)]
    pub is_ram_previewing: bool,
    #[serde(default = "default_true")]
    pub ram_cache_enabled: bool,
    #[serde(skip)]
    pub cached_frames: HashSet<usize>,
    #[serde(skip)]
    pub playback_fps: f32,
    #[serde(skip)]
    pub cache_size_mb: f32,
    #[serde(default)]
    pub ram_cache_purge_requested: bool,
    #[serde(default)]
    pub auto_frame_cache: bool,
    #[serde(skip)]
    pub auto_cache_in_progress: bool,
    #[serde(default)]
    pub pause_at_last_keyframe: bool,
}

fn default_zoom() -> f32 {
    100.0
}

fn default_comp_zoom() -> f32 {
    1.0
}

fn default_custom_yaw() -> f32 {
    -35.0
}

fn default_custom_pitch() -> f32 {
    25.0
}

fn default_custom_distance() -> f32 {
    2200.0
}

fn default_custom_target() -> [f32; 3] {
    [960.0, 540.0, 0.0]
}

impl Default for Composition {
    fn default() -> Self {
        Composition {
            layers: vec![],
            resources: vec![],
            current_time: 0.0,
            is_playing: false,
            show_curves: false,
            timeline_scroll_v: 0.0,
            timeline_scroll_h: 0.0,
            settings: Settings::default(),
            active_layer_index: None,
            work_area_in: 0.0,
            work_area_out: default_duration(),
            timeline_zoom: default_zoom(),
            hide_shy: false,
            switches_mode: false,
            active_tool: 0,
            right_panel_tab: 0,
            left_panel_tab: 0,
            show_guides: true,
            show_grid: false,
            show_rulers: false,
            show_checkerboard: false,
            comp_zoom: default_comp_zoom(),
            search_query: String::new(),
            layer_search_query: String::new(),
            markers: vec![],
            show_graph_editor: false,
            graph_mode: GraphMode::default(),
            snapping: true,
            solo_animated_properties: false,
            viewport_mode: ViewportMode::default(),
            custom_orbit_yaw: default_custom_yaw(),
            custom_orbit_pitch: default_custom_pitch(),
            custom_orbit_distance: default_custom_distance(),
            custom_orbit_target: default_custom_target(),
            custom_orbit_roll: 0.0,
            cache_compression_enabled: true,
            cache_compression_mode: CacheCompressionMode::default(),
            cache_max_frames: default_cache_max_frames(),
            cache_max_memory_mb: default_cache_max_memory_mb(),
            cache_raw_size_mb: 0.0,
            cache_compression_ratio: 1.0,
            is_ram_previewing: false,
            ram_cache_enabled: true,
            cached_frames: HashSet::new(),
            playback_fps: 60.0,
            cache_size_mb: 0.0,
            ram_cache_purge_requested: false,
            auto_frame_cache: false,
            auto_cache_in_progress: false,
            pause_at_last_keyframe: false,
        }
    }
}

pub fn create_default_properties() -> HashMap<String, Property> {
    [
        ("anchorX", 0.0),
        ("anchorY", 0.0),
        ("anchorZ", 0.0),
        ("x", 960.0),
        ("y", 540.0),
        ("z", 0.0),
        ("rotation", 0.0),
        ("rotationX", 0.0),
        ("rotationY", 0.0),
        ("scaleX", 100.0),
        ("scaleY", 100.0),
        ("scaleZ", 100.0),
        ("opacity", 100.0),
        ("audioVolume", 100.0),
        ("audioPan", 0.0),
    ]
    .iter()
    .map(|(name, val)| {
        (
            name.to_string(),
            Property {
                name: name.to_string(),
                base_value: *val,
                keyframes: vec![],
                wiggle: None,
            },
        )
    })
    .collect()
}

pub fn create_camera_properties(width: f32, height: f32) -> HashMap<String, Property> {
    let zoom = 1500.0;
    [
        ("x", width / 2.0),
        ("y", height / 2.0),
        ("z", -zoom),
        ("poiX", width / 2.0),
        ("poiY", height / 2.0),
        ("poiZ", 0.0),
        ("rotation", 0.0),
        ("rotationX", 0.0),
        ("rotationY", 0.0),
        ("zoom", zoom),
        ("fov", 50.0),
    ]
    .iter()
    .map(|(name, val)| {
        (
            name.to_string(),
            Property {
                name: name.to_string(),
                base_value: *val,
                keyframes: vec![],
                wiggle: None,
            },
        )
    })
    .collect()
}

pub fn default_layer(name: String, source: LayerSource, label_color_index: usize) -> Layer {
    let properties = match &source {
        LayerSource::Camera => create_camera_properties(1920.0, 1080.0),
        _ => create_default_properties(),
    };
    Layer {
        name,
        source,
        properties,
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
        markers: vec![],
        effects: vec![],
    }
}
