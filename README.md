# VoidGuard

> **Windows security and system monitoring — built locally, with visibility and control in mind.**

[![Download Latest Release](https://img.shields.io/github/v/release/Behnooddev/Web-VoidGuard?label=Download%20Latest%20Release\&style=for-the-badge)](https://github.com/Behnooddev/Web-VoidGuard/releases/latest)

**[⬇️ Download VoidGuard for Windows](https://github.com/Behnooddev/Web-VoidGuard/releases/latest)**

Pre-built Windows installers are available in every release:

* **`.msi`** — Windows Installer package
* **`.exe`** — NSIS installer

No Rust, Node.js, Visual Studio, or development environment is required to install and use the released application.

---

A local Windows security and system-monitoring desktop application for visibility, control, and auditing of your system.

VoidGuard provides process, service, network, port, firewall, DNS, file-integrity, startup, event, risk-scoring, and audit capabilities through a native Windows desktop interface.

VoidGuard is a **defensive security tool** intended for systems you own or are explicitly authorized to administer. It does not provide remote-control, arbitrary shell execution, or persistence capabilities of its own. See `SECURITY.md` for details.

## 🚀 Quick Start

### For Users

1. Open the **[Latest Release](https://github.com/Behnooddev/Web-VoidGuard/releases/latest)**.
2. Download either the `.msi` or `.exe` installer.
3. Run the installer.
4. Launch VoidGuard from Windows.

That's it. No development tools or source-code setup required.

### For Developers

If you want to build VoidGuard from source, see the **Development** section below.

## Repository Contents

* `src/`, `src-tauri/` — the VoidGuard application source code
* `handoffs/` — documentation for completed and in-progress development phases
* `wiki/` — source Markdown for the GitHub Wiki
* `docs/` — informational GitHub Pages documentation
* `.github/` — issue templates, pull request template, CI workflow, and Pages deployment workflow

## Current Status

VoidGuard currently implements **Phases 1 through 6** of the project's development plan described in `ARCHITECTURE.md`.

Implemented functionality includes:

* Application shell and dashboard
* Process monitoring and management
* Open-port monitoring and control
* Network adapter visibility
* Windows services management
* File-integrity monitoring
* Startup and persistence monitoring
* Scheduled Task monitoring
* Filterable event timeline
* Full audit log
* Risk correlation engine
* Windows Firewall rule management
* Per-interface DNS management
* Real system scanning
* Explained security scoring
* Configurable desktop notifications
* Configurable data retention
* Windows release packaging with MSI and NSIS installers

See the `handoffs/` directory for detailed records of what was implemented and verified during each development phase.

## Technology Stack

* **Frontend:** React + TypeScript + Tailwind CSS + Lucide Icons + Recharts
* **Desktop Shell:** Tauri
* **Backend:** Rust
* **Database:** SQLite
* **Native Windows APIs:** Windows crate
* **System Information:** sysinfo
* **Database Access:** rusqlite
* **WebView:** WebView2

## Development

### Prerequisites

Windows 10/11 with:

* [Rust](https://rustup.rs) stable toolchain
* [Node.js](https://nodejs.org) 18+
* Tauri prerequisites
* Microsoft Visual Studio C++ Build Tools
* WebView2

### Run in Development

```powershell
npm install
npm run tauri dev
```

### Build a Release

```powershell
npm run tauri build
```

Generated Windows installers are placed under:

```text
src-tauri/target/release/bundle/
```

## Project Layout

```text
src/
  components/         Shared UI components
  pages/              Application pages
  lib/                IPC wrappers and utilities
  types/              TypeScript types

src-tauri/
  src/
    main.rs           Application entry point
    commands/         Native Windows/Tauri commands
    db/               SQLite initialization and migrations
    models/            Shared Rust data models
```

## Security

VoidGuard is designed as a **local defensive security and system-monitoring application**.

It is intended for:

* Your own Windows systems
* Systems where you have explicit administrative authorization
* Development and security-testing environments you are authorized to manage

VoidGuard does not intentionally provide:

* Remote-control functionality
* Arbitrary remote shell execution
* Built-in persistence mechanisms
* Unauthorized access capabilities

For more information, see `SECURITY.md`.

## Documentation

* `ARCHITECTURE.md` — system architecture and development plan
* `SECURITY.md` — security model and privilege boundaries
* `DEVELOPMENT.md` — development workflow and testing
* `RELEASING.md` — release and packaging workflow
* `handoffs/` — phase-by-phase implementation records
* `wiki/` — extended project documentation
* `docs/` — GitHub Pages documentation

## Releases

Official pre-built Windows installers are published through **[GitHub Releases](https://github.com/Behnooddev/Web-VoidGuard/releases/latest)**.

Each release may provide:

* `.msi` Windows Installer package
* `.exe` NSIS installer

Users can install VoidGuard directly without cloning the repository or installing the development toolchain.

For developers, the complete source code and build instructions remain available in this repository.

## License

See the repository's license file for the applicable license and usage terms.
