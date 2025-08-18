# Production Deployment Guide

Comprehensive руководство по развертыванию BSL Gradual Type System в production окружениях.

## 🏗️ Production Architecture

### Recommended Setup
```
┌─────────────────────────────────────────────────┐
│                Load Balancer                    │
│            (nginx/traefik)                      │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│              Web Server Cluster                 │
│         (multiple bsl-web-server)               │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│               LSP Servers                       │
│        (per-developer instances)                │
└─────────────────┬───────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────┐
│            Shared Cache Storage                 │
│         (Redis/File System)                     │
└─────────────────────────────────────────────────┘
```

## 🚀 Production Deployment Options

### Option 1: Single Server Setup (Small Teams)

#### System Requirements
- **CPU**: 4+ cores (8+ recommended)
- **RAM**: 8GB+ (16GB+ recommended)
- **Storage**: 50GB+ SSD
- **OS**: Linux (Ubuntu 20.04+), Windows Server 2019+, macOS 11+

#### Deployment Steps
```bash
# 1. Server preparation
sudo apt-get update
sudo apt-get install build-essential cmake pkg-config libssl-dev

# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 3. Clone and build
git clone https://github.com/yourusername/bsl-gradual-types.git
cd bsl-gradual-types
cargo build --release

# 4. Create service user
sudo useradd -r -s /bin/false bsl-analyzer
sudo chown -R bsl-analyzer:bsl-analyzer /opt/bsl-gradual-types

# 5. Install binaries
sudo cp target/release/* /usr/local/bin/
sudo chmod +x /usr/local/bin/lsp-server
sudo chmod +x /usr/local/bin/bsl-web-server
```

#### Systemd Service Setup
```ini
# /etc/systemd/system/bsl-lsp.service
[Unit]
Description=BSL Gradual Type System LSP Server
After=network.target

[Service]
Type=simple
User=bsl-analyzer
Group=bsl-analyzer
ExecStart=/usr/local/bin/lsp-server
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=BSL_CACHE_DIR=/var/cache/bsl-analyzer

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/bsl-web.service
[Unit]
Description=BSL Gradual Type System Web Server
After=network.target

[Service]
Type=simple
User=bsl-analyzer
Group=bsl-analyzer
ExecStart=/usr/local/bin/bsl-web-server --port 8080
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

```bash
# Активация services
sudo systemctl daemon-reload
sudo systemctl enable bsl-lsp bsl-web
sudo systemctl start bsl-lsp bsl-web

# Проверка статуса
sudo systemctl status bsl-lsp
sudo systemctl status bsl-web
```

### Option 2: Docker Deployment (Recommended)

#### Dockerfile
```dockerfile
# Build stage
FROM rust:1.70-alpine as builder

RUN apk add --no-cache musl-dev cmake make gcc g++

WORKDIR /app
COPY . .

RUN cargo build --release

# Runtime stage
FROM alpine:latest

RUN apk add --no-cache ca-certificates

RUN addgroup -g 1001 bsl && \
    adduser -D -s /bin/sh -u 1001 -G bsl bsl

WORKDIR /app

# Copy binaries
COPY --from=builder /app/target/release/lsp-server ./
COPY --from=builder /app/target/release/bsl-web-server ./
COPY --from=builder /app/target/release/bsl-profiler ./
COPY --from=builder /app/target/release/type-check ./

# Create cache directory
RUN mkdir -p /app/cache && chown bsl:bsl /app/cache

USER bsl

# Default to web server
EXPOSE 8080
CMD ["./bsl-web-server", "--port", "8080"]
```

#### Docker Compose Setup
```yaml
# docker-compose.yml
version: '3.8'

services:
  bsl-web:
    build: .
    ports:
      - "8080:8080"
    volumes:
      - ./cache:/app/cache
      - ./projects:/app/projects:ro
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    
  bsl-lsp:
    build: .
    command: ["./lsp-server"]
    ports:
      - "3000:3000"
    volumes:
      - ./cache:/app/cache
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    
  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - bsl-web
    restart: unless-stopped
```

#### Deployment
```bash
# Build и start
docker-compose up -d

# Проверка логов
docker-compose logs -f bsl-web

# Scaling web servers
docker-compose up -d --scale bsl-web=3

# Health check
curl http://localhost:8080/api/stats
```

### Option 3: Kubernetes Deployment (Enterprise)

#### Kubernetes Manifests
```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: bsl-gradual-types
  labels:
    app: bsl-gradual-types
spec:
  replicas: 3
  selector:
    matchLabels:
      app: bsl-gradual-types
  template:
    metadata:
      labels:
        app: bsl-gradual-types
    spec:
      containers:
      - name: bsl-web
        image: bsl-gradual-types:1.0.0
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "info"
        - name: BSL_CACHE_DIR
          value: "/app/cache"
        volumeMounts:
        - name: cache-volume
          mountPath: /app/cache
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "512Mi" 
            cpu: "500m"
        livenessProbe:
          httpGet:
            path: /api/stats
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/stats
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: cache-volume
        persistentVolumeClaim:
          claimName: bsl-cache-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: bsl-gradual-types-service
spec:
  selector:
    app: bsl-gradual-types
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: LoadBalancer
```

```bash
# Deploy в Kubernetes
kubectl apply -f k8s/

# Проверка deployment
kubectl get pods -l app=bsl-gradual-types
kubectl logs -l app=bsl-gradual-types

# Port forward для тестирования
kubectl port-forward service/bsl-gradual-types-service 8080:80
```

## ⚙️ Production Configuration

### Environment Variables
```bash
# Core configuration
export RUST_LOG=info                    # Logging level
export BSL_CACHE_DIR=/var/cache/bsl     # Cache directory
export BSL_MAX_MEMORY_MB=1024           # Memory limit
export BSL_PARALLEL_THREADS=8           # Analysis threads

# LSP Server specific
export BSL_LSP_PORT=3000                # LSP port (if TCP mode)
export BSL_LSP_HOST=0.0.0.0             # LSP bind address

# Web Server specific  
export BSL_WEB_PORT=8080                # Web server port
export BSL_WEB_HOST=0.0.0.0             # Web server bind address
export BSL_WEB_STATIC_DIR=./web         # Static files directory
```

### Configuration Files
```toml
# /etc/bsl-analyzer/config.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[analysis]
enable_caching = true
cache_ttl_hours = 24
parallel_threads = 8
max_file_size_mb = 10

[performance]
enable_profiling = false
memory_limit_mb = 1024
gc_interval_minutes = 30

[logging]
level = "info"
file = "/var/log/bsl-analyzer/app.log"
rotation = "daily"
```

## 📊 Monitoring & Observability

### Health Checks
```bash
# Web server health
curl http://localhost:8080/api/stats

# Expected response:
{
  "total_functions": 89,
  "total_variables": 234,
  "memory_usage_mb": 15.7,
  "uptime_seconds": 3600
}

# LSP server health (если TCP mode)
telnet localhost 3000
```

### Metrics Collection
```bash
# Performance metrics endpoint
curl http://localhost:8080/api/metrics

# Expected metrics:
{
  "requests_per_second": 45.2,
  "average_response_time_ms": 12.5,
  "cache_hit_rate": 0.85,
  "memory_usage_mb": 256.7,
  "active_connections": 12
}
```

### Logging Setup
```bash
# Centralized logging с ELK Stack
# Logstash configuration для parsing BSL analyzer logs

# Или простое file logging
export RUST_LOG=info
export BSL_LOG_FILE=/var/log/bsl-analyzer/app.log

# Log rotation с logrotate
# /etc/logrotate.d/bsl-analyzer
/var/log/bsl-analyzer/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
}
```

## 🔒 Security Considerations

### Network Security
```bash
# Firewall rules
sudo ufw allow 8080/tcp                # Web server
sudo ufw allow 3000/tcp                # LSP server (если public)

# SSL/TLS с nginx reverse proxy
# /etc/nginx/sites-available/bsl-analyzer
server {
    listen 443 ssl http2;
    server_name bsl-analyzer.yourdomain.com;
    
    ssl_certificate /path/to/certificate.crt;
    ssl_certificate_key /path/to/private.key;
    
    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
    
    location /api/ {
        proxy_pass http://localhost:8080/api/;
        proxy_set_header Host $host;
    }
}
```

### Authentication (для enterprise)
```bash
# Basic Auth с nginx
# Создание password file
sudo htpasswd -c /etc/nginx/.htpasswd username

# nginx configuration
location /api/ {
    auth_basic "BSL Analyzer API";
    auth_basic_user_file /etc/nginx/.htpasswd;
    proxy_pass http://localhost:8080/api/;
}
```

### File Permissions
```bash
# Secure file permissions
sudo chmod 755 /usr/local/bin/lsp-server
sudo chmod 755 /usr/local/bin/bsl-web-server
sudo chmod 600 /etc/bsl-analyzer/config.toml
sudo chmod 700 /var/cache/bsl-analyzer
```

## 📈 Scaling Considerations

### Horizontal Scaling
```bash
# Multiple web server instances
for i in {1..3}; do
  ./target/release/bsl-web-server --port $((8080 + i)) &
done

# Load balancer configuration (nginx)
upstream bsl_backend {
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
    server 127.0.0.1:8083;
}
```

### Vertical Scaling
```bash
# Оптимизация для больших servers
export BSL_PARALLEL_THREADS=16         # Больше threads
export BSL_MAX_MEMORY_MB=4096          # Больше memory
export BSL_CACHE_SIZE_MB=1024          # Больше cache
```

### Database Integration (advanced)
```toml
# config.toml
[database]
url = "postgresql://user:password@localhost/bsl_analyzer"
pool_size = 10
cache_in_db = true
```

## 🔍 Performance Tuning

### System Optimization
```bash
# Linux kernel tuning
echo 'vm.swappiness=10' >> /etc/sysctl.conf
echo 'fs.file-max=65536' >> /etc/sysctl.conf

# Open files limit
echo 'bsl-analyzer soft nofile 65536' >> /etc/security/limits.conf
echo 'bsl-analyzer hard nofile 65536' >> /etc/security/limits.conf
```

### Application Tuning
```bash
# Memory optimization
export MALLOC_ARENA_MAX=2               # Reduce memory fragmentation
export RUST_MIN_STACK=8388608           # 8MB stack size

# Performance flags
export RUSTFLAGS="-C target-cpu=native -C lto=fat"

# Rebuild для production
cargo build --release
```

## 📊 Monitoring Dashboard

### Prometheus Integration
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'bsl-analyzer'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: /api/metrics
```

### Grafana Dashboard
```json
{
  "dashboard": {
    "title": "BSL Gradual Type System",
    "panels": [
      {
        "title": "Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "avg_response_time_ms",
            "legendFormat": "Average Response Time"
          }
        ]
      },
      {
        "title": "Cache Hit Rate", 
        "type": "singlestat",
        "targets": [
          {
            "expr": "cache_hit_rate * 100",
            "legendFormat": "Cache Hit Rate %"
          }
        ]
      }
    ]
  }
}
```

## 🚨 Disaster Recovery

### Backup Strategy
```bash
# Cache backup
tar -czf bsl-cache-backup-$(date +%Y%m%d).tar.gz /var/cache/bsl-analyzer/

# Configuration backup
cp -r /etc/bsl-analyzer/ /backup/config/

# Database backup (если используется)
pg_dump bsl_analyzer > bsl_analyzer_backup_$(date +%Y%m%d).sql
```

### Recovery Procedures
```bash
# Service recovery
sudo systemctl restart bsl-lsp bsl-web

# Cache recovery
tar -xzf bsl-cache-backup-latest.tar.gz -C /

# Complete recovery
sudo systemctl stop bsl-lsp bsl-web
# Restore files...
sudo systemctl start bsl-lsp bsl-web
```

## 🔧 Maintenance

### Regular Maintenance Tasks
```bash
# Weekly tasks
# 1. Cache cleanup
find /var/cache/bsl-analyzer -type f -mtime +7 -delete

# 2. Log rotation
logrotate /etc/logrotate.d/bsl-analyzer

# 3. Performance check
./target/release/bsl-profiler benchmark --iterations 5

# 4. Security updates
cargo audit
```

### Automated Maintenance Script
```bash
#!/bin/bash
# /usr/local/bin/bsl-maintenance.sh

echo "🔧 Starting BSL Analyzer maintenance..."

# Stop services
systemctl stop bsl-web bsl-lsp

# Clean old cache
find /var/cache/bsl-analyzer -name "*.cache" -mtime +7 -delete

# Clean logs
journalctl --vacuum-time=30d

# Update from git (если auto-update включен)
cd /opt/bsl-gradual-types
git pull origin master
cargo build --release

# Update binaries
cp target/release/lsp-server /usr/local/bin/
cp target/release/bsl-web-server /usr/local/bin/

# Restart services
systemctl start bsl-lsp bsl-web

# Health check
sleep 10
curl -f http://localhost:8080/api/stats || echo "❌ Health check failed"

echo "✅ Maintenance completed"
```

### Cron setup
```bash
# /etc/cron.d/bsl-analyzer
# Daily maintenance at 2 AM
0 2 * * * root /usr/local/bin/bsl-maintenance.sh

# Weekly full backup at Sunday 1 AM
0 1 * * 0 root /usr/local/bin/bsl-backup.sh
```

## 📋 Production Checklist

### Pre-deployment
- [ ] ✅ Все тесты проходят на target системе
- [ ] ✅ Performance benchmarks в пределах нормы
- [ ] ✅ Security scan завершен без critical issues
- [ ] ✅ Backup strategy настроена
- [ ] ✅ Monitoring и alerting настроены
- [ ] ✅ Load testing проведен
- [ ] ✅ Rollback plan подготовлен

### Post-deployment
- [ ] ✅ Services запущены и healthy
- [ ] ✅ Web interface доступен
- [ ] ✅ LSP server отвечает на requests
- [ ] ✅ Metrics собираются корректно
- [ ] ✅ Logs пишутся в правильные locations
- [ ] ✅ Cache работает эффективно
- [ ] ✅ Performance в ожидаемых пределах

### Ongoing Operations
- [ ] ✅ Daily health checks
- [ ] ✅ Weekly performance reviews
- [ ] ✅ Monthly security updates
- [ ] ✅ Quarterly capacity planning

## 🚨 Troubleshooting

### Common Issues

#### High Memory Usage
```bash
# Проверка memory usage
ps aux | grep bsl-
systemd-cgtop

# Решения:
# 1. Уменьшить cache size
export BSL_CACHE_SIZE_MB=512

# 2. Увеличить GC frequency
export BSL_GC_INTERVAL_MINUTES=15

# 3. Restart services periodically
systemctl restart bsl-web
```

#### Slow Response Times
```bash
# Диагностика performance
./target/release/bsl-profiler benchmark --iterations 10

# Проверка system resources
htop
iotop

# Решения:
# 1. Включить caching
# 2. Увеличить parallel threads
# 3. Upgrade hardware
```

#### LSP Connection Issues
```bash
# Проверка LSP server
systemctl status bsl-lsp
journalctl -u bsl-lsp -f

# Test connection
telnet localhost 3000

# Check process
ps aux | grep lsp-server
```

---

## 📞 Production Support

### Emergency Contacts
- 🆘 **Critical Issues**: Create [GitHub Issue](https://github.com/yourusername/bsl-gradual-types/issues) with "critical" label
- 📧 **Production Support**: bsl-production-support@example.com
- 💬 **Community**: [GitHub Discussions](https://github.com/yourusername/bsl-gradual-types/discussions)

### SLA Targets (Enterprise)
- **Uptime**: 99.9%
- **Response Time**: <100ms (95th percentile)
- **Error Rate**: <0.1%
- **Recovery Time**: <30 minutes

**🏆 BSL Gradual Type System готов для enterprise production deployment!**