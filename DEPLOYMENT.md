# Deployment Guide

This guide covers various deployment options for the Polygon POL Token Indexer, from development setups to production deployments.

## Quick Start Deployment

### Docker Compose (Recommended)

The fastest way to get started:

```bash
# Clone the repository
git clone <repository-url>
cd polygon-pol-indexer

# Copy and configure
cp config.example.toml config.toml
# Edit config.toml with your settings

# Start with Docker Compose
docker-compose up -d

# Check status
curl http://localhost:8080/status
```

## Deployment Options

### 1. Docker Deployment

#### Prerequisites

- Docker 20.10+
- Docker Compose 2.0+

#### Steps

1. **Build the image**

   ```bash
   docker build -t polygon-pol-indexer .
   ```

2. **Run with Docker Compose**

   ```bash
   docker-compose up -d
   ```

3. **Monitor logs**
   ```bash
   docker-compose logs -f indexer
   ```

#### Environment Variables

```bash
# .env file for docker-compose
POLYGON_RPC_URL=https://polygon-rpc.com/
DATABASE_PATH=/app/data/blockchain.db
API_PORT=8080
API_HOST=0.0.0.0
RUST_LOG=info
```

### 2. System Service Deployment (Linux)

#### Prerequisites

- Linux system with systemd
- Rust 1.75+
- SQLite 3.x

#### Steps

1. **Use the deployment script**

   ```bash
   sudo ./scripts/deploy.sh production
   ```

2. **Manual deployment**

   ```bash
   # Build the application
   cargo build --release

   # Create system user
   sudo useradd -r -s /bin/false -m -d /var/lib/polygon-pol-indexer indexer

   # Create directories
   sudo mkdir -p /opt/polygon-pol-indexer
   sudo mkdir -p /etc/polygon-pol-indexer
   sudo mkdir -p /var/log/polygon-pol-indexer
   sudo mkdir -p /var/lib/polygon-pol-indexer

   # Install binaries
   sudo cp target/release/{indexer,cli,server} /opt/polygon-pol-indexer/
   sudo chmod +x /opt/polygon-pol-indexer/*

   # Install configuration
   sudo cp config/production.toml /etc/polygon-pol-indexer/config.toml

   # Install systemd service
   sudo cp scripts/polygon-pol-indexer.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable polygon-pol-indexer
   sudo systemctl start polygon-pol-indexer
   ```

3. **Verify deployment**
   ```bash
   sudo systemctl status polygon-pol-indexer
   curl http://localhost:8080/status
   ```

### 3. User Installation

For single-user installations:

```bash
./scripts/install.sh
```

This installs binaries to `/usr/local/bin` and configuration to `~/.config/polygon-pol-indexer`.

## Environment-Specific Configurations

### Development Environment

```bash
# Use development configuration
cp config/development.toml config.toml

# Run locally
cargo run --bin indexer
```

### Staging Environment

```bash
# Deploy to staging
sudo ./scripts/deploy.sh staging

# Or use staging configuration
cp config/staging.toml config.toml
```

### Production Environment

```bash
# Deploy to production
sudo ./scripts/deploy.sh production

# Or use production configuration
cp config/production.toml config.toml
```

## Cloud Deployment

### AWS EC2

1. **Launch EC2 instance**

   - Ubuntu 22.04 LTS
   - t3.medium or larger
   - Security group allowing port 8080

2. **Install dependencies**

   ```bash
   sudo apt update
   sudo apt install -y build-essential pkg-config libssl-dev sqlite3
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

3. **Deploy application**

   ```bash
   git clone <repository-url>
   cd polygon-pol-indexer
   sudo ./scripts/deploy.sh production
   ```

4. **Configure security**

   ```bash
   # Configure firewall
   sudo ufw allow 22
   sudo ufw allow 8080
   sudo ufw enable

   # Set up SSL (optional)
   sudo apt install certbot nginx
   # Configure nginx reverse proxy
   ```

### Google Cloud Platform

1. **Create Compute Engine instance**

   ```bash
   gcloud compute instances create polygon-indexer \
     --image-family=ubuntu-2204-lts \
     --image-project=ubuntu-os-cloud \
     --machine-type=e2-medium \
     --tags=http-server
   ```

2. **SSH and deploy**
   ```bash
   gcloud compute ssh polygon-indexer
   # Follow standard deployment steps
   ```

### Azure VM

1. **Create VM**

   ```bash
   az vm create \
     --resource-group myResourceGroup \
     --name polygon-indexer \
     --image UbuntuLTS \
     --size Standard_B2s \
     --admin-username azureuser \
     --generate-ssh-keys
   ```

2. **Deploy application**
   ```bash
   ssh azureuser@<vm-ip>
   # Follow standard deployment steps
   ```

## Kubernetes Deployment

### Basic Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: polygon-pol-indexer
spec:
  replicas: 1
  selector:
    matchLabels:
      app: polygon-pol-indexer
  template:
    metadata:
      labels:
        app: polygon-pol-indexer
    spec:
      containers:
        - name: indexer
          image: polygon-pol-indexer:latest
          ports:
            - containerPort: 8080
          env:
            - name: POLYGON_RPC_URL
              value: "https://polygon-rpc.com/"
            - name: RUST_LOG
              value: "info"
          volumeMounts:
            - name: data
              mountPath: /app/data
            - name: config
              mountPath: /app/config.toml
              subPath: config.toml
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: indexer-data
        - name: config
          configMap:
            name: indexer-config
---
apiVersion: v1
kind: Service
metadata:
  name: polygon-pol-indexer-service
spec:
  selector:
    app: polygon-pol-indexer
  ports:
    - port: 8080
      targetPort: 8080
  type: LoadBalancer
```

### ConfigMap

```yaml
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: indexer-config
data:
  config.toml: |
    [rpc]
    endpoint = "https://polygon-rpc.com/"
    timeout_seconds = 30

    [database]
    path = "/app/data/blockchain.db"

    [api]
    enabled = true
    port = 8080
    host = "0.0.0.0"
```

### Persistent Volume

```yaml
# k8s/pvc.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: indexer-data
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 10Gi
```

## Monitoring and Observability

### Health Checks

The application provides health check endpoints:

```bash
# HTTP health check
curl http://localhost:8080/status

# Response format
{
  "status": "healthy",
  "last_processed_block": 12345678,
  "uptime_seconds": 3600,
  "database_status": "connected",
  "rpc_status": "connected"
}
```

### Logging

#### Systemd Logs

```bash
# View logs
sudo journalctl -u polygon-pol-indexer -f

# View logs with timestamp
sudo journalctl -u polygon-pol-indexer -f --since "1 hour ago"
```

#### Docker Logs

```bash
# View container logs
docker-compose logs -f indexer

# View logs with timestamp
docker-compose logs -f --timestamps indexer
```

#### File Logs

Configure file logging in `config.toml`:

```toml
[logging]
file_enabled = true
file_path = "/var/log/polygon-pol-indexer/indexer.log"
max_file_size_mb = 100
max_files = 5
```

### Metrics and Monitoring

#### Prometheus Integration (Optional)

Add to `docker-compose.yml`:

```yaml
services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
```

#### Custom Metrics

The application can be extended to export custom metrics:

- Blocks processed per minute
- RPC request latency
- Database operation duration
- Transfer detection rate

## Backup and Recovery

### Database Backup

```bash
# Create backup
sqlite3 /var/lib/polygon-pol-indexer/blockchain.db ".backup backup.db"

# Automated backup script
#!/bin/bash
BACKUP_DIR="/var/backups/polygon-pol-indexer"
DATE=$(date +%Y%m%d_%H%M%S)
mkdir -p "$BACKUP_DIR"
sqlite3 /var/lib/polygon-pol-indexer/blockchain.db ".backup $BACKUP_DIR/blockchain_$DATE.db"
find "$BACKUP_DIR" -name "blockchain_*.db" -mtime +7 -delete
```

### Configuration Backup

```bash
# Backup configuration
cp /etc/polygon-pol-indexer/config.toml /var/backups/polygon-pol-indexer/config_$(date +%Y%m%d).toml
```

### Recovery Procedure

1. **Stop the service**

   ```bash
   sudo systemctl stop polygon-pol-indexer
   ```

2. **Restore database**

   ```bash
   cp /var/backups/polygon-pol-indexer/blockchain_YYYYMMDD_HHMMSS.db /var/lib/polygon-pol-indexer/blockchain.db
   chown indexer:indexer /var/lib/polygon-pol-indexer/blockchain.db
   ```

3. **Start the service**
   ```bash
   sudo systemctl start polygon-pol-indexer
   ```

## Security Considerations

### Network Security

1. **Firewall Configuration**

   ```bash
   # Allow only necessary ports
   sudo ufw allow 22    # SSH
   sudo ufw allow 8080  # API (if public)
   sudo ufw enable
   ```

2. **Reverse Proxy with SSL**
   ```nginx
   # /etc/nginx/sites-available/polygon-indexer
   server {
       listen 443 ssl;
       server_name your-domain.com;

       ssl_certificate /path/to/cert.pem;
       ssl_certificate_key /path/to/key.pem;

       location / {
           proxy_pass http://localhost:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
       }
   }
   ```

### Application Security

1. **Run with minimal privileges**

   - Use dedicated system user
   - Restrict file permissions
   - Use systemd security features

2. **API Security**
   - Rate limiting
   - Authentication (if needed)
   - Input validation

### Data Security

1. **Database Security**

   - Regular backups
   - File permissions
   - Encryption at rest (if required)

2. **Configuration Security**
   - Secure RPC API keys
   - Restrict configuration file access
   - Use environment variables for secrets

## Troubleshooting

### Common Issues

1. **Service won't start**

   ```bash
   # Check service status
   sudo systemctl status polygon-pol-indexer

   # Check logs
   sudo journalctl -u polygon-pol-indexer -n 50

   # Check configuration
   /opt/polygon-pol-indexer/indexer --config /etc/polygon-pol-indexer/config.toml --check-config
   ```

2. **Database errors**

   ```bash
   # Check database integrity
   sqlite3 /var/lib/polygon-pol-indexer/blockchain.db "PRAGMA integrity_check;"

   # Check permissions
   ls -la /var/lib/polygon-pol-indexer/
   ```

3. **RPC connection issues**
   ```bash
   # Test RPC endpoint
   curl -X POST -H "Content-Type: application/json" \
     --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
     https://polygon-rpc.com/
   ```

### Performance Issues

1. **High CPU usage**

   - Check block processing rate
   - Adjust polling interval
   - Monitor RPC response times

2. **High memory usage**

   - Check database size
   - Monitor connection pool usage
   - Review batch sizes

3. **Slow queries**
   - Check database indexes
   - Monitor query performance
   - Consider database optimization

## Maintenance

### Regular Maintenance Tasks

1. **Log rotation**

   ```bash
   # Configure logrotate
   sudo tee /etc/logrotate.d/polygon-pol-indexer << EOF
   /var/log/polygon-pol-indexer/*.log {
       daily
       rotate 7
       compress
       delaycompress
       missingok
       notifempty
       postrotate
           systemctl reload polygon-pol-indexer
       endscript
   }
   EOF
   ```

2. **Database maintenance**

   ```bash
   # Vacuum database monthly
   sqlite3 /var/lib/polygon-pol-indexer/blockchain.db "VACUUM;"

   # Analyze database for query optimization
   sqlite3 /var/lib/polygon-pol-indexer/blockchain.db "ANALYZE;"
   ```

3. **System updates**

   ```bash
   # Update system packages
   sudo apt update && sudo apt upgrade

   # Update Rust toolchain
   rustup update

   # Rebuild application if needed
   cargo build --release
   ```

### Monitoring Checklist

- [ ] Service is running and healthy
- [ ] API endpoints are responding
- [ ] Database is accessible and not corrupted
- [ ] Logs are being written and rotated
- [ ] Disk space is sufficient
- [ ] Memory usage is within limits
- [ ] Network connectivity to RPC endpoint
- [ ] Backup system is working

This deployment guide provides comprehensive coverage of various deployment scenarios and operational considerations for the Polygon POL Token Indexer.
