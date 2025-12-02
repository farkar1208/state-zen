//! 状态机蓝图

use std::collections::HashMap;
use super::types::{StateAspectId, EventId};
use super::state_aspect::StateAspect;
use super::event::EventDef;
use super::transition::Transition;
use super::state_observer::StateObserver;

/// 状态机蓝图
/// 包含状态机的完整定义：方面、事件、转换和观察者
#[derive(Clone)]
pub struct StateMachineBlueprint {
    /// 状态方面定义
    pub aspects: HashMap<StateAspectId, StateAspect>,
    /// 事件定义
    pub events: HashMap<EventId, EventDef>,
    /// 状态转换定义
    pub transitions: Vec<Transition>,
    /// 状态观察者定义
    pub observers: Vec<StateObserver>,
}

impl StateMachineBlueprint {
    /// 创建一个新的空蓝图
    pub fn new() -> Self {
        Self {
            aspects: HashMap::new(),
            events: HashMap::new(),
            transitions: Vec::new(),
            observers: Vec::new(),
        }
    }

    /// 合并两个蓝图
    /// 返回一个新的蓝图，包含两个蓝图的所有定义
    pub fn merge(&self, other: &Self) -> Self {
        let mut aspects = self.aspects.clone();
        let mut events = self.events.clone();
        let mut transitions = self.transitions.clone();
        let mut observers = self.observers.clone();

        for (k, v) in &other.aspects {
            aspects.insert(*k, v.clone());
        }
        for (k, v) in &other.events {
            events.insert(*k, v.clone());
        }
        transitions.extend(other.transitions.iter().cloned());
        observers.extend(other.observers.iter().cloned());

        Self {
            aspects,
            events,
            transitions,
            observers,
        }
    }
}

impl Default for StateMachineBlueprint {
    fn default() -> Self {
        Self::new()
    }
}