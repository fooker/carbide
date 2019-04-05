use std::env;
use std::path::Path;
use std::fs::File;
use std::str::FromStr;
use std::io::Write;

use csv;
use phf_codegen;

fn generate_grbl_alarm_codes(root: &Path, f: &mut impl Write) {
    let mut csv = csv::Reader::from_reader(File::open(root.join("alarm_codes_en_US.csv")).unwrap());

    let mut builder = phf_codegen::Map::new();

    for code in csv.records() {
        let code = code.unwrap();
        builder.entry(u8::from_str(&code[0]).unwrap(),
                      &format!("\"{}\"", &code[2]));
    }

    write!(f, "pub const ALARM_CODES: phf::Map<u8, &'static str> = ").unwrap();
    builder.build(f).unwrap();
    write!(f, ";\n\n").unwrap();
}

fn generate_grbl_build_option_codes(root: &Path, f: &mut impl Write) {
    let mut csv = csv::Reader::from_reader(File::open(root.join("build_option_codes_en_US.csv")).unwrap());

    let mut builder = phf_codegen::Map::new();

    for code in csv.records() {
        let code = code.unwrap();
        builder.entry(char::from_str(&code[0]).unwrap(),
                      &format!("\"{}\"", &code[1]));
    }

    write!(f, "pub const BUILD_OPTION_CODES: phf::Map<char, &'static str> = ").unwrap();
    builder.build(f).unwrap();
    write!(f, ";\n\n").unwrap();
}

fn generate_grbl_error_codes(root: &Path, f: &mut impl Write) {
    let mut csv = csv::Reader::from_reader(File::open(root.join("error_codes_en_US.csv")).unwrap());

    let mut builder = phf_codegen::Map::new();

    for code in csv.records() {
        let code = code.unwrap();
        builder.entry(u8::from_str(&code[0]).unwrap(),
                      &format!("\"{}\"", &code[2]));
    }

    write!(f, "pub const ERROR_CODES: phf::Map<u8, &'static str> = ").unwrap();
    builder.build(f).unwrap();
    write!(f, ";\n\n").unwrap();
}

fn generate_grbl_setting_codes(root: &Path, f: &mut impl Write) {
    let mut csv = csv::Reader::from_reader(File::open(root.join("setting_codes_en_US.csv")).unwrap());

    let mut builder = phf_codegen::Map::new();

    for code in csv.records() {
        let code = code.unwrap();
        builder.entry(u8::from_str(&code[0]).unwrap(),
                      &format!("Setting {{ name: \"{}\", unit: \"{}\", desc: \"{}\" }}", &code[1], &code[2], &code[3]));

        writeln!(f, "pub const SETTING_CODE_{}: u8 = {};",
                 &code[1]
                     .replace(' ', "_")
                     .replace('-', "_")
                     .to_uppercase(),
                 &code[0],
        ).unwrap();
    }

    write!(f, "pub const SETTING_CODES: phf::Map<u8, Setting> = ").unwrap();
    builder.build(f).unwrap();
    write!(f, ";\n\n").unwrap();
}

fn generate_grbl_codes() {
    let root = Path::new("extern/grbl/doc/csv");

    let mut f = File::create(&Path::new(&env::var("OUT_DIR").unwrap()).join("grbl_codes.rs")).unwrap();

    generate_grbl_alarm_codes(&root, &mut f);
    generate_grbl_build_option_codes(&root, &mut f);
    generate_grbl_error_codes(&root, &mut f);
    generate_grbl_setting_codes(&root, &mut f);
}

fn main() {
    generate_grbl_codes();
}

