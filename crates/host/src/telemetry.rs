use axum::{
    extract::State,
    response::IntoResponse,
    routing::get,
    Router,
};
use prometheus::{Encoder, TextEncoder, Registry, Counter, Histogram, Gauge};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct TelemetryServer {
    registry: Arc<Registry>,
    metrics: Arc<RwLock<MetricsStore>>,
}

#[derive(Default)]
struct MetricsStore {
    counters: HashMap<String, Counter>,
    gauges: HashMap<String, Gauge>,
    histograms: HashMap<String, Histogram>,
}

impl TelemetryServer {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Arc::new(Registry::new());
        let metrics = Arc::new(RwLock::new(MetricsStore::default()));
        
        Ok(Self { registry, metrics })
    }

    pub fn counter_inc(&self, name: &str) {
        self.counter_add(name, 1);
    }

    pub fn counter_add(&self, name: &str, value: u64) {
        let mut store = self.metrics.write();
        let counter = store.counters.entry(name.to_string()).or_insert_with(|| {
            let opts = prometheus::Opts::new(name, name);
            let counter = Counter::with_opts(opts).unwrap();
            self.registry.register(Box::new(counter.clone())).ok();
            counter
        });
        counter.inc_by(value as f64);
    }

    pub fn gauge_set(&self, name: &str, value: f64) {
        let mut store = self.metrics.write();
        let gauge = store.gauges.entry(name.to_string()).or_insert_with(|| {
            let opts = prometheus::Opts::new(name, name);
            let gauge = Gauge::with_opts(opts).unwrap();
            self.registry.register(Box::new(gauge.clone())).ok();
            gauge
        });
        gauge.set(value);
    }

    pub fn histogram_observe(&self, name: &str, value: f64) {
        let mut store = self.metrics.write();
        let histogram = store.histograms.entry(name.to_string()).or_insert_with(|| {
            let opts = prometheus::HistogramOpts::new(name, name);
            let histogram = Histogram::with_opts(opts).unwrap();
            self.registry.register(Box::new(histogram.clone())).ok();
            histogram
        });
        histogram.observe(value);
    }

    pub async fn serve(self: Arc<Self>, bind: String, port: u16) -> anyhow::Result<()> {
        let app = Router::new()
            .route("/metrics", get(metrics_handler))
            .route("/health", get(health_handler))
            .with_state(self);

        let addr = format!("{}:{}", bind, port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        
        tracing::info!("Telemetry server listening on {}", addr);
        
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

async fn metrics_handler(State(telemetry): State<Arc<TelemetryServer>>) -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = telemetry.registry.gather();
    
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    
    (
        [("Content-Type", "text/plain; version=0.0.4")],
        buffer,
    )
}

async fn health_handler() -> impl IntoResponse {
    "OK"
}
