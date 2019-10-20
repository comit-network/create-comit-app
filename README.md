# Create Comit App

Set up a local development environment for COMIT apps with one command. 

## 1 - Install

1. Install Docker,
2. Install [yarn](https://yarnpkg.com/lang/en/docs/install/) or npm,
3. `create-comit-app` can now be run with either:
  - `yarn create comit-app --help`
  - `npx create-comit-app --help` (need to install npx first: `npm install --global npx`)

## 2 - Create your first project!

1. `yarn create comit-app new <your-app-name>`,
2. `yarn create comit-app start-env` (start blockchain nodes and COMIT nodes)
3. Run the [btc-eth](https://github.com/comit-network/create-comit-app/tree/master/new_project/examples/btc-eth) example (in separate terminal):
    1. Navigate to the btc-eth example app directory `cd <path-to-your-app>/examples/btc-eth`
    2. `yarn install` (or `npm install`) to install dependencies
    3. `yarn start` (or `npm start`) to run the application

You find additional examples in the [examples](https://github.com/comit-network/create-comit-app/tree/master/new_project/examples) directory that is created as part of step 1.

# Appendix

Important: You don't have to follow this section, the above section is actually sufficient.

## Appendix A: Build the project yourself

1. Install Docker,
2. Install Rust: `curl https://sh.rustup.rs -sSf | sh`,
3. Checkout the repo: `git clone https://github.com/comit-network/create-comit-app/; cd create-comit-app`,
4. Build and install: `make install`.
