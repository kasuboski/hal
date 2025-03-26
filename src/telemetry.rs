use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter};
use opentelemetry_sdk::{
    logs::SdkLoggerProvider, metrics::SdkMeterProvider, trace::SdkTracerProvider, Resource,
};
use tracing_opentelemetry::MetricsLayer;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Layer;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

fn get_resource() -> Resource {
    static RESOURCE: OnceLock<Resource> = OnceLock::new();
    RESOURCE
        .get_or_init(|| Resource::builder().with_service_name("hal").build())
        .clone()
}

fn _init_logs() -> SdkLoggerProvider {
    let exporter = LogExporter::builder()
        .with_http()
        .build()
        .expect("Failed to create log exporter");

    SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(get_resource())
        .build()
}

fn init_traces() -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_http()
        .build()
        .expect("Failed to create trace exporter");

    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(get_resource())
        .build()
}

fn init_metrics() -> SdkMeterProvider {
    let exporter = MetricExporter::builder()
        .with_http()
        .build()
        .expect("Failed to create metric exporter");

    SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(get_resource())
        .build()
}

// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
pub fn init_tracing_subscriber() -> OtelGuard {
    let tracer_provider = init_traces();
    let meter_provider = init_metrics();
    // let logger_provider = init_logs();

    let tracer = tracer_provider.tracer("hal");

    // let log_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    // For the OpenTelemetry layer, add a tracing filter to filter events from
    // OpenTelemetry and its dependent crates (opentelemetry-otlp uses crates
    // like reqwest/tonic etc.) from being sent back to OTel itself, thus
    // preventing infinite telemetry generation. The filter levels are set as
    // follows:
    // - Allow `info` level and above by default.
    // - Restrict `opentelemetry`, `hyper`, `tonic`, and `reqwest` completely.
    // Note: This will also drop events from crates like `tonic` etc. even when
    // they are used outside the OTLP Exporter. For more details, see:
    // https://github.com/open-telemetry/opentelemetry-rust/issues/761
    // let filter_otel = EnvFilter::new("info")
    //     .add_directive("hyper=off".parse().unwrap())
    //     .add_directive("opentelemetry=off".parse().unwrap())
    //     .add_directive("tonic=off".parse().unwrap())
    //     .add_directive("h2=off".parse().unwrap())
    //     .add_directive("reqwest=off".parse().unwrap());
    // let log_layer = log_layer.with_filter(filter_otel);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::from_default_env());

    tracing_subscriber::registry()
        .with(console_layer)
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(tracer))
        // .with(log_layer)
        .init();

    OtelGuard {
        tracer_provider,
        meter_provider,
        logger_provider: None,
    }
}

pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    logger_provider: Option<SdkLoggerProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("{err:?}");
        }
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }

        if let Some(logger) = self.logger_provider.take() {
            if let Err(err) = logger.shutdown() {
                eprintln!("{err:?}");
            }
        }
    }
}
