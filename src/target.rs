use crate::node::{NodeHead, NodeTrait, State};
use crate::root_ref::RootRef;
use std::array::from_fn;
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ptr::NonNull;
use State::{Trace, Unknown};

/// 可以被 GC 管理的数据部分
pub trait Target {
    /// 对应的 GC 引用部分
    type RefObject<'gc>: RefSet<'gc>;

    /// 预析构函数
    unsafe fn pre_drop<'gc>(&self, _ref_set: &Self::RefObject<'gc>) {}
}

/// 对引用部分的要求
pub unsafe trait RefSet<'gc>: Debug {
    unsafe fn build() -> Self;
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State);
}

pub struct StrongRef<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> {
    _marker: PhantomData<*mut &'gc ()>,
    cell: Cell<Option<NonNull<T>>>,
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> StrongRef<'gc, T> {
    pub fn get(&self) -> Option<RootRef<'gc, T>> {
        self.cell.get().map(|r| unsafe { RootRef::new(r.as_ref()) })
    }

    pub fn set(&self, r: Option<&T>) {
        self.cell.set(r.map(Into::into));
    }

    pub fn set_ref(&self, r: &T) {
        self.cell.set(Some(NonNull::from(r)));
    }

    pub fn set_none(&self) {
        self.cell.set(None);
    }
}

impl<'gc, T: ?Sized + NodeTrait<'gc>> Debug for StrongRef<'gc, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_tuple("StrongRef");
        if let Some(r) = self.cell.get() {
            s.field(&r);
        } else {
            s.field(&None::<()>);
        }
        s.finish()
    }
}

unsafe impl<'gc, T: ?Sized + NodeTrait<'gc>> RefSet<'gc> for StrongRef<'gc, T> {
    unsafe fn build() -> Self {
        Self {
            _marker: PhantomData,
            cell: Cell::new(None),
        }
    }
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State) {
        if let Some(r) = self.cell.get() {
            let r = r.as_ref();
            if NodeHead::from_node_trait(r).get_marker() == Unknown {
                NodeHead::from_node_trait(r).set_marker(Trace);
                stack.push(r.as_dyn_node());
            }
        }
    }
}

unsafe impl<'gc, T: RefSet<'gc>, const N: usize> RefSet<'gc> for [T; N] {
    unsafe fn build() -> Self {
        from_fn(|_| T::build())
    }

    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State) {
        for i in self {
            i.collect(stack, max);
        }
    }
}

unsafe impl<'gc> RefSet<'gc> for () {
    unsafe fn build() -> Self {
        ()
    }

    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, _max: State) {}
}

unsafe impl<'gc, A: RefSet<'gc>> RefSet<'gc> for (A,) {
    unsafe fn build() -> Self {
        (A::build(),)
    }

    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State) {
        self.0.collect(stack, max);
    }
}

unsafe impl<'gc, A: RefSet<'gc>, B: RefSet<'gc>> RefSet<'gc> for (A, B) {
    unsafe fn build() -> Self {
        (A::build(), B::build())
    }

    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>, max: State) {
        self.0.collect(stack, max);
        self.1.collect(stack, max);
    }
}
