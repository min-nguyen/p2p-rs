
## Async and Futures
- Async function: A function declared with async fn, which returns a future.
- Future: A value representing a computation that hasn’t completed yet. The async runtime is responsible for driving the future to completion.
- Async task: A unit of work that the runtime schedules and executes. An async task is a future being executed by the runtime.
- .await: A keyword used to pause the execution of an async function and yield control back to the runtime until the awaited future is ready.

1. Calling an async function creates a future

When you call an async fn, you are not starting the execution of the async function immediately.
Instead, it returns a future—an object that represents the work the async function will perform.
At this point, the future exists, but the code inside example (the async function) hasn't run yet.

```rust
async fn example() {
    println!("Async task started");
}
let future = example(); // Future is created, but nothing is executed yet
```

2. .await starts executing the future (async task)

When you call .await on a future, the async runtime starts executing the code inside the async function.
This is when the function begins to run. At this point, the runtime starts executing the code inside example from the beginning.

```rust
let future = example();
future.await; // Now the async function `example` starts running
```

3. Suspending and resuming the task

Inside the async function, when an .await is encountered, the async task (the execution of the future) may be suspended if the awaited operation isn't ready yet. This means the task is paused, but the thread running the task is not blocked. The runtime can switch to other tasks and make progress on them while the current task is paused.

```rust
async fn example() {
    println!("Task started");
    tokio::time::sleep(Duration::from_secs(2)).await; // Task suspended here
    println!("Task resumed");
}

let future = example();
future.await; // Task starts, but pauses for 2 seconds at `.await`
```
In this example, when the async function reaches .await tokio::time::sleep(Duration::from_secs(2)), the execution of this specific task is paused for 2 seconds. During this time, the runtime can work on other tasks. When the sleep operation completes, the runtime resumes the task and the function continues running from the point where it was paused.

3. Non-blocking behavior of .await

While the async task (the execution of the future) is suspended at an .await, the thread is not blocked. The runtime can continue running other tasks, making the overall system more efficient. This is what makes async tasks non-blocking in Rust.

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
    tokio::join!(task1(), task2()); // Runs both tasks concurrently
}
```
- task1() starts and immediately hits a .await for 2 seconds.
- While task1() is paused, the runtime switches to task2(), which runs and pauses for 1 second.
- After 1 second, task2() resumes and completes. Then, after 2 seconds total, task1() resumes and completes.