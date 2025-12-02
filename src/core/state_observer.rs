//! 状态观察者

use std::sync::Arc;
use super::types::ObserverId;
use super::state_in_range::StateInRange;
use super::runtime::State;

/// 状态观察者
/// 监控特定状态区域，在状态进入或退出该区域时触发回调
#[derive(Clone)]
pub struct StateObserver {
    /// 观察者的唯一标识符
    pub id: ObserverId,
    /// 观察的状态区域
    pub region: StateInRange,
    /// 状态进入该区域时的回调函数
    pub on_enter: Option<Arc<dyn Fn(&State) + Send + Sync>>,
    /// 状态退出该区域时的回调函数
    pub on_exit: Option<Arc<dyn Fn(&State) + Send + Sync>>,
}