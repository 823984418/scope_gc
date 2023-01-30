use crate::node::NodeTrait;
use std::fmt::Debug;

/// 可以被 GC 管理的数据部分
pub trait Target {
    /// 对应的 GC 引用部分
    type RefObject<'gc>: RefSet<'gc>;

    /// 预析构函数
    /// 
    /// 这将会在清理时由 GC 调用
    /// 
    /// # 安全
    /// 
    /// 用户不得调用
    /// 
    /// 实现必须保证期间任何引用对象持有的引用不会引起对象复活
    /// 
    #[inline(always)]
    unsafe fn pre_drop<'gc>(&self, _ref_set: &Self::RefObject<'gc>) {}
}

/// 对引用部分的要求
///
/// # 安全
///
/// 值的声明周期在 GC 内部被延长至稍长于 `'gc`，因此类型必须允许 `'gc` 在调用时悬空
///
/// 确保外部根引用表现正确
/// * 实现不得直接泄漏引用，外界获取引用通过 [`crate::root_ref::RootRef`]
/// * 或者使用类似 [`std::cell::RefCell::borrow_mut`] 手段提供
///
///
pub unsafe trait RefSet<'gc>: Debug {
    ///
    /// 构造一个此类型的值
    ///
    /// 这将会在托管对象时由 GC 调用
    ///
    /// # 安全
    ///
    /// 用户不得调用
    ///
    unsafe fn build() -> Self;

    ///
    /// 追踪内部引用
    ///
    /// 这将会在清理时由 GC 调用
    ///
    /// # 安全
    ///
    /// 实现必须不重不漏的遍历内部引用情况
    ///
    /// 用户不得调用
    ///
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>);
}
