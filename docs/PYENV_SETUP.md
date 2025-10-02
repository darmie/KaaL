# Python Environment Setup with Pyenv

## The Problem

When building KaaL, the `sel4-sys` crate needs Python packages (`tempita`, `ply`, etc.) to generate Rust bindings. If you use `pyenv`, you may encounter:

```
ModuleNotFoundError: No module named 'tempita'
```

This happens because:
1. **Pyenv manages multiple Python versions**
2. **The build script may use a different Python than where you installed packages**
3. **`/usr/bin/env python` resolves based on your PATH and pyenv shims**

## Understanding Your Environment

Check which Python environments you have:

```bash
# Show pyenv versions
pyenv versions

# Show which Python is active
pyenv which python3
python3 --version

# Show where pip installs packages
python3 -m pip show tempita

# Check what the build will use
/usr/bin/env python3 --version
```

### Example Output Explanation

```
$ pyenv versions
  system
  2.7.18
* 3.11.9 (set by /Users/you/.pyenv/version)  ← Active version
  3.13.0
```

The `*` shows your active pyenv Python.

## Solution: Install Packages in Pyenv Python

Since your pyenv is set to Python 3.11.9, install packages there:

```bash
# Method 1: Use pyenv's Python directly
~/.pyenv/versions/3.11.9/bin/python3 -m pip install --user \
    tempita ply jinja2 sel4-deps camkes-deps

# Method 2: Ensure pyenv shims are first in PATH
export PATH="$HOME/.pyenv/shims:$PATH"
python3 -m pip install --user tempita ply jinja2 sel4-deps camkes-deps

# Verify installation
python3 -c "import tempita; print('✓ Success')"
```

## Alternative Solutions

### Option 1: Use Homebrew Python (simpler but loses pyenv benefits)

```bash
# Unset pyenv temporarily
pyenv shell system

# Install with Homebrew Python
/opt/homebrew/bin/python3 -m pip install --user \
    --break-system-packages \
    tempita ply jinja2 sel4-deps camkes-deps

# Build project
cargo build
```

### Option 2: Create a Virtual Environment

```bash
# Create venv with your pyenv Python
python3 -m venv ~/kaal-venv

# Activate it
source ~/kaal-venv/bin/activate

# Install packages
pip install tempita ply jinja2 sel4-deps camkes-deps

# Build (with venv active)
cargo build

# Deactivate when done
deactivate
```

### Option 3: Set Python Path for Cargo

```bash
# Add to ~/.zshrc or ~/.bashrc
export PYTHON="/Users/amaterasu/.pyenv/versions/3.11.9/bin/python3"

# Or set temporarily
PYTHON="~/.pyenv/versions/3.11.9/bin/python3" cargo build
```

## Recommended Setup for KaaL Development

Add to your `~/.zshrc`:

```bash
# Pyenv initialization (if not already present)
export PYENV_ROOT="$HOME/.pyenv"
export PATH="$PYENV_ROOT/bin:$PATH"
eval "$(pyenv init -)"

# Set Python 3.11+ for KaaL (seL4 compatibility)
if [ -d "$(pwd)" ] && [ "$(basename $(pwd))" = "kaal" ]; then
    pyenv local 3.11.9
fi
```

Then install packages:

```bash
cd /path/to/kaal
pyenv local 3.11.9  # Creates .python-version file
python3 -m pip install --user tempita ply jinja2 sel4-deps camkes-deps
```

## Troubleshooting

### Issue: "Package installed but still not found"

```bash
# Check where it's installed
python3 -m pip show tempita

# Check if it's importable
python3 -c "import sys; print(sys.path)"
python3 -c "import tempita"
```

### Issue: "Multiple Python versions conflict"

```bash
# See all Pythons in PATH
which -a python3

# Force use of pyenv Python
$(pyenv which python3) -m pip install tempita
```

### Issue: "Permission denied"

```bash
# Don't use sudo with pip! Use --user instead
python3 -m pip install --user tempita

# Or use a virtual environment (recommended)
python3 -m venv ~/kaal-venv
source ~/kaal-venv/bin/activate
pip install tempita
```

## Verification Script

Run this to verify everything is set up correctly:

```bash
#!/bin/bash
echo "=== Pyenv Status ==="
pyenv version

echo -e "\n=== Python Location ==="
which python3
python3 --version

echo -e "\n=== Required Packages ==="
for pkg in tempita ply jinja2; do
    python3 -c "import $pkg; print('✓ $pkg')" 2>/dev/null || echo "✗ $pkg (missing)"
done

echo -e "\n=== seL4 Dependencies ==="
python3 -c "import sel4_deps; print('✓ sel4-deps')" 2>/dev/null || echo "✗ sel4-deps (missing)"

echo -e "\n=== Build Python ==="
/usr/bin/env python3 --version
echo "^ This is what cargo build uses"
```

## macOS Specific Notes

On macOS with Homebrew, you may have:
- **System Python**: `/usr/bin/python3` (usually Python 3.9)
- **Homebrew Python**: `/opt/homebrew/bin/python3` (latest)
- **Pyenv Pythons**: `~/.pyenv/versions/*/bin/python3`

The order in your PATH determines which is used:

```bash
# Good: Pyenv first
export PATH="$HOME/.pyenv/shims:/opt/homebrew/bin:$PATH"

# Shows which Python will be used
which python3
```

## Summary

✅ **Quick Fix:**
```bash
~/.pyenv/versions/3.11.9/bin/python3 -m pip install --user \
    tempita ply jinja2 sel4-deps camkes-deps
```

✅ **Verify:**
```bash
python3 -c "import tempita; print('Success!')"
```

✅ **Build:**
```bash
cargo build --workspace
```

---

**Note:** You only need to do this setup once per Python version. The packages will remain installed even after system reboots.
