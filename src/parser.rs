//! Battle Report Parser

use std::fmt::Debug;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while},
    character::complete::{alpha1, digit1, hex_digit1, line_ending, space1, u32, u8},
    combinator::{map, map_parser, opt, success, value},
    error::{context, convert_error, VerboseError},
    multi::{many0, many1, many_m_n, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Parser,
};

use crate::{
    battle_report::BattleReport, Award, BattleResult, Event, ModificationResearch, Reward, Vehicle,
    VehicleResearch,
};

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
    let (input, (result, mission_name)) = context("first line", result_line)(input)?;

    let (
        input,
        (
            events,
            awards,
            vehicles,
            reward_for_winning,
            other_awards,
            earned_rewards,
            activity,
            damaged_vehicles,
            automatic_repair,
            automatic_purchases,
            _,
            vehicle_research,
            modification_research,
            _,
            session_id,
            (balance, _raw_research),
        ),
    ) = tuple((
        context("events", parse_events),
        context("awards", award_table),
        context("activity and time played", vehicle_tables),
        context("reward for winning", opt(parse_reward_for_winning)),
        context("other awards", parse_other_awards),
        context("earned", parse_earned),
        context("activity", parse_activity),
        context("damaged vehicles", parse_damaged_vehicles),
        context("automatic repair", parse_automatic_repair),
        context("automatic purchase", parse_automatic_purchase),
        line_ending,
        context("researched vehicles", opt(parse_researched_units)),
        context(
            "researched modifications",
            opt(parse_researched_modifications),
        ),
        context("used items", opt(parse_used_items)),
        context("session id", parse_session_id),
        context("total", parse_total),
    ))(input)?;

    Ok((
        input,
        BattleReport {
            session_id,
            result,
            mission_name: mission_name.to_string(),
            events,
            awards,
            reward_for_winning,
            other_awards,
            vehicles,
            activity,
            damaged_vehicles,
            automatic_repair,
            automatic_purchases,
            vehicle_research: vehicle_research.unwrap_or_default(),
            modification_research: modification_research.unwrap_or_default(),
            earned_rewards,
            balance,
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

struct Table {
    name: String,
    rows: Vec<Row>,
}

#[derive(Debug)]
struct Row {
    time: u32,
    vehicle: String,
    enemy_vehicle: String,
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
fn table(input: &str) -> IResult<Table> {
    let (input, (name, count, _)) = context("table header", table_header)(input)?;

    let (input, rows) = context(
        "table rows",
        many_m_n(count as usize, count as usize, table_row),
    )(input)?;
    let (input, _) = line_ending(input)?; // empty line

    Ok((
        input,
        Table {
            name: name.to_string(),
            rows,
        },
    ))
}

fn table_header(input: &str) -> IResult<(String, u32, Reward)> {
    //let (input, (name, _, reward)) = tuple((
    //    context("table name", terminated(take_until(INDENT), row_separator)),
    //    context("row count", terminated(digit1, row_separator)),
    //    context("total reward", terminated(parse_reward, row_ending)),
    //))(input)?;

    let (input, name) =
        context("table name", terminated(take_until(INDENT), row_separator))(input)?;
    let (input, count) = context("row count", terminated(u32, row_separator))(input)?;
    let (input, reward) = context("total reward", terminated(parse_reward, row_ending))(input)?;

    Ok((input, (name.to_string(), count, reward)))
}

fn row_separator(input: &str) -> IResult<()> {
    context("row separator", value((), pair(tag(INDENT), many0(space1))))(input)
}

fn row_ending(input: &str) -> IResult<()> {
    context("row ending", value((), pair(many0(space1), line_ending)))(input)
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
fn table_row(input: &str) -> IResult<Row> {
    let (input, (time, vehicle, enemy_vehicle, _, reward)) = tuple((
        context(
            "time column",
            preceded(tag(INDENT), terminated(timestamp, row_separator)),
        ),
        context(
            "vehicle column",
            terminated(take_until(INDENT), row_separator),
        ),
        context(
            "enemy vehicle column",
            terminated(take_until(INDENT), row_separator),
        ),
        context("optional x", opt(pair(tag("\u{d7}"), row_separator))),
        context("reward column", terminated(parse_reward, row_ending)),
    ))(input)?;

    Ok((
        input,
        Row {
            time,
            vehicle: vehicle.to_string(),
            enemy_vehicle: enemy_vehicle.to_string(),
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
        pair(
            parse_silverlions,
            map(opt(preceded(row_separator, parse_research_points)), |rp| {
                rp.unwrap_or_default()
            }),
        ),
        pair(success(0), parse_research_points),
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
    context(
        "silverlions",
        alt((parse_silverlions_simple, parse_silverlions_complex)),
    )(input)
}

fn parse_silverlions_simple(input: &str) -> IResult<u32> {
    context("silverlions simple", terminated(u32, tag(" SL")))(input)
}

fn parse_silverlions_complex(input: &str) -> IResult<u32> {
    let (input, (_, _, silverlions)) = tuple((
        digit1,
        context(
            "additions",
            many1(tuple((
                tag(" + "),
                delimited(tag("("), alpha1, tag(")")),
                digit1,
            ))),
        ),
        preceded(tag(" = "), parse_silverlions_simple),
    ))(input)?;
    Ok((input, silverlions))
}

fn parse_research_points(input: &str) -> IResult<u32> {
    context(
        "research points",
        alt((parse_research_points_simple, parse_research_points_complex)),
    )(input)
}

fn parse_research_points_simple(input: &str) -> IResult<u32> {
    context("research points simple", terminated(u32, tag(" RP")))(input)
}

fn parse_research_points_complex(input: &str) -> IResult<u32> {
    let (input, (_, _, research_points)) = tuple((
        digit1,
        context(
            "additions",
            many1(tuple((
                tag(" + "),
                delimited(tag("("), alpha1, tag(")")),
                digit1,
            ))),
        ),
        preceded(tag(" = "), parse_research_points_simple),
    ))(input)?;
    Ok((input, research_points))
}

fn parse_crp(input: &str) -> IResult<u32> {
    terminated(u32, tag(" CRP"))(input)
}

fn parse_events(input: &str) -> IResult<Vec<Event>> {
    let (input, tables) = context("event tables", many0(table))(input)?;

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
    let (input, rows) = context("award header", preceded(table_header, many1(short_row)))(input)?;
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
        context("Time Played literal", tag("Time Played")),
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

fn parse_reward_for_winning(input: &str) -> IResult<Reward> {
    delimited(
        pair(tag("Reward for winning"), row_separator),
        parse_reward,
        pair(row_ending, line_ending),
    )(input)
}

// FIXME: too greedy :(
fn vehicle_name(input: &str) -> IResult<String> {
    map(
        take_while(|c: char| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | ' ' => true,
            '#' | '&' | '\'' | '(' | ')' | ',' | '-' | '.' | '/' | '_' => true,
            _ => false,
        }),
        String::from,
    )(input)
}

fn parse_earned(input: &str) -> IResult<Reward> {
    map(
        delimited(
            tag("Earned: "),
            separated_pair(parse_silverlions_simple, tag(", "), parse_crp),
            line_ending,
        ),
        |(silverlions, research)| Reward {
            silverlions,
            research,
        },
    )(input)
}

fn parse_activity(input: &str) -> IResult<u8> {
    map(
        delimited(tag("Activity: "), terminated(u8, tag("%")), line_ending),
        |activity| activity,
    )(input)
}

fn parse_damaged_vehicles(input: &str) -> IResult<Vec<String>> {
    delimited(
        tag("Damaged Vehicles: "),
        separated_list1(tag(", "), map(vehicle_name, String::from)),
        line_ending,
    )(input)
}

fn parse_automatic_repair(input: &str) -> IResult<u32> {
    delimited(
        tag("Automatic repair of all vehicles: -"),
        parse_silverlions_simple,
        line_ending,
    )(input)
}

fn parse_automatic_purchase(input: &str) -> IResult<u32> {
    delimited(
        tag("Automatic purchasing of ammo and \"Crew Replenishment\": -"),
        parse_silverlions_simple,
        line_ending,
    )(input)
}

fn parse_researched_units(input: &str) -> IResult<Vec<VehicleResearch>> {
    delimited(
        pair(tag("Researched unit: "), line_ending),
        context("researched vehicles", many1(parse_vehicle_research)),
        line_ending,
    )(input)
}

fn parse_vehicle_research(input: &str) -> IResult<VehicleResearch> {
    map(
        terminated(
            separated_pair(vehicle_name, tag(": "), parse_research_points_simple),
            line_ending,
        ),
        |(name, research)| VehicleResearch { name, research },
    )(input)
}

fn parse_researched_modifications(input: &str) -> IResult<Vec<ModificationResearch>> {
    delimited(
        pair(tag("Researching progress: "), line_ending),
        many1(parse_modification_research),
        line_ending,
    )(input)
}

fn parse_modification_research(input: &str) -> IResult<ModificationResearch> {
    dbg!(input);
    map(
        terminated(
            tuple((
                map_parser(take_until(" - "), vehicle_name),
                tag(" - "),
                context(
                    "name",
                    take_while(|c: char| c.is_ascii_alphanumeric() || c == ' '),
                ),
                tag(": "),
                parse_research_points_simple,
            )),
            line_ending,
        ),
        |(vehicle, _, name, _, research)| ModificationResearch {
            vehicle,
            name: name.to_string(),
            research,
        },
    )(input)
}

fn inspect<I, O, E, P>(mut p: P) -> impl FnMut(I) -> nom::IResult<I, O, E>
where
    P: Parser<I, O, E>,
    I: Debug + Clone,
    O: Debug,
{
    move |input: I| {
        let old_input = input.clone();
        let (input, output) = p.parse(input)?;
        eprintln!("{old_input:?} -> {output:?} + {input:?}");
        Ok((input, output))
    }
}

fn parse_used_items(input: &str) -> IResult<&str> {
    preceded(
        pair(tag("Used items: "), line_ending),
        take_until("Session: "),
    )(input)
}

fn parse_session_id(input: &str) -> IResult<String> {
    delimited(tag("Session: "), map(hex_digit1, String::from), line_ending)(input)
}

fn parse_total(input: &str) -> IResult<(Reward, u32)> {
    map(
        preceded(
            tag("Total: "),
            tuple((
                parse_silverlions_simple,
                tag(", "),
                parse_crp,
                tag(", "),
                parse_research_points_simple,
            )),
        ),
        |(silverlions, _, crp, _, research)| {
            (
                Reward {
                    silverlions,
                    research,
                },
                crp,
            )
        },
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
        let (input, value) = run_parser(input, super::parse_research_points_simple);
        assert!(input.is_empty());
        assert_eq!(value, expected)
    }

    #[rstest]
    #[case("10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", 40)]
    #[case("96 + (Talismans)96 = 192 RP", 192)]
    #[case("113 + (Talismans)113 = 226 RP", 226)]
    fn parse_research_points_complex(#[case] input: &str, #[case] expected: u32) {
        let (input, value) = run_parser(input, super::parse_research_points_complex);
        assert!(input.is_empty());
        assert_eq!(value, expected)
    }

    #[rstest]
    #[case("10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", 40)]
    #[case("100 RP", 100)]
    #[case("96 + (Talismans)96 = 192 RP", 192)]
    #[case("113 + (Talismans)113 = 226 RP", 226)]
    fn parse_research_points(#[case] input: &str, #[case] expected: u32) {
        let (input, value) = run_parser(input, super::parse_research_points);
        assert!(input.is_empty());
        assert_eq!(value, expected)
    }

    #[rstest]
    #[case("5820 SL     413 RP", 5820, 413)]
    #[case("1000 SL", 1000, 0)]
    #[case("505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP", 505, 40)]
    #[case("53 + (Booster)8 = 61 SL    3 + (Booster)2 = 5 RP", 61, 5)]
    fn parse_reward(#[case] input: &str, #[case] silverlions: u32, #[case] research: u32) {
        let (input, reward) = run_parser(input, super::parse_reward);
        assert_eq!("", input);
        assert_eq!(reward.silverlions, silverlions);
        assert_eq!(reward.research, research);
    }

    #[test]
    fn parse_reward_in_table_header() {
        let input = "255 SL               \n    2:05    Concept 3    M36 GMC()       51 SL\n    3:04    Concept 3    M36 GMC()       51 SL\n    5:56    Concept 3    Chi-To Late     51 SL\n 
   6:25    Concept 3    M6A1            51 SL\n    6:51    Concept 3    ISU-122()       51 SL\n\nDamage taken by scouted enemies               1     101 SL               \n    3:45    Concept 3    M
36 GMC()     101 SL\n\nDestruction by allies of scouted enemies      1     505 SL      40 RP    \n    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40
 RP\n";
        let (input, reward) = run_parser(input, super::parse_reward);
        assert!(matches!(
            reward,
            Reward {
                silverlions: 255,
                research: 0
            }
        ));

        let leftover = "               \n    2:05    Concept 3    M36 GMC()       51 SL\n    3:04    Concept 3    M36 GMC()       51 SL\n    5:56    Concept 3    Chi-To Late     51 SL\n 
   6:25    Concept 3    M6A1            51 SL\n    6:51    Concept 3    ISU-122()       51 SL\n\nDamage taken by scouted enemies               1     101 SL               \n    3:45    Concept 3    M
36 GMC()     101 SL\n\nDestruction by allies of scouted enemies      1     505 SL      40 RP    \n    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40
 RP\n";

        assert_eq!(input, leftover);
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
    fn parse_scouting_of_the_enemy_table() {
        let input = r#"Scouting of the enemy                         5     255 SL               
    2:05    Concept 3    M36 GMC()       51 SL
    3:04    Concept 3    M36 GMC()       51 SL
    5:56    Concept 3    Chi-To Late     51 SL
    6:25    Concept 3    M6A1            51 SL
    6:51    Concept 3    ISU-122()       51 SL

Damage taken by scouted enemies               1     101 SL               
    3:45    Concept 3    M36 GMC()     101 SL

Destruction by allies of scouted enemies      1     505 SL      40 RP    
    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP
"#;
        let (input, table) = run_parser(input, super::table);
        assert!(!input.is_empty());
        assert_eq!(table.name, "Scouting of the enemy");
        assert_eq!(table.rows.len(), 5);
    }

    #[test]
    fn parse_scouting_table_header_with_leftovers() {
        let input = r#"Scouting of the enemy                         5     255 SL               
    2:05    Concept 3    M36 GMC()       51 SL
    3:04    Concept 3    M36 GMC()       51 SL
    5:56    Concept 3    Chi-To Late     51 SL
    6:25    Concept 3    M6A1            51 SL
    6:51    Concept 3    ISU-122()       51 SL

Damage taken by scouted enemies               1     101 SL               
    3:45    Concept 3    M36 GMC()     101 SL

Destruction by allies of scouted enemies      1     505 SL      40 RP    
    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP
"#;
        let leftover = r#"    2:05    Concept 3    M36 GMC()       51 SL
    3:04    Concept 3    M36 GMC()       51 SL
    5:56    Concept 3    Chi-To Late     51 SL
    6:25    Concept 3    M6A1            51 SL
    6:51    Concept 3    ISU-122()       51 SL

Damage taken by scouted enemies               1     101 SL               
    3:45    Concept 3    M36 GMC()     101 SL

Destruction by allies of scouted enemies      1     505 SL      40 RP    
    3:45    Concept 3    M36 GMC()     ×    505 SL    10 + (PA)10 + (Booster)10 + (Talismans)10 = 40 RP
"#;

        let (input, (name, count, reward)) = run_parser(input, super::table_header);
        assert_eq!(input, leftover);
        assert_eq!(name, "Scouting of the enemy");
        assert_eq!(count, 5);
        assert_eq!(reward.silverlions, 255);
        assert_eq!(reward.research, 0);
    }

    #[test]
    fn parse_awards_table() {
        let input = r#"Awards                                       14    3450 SL     100 RP    
    3:46     Intelligence             100 SL           
    7:14     Tank Rescuer             50 SL            
    8:18     Rank does not matter     500 SL           
    8:32     Multi strike!            100 SL           
    8:32     Without a miss           200 SL           
    10:35    Ground Force Rescuer     150 SL           
    11:47    Without a miss           200 SL           
    13:14    Without a miss           200 SL           
    13:43    Eye for Eye              300 SL           
    13:43    Shadow strike streak!    100 SL           
    13:43    Multi strike!            100 SL           
    13:43    Without a miss           200 SL           
    13:55    Final blow!              250 SL           
    13:55    The Best Squad           1000 SL    100 RP

"#;
        let (input, awards) = run_parser(input, super::award_table);
        assert_eq!(input, "");
        assert_eq!(awards.len(), 14);
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

    #[test]
    fn test_parse_vehicle_research() {
        let input = "T-34 (1941): 1191 RP\n";
        let (input, research) = run_parser(input, super::parse_vehicle_research);
        assert_eq!(input, "");
        assert_eq!(research.name, "T-34 (1941)");
        assert_eq!(research.research, 1191);
    }

    #[test]
    fn test_parse_researched_units() {
        let input = r#"Researched unit: 
T-34 (1941): 1191 RP

"#;
        let (input, research) = run_parser(input, super::parse_researched_units);
        assert_eq!(input, "");
        assert_eq!(research.len(), 1);
        assert_eq!(research[0].name, "T-34 (1941)");
        assert_eq!(research[0].research, 1191);
    }

    #[test]
    fn test_parse_modification_research() {
        let input = "YaG-10 (29-K) - Improved Parts: 220 RP\n";
        let (input, research) = run_parser(input, super::parse_modification_research);
        assert_eq!(input, "");
        assert_eq!(research.vehicle, "YaG-10 (29-K)");
        assert_eq!(research.name, "Improved Parts");
        assert_eq!(research.research, 220);
    }
}
