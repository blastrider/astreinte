# Astreinte (MVP — lib + CLI)

Lib Rust + CLI minimalistes pour **gérer des astreintes localement** (sans BD) : création de créneaux, rotation round-robin, détection de conflits, swaps, import/export JSON/CSV.

## Statut & compatibilité
- Toolchain: **stable**
- Edition: **2021**
- **MSRV**: `1.79`
- `#![forbid(unsafe_code)]`, pas de `unwrap()` en prod
- Erreurs: `thiserror` (lib) / `anyhow` (CLI)

## Installation
```sh
cargo build --features serde
