use std::{env, error::Error, fs::{read_to_string, File}, io::{BufReader, BufRead, Write}, path::PathBuf, process::{Command, Stdio}};

fn main() -> Result<(), Box<dyn Error>> {
  println!("cargo:rerun-if-changed=src/bindings.h");
  println!("cargo:rerun-if-changed=src/sdkconfig.h");

  let target_dir = PathBuf::from(env::var("CARGO_TARGET_DIR")?);

  let esp_path = PathBuf::from(env::var("ESP_PATH")?);
  let idf_path = PathBuf::from(env::var("IDF_PATH")?);

  let esp_sysroot = esp_path.join("xtensa-esp32-elf").join("xtensa-esp32-elf").join("sysroot");

  let component_includes =
    globwalk::GlobWalkerBuilder::from_patterns(
      &idf_path,
      &["components/*/include"],
    )
    .build()?
    .into_iter()
    .filter_map(Result::ok)
    .map(|d| d.into_path());

  let component_additional_includes = globwalk::GlobWalkerBuilder::from_patterns(
      &idf_path,
      &["components/*/component.mk"],
    )
    .build()?
    .into_iter()
    .filter_map(Result::ok)
    .flat_map(|makefile| {
      let path = makefile.into_path();

      let mut contents = read_to_string(&path).unwrap().replace("$(info ", "$(warn ");
      contents.push_str("\n$(info ${COMPONENT_ADD_INCLUDEDIRS})");

      let mut child = Command::new("make")
        .arg("-f")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env("IDF_TARGET", "esp32")
        .spawn()
        .unwrap();

      let mut stdin = child.stdin.take().unwrap();
      let mut stdout = child.stdout.take().unwrap();

      writeln!(stdin, "{}", contents).unwrap();

      BufReader::new(stdout).lines()
        .filter_map(Result::ok)
        .map(|s| s.trim_end().to_string())
        .filter(|s| !s.is_empty())
        .flat_map(|s| {
          let s = s.split(' ');
          let s = s.map(|s| s.to_string());
          s.collect::<Vec<_>>().into_iter()
        })
        .map(move |s| {
          let p = path.parent().unwrap().join(s).canonicalize().unwrap();
          assert!(p.is_dir());
          p
        })
    });

  let mut includes = component_includes.chain(component_additional_includes)
    .map(|include| format!("-I{}", include.display()))
    .collect::<Vec<_>>();

  includes.sort();
  includes.dedup();

  let sdkconfig = include_str!("src/sdkconfig.h");

  let bindings = bindgen::Builder::default()
    .use_core()
    .layout_tests(false)
    .ctypes_prefix("libc")
    .header("src/bindings.h")
    .clang_arg(format!("--sysroot={}", esp_sysroot.display()))
    .clang_arg("-Isrc")
    .clang_arg("-D__bindgen")
    .clang_args(&["-target", "xtensa"])
    .clang_args(&["-x", "c"])
    .clang_args(includes);

  eprintln!("{:?}", bindings.command_line_flags());

  let out_path = PathBuf::from(env::var("OUT_DIR")?);
  bindings.generate()
    .expect("Failed to generate bindings")
    .write_to_file(out_path.join("bindings.rs"))?;

  Ok(())
}
