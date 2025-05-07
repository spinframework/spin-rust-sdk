# Cutting a new release of the Spin Rust SDK

To cut a new release, you will need to do the following:

1. Confirm that [CI is green](https://github.com/spinframework/spin-rust-sdk/actions) for the commit selected to be tagged and released.

2. Change the workspace version number in [Cargo.toml](./Cargo.toml) and the versions for any dependencies that are part of this workspace (e.g. `spin-macro`).

3. Create a pull request with these changes and merge once approved.

4. Checkout the commit with the version bump from above.

5. Create and push a new tag with a `v` and then the version number.

    As an example, via the `git` CLI:

    ```
    # Create a GPG-signed and annotated tag
    git tag -s -m "Spin Rust SDK v3.1.0" v3.1.0

    # Push the tag to the remote corresponding to spinframework/spin-rust-sdk (here 'origin')
    git push origin v3.1.0
    ```

6. Pushing the tag upstream will trigger the [release action](https://github.com/spinframework/spin-rust-sdk/actions/workflows/release.yml) which publishes the crates in this workspace to `crates.io`

7. If applicable, create PR(s) or coordinate [documentation](https://github.com/spinframework/spin-docs) needs, e.g. for new features or updated functionality.