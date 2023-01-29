use crate::node::NodeTrait;
use std::fmt::Debug;

/// 可以被 GC 管理的数据部分
pub trait Target {
    /// 对应的 GC 引用部分
    type RefObject<'gc>: RefSet<'gc>;

    /// 预析构函数
    #[inline(always)]
    unsafe fn pre_drop<'gc>(&self, _ref_set: &Self::RefObject<'gc>) {}
}

/// 对引用部分的要求
pub unsafe trait RefSet<'gc>: Debug {
    unsafe fn build() -> Self;
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>);
}
