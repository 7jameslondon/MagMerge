use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/logo.png");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let src = image::open("assets/logo.png")
        .expect("load assets/logo.png")
        .into_rgba8();
    let sizes = [256, 128, 64, 48, 32, 16];

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in sizes {
        let resized = image::imageops::resize(&src, size, size, image::imageops::Lanczos3);
        let icon = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir
            .add_entry(ico::IconDirEntry::encode(&icon).expect("encode icon"));
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let ico_path = out_dir.join("app.ico");
    let mut file = fs::File::create(&ico_path).expect("create ico");
    icon_dir.write(&mut file).expect("write ico");

    let rc_path = out_dir.join("app.rc");
    let ico_path_str = ico_path.to_string_lossy().replace('\\', "/");
    fs::write(&rc_path, format!("1 ICON \"{}\"", ico_path_str)).expect("write rc");

    embed_resource::compile(&rc_path, embed_resource::NONE);
}
