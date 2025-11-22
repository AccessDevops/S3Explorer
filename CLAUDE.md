# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Desktop application for managing S3-compatible object storage (AWS S3, GCP S3-compatible, MinIO, Backblaze, etc.) built with:
- **Backend**: Rust + Tauri (IPC layer for secure S3 operations)
- **Frontend**: Vue 3 + TypeScript + Vite + Pinia
- **Target Platforms**: Windows, macOS, Linux

## Core Architecture

The cargo in not in the path but there $HOME/.cargo/bin

### Frontend (Vue 3 + TypeScript)
- **State Management**: Pinia for global state (profiles, buckets, current navigation)
- **Router**: Vue Router for navigation between views
- **Key Components**:
  - ConnectionManager: Add/edit/test S3 connection profiles
  - BucketList: Display available buckets for active profile
  - ObjectList: Browse objects/folders with pagination
  - FilePreview/Editor: View images, edit text files, hex view for binary
  - UploadDialog: Drag-drop and file selection with progress tracking
  - Settings: Application preferences

### Backend (Rust + Tauri)
- **IPC Commands**: Tauri commands expose typed JSON RPC to frontend
- **S3 Adapter**: Abstraction layer supporting custom endpoints, path-style vs virtual-host, session tokens
- **Profile Management**: Secure local storage of connection credentials (encrypted or OS keyring)
- **Async Operations**: tokio runtime for non-blocking S3 operations

### IPC Contract
All Tauri commands use strongly-typed serde JSON payloads:
- `list_profiles()`, `save_profile(profile)`, `delete_profile(id)`, `test_connection(profile)`
- `list_buckets(profile)`, `list_objects(profile, bucket, prefix, continuation, max_keys)`
- `get_object(profile, bucket, key)`, `put_object(profile, bucket, key, bytes, content_type)`
- `delete_object(profile, bucket, key)`, `copy_object(profile, src_bucket, src_key, dest_bucket, dest_key)`
- `generate_presigned_url(profile, bucket, key, method, expires)`
- `multipart_upload_start/part/complete` for large files (>50MB)

## Development Commands

Once scaffolded, the project will use:

**Development**:
```bash
# Frontend dev server
npm run dev

# Tauri dev (hot reload)
cargo tauri dev
```

**Build**:
```bash
# Production build
npm run build
cargo tauri build
```

**Testing**:
```bash
# Rust tests (unit + integration)
cargo test

# Frontend tests
npm run test

# Start local MinIO for testing
docker-compose up -d
```

**Linting**:
```bash
# Rust
cargo fmt
cargo clippy

# Frontend
npm run lint
```

## S3 Compatibility Requirements

The S3 adapter MUST support:
- **Custom endpoints**: Allow full endpoint URL override (not just AWS regions)
- **Path-style vs virtual-host**: Toggle via boolean flag in profile
- **Session tokens**: Support temporary credentials
- **TLS toggle**: Allow HTTP for local development (MinIO)
- **Region configuration**: Even for non-AWS providers
- **Retry logic**: Exponential backoff for transient failures

Profile structure:
```json
{
  "id": "unique-id",
  "name": "Display Name",
  "endpoint": "http://127.0.0.1:9000",
  "region": "us-east-1",
  "access_key": "...",
  "secret_key": "...",
  "session_token": null,
  "path_style": true,
  "use_tls": false
}
```

## Security Constraints

- **Never store secrets in plaintext**: Use OS keyring or encrypted local file
- **No secrets in debug logs**: Redact credentials from logging output
- **TLS by default**: Only allow HTTP for explicitly trusted local endpoints
- **Endpoint validation**: Warn when connecting to untrusted/custom endpoints
- **No proprietary SDK calls**: Use generic S3 API to maintain cross-provider compatibility

## Feature Milestones

The project is structured around progressive milestones:

1. **M1 - Scaffold**: Tauri + Vue skeleton with IPC wired
2. **M2 - Connectivity**: Profile manager + list buckets
3. **M3 - Browsing**: List objects, pagination, folder navigation, download
4. **M4 - CRUD**: Upload, delete, rename (copy+delete), create folders, presigned URLs
5. **M5 - Editor**: Text file editing, image preview, binary detection
6. **M6 - Advanced**: Multipart uploads, versioning, SSE support, role assumption
7. **M7 - Polish**: Settings, tests, CI, packaging, documentation

## Testing Strategy

- **Unit tests**: Mock S3 adapter operations
- **Integration tests**: Run against local MinIO instance (docker-compose provided)
- **CI**: GitHub Actions to run tests + build cross-platform artifacts
- **Test coverage**: Focus on S3 adapter, IPC commands, critical UI flows

## Key Design Decisions

- **Folder semantics**: Treat S3 prefixes ending with `/` as folders
- **Multipart threshold**: Use multipart uploads for files >50MB
- **Pagination**: Use continuation tokens for object listing
- **Move operation**: Implemented as copy + delete (S3 has no native move)
- **Error handling**: Return structured errors from Rust; display user-friendly messages in UI
- **Text detection**: UTF-8 validation to determine if file is editable vs binary

## Development Workflow

When implementing features:
1. Define Rust Tauri command with proper types
2. Add error handling with structured error types
3. Create/update Vue service wrapper for IPC calls
4. Implement UI component with loading/error states
5. Add unit tests for Rust logic
6. Test against local MinIO instance
7. Update integration tests if needed
