//! 事件定义

use std::any::TypeId;
use super::types::EventId;

/// 事件定义
/// 包含事件ID和payload类型信息
#[derive(Clone)]
pub struct EventDef {
    /// 事件的唯一标识符
    pub id: EventId,
    /// payload类型的TypeId
    pub payload_type_id: TypeId,
}