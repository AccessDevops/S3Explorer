# S3 Browser - Setup Guide

This guide will help you set up the development environment and build the S3 Browser application.

## Prerequisites

Before you begin, ensure you have the following installed:

### 1. Rust
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify installation:
```bash
rustc --version
cargo --version
```

### 2. Node.js (v18 or later)
Download from [nodejs.org](https://nodejs.org/) or use a version manager like nvm:
```bash
# Using nvm
nvm install 20
nvm use 20
```

Verify installation:
```bash
node --version
npm --version
```

### 3. Tauri Dependencies

**macOS:**
```bash
xcode-select --install
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.0-dev \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev
```

**Windows:**
- Install [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Install [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

### 4. Docker (for testing with MinIO)
Download from [docker.com](https://www.docker.com/get-started)

## Installation

1. **Clone the repository** (if applicable) or navigate to the project directory:
   ```bash
   cd s3browser
   ```

2. **Install Node.js dependencies:**
   ```bash
   npm install
   ```

3. **Verify Rust dependencies:**
   ```bash
   cd src-tauri
   cargo check
   cd ..
   ```

## Development

### Start Development Server

To run the application in development mode with hot reload:

```bash
npm run tauri:dev
```

This will:
- Start the Vite dev server (frontend)
- Launch the Tauri development window
- Enable hot reload for both frontend and backend changes

### Alternative: Run frontend and backend separately

**Terminal 1 - Frontend:**
```bash
npm run dev
```

**Terminal 2 - Backend:**
```bash
cd src-tauri
cargo tauri dev
```

## Testing with MinIO

MinIO provides a local S3-compatible storage service for testing.

### Start MinIO

```bash
docker-compose up -d
```

This starts MinIO on:
- **API:** http://localhost:9000
- **Console:** http://localhost:9001

### MinIO Credentials

- **Username:** `minioadmin`
- **Password:** `minioadmin`

### Create a Test Profile

In the S3 Browser app, create a new connection profile with:

- **Profile Name:** Local MinIO
- **Endpoint URL:** http://localhost:9000
- **Region:** us-east-1
- **Access Key:** minioadmin
- **Secret Key:** minioadmin
- **Force path-style:** ✓ (checked)
- **Use TLS:** ☐ (unchecked)

Click "Test Connection" to verify connectivity.

### Stop MinIO

```bash
docker-compose down
```

To remove data:
```bash
docker-compose down -v
```

## Building for Production

### Development Build

```bash
npm run tauri:build
```

This creates platform-specific installers in `src-tauri/target/release/bundle/`.

### Platform-Specific Outputs

- **macOS:** `.app` and `.dmg` in `bundle/macos/`
- **Windows:** `.exe` and `.msi` in `bundle/windows/`
- **Linux:** `.deb` and `.AppImage` in `bundle/linux/`

## Linting and Formatting

### Frontend

```bash
# Lint
npm run lint

# Format
npm run format
```

### Backend (Rust)

```bash
cd src-tauri

# Format
cargo fmt

# Lint
cargo clippy
```

## Testing

### Rust Tests

```bash
cd src-tauri
cargo test
```

### Frontend Tests

```bash
npm run test
```

## Project Structure

```
s3browser/
├── src/                    # Vue 3 frontend
│   ├── components/         # UI components
│   ├── views/              # Page views
│   ├── stores/             # Pinia stores
│   ├── services/           # Tauri IPC services
│   ├── types/              # TypeScript types
│   └── assets/             # Static assets
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── commands.rs     # Tauri IPC commands
│   │   ├── s3_adapter.rs   # S3 client adapter
│   │   ├── profiles.rs     # Profile storage
│   │   ├── models.rs       # Data models
│   │   └── errors.rs       # Error types
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri configuration
├── docker-compose.yml      # MinIO for testing
├── package.json            # Node.js dependencies
└── vite.config.ts          # Vite configuration
```

## Common Issues

### 1. Rust compilation errors

**Solution:** Update Rust toolchain
```bash
rustup update
```

### 2. Node modules errors

**Solution:** Clear and reinstall
```bash
rm -rf node_modules package-lock.json
npm install
```

### 3. Tauri build fails on Linux

**Solution:** Install all required system dependencies (see Prerequisites section)

### 4. Can't connect to MinIO

**Solution:** Ensure Docker is running and MinIO container is up
```bash
docker ps
# Should show s3browser-minio container
```

### 5. Profile storage errors

**Solution:** Check permissions for config directory
- macOS/Linux: `~/.config/s3browser/`
- Windows: `%APPDATA%/s3browser/`

## Next Steps

1. Review [CLAUDE.md](CLAUDE.md) for architecture details
2. Check [README.md](README.md) for project overview
3. Start contributing!

## Support

For issues or questions, please check the GitHub issues or create a new one.
