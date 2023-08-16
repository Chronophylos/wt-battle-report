pub mod battle_report;
pub mod de;
mod parser;

pub use battle_report::{
    Award, BattleReport, BattleResult, Event, ModificationResearch, Reward, Vehicle,
    VehicleResearch,
};
pub use de::{from_reader, from_slice, from_str};
