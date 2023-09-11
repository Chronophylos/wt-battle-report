//! Battle Report Deserialization

use std::io;

use crate::{battle_report::BattleReport, parser};

pub use parser::Error;

pub fn from_str(input: &str) -> Result<BattleReport, parser::Error> {
    parser::parse(input)
}

pub fn from_slice(input: &[u8]) -> Result<BattleReport, parser::Error> {
    let buffer = String::from_utf8_lossy(input);
    parser::parse(&buffer)
}

pub fn from_reader<R: io::Read>(mut input: R) -> Result<BattleReport, parser::Error> {
    let mut buffer = String::new();
    input.read_to_string(&mut buffer).unwrap();

    parser::parse(&buffer)
}
