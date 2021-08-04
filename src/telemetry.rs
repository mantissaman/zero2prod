use tracing::Subscriber;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter};
use tracing_subscriber::layer::SubscriberExt;

/// Compose multiple layers into a `tracing`'s subscriber
///
/// # Implementation Notes
///
/// We are using `impl Subscriber` as return type to avoid having to
/// spell out the actual type of returned subscriber.
///
/// We need Send + Sync to make is possible to pass into `init_subscriber`
///
pub fn get_subscriber(
    name: String,
    env_filter: String,
    sink: impl MakeWriter + Sync + Send + 'static
) -> impl Subscriber + Send + Sync {
    //If RUST_LOG is not passed use default info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));
    //Using bunyan nor the one provided by tracing-subscriber as it doesn't implement metadata inheritance. output span to stdout
    let formatting_layer = BunyanFormattingLayer::new(name, sink);
    //With is provided by SubscriberExt
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

/// Register a subscriber as global defualt to process span data.
///
/// It should only be called once
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    //Redirect all log events to our subscribe
    LogTracer::init().expect("Failed to set Logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}
