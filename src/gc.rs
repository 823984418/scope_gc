use crate::node::State::{Strong, Trace, Unknown};
use crate::node::{Node, NodeHead, NodeTrait};
use crate::root_ref::RootRef;
use crate::target::Target;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::mem::{swap, take, transmute, ManuallyDrop};
use std::ops::Deref;
use std::ptr::NonNull;

#[derive(Copy, Clone, Debug)]
pub struct Config {
    pub pre_drop: bool,
    pub init_cap: usize,
    pub forget_cap: usize,
    pub stack_factor: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            pre_drop: false,
            init_cap: 32,
            forget_cap: 0,
            stack_factor: 0.1,
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
                .push(NonNull::new_unchecked(Box::into_raw(transmute::<
                    Box<dyn NodeTrait<'gc> + 'gc>,
                    Box<dyn NodeTrait<'gc> + 's>,
                >(
                    node
                ))));
            RootRef::new(node_ref)
        }
    }

    #[inline(always)]
    pub fn forget<T: Target>(self, value: T) -> RootRef<'gc, Node<'gc, T>> {
        unsafe {
            let node = Box::new(ManuallyDrop::new(Node::new(value)));
            let node_ref = transmute::<&'_ Node<'gc, T>, &'gc Node<'gc, T>>(node.deref());
            self.inner
                .borrow_mut()
                .forgets
                .push(NonNull::new_unchecked(Box::into_raw(transmute::<
                    Box<ManuallyDrop<dyn NodeTrait<'gc> + 'gc>>,
                    Box<ManuallyDrop<dyn NodeTrait<'gc> + 's>>,
                >(
                    node
                ))));
            RootRef::new(node_ref)
        }
    }

    pub fn reserve(self, cap: usize) {
        self.inner.borrow_mut().nodes.reserve(cap);
    }

    pub fn get_nodes(self) -> usize {
        self.inner.borrow().nodes.len()
    }
    pub fn get_forgets(self) -> usize {
        self.inner.borrow().forgets.len()
    }

    pub fn clear(self) {
        unsafe {
            let mut inner = self.inner.borrow_mut();

            let mut stack = Vec::with_capacity(
                ((inner.nodes.len() + inner.forgets.len()) as f32 * inner.config.stack_factor)
                    as usize,
            );

            for &r in inner.nodes.iter() {
                if r.as_ref().root() != 0 {
                    NodeHead::from_node_trait(r.as_ref()).set_marker(Trace);
                    stack.push(r.as_ref());
                } else {
                    NodeHead::from_node_trait(r.as_ref()).set_marker(Unknown);
                }
            }
            for &r in inner.forgets.iter() {
                if r.as_ref().root() != 0 {
                    NodeHead::from_node_trait(r.as_ref().deref()).set_marker(Trace);
                    stack.push(r.as_ref().deref())
                } else {
                    NodeHead::from_node_trait(r.as_ref().deref()).set_marker(Unknown);
                }
            }

            while let Some(r) = stack.pop() {
                match NodeHead::from_node_trait(r).get_marker() {
                    Trace => {
                        r.mark_and_collect(&mut stack);
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }

            let nodes = take(&mut inner.nodes);
            if inner.config.pre_drop {
                let mut drop_count = 0;
                for &i in nodes.iter() {
                    match NodeHead::from_node_trait(i.as_ref()).get_marker() {
                        Unknown => {
                            drop_count += 1;
                            i.as_ref().pre_drop();
                        }
                        Strong => {}
                        Trace => unreachable!(),
                    }
                }

                let mut new_nodes =
                    Vec::with_capacity(nodes.len() - drop_count + inner.config.init_cap);
                new_nodes.extend(nodes.into_iter().filter(|x| {
                    match NodeHead::from_node_trait(x.as_ref()).get_marker() {
                        Unknown => {
                            drop(Box::from_raw(x.as_ptr()));
                            false
                        }
                        Strong => true,
                        Trace => unreachable!(),
                    }
                }));

                swap(&mut inner.nodes, &mut new_nodes);
            } else {
                let mut new_nodes = nodes
                    .into_iter()
                    .filter(
                        |&i| match NodeHead::from_node_trait(i.as_ref()).get_marker() {
                            Unknown => false,
                            Strong => true,
                            Trace => unreachable!(),
                        },
                    )
                    .collect::<Vec<_>>();
                new_nodes.reserve(inner.config.init_cap);
                swap(&mut inner.nodes, &mut new_nodes);
            }

            let forgets = take(&mut inner.forgets);
            let mut new_forgets = forgets
                .into_iter()
                .filter(
                    |&i| match NodeHead::from_node_trait(i.as_ref().deref()).get_marker() {
                        Unknown => false,
                        Strong => true,
                        Trace => unreachable!(),
                    },
                )
                .collect::<Vec<_>>();
            new_forgets.reserve(inner.config.forget_cap);
            swap(&mut inner.forgets, &mut new_forgets);
        }
    }
}

struct GcInner<'gc, 's: 'gc> {
    config: Config,
    nodes: Vec<NonNull<dyn NodeTrait<'gc> + 's>>,
    forgets: Vec<NonNull<ManuallyDrop<dyn NodeTrait<'gc> + 's>>>,
}

impl<'gc, 's> GcInner<'gc, 's> {
    fn new(config: Config) -> Self {
        Self {
            config,
            nodes: Vec::with_capacity(config.init_cap),
            forgets: Vec::with_capacity(config.forget_cap),
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
            for node in self.forgets.iter() {
                drop(Box::from_raw(node.as_ptr()));
            }
        }
    }
}
