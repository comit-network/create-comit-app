#!/usr/bin/env bash

set -e

PROJECT_DIR=${0%/tests/*.sh}

CCA="${PROJECT_DIR}/target/debug/create-comit-app"

## Start tests
echo "Running $0"

$CCA start-env > /dev/null &
PID=$!

TIMEOUT=60
ENV_UP=false

# Count the number of containers. Only care about blockchain containers
function check_containers() {
  ONE_ABSENT=false
  for CONTAINER in ethereum bitcoin; do
    NUM=$(docker ps -qf name=${CONTAINER} | wc -l)
    if [ "$NUM" -eq 1 ]; then
      # Container is present, do nothing
      continue;
    else
      # Container is missing!
      ONE_ABSENT=true
      break;
    fi
  done
  $ONE_ABSENT && echo 1 || echo 0
}

# Waiting for blockchain nodes to be up
while [ $TIMEOUT -gt 0 ]; do
    if [ "$(check_containers)" -eq 0 ]; then
      ENV_UP=true;
      TIMEOUT=0
    else
      sleep 1;
      TIMEOUT=$((TIMEOUT-1));
    fi
done

if ! $ENV_UP; then
  echo "FAIL: Environment never started";
  exit 1;
fi

# Environment is (at least half) started

# SIGKILL create-comit-env, the env is not cleaned up
kill -9 $PID;
wait $PID || true; # Working around the `set -e` as SIGKILL makes it return bad code

# Check the environment is still up after SIGKILL'ng create-comit-app
if [ "$(check_containers)" -ne 0 ]; then
      echo "FAIL: The environment is not up for some reason."
      exit 1
fi

$CCA force-clean-env > /dev/null

TIMEOUT=60
CONTAINERS_DOWN=false

# Waiting for blockchain nodes to be down
while [ $TIMEOUT -gt 0 ]; do
    if [ "$(check_containers)" -eq 1 ]; then
      CONTAINERS_DOWN=true;
      TIMEOUT=0
    else
      sleep 1;
      TIMEOUT=$((TIMEOUT-1));
    fi
done

if ! $CONTAINERS_DOWN; then
  echo "FAIL: Containers were not stopped.";
  exit 1;
fi

if [ -d "$HOME/.create-comit-app" ]; then
  echo "FAIL: ~/.create-comit-app was not deleted.";
  exit 1;
fi

if test $(docker network ls | grep -n create-comit-app); then
  # IF grep successed then it found the network
  echo "FAIL: docker network is still up.";
  exit 1;
else
  echo "SUCCESS: Environment was cleaned up.";
  exit 0;
fi



