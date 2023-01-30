use crate::node::State::{Trace, Unknown};
use crate::target::{RefSet, Target};
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum State {
    /// 未知
    Unknown,

    /// 强可达
    Strong,

    /// 已追踪
    Trace,
}

pub struct NodeHead {
    root: Cell<usize>,
    marker: Cell<State>,
}

impl NodeHead {
    pub fn new() -> Self {
        Self {
            root: Cell::new(0),
            marker: Cell::new(Unknown),
        }
    }

    #[inline(always)]
    pub(crate) fn set_marker(&self, state: State) {
        self.marker.set(state);
    }

    #[inline(always)]
    pub(crate) fn get_marker(&self) -> State {
        self.marker.get()
    }

    #[inline(always)]
    pub(crate) fn inc_root(&self) {
        self.root.set(self.root.get() + 1);
    }

    #[inline(always)]
    pub(crate) fn dec_root(&self) {
        self.root.set(self.root.get() - 1);
    }

    #[inline(always)]
    pub(crate) fn from_node_trait<'s, 'gc, T: ?Sized + NodeTrait<'gc> + 's>(
        node: &'s T,
    ) -> &'s NodeHead {
        // # 安全
        //
        // [`NodeTrait`] 只被 [`Node`] 实现
        // [`Node`] 以 `C` 布局排列
        // `head` 是 [`Node`] 的第一个成员
        //
        unsafe { &*(node as *const T as *const Self) }
    }
}

#[repr(C)]
pub struct Node<'gc, T: Target> {
    head: NodeHead,
    pub ref_set: T::RefObject<'gc>,
    pub value: T,
}

impl<'gc, T: Target> Node<'gc, T> {
    #[inline(always)]
    pub fn ref_set(&self) -> &T::RefObject<'gc> {
        &self.ref_set
    }

    #[inline(always)]
    pub fn value(&self) -> &T {
        &self.value
    }

    pub(crate) unsafe fn new(value: T) -> Self {
        Self {
            head: NodeHead::new(),
            ref_set: T::RefObject::build(),
            value,
        }
    }
}

impl<'gc, T: Target> Deref for Node<'gc, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// 此特征唯一由 [`Node`] 实现
///
/// # 安全
///
/// 用户实现它总是不安全的
///
pub unsafe trait NodeTrait<'gc>: Debug {
    fn as_dyn_node(&self) -> &dyn NodeTrait<'gc>;

    fn head(&self) -> &NodeHead;

    fn root(&self) -> usize;

    unsafe fn mark_and_collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>);

    unsafe fn pre_drop(&self);
}

impl<'gc, T: Target> Debug for Node<'gc, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Node");
        s.field("#", &(self as *const _));
        s.field("root", &self.head.root.get());
        s.field("ref_set", &self.ref_set);
        s.finish()
    }
}

unsafe impl<'gc, T: Target> NodeTrait<'gc> for Node<'gc, T> {
    #[inline(always)]
    fn as_dyn_node(&self) -> &dyn NodeTrait<'gc> {
        self
    }

    #[inline(always)]
    fn head(&self) -> &NodeHead {
        &self.head
    }

    #[inline(always)]
    fn root(&self) -> usize {
        self.head.root.get()
    }

    /// 收集此对象的引用情况
    ///
    /// # 安全
    ///
    /// 用户调用总是不安全的
    ///
    #[inline(always)]
    unsafe fn mark_and_collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        match self.head.get_marker() {
            Trace => {
                self.head.marker.set(State::Strong);
                self.ref_set.collect(stack);
            }
            _ => {
                unreachable!();
            }
        }
    }

    /// 调用对象管理值的 [`Target::pre_drop`]
    ///
    /// # 安全
    ///
    /// 用户调用总是不安全的
    ///
    #[inline(always)]
    unsafe fn pre_drop(&self) {
        self.value.pre_drop(&self.ref_set);
    }
}
