#!/usr/bin/env bash

set -e

if [ "$#" -ne 1 ]; then
  echo "Usage: $0 <test name>"
  exit 2;
fi

EXAMPLE_NAME=$1;

PROJECT_DIR=${0%/tests/*.sh}
EXAMPLE_DIR="${PROJECT_DIR}/new_project/examples/${EXAMPLE_NAME}"

if ! [ -d "$EXAMPLE_DIR" ]; then
  echo "Example dir does not exit: $EXAMPLE_DIR";
  exit 2;
fi

CCA="${PROJECT_DIR}/target/debug/create-comit-app"

LOG_FILE=$(mktemp)

## Start tests

$CCA start-env > /dev/null &
CCA_PID=$!
ENV_READY=false

# Start the environment
CCA_TIMEOUT=60

function check_containers() {
  ERROR=false
  for CONTAINER in ethereum bitcoin cnd_0 cnd_1; do
    NUM=$(docker ps -qf name=${CONTAINER} |wc -l)
    if test "$NUM" -ne 1; then
      ERROR=true;
      break;
    fi
  done
  $ERROR && echo 1 || echo 0
}

while [ $CCA_TIMEOUT -gt 0 ]; do
    if [ "$(check_containers)" -eq 0 ]; then
      CCA_TIMEOUT=0
      ENV_READY=true
    else
      sleep 1;
      CCA_TIMEOUT=$((CCA_TIMEOUT-1));
    fi
done

if ! $ENV_READY; then
  echo "FAIL: ${CONTAINER} docker container was not started."
  kill $CCA_PID;
  wait $CCA_PID;
  exit 1;
fi

# Run the example
RUN_TIMEOUT=60
TEST_PASSED=false

cd "${EXAMPLE_DIR}"

yarn install > /dev/null

yarn run start > "${LOG_FILE}" 2>&1 &
RUN_PID=$!

function check_swap() {
  local LOG_FILE=$1;
  grep -q "Bitcoin HTLC redeemed! TXID" "$LOG_FILE" && grep -q "Ethereum HTLC redeemed! TXID" "$LOG_FILE";
  echo $?;
}

while [ $RUN_TIMEOUT -gt 0 ]; do
  if [ "$(check_swap "$LOG_FILE")" -eq 0 ]; then
    RUN_TIMEOUT=0;
    TEST_PASSED=true;
  else
    sleep 1;
    RUN_TIMEOUT=$((RUN_TIMEOUT-1));
  fi
done

rm -f "${LOG_FILE}"

if $TEST_PASSED; then
  echo "SUCCESS: It swapped.";
  EXIT_CODE=0;
else
  echo "FAIL: It did not swap.";
  EXIT_CODE=1;
fi


function kill_process() {
  if ! kill $1 > /dev/null 2>&1; then
    echo "Could not send SIGTERM to process $1. Not running anymore?" >&2
fi
}

kill_process $RUN_PID;
kill_process $CCA_PID;

wait $RUN_PID || echo -ne ""; # It always return bad code. See #108
wait $CCA_PID;
exit $EXIT_CODE;
