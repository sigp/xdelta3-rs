extern crate bindgen;
extern crate cc;

use std::env;
use std::path::PathBuf;

fn add_def(v: &mut Vec<(String, String)>, key: &str, val: &str) {
    v.push((key.to_owned(), val.to_owned()));
}

fn main() {
    let mut defines = Vec::new();

    let target_pointer_width = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();

    // Fail compilation on any non 64-bit platform.
    if target_pointer_width != "64" {
        panic!("Unsupported target pointer width: {}", target_pointer_width);
    }

    for i in &[
        "size_t",
        "unsigned int",
        "unsigned long",
        "unsigned long long",
    ] {
        let def_name = format!("SIZEOF_{}", i.to_uppercase().replace(" ", "_"));
        defines.push((def_name, "8".to_string()));
    }
    add_def(&mut defines, "SECONDARY_DJW", "1");
    add_def(&mut defines, "SECONDARY_FGK", "1");
    add_def(&mut defines, "EXTERNAL_COMPRESSION", "0");
    add_def(&mut defines, "XD3_USE_LARGEFILE64", "1");

    #[cfg(windows)]
    add_def(&mut defines, "XD3_WIN32", "1");
    add_def(&mut defines, "SHELL_TESTS", "0");

    #[cfg(feature = "lzma")]
    {
        add_def(&mut defines, "SECONDARY_LZMA", "1");
        pkg_config::Config::new().probe("liblzma").unwrap();
    }

    {
        let mut builder = cc::Build::new();
        builder.include("xdelta3/xdelta3");
        for (key, val) in &defines {
            builder.define(&key, Some(val.as_str()));
        }

        builder
            .file("xdelta3/xdelta3/xdelta3.c")
            .warnings(false)
            .compile("xdelta3");
    }

    {
        let mut builder = bindgen::Builder::default();

        // On macos, we need to explicitly include `errno.h`.
        // After Catalina, the headers are now located under the SDK. xcrun is an Xcode tool to
        // dynamically locate the path of the SDK. The header is located in the path:
        // `xcrun --show-sdk-path`/usr/include/errno.h
        // but we just include the entire directory with isysroot
        if cfg!(target_os = "macos") {
            if let Ok(sdk_path) = std::process::Command::new("xcrun")
                .arg("--show-sdk-path")
                .output()
            {
                let sdk_path = String::from_utf8(sdk_path.stdout).unwrap();
                builder = builder.clang_arg(format!("-isysroot{}", sdk_path.trim()));
            }
        }

        for (key, val) in &defines {
            builder = builder.clang_arg(format!("-D{}={}", key, val));
        }
        let bindings = builder
            .header("xdelta3/xdelta3/xdelta3.h")
            .parse_callbacks(Box::new(bindgen::CargoCallbacks))
            .allowlist_function("xd3_.*")
            .allowlist_type("xd3_.*")
            .rustified_enum("xd3_.*")
            .generate()
            .expect("Unable to generate bindings");

        // Write the bindings to the $OUT_DIR/bindings.rs file.
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }
}
