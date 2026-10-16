//! 重试工具（utils/retry.py 的 Rust 对等物）
//! 异步泛型重试：次数/间隔/错误白名单（IsRetryable trait 对应 Python 异常元组）

use std::future::Future;
use std::time::Duration;

/// 重试规格（Python retry_call 的 retries/delay 参数对等；retries ≥ 1）
#[derive(Debug, Clone, Copy)]
pub struct RetrySpec {
    pub retries: u32,
    pub delay: Duration,
}

/// 可重试错误判定（Python exceptions 元组白名单的 Rust 对等物）
/// 实现方按错误变体返回 true/false：网络类 true、解析/拒绝类 false
pub trait IsRetryable {
    fn is_retryable(&self) -> bool;
}

/// 通用重试：调用 f，失败时按 IsRetryable 判定，未达上限则 sleep 后重试
/// 全部失败上抛最后一次错误（Python raise last_exc 对等）；retries < 1 构造期 panic
pub async fn retry<F, Fut, T, E>(spec: RetrySpec, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: IsRetryable,
{
    assert!(spec.retries >= 1, "retries 必须大于等于 1");
    let mut last_err: Option<E> = None;
    for attempt in 0..spec.retries {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if !e.is_retryable() {
                    return Err(e);
                }
                last_err = Some(e);
                if attempt < spec.retries - 1 {
                    tokio::time::sleep(spec.delay).await;
                }
            }
        }
    }
    Err(last_err.expect("retries >= 1 保证至少一次调用"))
}

// ===== src/util/retry.rs 说明区 =====
// 输入：RetrySpec（次数/间隔）+ 闭包工厂（FnMut → Future，每轮新 Future）
// 输出：Ok(T) 或最后一次 Err（白名单外错误立即上抛）
// 逻辑步骤：循环尝试 → is_retryable 判定 → 非重试错误直通 → 达上限前 sleep → 上抛末次
// 设计理由：FnMut() -> Fut 形态适配 async 闭包/借用捕获（每次调用产新 Future）；
//   白名单经 trait 而非枚举硬编码，weather 传输层自行定义网络/解析分级
// 异常处理：retries < 1 为编程错误 assert 快速失败（Python ValueError 对等）
// 关联依赖：tokio::time（sleep）；消费方：weather 传输层（PL011.12）
#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum FakeErr {
        Net,
        Parse,
    }

    impl IsRetryable for FakeErr {
        fn is_retryable(&self) -> bool {
            matches!(self, FakeErr::Net)
        }
    }

    fn spec() -> RetrySpec {
        RetrySpec {
            retries: 3,
            delay: Duration::from_millis(0),
        }
    }

    #[tokio::test]
    async fn success_zero_retry() {
        // 成功零重试：计数恰 1 次调用
        let mut calls = 0;
        let v = retry(spec(), || {
            calls += 1;
            std::future::ready(Ok::<_, FakeErr>(42))
        })
        .await
        .expect("应成功");
        assert_eq!((v, calls), (42, 1));
    }

    #[tokio::test]
    async fn retryable_exhausts_with_last_error() {
        // 可重试失败耗尽：调用次数 = retries，上抛错误
        let mut calls = 0;
        let err = retry(spec(), || {
            calls += 1;
            std::future::ready(Err::<i32, _>(FakeErr::Net))
        })
        .await
        .expect_err("应耗尽失败");
        assert_eq!((err, calls), (FakeErr::Net, 3));
    }

    #[tokio::test]
    async fn non_retryable_raises_immediately() {
        // 白名单外错误：立即上抛，不消耗后续重试
        let mut calls = 0;
        let err = retry(spec(), || {
            calls += 1;
            if calls == 1 {
                std::future::ready(Err::<i32, _>(FakeErr::Parse))
            } else {
                std::future::ready(Ok(1))
            }
        })
        .await
        .expect_err("非重试错误应直通");
        assert_eq!((err, calls), (FakeErr::Parse, 1));
    }

    #[tokio::test]
    async fn recover_on_second_attempt() {
        // 第二次恢复：恰 2 次调用后成功
        let mut calls = 0;
        let v = retry(spec(), || {
            calls += 1;
            if calls == 1 {
                std::future::ready(Err::<i32, _>(FakeErr::Net))
            } else {
                std::future::ready(Ok(7))
            }
        })
        .await
        .expect("第二次应成功");
        assert_eq!((v, calls), (7, 2));
    }

    #[tokio::test]
    #[should_panic(expected = "retries 必须大于等于 1")]
    async fn zero_retries_panics() {
        // retries < 1：构造期快速失败（Python ValueError 对等）
        retry(
            RetrySpec {
                retries: 0,
                delay: Duration::ZERO,
            },
            || std::future::ready(Ok::<i32, FakeErr>(1)),
        )
        .await
        .expect("不应到达");
    }
}
