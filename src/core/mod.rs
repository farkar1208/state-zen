//! 核心状态机框架模块

// 子模块
pub mod types;
pub mod state_aspect;
pub mod state_in_range;
pub mod transfer;
pub mod event;
pub mod transition;
pub mod state_observer;
pub mod blueprint;
pub mod runtime;

// 重新导出常用类型
pub use types::*;
pub use state_aspect::StateAspect;
pub use state_in_range::StateInRange;
pub use transfer::Transfer;
pub use event::EventDef;
pub use transition::Transition;
pub use state_observer::StateObserver;
pub use blueprint::StateMachineBlueprint;
pub use runtime::{RuntimeStateMachine, State};