# HTTP API Documentation

The Polygon POL Indexer provides a REST API for querying POL token net-flow data and system status.

## Starting the Server

```bash
# Build and run the server
cargo run --bin server

# Or with custom options
cargo run --bin server -- --database ./custom.db --port 3000
```

## API Endpoints

### GET /net-flow

Returns the current cumulative net-flow data for POL tokens to/from Binance.

**Response:**

```json
{
  "total_inflow": "1500.75",
  "total_outflow": "500.25",
  "net_flow": "1000.50",
  "last_processed_block": 12345,
  "last_updated": 1640995200
}
```

**Fields:**

- `total_inflow`: Total POL tokens transferred TO Binance addresses
- `total_outflow`: Total POL tokens transferred FROM Binance addresses
- `net_flow`: Net flow (inflow - outflow)
- `last_processed_block`: Last block number processed
- `last_updated`: Unix timestamp of last update

### GET /status

Returns system health and status information.

**Response:**

```json
{
  "status": "healthy",
  "last_processed_block": 12345,
  "total_transactions": 1250,
  "last_updated": 1640995200,
  "database_status": "connected"
}
```

**Fields:**

- `status`: System health status ("healthy" or "unhealthy")
- `last_processed_block`: Last block number processed
- `total_transactions`: Total number of transactions stored
- `last_updated`: Unix timestamp of last update
- `database_status`: Database connection status

### GET /transactions

Returns recent transactions with pagination support.

**Query Parameters:**

- `limit` (optional): Number of transactions to return (default: 100, max: 1000)
- `offset` (optional): Number of transactions to skip (default: 0)

**Example:**

```
GET /transactions?limit=10&offset=20
```

**Response:**

```json
{
  "transactions": [
    {
      "id": 1,
      "block_number": 12345,
      "transaction_hash": "0x1234567890abcdef...",
      "log_index": 0,
      "from_address": "0xsender...",
      "to_address": "0xf977814e90da44bfa03b6295a0616a897441acec",
      "amount": "100.50",
      "timestamp": 1640995200,
      "direction": "inflow",
      "created_at": 1640995200
    }
  ],
  "total_count": 1250,
  "limit": 10,
  "offset": 20,
  "has_more": true
}
```

**Transaction Fields:**

- `id`: Database record ID
- `block_number`: Ethereum block number
- `transaction_hash`: Transaction hash
- `log_index`: Event log index within the transaction
- `from_address`: Sender address
- `to_address`: Recipient address
- `amount`: Transfer amount in POL tokens
- `timestamp`: Block timestamp (Unix)
- `direction`: Transfer direction ("inflow" or "outflow")
- `created_at`: Record creation timestamp (Unix)

**Pagination Fields:**

- `total_count`: Total number of transactions in database
- `limit`: Requested limit
- `offset`: Requested offset
- `has_more`: Whether more transactions are available

## Error Responses

All endpoints return error responses in the following format:

```json
{
  "error": "error_type",
  "message": "Human readable error message"
}
```

**Common Error Types:**

- `database_error`: Database operation failed
- `invalid_parameter`: Invalid query parameter provided

**HTTP Status Codes:**

- `200 OK`: Successful request
- `400 Bad Request`: Invalid parameters
- `500 Internal Server Error`: Server or database error

## CORS Support

The API includes CORS headers to allow cross-origin requests from web applications.

## Example Usage

### Using curl

```bash
# Get current net-flow
curl http://localhost:8080/net-flow

# Get system status
curl http://localhost:8080/status

# Get recent transactions
curl http://localhost:8080/transactions?limit=5

# Get transactions with pagination
curl http://localhost:8080/transactions?limit=10&offset=50
```

### Using JavaScript

```javascript
// Fetch net-flow data
const response = await fetch("http://localhost:8080/net-flow");
const netFlow = await response.json();
console.log(`Current net-flow: ${netFlow.net_flow} POL`);

// Fetch recent transactions
const txResponse = await fetch("http://localhost:8080/transactions?limit=10");
const txData = await txResponse.json();
console.log(`Found ${txData.transactions.length} transactions`);
```

## Configuration

The server can be configured via command line arguments:

- `--database <path>`: Database file path (default: ./blockchain.db)
- `--port <port>`: Server port (default: 8080)

## Performance Notes

- The `/transactions` endpoint supports pagination to handle large datasets efficiently
- Maximum limit per request is 1000 transactions
- All endpoints typically respond within 100ms for normal database sizes
- The server uses connection pooling for optimal database performance
