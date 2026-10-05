# Docker Setup Guide

This guide walks you through containerizing your entire multicam IoT app locally.

---

## What We've Changed

### 1. **Code Fixes** (no AWS needed for now)
- `commands.rs`: Capture script path now uses `CAPTURE_SCRIPT_PATH` env var (defaults to `./capture.py`)
- `images.rs`: S3 bucket defaults to `local-images` (configurable via `AWS_S3_BUCKET`)
- `frontend/config.js`: New file to make API endpoint configurable

### 2. **New Dockerfiles**
- `Dockerfile.camera`: MQTT publisher (4 instances in docker-compose)
- `Dockerfile.backend`: Axum web server + SQLite + WebSocket
- Both use multi-stage builds for smaller images

### 3. **docker-compose.yml**
- 1 backend service (port 8080)
- 4 camera services (cam01-cam04)
- Shared volumes: `/app/snapshots` (images), `/app/data` (SQLite)
- All on same network: `multicam-network`

---

## Prerequisites

```bash
# Docker & Docker Compose
docker --version      # Should be 20.10+
docker-compose --version  # Should be 1.29+
```

---

## Setup Steps

### Step 1: Copy Files to Your Project

Copy these new files to your **project root**:

```
your-project/
├── Dockerfile.backend
├── Dockerfile.camera
├── docker-compose.yml
├── .env.example
└── ... (existing files)
```

Replace your original hardcoded files:
- `src/backend/routes/commands.rs` ← use fixed version
- `src/backend/routes/images.rs` ← use fixed version
- Add `src/frontend/config.js` ← new file

**Update HTML files** (index.html, gallery.html, records.html):
- Add this at the top, before other scripts:
```html
<script src="config.js"></script>
```
- Replace hardcoded `'http://127.0.0.1:8080'` with `API` variable

Example in index.html:
```javascript
// OLD:
const API = 'http://127.0.0.1:8080';

// NEW (already in config.js):
// const API = API_CONFIG.getApiUrl();
// Now just use: fetch(`${API}/auth/login`, ...)
```

---

### Step 2: Create Local Directories

```bash
mkdir -p ./data
mkdir -p ./snapshots
```

---

### Step 3: Start Docker Compose

```bash
# Build all images (first time only, ~5-10 mins)
docker-compose build

# Start all services
docker-compose up

# Or in background:
docker-compose up -d
```

---

## Verification

### Check if services are running:

```bash
docker-compose ps
```

Expected output:
```
NAME                COMMAND             STATUS          PORTS
multicam-backend    "./backend"         Up (healthy)    0.0.0.0:8080->8080/tcp
multicam-cam01      "./camera"          Up              
multicam-cam02      "./camera"          Up              
multicam-cam03      "./camera"          Up              
multicam-cam04      "./camera"          Up              
```

### Test the API:

```bash
# Health check
curl http://localhost:8080/health

# Should return: OK
```

### Access the Frontend

Open browser:
```
http://localhost:8080/app/index.html
```

---

## Data Flow (In Docker)

```
┌─────────────┐
│  Browser    │ (port 8080)
└──────┬──────┘
       │
       ▼
┌──────────────────┐
│  Backend (Axum)  │◄─────── MQTT listener
│  - WebSocket /ws │         connects to
│  - REST APIs     │         test.mosquitto.org
│  - SQLite        │
└────────┬─────────┘
         │
         ├──►┌──────────┐
         │   │ cam01    │
         │   │(MQTT Pub)│ watches /app/snapshots
         │   └──────────┘
         │
         ├──►┌──────────┐
         │   │ cam02    │
         │   │(MQTT Pub)│ watches /app/snapshots
         │   └──────────┘
         │
         ├──►┌──────────┐
         │   │ cam03    │
         │   │(MQTT Pub)│ watches /app/snapshots
         │   └──────────┘
         │
         └──►┌──────────┐
             │ cam04    │
             │(MQTT Pub)│ watches /app/snapshots
             └──────────┘

Shared volumes:
- /app/snapshots ◄─── python3 capture.py writes here
- /app/data      ◄─── SQLite database
```

---

## Troubleshooting

### Containers keep restarting?

```bash
# Check logs
docker-compose logs backend
docker-compose logs cam01

# Common issues:
# 1. DATABASE_URL wrong → check docker-compose.yml
# 2. JWT_SECRET missing → set in docker-compose or .env
# 3. Cert files missing → OK for mock mode, ignored
```

### Backend can't find capture.py?

```bash
# Make sure capture.py is in project root
ls -la capture.py

# Check CAPTURE_SCRIPT_PATH in docker-compose.yml
# Should be: /app/capture.py
```

### Cameras can't connect to MQTT?

```bash
# test.mosquitto.org is public test broker
# Should work without certs in mock mode
docker-compose logs cam01 | grep -i mqtt
```

### WebSocket connection failing?

```bash
# Frontend tries to connect to /ws
# Make sure backend is running and healthy
curl http://localhost:8080/health

# Check browser console (DevTools) for errors
```

---

## Next Steps (After Verification)

1. ✅ **Test locally with docker-compose** ← You are here
2. **Kubernetes (minikube)** → We'll convert this to K8s manifests
3. **GitHub Actions** → Auto-build Docker images on git push
4. **AWS Deployment** → Push to ECR, deploy on EKS

---

## Useful Commands

```bash
# View all running containers
docker-compose ps

# View logs for one service
docker-compose logs backend -f

# View logs for all services
docker-compose logs -f

# Stop all services
docker-compose stop

# Stop and remove everything
docker-compose down

# Remove volumes too (careful!)
docker-compose down -v

# Rebuild specific service
docker-compose build backend

# Execute command in container
docker-compose exec backend sh

# Check specific service
docker-compose exec backend curl http://localhost:8080/health
```

---

## Environment Variables Reference

All services read from `docker-compose.yml`. No `.env` file needed for basic setup, but you can create one:

```bash
cp .env.example .env
# Edit .env with your values
docker-compose --env-file .env up
```

---

**Ready to start? Run:**

```bash
docker-compose up
```

Let me know if any service fails! 🚀
