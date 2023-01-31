//! # 使用
//!
//! 要使得类型可以被 GC 管理，需要为其实现 `scope_gc::target::Target`
//!
//! `RefObject` 指的是与该类型匹配的引用集合，一旦值进入 GC 系统，我们就会在内部为其创建一个 `RefObject`  
//! 只有 `RefObject` 可以拥有指向被管理对象的引用，并且会追踪自身拥有的全部引用
//!
//! `pre_drop` 将在 GC 确定要销毁，但还没有执行销毁时调用，此时，`RefObject` 持有的引用依然有效
//! `pre_drop` 过程不可逆，因此，任何试图修改引用结构以使得引用增加的行为都是未定义的
//! 在销毁之前 `pre_drop` 会保证被调用
//!
//! 当然，为值实现 `Drop` 也是可行的，并且同样保证调用，但此时已经无法访问 GC 引用了
//!
//! 将值加入 GC 后，可以得到一个对应类型的 `RootRef<'gc, Node<'gc, T>>`  
//! `Node<'gc, T>` 是 `RefObject<'gc>` 和 `T` 类型的值的总和  
//! 因此，你可以通过定义自己的 trait 来为 `Node<'gc, T>` 添加行为
//!
//! `GC::new` 只能接受存活时间长于闭包的值
//! 使用 `GC::forget` 接受存活时间较短的值，但执行回收时仅仅回收内存，其预析构和析构函数均不会被调用  
//! 使用 `unsafe GC::dangling` 接受存活时间较短的值，且执行与 `GC::new` 同样的逻辑  
//! 使用 `GC::new_raw(x)` 其等价于 `GC::new(RawType(x))`，不过不推荐如此，在此情况下，使用来自 `Rc` 无疑是更好的选择
//!

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
