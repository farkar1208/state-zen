//! State-Zen: 一个灵活的状态机框架
//! 
//! 这个库提供了一个通用的、事件驱动的状态机框架，支持多维度状态管理和观察者模式。

// 导出核心模块
pub mod core;
pub mod utils;
pub mod examples;

// 重新导出常用类型，方便用户使用
pub use core::{
    StateAspectId, EventId, TransitionId, ObserverId,
    StateAspect, StateInRange, Transfer, EventDef, Transition, StateObserver,
    StateMachineBlueprint, RuntimeStateMachine,
};

// 重新导出 State 类型
pub use core::runtime::State;