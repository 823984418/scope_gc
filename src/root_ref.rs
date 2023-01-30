use crate::node::{NodeHead, NodeTrait};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;

/// 外部根引用
///
/// 通过指向对象的引用计数判断是否为根引用
///
pub struct RootRef<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> {
    _marker: PhantomData<*mut &'gc ()>,
    ptr: NonNull<T>,
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> RootRef<'gc, T> {
    pub fn new(r: &T) -> Self {
        NodeHead::from_node_trait(r).inc_root();
        Self {
            _marker: PhantomData,
            ptr: NonNull::from(r),
        }
    }
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> Drop for RootRef<'gc, T> {
    fn drop(&mut self) {
        NodeHead::from_node_trait(self.deref().deref()).dec_root();
    }
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> Clone for RootRef<'gc, T> {
    fn clone(&self) -> Self {
        Self::new(self)
    }
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> Debug for RootRef<'gc, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RootRef").field(&self.deref()).finish()
    }
}

impl<'gc, T: ?Sized + NodeTrait<'gc> + 'gc> Deref for RootRef<'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}
