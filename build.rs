use winres;

fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico"); // Use a valid .ico file in this location
    res.compile().unwrap();
}
