use crate::node::State::{New, Trace, Unknown};
use crate::target::{RefSet, Target};
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::mem::transmute;
use State::Root;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum State {
    /// 未知/不可达
    Unknown,

    /// 新建
    New,

    /// 以追踪
    Trace,

    /// 强可达
    Strong,

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
            marker: Cell::new(New),
        }
    }

    pub unsafe fn set_marker(&self, state: State) {
        self.marker.set(state);
    }

    pub unsafe fn get_marker(&self) -> State {
        self.marker.get()
    }

    pub unsafe fn inc_root(&self) {
        self.root.set(self.root.get() + 1);
    }

    pub unsafe fn dec_root(&self) {
        self.root.set(self.root.get() - 1);
    }

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

    unsafe fn mark_and_collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State);

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
    fn as_dyn_node(&self) -> &dyn NodeTrait<'gc> {
        self
    }

    fn head(&self) -> &NodeHead {
        &self.head
    }

    fn root(&self) -> usize {
        self.head.root.get()
    }

    unsafe fn mark_and_collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State) {
        match self.head.marker.get() {
            Unknown => {
                unreachable!();
            }
            Trace => {
                self.head.marker.set(max);
                self.ref_set.collect(stack, max);
            }
            Root => {
                self.ref_set.collect(stack, max);
            }
            _ => {}
        }
    }

    unsafe fn pre_drop(&self) {
        self.value.pre_drop(&self.ref_set);
    }
}
