#!/usr/bin/env bash

set -e

PROJECT_DIR=${0%/tests/*.sh}

CCA="${PROJECT_DIR}/target/debug/create-comit-app"

# Random 10 char name
NAME="example-test-project"

function clean () {
    rm -rf "$NAME"
}

## Start tests
echo "Running $0"

$CCA new "${NAME}" > /dev/null || (echo "FAIL: Non-zero exit code returned."; clean ; exit 1;)

test -d "${NAME}" > /dev/null || (echo "FAIL: Project directory ${NAME} was not created."; clean; exit 1;)

test -f "${NAME}/package.json" > /dev/null  || (echo "FAIL: ${NAME} project was not initialized with a package.json file."; clean; exit 1;)

PACKAGE_JSON_PROJECT_NAME=$(cat "${NAME}/package.json" | jq .name)
if [ "$PACKAGE_JSON_PROJECT_NAME" != "\"$NAME\"" ]
then
  echo "FAIL: Project was not properly initialized with ${NAME} in package.json."
  clean
  exit 1
fi

clean;

echo "SUCCESS: New project was initialized.";
exit 0;
