# cargo-tests

description: generate llvm-cov reports when testings

commands:

     cargo tests <args>     run tests & generate cov report

     cargo tests all        runs clean && tests && report

     cargo tests clean      cleans up cov artifacts

     cargo tests report     open cov report
