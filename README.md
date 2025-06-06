# TRMNL Meeting Room Display

This application provides a web server that registers meeting room display devices and serves monochrome BMP images to them.

## Features

- Device registration API with secure token authentication
- Display endpoint providing monochrome BMP images using BlockKie.ttf font
- SQLite database integration for device storage
- Environment-based configuration
- Clean error handling
- Modular architecture with separate components

## Project Structure

The project is organized into several modules:

- `src/main.rs` - Application entry point and tests
- `src/server/` - Web server logic and API endpoints
- `src/database/` - Database connection and operations
- `src/bmp/` - BMP image generation functionality with font rendering

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

2. Configure the application:
```
cp .env.example .env
```

3. Edit the `.env` file with your preferred settings, especially the `ACCESS_TOKEN`.

4. Build the application:
```
cargo build --release
```

### Configuration

The application is configured using environment variables or a `.env` file. Here are the available configuration options:

| Variable | Description | Default |
|----------|-------------|---------|
| `SERVER_HOST` | Host/IP address the server binds to | `127.0.0.1` |
| `SERVER_PORT` | Port the server listens on | `8080` |
| `DATABASE_PATH` | Path to the SQLite database file | `devices.db` |
| `ACCESS_TOKEN` | Secret token for API authentication | *Required* |
| `FONT_PATH` | Path to the font used for text rendering | `assets/fonts/BlockKie.ttf` |
| `REFRESH_RATE` | Refresh rate for display updates in seconds | `200` |

## Usage

### Starting the Server

Run the application with:

```
cargo run --release
```

The server will start on the configured host and port (default: http://127.0.0.1:8080).

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

The `image_url` contains a Base64-encoded monochrome 800x480px BMP image displaying "hello world" text rendered using the configured font.

#### Device Logging

```
POST /api/log
```

This endpoint captures log messages from ESP32 devices for debugging purposes. It accepts any payload and logs the request details including headers and body content.

Headers:
- `ID`: Device MAC address (optional, for identification)
- `User-Agent`: Client identifier (typically "ESP32HTTPClient")
- `Content-Type`: Content type of the payload

Example:

```bash
curl -X POST "http://localhost:8080/api/log" \
    -H 'ID: 00:11:22:33:44:55' \
    -H 'User-Agent: ESP32HTTPClient' \
    -H 'Content-Type: application/json' \
    -d '{"level":"INFO","message":"Device startup","timestamp":1672531200}'
```

Response:

```json
{
  "status": "received",
  "message": "Log entry processed successfully"
}
```

#### Health Check

```
GET /health
```

Example:

```bash
curl "http://localhost:8080/health"
```

Response:

```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

## Development

### Environment Setup

For development, you can create a `.env` file in the project root with your configuration values:

```
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
DATABASE_PATH=devices.db
ACCESS_TOKEN=your-development-token
FONT_PATH=assets/fonts/BlockKie.ttf
REFRESH_RATE=200
```

### Running in Development Mode

```
cargo run
```

### Running Tests

```
cargo test
```

### Code Formatting

Always run the formatter before committing:

```
cargo fmt
```

## Security Considerations

- Change the `ACCESS_TOKEN` to a strong, randomly generated value for production use
- Consider implementing HTTPS for secure communication
- By default, the server binds to `127.0.0.1` (localhost only). To allow external connections, set `SERVER_HOST=0.0.0.0` or specify a particular network interface
- The `/api/log` endpoint does not require authentication and will log all request details for debugging ESP32 devices