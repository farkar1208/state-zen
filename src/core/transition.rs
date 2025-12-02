//! 状态转换定义

use std::sync::Arc;
use super::types::{TransitionId, EventId};
use super::state_in_range::StateInRange;
use super::transfer::Transfer;
use super::runtime::State;

/// 状态转换
/// 定义在特定事件和守卫条件下如何转换状态
#[derive(Clone)]
pub struct Transition {
    /// 转换的唯一标识符
    pub id: TransitionId,
    /// 触发转换的事件ID
    pub event_id: EventId,
    /// 守卫条件，状态必须满足此条件才能触发转换
    pub guard: StateInRange,
    /// 状态转换函数
    pub transfer: Transfer,
    /// 转换优先级（数值越大优先级越高）
    pub priority: i32,
    /// 转换执行时的回调函数
    pub on_tran: Option<Arc<dyn Fn(&State, &State) + Send + Sync>>,
}