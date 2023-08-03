use std::io;

use crate::{battle_report::BattleReport, parser};

pub fn from_str(input: &str) -> Result<BattleReport, parser::Error> {
    parser::parse(input.as_bytes())
}

pub fn from_slice(input: &[u8]) -> Result<BattleReport, parser::Error> {
    parser::parse(input)
}

pub fn from_reader<R: io::Read>(input: R) -> Result<BattleReport, parser::Error> {
    let bytes = input.bytes().collect::<io::Result<Vec<_>>>().unwrap();

    parser::parse(&bytes)
}
