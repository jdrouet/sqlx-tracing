use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use opentelemetry::trace::TracerProvider;
use opentelemetry::{InstrumentationScope, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_semantic_conventions::attribute as semver;
use testcontainers::core::{AccessMode, ContainerPort, Mount, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const SERVICE_NAME: &str = env!("CARGO_PKG_NAME");
const SERVICE_VERSION: &str = env!("CARGO_PKG_VERSION");

struct OpenTelemetryBuilder {
    otel_collector_endpoint: Cow<'static, str>,
    otel_internal_level: Cow<'static, str>,
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

    pub fn flush(&self) {
        if let Err(err) = self.metric.force_flush() {
            eprintln!("failed flushing metrics provider: {err:?}");
        }
        if let Err(err) = self.trace.force_flush() {
            eprintln!("failed flushing traces provider: {err:?}");
        }
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
                include_bytes!("../asset/otelcol-config.yml").to_vec(),
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

    pub fn json_traces(&self) -> RootTrace {
        // traces are appended to the same file, one line per flush,
        // so we need to take the last one only.
        let content = self.traces();
        let last = content
            .split("\n")
            .filter(|item| !item.is_empty())
            .last()
            .unwrap();
        serde_json::from_str(last).unwrap()
    }

    pub async fn address(&self) -> String {
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootTrace {
    pub resource_spans: Vec<TraceResourceSpan>,
}

impl RootTrace {
    pub fn find_scope_span(&self, name: &str) -> Option<&ScopeSpan> {
        self.resource_spans
            .iter()
            .flat_map(|span| span.scope_spans.iter())
            .find(|scope| scope.scope.name == name)
    }

    pub fn find_child(&self, parent_id: &str, name: &str) -> Option<&Span> {
        self.resource_spans
            .iter()
            .flat_map(|span| span.scope_spans.iter())
            .flat_map(|span| span.spans.iter())
            .find(|span| {
                span.parent_span_id
                    .as_ref()
                    .is_some_and(|id| id == parent_id)
                    && span.name == name
            })
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceResourceSpan {
    pub resource: TraceResource,
    pub scope_spans: Vec<ScopeSpan>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceResource {
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attribute {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeSpan {
    pub scope: Scope,
    #[serde(default)]
    pub spans: Vec<Span>,
}

impl ScopeSpan {
    pub fn first_span(&self) -> Option<&Span> {
        self.spans.iter().find(|span| span.parent_span_id.is_none())
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    pub flags: u32,
    pub name: String,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
    #[serde(default)]
    pub events: Vec<Event>,
    #[serde(default)]
    pub status: HashMap<String, serde_json::Value>,
}

impl Span {
    pub fn int_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attr| attr.key == name)
            .and_then(|attr| attr.value.as_object())
            .and_then(|obj| obj.get("intValue"))
            .and_then(|value| value.as_str())
    }

    pub fn string_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attr| attr.key == name)
            .and_then(|attr| attr.value.as_object())
            .and_then(|obj| obj.get("stringValue"))
            .and_then(|value| value.as_str())
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub name: String,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}
