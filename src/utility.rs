use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;

pub fn create_file_for_append<P: AsRef<Path>>(path: P) -> File {
    File::create(&path).unwrap();

    let f = OpenOptions::new()
        .write(true)
        .append(true)    
        .create(true)
        .open(path)
        .unwrap();

    f
}