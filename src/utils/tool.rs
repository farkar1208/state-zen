//! 工具函数

use crate::core::state_in_range::StateInRange;
use crate::core::transfer::Transfer;

/// 根据转换目标对状态范围进行分区
/// 
/// 给定两个状态范围 A 和 B，以及一个转换函数 f，
/// 返回两个新的状态范围：
/// 1. 在范围 A 中，且转换后进入范围 B 的状态
/// 2. 在范围 A 中，但转换后不进入范围 B 的状态
/// 
/// # 参数
/// - `a`: 原始状态范围
/// - `b`: 目标状态范围
/// - `f`: 转换函数
/// 
/// # 返回值
/// 返回一个元组 `(in_b, not_in_b)`，其中：
/// - `in_b`: 在 A 中且转换后进入 B 的状态
/// - `not_in_b`: 在 A 中但转换后不进入 B 的状态
pub fn partition_range_by_transfer_target(
    a: StateInRange,
    b: StateInRange,
    f: Transfer,
) -> (StateInRange, StateInRange) {
    let c = StateInRange::new(move |s| {
        let next = f.apply(s);
        b.contains(&next)
    });
    (a.clone().and(c.clone()), a.and(c.not()))
}