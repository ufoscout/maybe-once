# maybe-once

![crates.io](https://img.shields.io/crates/v/maybe-once.svg)
![Build Status](https://github.com/ufoscout/maybe-once/actions/workflows/build_and_test.yml/badge.svg)
[![codecov](https://codecov.io/gh/ufoscout/maybe-once/branch/master/graph/badge.svg)](https://codecov.io/gh/ufoscout/maybe-once)

# What is this?

`maybe-once` offers a variation of `OnceLock` that keeps track of the number of references to the internal data
and drops it every time the references counter goes to 0.

# Why is this useful?

In Rust static variables are not dropped when the program terminates. This is a problem when you need to initialize a shared resource that must be dropped when it is no longer used or when the process terminates. This happens, for example, when you a have a common resource to be usued by a set of integration tests, and you want it to be dropped when the tests terminates.

# Usage examples    

```rust
mod test {
    use std::sync::OnceLock;
    use maybe_once::blocking::{Data, MaybeOnce};
    
    /// A data initializer function. This can be called more than 
    /// once.
    /// If everything goes as expected, it should only be called
    /// once.
    fn init() -> String {
        // Expensive initialization logic here.
        // For example, you can start here a docker container 
        // (e.g. by using testcontainers),
        // when there will be no more references to the data, 
        // the data will be dropped and the container will be 
        // stopped.
        "hello".to_string()    
    }
    
    /// A function that holds a static reference to the `MaybeOnce` 
    /// object and returns a `Data` object.
    pub fn data(serial: bool) -> Data<'static, String> {
        static DATA: OnceLock<MaybeOnce<String>> = OnceLock::new();
        DATA.get_or_init(|| MaybeOnce::new(|| init()))
            .data(serial)
    }
    
    /// Here we have multiple tests that access the data. 
    /// As the tests are executed in parallel in multiple threads,
    /// the internal counter of the `MaybeOnce` object will be 
    /// incremented to 3. 
    /// It will then decrement every time a test finishes,
    /// and incrementes each time a new test starts.
    /// Once the internal counter goes to 0, the data will be dropped. 
    /// At this points all tests are (statistically) complete, but if 
    /// for some reason the data is accessed again, the data will be 
    /// recreated using the `init` function.
    /// 
    /// WARNING: If you execute the tests with a single thread, 
    /// the data will be dropped after each test and recreated 
    /// each time.
    #[test]
    fn test1() {
        let data = data(false);
        println!("{}", *data);
    }

    #[test]
    fn test2() {
        let data = data(false);
        println!("{}", *data);
    }

    #[test]
    fn test3() {
        let data = data(false);
        println!("{}", *data);
    }

}
```

# Usage with tokio

The `tokio` feature of this crate allows you to use the optional `MaybeOnceAsync` object to initialize a shared resource using an async function.

```rust
#[cfg(feature = "tokio")]
mod test {
    
    use std::sync::OnceLock;
    use maybe_once::tokio::{Data, MaybeOnceAsync};
    
    /// A data async initializer function.
    async fn init() -> String {
        // Expensive initialization logic here.
        "hello".to_string()    
    }
    
    /// A function that holds a static reference to the `MaybeOnceAsync`
    /// object and returns a `Data` object.
    pub async fn data(serial: bool) -> Data<'static, String> {
        static DATA: OnceLock<MaybeOnceAsync<String>> = OnceLock::new();
        DATA.get_or_init(|| MaybeOnceAsync::new(|| Box::pin(init())))
            .data(serial)
            .await
    }

    #[tokio::test]
    async fn test1() {
        let data = data(false).await;
        println!("{}", *data);  
    }

    #[tokio::test]
    async fn test2() {
        let data = data(false).await;
        println!("{}", *data);  
    }

    #[tokio::test]
    async fn test3() {
        let data = data(false).await;
        println!("{}", *data);  
    }

}
```
