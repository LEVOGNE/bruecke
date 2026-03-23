use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::PI;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// ─── js imports ───────────────────────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    fn js_now() -> f64;
    /// Called by WASM whenever state changes — JS fires async fetch to /state
    fn js_persist_state(json: &str);
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn js_now() -> f64 { 0.0 }

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn js_persist_state(_json: &str) {}

// ─── constants ────────────────────────────────────────────────────────────────
pub const SCENE_W: f32   = 800.0;
pub const SCENE_H: f32   = 600.0;
pub const SEGS:    usize = 64;
pub const LINE_W:  f32   = 3.0;
const COL_DEFAULT: [f32; 3] = [0.18, 0.52, 1.00];

// ─── transform ────────────────────────────────────────────────────────────────
#[derive(Clone, Debug, PartialEq)]
pub struct Transform {
    pub tx: f32,
    pub ty: f32,
    pub sx: f32,
    pub sy: f32,
}

impl Transform {
    pub fn identity() -> Self {
        Self { tx: 0.0, ty: 0.0, sx: 1.0, sy: 1.0 }
    }
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (x * self.sx + self.tx, y * self.sy + self.ty)
    }
}

// ─── shapes ───────────────────────────────────────────────────────────────────
#[derive(Clone, Debug)]
pub enum Shape {
    Rect   { x: f32, y: f32, w: f32, h: f32, col: [f32; 3], alpha: f32 },
    Circle { x: f32, y: f32, r: f32,          col: [f32; 3], alpha: f32 },
    Line   { x1: f32, y1: f32, x2: f32, y2: f32, col: [f32; 3], alpha: f32 },
}

// ─── draw state ───────────────────────────────────────────────────────────────
pub struct DrawState {
    pub shapes:        Vec<(Shape, Transform)>,
    pub transform:     Transform,
    pub current_color: [f32; 3],
    pub current_alpha: f32,
}

impl DrawState {
    pub fn new() -> Self {
        Self {
            shapes:        Vec::new(),
            transform:     Transform::identity(),
            current_color: COL_DEFAULT,
            current_alpha: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.shapes.clear();
        self.transform     = Transform::identity();
        self.current_color = COL_DEFAULT;
        self.current_alpha = 1.0;
    }

    /// Set current color. Accepts either 0–255 range (any component > 1.0)
    /// or 0.0–1.0 range (all components ≤ 1.0). Note: exactly 1.0 is treated as 0–1 range.
    pub fn set_color(&mut self, r: f32, g: f32, b: f32) {
        let scale = if r > 1.0 || g > 1.0 || b > 1.0 { 1.0 / 255.0 } else { 1.0 };
        self.current_color = [r * scale, g * scale, b * scale];
    }

    pub fn add_circle(&mut self, x: f32, y: f32, r: f32) {
        let col   = self.current_color;
        let alpha = self.current_alpha;
        self.shapes.push((Shape::Circle { x, y, r, col, alpha }, self.transform.clone()));
    }

    pub fn add_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let col   = self.current_color;
        let alpha = self.current_alpha;
        self.shapes.push((Shape::Rect { x, y, w, h, col, alpha }, self.transform.clone()));
    }

    pub fn add_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        let col   = self.current_color;
        let alpha = self.current_alpha;
        self.shapes.push((Shape::Line { x1, y1, x2, y2, col, alpha }, self.transform.clone()));
    }
}

// ─── thread-locals ────────────────────────────────────────────────────────────
thread_local! {
    static DRAW:     RefCell<DrawState>          = RefCell::new(DrawState::new());
    static CANVAS_W: RefCell<f32>                = RefCell::new(1200.0);
    static CANVAS_H: RefCell<f32>                = RefCell::new(800.0);
    /// Persistent key-value store. Values are stored as raw JSON strings.
    static STATE:    RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    /// RNG state for xorshift64 — seeded lazily from js_now() on first use.
    static RNG: RefCell<u64> = RefCell::new(0);
}

// ─── RNG (xorshift64) ─────────────────────────────────────────────────────────

fn rng_next() -> u64 {
    RNG.with(|rng| {
        let mut s = rng.borrow_mut();
        if *s == 0 {
            // seed from current time; fallback if js_now returns 0
            let t = js_now().to_bits();
            *s = if t == 0 { 0x9e3779b97f4a7c15 } else { t };
        }
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        *s
    })
}

/// Returns a float in [0.0, 1.0)
fn rng_f64() -> f64 {
    (rng_next() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// Returns an integer in [lo, hi] inclusive
fn rng_range(lo: i64, hi: i64) -> i64 {
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let range = (hi - lo + 1) as u64;
    lo + (rng_next() % range) as i64
}

// ─── image commands ───────────────────────────────────────────────────────────

/// One image draw command queued by Python each frame.
#[derive(Clone, Debug)]
pub struct ImageCmd {
    pub url:   String,
    pub x:     f32,  // scene x (0–800)
    pub y:     f32,  // scene y (0–600)
    pub w:     f32,  // dest width  (scene units)
    pub h:     f32,  // dest height (scene units)
    pub sx:    f32,  // source x in pixels (default 0)
    pub sy:    f32,  // source y in pixels (default 0)
    pub sw:    f32,  // source width  in pixels (-1 = full image)
    pub sh:    f32,  // source height in pixels (-1 = full image)
    pub angle: f32,  // rotation in degrees (default 0)
}

thread_local! {
    /// Image draw commands queued this frame by the Python image() builtin.
    /// Cleared by get_image_cmds_json() after each frame.
    static IMAGE_CMDS: RefCell<Vec<ImageCmd>> = RefCell::new(Vec::new());
}

thread_local! {
    /// Context-menu items as pipe-separated string, set each frame by _bruecke_after.
    static CONTEXTMENU_ITEMS:    RefCell<String> = RefCell::new(String::new());
    /// Index of the item the user selected (-1 = none). Consumed and reset to -1 each frame.
    static CONTEXTMENU_SELECTED: RefCell<i32>    = RefCell::new(-1);
    /// True when Python has set a custom cursor draw function — JS hides the SVG cursor overlay.
    static CURSOR_CUSTOM: RefCell<bool> = RefCell::new(false);
}

/// Serialise IMAGE_CMDS to a JSON array string and clear the list.
/// Called by the WASM export get_image_cmds() each frame.
pub fn get_image_cmds_json() -> String {
    IMAGE_CMDS.with(|cmds| {
        let mut cmds = cmds.borrow_mut();
        if cmds.is_empty() { return "[]".to_string(); }
        let items: Vec<serde_json::Value> = cmds.iter().map(|c| {
            serde_json::json!({
                "url": c.url,
                "x": c.x, "y": c.y, "w": c.w, "h": c.h,
                "sx": c.sx, "sy": c.sy, "sw": c.sw, "sh": c.sh,
                "angle": c.angle,
            })
        }).collect();
        cmds.clear();
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
    })
}

// ─── state helpers ────────────────────────────────────────────────────────────

fn get_state_json_string() -> String {
    STATE.with(|s| {
        let s = s.borrow();
        if s.is_empty() { return "{}".to_string(); }
        let pairs: Vec<String> = s.iter()
            .map(|(k, v)| format!("\"{}\":{}", k.replace('"', "\\\""), v))
            .collect();
        format!("{{{}}}", pairs.join(","))
    })
}

/// Hydrate STATE from a JSON object string. Called by JS after init().
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn set_state(json: &str) {
    if let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(json) {
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.clear();
            for (k, v) in map { st.insert(k, v.to_string()); }
        });
    }
}

/// Returns JSON array of image draw commands queued this frame, then clears the list.
/// Called by engine.js after each WASM frame() call.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_image_cmds() -> String {
    get_image_cmds_json()
}

fn cw() -> f32 { CANVAS_W.with(|v| *v.borrow()) }
fn ch() -> f32 { CANVAS_H.with(|v| *v.borrow()) }

// ─── tests ────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn clear_image_cmds() {
        IMAGE_CMDS.with(|c| c.borrow_mut().clear());
    }

    #[test]
    fn draw_state_starts_empty() {
        let ds = DrawState::new();
        assert!(ds.shapes.is_empty());
        assert_eq!(ds.current_color, COL_DEFAULT);
        assert_eq!(ds.current_alpha, 1.0);
    }

    #[test]
    fn draw_state_add_circle() {
        let mut ds = DrawState::new();
        ds.set_color(255.0, 128.0, 0.0);
        ds.add_circle(100.0, 200.0, 50.0);
        assert_eq!(ds.shapes.len(), 1);
        match &ds.shapes[0].0 {
            Shape::Circle { x, y, r, col, .. } => {
                assert_eq!(*x, 100.0);
                assert_eq!(*y, 200.0);
                assert_eq!(*r, 50.0);
                assert!((col[0] - 1.0).abs() < 0.01);
            }
            _ => panic!("expected circle"),
        }
    }

    #[test]
    fn draw_state_reset() {
        let mut ds = DrawState::new();
        ds.add_circle(1.0, 2.0, 3.0);
        ds.reset();
        assert!(ds.shapes.is_empty());
        assert_eq!(ds.transform, Transform::identity());
    }

    #[test]
    fn transform_identity() {
        let t = Transform::identity();
        assert_eq!(t.apply(100.0, 200.0), (100.0, 200.0));
    }

    #[test]
    fn transform_translate() {
        let t = Transform { tx: 10.0, ty: 20.0, sx: 1.0, sy: 1.0 };
        assert_eq!(t.apply(0.0, 0.0), (10.0, 20.0));
    }

    #[test]
    fn color_255_range_normalized() {
        let mut ds = DrawState::new();
        ds.set_color(255.0, 0.0, 0.0);
        assert!((ds.current_color[0] - 1.0).abs() < 0.01, "255 → 1.0");
        assert!((ds.current_color[1]).abs() < 0.01, "0 → 0.0");
    }

    #[test]
    fn color_01_range_unchanged() {
        let mut ds = DrawState::new();
        ds.set_color(0.5, 0.5, 0.5);
        assert!((ds.current_color[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn tess_circle_vertex_count() {
        CANVAS_W.with(|w| *w.borrow_mut() = 800.0);
        CANVAS_H.with(|h| *h.borrow_mut() = 600.0);
        DRAW.with(|d| {
            let mut ds = d.borrow_mut();
            ds.reset();
            ds.add_circle(400.0, 300.0, 50.0);
        });
        let verts = build_vertices();
        // SEGS triangles × 3 vertices × 6 floats
        assert_eq!(verts.len(), SEGS * 3 * 6);
    }

    #[test]
    fn tess_rect_vertex_count() {
        CANVAS_W.with(|w| *w.borrow_mut() = 800.0);
        CANVAS_H.with(|h| *h.borrow_mut() = 600.0);
        DRAW.with(|d| {
            let mut ds = d.borrow_mut();
            ds.reset();
            ds.add_rect(0.0, 0.0, 800.0, 600.0);
        });
        let verts = build_vertices();
        // 2 triangles × 3 vertices × 6 floats
        assert_eq!(verts.len(), 6 * 6);
    }

    #[test]
    fn tess_circle_center_at_ndc_origin() {
        CANVAS_W.with(|w| *w.borrow_mut() = 800.0);
        CANVAS_H.with(|h| *h.borrow_mut() = 600.0);
        DRAW.with(|d| {
            let mut ds = d.borrow_mut();
            ds.reset();
            ds.add_circle(400.0, 300.0, 50.0); // center of 800×600 scene
        });
        let verts = build_vertices();
        // first vertex of first triangle is the center, should be NDC (0, 0)
        assert!((verts[0]).abs() < 0.01, "center x NDC should be ~0, got {}", verts[0]);
        assert!((verts[1]).abs() < 0.01, "center y NDC should be ~0, got {}", verts[1]);
    }

    #[test]
    fn tess_line_vertex_count() {
        CANVAS_W.with(|w| *w.borrow_mut() = 800.0);
        CANVAS_H.with(|h| *h.borrow_mut() = 600.0);
        DRAW.with(|d| {
            let mut ds = d.borrow_mut();
            ds.reset();
            ds.add_line(0.0, 0.0, 100.0, 100.0);
        });
        let verts = build_vertices();
        // 2 triangles × 3 vertices × 6 floats
        assert_eq!(verts.len(), 6 * 6);
    }

    // ─── state tests ──────────────────────────────────────────────────────────

    fn clear_state() {
        STATE.with(|s| s.borrow_mut().clear());
    }

    #[test]
    fn state_empty_returns_braces() {
        clear_state();
        assert_eq!(get_state_json_string(), "{}");
    }

    #[test]
    fn state_single_int() {
        clear_state();
        STATE.with(|s| s.borrow_mut().insert("score".into(), "42".into()));
        let j = get_state_json_string();
        assert_eq!(j, r#"{"score":42}"#);
    }

    #[test]
    fn state_single_string_value() {
        clear_state();
        STATE.with(|s| s.borrow_mut().insert("name".into(), r#""Player""#.into()));
        let j = get_state_json_string();
        assert_eq!(j, r#"{"name":"Player"}"#);
    }

    #[test]
    fn state_multiple_values_valid_json() {
        clear_state();
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.insert("a".into(), "1".into());
            st.insert("b".into(), "2".into());
        });
        let j = get_state_json_string();
        // parse with serde_json to verify it is valid JSON and has correct values
        let v: serde_json::Value = serde_json::from_str(&j).expect("valid JSON");
        assert_eq!(v["a"], serde_json::json!(1));
        assert_eq!(v["b"], serde_json::json!(2));
    }

    #[test]
    fn state_cleared_returns_braces() {
        STATE.with(|s| s.borrow_mut().insert("x".into(), "99".into()));
        clear_state();
        assert_eq!(get_state_json_string(), "{}");
    }

    // ─── image cmd tests ──────────────────────────────────────────────────────

    #[test]
    fn image_cmd_pushed_with_defaults() {
        clear_image_cmds();
        IMAGE_CMDS.with(|c| c.borrow_mut().push(ImageCmd {
            url: "a.png".into(),
            x: 10.0, y: 20.0, w: 32.0, h: 32.0,
            sx: 0.0, sy: 0.0, sw: -1.0, sh: -1.0, angle: 0.0,
        }));
        let j = get_image_cmds_json();
        let v: serde_json::Value = serde_json::from_str(&j).unwrap();
        assert_eq!(v[0]["sw"], -1.0);
        assert_eq!(v[0]["angle"], 0.0);
    }

    #[test]
    fn image_cmds_empty_returns_bracket_pair() {
        clear_image_cmds();
        assert_eq!(get_image_cmds_json(), "[]");
    }

    #[test]
    fn image_cmds_single_cmd_correct_json() {
        clear_image_cmds();
        IMAGE_CMDS.with(|c| c.borrow_mut().push(ImageCmd {
            url: "hero.png".into(),
            x: 100.0, y: 200.0, w: 48.0, h: 48.0,
            sx: 0.0, sy: 0.0, sw: -1.0, sh: -1.0,
            angle: 0.0,
        }));
        let j = get_image_cmds_json();
        let v: serde_json::Value = serde_json::from_str(&j).expect("valid JSON");
        assert_eq!(v[0]["url"],   "hero.png");
        assert_eq!(v[0]["x"],     100.0);
        assert_eq!(v[0]["angle"], 0.0);
    }

    #[test]
    fn image_cmds_clears_after_call() {
        clear_image_cmds();
        IMAGE_CMDS.with(|c| c.borrow_mut().push(ImageCmd {
            url: "a.png".into(),
            x: 0.0, y: 0.0, w: 10.0, h: 10.0,
            sx: 0.0, sy: 0.0, sw: -1.0, sh: -1.0,
            angle: 0.0,
        }));
        let _ = get_image_cmds_json();
        assert_eq!(get_image_cmds_json(), "[]");
    }
}

// ─── tessellation ─────────────────────────────────────────────────────────────

fn s2c(sx: f32, sy: f32) -> (f32, f32) {
    (sx / SCENE_W * cw(), sy / SCENE_H * ch())
}

fn vert(buf: &mut Vec<f32>, px: f32, py: f32, c: [f32; 3], a: f32) {
    buf.extend_from_slice(&[
        px / cw() * 2.0 - 1.0,
        1.0 - py / ch() * 2.0,
        c[0], c[1], c[2], a,
    ]);
}

fn tess_rect(buf: &mut Vec<f32>, x: f32, y: f32, w: f32, h: f32,
             t: &Transform, col: [f32; 3], a: f32) {
    let (ax, ay) = s2c(t.apply(x,     y    ).0, t.apply(x,     y    ).1);
    let (bx, by) = s2c(t.apply(x + w, y    ).0, t.apply(x + w, y    ).1);
    let (cx, cy) = s2c(t.apply(x + w, y + h).0, t.apply(x + w, y + h).1);
    let (dx, dy) = s2c(t.apply(x,     y + h).0, t.apply(x,     y + h).1);
    vert(buf, ax, ay, col, a); vert(buf, bx, by, col, a); vert(buf, cx, cy, col, a);
    vert(buf, ax, ay, col, a); vert(buf, cx, cy, col, a); vert(buf, dx, dy, col, a);
}

fn tess_circle(buf: &mut Vec<f32>, cx_: f32, cy_: f32, r: f32,
               t: &Transform, col: [f32; 3], a: f32) {
    let (cx, cy) = s2c(t.apply(cx_, cy_).0, t.apply(cx_, cy_).1);
    let rs = r * t.sx * cw() / SCENE_W;
    for i in 0..SEGS {
        let a1 = i       as f32 * 2.0 * PI / SEGS as f32;
        let a2 = (i + 1) as f32 * 2.0 * PI / SEGS as f32;
        vert(buf, cx,                 cy,                 col, a);
        vert(buf, cx + rs * a1.cos(), cy + rs * a1.sin(), col, a);
        vert(buf, cx + rs * a2.cos(), cy + rs * a2.sin(), col, a);
    }
}

fn tess_line(buf: &mut Vec<f32>, x1: f32, y1: f32, x2: f32, y2: f32,
             t: &Transform, col: [f32; 3], a: f32) {
    let (ax, ay) = s2c(t.apply(x1, y1).0, t.apply(x1, y1).1);
    let (bx, by) = s2c(t.apply(x2, y2).0, t.apply(x2, y2).1);
    let dx  = bx - ax;
    let dy  = by - ay;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let pw  = LINE_W * 0.5;
    let px  = -dy / len * pw;
    let py  =  dx / len * pw;
    vert(buf, ax + px, ay + py, col, a); vert(buf, ax - px, ay - py, col, a);
    vert(buf, bx + px, by + py, col, a);
    vert(buf, ax - px, ay - py, col, a); vert(buf, bx - px, by - py, col, a);
    vert(buf, bx + px, by + py, col, a);
}

pub fn build_vertices() -> Vec<f32> {
    let mut buf = Vec::new();
    DRAW.with(|draw| {
        for (shape, t) in &draw.borrow().shapes {
            match shape {
                Shape::Rect   { x, y, w, h, col, alpha }     =>
                    tess_rect(&mut buf, *x, *y, *w, *h, t, *col, *alpha),
                Shape::Circle { x, y, r, col, alpha }         =>
                    tess_circle(&mut buf, *x, *y, *r, t, *col, *alpha),
                Shape::Line   { x1, y1, x2, y2, col, alpha } =>
                    tess_line(&mut buf, *x1, *y1, *x2, *y2, t, *col, *alpha),
            }
        }
    });
    buf
}

// ─── App state ────────────────────────────────────────────────────────────────
pub struct AppState {
    pub run_start_ts:  f64,
    pub compile_ms:    f64,
    pub source_lines:  usize,
    pub source_bytes:  usize,
    pub error_msg:     Option<String>,
    pub last_good_verts: Vec<f32>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            run_start_ts:  0.0,
            compile_ms:    0.0,
            source_lines:  0,
            source_bytes:  0,
            error_msg:     None,
            last_good_verts: Vec::new(),
        }
    }
}

thread_local! {
    static APP: RefCell<AppState> = RefCell::new(AppState::new());
}

// ─── WASM exports ─────────────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn on_resize(pw: f32, ph: f32, _dpr: f32) {
    CANVAS_W.with(|w| *w.borrow_mut() = pw);
    CANVAS_H.with(|h| *h.borrow_mut() = ph);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn fps_target() -> u32 { 60 }

/// Called when new Python source arrives via SSE.
/// Compiles the source + executes module body (class defs, globals).
/// The scope is stored in PYTHON_SCOPE — frame(t) runs in it every frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn on_run(source: &str) {
    let ts = js_now();
    let source_lines = source.lines().filter(|l| !l.trim().is_empty()).count();
    let source_bytes = source.len();

    // compile OUTSIDE the APP borrow — compile_source stores to PYTHON_SCOPE,
    // not APP, so there is no re-entrant borrow of APP here.
    let result = python::compile_source(source);

    APP.with(|app| {
        let mut app = app.borrow_mut();
        match result {
            Err(e) => {
                app.error_msg = Some(e);
            }
            Ok(()) => {
                app.run_start_ts  = ts;
                app.compile_ms    = js_now() - ts;
                app.source_lines  = source_lines;
                app.source_bytes  = source_bytes;
                app.error_msg     = None;
            }
        }
    });
}

/// Main loop — called every frame by engine.js.
/// Returns vertex buffer: stride 6 floats [ndc_x, ndc_y, r, g, b, alpha]
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn frame(ts: f64, keys: u32, mx: f32, my: f32, btn: u32) -> Box<[f32]> {
    // Belt-and-suspenders: clears stale image cmds if frame() errors out
    // before JS calls get_image_cmds(). get_image_cmds_json() also clears on read.
    IMAGE_CMDS.with(|c| c.borrow_mut().clear());
    let t = APP.with(|app| {
        let app = app.borrow();
        ((ts - app.run_start_ts) / 1000.0) as f64
    });

    let result = python::call_frame(t, keys, mx, my, btn);

    APP.with(|app| {
        let mut app = app.borrow_mut();
        match result {
            Ok(()) => {
                app.error_msg = None;
                let verts = build_vertices();
                app.last_good_verts = verts.clone();
                verts.into_boxed_slice()
            }
            Err(e) if e == "__no_script__" => {
                // no compiled script yet — keep existing error_msg from on_run
                app.last_good_verts.clone().into_boxed_slice()
            }
            Err(e) => {
                app.error_msg = Some(e);
                app.last_good_verts.clone().into_boxed_slice()
            }
        }
    })
}

/// True when Python has set a custom cursor draw function (JS hides SVG cursor overlay).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_cursor_custom() -> bool {
    CURSOR_CUSTOM.with(|c| *c.borrow())
}

/// Returns pipe-separated context-menu item labels set by Python this frame.
/// Empty string = no menu / hide menu.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_contextmenu_items() -> String {
    CONTEXTMENU_ITEMS.with(|ci| ci.borrow().clone())
}

/// Called by engine.js when the user selects a context-menu item.
/// index is delivered to Python as _contextmenu_selected in the next frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn on_contextmenu_select(index: i32) {
    CONTEXTMENU_SELECTED.with(|s| *s.borrow_mut() = index);
}

/// Returns JSON with status, error, and performance stats.
/// engine.js reads this after each frame.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_result() -> String {
    APP.with(|app| {
        let app = app.borrow();
        match &app.error_msg {
            Some(e) => {
                let esc = e
                    .replace('\\', "\\\\")
                    .replace('"',  "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r");
                format!(r#"{{"ok":false,"error":"{}"}}"#, esc)
            }
            None => {
                let shapes   = DRAW.with(|d| d.borrow().shapes.len());
                let vertices = DRAW.with(|d| {
                    d.borrow().shapes.iter().map(|s| match s.0 {
                        Shape::Rect   { .. } => 6,
                        Shape::Circle { .. } => SEGS * 3,
                        Shape::Line   { .. } => 6,
                    }).sum::<usize>()
                });
                format!(
                    r#"{{"ok":true,"shapes":{},"vertices":{},"compile_ms":{:.3},"source_lines":{},"source_bytes":{}}}"#,
                    shapes, vertices,
                    app.compile_ms, app.source_lines, app.source_bytes,
                )
            }
        }
    })
}

// ─── Python runtime ───────────────────────────────────────────────────────────
/// Python preamble: runs before every user script.
/// Registers `bruecke`, `random`, and `math` in sys.modules so that
/// all standard imports work: `from bruecke import *`, `import math`, etc.
const COMPAT_PREAMBLE: &str = r#"
import sys as _sys
class _M: pass

# ── random ──────────────────────────────────────────────────────────────────
_r = _M()
_r.random   = rand
_r.randint  = randint
_r.choice   = choice
_r.shuffle  = shuffle
_r.uniform  = lambda a, b: a + rand() * (b - a)
_r.seed     = lambda *a: None
_r.sample   = lambda population, k: [choice(population) for _ in range(k)]
_sys.modules['random'] = _r

# ── math ────────────────────────────────────────────────────────────────────
_m = _M()
_m.pi       = pi
_m.e        = 2.718281828459045
_m.tau      = 6.283185307179586
_m.inf      = float('inf')
_m.sin      = sin
_m.cos      = cos
_m.tan      = tan
_m.sqrt     = sqrt
_m.floor    = floor
_m.ceil     = ceil
_m.fabs     = abs
_m.atan2    = atan2
_m.hypot    = hypot
_m.radians  = lambda d: d * pi / 180.0
_m.degrees  = lambda r: r * 180.0 / pi
_m.log      = lambda x, base=None: log(x) if base is None else log(x) / log(base)
_log2_inv   = 1.0 / log(2.0)
_log10_inv  = 1.0 / log(10.0)
_m.log2     = lambda x: log(x) * _log2_inv
_m.log10    = lambda x: log(x) * _log10_inv
_m.exp      = exp
_m.pow      = lambda x, y: x ** y
_m.copysign = lambda x, y: abs(x) if y >= 0 else -abs(x)
_m.isfinite = lambda x: x == x and abs(x) != float('inf')
_m.isinf    = lambda x: abs(x) == float('inf')
_m.isnan    = lambda x: x != x
_sys.modules['math'] = _m

# ── bruecke ──────────────────────────────────────────────────────────────────
# Full draw + input API — import like any Python package:
#   from bruecke import *
#   import bruecke; bruecke.circle(x, y, r)
_b = _M()
# draw
_b.color     = color
_b.alpha     = alpha
_b.rect      = rect
_b.circle    = circle
_b.line      = line
_b.image     = image
_b.translate = translate
_b.scale     = scale
# math extras
_b.lerp      = lerp
_b.clamp     = clamp
_b.sign      = sign
# scene constants
_b.W         = 800
_b.H         = 600
# per-frame inputs (initial 0 — engine updates these in scope.globals each frame)
_b.t         = 0.0
_b.mouse_x   = 0.0
_b.mouse_y   = 0.0
_b.mouse_btn = 0
_b.keys      = 0
_b.__all__   = [
    'color','alpha','rect','circle','line','image','translate','scale',
    'lerp','clamp','sign',
    'W','H',
    't','mouse_x','mouse_y','mouse_btn','keys',
]
_sys.modules['bruecke'] = _b

# ── bruecke.cursor ───────────────────────────────────────────────────────────
_bc = _M()
_bc.color   = (255, 220, 80)
_bc.size    = 16
_bc.visible = True
_bc.draw    = None
_b.cursor   = _bc

# ── bruecke.contextmenu ──────────────────────────────────────────────────────
_bcm = _M()
_bcm._items = []
_b.contextmenu = _bcm

# ── engine after-frame hook ───────────────────────────────────────────────────
# _bruecke_after is called by the engine AFTER the user's frame().
# Default arg capture keeps _bc/_bcm alive even after del below.
def _b_after_frame(_cur=_bc, _cm=_bcm):
    sel = _contextmenu_selected
    if 0 <= sel < len(_cm._items):
        _label, cb = _cm._items[sel]
        if cb is not None:
            cb()
    has_custom = _cur.draw is not None
    _set_cursor_custom(has_custom)
    if _cur.visible and has_custom:
        _cur.draw()
    _set_contextmenu('|'.join(item[0] for item in _cm._items))
_b._after_frame = _b_after_frame
_bruecke_after  = _b_after_frame
del _bc, _bcm, _b_after_frame

del _sys, _r, _m, _b, _M, _log2_inv, _log10_inv
"#;

#[cfg(target_arch = "wasm32")]
pub mod python {
    use super::*;
    use rustpython_vm as vm;
    use rustpython_vm::AsObject;
    use vm::function::ArgIntoFloat;
    use vm::TryFromObject;

    thread_local! {
        static INTERP: vm::Interpreter = create_interpreter();
        /// Compiled Python scope lives here — separate from APP so that
        /// compile_source() can store it without re-borrowing APP while
        /// on_run() already holds a mutable borrow on APP.
        static PYTHON_SCOPE: RefCell<Option<vm::scope::Scope>> = RefCell::new(None);
    }

    fn create_interpreter() -> vm::Interpreter {
        vm::Interpreter::with_init(Default::default(), |virt| {
            // Register draw API as Python builtins
            register_builtins(virt);
        })
    }

    // ── JSON ↔ Python value converters ───────────────────────────────────────

    /// Convert a Python value to a JSON string. Supports: None, bool, int, float, str.
    fn py_to_json(obj: &vm::PyObjectRef, vm: &vm::VirtualMachine) -> vm::PyResult<String> {
        if obj.is(&vm.ctx.none()) { return Ok("null".to_string()); }
        // bool before int — PyBool is a subclass of PyInt
        if obj.class().fast_issubclass(vm.ctx.types.bool_type) {
            return Ok(if obj.clone().is_true(vm)? { "true" } else { "false" }.to_string());
        }
        if let Some(i) = obj.payload::<vm::builtins::PyInt>() {
            return Ok(i.as_bigint().to_string());
        }
        if let Some(f) = obj.payload::<vm::builtins::PyFloat>() {
            let v = f.to_f64();
            if !v.is_finite() {
                return Err(vm.new_value_error("store() value must be finite".to_string()));
            }
            return Ok(v.to_string());
        }
        if let Some(s) = obj.payload::<vm::builtins::PyStr>() {
            let esc = s.as_str()
                .replace('\\', "\\\\").replace('"', "\\\"")
                .replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
            return Ok(format!("\"{}\"", esc));
        }
        Err(vm.new_type_error(format!(
            "store() supports str/int/float/bool/None, got {}",
            obj.class().name()
        )))
    }

    /// Parse a JSON primitive string back to a Python object.
    fn json_to_py(s: &str, vm: &vm::VirtualMachine) -> vm::PyObjectRef {
        let s = s.trim();
        match s {
            "null"  => vm.ctx.none(),
            "true"  => vm.ctx.new_bool(true).into(),
            "false" => vm.ctx.new_bool(false).into(),
            _ if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 => {
                let inner = &s[1..s.len()-1];
                let unesc = inner
                    .replace("\\\"", "\"").replace("\\\\", "\\")
                    .replace("\\n", "\n").replace("\\r", "\r").replace("\\t", "\t");
                vm.ctx.new_str(unesc).into()
            }
            _ => {
                if let Ok(n) = s.parse::<i64>()  { return vm.ctx.new_int(n).into(); }
                if let Ok(f) = s.parse::<f64>()  { return vm.ctx.new_float(f).into(); }
                vm.ctx.new_str(s.to_string()).into()
            }
        }
    }

    fn register_builtins(virt: &mut vm::VirtualMachine) {
        // circle(x, y, r)
        let f = virt.new_function("circle", |x: ArgIntoFloat, y: ArgIntoFloat, r: ArgIntoFloat| {
            DRAW.with(|d| d.borrow_mut().add_circle(*x as f32, *y as f32, *r as f32));
        });
        virt.builtins.set_attr("circle", f, virt).unwrap();

        // rect(x, y, w, h)
        let f = virt.new_function("rect", |x: ArgIntoFloat, y: ArgIntoFloat, w: ArgIntoFloat, h: ArgIntoFloat| {
            DRAW.with(|d| d.borrow_mut().add_rect(*x as f32, *y as f32, *w as f32, *h as f32));
        });
        virt.builtins.set_attr("rect", f, virt).unwrap();

        // line(x1, y1, x2, y2)
        let f = virt.new_function("line", |x1: ArgIntoFloat, y1: ArgIntoFloat, x2: ArgIntoFloat, y2: ArgIntoFloat| {
            DRAW.with(|d| d.borrow_mut().add_line(*x1 as f32, *y1 as f32, *x2 as f32, *y2 as f32));
        });
        virt.builtins.set_attr("line", f, virt).unwrap();

        // color(r, g, b)
        let f = virt.new_function("color", |r: ArgIntoFloat, g: ArgIntoFloat, b: ArgIntoFloat| {
            DRAW.with(|d| d.borrow_mut().set_color(*r as f32, *g as f32, *b as f32));
        });
        virt.builtins.set_attr("color", f, virt).unwrap();

        // alpha(a)
        let f = virt.new_function("alpha", |a: ArgIntoFloat| {
            DRAW.with(|d| {
                d.borrow_mut().current_alpha = (*a as f32).clamp(0.0, 1.0);
            });
        });
        virt.builtins.set_attr("alpha", f, virt).unwrap();

        // clear()
        let f = virt.new_function("clear", || {
            DRAW.with(|d| d.borrow_mut().reset());
        });
        virt.builtins.set_attr("clear", f, virt).unwrap();

        // translate(dx, dy)
        let f = virt.new_function("translate", |dx: ArgIntoFloat, dy: ArgIntoFloat| {
            DRAW.with(|d| {
                let mut ds = d.borrow_mut();
                ds.transform.tx += *dx as f32;
                ds.transform.ty += *dy as f32;
            });
        });
        virt.builtins.set_attr("translate", f, virt).unwrap();

        // scale(s)
        let f = virt.new_function("scale", |s: ArgIntoFloat| {
            DRAW.with(|d| {
                let mut ds = d.borrow_mut();
                ds.transform.sx *= *s as f32;
                ds.transform.sy *= *s as f32;
            });
        });
        virt.builtins.set_attr("scale", f, virt).unwrap();

        // Math: sin, cos, tan, sqrt, abs, floor, ceil
        let f = virt.new_function("sin", |x: ArgIntoFloat| -> f64 { x.sin() });
        virt.builtins.set_attr("sin", f, virt).unwrap();

        let f = virt.new_function("cos", |x: ArgIntoFloat| -> f64 { x.cos() });
        virt.builtins.set_attr("cos", f, virt).unwrap();

        let f = virt.new_function("tan", |x: ArgIntoFloat| -> f64 { x.tan() });
        virt.builtins.set_attr("tan", f, virt).unwrap();

        let f = virt.new_function("sqrt", |x: ArgIntoFloat| -> f64 { x.sqrt() });
        virt.builtins.set_attr("sqrt", f, virt).unwrap();

        let f = virt.new_function("floor", |x: ArgIntoFloat| -> f64 { x.floor() });
        virt.builtins.set_attr("floor", f, virt).unwrap();

        let f = virt.new_function("ceil", |x: ArgIntoFloat| -> f64 { x.ceil() });
        virt.builtins.set_attr("ceil", f, virt).unwrap();

        // Note: abs already exists as a Python builtin, so we skip it.

        // ── extra math ───────────────────────────────────────────────────────

        // atan2(y, x) → angle in radians (like math.atan2)
        let f = virt.new_function("atan2", |y: ArgIntoFloat, x: ArgIntoFloat| -> f64 {
            y.atan2(*x)
        });
        virt.builtins.set_attr("atan2", f, virt).unwrap();

        // hypot(x, y) → Euclidean distance
        let f = virt.new_function("hypot", |x: ArgIntoFloat, y: ArgIntoFloat| -> f64 {
            x.hypot(*y)
        });
        virt.builtins.set_attr("hypot", f, virt).unwrap();

        // lerp(a, b, t) → linear interpolation
        let f = virt.new_function("lerp", |a: ArgIntoFloat, b: ArgIntoFloat, t: ArgIntoFloat| -> f64 {
            *a + (*b - *a) * *t
        });
        virt.builtins.set_attr("lerp", f, virt).unwrap();

        // clamp(v, lo, hi) → v clamped to [lo, hi]
        let f = virt.new_function("clamp", |v: ArgIntoFloat, lo: ArgIntoFloat, hi: ArgIntoFloat| -> f64 {
            (*v).max(*lo).min(*hi)
        });
        virt.builtins.set_attr("clamp", f, virt).unwrap();

        // sign(x) → -1.0, 0.0, or 1.0
        let f = virt.new_function("sign", |x: ArgIntoFloat| -> f64 {
            if *x > 0.0 { 1.0 } else if *x < 0.0 { -1.0 } else { 0.0 }
        });
        virt.builtins.set_attr("sign", f, virt).unwrap();

        // log(x) → natural logarithm
        let f = virt.new_function("log", |x: ArgIntoFloat| -> f64 { (*x as f64).ln() });
        virt.builtins.set_attr("log", f, virt).unwrap();

        // exp(x) → e^x
        let f = virt.new_function("exp", |x: ArgIntoFloat| -> f64 { (*x as f64).exp() });
        virt.builtins.set_attr("exp", f, virt).unwrap();

        // ── random ───────────────────────────────────────────────────────────

        // rand() → float in [0.0, 1.0)
        let f = virt.new_function("rand", || -> f64 { rng_f64() });
        virt.builtins.set_attr("rand", f, virt).unwrap();

        // randint(a, b) → int in [a, b] inclusive
        let f = virt.new_function("randint", |a: ArgIntoFloat, b: ArgIntoFloat| -> i64 {
            rng_range(*a as i64, *b as i64)
        });
        virt.builtins.set_attr("randint", f, virt).unwrap();

        // choice(seq) → random element from a list or tuple
        let f = virt.new_function("choice",
            |seq: vm::PyObjectRef, vm: &vm::VirtualMachine|
            -> vm::PyResult<vm::PyObjectRef>
        {
            let items: Vec<vm::PyObjectRef> = vm.extract_elements_with(&seq, |obj| Ok(obj))
                .map_err(|_| vm.new_type_error("choice() requires a sequence".to_string()))?;
            if items.is_empty() {
                return Err(vm.new_index_error("choice from empty sequence".to_string()));
            }
            let idx = (rng_next() % items.len() as u64) as usize;
            Ok(items[idx].clone())
        });
        virt.builtins.set_attr("choice", f, virt).unwrap();

        // shuffle(lst) → shuffles a list in-place (Fisher-Yates)
        let f = virt.new_function("shuffle",
            |lst: vm::builtins::PyListRef, _vm: &vm::VirtualMachine|
            -> vm::PyResult<()>
        {
            let mut items = lst.borrow_vec().to_vec();
            let n = items.len();
            for i in (1..n).rev() {
                let j = (rng_next() % (i as u64 + 1)) as usize;
                items.swap(i, j);
            }
            let mut borrow = lst.borrow_vec_mut();
            for (i, item) in items.into_iter().enumerate() {
                borrow[i] = item;
            }
            Ok(())
        });
        virt.builtins.set_attr("shuffle", f, virt).unwrap();

        // ── persistent state ─────────────────────────────────────────────────

        // load(key, default) → value from persistent store, or default
        let f = virt.new_function("load",
            |key: vm::builtins::PyStrRef, default: vm::PyObjectRef, vm: &vm::VirtualMachine|
            -> vm::PyResult<vm::PyObjectRef>
        {
            let json_str = STATE.with(|s| s.borrow().get(key.as_str()).cloned());
            match json_str {
                None    => Ok(default),
                Some(j) => Ok(json_to_py(&j, vm)),
            }
        });
        virt.builtins.set_attr("load", f, virt).unwrap();

        // store(key, value) — persist a JSON-serialisable value
        let f = virt.new_function("store",
            |key: vm::builtins::PyStrRef, value: vm::PyObjectRef, vm: &vm::VirtualMachine|
            -> vm::PyResult<()>
        {
            let json_val = py_to_json(&value, vm)?;
            STATE.with(|s| s.borrow_mut().insert(key.as_str().to_string(), json_val));
            js_persist_state(&get_state_json_string());
            Ok(())
        });
        virt.builtins.set_attr("store", f, virt).unwrap();

        // remove(key) — delete a key from the store
        let f = virt.new_function("remove",
            |key: vm::builtins::PyStrRef| -> ()
        {
            STATE.with(|s| s.borrow_mut().remove(key.as_str()));
            js_persist_state(&get_state_json_string());
        });
        virt.builtins.set_attr("remove", f, virt).unwrap();

        // image(url, x, y, w, h, sx=0, sy=0, sw=-1, sh=-1, angle=0)
        // sx/sy/sw/sh are source rect in pixels. sw/sh = -1 means full image.
        // angle is clockwise rotation in degrees.
        // We use FuncArgs directly because 10 params exceed the 7-tuple impl limit.
        let f = virt.new_function("image",
            |mut args: vm::function::FuncArgs, vm: &vm::VirtualMachine| -> vm::PyResult<()>
        {
            // Extract required positional args
            let url_obj = args.take_positional()
                .ok_or_else(|| vm.new_type_error("image() missing argument 'url'".to_string()))?;
            let url = url_obj.downcast::<vm::builtins::PyStr>()
                .map_err(|_| vm.new_type_error("image(): url must be a string".to_string()))?;

            fn take_float(args: &mut vm::function::FuncArgs, name: &str, vm: &vm::VirtualMachine) -> vm::PyResult<f64> {
                let obj = args.take_positional_keyword(name)
                    .ok_or_else(|| vm.new_type_error(format!("image() missing argument '{}'", name)))?;
                Ok(*ArgIntoFloat::try_from_object(vm, obj)?)
            }

            fn take_float_opt(args: &mut vm::function::FuncArgs, name: &str, vm: &vm::VirtualMachine, default: f64) -> vm::PyResult<f64> {
                match args.take_positional_keyword(name) {
                    None => Ok(default),
                    Some(obj) => Ok(*ArgIntoFloat::try_from_object(vm, obj)?),
                }
            }

            let x     = take_float(&mut args, "x",     vm)?;
            let y     = take_float(&mut args, "y",     vm)?;
            let w     = take_float(&mut args, "w",     vm)?;
            let h     = take_float(&mut args, "h",     vm)?;
            let sx    = take_float_opt(&mut args, "sx",    vm,  0.0)?;
            let sy    = take_float_opt(&mut args, "sy",    vm,  0.0)?;
            let sw    = take_float_opt(&mut args, "sw",    vm, -1.0)?;
            let sh    = take_float_opt(&mut args, "sh",    vm, -1.0)?;
            let angle = take_float_opt(&mut args, "angle", vm,  0.0)?;

            if args.take_positional().is_some() {
                return Err(vm.new_type_error(
                    "image() takes from 5 to 10 positional arguments".to_string()
                ));
            }

            if let Some(name) = args.kwargs.keys().next() {
                return Err(vm.new_type_error(
                    format!("image() got an unexpected keyword argument '{}'", name)
                ));
            }

            IMAGE_CMDS.with(|cmds| cmds.borrow_mut().push(ImageCmd {
                url:   url.as_str().to_string(),
                x:     x as f32,
                y:     y as f32,
                w:     w as f32,
                h:     h as f32,
                sx:    sx as f32,
                sy:    sy as f32,
                sw:    sw as f32,
                sh:    sh as f32,
                angle: angle as f32,
            }));
            Ok(())
        });
        virt.builtins.set_attr("image", f, virt).unwrap();

        // _set_contextmenu(pipe_str) — called by _bruecke_after each frame
        let f = virt.new_function("_set_contextmenu", |items: vm::builtins::PyStrRef| {
            CONTEXTMENU_ITEMS.with(|ci| *ci.borrow_mut() = items.as_str().to_string());
        });
        virt.builtins.set_attr("_set_contextmenu", f, virt).unwrap();

        // _set_cursor_custom(bool) — true when Python has a custom cursor draw fn
        let f = virt.new_function("_set_cursor_custom", |v: bool| {
            CURSOR_CUSTOM.with(|c| *c.borrow_mut() = v);
        });
        virt.builtins.set_attr("_set_cursor_custom", f, virt).unwrap();
    }

    fn exception_to_string(vm: &vm::VirtualMachine, exc: vm::builtins::PyBaseExceptionRef) -> String {
        let mut buf = String::new();
        let _ = vm.write_exception(&mut buf, &exc);
        buf
    }

    /// Compile Python source, reset scope, run module body, store scope in PYTHON_SCOPE.
    /// Stores to PYTHON_SCOPE (not APP) so this function is safe to call
    /// from outside an APP borrow — no double-borrow risk.
    pub fn compile_source(source: &str) -> Result<(), String> {
        INTERP.with(|interp| {
            interp.enter(|vm| -> Result<(), String> {
                let scope = vm.new_scope_with_builtins();

                // Inject math constants into globals
                scope.globals
                    .set_item("pi", vm.new_pyobj(std::f64::consts::PI), vm)
                    .map_err(|e| exception_to_string(vm, e))?;

                // Run compatibility preamble (import random / import math support)
                let preamble = vm
                    .compile(COMPAT_PREAMBLE, vm::compiler::Mode::Exec, "<compat>".to_owned())
                    .map_err(|e| vm.new_syntax_error(&e, Some(COMPAT_PREAMBLE)))
                    .map_err(|e| exception_to_string(vm, e))?;
                vm.run_code_obj(preamble, scope.clone())
                    .map_err(|e| format!("compat preamble: {}", exception_to_string(vm, e)))?;

                // Compile and run the module body
                let code = vm
                    .compile(source, vm::compiler::Mode::Exec, "<bruecke>".to_owned())
                    .map_err(|e| vm.new_syntax_error(&e, Some(source)))
                    .map_err(|e| exception_to_string(vm, e))?;

                vm.run_code_obj(code, scope.clone())
                    .map_err(|e| exception_to_string(vm, e))?;

                // Store scope in its own thread-local — never touches APP.
                PYTHON_SCOPE.with(|ps| {
                    *ps.borrow_mut() = Some(scope);
                });

                Ok(())
            })
        })
    }

    /// Call `frame(t)` in the persistent Python scope.
    pub fn call_frame(t: f64, keys: u32, mx: f32, my: f32, btn: u32) -> Result<(), String> {
        // Reset draw buffer so shapes from previous frames don't accumulate
        DRAW.with(|d| d.borrow_mut().reset());

        INTERP.with(|interp| {
            interp.enter(|vm| -> Result<(), String> {
                // Read from PYTHON_SCOPE — no APP borrow needed here.
                let scope = PYTHON_SCOPE.with(|ps| ps.borrow().clone());
                let scope = match scope {
                    Some(s) => s,
                    None => return Err("__no_script__".to_string()),
                };

                // Inject per-frame globals
                scope.globals
                    .set_item("t", vm.new_pyobj(t), vm)
                    .map_err(|e| exception_to_string(vm, e))?;
                scope.globals
                    .set_item("mouse_x", vm.new_pyobj(mx as f64), vm)
                    .map_err(|e| exception_to_string(vm, e))?;
                scope.globals
                    .set_item("mouse_y", vm.new_pyobj(my as f64), vm)
                    .map_err(|e| exception_to_string(vm, e))?;
                scope.globals
                    .set_item("keys", vm.new_pyobj(keys as i64), vm)
                    .map_err(|e| format!("inject keys: {:?}", e))?;
                scope.globals
                    .set_item("mouse_btn", vm.new_pyobj(btn as i64), vm)
                    .map_err(|e| format!("inject mouse_btn: {:?}", e))?;

                // Inject contextmenu selection (consumed and reset to -1)
                let sel = CONTEXTMENU_SELECTED.with(|s| {
                    let v = *s.borrow();
                    *s.borrow_mut() = -1;
                    v
                });
                scope.globals
                    .set_item("_contextmenu_selected", vm.new_pyobj(sel as i64), vm)
                    .map_err(|e| format!("inject _contextmenu_selected: {:?}", e))?;

                // Look up frame function and call it
                let frame_fn = scope.globals
                    .get_item("frame", vm)
                    .map_err(|_| "frame() function not defined".to_string())?;

                frame_fn
                    .call((t,), vm)
                    .map_err(|e| exception_to_string(vm, e))?;

                // Call engine after-frame hook (cursor + contextmenu)
                if let Ok(after_fn) = scope.globals.get_item("_bruecke_after", vm) {
                    after_fn
                        .call((), vm)
                        .map_err(|e| exception_to_string(vm, e))?;
                }

                Ok(())
            })
        })
    }
}
