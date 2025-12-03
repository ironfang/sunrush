# Deployment Guide

**Version:** 1.0  
**Last Updated:** December 3, 2025

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)
4. [Running SunRush](#running-sunrush)
5. [Production Deployment](#production-deployment)
6. [Security](#security)
7. [Maintenance](#maintenance)
8. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

**Minimum:**
- CPU: 4 cores (x86_64)
- RAM: 8 GB
- Storage: 50 GB SSD
- Network: 100 Mbps stable connection
- OS: Linux (Ubuntu 22.04+ recommended) or macOS

**Recommended (Production):**
- CPU: 16+ cores (x86_64)
- RAM: 32 GB
- Storage: 500 GB NVMe SSD
- Network: 1 Gbps dedicated connection
- OS: Ubuntu 22.04 LTS

### Software Dependencies

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update
rustup default stable

# Build tools
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    git

# Optional: Monitoring tools
sudo apt-get install -y \
    prometheus \
    grafana
```

### Jito ShredStream Access

- Active Jito ShredStream subscription
- Authentication token
- gRPC endpoint URL

Contact [Jito Labs](https://jito.wtf/) for access.

---

## Installation

### Option 1: Build from Source

```bash
# Clone repository
git clone https://github.com/ironfang/sunrush.git
cd sunrush

# Build host and all plugins
cargo build --release

# Install binaries
sudo mkdir -p /opt/sunrush/{bin,plugins,config}
sudo cp target/release/sunrush-host /opt/sunrush/bin/
sudo cp target/release/*.so /opt/sunrush/plugins/

# Create service user
sudo useradd -r -s /bin/false sunrush
sudo chown -R sunrush:sunrush /opt/sunrush
```

### Option 2: Pre-built Binaries

```bash
# Download latest release
wget https://github.com/ironfang/sunrush/releases/latest/download/sunrush-linux-x64.tar.gz

# Extract
tar -xzf sunrush-linux-x64.tar.gz
cd sunrush

# Install
sudo ./install.sh
```

### Verify Installation

```bash
/opt/sunrush/bin/sunrush-host --version
# SunRush Host v1.0.0

# Check plugins
ls -lh /opt/sunrush/plugins/
# sunrush_shred_ingest.so
# sunrush_block_assembler.so
# sunrush_tx_extractor.so
# sunrush_core.so
```

---

## Configuration

### Main Configuration File

Create `/opt/sunrush/config/config.toml`:

```toml
# ========================================
# SunRush Host Configuration
# ========================================

[host]
# Plugin directory
plugin_dir = "/opt/sunrush/plugins"

# Message bus capacity (per channel)
bus_capacity = 10000

# Enable hot reload
hot_reload = true

# Worker threads (0 = number of CPU cores)
worker_threads = 0

# ========================================
# Telemetry
# ========================================

[telemetry]
# Enable Prometheus metrics endpoint
enabled = true

# Metrics HTTP server port
port = 9090

# Bind address
bind = "0.0.0.0"

# ========================================
# Logging
# ========================================

[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log format: json, pretty, compact
format = "json"

# Log output: stdout, file
output = "stdout"

# Log file path (if output = "file")
file_path = "/var/log/sunrush/sunrush.log"

# Rotate logs daily
rotate = true

# Keep 30 days of logs
retention_days = 30

# ========================================
# Plugin: ShredIngest
# ========================================

[plugins.shred_ingest]
# Jito ShredStream endpoint
jito_endpoint = "grpc://shredstream.jito.wtf:10000"

# Authentication token
auth_token = "${JITO_AUTH_TOKEN}"

# Connection timeout (seconds)
connection_timeout = 30

# Reconnect on disconnect
auto_reconnect = true

# Reconnect delay (seconds)
reconnect_delay = 5

# Max reconnect attempts (0 = infinite)
max_reconnect_attempts = 0

# ========================================
# Plugin: BlockAssembler
# ========================================

[plugins.block_assembler]
# Maximum slots to keep in memory
max_slots_in_memory = 100

# Slot timeout (milliseconds)
slot_timeout_ms = 5000

# Expected shreds per slot
expected_shreds_per_slot = 128

# Enable streaming entry emission
streaming_entries = true

# ========================================
# Plugin: TxExtractor
# ========================================

[plugins.tx_extractor]
# Number of decode worker tasks
max_decode_workers = 4

# Enable parallel entry processing
parallel_processing = true

# Decode batch size
batch_size = 10

# ========================================
# Plugin: Core
# ========================================

[plugins.core]
# Strategy configuration file
strategy_config = "/opt/sunrush/config/strategies.toml"

# Enable filtering
enable_filters = true

# Filter configuration
[plugins.core.filters]
# Filter by program IDs
program_ids = [
    "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB",  # Jupiter
    "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",  # Orca Whirlpool
]

# Filter by accounts
accounts = []

# Minimum transaction size (bytes)
min_tx_size = 0

# Maximum transaction size (bytes)
max_tx_size = 1232
```

### Environment Variables

Create `/opt/sunrush/config/env`:

```bash
# Jito credentials
export JITO_AUTH_TOKEN="your-token-here"

# Rust settings
export RUST_LOG=info
export RUST_BACKTRACE=1

# Performance tuning
export MALLOC_ARENA_MAX=2
export MALLOC_MMAP_THRESHOLD_=131072
```

### Strategy Configuration

Create `/opt/sunrush/config/strategies.toml`:

```toml
[[strategy]]
name = "jupiter_swaps"
enabled = true

[strategy.filter]
program_id = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB"

[strategy.action]
type = "webhook"
url = "https://your-api.com/webhook/jupiter"
method = "POST"

[[strategy]]
name = "large_transactions"
enabled = true

[strategy.filter]
min_accounts = 10

[strategy.action]
type = "log"
level = "info"

[[strategy]]
name = "critical_accounts"
enabled = true

[strategy.filter]
accounts = [
    "SysvarC1ock11111111111111111111111111111111",
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
]

[strategy.action]
type = "alert"
channel = "slack"
webhook_url = "${SLACK_WEBHOOK_URL}"
```

---

## Running SunRush

### Manual Start

```bash
# Load environment
source /opt/sunrush/config/env

# Run host
/opt/sunrush/bin/sunrush-host \
    --config /opt/sunrush/config/config.toml
```

### Systemd Service

Create `/etc/systemd/system/sunrush.service`:

```ini
[Unit]
Description=SunRush - Solana Transaction Extraction System
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=sunrush
Group=sunrush

# Environment
EnvironmentFile=/opt/sunrush/config/env

# Execution
ExecStart=/opt/sunrush/bin/sunrush-host --config /opt/sunrush/config/config.toml
ExecReload=/bin/kill -HUP $MAINPID

# Restart on failure
Restart=always
RestartSec=10

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=sunrush

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/sunrush /var/log/sunrush

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable sunrush
sudo systemctl start sunrush

# Check status
sudo systemctl status sunrush

# View logs
sudo journalctl -u sunrush -f
```

---

## Production Deployment

### High Availability Setup

#### Load Balancer Configuration

Use HAProxy or nginx for multiple SunRush instances:

```nginx
# /etc/nginx/nginx.conf
upstream sunrush_cluster {
    least_conn;
    server 10.0.1.10:9090 max_fails=3 fail_timeout=30s;
    server 10.0.1.11:9090 max_fails=3 fail_timeout=30s;
    server 10.0.1.12:9090 max_fails=3 fail_timeout=30s;
}

server {
    listen 9090;
    
    location /metrics {
        proxy_pass http://sunrush_cluster;
        proxy_set_header Host $host;
    }
}
```

#### Kubernetes Deployment

```yaml
# sunrush-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sunrush
  namespace: sunrush
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sunrush
  template:
    metadata:
      labels:
        app: sunrush
    spec:
      containers:
      - name: sunrush
        image: sunrush:1.0.0
        ports:
        - containerPort: 9090
          name: metrics
        env:
        - name: JITO_AUTH_TOKEN
          valueFrom:
            secretKeyRef:
              name: sunrush-secrets
              key: jito-token
        resources:
          requests:
            memory: "16Gi"
            cpu: "8"
          limits:
            memory: "32Gi"
            cpu: "16"
        volumeMounts:
        - name: config
          mountPath: /opt/sunrush/config
        - name: plugins
          mountPath: /opt/sunrush/plugins
        livenessProbe:
          httpGet:
            path: /metrics
            port: 9090
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /metrics
            port: 9090
          initialDelaySeconds: 10
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: sunrush-config
      - name: plugins
        persistentVolumeClaim:
          claimName: sunrush-plugins

---
apiVersion: v1
kind: Service
metadata:
  name: sunrush-metrics
  namespace: sunrush
spec:
  selector:
    app: sunrush
  ports:
  - port: 9090
    targetPort: 9090
    name: metrics
  type: ClusterIP
```

### Performance Tuning

#### System Settings

```bash
# /etc/sysctl.d/99-sunrush.conf

# Network
net.core.rmem_max = 268435456
net.core.wmem_max = 268435456
net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728
net.core.netdev_max_backlog = 5000

# File descriptors
fs.file-max = 2097152

# Virtual memory
vm.swappiness = 10
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5

# Apply
sudo sysctl -p /etc/sysctl.d/99-sunrush.conf
```

#### CPU Pinning

Pin SunRush to specific CPU cores:

```bash
# Use cores 0-15 for SunRush
taskset -c 0-15 /opt/sunrush/bin/sunrush-host --config config.toml
```

Or in systemd:

```ini
[Service]
CPUAffinity=0-15
```

#### NUMA Awareness

For multi-socket systems:

```bash
# Run on NUMA node 0
numactl --cpunodebind=0 --membind=0 \
    /opt/sunrush/bin/sunrush-host --config config.toml
```

---

## Security

### Network Security

#### Firewall Rules

```bash
# Allow metrics endpoint (internal only)
sudo ufw allow from 10.0.0.0/8 to any port 9090

# Deny external access
sudo ufw deny 9090

# Allow Jito ShredStream (outbound)
sudo ufw allow out to shredstream.jito.wtf port 10000
```

#### TLS for Metrics

Use reverse proxy with TLS:

```nginx
server {
    listen 9443 ssl;
    
    ssl_certificate /etc/ssl/certs/sunrush.crt;
    ssl_certificate_key /etc/ssl/private/sunrush.key;
    
    location /metrics {
        proxy_pass http://127.0.0.1:9090;
        
        # Authentication
        auth_basic "SunRush Metrics";
        auth_basic_user_file /etc/nginx/.htpasswd;
    }
}
```

### Secrets Management

#### Vault Integration

```bash
# Install Vault
sudo apt-get install vault

# Store Jito token
vault kv put secret/sunrush jito_token="your-token"

# Retrieve in systemd
[Service]
ExecStartPre=/usr/local/bin/get-vault-token.sh
EnvironmentFile=/run/sunrush/env
```

#### Encrypted Configuration

```bash
# Encrypt config
gpg --encrypt --recipient sunrush@example.com config.toml

# Decrypt on startup
gpg --decrypt config.toml.gpg > /tmp/config.toml
```

### Process Isolation

#### AppArmor Profile

```
# /etc/apparmor.d/opt.sunrush.bin.sunrush-host
#include <tunables/global>

/opt/sunrush/bin/sunrush-host {
  #include <abstractions/base>
  #include <abstractions/nameservice>
  
  /opt/sunrush/** r,
  /opt/sunrush/plugins/*.so rm,
  /var/log/sunrush/** rw,
  
  network inet stream,
  network inet6 stream,
  
  deny /proc/** rw,
  deny /sys/** rw,
}
```

---

## Maintenance

### Backup

```bash
#!/bin/bash
# /opt/sunrush/scripts/backup.sh

BACKUP_DIR="/backup/sunrush"
DATE=$(date +%Y%m%d_%H%M%S)

# Backup configuration
tar -czf "$BACKUP_DIR/config_$DATE.tar.gz" /opt/sunrush/config/

# Backup plugins
tar -czf "$BACKUP_DIR/plugins_$DATE.tar.gz" /opt/sunrush/plugins/

# Backup logs (last 7 days)
find /var/log/sunrush -mtime -7 -type f | \
    tar -czf "$BACKUP_DIR/logs_$DATE.tar.gz" -T -

# Clean old backups (keep 30 days)
find "$BACKUP_DIR" -mtime +30 -delete
```

### Updates

#### Plugin Hot Reload

```bash
# Build new plugin
cd sunrush
cargo build -p sunrush-tx-extractor --release

# Copy to plugin directory (auto-reload)
sudo cp target/release/libsunrush_tx_extractor.so \
    /opt/sunrush/plugins/

# Verify reload
sudo journalctl -u sunrush -n 50 | grep "reload"
```

#### Full Update

```bash
# Download new version
wget https://github.com/ironfang/sunrush/releases/download/v1.1.0/sunrush-linux-x64.tar.gz

# Extract
tar -xzf sunrush-linux-x64.tar.gz

# Stop service
sudo systemctl stop sunrush

# Backup
sudo cp -r /opt/sunrush /opt/sunrush.backup

# Install
sudo cp sunrush/bin/* /opt/sunrush/bin/
sudo cp sunrush/plugins/* /opt/sunrush/plugins/

# Start service
sudo systemctl start sunrush

# Verify
sudo systemctl status sunrush
curl http://localhost:9090/metrics
```

### Log Rotation

```bash
# /etc/logrotate.d/sunrush
/var/log/sunrush/*.log {
    daily
    rotate 30
    compress
    delaycompress
    notifempty
    create 0644 sunrush sunrush
    sharedscripts
    postrotate
        systemctl reload sunrush > /dev/null 2>&1 || true
    endscript
}
```

---

## Troubleshooting

### Common Issues

#### 1. Connection to Jito Failed

```
Error: Failed to connect to Jito ShredStream
```

**Solution:**
- Verify auth token: `echo $JITO_AUTH_TOKEN`
- Check network connectivity: `telnet shredstream.jito.wtf 10000`
- Verify firewall rules
- Check Jito status page

#### 2. Plugin Load Failed

```
Error: Failed to load plugin: undefined symbol
```

**Solution:**
- Verify ABI version: `strings libplugin.so | grep ABI`
- Check dependencies: `ldd /opt/sunrush/plugins/libplugin.so`
- Rebuild plugin with correct SDK version

#### 3. High Memory Usage

```
Memory usage: 28GB / 32GB
```

**Solution:**
- Reduce `max_slots_in_memory` in config
- Lower `bus_capacity`
- Enable log rotation
- Check for memory leaks with `valgrind`

#### 4. Bus Backpressure

```
WARN: Bus lagged by 5000 messages
```

**Solution:**
- Increase `bus_capacity`
- Optimize slow plugins
- Add more worker threads
- Scale horizontally

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=debug,sunrush=trace

# Run with backtrace
export RUST_BACKTRACE=full

# Start host
/opt/sunrush/bin/sunrush-host --config config.toml
```

### Health Check Script

```bash
#!/bin/bash
# /opt/sunrush/scripts/health-check.sh

# Check process
if ! pgrep -x sunrush-host > /dev/null; then
    echo "ERROR: SunRush is not running"
    exit 1
fi

# Check metrics endpoint
if ! curl -sf http://localhost:9090/metrics > /dev/null; then
    echo "ERROR: Metrics endpoint unreachable"
    exit 1
fi

# Check recent activity
RECENT_TX=$(curl -s http://localhost:9090/metrics | \
    grep sunrush_tx_extracted_total | \
    awk '{print $2}')

if [ "$RECENT_TX" -eq 0 ]; then
    echo "WARN: No transactions processed recently"
fi

echo "OK: SunRush is healthy"
exit 0
```

### Prometheus Alerts

```yaml
# sunrush-alerts.yml
groups:
- name: sunrush
  interval: 30s
  rules:
  - alert: SunRushDown
    expr: up{job="sunrush"} == 0
    for: 1m
    annotations:
      summary: "SunRush instance down"
  
  - alert: HighBusBackpressure
    expr: rate(sunrush_bus_backpressure_total[1m]) > 100
    for: 5m
    annotations:
      summary: "High message bus backpressure"
  
  - alert: HighLatency
    expr: histogram_quantile(0.95, sunrush_tx_extract_latency_ms) > 10
    for: 5m
    annotations:
      summary: "High transaction extraction latency"
```

---

## Performance Benchmarks

Expected performance on recommended hardware:

| Metric | Value |
|--------|-------|
| Shreds/sec | 50,000 - 100,000 |
| Transactions/sec | 10,000 - 20,000 |
| Latency p50 | <2ms |
| Latency p95 | <6ms |
| Latency p99 | <12ms |
| Memory Usage | 8-16 GB |
| CPU Usage | 40-60% (16 cores) |

---

## Support

- **Documentation**: [docs/](.)
- **Issues**: [GitHub Issues](https://github.com/ironfang/sunrush/issues)
- **Community**: [Discord](https://discord.gg/sunrush)
- **Email**: support@sunrush.io

---

**Deployment guide complete. Good luck with your SunRush deployment! 🚀**
