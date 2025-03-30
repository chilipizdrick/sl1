#!/usr/bin/env bash

set -e

rustup toolchain install stable --component rust-src \
    && rustup target add riscv32imc-unknown-none-elf \
    && rustup toolchain install stable --component rust-analyzer
