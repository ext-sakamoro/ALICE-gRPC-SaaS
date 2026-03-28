#![allow(dead_code)]
use axum::{extract::State, response::Json, routing::{get, post}, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

// ── State ───────────────────────────────────────────────────
struct AppState {
    start_time: Instant,
    stats: Mutex<Stats>,
}

struct Stats {
    total_services: u64,
    total_invocations: u64,
    total_streams: u64,
    total_describes: u64,
    bytes_transferred: u64,
}

// ── Types ───────────────────────────────────────────────────
#[derive(Serialize)]
struct Health { status: String, version: String, uptime_secs: u64, total_ops: u64 }

// Services
#[derive(Serialize)]
struct ServiceInfo {
    name: String, package: String, method_count: u32,
    streaming: bool, endpoint: String,
}
#[derive(Serialize)]
struct ServicesResponse { services: Vec<ServiceInfo>, total: usize }

// Invoke
#[derive(Deserialize)]
#[allow(dead_code)]
struct InvokeRequest {
    service: Option<String>,
    method: Option<String>,
    payload: Option<serde_json::Value>,
    timeout_ms: Option<u64>,
}
#[derive(Serialize)]
struct InvokeResponse {
    call_id: String, status: String, service: String,
    method: String, response: serde_json::Value,
    latency_ms: u64, elapsed_us: u128,
}

// Stream
#[derive(Deserialize)]
#[allow(dead_code)]
struct StreamRequest {
    service: Option<String>,
    method: Option<String>,
    stream_type: Option<String>,
    buffer_size: Option<u32>,
}
#[derive(Serialize)]
struct StreamResponse {
    stream_id: String, status: String, service: String,
    method: String, stream_type: String, buffer_size: u32,
    endpoint: String, elapsed_us: u128,
}

// Describe
#[derive(Serialize)]
struct MethodInfo { name: String, input_type: String, output_type: String, client_streaming: bool, server_streaming: bool }
#[derive(Serialize)]
struct DescribeResponse {
    service: String, package: String,
    methods: Vec<MethodInfo>, full_name: String,
}

// Stats
#[derive(Serialize)]
struct StatsResponse {
    total_services: u64, total_invocations: u64,
    total_streams: u64, total_describes: u64, bytes_transferred: u64,
}

// ── Main ────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "grpc_engine=info".into()))
        .init();
    let state = Arc::new(AppState {
        start_time: Instant::now(),
        stats: Mutex::new(Stats {
            total_services: 0, total_invocations: 0, total_streams: 0,
            total_describes: 0, bytes_transferred: 0,
        }),
    });
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/grpc/services", get(services))
        .route("/api/v1/grpc/invoke", post(invoke))
        .route("/api/v1/grpc/stream", post(stream))
        .route("/api/v1/grpc/describe", get(describe))
        .route("/api/v1/grpc/stats", get(stats))
        .layer(cors).layer(TraceLayer::new_for_http()).with_state(state);
    let addr = std::env::var("GRPC_ADDR").unwrap_or_else(|_| "0.0.0.0:8132".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("gRPC Engine on {addr}");
    axum::serve(listener, app).await.unwrap();
}

// ── Handlers ────────────────────────────────────────────────
async fn health(State(s): State<Arc<AppState>>) -> Json<Health> {
    let st = s.stats.lock().unwrap();
    Json(Health {
        status: "ok".into(), version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: s.start_time.elapsed().as_secs(),
        total_ops: st.total_services + st.total_invocations + st.total_streams,
    })
}

async fn services(State(s): State<Arc<AppState>>) -> Json<ServicesResponse> {
    s.stats.lock().unwrap().total_services += 1;
    let list = vec![
        ServiceInfo {
            name: "Greeter".into(), package: "helloworld".into(),
            method_count: 2, streaming: false,
            endpoint: "grpc://localhost:50051".into(),
        },
        ServiceInfo {
            name: "RouteGuide".into(), package: "routeguide".into(),
            method_count: 4, streaming: true,
            endpoint: "grpc://localhost:50052".into(),
        },
    ];
    let total = list.len();
    Json(ServicesResponse { services: list, total })
}

async fn invoke(State(s): State<Arc<AppState>>, Json(req): Json<InvokeRequest>) -> Json<InvokeResponse> {
    let t = Instant::now();
    let service = req.service.unwrap_or_else(|| "helloworld.Greeter".into());
    let method = req.method.unwrap_or_else(|| "SayHello".into());
    let response = serde_json::json!({ "message": format!("Hello from {service}.{method}") });
    {
        let mut st = s.stats.lock().unwrap();
        st.total_invocations += 1;
        st.bytes_transferred += 128;
    }
    Json(InvokeResponse {
        call_id: uuid::Uuid::new_v4().to_string(),
        status: "ok".into(), service, method, response,
        latency_ms: 3, elapsed_us: t.elapsed().as_micros(),
    })
}

async fn stream(State(s): State<Arc<AppState>>, Json(req): Json<StreamRequest>) -> Json<StreamResponse> {
    let t = Instant::now();
    let service = req.service.unwrap_or_else(|| "routeguide.RouteGuide".into());
    let method = req.method.unwrap_or_else(|| "RecordRoute".into());
    let stream_type = req.stream_type.unwrap_or_else(|| "client-streaming".into());
    let buffer_size = req.buffer_size.unwrap_or(4096);
    let stream_id = uuid::Uuid::new_v4().to_string();
    let endpoint = format!("grpc://localhost:50051/{service}/{method}");
    s.stats.lock().unwrap().total_streams += 1;
    Json(StreamResponse {
        stream_id, status: "open".into(),
        service, method, stream_type, buffer_size, endpoint,
        elapsed_us: t.elapsed().as_micros(),
    })
}

async fn describe(State(s): State<Arc<AppState>>) -> Json<DescribeResponse> {
    s.stats.lock().unwrap().total_describes += 1;
    Json(DescribeResponse {
        service: "Greeter".into(), package: "helloworld".into(),
        full_name: "helloworld.Greeter".into(),
        methods: vec![
            MethodInfo {
                name: "SayHello".into(), input_type: "HelloRequest".into(),
                output_type: "HelloReply".into(),
                client_streaming: false, server_streaming: false,
            },
            MethodInfo {
                name: "SayHelloStreamReply".into(), input_type: "HelloRequest".into(),
                output_type: "HelloReply".into(),
                client_streaming: false, server_streaming: true,
            },
        ],
    })
}

async fn stats(State(s): State<Arc<AppState>>) -> Json<StatsResponse> {
    let st = s.stats.lock().unwrap();
    Json(StatsResponse {
        total_services: st.total_services,
        total_invocations: st.total_invocations,
        total_streams: st.total_streams,
        total_describes: st.total_describes,
        bytes_transferred: st.bytes_transferred,
    })
}
