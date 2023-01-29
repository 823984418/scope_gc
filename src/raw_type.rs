use crate::target::Target;
use std::ops::{Deref, DerefMut};

/// 不会拥有附加引用集合包装
pub struct RawType<T: ?Sized>(pub T);

impl<T: ?Sized> Target for RawType<T> {
    type RefObject<'gc> = ();
}

impl<T: ?Sized> Deref for RawType<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for RawType<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
