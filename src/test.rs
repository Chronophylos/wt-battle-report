use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, digit1, multispace0, space0},
    combinator::{map, map_res, opt},
    error::VerboseError,
    multi::separated_list1,
    sequence::{delimited, pair, preceded, terminated, tuple},
};

type IResult<'a, O> = nom::IResult<&'a str, O, VerboseError<&'a str>>;

fn parse_number(input: &str) -> IResult<u32> {
    map_res(digit1, |s: &str| s.parse::<u32>())(input)
}

fn parse_research_points_simple(input: &str) -> IResult<u32> {
    let (input, (rp, _)) = pair(parse_number, tag(" RP"))(input)?;
    Ok((input, rp))
}

fn parse_research_points_complex(input: &str) -> IResult<u32> {
    let (input, _) = take_until("= ")(input)?;
    preceded(tag("= "), parse_research_points_simple)(input)
}

fn parse_research_points(input: &str) -> IResult<u32> {
    alt((parse_research_points_simple, parse_research_points_complex))(input)
}

#[cfg(test)]
mod test {
    use rstest::*;

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
}
