use crate::node::State::{Root, Strong, Unknown};
use crate::node::{Node, NodeHead, NodeTrait, State};
use crate::root_ref::RootRef;
use crate::target::Target;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::mem::{swap, take, transmute, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use State::Trace;

#[derive(Default, Copy, Clone, Debug)]
pub struct Config {
    pub pre_drop: bool,
    pub init_cap: usize,
}

#[inline(always)]
pub fn scope_gc<F: for<'gc, 's> FnOnce(Gc<'gc, 's>) -> R, R>(config: Config, f: F) -> R {
    let inner = RefCell::new(GcInner::new(config));
    f(Gc { inner: &inner })
}

#[derive(Copy, Clone, Debug)]
pub struct Gc<'gc, 's: 'gc> {
    inner: &'gc RefCell<GcInner<'gc, 's>>,
}

impl<'gc, 's: 'gc> Gc<'gc, 's> {
    #[inline(always)]
    pub fn new<T: Target + 's>(self, value: T) -> RootRef<'gc, Node<'gc, T>> {
        unsafe {
            let node = Node::new_in_box(value);
            let node_ref = transmute::<&'_ Node<'gc, T>, &'gc Node<'gc, T>>(node.deref());
            self.inner.borrow_mut().nodes.push(node.dyn_box());
            RootRef::new(node_ref)
        }
    }

    pub fn reserve(self, cap: usize) {
        self.inner.borrow_mut().nodes.reserve(cap);
    }

    pub fn clear(self) {
        unsafe {
            let mut inner = self.inner.borrow_mut();

            let mut stack = Vec::with_capacity(inner.nodes.capacity());
            stack.extend(inner.nodes.iter().filter_map(|r| {
                if r.root() != 0 {
                    NodeHead::from_node_trait(r.deref()).set_marker(Root);
                    Some(r.deref())
                } else {
                    NodeHead::from_node_trait(r.deref()).set_marker(Unknown);
                    None
                }
            }));

            let mut count = 0;
            while let Some(r) = stack.pop() {
                match NodeHead::from_node_trait(r).get_marker() {
                    Root | Trace => {
                        count += 1;
                        r.mark_and_collect(&mut stack);
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }

            let r = take(inner.nodes.deref_mut());

            if inner.config.pre_drop {
                for i in &r {
                    if NodeHead::from_node_trait(i.deref()).get_marker() == Unknown {
                        i.pre_drop();
                    }
                }
            }

            let mut new = Vec::with_capacity(count);
            new.extend(
                r.into_iter()
                    .filter(|x| NodeHead::from_node_trait(x.deref()).get_marker() != Unknown),
            );

            swap(inner.nodes.deref_mut(), &mut new);
        }
    }
}

struct GcInner<'gc, 's: 'gc> {
    config: Config,
    nodes: ManuallyDrop<Vec<Box<dyn NodeTrait<'gc> + 's>>>,
}

impl<'gc, 's> GcInner<'gc, 's> {
    fn new(config: Config) -> Self {
        Self {
            config,
            nodes: ManuallyDrop::new(Vec::with_capacity(config.init_cap)),
        }
    }
}

impl<'gc, 's> Debug for GcInner<'gc, 's> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcInner")
            .field("nodes", self.nodes.deref())
            .finish()
    }
}

unsafe impl<#[may_dangle] 'gc, 's: 'gc> Drop for GcInner<'gc, 's> {
    fn drop(&mut self) {
        unsafe {
            for node in self.nodes.deref() {
                node.pre_drop();
            }
            ManuallyDrop::drop(&mut self.nodes);
        }
    }
}
