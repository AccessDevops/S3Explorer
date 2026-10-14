<div align="center">

<img src="https://s3explorer.accessdevops.com/og-image.png" alt="S3 Explorer — the modern S3 browser and desktop client for S3-compatible storage" width="640">

# S3 Explorer

**The modern S3 browser and desktop client for S3-compatible storage.**

One application to browse, edit and navigate millions of objects with a smart local index — across all your buckets, on every S3-compatible provider.

[![Latest release](https://img.shields.io/github/v/release/AccessDevops/S3Explorer?label=latest&color=2563eb)](https://github.com/AccessDevops/S3Explorer/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/AccessDevops/S3Explorer/total?color=2563eb)](https://github.com/AccessDevops/S3Explorer/releases)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-2563eb)](#download)
[![Website](https://img.shields.io/badge/website-s3explorer.accessdevops.com-2563eb)](https://s3explorer.accessdevops.com/)

[**Download**](#download) · [**Supported providers**](#supported-providers) · [**Features**](#features) · [**Website**](https://s3explorer.accessdevops.com/)

</div>

---

## What it is

S3 Explorer is a desktop S3 browser for Windows, macOS and Linux. It connects to Amazon S3 and to any S3-compatible object storage — MinIO, Cloudflare R2, Backblaze B2, Wasabi, Ceph, DigitalOcean Spaces, OVHcloud, Dell ECS and twenty more — and gives you one interface for all of them.

It is built with Rust and Tauri, so it starts fast, uses little memory and ships as a native binary rather than a bundled browser.

**This repository hosts the released binaries and the updater manifest.** Issues and feature requests are welcome here.

## Download

Grab the latest build from [**Releases**](https://github.com/AccessDevops/S3Explorer/releases/latest), or from [s3explorer.accessdevops.com](https://s3explorer.accessdevops.com/) which detects your platform automatically.

| Platform | Architecture | File |
|---|---|---|
| **Windows** | x64 | `S3.Explorer_<version>_x64-setup.exe` · `..._x64_en-US.msi` |
| **Windows** | ARM64 | `S3.Explorer_<version>_arm64-setup.exe` · `..._arm64_en-US.msi` |
| **macOS** | Universal (Intel + Apple Silicon) | `S3.Explorer_<version>_universal.dmg` |
| **Linux** | x86_64 | `..._amd64.AppImage` · `..._amd64.deb` · `...x86_64.rpm` |
| **Linux** | ARM64 | `..._aarch64.AppImage` · `..._arm64.deb` · `...aarch64.rpm` |

Updates are delivered in-app and every updater artifact is signed with an Ed25519 key, verified before anything is installed.

## Features

**Connect to anything**
- 25 ready-made provider presets — endpoint format, region list and addressing mode filled in for you
- Custom endpoint for any other S3-compatible service
- Corporate proxy support, DNS-over-HTTPS fallback, platform TLS trust store
- Temporary credentials (STS / AssumeRole) where the provider supports them

**Browse at scale**
- Smart local index for instant navigation across millions of objects
- Search and filter without re-listing the bucket
- Compact view and dark mode

**Work with objects**
- Drag-and-drop upload and download, multipart and parallel transfers
- Built-in text and image editor
- Preview for images, video and audio
- Object versioning, tags and metadata management
- Presigned URL generator
- Bucket statistics and dashboard metrics

**Everywhere you work**
- Windows, macOS and Linux, on x64 and ARM64
- 10 interface languages
- Auto-update with signature verification

## Supported providers

Each guide below documents the endpoint format, regions, addressing mode, where the credentials come from, a filled-in example and the mistakes that usually cost an afternoon.

| | Provider | Guide |
|---|---|---|
| ☁️ | **Amazon S3** | [Connection guide](https://s3explorer.accessdevops.com/providers/aws/) |
| ☁️ | **Google Cloud Storage** | [Connection guide](https://s3explorer.accessdevops.com/providers/gcs/) |
| ⚡ | **Cloudflare R2** | [Connection guide](https://s3explorer.accessdevops.com/providers/cloudflare/) |
| ⚡ | **Backblaze B2** | [Connection guide](https://s3explorer.accessdevops.com/providers/backblaze/) |
| ⚡ | **Wasabi** | [Connection guide](https://s3explorer.accessdevops.com/providers/wasabi/) |
| ⚡ | **DigitalOcean Spaces** | [Connection guide](https://s3explorer.accessdevops.com/providers/digitalocean/) |
| ⚡ | **OVHcloud** | [Connection guide](https://s3explorer.accessdevops.com/providers/ovhcloud/) |
| 🏠 | **MinIO** | [Connection guide](https://s3explorer.accessdevops.com/providers/minio/) |
| 🏠 | **Ceph RGW** | [Connection guide](https://s3explorer.accessdevops.com/providers/ceph/) |
| 🏢 | **Dell EMC ECS** | [Connection guide](https://s3explorer.accessdevops.com/providers/dell-ecs/) |

**Also supported, with a ready-made preset:** Oracle Cloud, IBM Cloud, Alibaba OSS, Scaleway, Hetzner, Linode / Akamai, Vultr, Exoscale, Garage, SeaweedFS, Zenko CloudServer, NetApp StorageGRID, Cloudian HyperStore, Scality RING — and any other S3-compatible service through a custom endpoint.

See [all provider guides](https://s3explorer.accessdevops.com/providers/).

## Pricing

The app is **free and fully functional**, with up to 5 connection profiles and an occasional reminder popup.

A licence removes both — unlimited profiles, no popup — and supports development: **11,50 € once** or **1,99 € per month** with a 5-day free trial. Details on [the pricing page](https://s3explorer.accessdevops.com/buy/).

## Privacy

Credentials never leave your machine. They are stored in your operating system's keychain and are sent only to the provider endpoint you configured.

Anonymous usage telemetry is enabled by default and can be turned off at any time in **Settings → Privacy**. It carries no bucket names, no object keys, no credentials and no personal data.

## Links

- **Website** — [s3explorer.accessdevops.com](https://s3explorer.accessdevops.com/)
- **Release notes** — [all versions with full changelogs](https://s3explorer.accessdevops.com/news/)
- **Report an issue** — [open an issue](https://github.com/AccessDevops/S3Explorer/issues/new)

---

<div align="center">
<sub>Built with Rust and Tauri · Windows, macOS and Linux · by <a href="https://cbarange.com">Clement Baranger</a></sub>
</div>
