use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

// --- 复用 main.rs 中的类型定义 ---
pub type StateAspectId = u64;
pub type EventId = u64;
pub type TransitionId = u64;
pub type ObserverId = u64;
pub type State = HashMap<StateAspectId, Arc<dyn std::any::Any + Send + Sync>>;

#[derive(Clone)]
pub struct StateInRange {
    predicate: Arc<dyn Fn(&State) -> bool + Send + Sync>,
}

impl StateInRange {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&State) -> bool + 'static + Send + Sync,
    {
        Self {
            predicate: Arc::new(f),
        }
    }

    pub fn contains(&self, state: &State) -> bool {
        (self.predicate)(state)
    }

    pub fn not(self) -> Self {
        Self::new(move |s| !self.contains(s))
    }

    pub fn and(self, other: Self) -> Self {
        Self::new(move |s| self.contains(s) && other.contains(s))
    }
}

#[derive(Clone)]
pub struct Transfer {
    func: Arc<dyn Fn(&State) -> State + Send + Sync>,
}

impl Transfer {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&State) -> State + 'static + Send + Sync,
    {
        Self {
            func: Arc::new(f),
        }
    }

    pub fn apply(&self, state: &State) -> State {
        (self.func)(state)
    }
}

#[derive(Clone)]
pub struct EventDef {
    pub id: EventId,
    pub payload_type_id: TypeId,
}

#[derive(Clone)]
pub struct Transition {
    pub id: TransitionId,
    pub event_id: EventId,
    pub guard: StateInRange,
    pub transfer: Transfer,
    pub priority: i32,
    pub on_tran: Option<Arc<dyn Fn(&State, &State) + Send + Sync>>,
}

#[derive(Clone)]
pub struct StateObserver {
    pub id: ObserverId,
    pub region: StateInRange,
    pub on_enter: Option<Arc<dyn Fn(&State) + Send + Sync>>,
    pub on_exit: Option<Arc<dyn Fn(&State) + Send + Sync>>,
}

#[derive(Clone)]
pub struct StateAspect {
    pub id: StateAspectId,
    pub value_type_id: TypeId,
}

#[derive(Clone)]
pub struct StateMachineBlueprint {
    pub aspects: HashMap<StateAspectId, StateAspect>,
    pub events: HashMap<EventId, EventDef>,
    pub transitions: Vec<Transition>,
    pub observers: Vec<StateObserver>,
}

impl StateMachineBlueprint {
    pub fn new() -> Self {
        Self {
            aspects: HashMap::new(),
            events: HashMap::new(),
            transitions: Vec::new(),
            observers: Vec::new(),
        }
    }

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

pub struct RuntimeStateMachine {
    pub blueprint: StateMachineBlueprint,
    pub current_state: State,
    pending_transition: Option<Transition>,
}

impl RuntimeStateMachine {
    pub fn new(blueprint: StateMachineBlueprint, initial_state: State) -> Self {
        Self {
            blueprint,
            current_state: initial_state,
            pending_transition: None,
        }
    }

    pub fn event_happen(&mut self, event_id: EventId, _payload: Option<Arc<dyn std::any::Any + Send + Sync>>) {
        let mut candidates: Vec<&Transition> = self
            .blueprint
            .transitions
            .iter()
            .filter(|t| t.event_id == event_id && t.guard.contains(&self.current_state))
            .collect();

        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.pending_transition = candidates.first().cloned().cloned();
    }

    pub fn transform(&mut self) {
        if let Some(transition) = self.pending_transition.take() {
            let next_state = transition.transfer.apply(&self.current_state);

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

    // 辅助方法：获取 Action 状态（用于测试）
    pub fn get_action(&self) -> Option<Action> {
        self.current_state
            .get(&1)
            .and_then(|v| v.downcast_ref::<Action>().cloned())
    }
    
    // 辅助方法：比较两个状态是否相等（用于测试）
    pub fn states_equal(&self, other: &State) -> bool {
        if self.current_state.len() != other.len() {
            return false;
        }
        
        for (key, value) in &self.current_state {
            match other.get(key) {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
enum Action {
    Idle,
    Walk,
}

// --- 测试用例 ---
#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_initial_state() {
        let (blueprint, initial_state) = create_player_blueprint();
        let runtime = RuntimeStateMachine::new(blueprint, initial_state);
        assert_eq!(runtime.get_action(), Some(Action::Idle));
    }

    #[test]
    fn test_transition_idle_to_walk() {
        let (blueprint, initial_state) = create_player_blueprint();
        let mut runtime = RuntimeStateMachine::new(blueprint, initial_state);

        // 触发 PressW
        runtime.event_happen(100, None);
        runtime.transform();

        assert_eq!(runtime.get_action(), Some(Action::Walk));
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

        assert_eq!(runtime.get_action(), Some(Action::Idle));
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

        // 状态不应改变 - 使用自定义比较方法
        assert!(runtime.states_equal(&prev_state));
    }

    #[test]
    fn test_observer_triggers() {
        let (mut blueprint, initial_state) = create_player_blueprint();

        // 添加带副作用的 observer
        let enter_triggered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let exit_triggered = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

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
        // --- Hunger 相关定义 ---
    const HUNGER_ASPECT_ID: StateAspectId = 2;

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

    // --- 辅助方法：获取 Hunger 状态 ---
    impl RuntimeStateMachine {
        pub fn get_hunger(&self) -> Option<i32> {
            self.current_state
                .get(&HUNGER_ASPECT_ID)
                .and_then(|v| v.downcast_ref::<i32>().copied())
        }
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
        assert_eq!(runtime.get_action(), Some(Action::Idle));
        assert_eq!(runtime.get_hunger(), Some(10));

        // 5. 触发行为事件：PressW → Walk
        runtime.event_happen(100, None);
        runtime.transform();
        assert_eq!(runtime.get_action(), Some(Action::Walk));
        assert_eq!(runtime.get_hunger(), Some(10)); // 饱食度不变

        // 6. 触发饱食度事件：Starve → 饱食度-1
        runtime.event_happen(201, None);
        runtime.transform();
        assert_eq!(runtime.get_action(), Some(Action::Walk)); // 行为不变
        assert_eq!(runtime.get_hunger(), Some(9));

        // 7. 再次触发行为事件：PressS → Idle
        runtime.event_happen(101, None);
        runtime.transform();
        assert_eq!(runtime.get_action(), Some(Action::Idle));
        assert_eq!(runtime.get_hunger(), Some(9));

        // 8. 触发 Eat → 饱食度+5
        runtime.event_happen(200, None);
        runtime.transform();
        assert_eq!(runtime.get_hunger(), Some(14));
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

        assert_eq!(runtime.get_hunger(), Some(4));
        assert!(hunger_enter_triggered.load(std::sync::atomic::Ordering::Relaxed));
    }
}