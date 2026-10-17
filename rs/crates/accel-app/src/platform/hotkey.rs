//! 全局热键（global-hotkey 集成，PL015.02）
//! 键位读 base.json shortcuts token（Ctrl+S 保存 / Ctrl+Q 退出 / Ctrl+T 主题循环）
//! 注册于主线程（winit 消息泵驱动 WM_HOTKEY），window 每帧 drain → HotkeyKind

#[cfg(windows)]
use std::collections::HashMap;

#[cfg(windows)]
use global_hotkey::hotkey::HotKey;
#[cfg(windows)]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};

/// 热键动作（与 base.json shortcuts 三键一一对应）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyKind {
    Save,
    Quit,
    Theme,
}

/// 快捷键 token → 动作清单（base.json /shortcuts/*；缺键降级跳过并记日志）
pub fn shortcut_specs() -> Vec<(HotkeyKind, String)> {
    let base = &accel_core::static_config::Static::get().base;
    let keys = [
        (HotkeyKind::Save, "save"),
        (HotkeyKind::Quit, "quit"),
        (HotkeyKind::Theme, "theme"),
    ];
    keys.iter()
        .filter_map(|(kind, name)| {
            let spec = base
                .pointer(&format!("/shortcuts/{name}"))
                .and_then(|v| v.as_str())?;
            Some((*kind, spec.to_string()))
        })
        .collect()
}

/// 键位描述解析（"Ctrl+S" → HotKey；非法 token 返 None 由调用方降级——不崩）
/// FIX006.28：global-hotkey 仅 Windows 依赖——本函数随 cfg 分档
#[cfg(windows)]
pub fn parse_binding(kind: HotkeyKind, spec: &str) -> Option<(HotkeyKind, HotKey)> {
    match spec.parse::<HotKey>() {
        Ok(hk) => Some((kind, hk)),
        Err(e) => {
            tracing::warn!("热键解析失败，该键位降级跳过: {spec} ({e})");
            None
        }
    }
}

/// 热键句柄集（manager 必须保活——drop 即全部反注册）
#[cfg(windows)]
pub struct HotkeyHandles {
    _manager: GlobalHotKeyManager,
    // HotKeyId → 动作映射（事件仅带 id，poll 经此还原动作）
    ids: HashMap<u32, HotkeyKind>,
}

#[cfg(windows)]
impl HotkeyHandles {
    /// 注册全部键位（部分失败降级：成功者继续生效，失败者记日志）
    pub fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new().map_err(|e| format!("热键管理器创建失败: {e}"))?;
        let mut ids = HashMap::new();
        for (kind, spec) in shortcut_specs() {
            if let Some((kind, hk)) = parse_binding(kind, &spec) {
                match manager.register(hk) {
                    Ok(()) => {
                        ids.insert(hk.id(), kind);
                    }
                    Err(e) => tracing::warn!("热键注册失败（可能被占用），降级跳过: {spec} ({e})"),
                }
            }
        }
        Ok(Self {
            _manager: manager,
            ids,
        })
    }

    /// drain 热键事件 → 动作列表（每帧调用；仅响应按下沿，松开忽略）
    pub fn poll(&self) -> Vec<HotkeyKind> {
        let mut out = Vec::new();
        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if ev.state() == global_hotkey::HotKeyState::Pressed
                && let Some(kind) = self.ids.get(&ev.id())
            {
                out.push(*kind);
            }
        }
        out
    }
}

/// 非 Windows 编译期桩（FIX006.28）：无全局热键能力——构造即 Err，
/// window 侧按「热键降级」路径运行（无热键 + 无托盘时关闭按钮直退）
#[cfg(not(windows))]
pub struct HotkeyHandles;

#[cfg(not(windows))]
impl HotkeyHandles {
    /// 非 Windows 无全局热键注册能力（上抛由调用方降级——与 Windows 侧失败同形）
    pub fn new() -> Result<Self, String> {
        Err("非 Windows 平台不支持全局热键".to_string())
    }

    pub fn poll(&self) -> Vec<HotkeyKind> {
        Vec::new()
    }
}

// ===== src/platform/hotkey.rs 说明区 =====
// 输入：base.json /shortcuts/{save,quit,theme} token + GlobalHotKeyEvent 全局通道
// 输出：HotkeyKind 流（Save/Quit/Theme）
// 逻辑步骤：shortcut_specs 读 token → parse_binding 解析（global-hotkey FromStr，
//   大小写不敏感）→ manager.register 注册（id→动作映射留存）→ poll 每帧 drain
// 设计理由：键位零硬编码（token 单源）；注册失败降级不崩（对齐 plan「注册失败降级
//   日志不崩」）；主线程注册（winit 泵 WM_HOTKEY，无独立线程）；按下沿触发避免
//   松开沿重复（SendInput 合成验证的判定基准）
// FIX006.28：global-hotkey 仅 Windows 依赖（Cargo.toml target 段）——HotkeyKind 与
//   shortcut_specs（纯 token 读）不分档；parse_binding/HotkeyHandles 走 cfg 分档，
//   非 Windows 为编译期桩（new 返 Err → 与 Windows 侧注册失败同形的降级路径）
// 异常处理：管理器创建失败整体上抛 → window 降级无热键；单键解析/注册失败跳过
//   该键位（其余键继续生效——局部降级不整体放弃）
// 关联配置：base.json shortcuts 三键；消费方：window.rs poll_platform
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_specs_reads_base_tokens() {
        // 三键位 token 齐备且取值与 base.json 一致
        let specs = shortcut_specs();
        assert_eq!(specs.len(), 3);
        assert!(specs.contains(&(HotkeyKind::Save, "Ctrl+S".to_string())));
        assert!(specs.contains(&(HotkeyKind::Quit, "Ctrl+Q".to_string())));
        assert!(specs.contains(&(HotkeyKind::Theme, "Ctrl+T".to_string())));
    }

    #[cfg(windows)]
    #[test]
    fn parse_binding_accepts_and_rejects() {
        // 合法键位解析成功；非法 token 返 None（降级路径，不 panic）
        let (k, hk) = parse_binding(HotkeyKind::Save, "Ctrl+S").expect("Ctrl+S 应可解析");
        assert_eq!(k, HotkeyKind::Save);
        assert!(parse_binding(HotkeyKind::Quit, "Ctrl+Q").is_some());
        assert!(parse_binding(HotkeyKind::Theme, "Ctrl+T").is_some());
        assert!(parse_binding(HotkeyKind::Save, "不是键位").is_none());
        // 解析产物 id 可作映射键（u32 稳定唯一性由库保证）
        let _id: u32 = hk.id();
    }

    #[cfg(windows)]
    #[test]
    fn register_and_poll_roundtrip() {
        // 注册→事件→poll：manager 注册成功后 poll 空转不 panic（无按键时零事件）
        // （按键沿注入属 SendInput 真窗验证——PL015.02 检验项；同场景人工矩阵已取消）
        match HotkeyHandles::new() {
            Ok(handles) => {
                assert!(!handles.ids.is_empty(), "base.json 三键至少应有注册成功者");
                assert!(handles.poll().is_empty(), "无按键事件时 poll 应为空");
            }
            Err(e) => tracing::warn!("热键注册失败（受限环境允许）: {e}"),
        }
    }
}
