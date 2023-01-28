#![feature(unsize)]
#![feature(coerce_unsized)]
#![feature(dropck_eyepatch)]

pub mod gc;
pub mod node;
pub mod root_ref;
pub mod target;

#[cfg(test)]
mod tests {
    use crate::gc::{scope_gc, Config, Gc};
    use crate::node::{Node, NodeTrait};
    use crate::target::{StrongRef, Target};
    use std::ops::Deref;
    use std::time::Instant;

    struct A {}

    impl Drop for A {
        #[inline(always)]
        fn drop(&mut self) {
            // println!("drop A");
        }
    }

    trait NodeA<'gc>: NodeTrait<'gc> {}

    impl<'gc> NodeA<'gc> for Node<'gc, A> {}

    impl Target for A {
        type RefObject<'gc> = StrongRef<'gc, dyn NodeA<'gc>>;
        #[inline(always)]
        unsafe fn pre_drop<'gc>(&self, _ref_set: &Self::RefObject<'gc>) {
            // println!("pre-drop A");
        }
    }

    #[test]
    fn test() {
        let config = Config {
            pre_drop: false,
            ..Default::default()
        };
        scope_gc(config, |gc: Gc| {
            let p1 = Instant::now();

            gc.reserve(10000000);

            let p2 = Instant::now();

            for i in 0..1000000 {
                gc.new(A {});
            }

            let p3 = Instant::now();

            gc.clear();

            let p4 = Instant::now();

            println!("{:#?}", gc);
            println!("reserve: {:.4?}", p2 - p1);
            println!("alloc: {:.4?}", p3 - p2);
            println!("clear: {:.4?}", p4 - p3);
        });
    }
}
