//! 状态方面定义

use std::any::TypeId;
use super::types::StateAspectId;

/// 状态方面
/// 表示状态的一个维度，有唯一的ID和值类型
#[derive(Clone)]
pub struct StateAspect {
    /// 方面的唯一标识符
    pub id: StateAspectId,
    /// 值类型的TypeId
    pub value_type_id: TypeId,
}