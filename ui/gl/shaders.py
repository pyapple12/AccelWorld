# 玻璃着色器源码模块（PL009 走查返工 2026-09-13 第二轮）：
# ①SDF 纵横比校正（药丸不圆根因）：uv 空间 x/y 像素密度不同（1 uv = W px vs H px），
#   距离场 p/half_size 乘 vec2(aspect,1) 换算到屏幕等比空间——圆角真圆、rim 全向等宽
# ②亮度模型反转（卡面比背面暗的根因）：暗色 tint 0.35 染色 + lift 8/255 提亮不足，
#   面内比透壁纸的背景还暗——改为 高透折射(0.80) + 白色 veil（ui.json glass.gl_lift
#   26/34）+ tint 淡染(0.15)：面内净亮于背景，参考图 Liquid Glass 同型
# ③rim 细化方向性：带宽 0.008→0.004 uv（~3px 细线）+ 对角加权（左上 1.15/右下 0.75，
#   受光方向感）+ 内侧次级亮带 bezel（双线玻璃厚度感，替代原内侧收暗）
# ④背景增彩：云雾双层错相（金/冷色）+ 双晕强度提升——给折射喂可弯折的色彩层次
# 其余：配色四组 vec4（field 自带 alpha）、glow pos/rad uniforms（直传语义）、
# u_sel[] 选中金描边呼吸、预乘 alpha 输出（Qt 合成假定 premultiplied）
# GLSL 版本：不带 #version（默认兼容 1.x 口径），最大化旧驱动兼容

_GLSL_VERT = """attribute vec2 a_pos;
void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"""

# 背景 + 玻璃合成单 pass（程序化光场：渐变/双层云雾/双晕/颗粒，避免 FBO 翻转复杂度）
_GLSL_FRAG = """precision highp float;
uniform vec2 u_res;
uniform int u_count;
uniform vec4 u_rect[16];
uniform float u_radius[16];
uniform float u_refr[16];
uniform vec3 u_tint[16];
uniform float u_sel[16];
uniform vec4 u_top;
uniform vec4 u_bottom;
uniform vec4 u_glow_a;
uniform vec4 u_glow_b;
uniform vec2 u_glow_a_pos;
uniform float u_glow_a_rad;
uniform vec2 u_glow_b_pos;
uniform float u_glow_b_rad;
uniform vec3 u_accent;
uniform float u_time;
uniform float u_lift;

vec4 background(vec2 uv) {
    vec3 col = mix(u_bottom.rgb, u_top.rgb, uv.y);
    float alpha = mix(u_bottom.a, u_top.a, uv.y);
    float aspect = u_res.x / u_res.y;
    vec2 wa = uv * vec2(aspect, 1.0);
    float cloud = sin(uv.x * 6.2831 + 0.7) * sin(uv.y * 4.5 - 0.31);
    col += vec3(0.89, 0.70, 0.42) * (0.5 + 0.5 * cloud) * 0.04;
    float cloud2 = sin(uv.x * 4.7 - 1.3) * sin(uv.y * 6.1 + 0.5);
    col += vec3(0.30, 0.45, 0.85) * (0.5 + 0.5 * cloud2) * 0.03;
    float da = distance(wa, u_glow_a_pos * vec2(aspect, 1.0));
    float fa = exp(-pow(da / max(u_glow_a_rad, 1e-4), 2.0) * 2.5);
    col += u_glow_a.rgb * fa * (u_glow_a.a * 2.4);
    float db = distance(wa, u_glow_b_pos * vec2(aspect, 1.0));
    float fb = exp(-pow(db / max(u_glow_b_rad, 1e-4), 2.0) * 2.5);
    col += u_glow_b.rgb * fb * (u_glow_b.a * 2.4);
    float grain = fract(sin(dot(uv * u_res, vec2(12.9898, 78.233))) * 43758.5453);
    col += (grain - 0.5) * 0.006;
    return vec4(col, alpha);
}

float sd_round_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

void main() {
    vec2 uv = gl_FragCoord.xy / u_res;
    float aspect = u_res.x / u_res.y;
    vec4 bg = background(uv);
    vec3 col = bg.rgb;
    float alpha = bg.a;
    for (int i = 0; i < 16; i++) {
        if (i >= u_count) { break; }
        vec2 center = u_rect[i].xy + u_rect[i].zw * 0.5;
        // SDF 纵横比校正（y 基准）：距离场在屏幕等比空间计算——圆角真圆
        vec2 p = (uv - center) * vec2(aspect, 1.0);
        vec2 half_size = u_rect[i].zw * vec2(aspect, 1.0) * 0.5;
        float r_cap = min(u_radius[i], u_rect[i].w * 0.5);
        float d = sd_round_box(p, half_size, r_cap);
        // SDF 偏移投影：影子中心向下偏 0.012，0.035 柔和衰减；面内不投影
        float ds = sd_round_box(p + vec2(0.0, 0.012), half_size, r_cap);
        float outside = smoothstep(0.0, 0.0019, d);
        float shadow = (1.0 - smoothstep(0.0, 0.035, ds)) * 0.35 * outside;
        col *= (1.0 - shadow);
        // cov 以 d 判向：d<=-1.2px 面内 → 1，面外 → 0
        float cov = 1.0 - smoothstep(0.0, 0.0019, d);
        if (cov <= 0.0) { continue; }
        vec2 grad = normalize(p / max(half_size, 1e-5) + 1e-6);
        float m = clamp(-d / half_size.y, 0.0, 1.0);
        float bend = (1.0 - m * m) * u_refr[i];
        vec2 sft = grad * bend * vec2(1.0 / aspect, 1.0);
        vec3 refr;
        refr.r = background(uv + sft * 1.3).r;
        refr.g = background(uv + sft * 1.0).g;
        refr.b = background(uv + sft * 0.7).b;
        float breathe = 1.0 + 0.18 * u_sel[i] * sin(u_time * 2.2);
        // 亮度模型：高透折射为底 + 白色 veil 提亮（gl_lift）+ tint 淡染——面内亮于背景
        vec3 glass = mix(refr, u_tint[i], 0.15);
        glass += vec3(u_lift);
        // 双带边缘：外亮 rim 细线（0.004 uv，对角方向性：左上受光 1.15/右下 0.75）
        // + 内侧次级亮带 bezel（0.008 uv 处弱亮线，双线厚度感）
        float rim = smoothstep(0.004, 0.0, abs(-d - 0.002));
        float diag = clamp(0.5 + 0.5 * (p.x / max(half_size.x, 1e-4)
                                        - p.y / max(half_size.y, 1e-4)), 0.0, 1.0);
        vec3 rim_color = mix(vec3(0.9, 0.92, 1.0), u_accent, u_sel[i]);
        glass += rim_color * rim * (0.70 * mix(1.15, 0.75, diag)) * breathe;
        float bezel = smoothstep(0.010, 0.0, abs(-d - 0.010));
        glass += vec3(0.55) * bezel * 0.22;
        // 顶缘镜面高光带：距顶缘向下深度指数衰减（0.05 uv 尺度，贴顶窄带）
        float depth_top = half_size.y - p.y;
        float spec = exp(-max(depth_top, 0.0) / 0.05) * smoothstep(0.003, 0.017, -d);
        glass += vec3(1.0) * spec * 0.20;
        col = mix(col, glass, cov);
        alpha = mix(alpha, min(alpha + 0.25, 0.80), cov);
    }
    gl_FragColor = vec4(col * alpha, alpha);
}
"""


def vertex_shader_source() -> str:
    # 顶点着色器源码（全屏四边形，属性 0 = vec2 位置）
    return _GLSL_VERT


def fragment_shader_source() -> str:
    # 片段着色器源码（程序化光场背景 + 多玻璃面折射/色散/立体感/选中呼吸合成）
    return _GLSL_FRAG


# ===== ui/gl/shaders.py 函数/模块级常量说明 =====
# _GLSL_VERT: 顶点着色器（a_pos 属性 0，直传裁剪坐标）
# _GLSL_FRAG: 片段着色器（uv 归一化空间；背景光场 + 玻璃面单 pass 合成，16 面上限）
#   SDF 全程纵横比校正（p/half_size 乘 vec2(aspect,1)，y 基准）——圆角真圆、
#   rim 全向等宽；u_rect[i]=(x, y_bottom, w, h) uv 系（画布注入时换算）
#   预乘输出 vec4(col*alpha, alpha)：Qt 合成 QOpenGLWidget 假定 premultiplied
#   亮度模型（走查返工定案）：glass = 折射背景×0.85 + tint 淡染 0.15 + 白 veil
#   （u_lift=ui.json glass.gl_lift，深 26/浅 34）；面内 alpha 0.80（透壁纸 20%）
#   玻璃面效果参数：rim 带宽 0.004 uv + 对角方向性（1.15/0.75）/ bezel 内亮带
#   0.010 uv 处 0.22 / 折射上限 u_refr（0.035 uv，中心满弯 m² 曲线）/ 色散
#   1.3/1.0/0.7 / 顶缘镜面高光带 0.05 uv 指数衰减强度 0.20 / SDF 投影（向下
#   0.012 uv、0.035 uv 衰减、35% 强度、面内剔除）
#   u_sel[16]：选中强度——rim 染 u_accent 金 + sin(u_time) 呼吸
#   背景增彩：金/冷双层错相云雾（0.09/0.07）+ 双晕强度 2.4——为折射提供色彩层次
# vertex_shader_source()/fragment_shader_source(): 源码读取接口（画布编译用）
#   设计理由：GLSL 常量集中在渲染模块，画布零着色器字符串拼接；云雾/颗粒/衰减系数
#   为材质微调常量（非业务配色）
#   异常处理：GLSL 编译错误由画布 initializeGL 链接失败路径降级（_ready=False）
