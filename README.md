# uChain - Blockchain for prototyping
> Work in progress

Blockchain base/framework for prototyping blockchain projects
## Quickstart

---
### 🧰 Install Dependencies
* Rust Toolchain
    ```shell
    # Run the following in your terminal, then follow the onscreen instructions.
    curl https://sh.rustup.rs -sSf | sh
    ```
    ```shell
    rustup default nightly
    ```
* Rocks DB
    ```shell
    sudo apt install clang libclang-dev llvm llvm-dev linux-kernel-headers libev-dev
    ```
### ⌛️ Running node `Linux/Mac`
* Build the node from source
    ```shell
    cargo build --release --package node
    ```
* Generate Node Identity
    ```shell
    ./target/release/node identity generate
    ```
* Initialize node configuration
    ```shell
    ./target/release/node config init
    ```
* Create a miner account (optional - required if you want to run as a miner)
    ```shell
    ./target/release/node account new
    ```
    Output:
    ```json
    {
       "address": "0xa253d958f45db8aa712787cee1322aa2d7438a8f",
       "secret" : "0xd2e73c5bf670001803d9436a78d14ca9c12185f33fbc197274a104d817a088ab"
    }
    ```
   Set miner
    ```shell
    ./target/release/node config update --miner="0xa253d958f45db8aa712787cee1322aa2d7438a8f"
    ```
* Run node
    ```shell
    /target/release/node run
    ```

### Running node `Windows`: 🤷🏼‍♂️