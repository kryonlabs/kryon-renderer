use kryon_core::load_krb_file;

fn main() {
    match load_krb_file("../kryon-compiler/test_transform_renderer.krb") {
        Ok(krb_file) => {
            println!("Successfully loaded KRB file with transform support!");
            println!("Transform count: {}", krb_file.header.transform_count);
            println!("Transforms: {:#?}", krb_file.transforms);
        }
        Err(e) => {
            eprintln!("Error loading KRB file: {}", e);
        }
    }
}