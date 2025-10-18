# Services Documentation

Business logic services that handle core application functionality.

## 📁 Files

- `mod.rs` - Module exports
- `transaction_service.rs` - External API communication
- `cache_service.rs` - Caching and data persistence
- `date_service.rs` - Date range processing

## 🔧 Services

### TransactionService
Handles communication with external transaction API.

**Key Methods:**
- `fetch_single_page()` - Fetch one page of transactions
- `fetch_all_pages()` - Fetch all pages with pagination
- `parse_transaction_record()` - Parse API response to internal format

**Features:**
- Automatic pagination handling
- Rate limiting with random delays
- Retry logic with exponential backoff
- Environment-based configuration

### CacheService
Manages intelligent caching system with memory and file persistence.

**Key Methods:**
- `get_from_memory_cache()` - Fast memory lookup
- `load_all_from_file_cache()` - Startup cache loading
- `save_cache_to_file()` - Persist cache to disk
- `fetch_and_cache_date_range_background()` - Background processing
- `get_missing_dates()` - Identify uncached dates

**Features:**
- Two-tier caching (memory + file)
- Background batch processing
- Memory usage optimization
- Atomic operations for thread safety

### DateService
Utility service for date range processing.

**Key Methods:**
- `get_date_range()` - Generate date sequences
- Date validation and parsing
- Range expansion utilities

## 🔄 Service Interaction

```
Handler → TransactionService → External API
    ↓
CacheService → Memory/File Storage
    ↓
DateService → Date Processing
```

## ⚙️ Configuration

Services use environment variables for configuration:
- `API_BASE_URL` - External API endpoint
- `STORE_ID` - Store identifier
- `BATCH_SIZE` - Processing batch size
- `MAX_MEMORY_MB` - Memory limits
- `CACHE_FILE_PATH` - Cache file location

## 🚀 Performance Features

- **Concurrent Processing**: Multiple background jobs
- **Memory Management**: Configurable memory limits
- **Batch Operations**: Efficient bulk processing
- **Smart Caching**: Avoid redundant API calls