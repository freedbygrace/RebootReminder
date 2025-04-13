use std::env;
use std::path::Path;
extern crate winres;

fn main() {
    // Only run the resource embedding on Windows
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        // Embed the application icon
        let icon_path = Path::new("resources/icons/icon.ico");
        if icon_path.exists() {
            // Use winres for icon and version information
            let mut res = winres::WindowsResource::new();
            res.set_icon("resources/icons/icon.ico");
            res.set_language(0x0409); // English (United States)

            // Set version information
            res.set("FileVersion", env!("CARGO_PKG_VERSION"));
            res.set("ProductVersion", env!("CARGO_PKG_VERSION"));
            res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
            res.set("ProductName", env!("CARGO_PKG_NAME"));
            res.set("OriginalFilename", "reboot_reminder.exe");
            res.set("LegalCopyright", "Copyright © 2023");

            // Compile the resource
            if let Err(e) = res.compile() {
                println!("cargo:warning=Failed to compile resources: {}", e);
            }
        } else {
            println!("cargo:warning=Icon file not found: {:?}", icon_path);
        }
    }
}
