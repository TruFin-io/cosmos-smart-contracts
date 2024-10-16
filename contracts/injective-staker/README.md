# Injective staker


## Prerequisites

Before starting, make sure you have [rustup](https://rustup.rs/) along with a
recent `rustc` and `cargo` version installed. Currently, we are testing on 1.58.1+.

And you need to have the `wasm32-unknown-unknown` target installed as well.

You can check that via:

```sh
rustc --version
cargo --version
rustup target list --installed
# if wasm32 is not listed above, run this
rustup target add wasm32-unknown-unknown
```

## Compiling and running tests

Now that you created your custom contract, make sure you can compile and run it before
making any changes. Go into the root repository and do:

```sh
# this will produce a wasm build in ./target/wasm32-unknown-unknown/release/YOUR_NAME_HERE.wasm
make build

# this runs all the tests
make test
```

Alternatively, to compile the contract, you can cd into contracts/injective-staker and run
```sh
# this will produce a wasm build in ./target/wasm32-unknown-unknown/release/YOUR_NAME_HERE.wasm
# can only be run from inside the contract folder
cargo wasm
```

Finally, you can also run tests from anywhere using
```sh
cargo test
```

## Generating JSON Schema

```sh
# auto-generate the json schema
make schema
```

## Checking the contract is a valid CosmWasm contract

If it's not installed, install `cosmwasm-check` with:
```sh
cargo install cosmwasm-check
```

Then, you can run the following command to check the contract is a valid CosmWasm contract:
```sh
make validate
```

If you see an error like `Error: Instantiation failed`, it means the contract is not a valid CosmWasm contract.

## Preparing the Wasm bytecode for production

Before we upload it to a chain, we need to ensure the smallest output size possible,
as this will be included in the body of a transaction. We also want to have a
reproducible build process, so third parties can verify that the uploaded Wasm
code did indeed come from the claimed rust code.

To solve both these issues, run:

```sh
make build-optimized
```

This produces an `artifacts` directory with a `injective_staker.wasm`, as well as
`checksums.txt`, containing the Sha256 hash of the wasm file.
The wasm file is compiled deterministically (anyone else running the same
docker on the same git commit should get the identical file with the same Sha256 hash).
It is also stripped and minimized for upload to a blockchain (we will also
gzip it in the uploading process to make it even smaller).

## Migrating
When deploying the contract, make sure to set an admin using the --admin flag. This user will have the ability to migrate the contract if updates need to be made. To migrate the contract, deploy the new version with a `migrate` function that takes care of state changes. Then, call

```sh
injectived tx wasm migrate contract_address new_contract_code_id '{}' --from admin --chain-id="injective-888" --fees=8000000000000000inj --gas=50000000 --node="https://testnet.sentry.tm.injective.network:443"
```

This will migrate the contract to the new code version.
