#!/usr/bin/env bash

set -e

PROJECT_DIR=$(git rev-parse --show-toplevel)

BIN="${PROJECT_DIR}/target/debug/comit-scripts"

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

$BIN force-clean-env > /dev/null

TIMEOUT=60
CONTAINERS_DOWN=false

# Waiting for blockchain nodes to be down
while [ $TIMEOUT -gt 0 ]; do
    if [ "$(check_containers)" -eq 1 ]; then
      CONTAINERS_DOWN=true
      TIMEOUT=0
    else
      sleep 1
      TIMEOUT=$((TIMEOUT-1))
    fi
done

if ! $CONTAINERS_DOWN; then
  echo "FAIL: Containers were not stopped."
  exit 1
fi

if [ -d "$HOME/.create-comit-app" ]; then
  echo "FAIL: ~/.create-comit-app was not deleted."
  exit 1
fi

if test $(docker network ls | grep -n create-comit-app); then
  # IF grep successed then it found the network
  echo "FAIL: docker network is still up."
  exit 1
else
  echo "SUCCESS: Environment was cleaned up."
  exit 0
fi



