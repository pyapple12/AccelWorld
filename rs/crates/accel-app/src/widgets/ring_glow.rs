//! 金圈 GPU 光晕（PL020）：一次 PaintCallback + GLSL，SDF 距离场画指数衰减 bloom。
//! 不叠多层 stroke；GL 资源懒初始化，编译/链接失败自动降级（调用方保留 CPU 实环）。

use std::sync::{Arc, Mutex, OnceLock};

use eframe::egui;
use eframe::egui_glow;
use eframe::glow;
use eframe::glow::HasContext;

const VS: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
out vec2 v_uv;
void main() {
    v_uv = a_pos * 0.5 + 0.5;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const FS: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 frag_color;
uniform vec2 u_center;
uniform vec2 u_size;
uniform vec2 u_half;
uniform float u_radius;
uniform float u_thickness;
uniform vec3 u_color;
uniform float u_bloom;
uniform float u_alpha;
uniform float u_inset;
uniform int u_mode;

float sd_rounded_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

void main() {
    vec2 p = (v_uv - u_center) * u_size;
    float d;
    if (u_mode == 0) {
        // 只在外侧画光晕；环内（表盘内部）alpha 恒 0
        d = length(p) - (u_radius + u_thickness * 0.5);
    } else if (u_mode == 1) {
        // 向胶囊内部扩 2px：填充边缘抗锯齿半透明时，底下透出的是光晕色而不是背景
        d = sd_rounded_box(p, u_half, u_radius) + 2.0;
    } else if (u_mode == 2) {
        // 圆点光斑（数字底光）：从中心直接指数衰减，无硬核、无圆边
        d = length(p);
    } else {
        // 矩形整体背光（抽屉）：内扩 u_inset 压住边框与光晕之间的亚像素缝
        d = sd_rounded_box(p, u_half, u_radius) + u_inset;
    }
    float a = 0.0;
    if (d > 0.0) {
        if (u_mode == 3) {
            float t = clamp(d / u_bloom, 0.0, 1.0);
            a = (1.0 - t) * (1.0 - t);
        } else {
            a = exp(-d / u_bloom) * 0.85;
        }
    }
    // rect 边缘淡出，避免光晕在矩形边界突变成 0 形成可见方框
    // mode 3 已在 bloom 处平方归零，不能再乘 edge（大矩形会把它压到肉眼不可见）
    float edge = 1.0;
    if (u_mode == 2) {
        edge = 1.0 - smoothstep(0.12, 0.5, max(abs(v_uv.x - 0.5), abs(v_uv.y - 0.5)));
    } else if (u_mode != 3) {
        edge = 1.0 - smoothstep(0.30, 0.5, max(abs(v_uv.x - 0.5), abs(v_uv.y - 0.5)));
    }
    a *= edge * u_alpha;
    frag_color = vec4(u_color * a, a);
}
"#;

struct RingGl {
    program: glow::Program,
    vao: glow::VertexArray,
    #[allow(dead_code)]
    vbo: glow::Buffer,
    u_center: Option<glow::UniformLocation>,
    u_size: Option<glow::UniformLocation>,
    u_half: Option<glow::UniformLocation>,
    u_radius: Option<glow::UniformLocation>,
    u_thickness: Option<glow::UniformLocation>,
    u_color: Option<glow::UniformLocation>,
    u_bloom: Option<glow::UniformLocation>,
    u_alpha: Option<glow::UniformLocation>,
    u_inset: Option<glow::UniformLocation>,
    u_mode: Option<glow::UniformLocation>,
}

enum RingState {
    Ready(RingGl),
    Failed,
}

static RING: OnceLock<Mutex<Option<RingState>>> = OnceLock::new();

fn ring_state() -> &'static Mutex<Option<RingState>> {
    RING.get_or_init(|| Mutex::new(None))
}

impl RingGl {
    /// 编译并链接 shader、建 quad VAO；任何一步失败返回 None（调用方保留 CPU 实环）
    fn new(gl: &glow::Context) -> Option<Self> {
        unsafe {
            let program = gl.create_program().ok()?;
            let vs = gl.create_shader(glow::VERTEX_SHADER).ok()?;
            gl.shader_source(vs, VS);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                tracing::warn!("金圈光晕 VS 编译失败: {}", gl.get_shader_info_log(vs));
                gl.delete_shader(vs);
                gl.delete_program(program);
                return None;
            }
            let fs = gl.create_shader(glow::FRAGMENT_SHADER).ok()?;
            gl.shader_source(fs, FS);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                tracing::warn!("金圈光晕 FS 编译失败: {}", gl.get_shader_info_log(fs));
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(program);
                return None;
            }
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                tracing::warn!("金圈光晕链接失败: {}", gl.get_program_info_log(program));
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(program);
                return None;
            }
            gl.detach_shader(program, vs);
            gl.detach_shader(program, fs);
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            let vao = gl.create_vertex_array().ok()?;
            let vbo = gl.create_buffer().ok()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let verts: [f32; 8] = [-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
            let bytes = std::slice::from_raw_parts(
                verts.as_ptr().cast::<u8>(),
                std::mem::size_of_val(&verts),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            let loc = |name: &str| gl.get_uniform_location(program, name);
            Some(Self {
                program,
                vao,
                vbo,
                u_center: loc("u_center"),
                u_size: loc("u_size"),
                u_half: loc("u_half"),
                u_radius: loc("u_radius"),
                u_thickness: loc("u_thickness"),
                u_color: loc("u_color"),
                u_bloom: loc("u_bloom"),
                u_alpha: loc("u_alpha"),
                u_inset: loc("u_inset"),
                u_mode: loc("u_mode"),
            })
        }
    }
}

/// 在表盘位置排一次 GPU bloom（实环由调用方 CPU 绘制，GPU 只负责向外/向内的光晕）
pub fn paint(
    ui: &egui::Ui,
    center: egui::Pos2,
    radius: f32,
    thickness: f32,
    color: egui::Color32,
    bloom: f32,
) {
    let ppp = ui.ctx().pixels_per_point();
    let pad = bloom * 3.0 + 8.0;
    let half = radius + thickness * 0.5 + pad;
    let rect = egui::Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0));
    let center_norm = egui::pos2(
        (center.x - rect.left()) / rect.width(),
        1.0 - (center.y - rect.top()) / rect.height(),
    );
    let size_px = egui::vec2(rect.width() * ppp, rect.height() * ppp);
    let radius_px = radius * ppp;
    let thickness_px = (thickness * ppp).max(1.0);
    let bloom_px = (bloom * ppp).max(1.0);
    let color_f = [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
    ];
    let cb = egui_glow::CallbackFn::new(move |_info, painter| {
        let gl = painter.gl();
        let mut guard = ring_state().lock().expect("金圈 GL 锁中毒");
        if guard.is_none() {
            *guard = Some(match RingGl::new(gl) {
                Some(r) => RingState::Ready(r),
                None => RingState::Failed,
            });
        }
        if let Some(RingState::Ready(r)) = guard.as_ref() {
            unsafe {
                gl.use_program(Some(r.program));
                gl.bind_vertex_array(Some(r.vao));
                gl.uniform_2_f32(r.u_center.as_ref(), center_norm.x, center_norm.y);
                gl.uniform_2_f32(r.u_size.as_ref(), size_px.x, size_px.y);
                gl.uniform_2_f32(r.u_half.as_ref(), 0.0, 0.0);
                gl.uniform_1_f32(r.u_radius.as_ref(), radius_px);
                gl.uniform_1_f32(r.u_thickness.as_ref(), thickness_px);
                gl.uniform_3_f32(r.u_color.as_ref(), color_f[0], color_f[1], color_f[2]);
                gl.uniform_1_f32(r.u_bloom.as_ref(), bloom_px);
                gl.uniform_1_f32(r.u_alpha.as_ref(), 1.0);
                gl.uniform_1_f32(r.u_inset.as_ref(), 0.0);
                gl.uniform_1_i32(r.u_mode.as_ref(), 0);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                gl.bind_vertex_array(None);
            }
        }
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });
}

/// 圆点底光（四正位数字）：一次 GPU 径向光斑
pub fn paint_dot(ui: &egui::Ui, center: egui::Pos2, radius: f32, color: egui::Color32, bloom: f32) {
    let ppp = ui.ctx().pixels_per_point();
    let pad = bloom * 8.0 + 8.0;
    let half = radius + pad;
    let rect = egui::Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0));
    let center_norm = egui::pos2(
        (center.x - rect.left()) / rect.width(),
        1.0 - (center.y - rect.top()) / rect.height(),
    );
    let size_px = egui::vec2(rect.width() * ppp, rect.height() * ppp);
    let radius_px = (radius * ppp).max(1.0);
    let bloom_px = (bloom * ppp).max(1.0);
    let color_f = [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
    ];
    let cb = egui_glow::CallbackFn::new(move |_info, painter| {
        let gl = painter.gl();
        let mut guard = ring_state().lock().expect("金圈 GL 锁中毒");
        if guard.is_none() {
            *guard = Some(match RingGl::new(gl) {
                Some(r) => RingState::Ready(r),
                None => RingState::Failed,
            });
        }
        if let Some(RingState::Ready(r)) = guard.as_ref() {
            unsafe {
                gl.use_program(Some(r.program));
                gl.bind_vertex_array(Some(r.vao));
                gl.uniform_2_f32(r.u_center.as_ref(), center_norm.x, center_norm.y);
                gl.uniform_2_f32(r.u_size.as_ref(), size_px.x, size_px.y);
                gl.uniform_2_f32(r.u_half.as_ref(), 0.0, 0.0);
                gl.uniform_1_f32(r.u_radius.as_ref(), radius_px);
                gl.uniform_1_f32(r.u_thickness.as_ref(), 1.0);
                gl.uniform_3_f32(r.u_color.as_ref(), color_f[0], color_f[1], color_f[2]);
                gl.uniform_1_f32(r.u_bloom.as_ref(), bloom_px);
                gl.uniform_1_f32(r.u_alpha.as_ref(), 1.0);
                gl.uniform_1_f32(r.u_inset.as_ref(), 0.0);
                gl.uniform_1_i32(r.u_mode.as_ref(), 2);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                gl.bind_vertex_array(None);
            }
        }
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });
}
pub fn paint_pill(
    ui: &egui::Ui,
    center: egui::Pos2,
    half_size: egui::Vec2,
    radius: f32,
    color: egui::Color32,
    bloom: f32,
) {
    let ppp = ui.ctx().pixels_per_point();
    // 大 pad：让 rect 边界处的 alpha 低到不可见，光晕不被方形裁切
    let pad = bloom * 3.0 + 8.0;
    let half = half_size + egui::vec2(pad, pad);
    let full = egui::Rect::from_center_size(center, half * 2.0);
    // 回调 rect 夹到可见区：越界时 viewport 与 SDF 坐标错位会把透明内部写成黑胶囊
    let rect = full.intersect(ui.painter().clip_rect());
    if !rect.is_positive() {
        return;
    }
    let center_norm = egui::pos2(
        (center.x - rect.left()) / rect.width(),
        1.0 - (center.y - rect.top()) / rect.height(),
    );
    let size_px = egui::vec2(rect.width() * ppp, rect.height() * ppp);
    let half_px = egui::vec2(half_size.x * ppp, half_size.y * ppp);
    let radius_px = (radius * ppp).max(1.0);
    let bloom_px = (bloom * ppp).max(1.0);
    let color_f = [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
    ];
    let cb = egui_glow::CallbackFn::new(move |_info, painter| {
        let gl = painter.gl();
        let mut guard = ring_state().lock().expect("金圈 GL 锁中毒");
        if guard.is_none() {
            *guard = Some(match RingGl::new(gl) {
                Some(r) => RingState::Ready(r),
                None => RingState::Failed,
            });
        }
        if let Some(RingState::Ready(r)) = guard.as_ref() {
            unsafe {
                gl.use_program(Some(r.program));
                gl.bind_vertex_array(Some(r.vao));
                gl.uniform_2_f32(r.u_center.as_ref(), center_norm.x, center_norm.y);
                gl.uniform_2_f32(r.u_size.as_ref(), size_px.x, size_px.y);
                gl.uniform_2_f32(r.u_half.as_ref(), half_px.x, half_px.y);
                gl.uniform_1_f32(r.u_radius.as_ref(), radius_px);
                gl.uniform_1_f32(r.u_thickness.as_ref(), 1.0);
                gl.uniform_3_f32(r.u_color.as_ref(), color_f[0], color_f[1], color_f[2]);
                gl.uniform_1_f32(r.u_bloom.as_ref(), bloom_px);
                gl.uniform_1_f32(r.u_alpha.as_ref(), 1.0);
                gl.uniform_1_f32(r.u_inset.as_ref(), 0.0);
                gl.uniform_1_i32(r.u_mode.as_ref(), 1);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                gl.bind_vertex_array(None);
            }
        }
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });
}

/// 矩形整体背光（抽屉）：只向外侧扩散，bloom 处平方衰减归零
pub fn paint_rect_glow(
    painter: &egui::Painter,
    bounds: egui::Rect,
    radius: f32,
    color: egui::Color32,
    bloom: f32,
    alpha: f32,
) {
    let ppp = painter.ctx().pixels_per_point();
    // 大 pad：让绘制区边界处的 alpha 低到不可见，光晕不被方形裁切
    let pad = bloom * 3.0 + 8.0;
    let full = bounds.expand(pad);
    let rect = full.intersect(painter.clip_rect());
    if !rect.is_positive() {
        return;
    }
    let center = bounds.center();
    let center_norm = egui::pos2(
        (center.x - rect.left()) / rect.width(),
        1.0 - (center.y - rect.top()) / rect.height(),
    );
    let size_px = egui::vec2(rect.width() * ppp, rect.height() * ppp);
    let half_px = egui::vec2(bounds.width() * 0.5 * ppp, bounds.height() * 0.5 * ppp);
    let radius_px = (radius * ppp).max(0.0);
    let bloom_px = (bloom * ppp).max(1.0);
    let color_f = [
        color.r() as f32 / 255.0,
        color.g() as f32 / 255.0,
        color.b() as f32 / 255.0,
    ];
    let cb = egui_glow::CallbackFn::new(move |_info, painter| {
        let gl = painter.gl();
        let mut guard = ring_state().lock().expect("金圈 GL 锁中毒");
        if guard.is_none() {
            *guard = Some(match RingGl::new(gl) {
                Some(r) => RingState::Ready(r),
                None => RingState::Failed,
            });
        }
        if let Some(RingState::Ready(r)) = guard.as_ref() {
            unsafe {
                gl.use_program(Some(r.program));
                gl.bind_vertex_array(Some(r.vao));
                gl.uniform_2_f32(r.u_center.as_ref(), center_norm.x, center_norm.y);
                gl.uniform_2_f32(r.u_size.as_ref(), size_px.x, size_px.y);
                gl.uniform_2_f32(r.u_half.as_ref(), half_px.x, half_px.y);
                gl.uniform_1_f32(r.u_radius.as_ref(), radius_px);
                gl.uniform_1_f32(r.u_thickness.as_ref(), 1.0);
                gl.uniform_3_f32(r.u_color.as_ref(), color_f[0], color_f[1], color_f[2]);
                gl.uniform_1_f32(r.u_bloom.as_ref(), bloom_px);
                gl.uniform_1_f32(r.u_alpha.as_ref(), alpha);
                gl.uniform_1_f32(r.u_inset.as_ref(), ppp);
                gl.uniform_1_i32(r.u_mode.as_ref(), 3);
                gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
                gl.bind_vertex_array(None);
            }
        }
    });
    painter.add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });
}

// ===== 刻度 GPU 光晕（线段 SDF，一次 draw）=====

const TICKS_VS: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_p1;
layout(location = 2) in vec2 a_p2;
layout(location = 3) in float a_half;
layout(location = 4) in vec4 a_color;
uniform vec2 u_origin;
uniform vec2 u_size;
out vec2 v_p;
out vec2 v_p1;
out vec2 v_p2;
out float v_half;
out vec4 v_color;
out vec2 v_uv;
void main() {
    v_p = a_pos;
    v_p1 = a_p1;
    v_p2 = a_p2;
    v_half = a_half;
    v_color = a_color;
    vec2 uv = (a_pos - u_origin) / u_size;
    v_uv = uv;
    gl_Position = vec4(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
}
"#;

const TICKS_FS: &str = r#"#version 330 core
in vec2 v_p;
in vec2 v_p1;
in vec2 v_p2;
in float v_half;
in vec4 v_color;
in vec2 v_uv;
out vec4 frag_color;
float sd_segment_round(vec2 p, vec2 a, vec2 b) {
    vec2 pa = p - a;
    vec2 ba = b - a;
    float h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
    return length(pa - ba * h);
}
float sd_segment_flat(vec2 p, vec2 a, vec2 b, float half_w) {
    vec2 ba = b - a;
    float len = max(length(ba), 1e-6);
    vec2 dir = ba / len;
    vec2 n = vec2(-dir.y, dir.x);
    vec2 pa = p - a;
    float along = dot(pa, dir);
    float across = abs(dot(pa, n));
    float caps = max(max(-along, along - len), 0.0);
    return max(across - half_w, caps);
}
void main() {
    // 实线由 CPU 画（亮绿、平头）；GPU 只画圆头光晕
    float round = sd_segment_round(v_p, v_p1, v_p2) - v_half;
    float a = round > 0.0 ? exp(-round / 1.6) * 0.38 : 0.0;
    // rect 边缘淡出，避免 quad 边界截断光晕形成方框
    float edge = 1.0 - smoothstep(0.30, 0.5, max(abs(v_uv.x - 0.5), abs(v_uv.y - 0.5)));
    a *= edge;
    frag_color = vec4(v_color.rgb * a, a * v_color.a);
}
"#;

struct TicksGl {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    u_origin: Option<glow::UniformLocation>,
    u_size: Option<glow::UniformLocation>,
}

enum TicksState {
    Ready(TicksGl),
    Failed,
}

static TICKS: OnceLock<Mutex<Option<TicksState>>> = OnceLock::new();

fn ticks_state() -> &'static Mutex<Option<TicksState>> {
    TICKS.get_or_init(|| Mutex::new(None))
}

impl TicksGl {
    fn new(gl: &glow::Context) -> Option<Self> {
        unsafe {
            let program = gl.create_program().ok()?;
            let vs = gl.create_shader(glow::VERTEX_SHADER).ok()?;
            gl.shader_source(vs, TICKS_VS);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                tracing::warn!("刻度光晕 VS 编译失败: {}", gl.get_shader_info_log(vs));
                gl.delete_shader(vs);
                gl.delete_program(program);
                return None;
            }
            let fs = gl.create_shader(glow::FRAGMENT_SHADER).ok()?;
            gl.shader_source(fs, TICKS_FS);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                tracing::warn!("刻度光晕 FS 编译失败: {}", gl.get_shader_info_log(fs));
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(program);
                return None;
            }
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                tracing::warn!("刻度光晕链接失败: {}", gl.get_program_info_log(program));
                gl.delete_shader(vs);
                gl.delete_shader(fs);
                gl.delete_program(program);
                return None;
            }
            gl.detach_shader(program, vs);
            gl.detach_shader(program, fs);
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            let vao = gl.create_vertex_array().ok()?;
            let vbo = gl.create_buffer().ok()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let stride = (11 * std::mem::size_of::<f32>()) as i32;
            for (loc, count, offset) in [
                (0u32, 2i32, 0i32),
                (1, 2, 2 * 4),
                (2, 2, 4 * 4),
                (3, 1, 6 * 4),
                (4, 4, 7 * 4),
            ] {
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, count, glow::FLOAT, false, stride, offset);
            }
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            let loc = |name: &str| gl.get_uniform_location(program, name);
            Some(Self {
                program,
                vao,
                vbo,
                u_origin: loc("u_origin"),
                u_size: loc("u_size"),
            })
        }
    }
}

/// 刻度光晕：所有刻度顶点一次上传、一次 draw（线段 SDF + exp 衰减）
pub fn paint_ticks(
    ui: &egui::Ui,
    center: egui::Pos2,
    radius: f32,
    ticks: &[(egui::Pos2, egui::Pos2, f32, egui::Color32)],
) {
    if ticks.is_empty() {
        return;
    }
    let pad = 10.0_f32;
    let half = radius + pad;
    let rect = egui::Rect::from_center_size(center, egui::vec2(half * 2.0, half * 2.0));
    let origin = [rect.min.x, rect.min.y];
    let size = [rect.width(), rect.height()];
    let mut verts: Vec<f32> = Vec::with_capacity(ticks.len() * 6 * 11);
    for &(p1, p2, half_w, color) in ticks {
        let dir = (p2 - p1).normalized();
        let n = egui::vec2(-dir.y, dir.x);
        let hp = half_w + pad;
        let a = p1 - dir * pad - n * hp;
        let b = p1 - dir * pad + n * hp;
        let c = p2 + dir * pad + n * hp;
        let d = p2 + dir * pad - n * hp;
        let cf = [
            color.r() as f32 / 255.0,
            color.g() as f32 / 255.0,
            color.b() as f32 / 255.0,
            color.a() as f32 / 255.0,
        ];
        for p in [a, b, c, a, c, d] {
            verts.extend_from_slice(&[
                p.x, p.y, p1.x, p1.y, p2.x, p2.y, half_w, cf[0], cf[1], cf[2], cf[3],
            ]);
        }
    }
    let count = (verts.len() / 11) as i32;
    let cb = egui_glow::CallbackFn::new(move |_info, painter| {
        let gl = painter.gl();
        let mut guard = ticks_state().lock().expect("刻度 GL 锁中毒");
        if guard.is_none() {
            *guard = Some(match TicksGl::new(gl) {
                Some(r) => TicksState::Ready(r),
                None => TicksState::Failed,
            });
        }
        if let Some(TicksState::Ready(r)) = guard.as_ref() {
            unsafe {
                gl.use_program(Some(r.program));
                gl.bind_vertex_array(Some(r.vao));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(r.vbo));
                let bytes = std::slice::from_raw_parts(
                    verts.as_ptr().cast::<u8>(),
                    std::mem::size_of_val(verts.as_slice()),
                );
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::DYNAMIC_DRAW);
                gl.uniform_2_f32(r.u_origin.as_ref(), origin[0], origin[1]);
                gl.uniform_2_f32(r.u_size.as_ref(), size[0], size[1]);
                // 夜光加色混合：叠在实线边缘只会提亮，不会压黑
                gl.enable(glow::BLEND);
                gl.blend_func(glow::ONE, glow::ONE);
                gl.draw_arrays(glow::TRIANGLES, 0, count);
                gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
                gl.bind_vertex_array(None);
            }
        }
    });
    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });
}

// ===== src/widgets/ring_glow.rs 说明区 =====
// 输入：Ui + 环心/半径/厚度/颜色/bloom；输出：一次 PaintCallback（GPU bloom）
// 逻辑步骤：rect 扩到光晕范围 → 归一化环心与像素尺寸 → CallbackFn 懒编译 shader →
//   设 uniform → draw_arrays(TRIANGLE_STRIP, 4)
// 设计理由（PL020）：用户要求真光晕而非叠圈；SDF + exp 衰减一次 fragment 出结果，
//   与 CPU 实环分工（GPU 只画 bloom，不重复实环）
// 异常处理：shader 编译/链接失败记 warn 并置 Failed，后续不再重试；实环仍由 CPU 画
// 关联依赖：eframe::egui_glow::CallbackFn、eframe::glow；消费方：pages::clock::draw_ring
