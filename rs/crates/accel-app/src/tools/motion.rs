//! 动效弹簧（PL016.06，kinetics 参数体系）：半隐式欧拉阻尼谐振子——
//! ζ<1 过冲（bouncy 抽屉）、ζ≥1 平滑（critically damped）；手写实现零依赖
//! 纯函数 step 可测（收敛性/过冲行为）

/// 阻尼谐振子状态（value 可越过 target 再回摆——过冲滑入的物理来源）
#[derive(Debug, Clone)]
pub struct Spring {
    pub value: f32,
    pub velocity: f32,
}

impl Spring {
    /// 自定义初值（一般从 0 起步）
    pub fn new(value: f32) -> Spring {
        Spring {
            value,
            velocity: 0.0,
        }
    }

    /// 单步推进（半隐式欧拉）：k=(2πf)²，c=2ζ√k；dt 钳 [1/240, 1/30] 防爆
    /// 返回是否仍在运动（|位移|或|速度|超阈）——调用方据此决定 16ms 提频
    pub fn step(&mut self, target: f32, dt: f32, freq_hz: f32, zeta: f32) -> bool {
        let dt = dt.clamp(1.0 / 240.0, 1.0 / 30.0);
        let omega = std::f32::consts::TAU * freq_hz.max(0.1);
        let k = omega * omega;
        let c = 2.0 * zeta.max(0.05) * omega;
        let accel = -k * (self.value - target) - c * self.velocity;
        self.velocity += accel * dt;
        self.value += self.velocity * dt;
        (self.value - target).abs() > 0.001 || self.velocity.abs() > 0.001
    }
}

/// 0..1 进度缓动（纯函数）：ease-out cubic——hover/淡入类小型动效的静止曲线
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

// ===== src/tools/motion.rs 说明区 =====
// 输入：target/dt/弹簧参数；输出：推进后的状态与「是否仍在运动」
// 逻辑步骤：半隐式欧拉（先速度后位移，能量稳定）→ 运动判定阈值 0.001
// 设计理由（PL016.06）：egui 无 tween 引擎——手写阻尼谐振子承载 kinetics
//   参数体系（bouncy 3.5Hz/ζ0.55 抽屉过冲）；dt 钳制防切帧爆炸；纯函数可测
// 异常处理：dt/频率/阻尼非法值全部钳制，无 panic 面
// 关联配置：ui.json motion.drawer_bouncy_{hz,zeta}；消费方：pages/mod 抽屉、
//   clock 闹钟脉冲（经 ease_out_cubic 或 Spring）
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_and_overshoots() {
        // bouncy（ζ<1）：过冲后收敛到 target；运动判定最终转 false
        let mut s = Spring::new(0.0);
        let mut overshot = false;
        for _ in 0..400 {
            let moving = s.step(1.0, 1.0 / 60.0, 3.5, 0.55);
            if s.value > 1.005 {
                overshot = true;
            }
            if !moving {
                break;
            }
        }
        assert!(overshot, "ζ<1 应出现 过冲");
        assert!(
            (s.value - 1.0).abs() < 0.01,
            "应收敛到目标，实际 {}",
            s.value
        );
    }

    #[test]
    fn spring_critically_damped_no_overshoot() {
        // ζ=1：平滑到达不过冲
        let mut s = Spring::new(0.0);
        for _ in 0..400 {
            if !s.step(1.0, 1.0 / 60.0, 3.5, 1.0) {
                break;
            }
        }
        assert!(s.value <= 1.0 + 1e-4, "ζ=1 不得过冲，实际 {}", s.value);
        assert!((s.value - 1.0).abs() < 0.01);
    }

    #[test]
    fn ease_out_cubic_bounds() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        assert_eq!(ease_out_cubic(2.0), 1.0, "越界钳满");
        assert!(ease_out_cubic(0.5) > 0.5, "ease-out 前快后慢");
    }
}
