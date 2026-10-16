//! 信号总线（前后端解耦通道，z.plan 修订一新增件）
//! PL013 落地：cmd/evt 双 mpsc 通道 + update() 每帧 drain + 命令发送 helper

// ===== src/bus.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL013 进入
// 设计约束：UI 意图全走 FrontendCommand（单一出口 send()）；
//   每帧 while try_recv drain 全部事件→UiState::apply(evt)——锁不跨帧
