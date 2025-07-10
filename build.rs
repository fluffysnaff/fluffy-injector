use winres;

fn main() {
    if std::env::var("TARGET").unwrap().contains("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().expect("Failed to compile Windows resource file. Make sure 'assets/icon.ico' exists and is a valid .ico file.");
    }
}