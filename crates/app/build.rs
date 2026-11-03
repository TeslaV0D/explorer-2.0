fn main() {
    slint_build::compile("ui/main.slint").expect("slint compile failed");
    println!("cargo:rerun-if-changed=res/explorer2.rc");
    println!("cargo:rerun-if-changed=res/explorer2.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("res/explorer2.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embedding Windows manifest failed");
    }
}
