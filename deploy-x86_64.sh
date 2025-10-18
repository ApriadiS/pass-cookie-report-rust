#!/bin/bash
# Deploy script for x86_64 architecture

set -e

echo "🚀 Deploying Pass Cookie Report API (x86_64)..."

# Check if .env exists
if [ ! -f .env ]; then
    echo "❌ Error: .env file not found!"
    echo "📝 Please copy .env.example to .env and fill in your credentials"
    exit 1
fi

# Stop and remove existing container
echo "🛑 Stopping existing container..."
docker-compose down 2>&1 | while IFS= read -r line; do
    echo "   $line"
done || true
echo "✅ Container stopped!"

# Build image first (separate from docker-compose to save memory)
echo "🔨 Building Docker image..."
echo "⏳ This may take a few minutes..."
docker build -f Dockerfile -t pass-cookie-report-rust . 2>&1 | while IFS= read -r line; do
    echo "   $line"
done
echo "✅ Image built successfully!"

# Start container with docker-compose
echo ""
echo "🚀 Starting container..."
docker-compose up -d 2>&1 | while IFS= read -r line; do
    echo "   $line"
done
echo "✅ Container started successfully!"

echo "✅ Deployment complete!"
echo "📊 Container status:"
docker ps -f name=pass-cookie-report-rust

echo ""
echo "📝 View logs with: docker logs -f pass-cookie-report-rust"
echo "🔍 Test API with: curl http://localhost:3000/"