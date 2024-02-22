use std::io::Write;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug,Deserialize)]
struct Config {
   set: String,
   outputfile: String,
}

fn read_original(c: char) -> &'static [u8] {
    const FONT_DATA: &[u8; 2097152] = include_bytes!("../../assets/font_data.bin");
    &FONT_DATA[(c as usize * ' ' as usize) as usize..(c as usize * ' ' as usize + 32) as usize]
}

fn main(){
    let config: Config = toml::from_str(include_str!("config.toml")).unwrap();
    extract_charset(&config.set, Path::new(&config.outputfile));
}

fn extract_charset<P: AsRef<Path> + std::fmt::Debug>(charset: &str, output: P){
    let mut out_file = std::fs::File::create(&output).unwrap();

    let bytes_wrote : usize = charset
        .chars()
        .map(|char| out_file.write(read_original(char)).unwrap())
        .sum();

    println!("Wrote {} bytes at {}", bytes_wrote, output.as_ref().to_string_lossy());
}