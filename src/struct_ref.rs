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
    unsafe fn build() -> Self {
        ()
    }

    #[inline(always)]
    unsafe fn collect(&self, _stack: &mut Vec<&dyn NodeTrait<'gc>>) {}
}

unsafe impl<'gc, A: RefSet<'gc>> RefSet<'gc> for (A,) {
    #[inline(always)]
    unsafe fn build() -> Self {
        (A::build(),)
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        self.0.collect(stack);
    }
}

unsafe impl<'gc, A: RefSet<'gc>, B: RefSet<'gc>> RefSet<'gc> for (A, B) {
    #[inline(always)]
    unsafe fn build() -> Self {
        (A::build(), B::build())
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        self.0.collect(stack);
        self.1.collect(stack);
    }
}

unsafe impl<'gc, A: RefSet<'gc>, B: RefSet<'gc>, C: RefSet<'gc>> RefSet<'gc> for (A, B, C) {
    #[inline(always)]
    unsafe fn build() -> Self {
        (A::build(), B::build(), C::build())
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        self.0.collect(stack);
        self.1.collect(stack);
        self.2.collect(stack);
    }
}

unsafe impl<'gc, A: RefSet<'gc>, B: RefSet<'gc>, C: RefSet<'gc>, D: RefSet<'gc>> RefSet<'gc>
    for (A, B, C, D)
{
    #[inline(always)]
    unsafe fn build() -> Self {
        (A::build(), B::build(), C::build(), D::build())
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        self.0.collect(stack);
        self.1.collect(stack);
        self.2.collect(stack);
        self.3.collect(stack);
    }
}

unsafe impl<'gc, A: RefSet<'gc>, B: RefSet<'gc>, C: RefSet<'gc>, D: RefSet<'gc>, E: RefSet<'gc>>
    RefSet<'gc> for (A, B, C, D, E)
{
    #[inline(always)]
    unsafe fn build() -> Self {
        (A::build(), B::build(), C::build(), D::build(), E::build())
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        self.0.collect(stack);
        self.1.collect(stack);
        self.2.collect(stack);
        self.3.collect(stack);
        self.4.collect(stack);
    }
}

unsafe impl<
        'gc,
        A: RefSet<'gc>,
        B: RefSet<'gc>,
        C: RefSet<'gc>,
        D: RefSet<'gc>,
        E: RefSet<'gc>,
        F: RefSet<'gc>,
    > RefSet<'gc> for (A, B, C, D, E, F)
{
    #[inline(always)]
    unsafe fn build() -> Self {
        (
            A::build(),
            B::build(),
            C::build(),
            D::build(),
            E::build(),
            F::build(),
        )
    }

    #[inline(always)]
    unsafe fn collect(&self, stack: &mut Vec<&dyn NodeTrait<'gc>>) {
        self.0.collect(stack);
        self.1.collect(stack);
        self.2.collect(stack);
        self.3.collect(stack);
        self.4.collect(stack);
        self.5.collect(stack);
    }
}
