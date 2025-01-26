use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::{MultiUseSandbox, UninitializedSandbox};
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::sync::atomic::{AtomicUsize, Ordering};

const INITIAL_POOL_SIZE: usize = 10;
const MAX_POOL_SIZE: usize = 100;
static CREATED_VM_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) static SEMAPHORE: Lazy<Arc<Semaphore>> = Lazy::new(|| Arc::new(Semaphore::new(MAX_POOL_SIZE)));

// Pool of sandboxes, initialized lazily
pub(crate) static MULTI_USE_SANDBOX_POOL: Lazy<Arc<Mutex<Vec<Arc<Mutex<MultiUseSandbox>>>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(Vec::with_capacity(MAX_POOL_SIZE)))
});

// Helper function to create a new sandbox
async fn create_sandbox() -> Arc<Mutex<MultiUseSandbox>> {
    // Increment the counter before creating a new sandbox
    CREATED_VM_COUNT.fetch_add(1, Ordering::SeqCst);
    let mut sandbox_cfg = hyperlight_host::sandbox::SandboxConfiguration::default();
            sandbox_cfg.set_input_data_size(33554432);
            sandbox_cfg.set_heap_size(33554432);
            sandbox_cfg.set_output_data_size(33554432);

    let uninitialized_sandbox = UninitializedSandbox::new(
        hyperlight_host::GuestBinary::FilePath("/usr/local/bin/chick-guest".to_string()),
        Some(sandbox_cfg), // default configuration
        None, // default run options
        None, // default host print function
    ).unwrap();

    let sandbox = uninitialized_sandbox.evolve(Noop::default()).unwrap();

    Arc::new(Mutex::new(
        sandbox,
    ))
}


// Function to acquire a sandbox, creating a new one if the pool is empty
pub(super) async fn acquire_sandbox() -> Arc<Mutex<MultiUseSandbox>> {
    // Wait for an available permit, respecting the max pool size
    let _permit = SEMAPHORE.acquire().await.unwrap();

    // Try to get an existing sandbox from the pool
    let mut pool = MULTI_USE_SANDBOX_POOL.lock().await;
    if let Some(sandbox) = pool.pop() {
        sandbox
    } else {
        // Create a new sandbox if the pool is empty and return it
        create_sandbox().await
    }
}

// Function to release a sandbox back to the pool
pub(super) async fn release_sandbox(sandbox: Arc<Mutex<MultiUseSandbox>>) {
    let mut pool = MULTI_USE_SANDBOX_POOL.lock().await;
    if pool.len() < MAX_POOL_SIZE {
        pool.push(sandbox); // Return the sandbox to the pool if under the max limit
    }
}

// Function to warm up the initial pool with a few instances
pub(super) async fn warm_up_pool() {
    let mut pool = MULTI_USE_SANDBOX_POOL.lock().await;
    for _ in 0..INITIAL_POOL_SIZE {
        let sandbox = create_sandbox().await;
        pool.push(sandbox);
        println!("Sandbox instance warmed up.");
    }
}
