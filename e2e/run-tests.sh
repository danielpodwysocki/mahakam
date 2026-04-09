#!/bin/sh

API="${API_BASE_URL:-http://mahakam-api.mahakam-system:3000}"
ENV_NAME="e2e-test"
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
echo "--- cleanup: removing leftover $ENV_NAME if present ---"
curl -s -X DELETE "$API/api/v1/environments/$ENV_NAME" > /dev/null 2>&1 || true
sleep 2

echo ""
echo "--- GET /api/v1/environments (empty) ---"
LIST=$(checked_curl "$API/api/v1/environments") || { fail "GET /api/v1/environments failed"; LIST="[]"; }
echo "$LIST"
assert_not_contains "$ENV_NAME" "$LIST" "env list does not contain $ENV_NAME before create"

echo ""
echo "--- POST /api/v1/environments (vcluster install may take several minutes) ---"
CREATED=$(checked_curl -X POST "$API/api/v1/environments" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"$ENV_NAME\",\"repos\":[\"https://github.com/danielpodwysocki/mahakam\"]}") || {
  fail "POST /api/v1/environments failed"
  echo "=== Results: $PASS passed, $FAIL failed ===" && exit 1
}
echo "$CREATED"
assert_contains "$ENV_NAME" "$CREATED" "create response contains env name"
assert_contains "namespace" "$CREATED" "create response contains namespace field"
assert_contains "status" "$CREATED" "create response contains status field"

echo ""
echo "--- GET /api/v1/environments (after create) ---"
LIST=$(checked_curl "$API/api/v1/environments") || { fail "GET after create failed"; LIST="[]"; }
echo "$LIST"
assert_contains "$ENV_NAME" "$LIST" "env list contains $ENV_NAME after create"

echo ""
echo "--- waiting for $ENV_NAME to become ready (up to 15 min) ---"
READY=0
for i in $(seq 1 180); do
  STATUS_VAL=$(curl -s "$API/api/v1/environments" \
    | grep -o "\"status\":\"[^\"]*\"" | head -1 | grep -o "[^\"]*$")
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
  pass "$ENV_NAME reached ready status"
else
  fail "$ENV_NAME did not reach ready status (last status: $STATUS_VAL)"
fi

echo ""
echo "--- GET viewer terminal endpoint for $ENV_NAME ---"
if [ "$READY" = "1" ]; then
  VIEWER_URL="http://viewer-${ENV_NAME}.env-${ENV_NAME}:7681/projects/viewers/${ENV_NAME}/"
  VIEWER_HTTP=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$VIEWER_URL")
  echo "HTTP $VIEWER_HTTP ($VIEWER_URL)"
  if [ "$VIEWER_HTTP" = "200" ]; then
    pass "viewer endpoint returns 200"
  else
    fail "viewer endpoint returned $VIEWER_HTTP (expected 200)"
  fi
else
  fail "skipping viewer test — environment not ready"
fi

echo ""
echo "--- DELETE /api/v1/environments/$ENV_NAME ---"
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X DELETE "$API/api/v1/environments/$ENV_NAME")
echo "HTTP $HTTP_STATUS"
if [ "$HTTP_STATUS" = "204" ]; then
  pass "delete returns 204"
else
  fail "delete returned $HTTP_STATUS (expected 204)"
fi

echo ""
echo "--- GET /api/v1/environments (after delete) ---"
LIST=$(checked_curl "$API/api/v1/environments") || { fail "GET after delete failed"; LIST="[]"; }
echo "$LIST"
assert_not_contains "$ENV_NAME" "$LIST" "env list does not contain $ENV_NAME after delete"

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then exit 1; fi
