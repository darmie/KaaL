#!/bin/bash
# Enter Docker development shell

set -e

echo "🐳 Starting KaaL development shell..."
echo ""

docker run -it --rm \
    -v $(pwd):/workspace \
    -w /workspace \
    -e SEL4_PREFIX=/seL4 \
    kaal-dev:latest \
    /bin/bash

echo ""
echo "✅ Exited development shell"
