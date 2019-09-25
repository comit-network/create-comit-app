# Create Comit App (cca)

Set up a local development environment to build COMIT apps by running one command. 

## Install

### The Easy Way

1. Install Docker,
2. Install yarn or npm, 
3. Go get the latest [release](https://github.com/comit-network/create-comit-app/releases) for your platform,
4. Unzip & done!

### The Hard Way

Swedish style, build it yourself.

1. Install Docker,
2. Install Rust: `curl https://sh.rustup.rs -sSf | sh`,
3. Checkout the repo: `git clone https://github.com/comit-network/create-comit-app/`,
4. Build and install: ` cargo install --path create-comit-app`.

## Profit!

1. Create a new COMIT app project: `create-comit-app new my-cool-app`,
2. Start local blockchain nodes & COMIT nodes: `create-comit-app start-env`,
3. Open a new terminal:
   - `cd my-cool-app`,
   - Install dependencies: `yarn install` (or `npm install`),
   - Run the [hello-swap](https://github.com/comit-network/hello-swap/) example: `yarn start` (or `npm start`),
   - Hit `CTRL-C` once the swap is done.
