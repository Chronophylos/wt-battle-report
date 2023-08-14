//! Battle Report Parser

use std::{backtrace::Backtrace, fmt::Display};

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, line_ending, newline, not_line_ending},
    combinator::{map, recognize},
    error::{context, convert_error, dbg_dmp, VerboseError},
    sequence::{delimited, pair, preceded, terminated, tuple},
};

use crate::{battle_report::BattleReport, BattleResult, Reward};

type IResult<'a, O> = nom::IResult<&'a str, O, VerboseError<&'a str>>;

#[derive(Debug, thiserror::Error)]
#[error("Error parsing battle report: {message}")]
pub struct Error {
    message: String,
}

pub fn parse(input: &str) -> Result<BattleReport, Error> {
    parse_battle_report(input)
        .map(|(_, report)| report)
        .map_err(|err| {
            let message = if let nom::Err::Error(err) = err {
                convert_error(input, err)
            } else {
                "Unknown error".to_string()
            };
            Error { message }
        })
}

fn parse_battle_report(input: &str) -> IResult<BattleReport> {
    let (input, (battle_result, mission_name)) = parse_result_line(input)?;

    Ok((
        input,
        BattleReport {
            session_id: "".to_string(),
            result: battle_result,
            map: mission_name.to_string(),
            events: vec![],
            awards: vec![],
            other_awards: Default::default(),
            vehicles: vec![],
            activity: 0,
            damaged_vehicles: vec![],
            repair_cost: 0,
            ammo_and_crew_cost: 0,
            vehicle_research: vec![],
            modification_research: vec![],
            earned_rewards: Default::default(),
            total_rewards: Default::default(),
        },
    ))
}

/// parse the first line in a battle report
fn parse_result_line(input: &str) -> IResult<(BattleResult, &str)> {
    let (input, result) = parse_battle_result(input)?;
    let (input, _) = tag(" in the ")(input)?;
    let (input, mission) = take_until(" mission!")(input)?;
    let (input, _) = tag(" mission!")(input)?;
    let (input, _) = line_ending(input)?;
    let (input, _) = line_ending(input)?;

    Ok((input, (result, mission)))
}

fn parse_battle_result(input: &str) -> IResult<BattleResult> {
    alt((
        map(tag("Victory"), |_| BattleResult::Win),
        map(tag("Defeat"), |_| BattleResult::Loss),
    ))(input)
}

struct Table<'a> {
    name: &'a str,
    reward: Reward,
    rows: Vec<Row<'a>>,
}

struct Row<'a> {
    time: &'a str,
    vehicle: &'a str,
    enemy_vehicle: &'a str,
    reward: Reward,
}

/// parse a table
///
/// # Example
/// ```text
/// Destruction of ground vehicles and fleets     6    5820 SL     413 RP    
///     7:13     Concept 3          M6A1            1010 SL    77 RP
///     8:17     Concept 3          ISU-122()       1010 SL    80 RP
///     8:31     Concept 3          Chi-To Late     1010 SL    73 RP
///     11:47    Sherman Firefly    T-34 (1942)     930 SL     58 RP
///     13:14    Sherman Firefly    Chi-Nu II       930 SL     61 RP
///     13:43    Sherman Firefly    KV-85           930 SL     64 RP
/// ```
fn parse_table(input: &str) -> IResult<Vec<Vec<&str>>> {
    todo!()
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use nom::{error::convert_error, Finish};
    use rstest::*;

    use super::*;

    #[test]
    fn parse_victory_as_result_name() {
        let input = "Victory";
        assert_eq!(
            parse_battle_result(input),
            Ok(("", crate::BattleResult::Win))
        )
    }

    #[test]
    fn parse_defeat_as_result_name() {
        let input = "Defeat";
        assert_eq!(
            parse_battle_result(input),
            Ok(("", crate::BattleResult::Loss))
        )
    }

    #[test]
    fn test_parse_result_line() {
        let input = "Victory in the [Domination] Poland (winter) mission!\r\n\n";
        let result = parse_result_line(input).finish();
        match result {
            Ok((_, (result, map))) => {
                assert_eq!(result, crate::BattleResult::Win);
                assert_eq!(map, "[Domination] Poland (winter)")
            }
            Err(err) => {
                panic!("Error parsing result line:\n{}", convert_error(input, err))
            }
        }
    }

    #[rstest]
    fn test_real_data(#[files("./data/*.report")] path: PathBuf) {
        let input = std::fs::read_to_string(&path).unwrap();
        let result = parse(&input);
        if let Err(err) = result {
            panic!("\n{err}")
        }
    }
}
