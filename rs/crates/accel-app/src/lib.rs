//! AccelWorld RS 应用库（accel-app）
//! bin 形态（main/render_probe）共享的模块根；GUI 依赖仅本 crate

pub mod bus;
pub mod glass;
pub mod logging;
pub mod nav;
pub mod pages;
pub mod platform;
pub mod theme;
pub mod tools;
pub mod widgets;
pub mod window;

/// 编译期版本（build.rs 注入 ACCELWORLD_VERSION，源出 base.json 单一来源）
/// 与运行期 Static::get().version() 的 parity 由本文件单测锚定
pub fn compiled_version() -> &'static str {
    env!("ACCELWORLD_VERSION")
}

#[cfg(test)]
mod tests {
    #[test]
    fn compiled_version_matches_runtime_source() {
        // 版本单源 parity：编译期 env（build.rs 注入）== 运行期 base.json 读取
        assert_eq!(
            super::compiled_version(),
            accel_core::static_config::Static::get().version()
        );
    }
}

// ===== src/lib.rs 说明区 =====
// bin/main.rs 声明同名 mod 时与本库并存（bin 内 mod 为独立编译单元——为避免
// 双编译，main.rs 改为引用 lib：use accel_app::window; 模块树单源在本文件）
// 依赖方向：app → core 单向（CI 关卡 4 强制 core 零 egui/eframe）
// compiled_version：build.rs 版本单源注入的消费面（PL015.03）；产品显示一律
//   走运行期 Static 读取（零硬编码纪律不变），本函数服务编译期 parity 锚定
//   与打包脚本（NSIS/exe 资源同源核验）
