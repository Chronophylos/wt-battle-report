//! Battle Report Parser

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until, take_while, take_while1},
    character::complete::{
        char, digit1, line_ending, multispace0, newline, not_line_ending, space0, u32,
    },
    combinator::{map, map_res, opt, recognize, value},
    error::{context, convert_error, dbg_dmp, VerboseError},
    multi::many1,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
};

use crate::{battle_report::BattleReport, BattleResult, Reward};

type IResult<'a, O> = nom::IResult<&'a str, O, VerboseError<&'a str>>;

#[derive(Debug, thiserror::Error)]
#[error("Error parsing battle report: {message}")]
pub struct Error {
    message: String,
}

pub fn parse(input: &str) -> Result<BattleReport, Error> {
    battle_report(input)
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

fn battle_report(input: &str) -> IResult<BattleReport> {
    let (input, (battle_result, mission_name)) = result_line(input)?;

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
fn result_line(input: &str) -> IResult<(BattleResult, &str)> {
    let (input, result) = battle_result(input)?;
    let (input, _) = tag(" in the ")(input)?;
    let (input, mission) = take_until(" mission!")(input)?;
    let (input, _) = tag(" mission!")(input)?;
    let (input, _) = line_ending(input)?;
    let (input, _) = line_ending(input)?;

    Ok((input, (result, mission)))
}

fn battle_result(input: &str) -> IResult<BattleResult> {
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
fn table(input: &str) -> IResult<Table<'_>> {
    // Header
    let (input, name) = take_until("     ")(input)?; // consume name
    let (input, _) = pair(multispace0, digit1)(input)?; // consume number of rows
    let (input, _) = row_separator(input)?; // consume separator
    let (input, reward) = parse_reward(input)?; // consume reward

    let (input, _) = row_ending(input)?; // consume line ending

    // Rows
    let (input, rows) = many1(table_row)(input)?;

    Ok((
        input,
        Table {
            name,
            reward,
            rows: vec![],
        },
    ))
}

fn row_separator(input: &str) -> IResult<()> {
    value((), pair(tag("    "), multispace0))(input)
}

fn row_ending(input: &str) -> IResult<()> {
    value((), pair(multispace0, line_ending))(input)
}

/// parse a table row
///
/// # Examples
/// ```text
///     7:13     Concept 3          M6A1            1010 SL    77 RP
///     8:17     Concept 3          ISU-122()       1010 SL    80 RP
///     8:31     Concept 3          Chi-To Late     1010 SL    73 RP
///     10:07    Wyvern S4          Pe-8            440 SL    11 + (Talismans)11 = 22 RP
///     13:14    Sherman Firefly    Chi-Nu II       930 SL     61 RP
///     13:43    Sherman Firefly    KV-85           930 SL     64 RP
///     3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP
/// ```
fn table_row(input: &str) -> IResult<Row<'_>> {
    // Time
    let (input, time) = terminated(timestamp, row_separator)(input)?;

    // Vehicle
    let (input, vehicle) = terminated(take_until("     "), row_separator)(input)?;

    // Enemy vehicle
    let (input, enemy_vehicle) = terminated(take_until("     "), row_separator)(input)?;

    // Optional "x"
    let (input, _) = opt(pair(tag("\u{d7}"), row_separator))(input)?;

    // Reward
    let (input, reward) = terminated(parse_reward, row_ending)(input)?;

    Ok((
        input,
        Row {
            time,
            vehicle,
            enemy_vehicle,
            reward,
        },
    ))
}

fn timestamp(input: &str) -> IResult<&str> {
    preceded(tag("    "), take_while(|c| c != ' '))(input)
}

/// parse a reward
///
/// # Examples
/// ```text
/// 5820 SL     413 RP
/// ```
/// ```text
/// 1000 SL
/// ```
/// ```text
/// 505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP
/// ```
fn parse_reward(input: &str) -> IResult<Reward> {
    let (input, (silverlions, research)) = alt((
        separated_pair(parse_silverlions, row_separator, parse_research_points),
        map(parse_silverlions, |sl| (sl, 0)),
    ))(input)?;

    Ok((
        input,
        Reward {
            silverlions,
            research,
        },
    ))
}

fn parse_silverlions(input: &str) -> IResult<u32> {
    let (input, silverlions) = u32(input)?;
    let (input, _) = tag(" SL")(input)?;

    Ok((input, silverlions))
}

fn parse_research_points(input: &str) -> IResult<u32> {
    alt((parse_research_points_simple, parse_research_points_complex))(input)
}

fn parse_research_points_simple(input: &str) -> IResult<u32> {
    let (input, (rp, _)) = pair(u32, tag(" RP"))(input)?;
    Ok((input, rp))
}

fn parse_research_points_complex(input: &str) -> IResult<u32> {
    let (input, _) = take_until("= ")(input)?;
    preceded(tag("= "), parse_research_points_simple)(input)
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
        assert_eq!(battle_result(input), Ok(("", crate::BattleResult::Win)))
    }

    #[test]
    fn parse_defeat_as_result_name() {
        let input = "Defeat";
        assert_eq!(battle_result(input), Ok(("", crate::BattleResult::Loss)))
    }

    #[test]
    fn test_parse_result_line() {
        let input = "Victory in the [Domination] Poland (winter) mission!\r\n\n";
        let result = result_line(input).finish();
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

    #[rstest]
    #[case("100 RP", 100)]
    #[case("3242 RP", 3242)]
    fn parse_research_points_simple(#[case] input: &str, #[case] expected: u32) {
        let (_, value) = super::parse_research_points_simple(input).unwrap();
        assert_eq!(value, expected)
    }

    #[rstest]
    #[case("10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", 40)]
    #[case("96 + (Talismans)96 = 192 RP", 192)]
    #[case("113 + (Talismans)113 = 226 RP", 226)]
    fn parse_research_points_complex(#[case] input: &str, #[case] expected: u32) {
        let (_, value) = super::parse_research_points_complex(input).unwrap();
        assert_eq!(value, expected)
    }

    #[rstest]
    #[case("10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", 40)]
    #[case("100 RP", 100)]
    #[case("96 + (Talismans)96 = 192 RP", 192)]
    #[case("113 + (Talismans)113 = 226 RP", 226)]
    fn parse_research_points(#[case] input: &str, #[case] expected: u32) {
        let (_, value) = super::parse_research_points(input).unwrap();
        assert_eq!(value, expected)
    }

    #[rstest]
    #[case("5820 SL     413 RP", 5820, 413)]
    #[case("1000 SL", 1000, 0)]
    #[case("505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", 505, 40)]
    fn parse_reward(#[case] input: &str, #[case] silverlions: u32, #[case] research: u32) {
        let (_, reward) = super::parse_reward(input).unwrap();
        assert_eq!(reward.silverlions, silverlions);
        assert_eq!(reward.research, research);
    }

    #[rstest]
    #[case(
        "    7:13     Concept 3          M6A1            1010 SL    77 RP\n",
        "7:13",
        "Concept 3",
        "M6A1",
        1010,
        77
    )]
    #[case(
        "    8:17     Concept 3          ISU-122()       1010 SL    80 RP\n",
        "8:17",
        "Concept 3",
        "ISU-122()",
        1010,
        80
    )]
    #[case(
        "    8:31     Concept 3          Chi-To Late     1010 SL    73 RP\n",
        "8:31",
        "Concept 3",
        "Chi-To Late",
        1010,
        73
    )]
    #[case(
        "    10:07    Wyvern S4          Pe-8            440 SL    11 + (Talismans)11 = 22 RP\n",
        "10:07",
        "Wyvern S4",
        "Pe-8",
        440,
        22
    )]
    #[case(
        "    13:14    Sherman Firefly    Chi-Nu II       930 SL     61 RP\n",
        "13:14",
        "Sherman Firefly",
        "Chi-Nu II",
        930,
        61
    )]
    #[case(
        "    13:43    Sherman Firefly    KV-85           930 SL     64 RP\n",
        "13:43",
        "Sherman Firefly",
        "KV-85",
        930,
        64
    )]
    #[case("    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", "3:45", "Concept 3", "M36 GMC()\n", 505, 40)]
    fn parse_row(
        #[case] input: &str,
        #[case] time: &str,
        #[case] vehice: &str,
        #[case] enemy_vehicle: &str,
        #[case] silverlions: u32,
        #[case] research: u32,
    ) {
        let (_, row) = super::table_row(input).unwrap();
        assert_eq!(row.time, time);
        assert_eq!(row.vehicle, vehice);
        assert_eq!(row.enemy_vehicle, enemy_vehicle);
        assert_eq!(row.reward.silverlions, silverlions);
        assert_eq!(row.reward.research, research);
    }
}
