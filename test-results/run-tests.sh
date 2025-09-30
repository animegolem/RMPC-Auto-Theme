#!/usr/bin/env bash
set -euo pipefail

TEST_DIR="$HOME/.config/rmpc/theme-switcher/test-results"
BINARY="$HOME/.local/bin/rmpc-theme-gen"

echo "=== Theme Generator Integration Tests ==="
echo "Started: $(date)"
echo ""

# Find some test images
TEST_IMAGES=(
    "/home/golem/.config/emacs/.local/straight/repos/el-get/logo/el-get.png"
    "/home/golem/.config/emacs/.local/straight/repos/el-get/logo/el-get.mono.png"
)

# Test each image
for i in "${!TEST_IMAGES[@]}"; do
    IMG="${TEST_IMAGES[$i]}"
    if [ ! -f "$IMG" ]; then
        echo "SKIP: $IMG (not found)"
        continue
    fi
    
    NAME="test-$(basename "$IMG" .png)"
    echo "--- Test $((i+1)): $NAME ---"
    
    # Generate theme
    START=$(date +%s%N)
    if "$BINARY" --image "$IMG" --k 8 --theme-output "$TEST_DIR/${NAME}.ron" > "$TEST_DIR/${NAME}.json" 2>&1; then
        END=$(date +%s%N)
        DURATION=$(( (END - START) / 1000000 ))
        echo "✓ Generation succeeded (${DURATION}ms)"
        
        # Check theme file
        if [ -f "$TEST_DIR/${NAME}.ron" ]; then
            SIZE=$(stat -f%z "$TEST_DIR/${NAME}.ron" 2>/dev/null || stat -c%s "$TEST_DIR/${NAME}.ron")
            echo "  Theme file: ${SIZE} bytes"
        fi
        
        # Extract color info from JSON
        if command -v jq &>/dev/null && [ -f "$TEST_DIR/${NAME}.json" ]; then
            BG=$(jq -r '.roleAssignments[] | select(.role=="Background") | .hex' "$TEST_DIR/${NAME}.json")
            TXT=$(jq -r '.roleAssignments[] | select(.role=="Text") | .hex' "$TEST_DIR/${NAME}.json")
            echo "  Colors: bg=$BG text=$TXT"
        fi
    else
        echo "✗ Generation failed"
        cat "$TEST_DIR/${NAME}.json" 2>/dev/null || true
    fi
    echo ""
done

echo "=== Test Complete ===" 
echo "Results saved to: $TEST_DIR"
