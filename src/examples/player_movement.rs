//! 玩家移动示例
//! 演示如何使用状态机框架实现玩家移动逻辑

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use crate::core::{
    StateAspect, StateInRange, Transfer, EventDef, Transition, StateObserver,
    StateMachineBlueprint, RuntimeStateMachine, State,
};

/// 玩家动作枚举
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Idle,
    Walk,
}

/// 创建玩家移动状态机示例
pub fn create_player_movement_example() -> RuntimeStateMachine {
    // 1. 定义 aspects
    let action_aspect = StateAspect {
        id: 1,
        value_type_id: TypeId::of::<Action>(),
    };

    // 2. 定义事件
    let press_w_event = EventDef {
        id: 100,
        payload_type_id: TypeId::of::<()>(), // 无 payload
    };

    // 3. 定义谓词
    let is_idle = StateInRange::new(|s| {
        s.get(&1)
            .and_then(|v| v.downcast_ref::<Action>())
            .map_or(false, |a| *a == Action::Idle)
    });

    let is_walking = StateInRange::new(|s| {
        s.get(&1)
            .and_then(|v| v.downcast_ref::<Action>())
            .map_or(false, |a| *a == Action::Walk)
    });

    // 4. 定义 transfer
    let press_w_to_walk = Transfer::new(|s| {
        let mut new_s = s.clone();
        new_s.insert(1, Arc::new(Action::Walk));
        new_s
    });

    // 5. 定义 transition
    let transition = Transition {
        id: 1,
        event_id: press_w_event.id,
        guard: is_idle,
        transfer: press_w_to_walk,
        priority: 0,
        on_tran: Some(Arc::new(|_prev, _next| {
            println!("OnTran: Playing footstep sound");
        })),
    };

    // 6. 定义 observer
    let walking_observer = StateObserver {
        id: 1,
        region: is_walking,
        on_enter: Some(Arc::new(|_state| {
            println!("OnEnter: Start walking animation");
        })),
        on_exit: Some(Arc::new(|_state| {
            println!("OnExit: Stop walking animation");
        })),
    };

    // 7. 构建蓝图
    let mut blueprint = StateMachineBlueprint::new();
    blueprint.aspects.insert(action_aspect.id, action_aspect);
    blueprint.events.insert(press_w_event.id, press_w_event);
    blueprint.transitions.push(transition);
    blueprint.observers.push(walking_observer);

    // 8. 初始状态
    let initial_state: State = {
        let mut s = State::new();
        s.insert(1, Arc::new(Action::Idle));
        s
    };

    // 9. 创建运行时状态机
    RuntimeStateMachine::new(blueprint, initial_state)
}

/// 运行玩家移动示例
pub fn run_player_movement_example() {
    println!("=== 玩家移动示例 ===");
    
    let mut runtime = create_player_movement_example();
    println!("初始状态: Idle");

    // 触发事件
    runtime.event_happen(100, None);
    runtime.transform();

    // 检查状态
    if let Some(action) = runtime.current_state.get(&1).and_then(|v| v.downcast_ref::<Action>()) {
        println!("最终状态: {:?}", action);
    }
    
    println!("=== 示例结束 ===\n");
}