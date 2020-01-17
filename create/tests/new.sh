#!/usr/bin/env bash

set -e

PROJECT_DIR=$(git rev-parse --show-toplevel)

CCA="${PROJECT_DIR}/target/debug/create-comit-app"
PROJECT_NAME="example-test-project"

function clean () {
    rm -rf "$PROJECT_NAME"
}

## Start tests
echo "Running $0"

$CCA "${PROJECT_NAME}" > /dev/null || (echo "FAIL: Non-zero exit code returned."; clean; exit 1;)

test -d "${PROJECT_NAME}" > /dev/null || (echo "FAIL: Project directory ${PROJECT_NAME} was not created."; clean; exit 1;)

test -f "${PROJECT_NAME}/package.json" > /dev/null  || (echo "FAIL: ${PROJECT_NAME} project was not initialized with a package.json file."; clean; exit 1;)

PACKAGE_JSON_PROJECT_NAME=$(cat "${PROJECT_NAME}/package.json" | jq .name)
if [ "$PACKAGE_JSON_PROJECT_NAME" != "\"$PROJECT_NAME\"" ]
then
  echo "FAIL: Project was not properly initialized with ${PROJECT_NAME} in package.json."
  ls
  cat "${PROJECT_NAME}/package.json"
  clean
  exit 1
fi

clean;

echo "SUCCESS: New project was initialized.";
exit 0;
