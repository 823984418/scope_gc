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
            pre_drop: true,
            ..Default::default()
        };
        scope_gc(config, |gc: Gc| {
            let x = gc.new(A {});
            let y = gc.new(A {});
            x.ref_set().set_ref(y.deref());
            y.ref_set().set_ref(x.deref());
            println!("{:#?}", gc);
            drop(x);
            drop(y);
            gc.clear();

            let p1 = Instant::now();

            for _ in 0..10000000 {
                gc.new(A {});
            }

            let p2 = Instant::now();
            gc.clear();
            let p3 = Instant::now();
            println!("alloc {:.4?}", p2 - p1);
            println!("clear {:.4?}", p3 - p2);
            println!("{:#?}", gc);
        });
    }
}
