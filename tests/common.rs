use std::time::Duration;

use testcontainers::{
    GenericImage, ImageExt,
    core::{AccessMode, ContainerPort, Mount, WaitFor},
    runners::AsyncRunner,
};

use std::borrow::Cow;

use opentelemetry::trace::TracerProvider;
use opentelemetry::{InstrumentationScope, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_semantic_conventions::attribute as semver;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const SERVICE_NAME: &str = env!("CARGO_PKG_NAME");
const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct OpenTelemetryBuilder {
    pub otel_collector_endpoint: Cow<'static, str>,
    pub otel_internal_level: Cow<'static, str>,
}

impl OpenTelemetryBuilder {
    fn attributes(&self) -> impl IntoIterator<Item = KeyValue> {
        [
            KeyValue::new(semver::SERVICE_NAME, SERVICE_NAME),
            KeyValue::new(semver::SERVICE_VERSION, SERVICE_VERSION),
            KeyValue::new("deployment.environment", "test"),
        ]
    }

    fn resources(&self) -> Resource {
        self.attributes()
            .into_iter()
            .fold(Resource::builder(), |res, attr| res.with_attribute(attr))
            .build()
    }

    fn metric_provider(&self) -> anyhow::Result<SdkMeterProvider> {
        let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_protocol(opentelemetry_otlp::Protocol::Grpc)
            .with_endpoint(self.otel_collector_endpoint.as_ref())
            .build()?;

        Ok(opentelemetry_sdk::metrics::MeterProviderBuilder::default()
            .with_periodic_exporter(metric_exporter)
            .with_resource(self.resources())
            .build())
    }

    fn trace_provider(&self) -> anyhow::Result<SdkTracerProvider> {
        let trace_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_protocol(opentelemetry_otlp::Protocol::Grpc)
            .with_endpoint(self.otel_collector_endpoint.as_ref())
            .build()?;

        let trace_processor = BatchSpanProcessor::builder(trace_exporter).build();

        Ok(opentelemetry_sdk::trace::TracerProviderBuilder::default()
            .with_span_processor(trace_processor)
            .with_resource(self.resources())
            .build())
    }

    fn logger_provider(&self) -> anyhow::Result<SdkLoggerProvider> {
        let log_exporter = opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_protocol(opentelemetry_otlp::Protocol::Grpc)
            .with_endpoint(self.otel_collector_endpoint.as_ref())
            .build()?;

        Ok(opentelemetry_sdk::logs::SdkLoggerProvider::builder()
            .with_resource(self.resources())
            .with_batch_exporter(log_exporter)
            .build())
    }

    pub fn build(self) -> anyhow::Result<OpenTelemetryProvider> {
        let scope = InstrumentationScope::builder(SERVICE_NAME)
            .with_version(SERVICE_VERSION)
            .with_schema_url(opentelemetry_semantic_conventions::SCHEMA_URL)
            .with_attributes(self.attributes())
            .build();

        Ok(OpenTelemetryProvider {
            metric: self.metric_provider()?,
            trace: self.trace_provider()?,
            logger: self.logger_provider()?,
            scope,
            internal_level: self.otel_internal_level,
        })
    }
}

pub struct OpenTelemetryProvider {
    internal_level: Cow<'static, str>,
    metric: SdkMeterProvider,
    trace: SdkTracerProvider,
    logger: SdkLoggerProvider,
    scope: InstrumentationScope,
}

impl std::fmt::Debug for OpenTelemetryProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(OpenTelemetryProvider))
            .finish_non_exhaustive()
    }
}

impl OpenTelemetryProvider {
    pub fn install(&self) -> anyhow::Result<()> {
        opentelemetry::global::set_meter_provider(self.metric.clone());
        opentelemetry::global::set_tracer_provider(self.trace.clone());
        opentelemetry::global::set_text_map_propagator(
            opentelemetry_sdk::propagation::TraceContextPropagator::new(),
        );

        let tracer = self.trace.tracer_with_scope(self.scope.clone());

        let registry = tracing_subscriber::registry()
            .with(otel_filter(&self.internal_level))
            .with(OpenTelemetryLayer::new(tracer))
            .with(MetricsLayer::new(self.metric.clone()))
            .with(OpenTelemetryTracingBridge::new(&self.logger));
        registry.try_init()?;

        Ok(())
    }

    pub fn shutdown(&self) {
        if let Err(err) = self.metric.shutdown() {
            eprintln!("failed shutting down metrics provider: {err:?}");
        }
        if let Err(err) = self.trace.shutdown() {
            eprintln!("failed shutting down traces provider: {err:?}");
        }
    }
}

impl Drop for OpenTelemetryProvider {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn otel_filter(level: &str) -> EnvFilter {
    EnvFilter::from_default_env()
        .add_directive("debug".parse().unwrap())
        .add_directive(format!("h2={level}").parse().unwrap())
        .add_directive(format!("hyper_util={level}").parse().unwrap())
        .add_directive(format!("opentelemetry={level}").parse().unwrap())
        .add_directive(format!("reqwest={level}").parse().unwrap())
        .add_directive(format!("tonic={level}").parse().unwrap())
        .add_directive(format!("tower={level}").parse().unwrap())
}

#[derive(Debug)]
pub struct ObservabilityContainer {
    container: testcontainers::ContainerAsync<testcontainers::GenericImage>,
    tmp_dir: tempfile::TempDir,
}

impl ObservabilityContainer {
    pub async fn create() -> Self {
        let tmp_dir = tempfile::TempDir::new().unwrap();

        let container = GenericImage::new("otel/opentelemetry-collector-contrib", "latest")
            .with_wait_for(WaitFor::message_on_stderr(
                "Everything is ready. Begin running and processing data.",
            ))
            .with_exposed_port(ContainerPort::Tcp(4317))
            .with_copy_to(
                "/etc/otelcol-contrib/config.yaml",
                include_bytes!("otelcol-config.yml").to_vec(),
            )
            .with_mount(
                Mount::bind_mount(tmp_dir.path().to_string_lossy(), "/tmp/output")
                    .with_access_mode(AccessMode::ReadWrite),
            )
            .with_user("1001:1001")
            .with_startup_timeout(Duration::from_secs(30))
            .start()
            .await
            .unwrap();

        Self { container, tmp_dir }
    }

    pub fn traces(&self) -> String {
        let path = self.tmp_dir.path().join("traces.json");
        std::fs::read_to_string(path).unwrap()
    }

    async fn address(&self) -> String {
        let port = self.container.get_host_port_ipv4(4317).await.unwrap();

        format!("http://127.0.0.1:{port}")
    }

    pub async fn install(&self) -> OpenTelemetryProvider {
        let builder = OpenTelemetryBuilder {
            otel_collector_endpoint: self.address().await.into(),
            otel_internal_level: "off".into(),
        };
        let provider = builder.build().unwrap();
        provider.install().unwrap();

        provider
    }
}
