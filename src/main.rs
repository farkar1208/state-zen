use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

// --- 类型别名 ---
pub type StateAspectId = u64;
pub type EventId = u64;
pub type TransitionId = u64;
pub type ObserverId = u64;

// 运行时状态：aspect_id -> Arc<dyn Any>
pub type State = HashMap<StateAspectId, Arc<dyn std::any::Any>>;

// --- StateInRange ---
#[derive(Clone)]
pub struct StateInRange {
    predicate: Arc<dyn Fn(&State) -> bool + 'static + Send + Sync>,
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

// --- Transfer ---
#[derive(Clone)]
pub struct Transfer {
    func: Arc<dyn Fn(&State) -> State + 'static + Send + Sync>,
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

// --- EventDef ---
#[derive(Clone)]
pub struct EventDef {
    pub id: EventId,
    pub payload_type_id: TypeId,
}

// --- Transition ---
#[derive(Clone)]
pub struct Transition {
    pub id: TransitionId,
    pub event_id: EventId,
    pub guard: StateInRange,
    pub transfer: Transfer,
    pub priority: i32,
    pub on_tran: Option<Arc<dyn Fn(&State, &State) + Send + Sync>>,
}

// --- StateObserver ---
#[derive(Clone)]
pub struct StateObserver {
    pub id: ObserverId,
    pub region: StateInRange,
    pub on_enter: Option<Arc<dyn Fn(&State) + Send + Sync>>,
    pub on_exit: Option<Arc<dyn Fn(&State) + Send + Sync>>,
}
// --- StateAspect ---
#[derive(Clone)]
pub struct StateAspect {
    pub id: StateAspectId,
    pub value_type_id: TypeId,
}


// --- StateMachineBlueprint ---
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

// --- 工具函数 ---
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

// --- 运行时状态机 ---
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

    // 领域事件 1: EventHappen
    pub fn event_happen(&mut self, event_id: EventId, _payload: Option<Box<dyn std::any::Any>>) {
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

    // 领域事件 2: Transform
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

// --- 示例：玩家移动 ---
#[derive(Debug, Clone, PartialEq)]
enum Action {
    Idle,
    Walk,
}

fn main() {
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

    // 9. 运行时
    let mut runtime = RuntimeStateMachine::new(blueprint, initial_state);

    println!("Initial state: Idle");

    // 触发事件
    runtime.event_happen(100, None);
    runtime.transform();

    // 检查状态
    if let Some(action) = runtime.current_state.get(&1).and_then(|v| v.downcast_ref::<Action>()) {
        println!("Final state: {:?}", action);
    }
}