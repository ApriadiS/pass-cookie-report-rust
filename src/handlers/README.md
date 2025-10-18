# Handlers Documentation

HTTP request handlers for all API endpoints.

## 📁 Files

- `mod.rs` - Module exports and common handler utilities
- Individual handler files for each endpoint group

## 🔧 Handler Functions

### Core Endpoints
- `root()` - Health check endpoint (`GET /`)
- `echo()` - Echo test endpoint (`POST /echo`)

### Transaction Endpoints
- `get_data_by_from_date_to_date()` - Get transaction data (`POST /data`)
- `get_data_by_from_date_to_date_debugging()` - Debug version (`POST /data-debug`)

### Cache Management
- `get_cached_data()` - Retrieve cached data (`POST /data-cached`)
- `force_refresh_data()` - Force cache refresh (`POST /force-refresh`)
- `force_empty_cache()` - Clear cache (`POST /force-empty`)

### Background Processing
- `start_fetch_data()` - Start background fetch job (`POST /start-fetch`)

## 🔄 Handler Pattern

All handlers follow this pattern:

```rust
pub async fn handler_name(
    State(state): State<AppState>,
    Json(payload): Json<RequestType>
) -> Result<Json<ResponseType>, ErrorType> {
    // 1. Validate input
    // 2. Call service layer
    // 3. Return formatted response
}
```

## 📝 Request/Response

- **Input**: JSON payloads via `Json<T>` extractor
- **Output**: JSON responses via `Json<T>` or error types
- **State**: Shared application state via `State<AppState>`

## 🛡️ Error Handling

Handlers use custom error types that automatically convert to HTTP responses:
- `AppError` - General application errors
- `DebugAppError` - Debug-specific errors with detailed logging