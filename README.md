# cargo-tests

description: generate llvm-cov reports when testings
note: requires nightly

code based on https://doc.rust-lang.org/nightly/unstable-book/compiler-flags/source-based-code-coverage.html

commands:

     cargo tests <args>     run tests & generate cov report

     cargo tests all        runs clean && tests && report

     cargo tests clean      cleans up cov artifacts

     cargo tests report     open cov report

## Setup

in order to use this package you must have the following set up

rustup default nightly
rustup component add llvm-tools-preview
