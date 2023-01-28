use crate::node::State::{Trace, Unknown};
use crate::target::{RefSet, Target};
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::mem::transmute;
use State::Root;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum State {
    /// 未知/不可达
    Unknown,

    /// 强可达
    Strong,

    /// 已追踪
    Trace,

    /// 引用根
    Root,
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
    pub unsafe fn set_marker(&self, state: State) {
        self.marker.set(state);
    }

    #[inline(always)]
    pub unsafe fn get_marker(&self) -> State {
        self.marker.get()
    }

    #[inline(always)]
    pub unsafe fn inc_root(&self) {
        self.root.set(self.root.get() + 1);
    }

    #[inline(always)]
    pub unsafe fn dec_root(&self) {
        self.root.set(self.root.get() - 1);
    }

    #[inline(always)]
    pub fn from_node_trait<'s, 'gc, T: ?Sized + NodeTrait<'gc> + 's>(node: &'s T) -> &'s NodeHead {
        unsafe { &*(node as *const T as *const Self) }
    }
}

pub struct Node<'gc, T: Target> {
    head: NodeHead,
    ref_set: T::RefObject<'gc>,
    value: T,
}

impl<'gc, T: Target> Node<'gc, T> {
    #[inline(always)]
    pub fn ref_set(&self) -> &T::RefObject<'gc> {
        &self.ref_set
    }

    #[inline(always)]
    pub(crate) unsafe fn new_in_box(value: T) -> Box<Self> {
        Box::new(Self {
            head: NodeHead::new(),
            ref_set: unsafe { T::RefObject::build() },
            value,
        })
    }

    #[inline(always)]
    pub(crate) fn dyn_box<'s: 'gc>(self: Box<Self>) -> Box<dyn NodeTrait<'gc> + 's>
    where
        T: 's,
    {
        unsafe { transmute::<Box<dyn NodeTrait<'gc> + 'gc>, Box<dyn NodeTrait<'gc> + 's>>(self) }
    }
}

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
        s.field("last_marker", &self.head.marker.get());
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

    #[inline(always)]
    unsafe fn mark_and_collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        match self.head.get_marker() {
            Trace => {
                self.head.marker.set(State::Strong);
                self.ref_set.collect(stack);
            }
            Root => {
                self.ref_set.collect(stack);
            }
            _ => {
                unreachable!();
            }
        }
    }

    #[inline(always)]
    unsafe fn pre_drop(&self) {
        self.value.pre_drop(&self.ref_set);
    }
}
