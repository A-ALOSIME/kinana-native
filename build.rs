fn main() {
    slint_build::compile("ui/appwindow.slint").expect("Failed to compile Slint UI");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("app_icon.ico");
        let _ = res.compile();
    }
}
