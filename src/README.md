# Source Code Documentation

This directory contains the main application source code organized by functionality.

## 📁 Structure

```
src/
├── handlers/          # HTTP request handlers
├── services/          # Business logic services  
├── models/            # Data structures and types
├── errors.rs          # Error handling and types
├── main.rs           # Application entry point
└── state.rs          # Application state management
```

## 🔧 Core Files

### `main.rs`
- Application entry point
- Server configuration and startup
- Environment variable loading
- Route registration

### `state.rs`
- Application state management
- Job tracking and concurrency control
- Cache coordination
- Memory management

### `errors.rs`
- Centralized error handling
- Custom error types
- HTTP response mapping
- Debug error variants

## 📦 Modules

Each subdirectory contains specialized functionality:

- **handlers/**: HTTP endpoint implementations
- **services/**: Core business logic
- **models/**: Data structures and serialization

## 🔄 Data Flow

```
HTTP Request → Handler → Service → State/Cache → Response
```

1. **Handler** receives and validates HTTP requests
2. **Service** implements business logic
3. **State** manages application state and jobs
4. **Cache** provides data persistence
5. **Response** returns formatted results