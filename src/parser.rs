//! Battle Report Parser

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until, take_while, take_while1},
    character::complete::{
        char, digit1, line_ending, multispace0, newline, not_line_ending, space0, space1, u32, u8,
    },
    combinator::{map, opt, recognize, value},
    error::{convert_error, VerboseError},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
};

use crate::{battle_report::BattleReport, Award, BattleResult, Event, Reward, Vehicle};

type IResult<'a, O> = nom::IResult<&'a str, O, VerboseError<&'a str>>;

const INDENT: &str = "    "; // 4 spaces

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
    let (input, (result, mission_name)) = result_line(input)?;

    let (input, (events, awards, vehicles, other_awards)) = tuple((
        parse_events,
        award_table,
        vehicle_tables,
        parse_other_awards,
    ))(input)?;

    // TODO: earned
    // TODO: activity
    // TODO: damaged vehicles
    // TODO: automatic repair
    // TODO: automatic restock

    // TODO: vehicle research
    // TODO: modification research

    // TODO: used items (optional)

    // TODO: session
    // TODO: total

    Ok((
        input,
        BattleReport {
            session_id: "".to_string(),
            result,
            mission_name: mission_name.to_string(),
            events,
            awards,
            other_awards,
            vehicles,
            activity: 0,
            damaged_vehicles: vec![],
            repair_cost: 0,
            ammo_and_crew_cost: 0,
            vehicle_research: vec![],
            modification_research: vec![],
            earned_rewards: Default::default(),
            balance: Default::default(),
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
    time: u32,
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
///
/// ```
fn table(input: &str) -> IResult<Table<'_>> {
    let (input, (name, reward)) = table_header(input)?;
    let (input, rows) = many1(table_row)(input)?;
    let (input, _) = line_ending(input)?; // empty line

    Ok((input, Table { name, reward, rows }))
}

fn table_header(input: &str) -> IResult<(&str, Reward)> {
    let (input, name) = take_until(INDENT)(input)?; // consume name
    let (input, _) = pair(multispace0, digit1)(input)?; // consume number of rows
    let (input, _) = row_separator(input)?; // consume separator
    let (input, reward) = parse_reward(input)?; // consume reward
    let (input, _) = row_ending(input)?; // consume line ending

    Ok((input, (name, reward)))
}

fn row_separator(input: &str) -> IResult<()> {
    value((), pair(tag(INDENT), many0(space1)))(input)
}

fn row_ending(input: &str) -> IResult<()> {
    value((), pair(many0(space1), line_ending))(input)
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
    let (input, time) = preceded(tag(INDENT), terminated(timestamp, row_separator))(input)?;
    let (input, vehicle) = terminated(take_until(INDENT), row_separator)(input)?;
    let (input, enemy_vehicle) = terminated(take_until(INDENT), row_separator)(input)?;
    let (input, _) = opt(pair(tag("\u{d7}"), row_separator))(input)?;
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

fn timestamp(input: &str) -> IResult<u32> {
    map(separated_pair(u32, tag(":"), u32), |(hours, minutes)| {
        hours * 60 + minutes
    })(input)
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

fn parse_events(input: &str) -> IResult<Vec<Event>> {
    let (input, tables) = many0(table)(input)?;
    let events = tables
        .into_iter()
        .map(|table| {
            table
                .rows
                .into_iter()
                .map(move |row| {
                    let time = row.time;
                    let vehicle = row.vehicle.to_string();
                    let enemy = Some(row.enemy_vehicle.to_string());
                    let reward = row.reward;
                    let kind = table.name.to_string();

                    Event {
                        time,
                        kind,
                        vehicle,
                        enemy,
                        reward,
                    }
                })
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok((input, events))
}

fn award_table(input: &str) -> IResult<Vec<Award>> {
    let (input, rows) = preceded(table_header, many1(short_row))(input)?;
    let (input, _) = line_ending(input)?; // empty line

    let awards = rows
        .into_iter()
        .map(|(time, name, reward)| Award {
            time,
            name: name.to_string(),
            reward,
        })
        .collect();

    Ok((input, awards))
}

fn short_row(input: &str) -> IResult<(u32, &str, Reward)> {
    tuple((
        preceded(tag(INDENT), terminated(timestamp, row_separator)),
        terminated(take_until(INDENT), row_separator),
        terminated(parse_reward, row_ending),
    ))(input)
}

fn vehicle_tables(input: &str) -> IResult<Vec<Vehicle>> {
    // activity time
    let (input, activity_rows) = preceded(table_header, many1(short_row))(input)?;
    let (input, _) = line_ending(input)?; // empty line

    // time played
    let (input, _) = tuple((
        tag("Time Played"),
        pair(many1(space1), digit1),
        row_separator,
        parse_research_points,
        row_ending,
    ))(input)?;

    let (input, time_played_rows) = many1(tuple((
        preceded(tag(INDENT), terminated(take_until(INDENT), row_separator)), // name
        terminated(terminated(u8, tag("%")), row_separator),                  // activity
        terminated(timestamp, row_separator),                                 // time played
        terminated(parse_research_points, row_ending),                        // reward
    )))(input)?;

    let (input, _) = line_ending(input)?; // empty line

    let vehicles = activity_rows
        .into_iter()
        .zip(time_played_rows.into_iter())
        .map(
            |((_, name, reward), (_, activity, time_played, additional_rp))| Vehicle {
                name: name.to_string(),
                activity,
                time_played,
                reward: Reward {
                    silverlions: reward.silverlions,
                    research: reward.research + additional_rp,
                },
            },
        )
        .collect();

    Ok((input, vehicles))
}

fn parse_other_awards(input: &str) -> IResult<Reward> {
    delimited(
        pair(tag("Other awards"), row_separator),
        parse_reward,
        pair(row_ending, line_ending),
    )(input)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use nom::{error::convert_error, Finish};
    use rstest::*;

    use crate::*;

    fn run_parser<T, P>(input: &str, parser: P) -> (&str, T)
    where
        P: Fn(&str) -> super::IResult<T>,
    {
        match parser(input).finish() {
            Ok(result) => result,
            Err(err) => panic!("\n{}", convert_error(input, err)),
        }
    }

    #[test]
    fn parse_victory_as_result_name() {
        let input = "Victory";
        assert_eq!(super::battle_result(input), Ok(("", BattleResult::Win)))
    }

    #[test]
    fn parse_defeat_as_result_name() {
        let input = "Defeat";
        assert_eq!(super::battle_result(input), Ok(("", BattleResult::Loss)))
    }

    #[test]
    fn test_parse_result_line() {
        let input = "Victory in the [Domination] Poland (winter) mission!\r\n\n";
        let result = super::result_line(input).finish();
        match result {
            Ok((_, (result, map))) => {
                assert_eq!(result, BattleResult::Win);
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
        let result = super::parse(&input);
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
        7*60+13,
        "Concept 3",
        "M6A1",
        1010,
        77
    )]
    #[case(
        "    8:17     Concept 3          ISU-122()       1010 SL    80 RP\n",
        8*60+17,
        "Concept 3",
        "ISU-122()",
        1010,
        80
    )]
    #[case(
        "    8:31     Concept 3          Chi-To Late     1010 SL    73 RP\n",
        8*60+31,
        "Concept 3",
        "Chi-To Late",
        1010,
        73
    )]
    #[case(
        "    10:07    Wyvern S4          Pe-8            440 SL    11 + (Talismans)11 = 22 RP\n",
        10*60+7,
        "Wyvern S4",
        "Pe-8",
        440,
        22
    )]
    #[case(
        "    13:14    Sherman Firefly    Chi-Nu II       930 SL     61 RP\n",
        13*60+14,
        "Sherman Firefly",
        "Chi-Nu II",
        930,
        61
    )]
    #[case(
        "    13:43    Sherman Firefly    KV-85           930 SL     64 RP\n",
        13*60+43,
        "Sherman Firefly",
        "KV-85",
        930,
        64
    )]
    #[case("    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP\n", 3*60+45, "Concept 3", "M36 GMC()", 505, 40)]
    fn parse_row(
        #[case] input: &str,
        #[case] time: u32,
        #[case] vehice: &str,
        #[case] enemy_vehicle: &str,
        #[case] silverlions: u32,
        #[case] research: u32,
    ) {
        let (input, row) = super::table_row(input).unwrap();
        assert_eq!(input, "");
        assert_eq!(row.time, time);
        assert_eq!(row.vehicle, vehice);
        assert_eq!(row.enemy_vehicle, enemy_vehicle);
        assert_eq!(row.reward.silverlions, silverlions);
        assert_eq!(row.reward.research, research);
    }

    #[test]
    fn parse_other_awards() {
        let input = "Other awards                                       5295 SL     115 RP    \n\n";
        let (input, reward) = super::parse_other_awards(input).unwrap();
        assert_eq!(input, "");
        assert_eq!(reward.silverlions, 5295);
        assert_eq!(reward.research, 115);
    }

    #[test]
    fn parse_vehicle_tables() {
        let input = r#"Activity Time                                 3    3152 SL     160 RP    
    13:54    Concept 3          730 SL     68 RP                     
    13:54    Sherman Firefly    522 SL     56 RP                     
    13:54    Wyvern S4          1900 SL    18 + (Talismans)18 = 36 RP

Time Played                                   3               1057 RP    
    Concept 3          97%    8:21    680 RP                     
    Sherman Firefly    84%    2:51    185 RP                     
    Wyvern S4          67%    1:33    96 + (Talismans)96 = 192 RP

"#;
        let (input, vehicles) = run_parser(input, super::vehicle_tables);
        assert_eq!(input, "");
        assert_eq!(vehicles.len(), 3);
        assert_eq!(vehicles[0].name, "Concept 3");
        assert_eq!(vehicles[0].activity, 97);
        assert_eq!(vehicles[0].time_played, 8 * 60 + 21);
        assert_eq!(vehicles[0].reward.silverlions, 730);
        assert_eq!(vehicles[0].reward.research, 68 + 680);
    }
}
