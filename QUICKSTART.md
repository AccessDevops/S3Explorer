# Quick Start Guide

## ✅ Installation Complete!

Rust and all Node.js dependencies have been installed successfully.

## 🚀 Running the Application

### Option 1: Start Everything at Once (Recommended)

```bash
npm run tauri:dev
```

This command will:
- Build the frontend with Vite
- Start the Tauri development window
- Enable hot reload for both frontend and backend

### Option 2: Start Frontend and Backend Separately

**Terminal 1 - Frontend:**
```bash
npm run dev
```

**Terminal 2 - Backend:**
```bash
cd src-tauri
cargo tauri dev
```

## 🧪 Testing with MinIO

### 1. Start MinIO Server

```bash
docker-compose up -d
```

MinIO will be available at:
- **API**: http://localhost:9000
- **Web Console**: http://localhost:9001
- **Credentials**: minioadmin / minioadmin

### 2. Create Your First Connection

In the S3 Browser app:

1. Click **"+ Add"** button in the sidebar
2. Fill in the connection details:
   - **Profile Name**: `Local MinIO`
   - **Endpoint URL**: `http://localhost:9000`
   - **Region**: `us-east-1`
   - **Access Key**: `minioadmin`
   - **Secret Key**: `minioadmin`
   - **☑ Force path-style addressing**: Checked
   - **☐ Use TLS/HTTPS**: Unchecked
3. Click **"Test Connection"** to verify
4. Click **"Save"**

### 3. Create a Test Bucket

1. Open MinIO Console: http://localhost:9001
2. Login with `minioadmin` / `minioadmin`
3. Click "Buckets" → "Create Bucket"
4. Name it `test-bucket` and create
5. Refresh your S3 Browser app to see the new bucket

## 📁 Features You Can Try

Once connected:

- **Browse Buckets**: Click on any bucket to explore its contents
- **Upload Files**: Click "Upload" button and select files
- **Create Folders**: Click "New Folder" and enter a name
- **Download Files**: Click "Download" on any file
- **View Text Files**: Click "View" to preview text and image files
- **Delete Objects**: Click "Delete" to remove files or folders

## 🛠️ Development Commands

```bash
# Lint frontend code
npm run lint

# Format code
npm run format

# Check Rust code
cd src-tauri && cargo check

# Run Rust tests
cd src-tauri && cargo test

# Format Rust code
cd src-tauri && cargo fmt

# Lint Rust code
cd src-tauri && cargo clippy
```

## 📦 Building for Production

```bash
npm run tauri:build
```

Binaries will be in `src-tauri/target/release/bundle/`

## 🐛 Troubleshooting

### "Connection failed" error
- Make sure MinIO is running: `docker ps`
- Verify endpoint is `http://localhost:9000` (not https)
- Check "Force path-style addressing" is enabled

### App won't start
- Make sure all dependencies are installed: `npm install`
- Try cleaning and rebuilding:
  ```bash
  rm -rf node_modules package-lock.json
  npm install
  ```

### Rust compilation errors
- Update Rust: `rustup update`
- Clean build: `cd src-tauri && cargo clean && cargo check`

## 📚 Documentation

- [README.md](README.md) - Full project documentation
- [SETUP.md](SETUP.md) - Detailed setup instructions
- [CLAUDE.md](CLAUDE.md) - Architecture guide

## 🎉 You're All Set!

Your S3 Browser application is ready to use. Run `npm run tauri:dev` to get started!
