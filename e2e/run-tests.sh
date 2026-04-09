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
