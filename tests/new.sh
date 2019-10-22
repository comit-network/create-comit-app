#!/usr/bin/env bash

set -e

# Remove the `/tests` at the end of the current path
# to allow this script to be run from root project
# and from withing `tests` folder
PWD=$(pwd)
CWD=${PWD%%/tests}

CCA="${CWD}/target/debug/create-comit-app"

# Random 10 char name
NAME=$(LC_CTYPE=C tr -dc A-Za-z0-9 < /dev/urandom |  fold -w 10 | head -n 1)

function clean () {
    rm -rf "$NAME"
}

## Start tests
echo "Running $0"

$CCA new "${NAME}" > /dev/null || (echo "FAIL: Non-zero exit code returned."; clean ; exit 1;)

test -d "${NAME}" > /dev/null || (echo "FAIL: Project directory ${NAME} was not created."; clean; exit 1;)

test -f "${NAME}/package.json" > /dev/null  || (echo "FAIL: ${NAME} project was not initialized with a package.json file."; clean; exit 1;)

clean;

echo "SUCCESS: New project was initialized.";
exit 0;
