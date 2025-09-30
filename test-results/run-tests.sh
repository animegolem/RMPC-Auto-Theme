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
            TRACK_LINE=$(grep -E 'track_style:' "$TEST_DIR/${NAME}.ron" | head -1 || true)
            ENDS_LINE=$(grep -E 'ends_style:' "$TEST_DIR/${NAME}.ron" | head -1 || true)
            if [[ "$TRACK_LINE" == *"$BG"* && "$ENDS_LINE" == *"$BG"* ]]; then
                echo "  Scrollbar styles match background"
            else
                echo "  WARN: Scrollbar styles do not match background"
            fi
            SB_ENABLED=$(jq -r '.scrollbarEnabled' "$TEST_DIR/${NAME}.json")
            echo "  Scrollbar enabled: $SB_ENABLED"
        fi
    else
        echo "✗ Generation failed"
        cat "$TEST_DIR/${NAME}.json" 2>/dev/null || true
    fi
    echo ""

    # No-scrollbar variant
    NO_SB_NAME="${NAME}-no-scrollbar"
    if "$BINARY" --image "$IMG" --k 8 --theme-output "$TEST_DIR/${NO_SB_NAME}.ron" --disable-scrollbar > "$TEST_DIR/${NO_SB_NAME}.json" 2>&1; then
        if command -v jq &>/dev/null; then
            SB_ENABLED=$(jq -r '.scrollbarEnabled' "$TEST_DIR/${NO_SB_NAME}.json")
            echo "  (No-scrollbar) scrollbar enabled: $SB_ENABLED"
        fi
        if grep -q 'scrollbar: None' "$TEST_DIR/${NO_SB_NAME}.ron"; then
            echo "  (No-scrollbar) Theme omits scrollbar block"
        else
            echo "  WARN: No-scrollbar theme still contains scrollbar block"
        fi
    else
        echo "✗ No-scrollbar variant failed"
        cat "$TEST_DIR/${NO_SB_NAME}.json" 2>/dev/null || true
    fi
    echo ""
done

echo "=== Test Complete ===" 
echo "Results saved to: $TEST_DIR"
