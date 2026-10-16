//! 世界时钟页（新页：城市管理，PL014 落地）
//! 用户产品定案：加号添加城市（统一城市表检索）、减号删除、卡式多城展示

// ===== src/pages/world.rs 说明区 =====
// 占位：结构镜像预建（PL011.02），实现随 PL014 进入
// 数据源：config/static/world_cities.json 统一城市表（PL012.H schema）
//   经 interface 检索；时区换算 chrono-tz（PL012 引入后可用）
