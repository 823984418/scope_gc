# 基于作用域的安全GC设计

## 使用

首先，假设你需要管理的类型为

```rust
struct A<'n>(&'n i32);
```

要使得它可以被 GC 管理，需要为其实现 `scope_gc::target`

```rust
impl<'n> Target for A<'n> {
    type RefObject<'gc> = StrongRef<'gc, dyn NodeA<'gc>>;

    unsafe fn pre_drop<'gc>(&self, _ref_set: &Self::RefObject<'gc>) {
        println!("pre-drop A");
    }
}
```

`RefObject` 指的是与该类型匹配的引用集合，一旦值进入 GC 系统，我们就会在内部为其创建一个 `RefObject`  
只有 `RefObject` 可以拥有指向被管理对象的引用，并且会追踪自身拥有的全部引用

`pre_drop` 将在 GC 确定要销毁，但还没有执行销毁时调用，此时，`RefObject` 持有的引用依然有效
`pre_drop` 过程不可逆，因此，任何试图修改引用结构以使得引用增加的行为都是未定义的
在销毁之前 `pre_drop` 会保证被调用

当然，为值实现 `Drop` 也是可行的，并且同样保证调用，但此时已经无法访问 GC 引用了

```rust
impl<'n> Drop for A<'n> {
    fn drop(&mut self) {
        println!("drop A");
    }
}
```

将值加入 GC 后，可以得到一个对应类型的 `RootRef<'gc, Node<'gc, T>>`  
`Node<'gc, T>` 是 `RefObject<'gc>` 和 `T` 类型的值的总和  
因此，你可以通过定义自己的 trait 来为 `Node<'gc, T>` 添加行为

```rust
trait NodeA<'gc>: NodeTrait<'gc> {
    fn inner(&self) -> &i32;
}

impl<'gc, 'n> NodeA<'gc> for Node<'gc, A<'n>> {
    fn inner(&self) -> &i32 {
        &self.deref().0
    }
}
```

这样，你就可以在 `RootRef<'gc, dyn NodeA<'gc>>` 中调用 `inner` 了

整体的用法类似于

```rust
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
        let _p = x.inner();
        println!("{:#?}", gc);
        gc.clear();
        println!("{:#?}", gc);
    });
}
```

`GC::new` 只能接受存活时间长于闭包的值，具体原因如后文所述
使用 `GC::forget` 接受存活时间较短的值，但执行回收时仅仅回收内存，其预析构和析构函数均不会被调用


## 在 Rust 中引入 GC 所存在的问题

### 销毁问题

用户可以自行实现 `std::ops::Drop` 来为自定义类型制定销毁动作

```rust
struct A<'s>(&'s i32);

impl Drop for A {
    fn drop(&mut self) {
        println!("drop A({})", self.0);
    }
}
```

如果简单的在栈上创建值，编译器会确保 `drop` 在合适的时刻被调用，然而，复杂的内存管理策略却无法保证调用是及时的  
例如，如果使用 `std::rc::Rc` 来管理 `A<'s>`，并且创建了一个循环引用，值将会存活的比 `'s` 还久  
倘若我们错过了 `drop` 的合理调用时刻，在此之后 `'s` 进入悬空状态，被其修饰的内容都是悬空的，调用 `drop` 就可能导致未定义的行为




