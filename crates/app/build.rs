fn main() {
    // D-029: `@tr` texts are English, translations from `lang/<code>/LC_MESSAGES/explorer2.po`
    // are compiled into the exe; no translation contexts (one text = one meaning,
    // shared with the Rust texts of `explorer-i18n`).
    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations("../../lang")
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/main.slint", config).expect("slint compile failed");
    println!("cargo:rerun-if-changed=../../lang");
    println!("cargo:rerun-if-changed=res/explorer2.rc");
    println!("cargo:rerun-if-changed=res/explorer2.manifest");
    println!("cargo:rerun-if-changed=res/icon/explorer2.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("res/explorer2.rc", embed_resource::NONE)
            .manifest_required()
            .expect("embedding Windows manifest failed");
    }
}
