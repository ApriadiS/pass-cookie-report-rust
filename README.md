# 🦀 Pass Cookie Report Rust v1.2.0

A high-performance Rust-based API server for transaction reporting with intelligent caching and multi-architecture Docker support.

## ✨ Features

- **Fast & Efficient**: Built with Axum framework for high-performance HTTP handling
- **Smart Caching**: Intelligent memory and file-based caching system
- **Background Processing**: Asynchronous data fetching with job management
- **Multi-Architecture**: Support for both x86_64 and ARM64 deployments
- **Environment-Based Config**: Secure configuration via environment variables
- **Docker Ready**: Complete containerization with health checks

## 🚀 Quick Start

### Prerequisites

- Rust 1.82+ (stable)
- Docker (optional)

### Local Development

1. **Clone and setup**:
   ```bash
   git clone <repository-url>
   cd pass-cookie-report-rust
   cp .env.example .env
   ```

2. **Configure environment**:
   Edit `.env` with your actual values:
   ```env
   API_BASE_URL=https://your-api-server.com
   STORE_ID=your_store_id
   API_TIMESTAMP=your_timestamp
   ```

3. **Run locally**:
   ```bash
   cargo run
   ```

### Docker Deployment

#### x86_64 Architecture
```bash
chmod +x deploy-x86_64.sh
./deploy-x86_64.sh
```

#### ARM64 Architecture (AWS Graviton)
```bash
chmod +x deploy-arm64.sh
./deploy-arm64.sh
```

## 📡 API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/` | Health check |
| `POST` | `/data-cached` | Get cached transaction data with smart fetching |
| `POST` | `/force-refresh` | Force refresh all cache from database |
| `POST` | `/login` | Login endpoint for authentication |

### Example Usage

```bash
# Health check
curl http://localhost:3000/

# Get cached transaction data (with smart fetching)
curl -X POST http://localhost:3000/data-cached \
  -H "Content-Type: application/json" \
  -d '{
    "from": "01/10/2025",
    "to": "27/10/2025",
    "cookie": "your_session_cookie"
  }'

# Force refresh cache
curl -X POST http://localhost:3000/force-refresh \
  -H "Content-Type: application/json" \
  -d '{
    "from": "01/10/2025",
    "to": "27/10/2025",
    "cookie": "your_session_cookie"
  }'
```

## ⚙️ Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PORT` | Internal server port | `3000` |
| `EXTERNAL_PORT` | External Docker port | `3000` |
| `HOST` | Server bind address | `0.0.0.0` |
| `LOG_LEVEL` | Logging level | `info` |
| `CACHE_FILE_PATH` | Cache backup file path | `cache_backup.json` |
| `MAX_CONCURRENT_JOBS` | Max parallel jobs | `3` |
| `BATCH_SIZE` | Processing batch size | `5` |
| `MAX_MEMORY_MB` | Memory limit per batch | `50` |
| `API_BASE_URL` | Target API base URL | Required |
| `STORE_ID` | Store identifier | Required |
| `API_TIMESTAMP` | API timestamp parameter | Required |

### Port Configuration

To change external access port, update `.env`:
```env
EXTERNAL_PORT=8080  # Access via localhost:8080
PORT=3000          # Internal container port
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   HTTP Client   │───▶│   Axum Server    │───▶│  Cache Service  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌──────────────────┐    ┌─────────────────┐
                       │ Transaction API  │    │  File Storage   │
                       └──────────────────┘    └─────────────────┘
```

### Key Components

- **Handlers**: HTTP request handlers for each endpoint
- **Services**: Business logic for transactions, caching, and date processing
- **Models**: Data structures for requests and responses
- **State**: Application state management with job tracking
- **Errors**: Centralized error handling

## 📁 Project Structure

```
pass-cookie-report-rust/
├── src/
│   ├── handlers/           # HTTP request handlers
│   ├── services/          # Business logic services
│   ├── models/            # Data models
│   ├── errors.rs          # Error handling
│   ├── main.rs           # Application entry point
│   └── state.rs          # Application state
├── cache_backup.json     # Cache persistence file
├── Dockerfile            # x86_64 container
├── Dockerfile.arm64      # ARM64 container
├── docker-compose.yml    # x86_64 deployment
├── docker-compose.arm64.yml # ARM64 deployment
├── deploy-x86_64.sh     # x86_64 deployment script
├── deploy-arm64.sh      # ARM64 deployment script
└── .env.example         # Environment template
```

## 🔧 Development

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Docker Build

```bash
# x86_64
docker build -f Dockerfile -t pass-cookie-report-rust .

# ARM64
docker build -f Dockerfile.arm64 -t pass-cookie-report-rust-arm64 .
```

## 📊 Monitoring

### Health Checks

The application includes built-in health checks:
- HTTP endpoint: `GET /`
- Docker health check via wget

### Logging

Structured logging with configurable levels:
```bash
# View logs
docker logs -f pass-cookie-report-rust

# Follow logs with timestamps
docker logs -f --timestamps pass-cookie-report-rust
```

## 🔒 Security

- **Environment Variables**: All sensitive data in `.env`
- **No Hardcoded Secrets**: API URLs and credentials externalized
- **Gitignore Protection**: Debug files and secrets excluded
- **Container Security**: Minimal Alpine-based images

## 🚀 Deployment

### Production Checklist

- [ ] Configure `.env` with production values
- [ ] Set appropriate `LOG_LEVEL` (warn/error for production)
- [ ] Configure `MAX_CONCURRENT_JOBS` based on server capacity
- [ ] Set up log rotation for Docker containers
- [ ] Configure reverse proxy (nginx/traefik) if needed
- [ ] Set up monitoring and alerting

### AWS EC2 Graviton

For ARM64 deployment on AWS Graviton instances:
```bash
# Use ARM64 deployment script
./deploy-arm64.sh
```

## 📝 License

This project is licensed under the MIT License.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📞 Support

For issues and questions:
- Create an issue in the repository
- Check existing documentation
- Review logs for error details