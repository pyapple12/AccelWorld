//! 渲染断言质量门探针（PL013.07，Python 版探针的 Rust 对等物）
//! 离屏 wgpu 渲染标准场景（胶囊 + 卡片两已知面）→ buffer readback → 分区采样断言
//! 运行：cargo run -p accel-app --bin render_probe（退出码 0=全 PASS）

use accel_app::glass::painter::build_uniforms;
use accel_app::glass::shader::{UNIFORMS_SIZE, build_shader_source};
use accel_app::glass::surface::{GlassScene, GlassSurface};
use accel_app::theme::FieldTokens;
use egui::Rect;
use wgpu::util::DeviceExt;

/// 断言阈值（沿用 Python 版探针实测基线——见 z.plan PL013.07 定案）
const THRESH_FACE_VS_BG: i32 = 12; // 面内外色差（亮度通道）
const THRESH_RIM_DELTA: i32 = 60; // rim 线增量
const BG_ALPHA_RANGE: (f32, f32) = (0.05, 0.25); // 背景 alpha 微黑档

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
}

fn main() {
    let mut failures = 0;
    println!("== 渲染断言质量门（离屏 wgpu 640x480）==");

    // 适配器/设备（离屏无表面）
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .expect("无可用 GPU 适配器");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("render_probe"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        memory_hints: Default::default(),
        trace: Default::default(),
        experimental_features: Default::default(),
    }))
    .expect("设备创建失败");

    // 标准场景：卡片（中部）+ 胶囊（上部窄条）——面内外/rim/晕四区采样基准
    let mut scene = GlassScene::new();
    let mut card = GlassSurface::new("card", rect(170.0, 190.0, 300.0, 100.0));
    card.radius = 16.0;
    let mut pill = GlassSurface::new("pill", rect(220.0, 80.0, 200.0, 40.0));
    pill.radius = 20.0;
    pill.elevation = 1;
    scene.register(card);
    scene.register(pill);
    let tokens = FieldTokens::load("dark");
    let uniforms = build_uniforms(
        &scene,
        [640.0, 480.0],
        &tokens,
        1.234,
        [1.0, 0.78, 0.30, 1.0],
        0.10,
    );
    let bytes = uniforms.to_bytes();
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);

    // 离屏目标
    let (w, h) = (640u32, 480u32);
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("probe_target"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    // 管线（与 painter 相同形态：单 uniform binding + 全屏三角 + 预乘混合）
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("probe_wgsl"),
        source: wgpu::ShaderSource::Wgsl(build_shader_source().into()),
    });
    let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("probe_uniforms"),
        contents: &bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("probe_bgl"),
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
        label: Some("probe_bg"),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buf.as_entire_binding(),
        }],
    });
    let playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("probe_pl"),
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("probe_rp"),
        layout: Some(&playout),
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
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // 绘制 + readback
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("probe_enc"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("probe_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
    let out_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("probe_readback"),
        size: (w * h * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &out_buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
    let cmd = encoder.finish();
    queue.submit([cmd]);
    let slice = out_buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    // 阻塞等待映射完成（FIX006.17：poll 结果判定 + recv 超时兜底——原实现丢弃 poll 结果
    // 且 recv 无超时，poll 出错时探针会永久挂起）
    if let Err(e) = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    }) {
        panic!("device.poll 失败: {e:?}");
    }
    match rx.recv_timeout(std::time::Duration::from_secs(10)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("map_async 回调失败: {e:?}"),
        Err(e) => panic!("map_async 回调超时未到达: {e}"),
    }
    let data: Vec<u8> = slice.get_mapped_range().expect("映射失败").to_vec();
    // slice 为 Copy 借位句柄无须显式 drop；映射在 out_buf 生命周期内有效

    // 像素取样（(x,y) → RGBA8；wgpu 帧原点左上）
    let px = |x: u32, y: u32| -> [f32; 4] {
        let i = ((y * w + x) * 4) as usize;
        [
            data[i] as f32 / 255.0,
            data[i + 1] as f32 / 255.0,
            data[i + 2] as f32 / 255.0,
            data[i + 3] as f32 / 255.0,
        ]
    };
    let luma = |c: [f32; 4]| -> i32 { (c[0] * 60.0 + c[1] * 120.0 + c[2] * 60.0) as i32 };

    // 断言 1 背景非透明微黑（左上角远场面外）
    {
        let bg = px(8, 8);
        let pass = bg[3] > BG_ALPHA_RANGE.0 && bg[3] < BG_ALPHA_RANGE.1;
        println!(
            "{} 背景 alpha 微黑档: alpha={:.3}（期望 {:.2}-{:.2}）",
            show(pass),
            bg[3],
            BG_ALPHA_RANGE.0,
            BG_ALPHA_RANGE.1
        );
        failures += !pass as i32;
    }
    // 断言 2 面内外色差（卡片中心 vs 卡片左侧远处背景）
    {
        let inside = px(320, 240);
        let outside = px(60, 240);
        let delta = (luma(inside) - luma(outside)).abs();
        let pass = delta >= THRESH_FACE_VS_BG;
        println!(
            "{} 面内外色差: {delta}（期望 ≥{THRESH_FACE_VS_BG}）内=({},{} {}) 外=({},{} {})",
            show(pass),
            inside[0],
            inside[1],
            inside[2],
            outside[0],
            outside[1],
            outside[2],
        );
        failures += !pass as i32;
    }
    // 断言 3 rim 增量：峰值在卡片**底缘内侧 ~2px**（card rect y∈[190,290]，y=288 距底缘 2px）
    // 采样 rim 峰 (320,288) vs 底缘外 6px 基线 (320,296)
    {
        let rim = px(320, 288);
        let far = px(320, 296);
        let delta = luma(rim) - luma(far);
        let pass = delta >= THRESH_RIM_DELTA;
        println!(
            "{} rim 增量: {delta}（期望 ≥{THRESH_RIM_DELTA}）峰=({:.3},{:.3},{:.3}) 外=({:.3},{:.3},{:.3})",
            show(pass),
            rim[0],
            rim[1],
            rim[2],
            far[0],
            far[1],
            far[2],
        );
        failures += !pass as i32;
    }
    // 断言 4 光场双晕：glow_a（金，右上方向）色相偏暖 / glow_b（冷，左下）
    {
        let warm = px(520, 100);
        let cool = px(120, 400);
        let warm_bias = warm[0] + warm[1] - 2.0 * warm[2];
        let cool_bias = cool[2] - (cool[0] + cool[1]) / 2.0;
        let pass = warm_bias > 0.0 && cool_bias > -0.05;
        println!(
            "{show} 光场双晕色相: 暖偏 {warm_bias:.3} / 冷偏 {cool_bias:.3}（暖>0 且 冷>-0.05）",
            show = show(pass)
        );
        failures += !pass as i32;
    }
    // 断言 5 预乘：alpha 通道 ≤ RGB 最大通道（premultiplied 不变量）
    {
        let mut ok = true;
        for (x, y) in [(320u32, 240u32), (320, 60), (8, 8), (600, 460)] {
            let c = px(x, y);
            if c[3] < c[0].max(c[1]).max(c[2]) - 0.01 {
                ok = false;
            }
        }
        println!(
            "{show} 预乘不变量（alpha ≥ RGB 各通道，四采样点）",
            show = show(ok)
        );
        failures += !ok as i32;
    }

    println!("== 结果: {} FAIL ==", failures);
    std::process::exit(if failures == 0 { 0 } else { 1 });
}

fn show(pass: bool) -> &'static str {
    if pass { "PASS" } else { "FAIL" }
}

// ===== src/bin/render_probe.rs 说明区 =====
// 输入：无（标准场景内置：卡片 170,190,300,100 + 胶囊 220,80,200,40，640x480）
// 输出：stdout 每断言一行 PASS/FAIL；退出码 0=全过 / 1=任一 FAIL（CI 可选关卡）
// 逻辑步骤：离屏 wgpu 适配器/设备 → 同一 WGSL（build_shader_source）管线 →
//   uniforms 打包提交 → 渲染 → buffer readback → 五区采样断言
// 断言与阈值（沿用 Python 版探针实测基线）：背景 alpha 0.05-0.25 / 面内外色差 ≥12 /
//   rim 增量 ≥60（峰在面内侧 ~1.3px）/ 光场双晕色相 / 预乘不变量（alpha ≥ RGB）
// 设计理由：玻璃观感缺陷无法用功能探针捕捉（PL009 三轮返工教训）——渲染质量门
//   机器化，任何 WGSL/uniform 改动先过本探针
// 异常处理：适配器/设备缺失 panic（无 GPU 环境探针无法运行，属环境错误）
// 关联依赖：wgpu 30/accel-app glass；灵敏度验证：rim 强度 0.70→0.05 注入即红
