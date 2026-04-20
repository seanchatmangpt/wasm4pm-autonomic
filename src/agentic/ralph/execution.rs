use anyhow::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task;

pub struct ExecutionEngine {
    max_concurrency: usize,
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new(1)
    }
}

impl ExecutionEngine {
    pub fn new(max_concurrency: usize) -> Self {
        Self { max_concurrency }
    }

    /// Runs the execution engine, processing a stream of ideas using a task pool.
    pub async fn run<F, Fut>(&self, ideas: Vec<(String, String)>, process_fn: F) -> Result<()>
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<(String, String)>(100);
        let rx = Arc::new(Mutex::new(rx));
        let process_fn = Arc::new(process_fn);

        let mut worker_handles = vec![];

        // Spawn worker pool
        for _ in 0..self.max_concurrency {
            let rx = Arc::clone(&rx);
            let process_fn = Arc::clone(&process_fn);

            let handle = task::spawn(async move {
                loop {
                    let item = {
                        let mut rx_lock = rx.lock().await;
                        rx_lock.recv().await
                    };

                    match item {
                        Some((id, idea)) => {
                            if let Err(e) = process_fn(id, idea).await {
                                tracing::error!("Error processing idea: {}", e);
                            }
                        }
                        None => break, // Channel closed, no more work
                    }
                }
            });

            worker_handles.push(handle);
        }

        // Send all work to the pool
        for idea in ideas {
            tx.send(idea).await?;
        }

        // Close the channel so workers know to exit when done
        drop(tx);

        // Wait for all workers to complete
        for handle in worker_handles {
            let _ = handle.await?;
        }

        Ok(())
    }
}
