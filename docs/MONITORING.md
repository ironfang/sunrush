# Monitoring and Observability Guide

**Version:** 1.0  
**Last Updated:** December 3, 2025

## Table of Contents

1. [Overview](#overview)
2. [Metrics](#metrics)
3. [Logging](#logging)
4. [Dashboards](#dashboards)
5. [Alerting](#alerting)
6. [Tracing](#tracing)
7. [Performance Analysis](#performance-analysis)

---

## Overview

SunRush provides comprehensive observability through:

- **Prometheus Metrics**: Real-time performance and health metrics
- **Structured Logging**: JSON-formatted logs with rich context
- **Distributed Tracing**: Request flow through the pipeline (optional)
- **Custom Dashboards**: Pre-built Grafana dashboards

### Observability Stack

```
┌─────────────┐
│  SunRush    │
│   Host      │
└──────┬──────┘
       │
       ├─→ Prometheus (:9090/metrics)
       │   └─→ Grafana (visualization)
       │       └─→ Alerts
       │
       ├─→ Logs (stdout/file)
       │   └─→ Loki/ELK
       │
       └─→ Traces (optional)
           └─→ Jaeger/Tempo
```

---

## Metrics

### Metrics Endpoint

SunRush exposes Prometheus metrics at:

```
http://localhost:9090/metrics
```

### Metric Categories

#### 1. Shred Ingestion Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `sunrush_shreds_received_total` | Counter | Total shreds received from Jito |
| `sunrush_shreds_invalid_total` | Counter | Invalid/malformed shreds |
| `sunrush_ingest_latency_ms` | Histogram | Latency from network to bus (ms) |
| `sunrush_jito_connection_status` | Gauge | Connection status (1=connected, 0=disconnected) |
| `sunrush_jito_reconnects_total` | Counter | Number of reconnection attempts |

**Example Queries:**

```promql
# Shred ingestion rate
rate(sunrush_shreds_received_total[1m])

# P95 ingestion latency
histogram_quantile(0.95, rate(sunrush_ingest_latency_ms_bucket[5m]))

# Invalid shred rate
rate(sunrush_shreds_invalid_total[1m]) / rate(sunrush_shreds_received_total[1m])
```

#### 2. Block Assembly Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `sunrush_slot_entries_emitted_total` | Counter | Total slot entries emitted |
| `sunrush_blocks_assembled_total` | Counter | Total complete blocks assembled |
| `sunrush_assemble_entry_latency_ms` | Histogram | Entry assembly latency (ms) |
| `sunrush_assemble_block_latency_ms` | Histogram | Full block assembly latency (ms) |
| `sunrush_active_slots` | Gauge | Number of slots currently in memory |
| `sunrush_slots_evicted_total` | Counter | Slots evicted due to timeout/memory |
| `sunrush_missing_shreds_total` | Counter | Missing shreds detected |

**Example Queries:**

```promql
# Entry emission rate
rate(sunrush_slot_entries_emitted_total[1m])

# Average active slots
avg_over_time(sunrush_active_slots[5m])

# Block completion rate
rate(sunrush_blocks_assembled_total[1m])

# P99 entry latency
histogram_quantile(0.99, rate(sunrush_assemble_entry_latency_ms_bucket[5m]))
```

#### 3. Transaction Extraction Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `sunrush_tx_extracted_total` | Counter | Total transactions extracted |
| `sunrush_tx_extract_latency_ms` | Histogram | Transaction extraction latency (ms) |
| `sunrush_tx_failures_total` | Counter | Transaction decode failures |
| `sunrush_tx_by_program_total` | Counter | Transactions by program ID (label: `program`) |
| `sunrush_tx_size_bytes` | Histogram | Transaction size distribution |

**Example Queries:**

```promql
# Transaction extraction rate
rate(sunrush_tx_extracted_total[1m])

# P50/P95/P99 extraction latency
histogram_quantile(0.50, rate(sunrush_tx_extract_latency_ms_bucket[5m]))
histogram_quantile(0.95, rate(sunrush_tx_extract_latency_ms_bucket[5m]))
histogram_quantile(0.99, rate(sunrush_tx_extract_latency_ms_bucket[5m]))

# Top programs by transaction count
topk(10, rate(sunrush_tx_by_program_total[5m]))

# Error rate
rate(sunrush_tx_failures_total[1m]) / rate(sunrush_tx_extracted_total[1m])
```

#### 4. Pipeline Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `sunrush_bus_messages_sent_total` | Counter | Messages sent to bus (label: `type`) |
| `sunrush_bus_messages_dropped_total` | Counter | Messages dropped (no receivers) |
| `sunrush_bus_backpressure_total` | Counter | Backpressure events (lag) |
| `sunrush_queue_depth` | Gauge | Current message queue depth |
| `sunrush_pipeline_latency_ms` | Histogram | End-to-end latency (shred→tx) |

**Example Queries:**

```promql
# Message throughput by type
sum(rate(sunrush_bus_messages_sent_total[1m])) by (type)

# Bus saturation
sunrush_queue_depth / 10000  # Assuming capacity = 10000

# Backpressure rate
rate(sunrush_bus_backpressure_total[1m])

# End-to-end P95 latency
histogram_quantile(0.95, rate(sunrush_pipeline_latency_ms_bucket[5m]))
```

#### 5. System Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `sunrush_plugin_load_total` | Counter | Plugin load/reload events |
| `sunrush_plugin_panics_total` | Counter | Plugin panics/crashes |
| `sunrush_plugin_status` | Gauge | Plugin status (label: `plugin`) |
| `sunrush_memory_bytes` | Gauge | Memory usage by component |
| `sunrush_cpu_usage_percent` | Gauge | CPU usage percentage |

**Example Queries:**

```promql
# Plugin health
sunrush_plugin_status

# Memory usage trend
rate(sunrush_memory_bytes[5m])

# Plugin crash rate
rate(sunrush_plugin_panics_total[1h])
```

### Custom Metrics from Plugins

Plugins can emit custom metrics:

```rust
// In your plugin
host.counter_inc("my_plugin_events_total");
host.gauge_set("my_plugin_queue_size", 42.0);
host.histogram_observe("my_plugin_processing_ms", 3.5);
```

These appear as:

```
sunrush_my_plugin_events_total
sunrush_my_plugin_queue_size
sunrush_my_plugin_processing_ms
```

---

## Logging

### Log Format

SunRush uses structured JSON logging by default:

```json
{
  "timestamp": "2025-12-03T10:15:30.123Z",
  "level": "INFO",
  "component": "tx-extractor",
  "message": "Transaction extracted",
  "slot": 245123456,
  "tx_signature": "3Xm...",
  "latency_ms": 1.234,
  "accounts": 5,
  "instructions": 2
}
```

### Log Levels

Configure via `RUST_LOG` environment variable:

```bash
# All components at INFO level
export RUST_LOG=info

# Specific component at DEBUG
export RUST_LOG=sunrush_tx_extractor=debug

# Multiple levels
export RUST_LOG=info,sunrush_block_assembler=debug,sunrush_core=trace

# Include dependencies
export RUST_LOG=info,tokio=debug
```

### Log Outputs

#### 1. Standard Output (Default)

```toml
[logging]
output = "stdout"
format = "json"
```

Redirect to file:

```bash
/opt/sunrush/bin/sunrush-host 2>&1 | tee /var/log/sunrush/sunrush.log
```

#### 2. File Output

```toml
[logging]
output = "file"
file_path = "/var/log/sunrush/sunrush.log"
rotate = true
retention_days = 30
```

#### 3. Journald (systemd)

```toml
[logging]
output = "journald"
```

Query logs:

```bash
# All logs
journalctl -u sunrush

# Follow live
journalctl -u sunrush -f

# Filter by level
journalctl -u sunrush -p err

# JSON output
journalctl -u sunrush -o json-pretty
```

### Log Aggregation

#### Loki Integration

```yaml
# promtail-config.yml
server:
  http_listen_port: 9080

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: sunrush
    static_configs:
      - targets:
          - localhost
        labels:
          job: sunrush
          __path__: /var/log/sunrush/*.log
    pipeline_stages:
      - json:
          expressions:
            level: level
            component: component
            slot: slot
      - labels:
          level:
          component:
```

#### Elasticsearch Integration

```bash
# Ship logs to Elasticsearch
filebeat -e -c filebeat.yml
```

```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/sunrush/*.log
    json.keys_under_root: true
    json.add_error_key: true

output.elasticsearch:
  hosts: ["localhost:9200"]
  index: "sunrush-%{+yyyy.MM.dd}"
```

### Important Log Patterns

#### Error Tracking

```bash
# All errors
journalctl -u sunrush | grep '"level":"ERROR"'

# Plugin crashes
journalctl -u sunrush | grep "panicked"

# Connection failures
journalctl -u sunrush | grep "connection.*failed"
```

#### Performance Analysis

```bash
# High latency events
journalctl -u sunrush | jq 'select(.latency_ms > 10)'

# Slow transactions
journalctl -u sunrush | jq 'select(.component == "tx-extractor" and .latency_ms > 5)'
```

---

## Dashboards

### Grafana Setup

#### Install Grafana

```bash
sudo apt-get install -y software-properties-common
sudo add-apt-repository "deb https://packages.grafana.com/oss/deb stable main"
wget -q -O - https://packages.grafana.com/gpg.key | sudo apt-key add -
sudo apt-get update
sudo apt-get install grafana

sudo systemctl enable grafana-server
sudo systemctl start grafana-server
```

Access: `http://localhost:3000` (admin/admin)

#### Configure Prometheus Data Source

1. Navigate to **Configuration → Data Sources**
2. Add **Prometheus**
3. URL: `http://localhost:9090`
4. Save & Test

### Pre-built Dashboards

#### Dashboard 1: System Overview

**Panels:**

1. **Shred Ingestion Rate**
   ```promql
   sum(rate(sunrush_shreds_received_total[1m]))
   ```

2. **Transaction Extraction Rate**
   ```promql
   sum(rate(sunrush_tx_extracted_total[1m]))
   ```

3. **Active Slots**
   ```promql
   sunrush_active_slots
   ```

4. **Pipeline Latency (P50/P95/P99)**
   ```promql
   histogram_quantile(0.50, rate(sunrush_pipeline_latency_ms_bucket[5m]))
   histogram_quantile(0.95, rate(sunrush_pipeline_latency_ms_bucket[5m]))
   histogram_quantile(0.99, rate(sunrush_pipeline_latency_ms_bucket[5m]))
   ```

5. **Error Rate**
   ```promql
   sum(rate(sunrush_tx_failures_total[1m]))
   ```

6. **Memory Usage**
   ```promql
   sum(sunrush_memory_bytes) by (component)
   ```

#### Dashboard 2: Performance Deep Dive

**Panels:**

1. **Ingestion Latency Heatmap**
   ```promql
   sum(rate(sunrush_ingest_latency_ms_bucket[1m])) by (le)
   ```
   Type: Heatmap

2. **Assembly Latency Distribution**
   ```promql
   histogram_quantile(0.95, rate(sunrush_assemble_entry_latency_ms_bucket[5m]))
   ```

3. **Transaction Size Distribution**
   ```promql
   histogram_quantile(0.95, rate(sunrush_tx_size_bytes_bucket[5m]))
   ```

4. **Queue Depth Over Time**
   ```promql
   sunrush_queue_depth
   ```

5. **Backpressure Events**
   ```promql
   increase(sunrush_bus_backpressure_total[5m])
   ```

#### Dashboard 3: Business Metrics

**Panels:**

1. **Top Programs by Volume**
   ```promql
   topk(10, sum(rate(sunrush_tx_by_program_total[5m])) by (program))
   ```
   Type: Bar chart

2. **Transactions per Slot**
   ```promql
   rate(sunrush_tx_extracted_total[1m]) / rate(sunrush_blocks_assembled_total[1m])
   ```

3. **Strategy Matches**
   ```promql
   sum(rate(sunrush_strategy_matches_total[5m])) by (strategy)
   ```

### Import Dashboards

Pre-built dashboard JSON files:

```bash
# Download
wget https://raw.githubusercontent.com/ironfang/sunrush/main/dashboards/overview.json
wget https://raw.githubusercontent.com/ironfang/sunrush/main/dashboards/performance.json
wget https://raw.githubusercontent.com/ironfang/sunrush/main/dashboards/business.json

# Import in Grafana UI
# Dashboards → Import → Upload JSON file
```

---

## Alerting

### Prometheus AlertManager Setup

```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'sunrush'
    static_configs:
      - targets: ['localhost:9090']

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['localhost:9093']

rule_files:
  - "sunrush_alerts.yml"
```

### Alert Rules

```yaml
# sunrush_alerts.yml
groups:
  - name: sunrush_critical
    interval: 30s
    rules:
      - alert: SunRushDown
        expr: up{job="sunrush"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "SunRush instance {{ $labels.instance }} is down"
          description: "SunRush has been down for more than 1 minute"

      - alert: HighErrorRate
        expr: |
          rate(sunrush_tx_failures_total[5m]) /
          rate(sunrush_tx_extracted_total[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High transaction error rate"
          description: "Error rate is {{ $value | humanizePercentage }}"

      - alert: JitoDisconnected
        expr: sunrush_jito_connection_status == 0
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "Jito ShredStream disconnected"

  - name: sunrush_warnings
    interval: 1m
    rules:
      - alert: HighLatency
        expr: |
          histogram_quantile(0.95,
            rate(sunrush_pipeline_latency_ms_bucket[5m])
          ) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High pipeline latency"
          description: "P95 latency is {{ $value }}ms"

      - alert: BusBackpressure
        expr: rate(sunrush_bus_backpressure_total[5m]) > 10
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Message bus experiencing backpressure"

      - alert: HighMemoryUsage
        expr: |
          sum(sunrush_memory_bytes) / (32 * 1024 * 1024 * 1024) > 0.8
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage"
          description: "Using {{ $value | humanizePercentage }} of available memory"

      - alert: PluginCrash
        expr: increase(sunrush_plugin_panics_total[5m]) > 0
        labels:
          severity: warning
        annotations:
          summary: "Plugin crashed"
          description: "{{ $labels.plugin }} crashed {{ $value }} times"

      - alert: ManyMissingShreds
        expr: rate(sunrush_missing_shreds_total[5m]) > 100
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High rate of missing shreds"
```

### AlertManager Configuration

```yaml
# alertmanager.yml
global:
  resolve_timeout: 5m

route:
  receiver: 'default'
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
      continue: true
    
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'default'
    webhook_configs:
      - url: 'http://localhost:5001/webhook'

  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
        channel: '#sunrush-alerts'
        title: 'SunRush Alert'
        text: '{{ range .Alerts }}{{ .Annotations.summary }}\n{{ end }}'

  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_KEY'
```

---

## Tracing

### OpenTelemetry Integration (Optional)

Enable distributed tracing for detailed request flow analysis.

#### Setup

```toml
# config.toml
[tracing]
enabled = true
endpoint = "http://localhost:4317"
sample_rate = 0.1  # Sample 10% of requests
```

#### Jaeger Backend

```bash
# Run Jaeger all-in-one
docker run -d \
  --name jaeger \
  -p 16686:16686 \
  -p 4317:4317 \
  jaegertracing/all-in-one:latest

# Access UI
open http://localhost:16686
```

#### Trace Example

A complete trace shows:

```
Shred Received (1.2ms)
  ├─ Validate Shred (0.3ms)
  ├─ Publish to Bus (0.1ms)
  └─ Assembly Started (2.1ms)
      ├─ Insert Shred (0.5ms)
      ├─ Check Completeness (0.8ms)
      └─ Emit Entry (0.8ms)
          └─ Extract Transaction (1.1ms)
              ├─ Decode Entry (0.6ms)
              ├─ Parse Transaction (0.3ms)
              └─ Publish Transaction (0.2ms)
```

---

## Performance Analysis

### Latency Analysis

#### Identify Bottlenecks

```promql
# Component latency breakdown
sum(rate(sunrush_ingest_latency_ms_sum[5m])) by (component)
sum(rate(sunrush_assemble_entry_latency_ms_sum[5m])) by (component)
sum(rate(sunrush_tx_extract_latency_ms_sum[5m])) by (component)
```

#### P99 Latency by Component

```promql
histogram_quantile(0.99, rate(sunrush_ingest_latency_ms_bucket[5m]))
histogram_quantile(0.99, rate(sunrush_assemble_entry_latency_ms_bucket[5m]))
histogram_quantile(0.99, rate(sunrush_tx_extract_latency_ms_bucket[5m]))
```

### Throughput Analysis

```promql
# Messages per second by type
sum(rate(sunrush_bus_messages_sent_total[1m])) by (type)

# Saturation
sunrush_queue_depth / on() group_left() scalar(sunrush_bus_capacity)
```

### Resource Utilization

```bash
# CPU per plugin
top -H -p $(pgrep sunrush-host)

# Memory breakdown
cat /proc/$(pgrep sunrush-host)/smaps | grep -A 1 "\.so"

# Network usage
nethogs -p sunrush-host
```

---

## Monitoring Best Practices

1. **Set Baselines**: Establish normal performance metrics
2. **Alert on Anomalies**: Use rate-of-change alerts
3. **Correlate Metrics**: Cross-reference latency with throughput
4. **Regular Reviews**: Weekly performance review meetings
5. **Capacity Planning**: Monitor trends for scaling decisions
6. **Test Alerts**: Regularly test alert routing and escalation

---

## Monitoring Checklist

- [ ] Prometheus scraping SunRush metrics
- [ ] Grafana dashboards imported
- [ ] AlertManager configured
- [ ] Critical alerts routed to PagerDuty/on-call
- [ ] Log aggregation set up (Loki/ELK)
- [ ] Weekly performance review scheduled
- [ ] Runbooks created for common alerts
- [ ] Baseline metrics documented

---

**Comprehensive monitoring ensures SunRush runs smoothly and issues are caught early! 📊**
