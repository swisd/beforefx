Custom plugins for the application are written as plain text files with the `.bfxplugin` extension and stored in the `./plugins` directory. They are parsed at runtime based on their header specifications and can be either **Effect** plugins (modifying pixel color data) or **Functional** plugins (automating composition actions and toolsets).

## 1. Plugin File Structure & Headers

Every plugin file must begin with a specification directive within the first 25 lines to define its type:

* **Effect Plugin Header:** `.spec effect` or `spec: effect`
* **Functional Plugin Header:** `.spec functional` or `spec: functional`

### Common Metadata Fields

* `name:` Display name of the plugin (falls back to the filename if omitted).
* `category:` Organization category in the UI (e.g., `Color Correction`, `Stylize`, `Animation Tools`).
* `description:` A brief summary of what the plugin does.

---

## 2. Creating an Effect Plugin

Effect plugins manipulate image pixels using either custom mathematical formulas or built-in rendering shortcuts.

### Defining Sliders

Parameters and user-adjustable UI sliders are declared using the following syntax:

```text
slider: name, default_value, min_value, max_value, step

```

*Alternative formats:* `slider: name = default` or `property: name, ...`

### Writing Formula Lines

Effect plugins can execute custom per-pixel math formulas using color channels (`r`, `g`, `b`, `a`), global `time`, and any declared slider names. Statements must end with a semicolon (`;`).

**Example: Custom Brightness/Tint Plugin (`sepia_tone.bfxplugin`)**

```text
.spec effect
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

```

### Supported Math Functions in Formulas

* `sin(x)`, `cos(x)`, `tan(x)`
* `abs(x)`, `sqrt(x)`
* `floor(x)`, `ceil(x)`, `round(x)`, `fract(x)`
* `min(a, b)`, `max(a, b)`
* `clamp(val, min, max)`
* `mix(a, b, t)` or `lerp(a, b, t)`
* `pow(base, exp)`
* `step(edge, x)`

---

## 3. Creating a Functional Plugin

Functional plugins automate repetitive workflow tasks, generate assets, or set up multi-layer rigs.

### Defining Actions

Use the `action:` keyword to bind the plugin to built-in automation routines:

```text
.spec functional
name: Create 3D Camera Rig
category: Cameras & 3D
description: Creates a 3D Camera with an Orbit Null Controller.
action: add_camera_rig
slider: distance, 1800.0, 500.0, 5000.0, 50.0
slider: orbit_speed, 1.0, 0.1, 10.0, 0.1

```

### Supported Built-in Actions

* `add_camera_rig`: Generates a 3D camera parented to an animated orbit null controller.
* `stagger_layers`: Automatically offsets the in/out times of layers across the timeline.
* `easy_ease_all`: Applies smooth cubic Bezier Easy Ease curves to keyframes.
* `add_adjustment_layer`: Creates an adjustment layer pre-populated with master FX.
* `create_palette_solids`: Generates a 5-color solid layer aesthetic palette.