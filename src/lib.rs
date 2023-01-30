#![cfg_attr(feature = "_unsize", feature(unsize))]
#![cfg_attr(feature = "_coerce_unsized", feature(coerce_unsized))]

pub mod gc;
pub mod node;
pub mod raw_type;
pub mod root_ref;
pub mod strong_ref;
pub mod strong_vec;
pub mod struct_ref;
pub mod target;

#[cfg(test)]
mod tests {
    use crate::gc::{scope_gc, Config, Gc};
    use crate::node::{Node, NodeTrait};
    use crate::strong_ref::StrongRef;
    use crate::target::Target;
    use std::ops::Deref;
    use std::time::Instant;

    struct A<'n>(&'n i32);

    impl<'n> Drop for A<'n> {
        #[inline(always)]
        fn drop(&mut self) {
            // println!("drop A");
        }
    }

    trait NodeA<'gc>: NodeTrait<'gc> {
        fn inner(&self) -> &i32;
    }

    impl<'gc, 'n> NodeA<'gc> for Node<'gc, A<'n>> {
        fn inner(&self) -> &i32 {
            self.deref().0
        }
    }

    impl<'n> Target for A<'n> {
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
        let i = 1;
        scope_gc(config, |gc: Gc| {
            let x = gc.new(A(&i));
            let y = gc.new(A(&i));
            x.ref_set().set_ref(y.deref());
            y.ref_set().set_ref(x.deref());
            println!("{:#?}", gc);
            drop(x);
            drop(y);
            gc.clear();

            let p1 = Instant::now();

            for _ in 0..10000000 {
                gc.new(A(&i));
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
