pub mod rest;
pub mod async_trait;

pub type ThreadSafeError = Box<dyn std::error::Error + Send + Sync>;
pub type SubscriptionResult = Result<tokio::task::JoinHandle<()>, ThreadSafeError>;