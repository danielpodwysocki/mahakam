#!/bin/sh

API="${API_BASE_URL:-http://mahakam-api.mahakam-system:3000}"
WS_NAME="e2e-test"
PASS=0
FAIL=0

pass() { echo "  PASS: $1"; PASS=$((PASS + 1)); }
fail() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# Runs curl; on failure prints status + body and returns 1.
checked_curl() {
  RESPONSE=$(curl -s -w "\n__HTTP_STATUS__%{http_code}" "$@")
  STATUS=$(echo "$RESPONSE" | tail -1 | sed 's/__HTTP_STATUS__//')
  BODY=$(echo "$RESPONSE" | sed '$d')
  if [ "${STATUS#2}" = "$STATUS" ]; then
    echo "  HTTP $STATUS — $BODY"
    return 1
  fi
  printf '%s' "$BODY"
}

assert_contains() {
  if echo "$2" | grep -q "$1"; then
    pass "$3"
  else
    fail "$3 (expected '$1' in: $2)"
  fi
}

assert_not_contains() {
  if echo "$2" | grep -q "$1"; then
    fail "$3 (did not expect '$1' in: $2)"
  else
    pass "$3"
  fi
}

echo "=== Mahakam E2E Tests ==="
echo "API: $API"
echo ""

# --- Clean up any leftover state from previous runs ---
echo "--- cleanup: removing leftover $WS_NAME if present ---"
curl -s -X DELETE "$API/api/v1/workspaces/$WS_NAME" > /dev/null 2>&1 || true
sleep 2

echo ""
echo "--- GET /api/v1/workspaces (empty) ---"
LIST=$(checked_curl "$API/api/v1/workspaces") || { fail "GET /api/v1/workspaces failed"; LIST="[]"; }
echo "$LIST"
assert_not_contains "$WS_NAME" "$LIST" "workspace list does not contain $WS_NAME before create"

echo ""
echo "--- POST /api/v1/workspaces (vcluster install may take several minutes) ---"
CREATED=$(checked_curl -X POST "$API/api/v1/workspaces" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"$WS_NAME\",\"repos\":[\"https://github.com/danielpodwysocki/mahakam\"],\"viewers\":[\"terminal\",\"browser\"]}") || {
  fail "POST /api/v1/workspaces failed"
  echo "=== Results: $PASS passed, $FAIL failed ===" && exit 1
}
echo "$CREATED"
assert_contains "$WS_NAME" "$CREATED" "create response contains workspace name"
assert_contains "namespace" "$CREATED" "create response contains namespace field"
assert_contains "status" "$CREATED" "create response contains status field"

echo ""
echo "--- GET /api/v1/workspaces (after create) ---"
LIST=$(checked_curl "$API/api/v1/workspaces") || { fail "GET after create failed"; LIST="[]"; }
echo "$LIST"
assert_contains "$WS_NAME" "$LIST" "workspace list contains $WS_NAME after create"

echo ""
echo "--- waiting for $WS_NAME to become ready (up to 15 min) ---"
READY=0
for i in $(seq 1 180); do
  STATUS_VAL=$(curl -s "$API/api/v1/workspaces" \
    | grep -o '"status":"[^"]*' | head -1 | sed 's/"status":"//')
  if [ "$STATUS_VAL" = "ready" ]; then
    READY=1
    break
  fi
  if [ "$STATUS_VAL" = "failed" ]; then
    break
  fi
  sleep 5
done
if [ "$READY" = "1" ]; then
  pass "$WS_NAME reached ready status"
else
  fail "$WS_NAME did not reach ready status (last status: $STATUS_VAL)"
fi

# Poll a URL until it returns 200 with body containing EXPECT, or times out.
# Usage: poll_viewer LABEL URL EXPECT MAX_ATTEMPTS SLEEP_SECS [CURL_EXTRA_FLAGS]
poll_viewer() {
  LABEL="$1"; URL="$2"; EXPECT="$3"; MAX="$4"; INTERVAL="$5"; EXTRA="${6:-}"
  i=0
  while [ "$i" -lt "$MAX" ]; do
    BODY=$(curl -s $EXTRA --max-time 10 "$URL")
    CODE=$?
    if echo "$BODY" | grep -q "$EXPECT"; then
      pass "$LABEL endpoint returns expected content"
      return 0
    fi
    i=$((i + 1))
    echo "  $LABEL: expected '$EXPECT' not found (attempt $i/$MAX, retrying in ${INTERVAL}s…)"
    sleep "$INTERVAL"
  done
  fail "$LABEL endpoint did not return expected content after $MAX attempts"
}

echo ""
echo "--- GET viewer terminal endpoint for $WS_NAME ---"
if [ "$READY" = "1" ]; then
  # Viewers are spawned after ArgoCD becomes Healthy; poll until the pod is up.
  TERMINAL_URL="http://viewer-${WS_NAME}-terminal.ws-${WS_NAME}:80/projects/viewers/${WS_NAME}/terminal/"
  poll_viewer "terminal viewer" "$TERMINAL_URL" "ttyd" 30 5
else
  fail "skipping terminal viewer test — workspace not ready"
fi

echo ""
echo "--- GET viewer browser endpoint for $WS_NAME (follows redirect to noVNC) ---"
if [ "$READY" = "1" ]; then
  # Check that the browser viewer index.html contains the "Browser Viewer" title,
  # proving noVNC content is served (not the nginx default welcome page).
  BROWSER_SVC_URL="http://viewer-${WS_NAME}-browser.ws-${WS_NAME}:80/projects/viewers/${WS_NAME}/browser/"
  poll_viewer "browser viewer" "$BROWSER_SVC_URL" "Browser Viewer" 30 5
else
  fail "skipping browser viewer test — workspace not ready"
fi

echo ""
echo "--- DELETE /api/v1/workspaces/$WS_NAME ---"
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API/api/v1/workspaces/$WS_NAME")
echo "HTTP $HTTP_STATUS"
if [ "$HTTP_STATUS" = "204" ]; then
  pass "delete returns 204"
else
  fail "delete returned $HTTP_STATUS (expected 204)"
fi

echo ""
echo "--- waiting for $WS_NAME to disappear from list (ArgoCD cascade is async) ---"
GONE=0
for i in $(seq 1 60); do
  LIST=$(curl -s "$API/api/v1/workspaces")
  if ! echo "$LIST" | grep -q "\"name\":\"$WS_NAME\""; then
    GONE=1
    break
  fi
  sleep 5
done
if [ "$GONE" = "1" ]; then
  pass "workspace list does not contain $WS_NAME after delete"
else
  fail "workspace list still contains $WS_NAME after delete (ArgoCD cascade may still be running)"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then exit 1; fi
