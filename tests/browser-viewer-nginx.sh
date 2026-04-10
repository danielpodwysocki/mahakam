#!/bin/sh
# Regression test: verify nginx alias config serves noVNC index.html and NOT
# the nginx welcome page when the full path prefix is requested.
#
# Previously, `rewrite + root` caused the index subrequest to escape the
# location block, so nginx fell back to /usr/share/nginx/html/index.html
# (the welcome page) and returned HTTP 200 with wrong content. The fix was
# to use `alias` instead.
#
# This test exercises the container directly via docker — no cluster needed.

set -e

IMAGE="${1:-mahakam-browser-viewer:latest}"
ENV="regression-test"
PORT=16080
CONTAINER="browser-viewer-nginx-test-$$"

cleanup() { docker rm -f "$CONTAINER" > /dev/null 2>&1 || true; }
trap cleanup EXIT

echo "Starting $IMAGE as $CONTAINER …"
docker run -d \
  --name "$CONTAINER" \
  -e WS_NAME="$ENV" \
  -p "$PORT:6080" \
  "$IMAGE" > /dev/null

# Wait for nginx to be ready (entrypoint templates the config then starts nginx)
i=0
while [ "$i" -lt 15 ]; do
  BODY=$(curl -s --max-time 3 "http://localhost:$PORT/projects/viewers/$ENV/browser/" 2>/dev/null || true)
  if echo "$BODY" | grep -q "Browser Viewer"; then
    echo "PASS: /projects/viewers/$ENV/browser/ serves noVNC index (not nginx welcome page)"
    exit 0
  fi
  i=$((i + 1))
  sleep 1
done

# Show what was actually returned to help diagnose failures
echo "FAIL: expected 'Browser Viewer' in response body"
curl -s "http://localhost:$PORT/projects/viewers/$ENV/browser/" || true
exit 1
