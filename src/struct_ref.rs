use crate::node::NodeTrait;
use crate::target::RefSet;
use std::array::from_fn;

unsafe impl<'gc, T: RefSet<'gc>, const N: usize> RefSet<'gc> for [T; N] {
    #[inline(always)]
    unsafe fn build() -> Self {
        from_fn(|_| T::build())
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        for i in self {
            i.collect(stack);
        }
    }
}

unsafe impl<'gc> RefSet<'gc> for () {
    #[inline(always)]
    unsafe fn build() -> Self {}

    #[inline(always)]
    unsafe fn collect(&self, _stack: &mut Vec<&dyn NodeTrait<'gc>>) {}
}

macro_rules! impl_ref_set_for_tuple {
    ($($T:ident:$i:tt),*) => {
        unsafe impl<'gc, $($T: RefSet<'gc>),*> RefSet<'gc> for ($($T,)*) {
            #[inline(always)]
            unsafe fn build() -> Self {
                ($($T::build(),)*)
            }

            #[inline(always)]
            unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
                $(self.$i.collect(stack);)*
            }
        }
    };
}

impl_ref_set_for_tuple!(A:0);
impl_ref_set_for_tuple!(A:0, B:1);
impl_ref_set_for_tuple!(A:0, B:1, C:2);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10);
impl_ref_set_for_tuple!(A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11);
