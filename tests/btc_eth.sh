#!/usr/bin/env bash

set -e

echo "Running $0"

# Remove the `/tests` at the end of the current path
# to allow this script to be run from root project
# and from withing `tests` folder
PROJECT_DIR=${0%/*.sh}

"${PROJECT_DIR}/run_example.sh" "btc-eth"
