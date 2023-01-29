use crate::node::State::{Trace, Unknown};
use crate::node::{NodeHead, NodeTrait};
use crate::root_ref::RootRef;
use crate::target::RefSet;
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct StrongRef<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> {
    _marker: PhantomData<*mut &'gc ()>,
    cell: Cell<Option<NonNull<T>>>,
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> StrongRef<'gc, T> {
    #[inline(always)]
    pub fn get(&self) -> Option<RootRef<'gc, T>> {
        self.cell.get().map(|r| unsafe { RootRef::new(r.as_ref()) })
    }

    #[inline(always)]
    pub fn set(&self, r: Option<&T>) {
        self.cell.set(r.map(Into::into));
    }

    #[inline(always)]
    pub fn set_ref(&self, r: &T) {
        self.cell.set(Some(NonNull::from(r)));
    }

    #[inline(always)]
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
    #[inline(always)]
    unsafe fn build() -> Self {
        Self {
            _marker: PhantomData,
            cell: Cell::new(None),
        }
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        if let Some(r) = self.cell.get() {
            let r = r.as_ref();
            if NodeHead::from_node_trait(r).get_marker() == Unknown {
                NodeHead::from_node_trait(r).set_marker(Trace);
                stack.push(r.as_dyn_node());
            }
        }
    }
}
