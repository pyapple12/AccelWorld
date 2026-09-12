# 玻璃着色器源码模块（PL008.04）：顶点/背景/玻璃三段 GLSL 常量
# GLSL 版本：不带 #version（默认兼容 1.x 口径），最大化旧驱动兼容；
# 折射数学：玻璃面内像素沿 SDF 梯度方向偏移采样背景，偏移 ∝ (1-m²)×refr
# （m = 到边缘的归一化深度），曲率从边缘向内部连续衰减，无折角

_GLSL_VERT = """attribute vec2 a_pos;
void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"""

# 背景 + 玻璃合成单 pass（程序化光场：渐变/云雾/双晕/颗粒，避免 FBO 翻转复杂度）
_GLSL_FRAG = """precision highp float;
uniform vec2 u_res;
uniform int u_count;
uniform vec4 u_rect[8];
uniform float u_radius[8];
uniform float u_refr[8];
uniform vec3 u_tint[8];
uniform vec3 u_top;
uniform vec3 u_bottom;
uniform vec3 u_glow_a;
uniform vec3 u_glow_b;

vec3 background(vec2 px) {
    vec2 uv = px / u_res;
    vec3 col = mix(u_bottom, u_top, uv.y);
    float cloud = sin(uv.x * 6.2831 + 0.7) * sin(uv.y * 4.5 - 0.31);
    col += vec3(0.89, 0.70, 0.42) * (0.5 + 0.5 * cloud) * 0.05;
    float d1 = distance(uv, vec2(0.84, 0.05));
    col += u_glow_a * exp(-d1 * 4.0) * 0.30;
    float d2 = distance(uv, vec2(0.02, 0.95));
    col += u_glow_b * exp(-d2 * 3.5) * 0.28;
    float grain = fract(sin(dot(px, vec2(12.9898, 78.233))) * 43758.5453);
    col += (grain - 0.5) * 0.012;
    return col;
}

float sd_round_box(vec2 p, vec2 b, float r) {
    vec2 q = abs(p) - b + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

void main() {
    vec2 frag = gl_FragCoord.xy;
    vec3 col = background(frag);
    for (int i = 0; i < 8; i++) {
        if (i >= u_count) { break; }
        vec2 center = u_rect[i].xy + u_rect[i].zw * 0.5;
        vec2 half_size = u_rect[i].zw * 0.5;
        float d = sd_round_box(frag - center, half_size, min(u_radius[i], half_size.y));
        float cov = 1.0 - smoothstep(0.0, 1.2, -d);
        if (cov <= 0.0) { continue; }
        vec2 grad = normalize((frag - center) / half_size + 1e-6);
        float bend = (1.0 - pow(clamp(-d / half_size.y, 0.0, 1.0), 2.0)) * u_refr[i];
        vec3 refr;
        refr.r = background(frag + grad * bend * 1.3).r;
        refr.g = background(frag + grad * bend * 1.0).g;
        refr.b = background(frag + grad * bend * 0.7).b;
        vec3 glass = mix(refr, u_tint[i], 0.22);
        float rim = smoothstep(0.006, 0.0, abs(-d - 0.004));
        glass += vec3(0.9, 0.92, 1.0) * rim * 0.55;
        col = mix(col, glass, cov);
    }
    gl_FragColor = vec4(col, 1.0);
}
"""


def vertex_shader_source() -> str:
    # 顶点着色器源码（全屏四边形，属性 0 = vec2 位置）
    return _GLSL_VERT


def fragment_shader_source() -> str:
    # 片段着色器源码（程序化光场背景 + 多玻璃面折射/色散/rim 合成）
    return _GLSL_FRAG
