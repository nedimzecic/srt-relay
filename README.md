# srt-relay - SRT Streaming Proxy
A high-performance SRT (Secure Reliable Transport) streaming proxy built with Tokio. This server accepts a single SRT input stream and broadcasts it to multiple SRT callers.

## Features
- **Single Input, Multiple Outputs**: Accepts one SRT stream and broadcasts to multiple callers
- **Asynchronous I/O**: Built on Tokio for efficient concurrent connections
- **Low Latency**: Optimized for real-time streaming applications
- **Configurable Binding**: Specify custom addresses and ports for input and output sockets
- **Automatic Reconnection**: Handles client disconnections gracefully

## Installation
### Building from Source
```bash
cd srt-relay

cargo build --release
```

## Usage
```bash
srt-relay <input_address>:<input_port> <output_address>:<output_port>
```

### Parameters
- `input_address`, `input_port`: The address and port for the input SRT socket (e.g., `0.0.0.0:10001`)
- `output_address`, `output_port`: The address and port for the output SRT socket (e.g., `0.0.0.0:11001`)

### Example
```bash
./target/release/srt-relay 0.0.0.0:10001 0.0.0.0:11001
```

This will:
1. Listen for a single SRT caller connection on port 10001
2. Listen for multiple SRT caller connections on port 11001
3. Broadcast the stream from the input to all connected outputs

## Architecture
```
┌──────────────┐
│  SRT Source  │
│   (Caller)   │
└──────┬───────┘
       │
       ▼ Port 10001
┌──────────────┐
│              │
│   srt-relay  │
│              │
└──────┬───────┘
       │
       ▼ Port 11001
       │
   ┌───┴───┬───┬───┐
   ▼       ▼   ▼   ▼
┌──────┐ ┌──────┐ ┌──────┐
│Caller│ │Caller│ │Caller│
│  #1  │ │  #2  │ │  #N  │
└──────┘ └──────┘ └──────┘
```
