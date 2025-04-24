use std::collections::HashMap;

use winnow::ascii::{alphanumeric1, float, line_ending, multispace0, space0};
use winnow::combinator::{
    alt, delimited, dispatch, eof, fail, preceded, repeat, repeat_till, separated_pair, terminated,
};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext};
use winnow::stream::AsChar;
use winnow::token::{take, take_while};
use winnow::{ModalResult, Parser};

pub struct AccessTrace<'a> {
    functions: HashMap<&'a str, Function<'a>>,
    main_block: Vec<Statement<'a>>,
}

impl<'a> TryFrom<&mut &'a str> for AccessTrace<'a> {
    type Error = ParseError<&'a str, ContextError>;

    fn try_from(input: &mut &'a str) -> Result<Self, Self::Error> {
        (repeat(0.., function), block)
            .parse(input)
            .map(|(functions, main_block)| Self {
                functions,
                main_block,
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Function<'a> {
    block: Vec<Statement<'a>>,
}

fn function<'a>(input: &mut &'a str) -> ModalResult<(&'a str, Function<'a>)> {
    delimited((multispace0, "fn", space0), (function_name, block), end)
        .parse_next(input)
        .map(|(function_name, block)| (function_name, Function { block }))
}

fn function_name<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    if input.chars().next().is_some_and(|c| c.is_alpha()) {
        (alphanumeric1).parse_next(input)
    } else {
        fail.parse_next(input)
    }
}

fn end<'a>(input: &mut &'a str) -> ModalResult<(&'a str, &'a str, &'a str)> {
    (space0, alt((line_ending, eof)), multispace0).parse_next(input)
}

#[derive(Debug, Clone, PartialEq)]
enum Statement<'a> {
    Address {
        addr: usize,
    },
    Range {
        addr_start: usize,
        addr_end: usize,
    },
    FunctionCall {
        function_name: &'a str,
    },
    Loop {
        count: usize,
        block: Vec<Statement<'a>>,
    },
    Switch {
        cases: Vec<SwitchCase<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct SwitchCase<'a> {
    probability: f32,
    block: Vec<Statement<'a>>,
}

fn block<'a>(input: &mut &'a str) -> ModalResult<Vec<Statement<'a>>> {
    delimited(
        (multispace0, '{'),
        repeat_till(0.., statement, (multispace0, '}')).context(StrContext::Label("block end")),
        end,
    )
    .parse_next(input)
    .map(|(statements, _)| statements)
}

fn statement<'a>(input: &mut &'a str) -> ModalResult<Statement<'a>> {
    // important: try 'range' before 'address' because of ambiguity
    preceded(
        multispace0,
        alt((range, address, function_call, looop, switch)),
    )
    .parse_next(input)
}

fn address<'a>(input: &mut &'a str) -> ModalResult<Statement<'a>> {
    terminated(integer, end)
        .context(StrContext::Label("address"))
        .parse_next(input)
        .map(|addr| Statement::Address { addr })
}

fn range<'a>(input: &mut &'a str) -> ModalResult<Statement<'a>> {
    terminated(separated_pair(integer, "..", integer), end)
        .parse_next(input)
        .map(|(addr_start, addr_end)| Statement::Range {
            addr_start,
            addr_end,
        })
}

fn function_call<'a>(input: &mut &'a str) -> ModalResult<Statement<'a>> {
    terminated(function_name, ("()", end))
        .parse_next(input)
        .map(|function_name| Statement::FunctionCall { function_name })
}

fn looop<'a>(input: &mut &'a str) -> ModalResult<Statement<'a>> {
    preceded(
        "loop",
        (
            delimited((space0, '(', space0), integer, (space0, ')')),
            block,
        ),
    )
    .parse_next(input)
    .map(|(count, block)| Statement::Loop { count, block })
}

fn switch<'a>(input: &mut &'a str) -> ModalResult<Statement<'a>> {
    let (cases, _): (Vec<SwitchCase<'a>>, _) = delimited(
        ("switch:", end),
        repeat_till(1.., switch_case, (multispace0, "endswitch")),
        end,
    )
    .parse_next(input)?;

    if cases.iter().fold(0.0, |sum, case| sum + case.probability) <= 1.0 {
        Ok(Statement::Switch { cases })
    } else {
        Err(ErrMode::Backtrack(ContextError::new()))
    }
}

fn switch_case<'a>(input: &mut &'a str) -> ModalResult<SwitchCase<'a>> {
    separated_pair(
        delimited((space0, '(', space0), float, (space0, ')', space0)),
        (space0, ':', space0),
        block,
    )
    .parse_next(input)
    .map(|(probability, block)| SwitchCase { probability, block })
}

fn integer(input: &mut &str) -> ModalResult<usize> {
    alt((dispatch! {
        take(2usize);
        "0b" => take_while(1.., '0'..='1').try_map(|s| usize::from_str_radix(s, 2)),
        "0o" => take_while(1.., '0'..='7').try_map(|s| usize::from_str_radix(s, 8)),
        "0x" => take_while(1.., ('0'..='9', 'a'..='f', 'A'..='F')).try_map(|s| usize::from_str_radix(s, 16)),
        _ => fail::<_, usize, _>,
    }, decimal_integer))
    .parse_next(input)
}

fn decimal_integer(input: &mut &str) -> ModalResult<usize> {
    take_while(1.., '0'..='9')
        .try_map(str::parse::<usize>)
        .parse_next(input)
}

#[cfg(test)]
mod test {
    use super::*;
    use winnow::Parser;

    #[test]
    fn test() {
        let mut trace = String::from(
            r#"
            {
                0x00
                0x00..0x20
                abc()

                switch:
                    (0.5): {
                        loop(10) {
                            0x05..0x06
                        }
                    }
                    (0.5): {
                        0x03..0x04
                    }
                endswitch

                loop(10) {
                    0x05..0x06
                }
            }
            "#,
        );

        println!("{:?}", block.parse(trace.as_mut_str()).unwrap());
    }
}
