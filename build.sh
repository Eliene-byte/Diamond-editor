#!/bin/bash
# build.sh - Build script for DaVinci Clone

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
PROFILE="release"
FEATURES="gpu-vulkan,codec-ffmpeg,audio-cpal"
TARGET=""
CLEAN=false
CHECK_SIZE=true
RUN=false

usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -p, --profile PROFILE     Build profile: release, release-with-debug, debug (default: release)"
    echo "  -f, --features FEATURES   Comma-separated features (default: gpu-vulkan,codec-ffmpeg,audio-cpal)"
    echo "  -t, --target TARGET       Target triple (default: native)"
    echo "  -c, --clean               Clean before building"
    echo "  --no-size-check           Skip binary size check"
    echo "  -r, --run                 Run after building"
    echo "  -h, --help                Show this help"
    echo ""
    echo "Feature groups:"
    echo "  minimal:        gpu-vulkan,codec-ffmpeg,audio-cpal"
    echo "  full:           gpu-vulkan,codec-ffmpeg,audio-cpal,profiling,simd"
    echo "  metal:          gpu-metal,codec-ffmpeg,audio-cpal (macOS)"
    echo "  dx12:           gpu-dx12,codec-ffmpeg,audio-cpal (Windows)"
    echo "  headless:       gpu-vulkan,codec-ffmpeg (no audio)"
    echo ""
    echo "Examples:"
    echo "  $0                          # Standard release build"
    echo "  $0 -p release-with-debug    # Release with debug symbols"
    echo "  $0 -f full                  # All features"
    echo "  $0 -t x86_64-pc-windows-msvc # Cross-compile for Windows"
}

while [[ $# -gt 0 ]]; do
    case $1 in
        -p|--profile)
            PROFILE="$2"
            shift 2
            ;;
        -f|--features)
            FEATURES="$2"
            shift 2
            ;;
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -c|--clean)
            CLEAN=true
            shift
            ;;
        --no-size-check)
            CHECK_SIZE=false
            shift
            ;;
        -r|--run)
            RUN=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            usage
            exit 1
            ;;
    esac
done

# Feature presets
case $FEATURES in
    minimal) FEATURES="gpu-vulkan,codec-ffmpeg,audio-cpal" ;;
    full) FEATURES="gpu-vulkan,codec-ffmpeg,audio-cpal,profiling,simd" ;;
    metal) FEATURES="gpu-metal,codec-ffmpeg,audio-cpal" ;;
    dx12) FEATURES="gpu-dx12,codec-ffmpeg,audio-cpal" ;;
    headless) FEATURES="gpu-vulkan,codec-ffmpeg" ;;
esac

# Build command
CARGO_CMD="cargo build --profile $PROFILE --features \"$FEATURES\""
if [ -n "$TARGET" ]; then
    CARGO_CMD="$CARGO_CMD --target $TARGET"
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  DaVinci Clone Build Script${NC}"
echo -e "${BLUE}========================================${NC}"
echo -e "Profile:    ${YELLOW}$PROFILE${NC}"
echo -e "Features:   ${YELLOW}$FEATURES${NC}"
echo -e "Target:     ${YELLOW}${TARGET:-native}${NC}"
echo -e ""

# Clean if requested
if [ "$CLEAN" = true ]; then
    echo -e "${YELLOW}Cleaning...${NC}"
    cargo clean
fi

# Build
echo -e "${YELLOW}Building...${NC}"
echo -e "${BLUE}$CARGO_CMD${NC}"
eval $CARGO_CMD

# Check binary size
if [ "$CHECK_SIZE" = true ]; then
    BINARY_PATH="target/${TARGET:-}/$PROFILE/davinci-clone"
    if [ -f "$BINARY_PATH" ]; then
        SIZE=$(stat -c%s "$BINARY_PATH" 2>/dev/null || stat -f%z "$BINARY_PATH" 2>/dev/null)
        SIZE_MB=$((SIZE / 1024 / 1024))
        echo -e ""
        echo -e "${GREEN}Build successful!${NC}"
        echo -e "Binary: ${BLUE}$BINARY_PATH${NC}"
        echo -e "Size:   ${YELLOW}${SIZE_MB} MB${NC} (${SIZE} bytes)"
        
        if [ $SIZE -gt 47185920 ]; then
            echo -e "${RED}WARNING: Binary exceeds 45MB limit!${NC}"
        else
            echo -e "${GREEN}OK: Binary within 45MB target${NC}"
        fi
    fi
fi

# Run if requested
if [ "$RUN" = true ]; then
    BINARY_PATH="target/${TARGET:-}/$PROFILE/davinci-clone"
    if [ -f "$BINARY_PATH" ]; then
        echo -e ""
        echo -e "${YELLOW}Running...${NC}"
        exec "$BINARY_PATH"
    else
        echo -e "${RED}Binary not found!${NC}"
        exit 1
    fi
fi