#!/usr/bin/env bash

set -e

echo "Running $0"

PROJECT_DIR=$(git rev-parse --show-toplevel)
EXAMPLE_DIR="${PROJECT_DIR}/create/new_project/demos/btc_eth"

if ! [ -d "$EXAMPLE_DIR" ]; then
  echo "Example dir does not exit: $EXAMPLE_DIR";
  exit 2;
fi

LOG_FILE=$(mktemp)

## Start tests

cd "${EXAMPLE_DIR}"
yarn install > /dev/null
yarn run start-env > /dev/null &
STARTENV_PID=$!
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
  kill $STARTENV_PID;
  wait $STARTENV_PID;
  exit 1;
fi

# Run the example
RUN_TIMEOUT=60
TEST_PASSED=false

yarn run swap > "${LOG_FILE}" 2>&1 &
RUN_PID=$!

function check_swap() {
  local LOG_FILE=$1;
  grep -q "Swapped!" "$LOG_FILE";
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

if $TEST_PASSED; then
  echo "SUCCESS: It swapped.";
  EXIT_CODE=0;
else
  echo "FAIL: It did not swap.";
  cat "$LOG_FILE";
  EXIT_CODE=1;
fi

wait $RUN_PID || true;

kill -s SIGINT $STARTENV_PID;
wait $STARTENV_PID || true;

rm -f "${LOG_FILE}"
exit $EXIT_CODE;

