//! 玻璃绘制器（ui/gl/glass_canvas.py 的 Rust 对等物，PL013.06）
//! egui PaintCallback 跑 wgpu：每帧对场景可见面组 uniforms → 单 pass 全屏绘制

use std::sync::Arc;

use egui_wgpu::CallbackTrait;
use wgpu::util::DeviceExt;

use super::shader::{GlassUniforms, UNIFORMS_SIZE, build_shader_source};
use super::surface::{GlassScene, GlassSurface};
use crate::theme::FieldTokens;

/// 管线资源（挂 Renderer::callback_resources；首次 prepare 惰性初始化）
#[derive(Default)]
pub struct GlassResources {
    pipeline: Option<GlassPipeline>,
}

struct GlassPipeline {
    pipeline: wgpu::RenderPipeline,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

/// 单帧回调数据：uniforms 快照（字节直传，Send+Sync）
struct GlassFrame {
    bytes: Arc<Vec<u8>>,
}

impl CallbackTrait for GlassFrame {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = resources.get_mut::<GlassResources>()
            && let Some(pipeline) = &res.pipeline
        {
            queue.write_buffer(&pipeline.uniform_buf, 0, &self.bytes);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::epaint::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(res) = resources.get::<GlassResources>() else {
            return;
        };
        let Some(p) = &res.pipeline else { return };
        render_pass.set_pipeline(&p.pipeline);
        render_pass.set_bind_group(0, &p.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

/// 构造管线（frame 首帧 wgpu_render_state 可用时调用一次；已构造则早退）
/// FIX006.7：真实错误经 wgpu uncaptured_error 处理器记日志（不 panic）——本函数恒成功，
/// 降级面 = 「无 wgpu render state」（window.rs 早退分支走纯色背板）
pub fn attach_pipeline(
    device: &wgpu::Device,
    target_format: wgpu::TextureFormat,
    resources: &mut egui_wgpu::CallbackResources,
) {
    if resources
        .get::<GlassResources>()
        .is_some_and(|r| r.pipeline.is_some())
    {
        return;
    }
    // TypeMap 首次挂载：条目不存在则插入（否则尾部 get_mut panic——真窗首帧实证）
    if resources.get::<GlassResources>().is_none() {
        resources.insert(GlassResources::default());
    }
    // A005 #14 修正（实证）：不用 error scope + block_on——eframe 主线程等 wgpu 设备
    // 回执会死锁（首帧挂起，真窗探针实证）；改为安装 uncaptured_error 处理器，
    // 把管线/着色器错误降级为日志而非默认 panic
    device.on_uncaptured_error(std::sync::Arc::new(|e: wgpu::Error| {
        tracing::error!("玻璃管线 wgpu 错误（降级为日志，不 panic）: {e}");
    }));
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("glass_wgsl"),
        source: wgpu::ShaderSource::Wgsl(build_shader_source().into()),
    });
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("glass_uniforms"),
        contents: &[0u8; UNIFORMS_SIZE as usize],
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("glass_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("glass_bg"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("glass_pl"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("glass_rp"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    // A005 #14：管线错误经 uncaptured_error 处理器记录（不 panic）；本函数恒 Ok，
    // 真实降级路径 = frame 无 wgpu_render_state（软件回退）时的纯色背板
    resources
        .get_mut::<GlassResources>()
        .expect("GlassResources 必已注册")
        .pipeline = Some(GlassPipeline {
        pipeline,
        uniform_buf,
        bind_group,
    });
}

/// 场景 → uniforms 打包（uv 换算：像素 rect / 画布尺寸，y 翻底原点）
pub fn build_uniforms(
    scene: &GlassScene,
    size_px: [f32; 2],
    tokens: &FieldTokens,
    time_s: f32,
    accent: [f32; 4],
    lift: f32,
) -> GlassUniforms {
    // A005 #23：帧内零堆分配——可见面收集进栈数组（≤16）并按 elevation 插入排序
    // （原实现每帧 draw_list() 产 Vec + sort_by_key）
    let mut list: [Option<&GlassSurface>; GlassScene::MAX_SURFACES] =
        [None; GlassScene::MAX_SURFACES];
    let mut visible = 0usize;
    for s in scene.surfaces() {
        if !s.visible || visible == list.len() {
            continue;
        }
        let mut pos = visible;
        while pos > 0 && list[pos - 1].is_some_and(|p| p.elevation > s.elevation) {
            list[pos] = list[pos - 1];
            pos -= 1;
        }
        list[pos] = Some(s);
        visible += 1;
    }
    let mut u = GlassUniforms {
        res: size_px,
        count: visible as u32,
        rect: [[0.0; 4]; 16],
        radius_tint: [[0.0; 4]; 16],
        refr: [[0.0; 4]; 16],
        sel: [[0.0; 4]; 16],
        top: tokens.top,
        bottom: tokens.bottom,
        glow_a: tokens.glow_a,
        glow_b: tokens.glow_b,
        glow_a_pos: [tokens.glow_a_x, tokens.glow_a_y],
        glow_a_rad: tokens.glow_a_r,
        glow_b_pos: [tokens.glow_b_x, tokens.glow_b_y],
        glow_b_rad: tokens.glow_b_r,
        accent: [accent[0], accent[1], accent[2]],
        time: time_s,
        lift,
    };
    let (w, h) = (size_px[0].max(1.0), size_px[1].max(1.0));
    for (i, s) in list.iter().take(visible).flatten().enumerate() {
        // 像素 → uv（y 底原点）：y_uv = 1 - (y + h_px)/H
        let x = s.rect.min.x / w;
        let y = 1.0 - (s.rect.max.y) / h;
        let sw = s.rect.width() / w;
        let sh = s.rect.height() / h;
        u.rect[i] = [x, y, sw, sh];
        u.radius_tint[i] = [s.radius, s.tint[0], s.tint[1], s.tint[2]];
        u.refr[i] = [s.refraction, 0.0, 0.0, 0.0];
        u.sel[i] = [s.selected, 0.0, 0.0, 0.0];
    }
    u
}

/// 组装本帧回调（window.rs 每帧调用）：rect=全屏，回调持 uniforms 快照
pub fn push_paint_callback(painter: &egui::Painter, uniforms: GlassUniforms) {
    let callback = egui_wgpu::Callback::new_paint_callback(
        painter.clip_rect(),
        GlassFrame {
            bytes: Arc::new(uniforms.to_bytes()),
        },
    );
    painter.add(callback);
}

// ===== src/glass/painter.rs 说明区 =====
// 输入：GlassScene + FieldTokens + 时间/强调色/lift；输出：egui PaintCallback（wgpu 绘制）
// 逻辑步骤：build_uniforms（像素→uv 底原点 + radius/tint 打包）→ to_bytes →
//   GlassFrame 回调 → prepare 写 uniform buffer → paint 全屏 draw(3)
// 设计理由：单 pass 全屏与 Python GL 版管线形态一致（背景光场+玻璃合成同帧）；
//   管线挂 CallbackResources 惰性构造一次（eframe wgpu_render_state 就绪后）；
//   回调数据仅 1.2KB uniforms 快照（Send+Sync 安全克隆，场景本体不跨线程）；
//   uv y 翻转在 uniforms 组装端完成（wgpu 帧原点左上），shader 保持 GL 语义逐行
// 异常处理：管线错误走 wgpu uncaptured_error 处理器（日志不 panic，FIX006.7 后本函数
//   无 Result）；降级面 = 无 wgpu render state（window.rs 早退走纯色背板）；
//   paint 缺资源静默跳过（竞态防御）
// 已知偏移：①blend 用 wgpu PREMULTIPLIED_ALPHA_BLENDING（对齐预乘输出）；
//   ②重绘调度（touch→request_repaint）由 window.rs 注入 hook，painter 只负责绘制
// 关联依赖：wgpu 30/egui-wgpu 0.37；消费方：window.rs（每帧）、render_probe（离屏）
#[cfg(test)]
mod tests {
    use super::*;
    use crate::glass::surface::GlassSurface;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }

    #[test]
    fn uniforms_uv_conversion_bottom_origin() {
        // 像素 → uv 底原点换算：800x600 画布上 (100,200,300,100) 面
        let mut scene = GlassScene::new();
        let mut s = GlassSurface::new("a", rect(100.0, 200.0, 300.0, 100.0));
        s.radius = 10.0;
        s.tint = [0.1, 0.2, 0.3];
        s.selected = 0.5;
        scene.register(s);
        let tokens = FieldTokens::load("dark");
        let u = build_uniforms(
            &scene,
            [800.0, 600.0],
            &tokens,
            1.0,
            [1.0, 0.8, 0.3, 1.0],
            0.1,
        );
        assert_eq!(u.count, 1);
        assert!((u.rect[0][0] - 0.125).abs() < 1e-6, "x/w");
        // y = 1 - (200+100)/600 = 0.5
        assert!((u.rect[0][1] - 0.5).abs() < 1e-6);
        assert!((u.rect[0][2] - 0.375).abs() < 1e-6);
        assert!((u.rect[0][3] - 100.0 / 600.0).abs() < 1e-6);
        assert_eq!(u.radius_tint[0][0], 10.0);
        assert_eq!(u.radius_tint[0][1..4], [0.1, 0.2, 0.3]);
        assert_eq!(u.sel[0][0], 0.5);
        // 光场 token 直传
        assert!((u.glow_a_pos[0] - tokens.glow_a_x).abs() < 1e-6);
    }

    #[test]
    fn uniforms_capped_at_16_and_hidden_excluded() {
        // A005 #22 修正后契约：register 侧强制 16 面上限（超限拒收 + 告警），
        // 隐藏面仍剔除；build_uniforms 的 take(16) 保留为第二道保险
        let mut scene = GlassScene::new();
        for i in 0..20 {
            scene.register(GlassSurface::new(
                &format!("s{i}"),
                rect(0.0, 0.0, 10.0, 10.0),
            ));
        }
        assert_eq!(scene.total_count(), 16, "register 侧应拒收超限新面");
        scene.set_visible("s5", false);
        let tokens = FieldTokens::load("dark");
        let u = build_uniforms(&scene, [800.0, 600.0], &tokens, 0.0, [0.0; 4], 0.0);
        assert_eq!(u.count, 15, "16 面 - 1 隐藏面");
    }
}
