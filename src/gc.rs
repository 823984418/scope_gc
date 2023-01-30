use crate::node::State::{Strong, Trace, Unknown};
use crate::node::{Node, NodeHead, NodeTrait};
use crate::raw_type::RawType;
use crate::root_ref::RootRef;
use crate::target::Target;
use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::mem::{swap, take, transmute, ManuallyDrop};
use std::ops::Deref;
use std::ptr::NonNull;

/// 初始化 GC 多使用的配置项
#[derive(Copy, Clone, Debug)]
pub struct Config {
    /// 是否执行预销毁
    pub pre_drop: bool,

    /// 初始化和清理后保持对象的剩余容量
    pub init_cap: usize,

    /// 初始化和清理后保持遗忘对象的剩余容量
    pub forget_cap: usize,

    /// 追踪可达性时使用的预分配栈大小因子
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

/// 创建并使用一个 GC
///
#[inline(always)]
pub fn scope_gc<'s, F: for<'gc> FnOnce(Gc<'gc, 's>) -> R, R>(config: Config, f: F) -> R {
    unsafe {
        let inner = RefCell::new(GcInner::new(config));
        let r = f(Gc { inner: &inner });
        inner.borrow_mut().clear_all();
        r
    }
}

/// 代表一个可以用于控制 GC 的句柄
///
/// # 生命周期
///
/// `'gc` 从中托管所能得到的借用长度，并且唯一标识一个 GC 来源
///
/// `'s` 安全托管所要求的值的存活时间，参见 [`Gc::new`]
///
#[derive(Copy, Clone, Debug)]
pub struct Gc<'gc, 's: 'gc> {
    inner: &'gc RefCell<GcInner<'gc, 's>>,
}

impl<'gc, 's: 'gc> Gc<'gc, 's> {
    /// 托管一个值，该值必须实现 [`Target`]，并且存活时间久于 `'s`
    ///
    pub fn new<T: Target + 's>(self, value: T) -> RootRef<'gc, Node<'gc, T>> {
        unsafe { self.dangling(value) }
    }

    /// 托管一个值，该值必须实现 [`Target`]，但不要求值的存活时间
    ///
    /// # 安全
    ///
    /// `T` 的所有生命周期参数在执行 [`Target::pre_drop`] 和 [`Drop::drop`] 时允许悬空
    ///
    pub unsafe fn dangling<T: Target>(self, value: T) -> RootRef<'gc, Node<'gc, T>> {
        let node = Box::new(Node::new(value));
        let node_ref = transmute::<&'_ Node<'gc, T>, &'gc Node<'gc, T>>(node.deref());
        self.inner
            .borrow_mut()
            .nodes
            .push(NonNull::new_unchecked(Box::into_raw(transmute::<
                Box<dyn NodeTrait<'gc> + 'gc>,
                Box<dyn NodeTrait<'gc> + 's>,
            >(node))));
        RootRef::new(node_ref)
    }

    /// 托管一个值，该值必须实现 [`Target`]，但不要求值的存活时间
    ///
    /// 执行回收时仅仅回收内存，其预析构和析构函数均不会被调用
    ///
    /// 其行为就好像是一旦值不可达，立即调用 [`std::mem::forget`] 将值遗忘
    ///
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

    /// 使用 [`RawType`] 包裹并调用 [`Gc::new`]
    ///
    /// [`RawType`] 将 [`Target`] 的 `RefObject` 实现为 `()`，因此不具有引用其他被管理对象的能力
    ///
    /// 优先考虑使用 [`std::rc::Rc`]
    ///
    pub fn new_raw<T: 's>(self, value: T) -> RootRef<'gc, Node<'gc, RawType<T>>> {
        self.new(RawType(value))
    }

    /// 使用 [`RawType`] 包裹并调用 [`Gc::dangling`]
    ///
    /// 优先考虑使用 [`std::rc::Rc`]
    ///
    /// # 安全
    ///
    /// `T` 的所有生命周期参数在执行 [`Drop::drop`] 时允许悬空
    ///
    pub unsafe fn dangling_raw<T>(self, value: T) -> RootRef<'gc, Node<'gc, RawType<T>>> {
        self.dangling(RawType(value))
    }

    /// 使用 [`RawType`] 包裹并调用 [`Gc::forget`]
    ///
    /// 优先考虑使用 [`std::rc::Rc`]
    ///
    pub fn forget_raw<T>(self, value: T) -> RootRef<'gc, Node<'gc, RawType<T>>> {
        self.forget(RawType(value))
    }

    /// 确保剩余容量大于 `cap`
    ///
    pub fn reserve(self, cap: usize) {
        self.inner.borrow_mut().nodes.reserve(cap);
    }

    /// 确保剩余用于储存 [`Gc::forget`] 的容量大于 `cap`
    ///
    pub fn reserve_forgets(self, cap: usize) {
        self.inner.borrow_mut().forgets.reserve(cap);
    }

    /// 获取当前管理的对象数
    ///
    pub fn get_node_count(self) -> usize {
        self.inner.borrow().nodes.len()
    }

    /// 获取当前管理的 [`Gc::forget`] 对象数
    ///
    pub fn get_forget_count(self) -> usize {
        self.inner.borrow().forgets.len()
    }

    /// 执行标记清扫，回收不可达对象
    ///
    /// 如果不可达对象以 [`Gc::new`] 和 [`Gc::dangling`] 方式加入，会统一在销毁前调用 [`Target::pre_drop`]，然后调用 [`Drop::drop`] 并回收内存
    ///
    /// 如果不可达对象以 [`Gc::forget`] 方式加入，则仅仅回收内存
    ///
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
    _marker: PhantomData<*mut &'gc ()>,
    config: Config,
    nodes: Vec<NonNull<dyn NodeTrait<'gc> + 's>>,
    forgets: Vec<NonNull<ManuallyDrop<dyn NodeTrait<'gc> + 's>>>,
}

impl<'gc, 's> GcInner<'gc, 's> {
    unsafe fn new(config: Config) -> Self {
        Self {
            _marker: PhantomData,
            config,
            nodes: Vec::with_capacity(config.init_cap),
            forgets: Vec::with_capacity(config.forget_cap),
        }
    }
    unsafe fn clear_all(&mut self) {
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
        self.nodes.clear();
        self.forgets.clear();
    }
}

impl<'gc, 's> Debug for GcInner<'gc, 's> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("GcInner");
        s.field("config", &self.config);
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
        s.field("nodes", &D(&self.nodes));
        struct M<'n, 'gc, 's: 'gc>(&'n Vec<NonNull<ManuallyDrop<dyn NodeTrait<'gc> + 's>>>);
        impl<'n, 'gc, 's: 'gc> Debug for M<'n, 'gc, 's> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let mut n = f.debug_list();
                let p: &'n Vec<NonNull<ManuallyDrop<dyn NodeTrait<'gc> + 's>>> = self.0;
                for i in p {
                    unsafe {
                        n.entry(&N(i.as_ref().deref()));
                    }
                }
                n.finish()
            }
        }
        s.field("forgets", &M(&self.forgets));
        s.finish()
    }
}
