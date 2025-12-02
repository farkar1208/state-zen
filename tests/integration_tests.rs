//! State-Zen 状态机框架集成测试
//! 
//! 这些测试验证状态机框架的实际使用场景

use std::any::TypeId;
use std::sync::Arc;

// 使用项目中的库
use state_zen::{
    StateAspectId,
    StateAspect, StateInRange, Transfer, EventDef, Transition, StateObserver,
    StateMachineBlueprint, RuntimeStateMachine, State,
};

// 测试中使用的类型定义
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
enum Action {
    Idle,
    Walk,
}

// 辅助函数：创建玩家移动蓝图
fn create_player_blueprint() -> (StateMachineBlueprint, State) {
    let action_aspect = StateAspect {
        id: 1,
        value_type_id: TypeId::of::<Action>(),
    };

    let press_w_event = EventDef {
        id: 100,
        payload_type_id: TypeId::of::<()>(),
    };

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

    let press_w_to_walk = Transfer::new(|s| {
        let mut new_s = s.clone();
        new_s.insert(1, Arc::new(Action::Walk));
        new_s
    });

    let press_s_to_idle = Transfer::new(|s| {
        let mut new_s = s.clone();
        new_s.insert(1, Arc::new(Action::Idle));
        new_s
    });

    let mut blueprint = StateMachineBlueprint::new();
    blueprint.aspects.insert(action_aspect.id, action_aspect);
    blueprint.events.insert(press_w_event.id, press_w_event);

    // Walk transition
    blueprint.transitions.push(Transition {
        id: 1,
        event_id: 100,
        guard: is_idle.clone(),
        transfer: press_w_to_walk,
        priority: 0,
        on_tran: None,
    });

    // Idle transition
    let press_s_event = EventDef {
        id: 101,
        payload_type_id: TypeId::of::<()>(),
    };
    blueprint.events.insert(press_s_event.id, press_s_event);
    blueprint.transitions.push(Transition {
        id: 2,
        event_id: 101,
        guard: is_walking,
        transfer: press_s_to_idle,
        priority: 0,
        on_tran: None,
    });

    // Observer
    blueprint.observers.push(StateObserver {
        id: 1,
        region: StateInRange::new(|s| {
            s.get(&1)
                .and_then(|v| v.downcast_ref::<Action>())
                .map_or(false, |a| *a == Action::Walk)
        }),
        on_enter: None,
        on_exit: None,
    });

    let initial_state: State = {
        let mut s = State::new();
        s.insert(1, Arc::new(Action::Idle));
        s
    };

    (blueprint, initial_state)
}

// 辅助函数：获取 Action 状态
fn get_action(state: &State) -> Option<Action> {
    state
        .get(&1)
        .and_then(|v| v.downcast_ref::<Action>().cloned())
}

// 辅助函数：比较两个状态是否相等
fn states_equal(state1: &State, state2: &State) -> bool {
    if state1.len() != state2.len() {
        return false;
    }
    
    for (key, value) in state1 {
        match state2.get(key) {
            Some(other_value) => {
                // 尝试比较 Action 类型
                if let Some(action1) = value.downcast_ref::<Action>() {
                    if let Some(action2) = other_value.downcast_ref::<Action>() {
                        if action1 != action2 {
                            return false;
                        }
                        continue;
                    }
                }
                // 对于其他类型，暂时认为不相等
                return false;
            }
            None => return false,
        }
    }
    true
}

// --- 测试用例 ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let (blueprint, initial_state) = create_player_blueprint();
        let runtime = RuntimeStateMachine::new(blueprint, initial_state);
        assert_eq!(get_action(&runtime.current_state), Some(Action::Idle));
    }

    #[test]
    fn test_transition_idle_to_walk() {
        let (blueprint, initial_state) = create_player_blueprint();
        let mut runtime = RuntimeStateMachine::new(blueprint, initial_state);

        // 触发 PressW
        runtime.event_happen(100, None);
        runtime.transform();

        assert_eq!(get_action(&runtime.current_state), Some(Action::Walk));
    }

    #[test]
    fn test_transition_walk_to_idle() {
        let (blueprint, _) = create_player_blueprint();
        let mut runtime = RuntimeStateMachine::new(blueprint, {
            let mut s = State::new();
            s.insert(1, Arc::new(Action::Walk));
            s
        });

        // 触发 PressS
        runtime.event_happen(101, None);
        runtime.transform();

        assert_eq!(get_action(&runtime.current_state), Some(Action::Idle));
    }

    #[test]
    fn test_no_transition_when_guard_fails() {
        let (blueprint, _) = create_player_blueprint();
        let mut runtime = RuntimeStateMachine::new(blueprint, {
            let mut s = State::new();
            s.insert(1, Arc::new(Action::Walk));
            s
        });

        // 在 Walk 状态下触发 PressW（应无效）
        let prev_state = runtime.current_state.clone();
        runtime.event_happen(100, None);
        runtime.transform();

        // 状态不应改变
        assert!(states_equal(&runtime.current_state, &prev_state));
    }

    #[test]
    fn test_observer_triggers() {
        let (mut blueprint, initial_state) = create_player_blueprint();

        // 添加带副作用的 observer
        let enter_triggered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let exit_triggered = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let enter_flag = enter_triggered.clone();
        let exit_flag = exit_triggered.clone();

        blueprint.observers.push(StateObserver {
            id: 2,
            region: StateInRange::new(|s| {
                s.get(&1)
                    .and_then(|v| v.downcast_ref::<Action>())
                    .map_or(false, |a| *a == Action::Walk)
            }),
            on_enter: Some(Arc::new(move |_| {
                enter_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            })),
            on_exit: Some(Arc::new(move |_| {
                exit_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            })),
        });

        let mut runtime = RuntimeStateMachine::new(blueprint, initial_state);

        // Idle -> Walk
        runtime.event_happen(100, None);
        runtime.transform();
        assert!(enter_triggered.load(std::sync::atomic::Ordering::Relaxed));

        // Walk -> Idle
        runtime.event_happen(101, None);
        runtime.transform();
        assert!(exit_triggered.load(std::sync::atomic::Ordering::Relaxed));
    }
}

// --- 多维度状态测试 ---
#[cfg(test)]
mod multi_aspect_tests {
    use super::*;

    const HUNGER_ASPECT_ID: StateAspectId = 2;

    // 辅助函数：创建饥饿系统蓝图
    fn create_hunger_blueprint() -> (StateMachineBlueprint, State) {
        let hunger_aspect = StateAspect {
            id: HUNGER_ASPECT_ID,
            value_type_id: TypeId::of::<i32>(),
        };

        // 事件：吃东西（+5 饱食度）
        let eat_event = EventDef {
            id: 200,
            payload_type_id: TypeId::of::<()>(),
        };

        // 事件：饥饿（-1 饱食度）
        let starve_event = EventDef {
            id: 201,
            payload_type_id: TypeId::of::<()>(),
        };

        // 谓词：饥饿（<= 5）
        let is_hungry = StateInRange::new(|s| {
            s.get(&HUNGER_ASPECT_ID)
                .and_then(|v| v.downcast_ref::<i32>())
                .map_or(false, |h| *h <= 5)
        });

        // Transfer: 吃东西
        let eat_transfer = Transfer::new(|s| {
            let mut new_s = s.clone();
            let current = s
                .get(&HUNGER_ASPECT_ID)
                .and_then(|v| v.downcast_ref::<i32>())
                .copied()
                .unwrap_or(0);
            let new_hunger = (current + 5).min(20); // 上限 20
            new_s.insert(HUNGER_ASPECT_ID, Arc::new(new_hunger));
            new_s
        });

        // Transfer: 饥饿
        let starve_transfer = Transfer::new(|s| {
            let mut new_s = s.clone();
            let current = s
                .get(&HUNGER_ASPECT_ID)
                .and_then(|v| v.downcast_ref::<i32>())
                .copied()
                .unwrap_or(20);
            let new_hunger = (current - 1).max(0); // 下限 0
            new_s.insert(HUNGER_ASPECT_ID, Arc::new(new_hunger));
            new_s
        });

        let mut blueprint = StateMachineBlueprint::new();
        blueprint.aspects.insert(hunger_aspect.id, hunger_aspect);
        blueprint.events.insert(eat_event.id, eat_event);
        blueprint.events.insert(starve_event.id, starve_event);

        // Eat transition（任何状态都能吃）
        blueprint.transitions.push(Transition {
            id: 3,
            event_id: 200,
            guard: StateInRange::new(|_| true), // 通配
            transfer: eat_transfer,
            priority: 0,
            on_tran: None,
        });

        // Starve transition（任何状态都能饿）
        blueprint.transitions.push(Transition {
            id: 4,
            event_id: 201,
            guard: StateInRange::new(|_| true),
            transfer: starve_transfer,
            priority: 0,
            on_tran: None,
        });

        // Observer: 进入饥饿状态
        blueprint.observers.push(StateObserver {
            id: 3,
            region: is_hungry,
            on_enter: None,
            on_exit: None,
        });

        // 初始状态：饱食度 = 10
        let initial_state: State = {
            let mut s = State::new();
            s.insert(HUNGER_ASPECT_ID, Arc::new(10i32));
            s
        };

        (blueprint, initial_state)
    }

    // 辅助函数：获取 Hunger 状态
    fn get_hunger(state: &State) -> Option<i32> {
        state
            .get(&HUNGER_ASPECT_ID)
            .and_then(|v| v.downcast_ref::<i32>().copied())
    }

    #[test]
    fn test_blueprint_merge() {
        // 1. 创建两个独立蓝图
        let (action_bp, action_state) = create_player_blueprint();
        let (hunger_bp, hunger_state) = create_hunger_blueprint();

        // 2. 合并蓝图
        let merged_bp = action_bp.merge(&hunger_bp);

        // 3. 合并初始状态
        let mut initial_state = action_state;
        initial_state.extend(hunger_state);

        // 4. 创建运行时
        let mut runtime = RuntimeStateMachine::new(merged_bp, initial_state);

        // 验证初始状态
        assert_eq!(get_action(&runtime.current_state), Some(Action::Idle));
        assert_eq!(get_hunger(&runtime.current_state), Some(10));

        // 5. 触发行为事件：PressW → Walk
        runtime.event_happen(100, None);
        runtime.transform();
        assert_eq!(get_action(&runtime.current_state), Some(Action::Walk));
        assert_eq!(get_hunger(&runtime.current_state), Some(10)); // 饱食度不变

        // 6. 触发饱食度事件：Starve → 饱食度-1
        runtime.event_happen(201, None);
        runtime.transform();
        assert_eq!(get_action(&runtime.current_state), Some(Action::Walk)); // 行为不变
        assert_eq!(get_hunger(&runtime.current_state), Some(9));

        // 7. 再次触发行为事件：PressS → Idle
        runtime.event_happen(101, None);
        runtime.transform();
        assert_eq!(get_action(&runtime.current_state), Some(Action::Idle));
        assert_eq!(get_hunger(&runtime.current_state), Some(9));

        // 8. 触发 Eat → 饱食度+5
        runtime.event_happen(200, None);
        runtime.transform();
        assert_eq!(get_hunger(&runtime.current_state), Some(14));
    }

    #[test]
    fn test_observer_in_merged_blueprint() {
        let (action_bp, action_state) = create_player_blueprint();
        let (hunger_bp, hunger_state) = create_hunger_blueprint();

        // 添加饥饿 Observer（带副作用）
        let hunger_enter_triggered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = hunger_enter_triggered.clone();
        let mut hunger_bp_with_observer = hunger_bp.clone();
        hunger_bp_with_observer.observers.push(StateObserver {
            id: 4,
            region: StateInRange::new(|s| {
                s.get(&HUNGER_ASPECT_ID)
                    .and_then(|v| v.downcast_ref::<i32>())
                    .map_or(false, |h| *h <= 5)
            }),
            on_enter: Some(Arc::new(move |_| {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            })),
            on_exit: None,
        });

        let merged_bp = action_bp.merge(&hunger_bp_with_observer);
        let mut initial_state = action_state;
        initial_state.extend(hunger_state);

        let mut runtime = RuntimeStateMachine::new(merged_bp, initial_state);

        // 将饱食度降到 5 以下
        for _ in 0..6 {
            runtime.event_happen(201, None); // Starve 6 次: 10 → 4
            runtime.transform();
        }

        assert_eq!(get_hunger(&runtime.current_state), Some(4));
        assert!(hunger_enter_triggered.load(std::sync::atomic::Ordering::Relaxed));
    }
}