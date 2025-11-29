# S3 Browser

A cross-platform desktop application for managing S3-compatible object storage services including AWS S3, MinIO, Backblaze B2, Google Cloud Storage (S3-compatible), and other S3-compatible providers.

Built with Rust (Tauri) and Vue 3 for a native, performant desktop experience.

```bash
docker stop minio || true; docker rm minio
docker run -d --name minio -p 9000:9000 -p 9001:9001 \
-e MINIO_ROOT_USER=minioadmin \
-e MINIO_ROOT_PASSWORD=minioadmin \
minio/minio server /data --console-address ":9001"
```


```bash
# Create a self-signed certificate
openssl req -x509 -newkey rsa:2048 -keyout key.pem -out cert.pem -days 365 -nodes

# 2. Convert Certificate to Windows Format (if needed)
# If you only have .pem files, convert them to .pfx:
openssl pkcs12 -export -out certificate.pfx -inkey key.pem -in cert.pem -passout pass:your_password


# 3. Install SignTool on macOS
# Since you're on macOS, you need to use a tool that can sign Windows executables. You have two options:
# Option A: Use osslsigncode (easier)
brew install osslsigncode
# Option B: Use Windows SDK tools via Docker
# This is more complex but more official. I'd recommend Option A.

# 4. Sign Your Portable .exe
# Using osslsigncode:
osslsigncode sign -pkcs12 certificate.pfx -pass your_password \
  -n "Your App Name" \
  -i "https://yourwebsite.com" \
  -t http://timestamp.sectigo.com \
  -in your-app.exe \
  -out your-app-signed.exe

# Important flags explained:
# -pkcs12: Path to your .pfx certificate
# -pass: Your certificate password
# -n: App name (shows in Windows properties)
# -i: URL (optional, shows in Windows properties)
# -t: Timestamp server (crucial—prevents expiration issues)
# -in: Your original executable
# -out: Output signed executable

# 5. Verify the Signature
bashosslsigncode verify -in your-app-signed.exe

```

## Features

### Connection Management
- ✅ Multiple connection profiles
- ✅ Support for custom S3-compatible endpoints
- ✅ AWS S3, MinIO, Backblaze, GCP support
- ✅ Session token and temporary credentials
- ✅ Path-style and virtual-host addressing
- ✅ Connection testing before saving

### Storage Operations
- ✅ List and browse buckets
- ✅ Navigate folder hierarchies (prefix-based)
- ✅ Upload files with progress tracking
- ✅ Download objects to local filesystem
- ✅ Delete objects and folders
- ✅ Copy/move objects (copy + delete)
- ✅ Create folders (zero-byte objects with trailing slash)
- ✅ View text files and images inline
- ✅ Generate presigned URLs (GET/PUT)

### UI Features
- ✅ Sidebar navigation with profiles and buckets
- ✅ Breadcrumb navigation for folder paths
- ✅ File preview for text and images
- ✅ Contextual actions (download, view, delete)
- ✅ Responsive desktop layout
- ✅ Error handling and user feedback

## Quick Start

### Prerequisites
- **Rust** (latest stable) - [Install Rust](https://rustup.rs/)
- **Node.js** (v18+) - [Install Node.js](https://nodejs.org/)
- **Docker** (optional, for testing with MinIO)

See [SETUP.md](SETUP.md) for detailed installation instructions.

### Installation

1. Install dependencies:
   ```bash
   npm install
   ```

2. Start development server:
   ```bash
   npm run tauri:dev
   ```

3. (Optional) Start MinIO for local testing:
   ```bash
   docker-compose up -d
   ```

## Development

### Project Structure

```
s3explorer/
├── src/                    # Vue 3 frontend (TypeScript)
│   ├── components/         # Reusable UI components
│   ├── views/              # Page views
│   ├── stores/             # Pinia state management
│   ├── services/           # Tauri IPC service wrappers
│   ├── types/              # TypeScript type definitions
│   └── assets/             # Static assets and CSS
│
├── src-tauri/              # Rust backend (Tauri)
│   ├── src/
│   │   ├── main.rs         # Application entry point
│   │   ├── commands.rs     # Tauri command handlers (IPC)
│   │   ├── s3_adapter.rs   # S3 client abstraction layer
│   │   ├── profiles.rs     # Profile storage management
│   │   ├── models.rs       # Data structures and types
│   │   └── errors.rs       # Error handling
│   ├── Cargo.toml          # Rust dependencies
│   └── tauri.conf.json     # Tauri app configuration
│
├── docker-compose.yml      # MinIO local testing setup
├── CLAUDE.md               # Architecture guide for AI assistants
├── SETUP.md                # Detailed setup instructions
└── package.json            # Node.js dependencies and scripts
```

### Available Commands

**Development:**
```bash
npm run tauri:dev       # Start Tauri dev server with hot reload
npm run dev             # Start frontend dev server only
```

**Building:**
```bash
npm run tauri:build     # Build production binaries
npm run build           # Build frontend only
```

**Linting & Testing:**
```bash
npm run lint            # Lint Vue/TypeScript code
npm run format          # Format code with Prettier
cd src-tauri && cargo fmt      # Format Rust code
cd src-tauri && cargo clippy   # Lint Rust code
cd src-tauri && cargo test     # Run Rust tests
```

## Testing with MinIO

MinIO provides a lightweight, S3-compatible storage server perfect for local development and testing.

### Start MinIO
```bash
docker-compose up -d
```

- **API Endpoint:** http://localhost:9000
- **Web Console:** http://localhost:9001
- **Credentials:** minioadmin / minioadmin

### Create a Test Profile

In the S3 Browser app:
1. Click "+ Add" to create a new profile
2. Fill in the following:
   - **Profile Name:** Local MinIO
   - **Endpoint URL:** http://localhost:9000
   - **Region:** us-east-1
   - **Access Key:** minioadmin
   - **Secret Key:** minioadmin
   - **Force path-style:** ✓ Checked
   - **Use TLS:** ☐ Unchecked
3. Click "Test Connection" to verify
4. Click "Save"

## Architecture

### Backend (Rust + Tauri)

The Rust backend provides:
- **S3 Adapter**: Abstraction layer over AWS SDK for S3 supporting custom endpoints
- **Profile Management**: Encrypted local storage of connection profiles
- **IPC Commands**: Type-safe JSON RPC interface to the frontend
- **Error Handling**: Structured error types with user-friendly messages

Key dependencies:
- `tauri` - Desktop application framework
- `aws-sdk-s3` - Official AWS S3 SDK
- `tokio` - Async runtime
- `serde` - Serialization/deserialization

### Frontend (Vue 3 + TypeScript)

The Vue frontend provides:
- **Pinia Store**: Centralized state management for profiles, buckets, objects
- **Component Architecture**: Modular, reusable UI components
- **Tauri Services**: Type-safe wrappers around Tauri IPC commands
- **Responsive UI**: Desktop-optimized layout and interactions

Key dependencies:
- `vue` - Reactive UI framework
- `pinia` - State management
- `@tauri-apps/api` - Tauri frontend API
- `typescript` - Type safety

### IPC Communication

The frontend and backend communicate via Tauri's command system:

```typescript
// Frontend (TypeScript)
import { invoke } from '@tauri-apps/api/tauri'
const buckets = await invoke('list_buckets', { profileId })
```

```rust
// Backend (Rust)
#[tauri::command]
pub async fn list_buckets(profile_id: String) -> Result<Vec<Bucket>, String> {
    // Implementation
}
```

## Security Considerations

- **Credentials Storage**: Profiles are stored locally in `~/.config/s3explorer/` (or OS equivalent)
- **No Telemetry**: This application does not send any data externally
- **Local Processing**: All S3 operations run locally on your machine
- **Plaintext Warning**: Currently credentials are stored in plaintext JSON. Consider OS keyring integration for production use.

## Building for Production

### All Platforms
```bash
npm run tauri:build
```

### Output Locations
- **macOS:** `src-tauri/target/release/bundle/macos/`
- **Windows:** `src-tauri/target/release/bundle/windows/`
- **Linux:** `src-tauri/target/release/bundle/linux/`

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please ensure:
- Code is formatted (`cargo fmt`, `npm run format`)
- Linting passes (`cargo clippy`, `npm run lint`)
- Tests pass (`cargo test`)

## License

[Add your license here]

## Acknowledgments

- Built with [Tauri](https://tauri.app/)
- Uses [AWS SDK for Rust](https://github.com/awslabs/aws-sdk-rust)
- UI built with [Vue 3](https://vuejs.org/)
- Icon assets (placeholder - add your own icons)

## Support

For issues, questions, or feature requests, please open an issue on GitHub.

---

**Note:** This application is in active development. Features and APIs may change.
