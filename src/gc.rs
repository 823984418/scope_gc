use crate::node::State::{Trace, Unknown};
use crate::node::{Node, NodeHead, NodeTrait};
use crate::root_ref::RootRef;
use crate::target::Target;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::mem::{swap, take, transmute};
use std::ops::Deref;
use std::ptr::NonNull;

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub pre_drop: bool,
    pub init_cap: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pre_drop: false,
            init_cap: 32,
        }
    }
}

#[inline(always)]
pub fn scope_gc<'s, F: for<'gc> FnOnce(Gc<'gc, 's>) -> R, R>(config: Config, f: F) -> R {
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
            let node = Box::new(Node::new(value));
            let node_ref = transmute::<&'_ Node<'gc, T>, &'gc Node<'gc, T>>(node.deref());
            self.inner
                .borrow_mut()
                .nodes
                .push(NonNull::new_unchecked(Box::into_raw(unsafe { transmute::<Box<dyn NodeTrait<'gc> + 'gc>, Box<dyn NodeTrait<'gc> + 's>>(node) })));
            RootRef::new(node_ref)
        }
    }

    pub fn reserve(self, cap: usize) {
        self.inner.borrow_mut().nodes.reserve(cap);
    }

    pub fn clear(self) {
        unsafe {
            let mut inner = self.inner.borrow_mut();

            let mut stack: Vec<&(dyn NodeTrait<'gc> + 'gc)> = Vec::with_capacity(inner.nodes.len());
            stack.extend(inner.nodes.iter().filter_map(|&r| {
                if r.as_ref().root() != 0 {
                    NodeHead::from_node_trait(r.as_ref()).set_marker(Trace);
                    Some(r.as_ref())
                } else {
                    NodeHead::from_node_trait(r.as_ref()).set_marker(Unknown);
                    None
                }
            }));

            let mut count = 0;
            while let Some(r) = stack.pop() {
                match NodeHead::from_node_trait(r).get_marker() {
                    Trace => {
                        count += 1;
                        r.mark_and_collect(&mut stack);
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }

            let r = take(&mut inner.nodes);

            if inner.config.pre_drop {
                for &i in &r {
                    if NodeHead::from_node_trait(i.as_ref()).get_marker() == Unknown {
                        i.as_ref().pre_drop();
                    }
                }
            }

            let mut new = Vec::with_capacity(count + inner.config.init_cap);
            new.extend(r.into_iter().filter_map(|x| {
                if NodeHead::from_node_trait(x.as_ref()).get_marker() != Unknown {
                    Some(x)
                } else {
                    drop(Box::from_raw(x.as_ptr()));
                    None
                }
            }));

            debug_assert_eq!(count, new.len());

            swap(&mut inner.nodes, &mut new);
        }
    }
}

struct GcInner<'gc, 's: 'gc> {
    config: Config,
    nodes: Vec<NonNull<dyn NodeTrait<'gc> + 's>>,
}

impl<'gc, 's> GcInner<'gc, 's> {
    fn new(config: Config) -> Self {
        Self {
            config,
            nodes: Vec::with_capacity(config.init_cap),
        }
    }
}

impl<'gc, 's> Debug for GcInner<'gc, 's> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        struct N<'n, 'gc, 's: 'gc>(&'n (dyn NodeTrait<'gc> + 's));
        impl<'n, 'gc, 's: 'gc> Debug for N<'n, 'gc, 's> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
        struct D<'n, 'gc, 's: 'gc>(&'n Vec<NonNull<dyn NodeTrait<'gc> + 's>>);
        impl<'n, 'gc, 's: 'gc> Debug for D<'n, 'gc, 's> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let mut n = f.debug_list();
                for i in self.0 {
                    unsafe {
                        n.entry(&N(i.as_ref()));
                    }
                }
                n.finish()
            }
        }
        let mut s = f.debug_struct("GcInner");
        s.field("config", &self.config);
        s.field("nodes", &D(&self.nodes));
        s.finish()
    }
}

unsafe impl<#[may_dangle] 'gc, 's: 'gc> Drop for GcInner<'gc, 's> {
    fn drop(&mut self) {
        unsafe {
            if self.config.pre_drop {
                for node in self.nodes.iter() {
                    node.as_ref().pre_drop();
                }
            }
            for node in self.nodes.iter() {
                drop(Box::from_raw(node.as_ptr()));
            }
        }
    }
}
