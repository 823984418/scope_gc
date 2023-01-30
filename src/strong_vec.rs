use crate::node::State::{Trace, Unknown};
use crate::node::{NodeHead, NodeTrait};
use crate::root_ref::RootRef;
use crate::target::RefSet;
use std::cell::{Cell, RefCell};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ptr::NonNull;


/// 一个边长的强引用数组
/// 
pub struct StrongVec<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> {
    _marker: PhantomData<*mut &'gc ()>,
    vec: RefCell<Vec<Cell<NonNull<T>>>>,
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> StrongVec<'gc, T> {
    #[inline(always)]
    pub fn get(&self, index: usize) -> Result<RootRef<'gc, T>, ()> {
        self.vec
            .borrow()
            .get(index)
            .map(|i| RootRef::new(unsafe { i.get().as_ref() }))
            .ok_or(())
    }

    #[inline(always)]
    pub fn set(&self, index: usize, r: &T) -> Result<(), ()> {
        self.vec
            .borrow()
            .get(index)
            .map(|i| i.set(NonNull::from(r)))
            .ok_or(())
    }

    #[inline(always)]
    pub fn push(&self, r: &T) {
        self.vec.borrow_mut().push(Cell::new(NonNull::from(r)));
    }

    pub fn extend<'s, I: IntoIterator<Item = &'s T>>(&self, i: I)
    where
        'gc: 's,
    {
        self.vec
            .borrow_mut()
            .extend(i.into_iter().map(|i| Cell::new(NonNull::from(i))))
    }

    pub fn get_all<B: FromIterator<RootRef<'gc, T>>>(&self) -> B {
        self.vec
            .borrow()
            .iter()
            .map(|i| RootRef::new(unsafe { i.get().as_ref() }))
            .collect()
    }
}

impl<'gc, T: ?Sized + NodeTrait<'gc>> Debug for StrongVec<'gc, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_list();
        s.entries(self.vec.borrow().iter().map(|i| i.get()));
        s.finish()
    }
}

unsafe impl<'gc, T: ?Sized + NodeTrait<'gc>> RefSet<'gc> for StrongVec<'gc, T> {
    #[inline(always)]
    unsafe fn build() -> Self {
        Self {
            _marker: PhantomData,
            vec: Default::default(),
        }
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        for i in self.vec.borrow().iter() {
            let r = i.get().as_ref();
            if NodeHead::from_node_trait(r).get_marker() == Unknown {
                NodeHead::from_node_trait(r).set_marker(Trace);
                stack.push(r.as_dyn_node());
            }
        }
    }
}
