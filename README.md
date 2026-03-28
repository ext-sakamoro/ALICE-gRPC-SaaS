# ALICE gRPC SaaS

gRPC service framework powered by ALICE. Register services, invoke methods, stream data, and inspect descriptors via a simple REST API.

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL%203.0-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Status

| Check | Status |
|-------|--------|
| `cargo check` | passing |
| API health | `/health` |

## Quick Start

```bash
docker compose up -d
```

API Gateway: http://localhost:8132

## Architecture

```
Client
  |
  v
API Gateway       :8132
  |
  v
gRPC Engine       :8133-internal
(service registry, method invocation, streaming, reflection)
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET`  | `/api/v1/grpc/services` | List registered gRPC services |
| `POST` | `/api/v1/grpc/invoke` | Invoke a gRPC method |
| `POST` | `/api/v1/grpc/stream` | Open a streaming call |
| `GET`  | `/api/v1/grpc/describe` | Describe a service or method |
| `GET`  | `/api/v1/grpc/stats` | Retrieve invocation statistics |
| `GET`  | `/health` | Service health check |

### invoke

```json
POST /api/v1/grpc/invoke
{
  "service": "helloworld.Greeter",
  "method": "SayHello",
  "payload": { "name": "Alice" }
}
```

### describe

```json
GET /api/v1/grpc/describe?service=helloworld.Greeter
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `GRPC_ADDR` | `0.0.0.0:8132` | Core engine bind address |
| `GATEWAY_ADDR` | `0.0.0.0:8131` | API gateway bind address |

## License

AGPL-3.0. Commercial dual-license available — contact for pricing.
