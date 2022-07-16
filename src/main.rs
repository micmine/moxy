#[warn(missing_docs)]
pub mod builder;
#[warn(missing_docs)]
pub mod configuration;
#[warn(missing_docs)]
pub mod data_loader;
#[warn(missing_docs)]
pub mod router;
#[warn(missing_docs)]
pub mod route_handler;

extern crate pretty_env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "moxy=info");
    }
    pretty_env_logger::init();

    router::start().await;

    Ok(())
}
