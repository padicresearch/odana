#!/bin/bash
set -e
#
# Copyright (c) 2023 Padic Research.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#

# Function to install dependencies for Mac OS
function install_mac_dependencies() {
  echo "Installing dependencies for Mac OS"
  brew install llvm cmake automake libtool protobuf
}

# Function to install dependencies for Linux OS
function install_linux_dependencies() {
  echo "Installing dependencies for Linux OS"
  sudo apt install -y clang libclang-dev llvm llvm-dev linux-kernel-headers libev-dev cmake libprotobuf-dev protobuf-compiler
}

# Function to install Rust and Cargo
function install_rust_dependencies() {
  if ! command -v rustc >/dev/null; then
    echo "Rust is not installed. Installing with curl..."
    curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
    source "$HOME/.cargo/env"
  fi

  if ! command -v cargo >/dev/null; then
    echo "Cargo is not installed. Installing with rustup..."
    rustup install cargo
  fi
  rustup target add wasm32-unknown-unknown
}

# Function to install all dependencies
function install_dependencies() {
  if [ "$(uname)" == "Darwin" ]; then
    install_mac_dependencies
  elif [ "$(expr substr $(uname -s) 1 5)" == "Linux" ]; then
    install_linux_dependencies
  else
    echo "Unknown OS. Exiting..."
    exit 1
  fi

  install_rust_dependencies
}

run_docker() {
  if ! command -v docker >/dev/null; then
    echo "Docker is not installed. Please install Docker and try again."
    exit 1
  fi
  # build docker
  docker build -t odana/build -f ./docker/build.dockerfile .
  docker build -t odana/local -f ./docker/run.dockerfile .
  # run docker
  docker run -i -p 9020:9020 -p 9121:9121 -t odana/local "$@"
}

run_docker_localnetwork() {
  if ! command -v docker >/dev/null; then
    echo "Docker is not installed. Please install Docker and try again."
    exit 1
  fi
  # build docker
  docker build -t odana/build -f ./docker/build.dockerfile .
  # run docker compose
  docker compose -f docker/localnetwork/docker-compose.yml up
}

# Function to run the Node program
function run_node() {
  cargo run --release --bin odana -- "$@"
}

# Function to build the Rust project in release mode
function build_release() {
  install_dependencies
  cargo build --release
}

function print_help() {
  echo "Usage: $(basename "$0") [subcommand] [arguments]"
  echo ""
  echo "Available subcommands:"
  echo -e "\033[32m node\033[0m   \t\t[arguments]\tRuns the 'runner' program with the provided arguments."
  echo -e "\033[32m docker\033[0m \t\t[arguments]\tRuns a Docker command with the provided arguments."
  echo -e "\033[32m localnetwork\033[0m \t\t[arguments]\tRuns a Docker Compose with the provided arguments."
  echo -e "\033[32m install-dependencies\033[0m\t\t\tInstalls the dependencies for the current OS."
  echo -e "\033[32m release\033[0m\t\t\t\tBuild in release mode"
  echo -e "\033[32m --help\033[0m\t\t\t\t\tDisplays this help message."
}

case "$1" in
"install-dependencies")
  install_dependencies
  ;;
release)
  build_release
  ;;
node)
  shift
  run_node "$@"
  ;;
docker)
  shift
  run_docker "$@"
  ;;
localnetwork)
  shift
  run_docker_localnetwork "$@"
  ;;
-h | --help)
  print_help
  ;;
*)
  echo "run '$0 --help' to get more info"
  ;;

esac
