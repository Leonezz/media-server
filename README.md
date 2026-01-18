# YAMS - Yet Another Media Server

A high-performance media server written in Rust 2024 edition, implementing streaming media protocols and codecs with a modular, event-driven architecture.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         YAMS Media Server                           │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────────────┐    │
│  │   RTMP Server│   │  RTSP Server │   │    HTTP Server       │    │
│  │   :1935      │   │   :554       │   │    :80               │    │
│  └──────┬───────┘   └──────┬───────┘   └──────────┬───────────┘    │
│         │                  │                       │                │
│         └──────────────────┼───────────────────────┘                │
│                            │                                        │
│                    ┌───────┴───────┐                                │
│                    │  StreamCenter │  ◄── Central Event Hub        │
│                    │  (Pub/Sub)    │                                │
│                    └───────┬───────┘                                │
│                            │                                        │
│         ┌──────────────────┼──────────────────┐                    │
│         │                  │                  │                     │
│  ┌──────┴───────┐   ┌──────┴───────┐   ┌─────┴────────┐           │
│  │   Codecs     │   │   Formats    │   │   Servers    │           │
│  ├──────────────┤   ├──────────────┤   ├──────────────┤           │
│  │ • H264       │   │ • RTMP       │   │ • RTP        │           │
│  │ • AAC        │   │ • RTSP       │   │ • STUN       │           │
│  │ • Bitstream  │   │ • RTP        │   │ • TURN       │           │
│  │ • Common     │   │ • FLV        │   │ • HTTP       │           │
│  │              │   │ • WebRTC     │   │              │           │
│  │              │   │ • AMF        │   │              │           │
│  │              │   │ • SDP        │   │              │           │
│  │              │   │ • STUN/TURN  │   │              │           │
│  └──────────────┘   └──────────────┘   └──────────────┘           │
└─────────────────────────────────────────────────────────────────────┘
```

### Core Design Patterns

- **Event-Driven Architecture**: All servers communicate through `StreamCenter`, a central event hub managing stream publish/subscribe events
- **Unified I/O Abstraction**: `UnifiedIO` trait abstracts TCP/UDP/Channel transports with consistent async Stream/Sink interface
- **Protocol Adapters**: `StreamCenter` provides adaptors (FLV, RTP) for protocol translation
- **GOP (Group of Pictures) Management**: Built-in GOP cache and management for efficient stream distribution

## Supported Protocols

### Publishing Protocols

| Protocol | Port | Description |
|----------|------|-------------|
| **RTMP** | 1935 | Real-Time Messaging Protocol for flash-compatible streaming |
| **RTSP** | 554 | Real-Time Streaming Protocol with RTP transport |
| **WebRTC (WIP)** | (via HTTP) | WHIP endpoint for WebRTC ingest |

### Playback Protocols

| Protocol | Endpoint | Description |
|----------|----------|-------------|
| **HTTP-FLV** | `/live_stream/v1/{app}/{stream}` | FLV streaming over HTTP |
| **RTSP** | rtsp:// | RTSP play with RTP transport |
| **WebRTC (WIP)** | (future) | WebRTC playback via WHEP |

### Network Protocols

| Protocol | Usage |
|----------|-------|
| **STUN** | NAT traversal for WebRTC (RFC 5389) |
| **TURN** | Relay server for NAT traversal (RFC 8656) |
| **RTP/RTCP** | Media transport with RTCP feedback |

## Supported Codecs

### Video Codecs

| Codec | Support | Features |
|-------|---------|----------|
| **H.264/AVC** | ✅ Full | SPS/PPS parsing, NALU processing, AVCC format, VUI parameters, Scaling lists |
| **H.264 Bitstream** | ✅ Parser | Bit-level reading/writing |

### Audio Codecs

| Codec | Support | Features |
|-------|---------|----------|
| **AAC** | ✅ Full | MPEG-4 Audio Specific Config, all profiles (LC, SBR, PS,ELD, etc.) |
| **AAC Parser** | ✅ Complete | GA, SSC, TTS, CELP, HVXC, DST, ALS, SLS specific configs |

### Codec Framework

```
codec/
├── common/          # Shared audio/video traits
│   ├── audio/       # AudioCodecCommon, AudioConfig
│   └── video/       # VideoCodecCommon, VideoConfig
├── h264/            # Complete H.264 implementation
│   ├── sps/         # Sequence Parameter Set parsing/generation
│   ├── pps/         # Picture Parameter Set
│   ├── nalu/        # NAL Unit handling
│   ├── vui/         # Video Usability Information
│   └── avc_config/  # AVC Decoder Configuration Record
└── aac/             # Complete AAC implementation
    └── mpeg4_configuration/  # All audio object types
```

## Format Specifications

### Message Formats

| Format | Support | Features |
|--------|---------|----------|
| **RTMP** | ✅ Complete | Handshake, Chunking, Commands, AMF0/AMF1, User Control |
| **FLV** | ✅ Complete | Header, Tags, OnMetaData, Script/Video/Audio tags |
| **RTSP** | ✅ Complete | Request/Response parsing, Headers, SDP integration |
| **RTP** | ✅ Complete | Packet building, Payload formatting, RTCP SR/RR/SDES |
| **AMF** | ✅ Complete | AMF0/AMF3 encoding/decoding |
| **SDP** | ✅ Complete | Session description, Media attributes, RTP map |
| **WebRTC** | ✅ SDP | SDP extensions for WebRTC (RFC 8830) |
| **STUN/TURN** | ✅ Complete | Attributes, Channel Data, RFC 8656 compliance |

## Key Components

### StreamCenter

The central hub managing all stream operations:

```rust
// Publish a stream from any protocol
StreamCenter::publish(
    &event_sender,
    PublishProtocol::RTMP,
    &stream_id,
    &context,
).await?;

// Subscribe to receive frames
let response = StreamCenter::subscribe(
    &event_sender,
    PlayProtocol::HTTP_FLV,
    &stream_id,
    &context,
).await?;

// Get stream information
let description = StreamCenter::describe(&event_sender, &stream_id).await?;
```

### Unified I/O

Abstracted transport layer:

```rust
// Single interface for TCP, UDP, Channel
pub trait UnifiedIO:
    Stream<Item = Result<Bytes, std::io::Error>>
    + Sink<Bytes, Error = std::io::Error>>

// Used with custom codecs
let stream = UnifiyStreamed::new(io, codec);
```

### Event System

```rust
pub enum StreamCenterEvent {
    Publish { protocol, stream_id, context, result_sender },
    Unpublish { stream_id, result_sender },
    Subscribe { stream_id, protocol, result_sender, context },
    Unsubscribe { stream_id, uuid, result_sender },
    Describe { stream_id, result_sender },
}
```

## Server Configuration

### RTMP Server

```toml
[rtmp_server]
enable = true
address = "0.0.0.0"
port = 1935
chunk_size = 6000
write_timeout_ms = 6000
read_timeout_ms = 6000
```

### HTTP Server

```toml
[http_server]
enable = true
address = "0.0.0.0"
port = 80
workers = 10
```

### RTSP Server

```toml
[rtsp_server]
enable = true
address = "0.0.0.0"
port = 554
```

## Building

```bash
# Build all workspace crates
cargo build --workspace

# Build release
cargo build --workspace --release

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace

# Format code
cargo fmt
```

## Project Structure

```
media-server/
├── codec/              # Codec implementations
│   ├── aac/           # AAC audio codec
│   ├── bitstream/     # Bitstream utilities
│   ├── common/        # Shared codec traits
│   └── h264/          # H.264 video codec
├── formats/           # Protocol format implementations
│   ├── amf/           # AMF0/AMF3
│   ├── flv/           # FLV container
│   ├── rtmp/          # RTMP protocol
│   ├── rtp/           # RTP/RTCP
│   ├── rtsp/          # RTSP protocol
│   ├── sdp/           # SDP description
│   ├── stun/          # STUN format
│   ├── turn/          # TURN format
│   ├── webrtc/        # WebRTC SDP extensions
│   └── iana/          # IANA constants
├── servers/           # Server implementations
│   ├── http/          # HTTP server (HTTP-FLV, WHIP)
│   ├── rtmp/          # RTMP server
│   ├── rtp/           # RTP session management
│   ├── rtsp/          # RTSP server
│   ├── stun/          # STUN server
│   ├── turn/          # TURN relay server
│   └── utils/         # Server utilities
├── streamcenter/      # Central stream management
│   ├── adaptors/      # Protocol adaptors (FLV, RTP)
│   ├── events/        # Event definitions
│   ├── gop/           # GOP management
│   └── stream_center/ # Core stream hub
├── unifiedio/         # Unified I/O abstraction
├── connection/        # Connection utilities
├── utils/             # General utilities
├── yam_server/        # Main server binary
└── src/               # Standalone binaries
    ├── media_server.rs
    ├── stun_client.rs
    └── stun_server.rs
```

## Dependencies

- **tokio**: Async runtime with full features and tracing
- **tracing**: Structured logging
- **thiserror**: Error type derivation
- **serde**: Serialization
- **clap**: CLI argument parsing
- **axum**: HTTP server framework

## License

MIT License - See LICENSE file for details
