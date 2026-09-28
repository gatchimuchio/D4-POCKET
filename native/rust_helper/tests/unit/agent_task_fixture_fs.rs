use std::path::Path;

pub(crate) fn write_marker(path: &Path, content: &str) -> std::io::Result<()> {
    std::fs::write(path, content)
}

pub(crate) fn read_marker(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}
