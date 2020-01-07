# Releasing create-comit-app and comit-scripts

There are 2 crates/binaries/npm packages in this repository:
- `create` dir contains `create-comit-app` binary and npm package
- `scripts` dir contains `comit-scripts` binary and npm package

The version of the Rust crate and corresponding npm must always stay aligned.
E.g., `scripts/Cargo.toml` and `scripts/npm/package.json` must have the same version.

`create-comit-app` version bump must be at least as significant as `comit-scripts` version bump.
E.g. if `comit-scripts` has a MINOR version bump than `create-comit-app` that includes the new `comit-scripts` version must have a MINOR or MAJOR version bump. 

The dependencies are as followed:

- `create-comit-app` contains examples.
- These examples depends on a specific version of `comit-scripts`

Hence, `create-comit-app` depends on `comit-scripts`.

## Releasing `create-comit-app`

To release a new version `X.Y.Z` of `create-comit-app`:
1. Create new `release/create/X.Y.Z` git branch from `master`,
1. Bump version in `create/Cargo.toml`,
1. Run `cargo build` to update `Cargo.lock` file,
1. Bump version in `create/npm/package.json`,
1. Update `create/CHANGELOG.md` file,
1. Create commit with title `Release create-comit-app X.Y.Z`
1. Open Pull Request,
1. Wait until checks pass and PR is approved,
1. Tag release commit `git tag create-comit-app-X.Y.Z`,
1. Push tag: `git push --tags`,
1. Wait for GitHub Action to proceed with binary and npm release,
1. Merge PR.

## Releasing `comit-scripts`

To release a new version `X.Y.Z` of `comit-scripts`:
1. Create new `release/scripts/X.Y.Z` git branch from `master`,
1. Bump version in `scripts/Cargo.toml`,
1. Run `cargo build` to update `Cargo.lock` file,
1. Bump version in `scripts/npm/package.json`,
1. Update `scripts/CHANGELOG.md` file,
1. Create commit with title `Release comit-scripts X.Y.Z`
1. Open Pull Request,
1. Wait until checks pass and PR is approved,
1. Tag release commit `git tag comit-scripts-X.Y.Z`,
1. Push tag: `git push --tags`,
1. Wait for GitHub Action to proceed with binary and npm release,
1. Merge PR, pull master locally: `git checkout master && git pull master`
1. Create new branch: `git checkout -b comit-scripts-X.Y.Z`
1. Update `comit-scripts` version in the `package.json` file of all `create/new_project/examples`,
1. Run `yarn install` in each example folder to update `yarn.lock` files,
1. Update `comit-scripts` version in `new_projectpackage.json`,
1. Do a PR as usual,
1. Once merged, proceed with release of `create-comit-app` as per [Releasing `create-comit-app`](#releasing-create-comit-app).