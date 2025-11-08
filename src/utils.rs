use std::env;
use std::io::{self, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub fn find_executable(file_path_str: &str) -> Option<PathBuf> {
    let path_var = env::var("PATH").unwrap_or_default();
    for path in path_var.split(':') {
        let file_path_str = format!("{}/{}", path, file_path_str);
        let file_path = PathBuf::from(file_path_str);
        if let Ok(metadata) = file_path.metadata() {
            let permissions = metadata.permissions();
            let is_executable = permissions.mode() & 0o111 != 0;
            if metadata.is_file() && is_executable {
                return Some(file_path);
            }
        }
    }
    None
}

pub fn write_line(writer: &mut dyn Write, content: &str) -> io::Result<()> {
    writer.write_all(content.as_bytes())?;
    writer.write_all(b"\n")
}
