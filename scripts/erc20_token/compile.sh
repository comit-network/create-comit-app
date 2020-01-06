set -e

yarn
yarn truffle compile

cat ./build/contracts/Token.json | jq .bytecode -r > ./build/contract.hex
cat ./build/contracts/Token.json | jq .abi -r > ./build/abi.json
