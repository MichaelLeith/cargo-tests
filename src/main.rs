//! # cargo-tests
//!
//! description: generate llvm-cov reports when testings
//! commands:
//!     cargo tests <args>     run tests & generate cov report
//!     cargo tests all        runs clean && tests && report
//!     cargo tests clean      cleans up cov artifacts
//!     cargo tests report     open cov report
use std::{ffi::OsString, fs, path::Path, process::{Command, Stdio}};

use log::{debug, error};
use simple_logger::SimpleLogger;
/**
    cargo-tests is a small proxy that adds coverage to cargo test
 */
fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Error)
        .init().unwrap();
    let args: Vec<_> = std::env::args_os().collect();
    let mut i = 1;
    if args.get(i).and_then(|s| s.to_str()) == Some("tests") {
        i += 1;
    }

    let config = get_config();
    debug!("found config: {}", config);
    let (path, exec) = get_executable_name(&config);
    match args.get(i) {
        Some(subcommand) => match subcommand.to_str().unwrap() {
            "all" => {
                run_clean(&path, &args[i+1..]);
                run_tests(&path, &exec, &args[i..]);
                run_report(&path, &exec, &args[i+1..]);
            },
            "report" => run_report(&path, &exec, &args[i+1..]),
            "clean" => run_clean(&path, &args[i+1..]),
            "help" => run_help(),
            _ => run_tests(&path, &exec, &args[i..])
        }
        _ => run_tests(&path, &exec, &args[i..])
    }
}

fn run_help() {
    println!("cargo tests");
    println!("description: generate llvm-cov reports when testings");
    println!("commands:");
    println!("    cargo tests <args>: run tests & generate cov report");
    println!("    cargo tests all: runs clean && tests && report");
    println!("    cargo tests clean: cleans up cov artifacts");
    println!("    cargo tests report: open cov report");
}

fn run_clean(path: &str, _args: &[OsString]) {
    debug!("running clean");
    // @todo: get right directory
    let path = format!("{}/cov", path);
    if fs::metadata(&path).is_ok() {
        fs::remove_dir_all(path).expect("unable to delete cov folder");
    }
}

fn run_tests(path: &str, exec: &str, args: &[OsString]) {
    run_clean(path, args);
    debug!("running tests");


    let mut params = Vec::with_capacity(args.len() + 1);
    params.push("test");
    args.iter().for_each(|a| params.push(a.to_str().unwrap()));
    let test = Command::new("cargo")
            .env("RUSTFLAGS", "-Zinstrument-coverage")
            .env("LLVM_PROFILE_FILE", "cov/json5format-%m.profraw")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .args(&params)
            .output()
            .expect("failed to execute process");
    if !test.status.success() {
        error!("tests failed");
        return;
    }
    debug!("finished running tests, updating profdata");


    let prof = format!("{}/cov/json5format.profdata", path);
    let profdata = Command::new("llvm-profdata")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("merge")
        .arg("-sparse")
        .args(fs::read_dir(format!("{}/cov", path)).unwrap()
            .map(|p| p.unwrap().path()))
        .arg("-o")
        .arg(&prof)
        .output()
        .expect("failed to generate profdata");
    if !profdata.status.success() {
        error!("failed to generate profdata");
        return;
    }
    debug!("created profdata, running cov");

    let target = get_targets(path, exec);

    Command::new("llvm-cov")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("report")
        .arg("--use-color")
        .arg("--ignore-filename-regex").arg(".*/.cargo/registry")
        .arg("--instr-profile").arg(&prof)
        .args(&target)
        .output()
        .expect("failed to run cov");

    Command::new("llvm-cov")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .arg("show")
        .arg("--use-color")
        .arg("--ignore-filename-regex").arg(".*/.cargo/registry")
        .arg("--instr-profile").arg(prof)
        .arg("--show-instantiations")
        .arg("--show-line-counts-or-regions")
        .arg("--Xdemangler=rustfilt")
        .arg("-format").arg("html")
        .arg("-output-dir").arg(format!("{}/cov/html", path))
        .args(target)
        .output()
        .expect("failed to run cov");
}

fn run_report(path: &str, exec: &str, args: &[OsString]) {
    let html = format!("{}/cov/html/index.html", path);
    if !fs::metadata(&html).is_ok() {
        debug!("no report found, running tests");
        run_tests(path, exec, args);
    }
    Command::new("open")
        .arg(html)
        .output()
        .expect("failed to open coverage data");
}

fn get_config() -> String {
    // first make sure we're in a cargo repo (and store the config path)
    let proj = Command::new("cargo")
        .arg("locate-project")
        .arg("--message-format")
        .arg("plain")
        .output()
        .expect("failed to locate project");
    if !proj.status.success() {
        error!("failed to locate project");
        std::process::exit(1);
    }
    String::from_utf8(proj.stdout).expect("Found invalid UTF-8")
}

fn get_targets(path: &str, exec: &str) -> Vec<String> {
    // @todo: support release
    let path = format!("{}/target/debug/deps", path);
    debug!("looking for dep {}", exec);

    let mut targets = Vec::new();
    for path in fs::read_dir(&path).unwrap() {
        let path = path.unwrap().path();
        if path.is_file() && path.extension().filter(|i| i.to_str().unwrap() == "d").is_none()
                && path.file_name().unwrap().to_str().unwrap().starts_with(&exec) {
            targets.push("--object".to_string());
            targets.push(path.to_str().unwrap().to_string());
        }
    }
    targets
}

fn get_executable_name(path: &str) -> (String, String) {
    // @todo: This definately isn't a general purpose solution, it's just good enough to start
    // @todo: also it's just horrifically ugly
    let path = Path::new(path.trim_end());
    debug!("parsing {:?}", path);
    let file = std::fs::read_to_string(&path).expect("failed to read Cargo.toml");
    let val: toml::Value = toml::from_str(file.as_str()).expect("failed to parse Cargo.lock");
    let prefix = val.get("package").expect("no package found in Cargo.toml")
        .get("name").expect("no name found in Cargo.toml").as_str()
        .unwrap().to_string();
    let path = path.parent().unwrap();
    (path.to_str().unwrap().to_string(), prefix.replace("-", "_") + "-")
}
