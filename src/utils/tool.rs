//! 工具函数

use crate::core::state_in_range::StateInRange;
use crate::core::transfer::Transfer;
use crate::core::StateMachineBlueprint;
use crate::core::transition::Transition;

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

/// 将 blueprint 中所有 Transition 按 forbidden 区域拆分为两组
fn split_blueprint_by_forbidden_region(
    blueprint: StateMachineBlueprint,
    forbidden: StateInRange,
) -> (StateMachineBlueprint, StateMachineBlueprint) {
    let mut into_forbidden = blueprint.clone();
    let mut not_into_forbidden = blueprint.clone();

    // 处理 into_forbidden：保留会进入 forbidden 的部分
    into_forbidden.transitions = blueprint
        .transitions
        .iter()
        .cloned()
        .map(|t| {
            let (into, _) = partition_range_by_transfer_target(t.guard.clone(), forbidden.clone(), t.transfer.clone());
            Transition {
                guard: into,
                ..t
            }
        })
        .collect();

    // 处理 not_into_forbidden：保留不会进入 forbidden 的部分
    not_into_forbidden.transitions = blueprint
        .transitions
        .iter()
        .cloned()
        .map(|t| {
            let (_, not_into) = partition_range_by_transfer_target(t.guard.clone(), forbidden.clone(), t.transfer.clone());
            Transition {
                guard: not_into,
                ..t
            }
        })
        .collect();
    (into_forbidden, not_into_forbidden)
}