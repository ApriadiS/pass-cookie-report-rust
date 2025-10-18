# Models Documentation

Data structures and types used throughout the application.

## 📁 Files

- `mod.rs` - Module exports and common model utilities
- Individual model files for different data domains

## 🔧 Core Models

### Request Models

#### `Payload`
Main request structure for transaction queries.
```rust
pub struct Payload {
    pub from: String,      // Start date (YYYY-MM-DD)
    pub to: String,        // End date (YYYY-MM-DD)  
    pub cookie: String,    // Authentication cookie
}
```

#### `EchoRequest`
Simple echo endpoint request.
```rust
pub struct EchoRequest {
    pub message: String,
}
```

### Response Models

#### `DebugResponse`
Transaction data response with metadata.
```rust
pub struct DebugResponse {
    pub total_transaksi: usize,    // Total transaction count
    pub data: Vec<Transaksi>,      // Transaction records
}
```

#### `EchoResponse`
Echo endpoint response.
```rust
pub struct EchoResponse {
    pub echoed: String,
}
```

### Data Models

#### `Transaksi`
Core transaction record structure.
```rust
pub struct Transaksi {
    pub tanggal_transaksi: Option<NaiveDate>,      // Transaction date
    pub waktu_transaksi: Option<NaiveDateTime>,    // Transaction timestamp
    pub keterangan: String,                        // Description
    pub total_tagihan: u64,                       // Total amount
    pub no_nota: String,                          // Receipt number
}
```

## 📝 Serialization

All models implement:
- `Serialize` - JSON output serialization
- `Deserialize` - JSON input deserialization
- `Debug` - Debug formatting
- `Clone` - Value cloning

## 🔄 Data Flow

```
JSON Request → Deserialize → Model → Business Logic → Model → Serialize → JSON Response
```

## 📅 Date Handling

Date fields use `chrono` types:
- `NaiveDate` - Date without timezone
- `NaiveDateTime` - DateTime without timezone
- `Option<T>` - Nullable date fields

## 🛡️ Validation

Models include built-in validation:
- Date format validation
- Required field checking
- Type safety guarantees