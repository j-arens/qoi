//! Utility commands for building and developing the example QOI WASM site.
//! 
//! # Commands
//! 
//! ```sh
//! cargo run -- build
//! ```
//! 
//! Compiles the `qoi_wasm` crate and copies the built WASM file into the
//! `site` directory.
//! 
//! ```sh
//! cargo run -- dev
//! ```
//! 
//! Does the same thing as the `build` command and also starts a local
//! development server that serves content out of the `site` directory.
//! Modifying a source file in the `qoi_wasm` crate or any file in the `site`
//! directory will automatically refresh the page.
//! 

use std::env;
use std::fs;
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::path::PathBuf;
use std::process::Command;

use hotwatch::{Event, Hotwatch};

#[derive(Clone)]
struct Paths {
  site_dir: PathBuf,
  wasm_build: PathBuf,
  wasm_dir: PathBuf,
  wasm_src: PathBuf,
  wasm_target: PathBuf,
}

impl Paths {
  fn try_new() -> Result<Self, std::io::Error> {
    let root_dir = fs::canonicalize("../")?;

    Ok(Self {
      site_dir: root_dir.clone().join("site"),
      wasm_dir: root_dir.clone().join("qoi_wasm"),
      wasm_src: root_dir.clone().join("qoi_wasm/src/lib.rs"),
      wasm_build: root_dir
        .clone()
        .join("target/wasm32-unknown-unknown/release-wasm/qoi_wasm.wasm"),
      wasm_target: root_dir.clone().join("site/qoi.wasm"),
    })
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let paths = Paths::try_new()?;

  let command = env::args()
    .nth(1)
    .expect("Expected to be called with a `build` or `dev` argument, but received none instead");

  match command.as_str() {
    "build" => {
      build_and_copy(&paths);
    }
    "dev" => {
      build_and_copy(&paths);

      let mut watcher = Hotwatch::new()?;
      let p = paths.clone();

      watcher.watch(&paths.wasm_src, move |e: Event| {
        if let Event::Write(_) = e {
          build_and_copy(&p);
        }
      })?;

      start_dev_server(&paths);
    }
    other => {
      panic!(
        "Expected to be called with a `build` or `dev` argument, but received `{}` instead",
        other
      );
    }
  };

  Ok(())
}

fn build_and_copy(paths: &Paths) {
  println!("==> Building crate {}", paths.wasm_dir.display());

  let build_args = [
    "build",
    // Use the custom `release-wasm` build profile defined in the workspace's
    // root `Cargo.toml`.
    "--profile",
    "release-wasm",
    // Target WASM architecture.
    "--target",
    "wasm32-unknown-unknown",
  ];

  let build_status = Command::new("cargo")
    .current_dir(&paths.wasm_dir)
    .args(build_args)
    .status()
    .expect("Spawning build command failed");

  if !build_status.success() {
    panic!(
      "Build command exited with status code `{:?}`",
      build_status.code()
    );
  }

  println!(
    "==> Copying {} to {}",
    paths.wasm_build.display(),
    paths.wasm_target.display(),
  );

  fs::copy(&paths.wasm_build, &paths.wasm_target).expect("Copying failed");
}

fn start_dev_server(paths: &Paths) {
  match get_open_port() {
    Some(port) => {
      println!("==> Starting dev server at http://localhost:{}", port);
      println!("==> Press ctrl+c to stop");

      devserver_lib::run(
        "localhost",
        port.into(),
        paths.site_dir.to_str().unwrap(),
        true,
        "",
      );
    }
    None => {
      panic!("Failed to find an open port between 7000 and 8000");
    }
  }
}

fn get_open_port() -> Option<u16> {
  let mut port = 7000;

  while port < 8001 {
    let socket_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);

    if TcpListener::bind(socket_addr).is_ok() {
      return Some(port);
    }

    port += 1;
  }

  None
}
