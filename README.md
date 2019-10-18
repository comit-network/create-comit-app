# Create Comit App

Set up a local development environment for COMIT apps with one command. 

## 1 - Install

1. Install Docker,
2. Install [yarn](https://yarnpkg.com/lang/en/docs/install/) or npm,
3. `create-comit-app` can now be run with either:
  - `yarn create comit-app --help`
  - `npx create-comit-app --help` (need to install npx first: `npm install --global npx`)

## 2 - Create your first project!

1. Create [hello-swap](https://github.com/comit-network/hello-swap/) app: `yarn create comit-app new hello-swap`,
2. Start environment (blockchain nodes and COMIT nodes): `yarn create comit-app start-env`
3. Run hello-swap (in separate terminal):
    1. Navigate to the example app directory `cd <path-to-hello-swap>`
    2. `yarn install` (or `npm install`) to install dependencies
    3. `yarn start` (or `npm start`) to run the application

# Appendix

Important: You don't have to follow this section, the above section is actually sufficient.

## Appendix A: Build the project yourself

1. Install Docker,
2. Install Rust: `curl https://sh.rustup.rs -sSf | sh`,
3. Checkout the repo: `git clone https://github.com/comit-network/create-comit-app/; cd create-comit-app`,
4. Build and install: `make install`.
