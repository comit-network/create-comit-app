# Create Comit App (cca)

Set up a local development environment for COMIT apps with one command. 

## 1 - Install

1. Install Docker,
2. Install yarn or npm, 
3. Run `yarn create comit-app --help` or `npx create-comit-app --help` 

## 2 - Create your first project!

1. Start local blockchain nodes & COMIT nodes: `npx create-comit-app start-env`,
2. Create a new COMIT app project: `npx create-comit-app new my-cool-app`,
3. Open a new terminal:
   - `cd <path-to-my-cool-app>`,
   - Install dependencies: `yarn install` (or `npm install`),
   - Run the [hello-swap](https://github.com/comit-network/hello-swap/) example: `yarn start` (or `npm start`),
   - Hit `CTRL-C` once the swap is done.
   

# Appendix

Important: You don't have to follow this section, the above section is actually sufficient.

## Appendix A: Build the project yourself

1. Install Docker,
2. Install Rust: `curl https://sh.rustup.rs -sSf | sh`,
3. Checkout the repo: `git clone https://github.com/comit-network/create-comit-app/`,
4. Build and install: `cargo install --path create-comit-app`.