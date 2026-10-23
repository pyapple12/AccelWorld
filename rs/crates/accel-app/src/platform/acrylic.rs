//! Acrylic 窗口背板（ui/backdrop.py 的 Rust 对等物，PL013.04）
//! DwmSetWindowAttribute(38=SYSTEMBACKDROP_TYPE, 3=TRANSIENTWINDOW)；失败降级实底

// DWM 常量（Windows SDK dwmapi 头文件；属性 38 = SYSTEMBACKDROP_TYPE）
// FIX007.4：cfg 化——仅 Windows 消费，非 Windows 构建不再产生 dead_code 告警
#[cfg(windows)]
const DWMSBT_TRANSIENTWINDOW: u32 = 3; // Acrylic（2=Mica、4=Mica Alt 均不采用）

/// 开启 Win11 Acrylic 背板；不支持/失败静默降级并返 false（调用方无须提示）
/// 步骤对齐 Python 版：①DwmExtendFrameIntoClientArea 底座（1px 底边，qfw 同款）
/// ②DwmSetWindowAttribute(38=SYSTEMBACKDROP_TYPE, 3=TRANSIENTWINDOW)
pub fn apply_acrylic(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        // FIX006.28：导入随 cfg 块（非 Windows 构建不再产生 unused_imports）
        use tracing::error;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::{
            DWMWA_SYSTEMBACKDROP_TYPE as ATTR, DwmExtendFrameIntoClientArea, DwmSetWindowAttribute,
        };
        use windows::Win32::UI::Controls::MARGINS;

        let hwnd = HWND(hwnd as *mut _);
        // 底座：扩展框进客户区（qfw setMicaEffectEnabled 同款 1px 底边）——
        // 无此步直接覆写背板会导致 DWM 合成异常（窗口不可见，真窗实证）
        let margins = MARGINS {
            cxLeftWidth: 0,
            cxRightWidth: 0,
            cyTopHeight: 0,
            cyBottomHeight: 1,
        };
        let hr_base = unsafe { DwmExtendFrameIntoClientArea(hwnd, &margins) };
        if hr_base.is_err() {
            error!("Acrylic 底座扩展失败（降级实底）: {hr_base:?}");
            return false;
        }
        let value: u32 = DWMSBT_TRANSIENTWINDOW;
        let hr = unsafe {
            DwmSetWindowAttribute(
                hwnd,
                ATTR,
                &value as *const u32 as *const _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        if hr.is_ok() {
            tracing::info!("Acrylic 背板开启成功");
            true
        } else {
            error!("Acrylic 背板开启失败（降级实底）: {hr:?}");
            false
        }
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

/// 失败注入探针（.04 检验：非法属性常量 → 函数返 false 且窗口不崩）
/// 生产零调用——render/截图探针与单测验证灵敏度用
pub fn apply_acrylic_invalid_for_probe(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;
        let invalid_attr: i32 = 9999; // 不存在的 DWM 属性 → E_INVALIDARG
        let value: u32 = DWMSBT_TRANSIENTWINDOW;
        let hr = unsafe {
            DwmSetWindowAttribute(
                HWND(hwnd as *mut _),
                windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(invalid_attr),
                &value as *const u32 as *const _,
                std::mem::size_of::<u32>() as u32,
            )
        };
        hr.is_ok()
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

// ===== src/platform/acrylic.rs 说明区 =====
// 输入：窗口句柄（isize HWND）；输出：是否成功开启 Acrylic
// 逻辑步骤：DwmSetWindowAttribute(38, 3) → hr 判定；失败 error 日志 + 返 false
// 设计理由：复用 Python PL003 定案（只做 Acrylic 不做 Mica；复用透明底座 + 覆写
//   背板类型 3）；viewport transparent（window.rs NativeOptions）+ wgpu alpha 通道
//   组成透明链路；失败降级实底（field.top token 由 theme 背板兜底——透明 viewport
//   在 DWM 失败时由 egui CentralPanel 实底填充）
// 异常处理：仅 DWM 返回码判定，不 panic；非 Windows 平台恒 false（编译期裁剪）
// FIX006.28：`use tracing::error` 移入 `#[cfg(windows)]` 块（原置文件顶层，非 Windows
//   构建产生 unused_imports 告警）
// 关联依赖：windows-rs（Win32_Graphics_Dwm）；消费方：window.rs（创建后挂接）、
//   .04 检验探针（apply_acrylic_invalid_for_probe 灵敏度验证）
#[cfg(test)]
mod tests {
    #[test]
    fn invalid_attr_injection_fails_gracefully() {
        // 失败路径（真窗句柄 0 —— 无效句柄 + 非法属性双失败注入）：
        // 断言返 false 且进程存活（无 panic）
        let ok = super::apply_acrylic_invalid_for_probe(0);
        assert!(!ok, "非法属性注入应失败返 false");
        // 真函数对无效句柄同样安全
        assert!(!super::apply_acrylic(0));
    }
}
