#!/bin/bash
# build.sh - Build Colorant Rust with optimizations

echo "🔨 Building Colorant Rust v2.0..."

# Clean previous builds
cargo clean

# Build release with optimizations
echo "📦 Compiling release build..."
cargo build --release

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo ""
    echo "📁 Output files:"
    echo "   • target/release/colorant.exe - Main executable"
    echo ""
    echo "🚀 To run:"
    echo "   ./target/release/colorant.exe"
else
    echo "❌ Build failed!"
    exit 1
fi