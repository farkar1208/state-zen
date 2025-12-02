//! 运行时状态机

use std::collections::HashMap;
use std::sync::Arc;
use super::types::{StateAspectId, EventId};
use super::blueprint::StateMachineBlueprint;
use super::transition::Transition;

/// 运行时状态：aspect_id -> Arc<dyn Any>
pub type State = HashMap<StateAspectId, Arc<dyn std::any::Any + Send + Sync>>;

/// 运行时状态机
/// 管理状态机的当前状态和执行转换
pub struct RuntimeStateMachine {
    /// 状态机蓝图
    pub blueprint: StateMachineBlueprint,
    /// 当前状态
    pub current_state: State,
    /// 待处理的转换
    pending_transition: Option<Transition>,
}

impl RuntimeStateMachine {
    /// 创建一个新的运行时状态机
    pub fn new(blueprint: StateMachineBlueprint, initial_state: State) -> Self {
        Self {
            blueprint,
            current_state: initial_state,
            pending_transition: None,
        }
    }

    /// 领域事件 1: EventHappen
    /// 处理事件发生，选择符合条件的转换
    pub fn event_happen(&mut self, event_id: EventId, _payload: Option<Arc<dyn std::any::Any + Send + Sync>>) {
        let mut candidates: Vec<&Transition> = self
            .blueprint
            .transitions
            .iter()
            .filter(|t| t.event_id == event_id && t.guard.contains(&self.current_state))
            .collect();

        // 按优先级降序，同优先级按顺序（取第一个）
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

        self.pending_transition = candidates.first().cloned().cloned();
    }

    /// 领域事件 2: Transform
    /// 执行待处理的转换
    pub fn transform(&mut self) {
        if let Some(transition) = self.pending_transition.take() {
            let next_state = transition.transfer.apply(&self.current_state);

            // 计算 observers 的进出
            let mut on_exits = Vec::new();
            let mut on_enters = Vec::new();

            for observer in &self.blueprint.observers {
                let was_in = observer.region.contains(&self.current_state);
                let now_in = observer.region.contains(&next_state);

                if was_in && !now_in {
                    if let Some(on_exit) = &observer.on_exit {
                        on_exits.push(on_exit.clone());
                    }
                }
                if !was_in && now_in {
                    if let Some(on_enter) = &observer.on_enter {
                        on_enters.push(on_enter.clone());
                    }
                }
            }

            // 执行顺序: OnExit -> OnTran -> OnEnter
            for on_exit in on_exits {
                on_exit(&self.current_state);
            }

            if let Some(on_tran) = &transition.on_tran {
                on_tran(&self.current_state, &next_state);
            }

            for on_enter in on_enters {
                on_enter(&next_state);
            }

            self.current_state = next_state;
        }
    }
}