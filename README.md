# Mail Extractor

A Rust library to extract files from MIME type files and returns you a hashmap which will contain filename and it corresponding file content as bytes.

### How to use

``` Rust
use std::collections::HashMap;
use mail_extractor;
fn get_files(file_stream: Vec<u8>) -> HashMap<String, Vec<u8>> {
    et extracted_file: HashMap<String, Vec<u8>> = mail_extractor::rewrite(file_stream);
    extracted_file
}
```

### Add the dependency in **Cargo.toml**

``` Rust
[dependencies]
mail_extractor = "0.1.2"
```