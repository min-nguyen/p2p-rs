
## Async and Futures
- Async function: A function declared with async fn, which returns a future.
- Future: A value representing a computation that hasn’t completed yet. The async runtime is responsible for driving the future to completion.
- .await: A keyword that used _inside_ an Async Function _on_ a Future. Used to pause the execution of an async function and yield control back to the runtime until the awaited future is ready.

1. Calling an async function creates a future

When you call an async fn, you are not starting the execution of the async function immediately.
Instead, it returns a future—an object that represents the work the async function will perform.
At this point, the future exists, but the code inside example (the async function) hasn't run yet.

```rust
async fn example() {
    println!("Async function started");
}
let future = example(); // Future is created, but nothing is executed yet
```

2. .await starts executing the future

When you call .await on a future, the async runtime starts executing the code inside the async function.
This is when the function begins to run. At this point, the runtime starts executing the code inside example from the beginning.

```rust
let future = example();
future.await; // Now the async function `example` starts running
```

3. .await suspends the current async function

Inside the async function, if it calls an .await on another future, the async function is suspended if the awaited future isn't ready yet.
However, the main thread itself is not suspended, and is is free to switch to other async functions.

```rust
async fn example_with_await() {
    let result = some_async_operation().await; // If not ready, suspend this function
    println!("Result: {:?}", result);
}
```


4. Non-blocking behavior of .await

While the async function can be suspended if it calls an .await, the main thread is not blocked. The runtime can continue running other functions, making the overall system more efficient. This is what makes async functions non-blocking in Rust.

```rust
async fn task1() {
    println!("Task 1 started");
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!("Task 1 completed");
}

async fn task2() {
    println!("Task 2 started");
    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("Task 2 completed");
}

#[tokio::main]
async fn main() {
    tokio::join!(task1(), task2()); // Runs both functions concurrently
}
```
- task1() starts and immediately hits a .await for 2 seconds.
- While task1() is paused, the runtime switches to task2(), which runs and pauses for 1 second.
- After 1 second, task2() resumes and completes. Then, after 2 seconds total, task1() resumes and completes.