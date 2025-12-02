//! 状态转换函数

use std::sync::Arc;
use super::runtime::State;

/// 状态转换函数
/// 定义如何从一个状态转换到另一个状态
#[derive(Clone)]
pub struct Transfer {
    func: Arc<dyn Fn(&State) -> State + 'static + Send + Sync>,
}

impl Transfer {
    /// 创建一个新的转换函数
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&State) -> State + 'static + Send + Sync,
    {
        Self {
            func: Arc::new(f),
        }
    }

    /// 应用转换函数到给定的状态
    pub fn apply(&self, state: &State) -> State {
        (self.func)(state)
    }
}