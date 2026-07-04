fn main() {
    slint_build::compile("ui/app.slint").expect("slint build failed");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "PetPhrase");
        res.set("FileDescription", "PetPhrase 桌宠常用语工具");
        res.compile().expect("winresource failed");
    }
}
