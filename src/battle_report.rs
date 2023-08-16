//! Battle Report Value

use std::str::FromStr;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BattleReport {
    pub session_id: String,
    pub result: BattleResult,
    pub mission_name: String,

    pub events: Vec<Event>,

    pub awards: Vec<Award>,
    pub other_awards: Reward,

    pub vehicles: Vec<Vehicle>,

    pub activity: u8,

    pub damaged_vehicles: Vec<String>,
    pub repair_cost: u32,
    pub ammo_and_crew_cost: u32,
    pub vehicle_research: Vec<VehicleResearch>,
    pub modification_research: Vec<ModificationResearch>,

    pub earned_rewards: Reward,
    pub balance: Reward,
}

impl FromStr for BattleReport {
    type Err = crate::parser::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        crate::parser::parse(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BattleResult {
    Win,
    Loss,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub time: u32,
    pub kind: String,
    pub vehicle: String,
    pub enemy: Option<String>,
    pub reward: Reward,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Reward {
    pub silverlions: u32,
    pub research: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Vehicle {
    pub name: String,
    pub activity: u8,
    pub time_played: u32,
    pub reward: Reward,
}

#[derive(Debug, Clone, Serialize)]
pub struct VehicleResearch {
    pub name: String,
    pub research: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModificationResearch {
    pub vehicle: String,
    pub name: String,
    pub research: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Award {
    pub time: u32,
    pub name: String,
    pub reward: Reward,
}
