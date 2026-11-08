//! The setup carries the release build of the app: `EXPLORER2_SETUP_PAYLOAD`
//! = folder with `explorer2.exe` + `explorer2-recover.exe` (built before).
//! Without it (normal workspace builds, clippy, tests) the setup compiles
//! empty and only reports that it was built without content.
use std::path::Path;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(payload)");
    println!("cargo:rerun-if-env-changed=EXPLORER2_SETUP_PAYLOAD");
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let (app, recover) = if windows {
        ("explorer2.exe", "explorer2-recover.exe")
    } else {
        ("explorer2", "explorer2-recover")
    };
    if let Ok(dir) = std::env::var("EXPLORER2_SETUP_PAYLOAD") {
        let dir = Path::new(&dir);
        let (a, r) = (dir.join(app), dir.join(recover));
        for f in [&a, &r] {
            assert!(
                f.is_file(),
                "EXPLORER2_SETUP_PAYLOAD: {} fehlt",
                f.display()
            );
            println!("cargo:rerun-if-changed={}", f.display());
        }
        println!("cargo:rustc-cfg=payload");
        println!("cargo:rustc-env=EXPLORER2_PAYLOAD_APP={}", a.display());
        println!("cargo:rustc-env=EXPLORER2_PAYLOAD_RECOVER={}", r.display());
    }
    println!("cargo:rerun-if-changed=res/setup.rc");
    println!("cargo:rerun-if-changed=res/setup.manifest");
    if windows {
        embed_resource::compile("res/setup.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embedding the setup manifest failed");
    }
}
