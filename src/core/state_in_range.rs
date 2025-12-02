//! 状态谓词（StateInRange）
//! 用于判断状态是否在特定范围内

use std::sync::Arc;
use super::runtime::State;

/// 状态谓词，判断状态是否在特定范围内
#[derive(Clone)]
pub struct StateInRange {
    predicate: Arc<dyn Fn(&State) -> bool + 'static + Send + Sync>,
}

impl StateInRange {
    /// 创建一个新的状态谓词
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&State) -> bool + 'static + Send + Sync,
    {
        Self {
            predicate: Arc::new(f),
        }
    }

    /// 判断给定的状态是否满足谓词条件
    pub fn contains(&self, state: &State) -> bool {
        (self.predicate)(state)
    }

    /// 创建一个新的谓词，表示当前谓词的逻辑非
    pub fn not(self) -> Self {
        Self::new(move |s| !self.contains(s))
    }

    /// 创建一个新的谓词，表示当前谓词和另一个谓词的逻辑与
    pub fn and(self, other: Self) -> Self {
        Self::new(move |s| self.contains(s) && other.contains(s))
    }
}