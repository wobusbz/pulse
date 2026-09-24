///! 将应用程序图标嵌入到可执行的二进制
///! Windows可执行文件，因此“Pulse.exe”到处都显示自己的图标。
///!
///! `embed_resource `需要一个支持Windows的资源编译器，所以图标是
///! 仅在构建本身在Windows上运行时嵌入；交叉检查来自
///! 另一个主机只是在没有它的情况下构建。

fn main() {
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=installer/pulse.ico");

    if !cfg!(target_os = "windows") {
        return;
    }

    if let Err(error) = embed_resource::compile("app.rc", embed_resource::NONE).manifest_optional()
    {
        println!("cargo:warning=failed to embed the application icon: {error:?}");
    }
}
