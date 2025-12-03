# SunRush

**Ultra-low-latency Solana transaction extraction system**

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

SunRush is a high-performance, modular system designed to ingest **Solana shreds** in real time via Jito ShredStream, assemble slot data, and extract transactions with minimal latency. Built as a single host process with hot-reloadable plugin architecture.

## ⚡ Key Features

- **Ultra-Low Latency**: p50 < 2ms from shred to transaction
- **Hot-Reloadable Plugins**: Update components without downtime
- **Streaming Pipeline**: Extract transactions as soon as bytes are available
- **Full Observability**: Prometheus metrics + structured logging
- **Zero-Copy Design**: Minimal allocations for maximum throughput
- **Modular Architecture**: Independent `.so` plugins with stable ABI

## 🏗️ Architecture Overview

```
Jito ShredStream → ShredIngest → BlockAssembler → TxExtractor → Core
                        ↓              ↓              ↓          ↓
                    [In-Process Message Bus (Tokio Broadcast)]
                                       ↓
                            [Telemetry & Logging]
```

### Core Components

- **Host Process**: Manages runtime, plugin lifecycle, and shared resources
- **ShredIngest Plugin**: Ingests raw shreds from Jito ShredStream
- **BlockAssembler Plugin**: Streaming slot assembly with immediate entry emission
- **TxExtractor Plugin**: Zero-copy transaction decoding
- **Core Plugin**: Strategy and filtering logic

## 🚀 Quick Start

### Prerequisites

- Rust 1.70 or later
- Jito ShredStream access credentials
- Linux (recommended) or macOS

### Building

```bash
# Build the host and all plugins
cargo build --release

# Build specific plugin
cargo build -p sunrush-shred-ingest --release
```

### Running

```bash
# Start the host process
./target/release/sunrush-host --config config.toml

# Access metrics
curl http://localhost:9090/metrics
```

### Configuration

Create a `config.toml`:

```toml
[host]
plugin_dir = "/opt/sunrush/plugins"
bus_capacity = 10000
hot_reload = true

[telemetry]
port = 9090
enabled = true

[logging]
level = "info"
format = "json"

[plugins.shred_ingest]
jito_endpoint = "grpc://shredstream.jito.wtf:10000"
auth_token = "your-token-here"

[plugins.block_assembler]
slot_timeout_ms = 5000
max_slots_in_memory = 100

[plugins.tx_extractor]
max_decode_workers = 4
```

## 📊 Performance

### Latency Targets

| Metric | p50 | p95 | p99 |
|--------|-----|-----|-----|
| Shred → Transaction | <2ms | <6ms | <12ms |
| Entry Assembly | <3ms | <5ms | <8ms |
| Transaction Decode | <1ms | <2ms | <4ms |

### Throughput

- **50k-100k shreds/sec**
- **10k-20k transactions/sec**
- Bounded memory usage per slot

## 📚 Documentation

- [Architecture](docs/ARCHITECTURE.md) - Detailed system design
- [Plugin Development](docs/PLUGIN_DEVELOPMENT.md) - Creating custom plugins
- [API Reference](docs/API_REFERENCE.md) - ABI interface documentation
- [Deployment](docs/DEPLOYMENT.md) - Production deployment guide
- [Monitoring](docs/MONITORING.md) - Observability and troubleshooting

## 🔌 Plugin Development

SunRush plugins are `.so` libraries that implement a stable ABI:

```rust
#[no_mangle]
pub extern "C" fn mef_component_init(
    callbacks: *const HostCallbacks,
    config: *const c_char,
) -> *mut c_void {
    // Plugin initialization
}

#[no_mangle]
pub extern "C" fn mef_component_start(handle: *mut c_void) -> i32 {
    // Start processing
}
```

See [Plugin Development Guide](docs/PLUGIN_DEVELOPMENT.md) for details.

## 📈 Monitoring

### Prometheus Metrics

```
# Shred ingestion
sunrush_shreds_received_total
sunrush_ingest_latency_ms

# Block assembly
sunrush_slot_entries_emitted_total
sunrush_blocks_assembled_total
sunrush_assemble_entry_latency_ms

# Transaction extraction
sunrush_tx_extracted_total
sunrush_tx_extract_latency_ms
sunrush_tx_failures_total

# System health
sunrush_queue_depth
sunrush_bus_backpressure_total
```

### Structured Logging

All components emit JSON logs with fields:

- `component`: Plugin name
- `slot`: Slot number
- `latency_ms`: Operation latency
- `tx_signature`: Transaction signature (when applicable)

## 🛠️ Development

### Project Structure

```
sunrush/
├── crates/
│   ├── host/           # Main host process
│   ├── abi/            # Stable ABI definitions
│   ├── types/          # Shared data structures
│   ├── bus/            # Message bus implementation
│   └── plugin-sdk/     # Plugin development SDK
├── plugins/
│   ├── shred-ingest/   # Jito ShredStream reader
│   ├── block-assembler/# Streaming slot assembly
│   ├── tx-extractor/   # Transaction decoder
│   └── core/           # Strategy logic
├── docs/               # Documentation
└── examples/           # Example plugins
```

### Running Tests

```bash
# Run all tests
cargo test --all

# Run with logging
RUST_LOG=debug cargo test

# Run benchmarks
cargo bench
```

### Hot Reload in Development

```bash
# Terminal 1: Run host with hot reload
cargo run --release

# Terminal 2: Rebuild plugin
cargo build -p sunrush-tx-extractor --release

# Plugin automatically reloads
```

## 🔐 Security Considerations

- Plugins run in the same process (no sandboxing)
- Validate all external inputs
- Use authentication for Jito ShredStream
- Monitor resource usage per plugin
- Implement rate limiting on public endpoints

## 🗺️ Roadmap

- [x] Core host implementation
- [x] Plugin ABI v1.0
- [x] ShredIngest plugin
- [ ] BlockAssembler plugin
- [ ] TxExtractor plugin
- [ ] Core strategy plugin
- [ ] Plugin SDK with examples
- [ ] Distributed multi-node support
- [ ] GPU-accelerated decoding
- [ ] Historical block reconstruction

## 🤝 Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) first.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Jito Labs](https://jito.wtf/) for ShredStream access
- Solana Foundation for the core protocol
- The Rust community for excellent tooling

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/ironfang/sunrush/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ironfang/sunrush/discussions)
- **Documentation**: [docs/](docs/)

---

**Built with ⚡ by the SunRush team**
