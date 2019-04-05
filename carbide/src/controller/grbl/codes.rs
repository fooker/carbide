#![allow(dead_code)]

pub struct Setting {
    pub name: &'static str,
    pub unit: &'static str,
    pub desc: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/grbl_codes.rs"));
