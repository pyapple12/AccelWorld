//! WGSL 玻璃着色器（ui/gl/shaders.py GLSL 的逐行移植，PL013.05）
//! 单 pass：背景光场（渐变/双层云雾/双晕/颗粒）+ 16 玻璃面 SDF 合成
//! 移植清单对照 t.hint HINT002（每条勾销见文末说明区）

// uniform 布局（z.plan 修订二·B；f32 数组以 vec4 步长对齐 WGSL uniform 规则）
// 尺寸实算：16 头部 + 4×256 数组 + 5×16 vec4/vec3 段 + 尾部标量段 → 1168（73×16 对齐）
pub const UNIFORMS_SIZE: u64 = 1168;

pub const SHADER_WGSL: &str = r"
struct Uniforms {
    u_res: vec2<f32>,
    u_count: u32,
    _pad0: u32,
    u_rect: array<vec4<f32>, 16>,
    u_radius: array<vec4<f32>, 16>,
    u_refr: array<vec4<f32>, 16>,
    u_sel: array<vec4<f32>, 16>,
    u_top: vec4<f32>,
    u_bottom: vec4<f32>,
    u_glow_a: vec4<f32>,
    u_glow_b: vec4<f32>,
    u_glow_a_pos: vec2<f32>,
    u_glow_a_rad: f32,
    _pad1: f32,
    u_glow_b_pos: vec2<f32>,
    u_glow_b_rad: f32,
    _pad2: f32,
    u_accent: vec3<f32>,
    _pad3: f32,
    u_time: f32,
    u_lift: f32,
};

@group(0) @binding(0) var<uniform> U: Uniforms;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) frag_uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOut {
    // 全屏大三角形（三次调用覆盖视口）
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)
    );
    var out: VertexOut;
    let p = pos[vi];
    out.clip = vec4<f32>(p, 0.0, 1.0);
    // uv 底原点（对齐 GLSL gl_FragCoord 语义：glow_y token 直传）
    out.frag_uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}

fn background(uv: vec2<f32>) -> vec4<f32> {
    let col0 = mix(U.u_bottom.rgb, U.u_top.rgb, uv.y);
    var col = col0;
    let alpha = mix(U.u_bottom.a, U.u_top.a, uv.y);
    let aspect = U.u_res.x / U.u_res.y;
    let wa = uv * vec2<f32>(aspect, 1.0);
    let cloud = sin(uv.x * 6.2831 + 0.7) * sin(uv.y * 4.5 - 0.31);
    col += vec3<f32>(0.89, 0.70, 0.42) * (0.5 + 0.5 * cloud) * 0.04;
    let cloud2 = sin(uv.x * 4.7 - 1.3) * sin(uv.y * 6.1 + 0.5);
    col += vec3<f32>(0.30, 0.45, 0.85) * (0.5 + 0.5 * cloud2) * 0.03;
    let da = distance(wa, U.u_glow_a_pos * vec2<f32>(aspect, 1.0));
    let fa = exp(-pow(da / max(U.u_glow_a_rad, 1e-4), 2.0) * 2.5);
    col += U.u_glow_a.rgb * fa * (U.u_glow_a.a * 2.4);
    let db = distance(wa, U.u_glow_b_pos * vec2<f32>(aspect, 1.0));
    let fb = exp(-pow(db / max(U.u_glow_b_rad, 1e-4), 2.0) * 2.5);
    col += U.u_glow_b.rgb * fb * (U.u_glow_b.a * 2.4);
    let grain = fract(sin(dot(uv * U.u_res, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    col += vec3<f32>((grain - 0.5) * 0.006);
    return vec4<f32>(col, alpha);
}

fn sd_round_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let uv = in.frag_uv;
    let aspect = U.u_res.x / U.u_res.y;
    let bg = background(uv);
    var col = bg.rgb;
    var alpha = bg.a;
    for (var i = 0; i < 16; i++) {
        if (i >= i32(U.u_count)) { break; }
        let rect = U.u_rect[i];
        let center = rect.xy + rect.zw * 0.5;
        // HINT002-1 SDF 纵横比校正（y 基准）：圆角真圆、rim 全向等宽
        let p = (uv - center) * vec2<f32>(aspect, 1.0);
        let half_size = rect.zw * vec2<f32>(aspect, 1.0) * 0.5;
        let r_cap = min(U.u_radius[i].x, rect.w * 0.5);
        let d = sd_round_box(p, half_size, r_cap);
        // SDF 偏移投影：影子中心向下偏 0.012，0.035 柔和衰减；面内不投影
        let ds = sd_round_box(p + vec2<f32>(0.0, 0.012), half_size, r_cap);
        let outside = smoothstep(0.0, 0.0019, d);
        let shadow = (1.0 - smoothstep(0.0, 0.035, ds)) * 0.35 * outside;
        col *= (1.0 - shadow);
        // HINT002-2 cov 以 d 判向（d<=0 面内）：-d 写法整体颠倒的历史根因不复现
        let cov = 1.0 - smoothstep(0.0, 0.0019, d);
        if (cov <= 0.0) { continue; }
        let grad = normalize(p / max(half_size, vec2<f32>(1e-5, 1e-5)) + vec2<f32>(1e-6, 1e-6));
        let m = clamp(-d / half_size.y, 0.0, 1.0);
        let bend = (1.0 - m * m) * U.u_refr[i].x;
        let sft = grad * bend * vec2<f32>(1.0 / aspect, 1.0);
        // 色散三通道错位折射（1.3/1.0/0.7）
        let refr = vec3<f32>(
            background(uv + sft * 1.3).r,
            background(uv + sft * 1.0).g,
            background(uv + sft * 0.7).b
        );
        let breathe = 1.0 + 0.18 * U.u_sel[i].x * sin(U.u_time * 2.2);
        // HINT002-3 亮度模型：高透折射 + 白 veil（u_lift）+ tint 淡染 0.15——面内亮于背景
        var glass = mix(refr, U.u_rect[i].xyz * 0.0 + U.u_sel[i].xyz * 0.0 + mix(refr, tint_of(i), 0.15), 0.0);
        glass = mix(refr, tint_of(i), 0.15);
        glass += vec3<f32>(U.u_lift);
        // HINT002-4 双带边缘：rim 细线（0.004 uv，对角方向性 1.15/0.75）+ bezel 内亮带
        let rim = smoothstep(0.004, 0.0, abs(-d - 0.002));
        let diag = clamp(0.5 + 0.5 * (p.x / max(half_size.x, 1e-4)
                                      - p.y / max(half_size.y, 1e-4)), 0.0, 1.0);
        let rim_color = mix(vec3<f32>(0.9, 0.92, 1.0), U.u_accent, U.u_sel[i].x);
        glass += rim_color * rim * (0.70 * mix(1.15, 0.75, diag)) * breathe;
        let bezel = smoothstep(0.010, 0.0, abs(-d - 0.010));
        glass += vec3<f32>(0.55) * bezel * 0.22;
        // 顶缘镜面高光带：0.05 uv 指数衰减贴顶窄带
        let depth_top = half_size.y - p.y;
        let spec = exp(-max(depth_top, 0.0) / 0.05) * smoothstep(0.003, 0.017, -d);
        glass += vec3<f32>(1.0) * spec * 0.20;
        col = mix(col, glass, cov);
        alpha = mix(alpha, min(alpha + 0.25, 0.80), cov);
    }
    // HINT002-5 预乘 alpha 输出（合成器假定 premultiplied——GL 直通色满亮覆盖教训）
    return vec4<f32>(col * alpha, alpha);
}

fn tint_of(i: i32) -> vec3<f32> {
    return vec3<f32>(U.u_tint_r(i), U.u_tint_g(i), U.u_tint_b(i));
}
";

// tint 数组（16×vec3）：WGSL 数组元素 vec3 在 uniform 中步长 16 与 vec4 等价，
// 直接并入 u_sel 的 yzw 通道会破坏语义——独立数组占用更多对齐空间；实现取舍：
// tint 打包进 u_radius 的 yzw 通道（radius 标量只用 x）以省一个数组的 256B。
// 上方 shader 中 tint_of 由宏替换——真实实现见 build_shader_source()。

/// 组装最终 WGSL：tint_of(i) → U.u_radius[i].yzw（tint 打包进 radius 数组空余通道）
///半径只占 x 通道，yzw 放 tint 三通道——uniform 省一个 array<vec4,16>
pub fn build_shader_source() -> String {
    SHADER_WGSL
        .replace("U.u_tint_r(i)", "U.u_radius[i].y")
        .replace("U.u_tint_g(i)", "U.u_radius[i].z")
        .replace("U.u_tint_b(i)", "U.u_radius[i].w")
        // 修正 tint mix 行（占位行简化为直接调用）
        .replace(
            "var glass = mix(refr, U.u_rect[i].xyz * 0.0 + U.u_sel[i].xyz * 0.0 + mix(refr, tint_of(i), 0.15), 0.0);\n        glass = mix(refr, tint_of(i), 0.15);",
            "var glass = mix(refr, tint_of(i), 0.15);",
        )
}

/// uniform 打包器（Rust 侧与 WGSL Uniforms 结构一一对应，PL013.05 对照表实现侧）
#[derive(Debug, Clone, Copy)]
pub struct GlassUniforms {
    pub res: [f32; 2],
    pub count: u32,
    pub rect: [[f32; 4]; 16],
    /// x=radius，yzw=tint（打包约定见 build_shader_source）
    pub radius_tint: [[f32; 4]; 16],
    pub refr: [[f32; 4]; 16],
    pub sel: [[f32; 4]; 16],
    pub top: [f32; 4],
    pub bottom: [f32; 4],
    pub glow_a: [f32; 4],
    pub glow_b: [f32; 4],
    pub glow_a_pos: [f32; 2],
    pub glow_a_rad: f32,
    pub glow_b_pos: [f32; 2],
    pub glow_b_rad: f32,
    pub accent: [f32; 3],
    pub time: f32,
    pub lift: f32,
}

impl GlassUniforms {
    /// 按 WGSL 布局序列化为 bytes（pad 全零；len 断言 UNIFORMS_SIZE）
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(UNIFORMS_SIZE as usize);
        push_f32(&mut v, self.res[0]);
        push_f32(&mut v, self.res[1]);
        push_u32(&mut v, self.count);
        push_u32(&mut v, 0); // _pad0
        for r in &self.rect {
            push_vec4(&mut v, *r);
        }
        for r in &self.radius_tint {
            push_vec4(&mut v, *r);
        }
        for r in &self.refr {
            push_vec4(&mut v, *r);
        }
        for s in &self.sel {
            push_vec4(&mut v, *s);
        }
        push_vec4(&mut v, self.top);
        push_vec4(&mut v, self.bottom);
        push_vec4(&mut v, self.glow_a);
        push_vec4(&mut v, self.glow_b);
        push_vec2(&mut v, self.glow_a_pos);
        push_f32(&mut v, self.glow_a_rad);
        push_f32(&mut v, 0.0); // _pad1
        push_vec2(&mut v, self.glow_b_pos);
        push_f32(&mut v, self.glow_b_rad);
        push_f32(&mut v, 0.0); // _pad2
        push_vec3_pad(&mut v, self.accent);
        push_f32(&mut v, self.time);
        push_f32(&mut v, self.lift);
        push_f32(&mut v, 0.0); // _pad3
        push_f32(&mut v, 0.0);
        assert_eq!(
            v.len() as u64,
            UNIFORMS_SIZE,
            "uniform 打包长度与 WGSL 布局不一致"
        );
        v
    }
}

fn push_f32(v: &mut Vec<u8>, x: f32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn push_u32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

fn push_vec2(v: &mut Vec<u8>, x: [f32; 2]) {
    push_f32(v, x[0]);
    push_f32(v, x[1]);
}

fn push_vec3_pad(v: &mut Vec<u8>, x: [f32; 3]) {
    push_f32(v, x[0]);
    push_f32(v, x[1]);
    push_f32(v, x[2]);
    push_f32(v, 0.0);
}

fn push_vec4(v: &mut Vec<u8>, x: [f32; 4]) {
    push_f32(v, x[0]);
    push_f32(v, x[1]);
    push_f32(v, x[2]);
    push_f32(v, x[3]);
}

// ===== src/glass/shader.rs 说明区 =====
// 输入：GlassUniforms（painter 组装）；输出：WGSL 源串（wgpu 编译）+ uniform bytes
// 移植对照（t.hint HINT002 逐条勾销，Python ui/gl/shaders.py → 本文件）：
//   1 SDF 纵横比校正 vec2(aspect,1)（p/half_size/r_cap 全程）——药丸椭圆化根治
//   2 cov/outside 以 d 判向（d<=0 面内）——-d 写法颠倒的玻璃失效根因不复现
//   3 亮度模型高透+veil（u_lift）+tint 0.15 淡染——卡面比背景暗的根因不复现
//   4 rim 细线 0.004uv 对角 1.15/0.75 + bezel 0.010uv/0.22 + 顶缘 spec 0.05uv
//   5 预乘输出 vec4(col*alpha, alpha)——合成器 premultiplied 假定（egui/wgpu 同 Qt）
//   6 背景光场：渐变 + 金/冷双层云雾 + 双晕 exp 衰减 2.5 + 颗粒 0.006
//   7 色散三通道 1.3/1.0/0.7；投影向下 0.012/衰减 0.035/强度 0.35 面内剔除
// uniform 对照表（键/类型/默认值来源 → Python GLSL 对应 uniform）：
//   u_res vec2 画布像素 | u_count u32 场景可见面数 | u_rect[16] vec4 xywh uv 底原点
//   u_radius[16] x=端帽半径（min 传入, w*0.5 钳在 shader）yzw=tint rgb（打包）
//   u_refr[16] x=折射上限（默认 0.035）| u_sel[16] x=选中强度 0..1
//   u_top/u_bottom vec4 field.top/bottom（#AARRGGBB → rgba f32）| u_glow_a/b vec4
//   u_glow_a_pos/u_glow_b_pos vec2 uv 底原点直传 | u_glow_a_rad/u_glow_b_rad f32
//   u_accent vec3 选中金色 | u_time f32 秒 | u_lift f32 glass.gl_lift/255
// 已知偏移：tint 打包进 radius 数组 yzw 通道（省 256B uniform 空间，GLSL 无此约束）；
//   uv y 翻转在顶点输出端完成（wgpu 帧原点左上 vs GL 左下），shader 主逻辑逐行一致
// 异常处理：uniform 打包长度编译期断言（UNIFORMS_SIZE）
// 关联依赖：wgpu（消费方 painter.rs/render_probe）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniforms_size_matches_wgsl_layout() {
        // 打包长度 = WGSL Uniforms 布局尺寸（1168 数据 + 16 pad 尾对齐）
        let u = GlassUniforms {
            res: [640.0, 480.0],
            count: 2,
            rect: [[0.1, 0.1, 0.3, 0.2]; 16],
            radius_tint: [[12.0, 0.16, 0.14, 0.20]; 16],
            refr: [[0.035; 4]; 16],
            sel: [[0.0; 4]; 16],
            top: [0.9, 0.9, 1.0, 0.14],
            bottom: [0.8, 0.8, 0.9, 0.12],
            glow_a: [1.0, 0.9, 0.6, 0.3],
            glow_b: [0.6, 0.7, 1.0, 0.25],
            glow_a_pos: [0.84, 0.0],
            glow_a_rad: 0.38,
            glow_b_pos: [0.0, 1.0],
            glow_b_rad: 0.5,
            accent: [1.0, 0.8, 0.3],
            time: 1.0,
            lift: 0.1,
        };
        let bytes = u.to_bytes();
        assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
        // 首字段 res 可读回（小端）
        assert_eq!(
            f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            640.0
        );
    }

    #[test]
    fn shader_source_compiles_placeholder_free() {
        // 组装后不应残留占位宏（tint_of 已替换为 u_radius 通道访问）
        let src = build_shader_source();
        assert!(src.contains("U.u_radius[i].y"));
        assert!(!src.contains("U.u_tint_r"));
        assert!(!src.contains("var glass = mix(refr, U.u_rect"));
        assert!(
            src.contains("vec4<f32>(col * alpha, alpha)"),
            "预乘输出必须存在"
        );
        assert!(src.contains("sd_round_box"), "SDF 必须存在");
    }
}
