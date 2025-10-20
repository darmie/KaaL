# Chapter 8 Phase 1: Verification Setup Guide

**Date**: 2025-10-19
**Phase**: Setup & Framework (Week 1)
**Status**: 🚧 In Progress

---

## Overview

This guide walks through setting up the Verus verification framework for the KaaL microkernel.

**What we'll set up**:
1. Verus verification tool
2. Z3 SMT solver (dependency)
3. Verification directory structure
4. Build configuration for verification
5. Example verified module
6. CI/CD integration

---

## Prerequisites

### System Requirements

- **Rust**: 1.70+ (already installed for KaaL)
- **Python**: 3.8+ (for Verus build scripts)
- **Git**: For cloning Verus
- **C++ Compiler**: For building Z3
- **Memory**: 8GB+ RAM (for Z3 solver)
- **Disk**: 2GB for Verus + dependencies

### Supported Platforms

- ✅ macOS (Intel/Apple Silicon)
- ✅ Linux (x86_64, aarch64)
- ⚠️ Windows (WSL2 recommended)

---

## Step 1: Install Z3 SMT Solver

Z3 is the theorem prover that Verus uses to verify proofs.

### Option A: Package Manager (Recommended)

**macOS** (Homebrew):
```bash
brew install z3
```

**Linux** (apt):
```bash
sudo apt-get install z3
```

**Linux** (pacman):
```bash
sudo pacman -S z3
```

### Option B: Build from Source

```bash
# Clone Z3
git clone https://github.com/Z3Prover/z3.git
cd z3

# Build and install
python scripts/mk_make.py
cd build
make
sudo make install

# Verify installation
z3 --version  # Should show v4.12.x or newer
```

---

## Step 2: Install Verus

### Clone Verus Repository

```bash
# Clone into your home directory or preferred location
cd ~
git clone https://github.com/verus-lang/verus.git
cd verus

# Check out latest stable release (or main for cutting edge)
git checkout main
```

### Build Verus

```bash
# This takes 10-15 minutes
./tools/get-z3.sh  # Downloads prebuilt Z3 if needed
./tools/vargo build --release

# Verus binary will be at: ./source/target-verus/release/verus
```

### Add to PATH

**macOS/Linux** - Add to `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="$HOME/verus/source/target-verus/release:$PATH"
```

**Reload shell**:
```bash
source ~/.bashrc  # or source ~/.zshrc
```

### Verify Installation

```bash
verus --version
# Should output: verus 0.X.Y
```

---

## Step 3: Configure KaaL for Verification

### Directory Structure

The verification setup uses this structure:

```
kaal/
├── .verus/                    # Verus configuration
│   ├── config.toml           # Verus settings
│   └── cache/                # Verification cache
├── kernel/src/
│   ├── verified/             # Verified modules
│   │   ├── mod.rs           # Verified module exports
│   │   ├── bitmap.rs        # Example: verified bitmap
│   │   └── frame.rs         # Phase 2: verified frame allocator
│   └── memory/
│       └── frame_allocator.rs  # Original (unverified) implementation
├── docs/verification/
│   ├── SETUP.md              # This file
│   ├── WORKFLOW.md           # Verification workflow
│   └── PROOFS.md             # Proof techniques
└── tests/verification/       # Verification test suite
    └── bitmap_tests.rs
```

### Create Configuration File

Create `.verus/config.toml`:

```toml
# Verus Configuration for KaaL Microkernel

[verus]
# Verus binary path (auto-detected if in PATH)
# binary = "/path/to/verus"

# Z3 solver path (auto-detected if in PATH)
# z3_path = "/path/to/z3"

# Verification timeout (seconds)
timeout = 60

# Number of parallel verification tasks
jobs = 4

# Verification cache directory
cache_dir = ".verus/cache"

[build]
# Features to enable during verification
features = ["verification"]

# Target for verification (usually host, not embedded)
target = "x86_64-unknown-linux-gnu"  # or x86_64-apple-darwin

[proof]
# Enable debug output for failing proofs
debug = true

# Show Z3 queries for debugging
show_queries = false

# Maximum proof search depth
max_depth = 100
```

### Update .gitignore

Add to `.gitignore`:

```gitignore
# Verus verification cache
.verus/cache/

# Verification build artifacts
target-verus/
```

---

## Step 4: Create Example Verified Module

Let's start with a simple verified bitmap module to learn Verus.

### Create `kernel/src/verified/mod.rs`

```rust
//! Verified Kernel Modules
//!
//! This module contains mathematically verified implementations
//! of critical kernel data structures and algorithms.
//!
//! See docs/verification/ for verification methodology.

pub mod bitmap;
```

### Create `kernel/src/verified/bitmap.rs`

```rust
//! Verified Bitmap Operations
//!
//! This module provides a simple bitmap with verified operations.
//! Used as an introduction to Verus verification.

#![allow(unused_imports)]
use builtin::*;
use builtin_macros::*;
use vstd::prelude::*;

verus! {

/// A bitmap represented as a fixed-size array
pub struct Bitmap {
    bits: [bool; 64],
}

impl Bitmap {
    /// Specification: What does it mean for a bit to be set?
    pub closed spec fn bit_is_set(self, index: usize) -> bool {
        index < 64 && self.bits[index as int]
    }

    /// Specification: How many bits are set?
    pub closed spec fn count_set(self) -> int
        decreases 64
    {
        self.count_set_up_to(64)
    }

    /// Helper spec: Count set bits up to index
    spec fn count_set_up_to(self, index: int) -> int
        decreases index
    {
        if index <= 0 {
            0
        } else {
            let prev = self.count_set_up_to(index - 1);
            if self.bits[(index - 1) as int] {
                prev + 1
            } else {
                prev
            }
        }
    }

    /// Create a new empty bitmap
    ///
    /// Ensures: All bits are false
    pub fn new() -> (result: Self)
        ensures
            forall|i: int| 0 <= i < 64 ==> !result.bit_is_set(i as usize),
            result.count_set() == 0,
    {
        Bitmap {
            bits: [false; 64],
        }
    }

    /// Set a bit to true
    ///
    /// Requires: index < 64
    /// Ensures: The bit at index is now true
    pub fn set(&mut self, index: usize)
        requires
            index < 64,
        ensures
            self.bit_is_set(index),
            forall|i: usize| i != index ==> self.bit_is_set(i) == old(self).bit_is_set(i),
    {
        self.bits[index] = true;
    }

    /// Clear a bit to false
    ///
    /// Requires: index < 64
    /// Ensures: The bit at index is now false
    pub fn clear(&mut self, index: usize)
        requires
            index < 64,
        ensures
            !self.bit_is_set(index),
            forall|i: usize| i != index ==> self.bit_is_set(i) == old(self).bit_is_set(i),
    {
        self.bits[index] = false;
    }

    /// Check if a bit is set
    ///
    /// Requires: index < 64
    /// Ensures: Result matches specification
    pub fn is_set(&self, index: usize) -> (result: bool)
        requires
            index < 64,
        ensures
            result == self.bit_is_set(index),
    {
        self.bits[index]
    }

    /// Find first unset bit
    ///
    /// Ensures: If Some(i), then bit i is not set
    ///          If None, then all bits are set
    pub fn find_first_unset(&self) -> (result: Option<usize>)
        ensures
            match result {
                Some(i) => i < 64 && !self.bit_is_set(i),
                None => forall|i: usize| i < 64 ==> self.bit_is_set(i),
            }
    {
        let mut i: usize = 0;

        while i < 64
            invariant
                i <= 64,
                forall|j: usize| j < i ==> self.bit_is_set(j),
        {
            if !self.bits[i] {
                return Some(i);
            }
            i = i + 1;
        }

        None
    }
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_new() {
        let bm = Bitmap::new();
        for i in 0..64 {
            assert!(!bm.is_set(i));
        }
    }

    #[test]
    fn test_bitmap_set_clear() {
        let mut bm = Bitmap::new();
        bm.set(5);
        assert!(bm.is_set(5));
        bm.clear(5);
        assert!(!bm.is_set(5));
    }

    #[test]
    fn test_find_first_unset() {
        let mut bm = Bitmap::new();
        assert_eq!(bm.find_first_unset(), Some(0));

        bm.set(0);
        assert_eq!(bm.find_first_unset(), Some(1));

        // Set all bits
        for i in 0..64 {
            bm.set(i);
        }
        assert_eq!(bm.find_first_unset(), None);
    }
}
```

---

## Step 5: Verify the Example

### Run Verification

```bash
cd /path/to/kaal

# Verify the bitmap module
verus kernel/src/verified/bitmap.rs
```

**Expected output**:
```
verification results:: verified: 7 errors: 0
```

### Common Verification Errors

If you see errors, here are common causes:

**Error**: `loop invariant not maintained`
```
Fix: The loop invariant must be true:
1. Before the loop starts
2. After each iteration
3. When the loop exits
```

**Error**: `postcondition not satisfied`
```
Fix: The ensures clause isn't proven.
- Add intermediate proof steps with assert!()
- Strengthen loop invariants
- Add helper lemmas
```

**Error**: `timeout`
```
Fix: Proof too complex for Z3
- Simplify the proof
- Add manual proof steps
- Increase timeout in config
```

---

## Step 6: Integrate with Build System

### Update Cargo.toml

Add verification feature to `kernel/Cargo.toml`:

```toml
[features]
default = []
verification = ["vstd"]  # Enable Verus standard library

[dependencies]
# Verus standard library (only for verification)
vstd = { git = "https://github.com/verus-lang/verus", optional = true }
```

### Create Verification Script

Create `scripts/verify.sh`:

```bash
#!/bin/bash
# Verify all verified modules in the kernel

set -e

echo "🔍 Running Verus verification..."

# Verify each module
verus kernel/src/verified/bitmap.rs
# Add more modules as they're created:
# verus kernel/src/verified/frame.rs
# verus kernel/src/verified/pagetable.rs

echo "✓ All verifications passed!"
```

Make it executable:
```bash
chmod +x scripts/verify.sh
```

---

## Step 7: CI/CD Integration

### GitHub Actions Workflow

Create `.github/workflows/verify.yml`:

```yaml
name: Verus Verification

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Z3
        run: sudo apt-get install -y z3

      - name: Install Verus
        run: |
          git clone https://github.com/verus-lang/verus.git
          cd verus
          ./tools/get-z3.sh
          ./tools/vargo build --release
          echo "$PWD/source/target-verus/release" >> $GITHUB_PATH

      - name: Run Verification
        run: ./scripts/verify.sh
```

---

## Next Steps

Now that verification is set up:

1. **Learn Verus**: Work through the verified bitmap example
2. **Verify Frame Allocator**: Phase 2 - memory safety proofs
3. **Verify IPC**: Phase 3 - message passing correctness
4. **Verify Isolation**: Phase 4 - process isolation

---

## Troubleshooting

### Verus not found

**Problem**: `verus: command not found`

**Solution**:
```bash
# Check PATH
echo $PATH | grep verus

# Add to PATH if missing
export PATH="$HOME/verus/source/target-verus/release:$PATH"
```

### Z3 timeout

**Problem**: Verification times out

**Solutions**:
1. Increase timeout in `.verus/config.toml`
2. Simplify the proof
3. Add intermediate `assert!()` statements
4. Break complex functions into smaller pieces

### Proof fails unexpectedly

**Problem**: Proof should work but fails

**Solutions**:
1. Enable debug output: `verus --debug kernel/src/verified/bitmap.rs`
2. Show Z3 queries: `verus --show-triggers kernel/src/verified/bitmap.rs`
3. Add manual proof steps with `proof { ... }`
4. Check for integer overflow issues

---

## Resources

### Verus Documentation
- [Verus Guide](https://verus-lang.github.io/verus/)
- [Tutorial](https://github.com/verus-lang/verus/tree/main/source/docs)
- [Examples](https://github.com/verus-lang/verus/tree/main/source/rust_verify/example)

### Community
- [Verus Discussions](https://github.com/verus-lang/verus/discussions)
- [Zulip Chat](https://verus-lang.zulipchat.com/)

### Papers
- [Verus: Verifying Rust Programs](https://arxiv.org/abs/2303.05491)
- [Linear Dafny](https://dl.acm.org/doi/10.1145/3591280) (theoretical foundation)

---

**Phase 1 Complete When**:
- ✅ Verus installed and working
- ✅ Example bitmap module verified
- ✅ CI/CD running verification
- ✅ Team understands verification workflow

**Next**: [Phase 2 - Memory Safety Proofs](./PHASE_2_MEMORY.md)

---

**Last Updated**: 2025-10-19
**Status**: Ready to begin
