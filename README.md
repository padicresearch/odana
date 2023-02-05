# Odana

Odana is an open source decentralized platform for building and running decentralized applications (dapps) and smart
contracts.

## Key Features

* Proof of Work
* Multi Runtime

## Quickstart

### 🧰 Install Dependencies

* **Rust Toolchain `Linux/MacOS`**
    ```shell
    curl https://sh.rustup.rs -sSf | sh
    ```
    ```shell
    rustup default nightly
    ```
* **Install gRPC, RockDB dependencies**

  **`Linux`**

  ```shell
  sudo apt install clang libclang-dev llvm llvm-dev linux-kernel-headers libev-dev
  ```
  ```shell
  sudo apt install cmake libprotobuf-dev protobuf-compiler
  ```
  **`MacOS`**

  ```shell
  brew install llvm cmake automake libtool protobuf
  ```
* **Install WASM target**
  ```shell
  rustup target add wasm32-unknown-unknown
  ```

### ⬇️ Download

* Download the source code
    ```shell
    git clone https://github.com/padicresearch/odana
    ```
    ```shell
    cd odana
    ```

### ⌛️ Running node `Linux/MacOS`

* Build the node from source
    ```shell
    cargo build --release
    ```
* Generate Node Identity
    ```shell
    ./target/release/odana identity generate
    ```
* Initialize node configuration
    ```shell
    ./target/release/odana config init
    ```
* Create a miner account (optional - required if you want to run as a miner)
  ```shell
  ./target/release/odana account new
  ```
  Output:
  ```json
  {
    "address": "0xffff…ffff",
    "secret" : "0xffff…ffff"
  }
  ```
  Set miner
  ```shell
  ./target/release/odana config update --miner="0xffff…ffff"
  ```
* Run node
  ```shell
  ./target/release/odana run
  ```
