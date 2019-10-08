# Create Comit App

Set up a local development environment for COMIT apps with one command. 

## 1 - Install

1. Install Docker,
2. Install yarn or npm, 
3. Download and unzip the latest [release](https://github.com/comit-network/create-comit-app/releases) (`zip` or `tar.gz`)
4. Add the `create-comit-app` binary to `$PATH`

## 2 - Create your first project!

1. Create [hello-swap](https://github.com/comit-network/hello-swap/) app: `create-comit-app new hello-swap`,
2. Start environment (blockchain nodes and COMIT nodes): `create-comit-app start-env`
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
4. Build and install: `cargo install --path . create-comit-app`.
