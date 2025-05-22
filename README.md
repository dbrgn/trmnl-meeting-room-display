# TRMNL Meeting Room Display

This application provides a web server that registers meeting room display devices using a simple REST API.

## Features

- Device registration API
- Display endpoint providing monochrome BMP images
- SQLite database integration for device storage
- Authentication using access tokens
- Modular architecture with separate components

## Project Structure

The project is organized into several modules:

- `src/main.rs` - Application entry point and tests
- `src/server/` - Web server logic and API endpoints
- `src/database/` - Database connection and operations
- `src/bmp/` - BMP image generation functionality

## Setup

### Prerequisites

- Rust (latest stable version)
- Cargo package manager

### Installation

1. Clone the repository:
```
git clone https://github.com/your-username/trmnl-meeting-room-display.git
cd trmnl-meeting-room-display
```

2. Build the application:
```
cargo build --release
```

### Configuration

Before running the application, make sure to configure the access token in `src/server/handlers.rs`:

```rust
pub const ACCESS_TOKEN: &str = "your-secret-access-token"; // Replace with your actual token
```

## Usage

### Starting the Server

Run the application with:

```
cargo run --release
```

The server will start on http://localhost:8080.

### API Endpoints

#### Device Setup

```
GET /api/setup/
```

Headers:
- `ID`: Device MAC address
- `Access-Token`: The configured access token
- `Accept`: application/json
- `Content-Type`: application/json

Example:

```bash
curl "http://localhost:8080/api/setup/" \
    -H 'ID: 00:11:22:33:44:55' \
    -H 'Access-Token: your-secret-access-token' \
    -H 'Accept: application/json' \
    -H 'Content-Type: application/json'
```

Response:

```json
{
  "message": "Device 00:11:22:33:44:55 registered successfully",
  "device_id": "00:11:22:33:44:55"
}
```

#### Device Display

```
GET /api/display
```

Headers:
- `ID`: Device MAC address
- `Access-Token`: The configured access token
- `Accept`: application/json

Example:

```bash
curl "http://localhost:8080/api/display" \
    -H 'ID: 00:11:22:33:44:55' \
    -H 'Access-Token: your-secret-access-token' \
    -H 'Accept: application/json'
```

Response:

```json
{
  "filename": "demo.bmp",
  "image_url": "data:image/bmp;base64,<truncated>",
  "image_url_timeout": 0,
  "refresh_rate": 200
}
```

The `image_url` contains a Base64-encoded monochrome 800x480px BMP image displaying "hello world" text.

## Database

The application uses SQLite to store device information. The database file is created at `devices.db` in the application root directory.

## Development

### Running in Development Mode

```
cargo run
```

### Running Tests

```
cargo test
```