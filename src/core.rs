use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct BezierControl { pub cp1: f32, pub cp2: f32 }

#[derive(Clone)]
pub struct SelectedKeyframe {
    pub layer_index: usize,
    pub property_name: String,
    pub keyframe_index: usize,
}

pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub ease: Option<BezierControl>,
}

pub struct Property {
    pub name: String,
    pub base_value: f32,
    pub keyframes: Vec<Keyframe>,
}

impl Property {
    pub fn get_value_at(&self, time: f32) -> f32 {
        if self.keyframes.is_empty() { return self.base_value; }
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
            } else if time >= curr.time { return curr.value; }
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

pub struct Layer { pub name: String, pub properties: HashMap<String, Property> }
pub struct Composition { pub layers: Vec<Layer>, pub current_time: f32, pub is_playing: bool }
