use std::io;
#[cfg(windows)] use winres::WindowsResource;

fn main() -> io::Result<()> {
    #[cfg(windows)] {
        WindowsResource::new()
            .set_icon("assets/icon.ico").set_language(0x0009).compile()?;
    }; Ok(()) }
