use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use winnow::ascii::{alphanumeric1, float, line_ending, multispace0, space0};
use winnow::combinator::{
    alt, delimited, dispatch, eof, fail, preceded, repeat, repeat_till, separated_pair, terminated,
};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext};
use winnow::stream::AsChar;
use winnow::token::{take, take_while};
use winnow::{ModalResult, Parser};

#[derive(Debug)]
pub enum TraceParseError<'a> {
    ParseError(ParseError<&'a str, ContextError>),
    SyntaxError(String),
}

impl std::fmt::Display for TraceParseError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceParseError::ParseError(parse_error) => f.write_fmt(format_args!("{parse_error}")),
            TraceParseError::SyntaxError(e) => f.write_fmt(format_args!("{e}")),
        }
    }
}

impl std::error::Error for TraceParseError<'_> {}

#[derive(Debug)]
pub struct AccessTrace<'a> {
    functions: HashMap<&'a str, Function<'a>>,
    main_block: Vec<Op<'a>>,
}

impl<'a> TryFrom<&mut &'a str> for AccessTrace<'a> {
    type Error = TraceParseError<'a>;

    fn try_from(input: &mut &'a str) -> Result<Self, Self::Error> {
        let (functions_list, main_block): (Vec<(&'a str, Function<'a>)>, Vec<Op<'a>>) =
            (repeat(0.., function), block)
                .parse(input)
                .map_err(TraceParseError::ParseError)?;

        let mut functions = HashMap::new();
        for (function_name, function) in functions_list {
            if functions.contains_key(function_name) {
                return Err(TraceParseError::SyntaxError(format!(
                    "function with name '{function_name}' defined multiple times"
                )));
            }
            functions.insert(function_name, function);
        }

        let mut queue = Vec::<&Op<'a>>::from_iter(main_block.iter());
        for stmt in main_block.iter() {
            match stmt {
                Op::FunctionCall { function_name } => {
                    if !functions.contains_key(function_name) {
                        return Err(TraceParseError::SyntaxError(format!(
                            "unknown function {function_name}"
                        )));
                    }
                }
                Op::Loop { count: _, block } => {
                    queue.extend(block.iter());
                }
                Op::Switch { cases } => {
                    for case in cases {
                        queue.extend(case.block.iter());
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            functions,
            main_block,
        })
    }
}

impl<'a> IntoIterator for AccessTrace<'a> {
    type Item = usize;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut rng: StdRng = StdRng::seed_from_u64(0);
        let mut addresses = Vec::new();

        let mut queue = Vec::<&Op<'a>>::from_iter(self.main_block.iter().rev());
        while let Some(op) = queue.pop() {
            match op {
                Op::Address { addr } => addresses.push(*addr),
                Op::Range {
                    addr_start,
                    addr_end,
                } => addresses.extend((*addr_start)..(*addr_end)),
                Op::FunctionCall { function_name } => {
                    let function = self.functions.get(function_name).unwrap();
                    queue.extend(function.block.iter().rev());
                }
                Op::Loop { count, block } => {
                    for _ in 0..*count {
                        queue.extend(block.iter().rev());
                    }
                }
                Op::Switch { cases } => {
                    let random_float: f32 = rng.sample(rand::distr::StandardUniform);

                    let mut probabilities: Vec<(usize, f32)> = cases
                        .iter()
                        .enumerate()
                        .map(|(i, case)| (i, case.probability))
                        .collect();
                    probabilities.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

                    let mut sum = 0.0;
                    for (i, probability) in probabilities {
                        sum += probability;
                        if sum >= random_float {
                            queue.extend(cases.get(i).unwrap().block.iter().rev());
                            break;
                        }
                    }
                }
            }
        }

        addresses.into_iter()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Function<'a> {
    block: Vec<Op<'a>>,
}

fn function<'a>(input: &mut &'a str) -> ModalResult<(&'a str, Function<'a>)> {
    preceded(
        (multispace0, "fn", space0),
        separated_pair(function_name, "()", block),
    )
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
pub enum Op<'a> {
    Address { addr: usize },
    Range { addr_start: usize, addr_end: usize },
    FunctionCall { function_name: &'a str },
    Loop { count: usize, block: Vec<Op<'a>> },
    Switch { cases: Vec<SwitchCase<'a>> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchCase<'a> {
    probability: f32,
    block: Vec<Op<'a>>,
}

fn block<'a>(input: &mut &'a str) -> ModalResult<Vec<Op<'a>>> {
    delimited(
        (multispace0, '{'),
        repeat_till(0.., statement, (multispace0, '}')).context(StrContext::Label("block end")),
        end,
    )
    .parse_next(input)
    .map(|(statements, _)| statements)
}

fn statement<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    // important: try 'range' before 'address' because of ambiguity
    preceded(
        multispace0,
        alt((range, address, function_call, looop, switch)),
    )
    .parse_next(input)
}

fn address<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    terminated(integer, end)
        .context(StrContext::Label("address"))
        .parse_next(input)
        .map(|addr| Op::Address { addr })
}

fn range<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    terminated(separated_pair(integer, "..", integer), end)
        .parse_next(input)
        .map(|(addr_start, addr_end)| Op::Range {
            addr_start,
            addr_end,
        })
}

fn function_call<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    terminated(function_name, ("()", end))
        .parse_next(input)
        .map(|function_name| Op::FunctionCall { function_name })
}

fn looop<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    preceded(
        "loop",
        (
            delimited((space0, '(', space0), integer, (space0, ')')),
            block,
        ),
    )
    .parse_next(input)
    .map(|(count, block)| Op::Loop { count, block })
}

fn switch<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    let (cases, _): (Vec<SwitchCase<'a>>, _) = delimited(
        ("switch:", end),
        repeat_till(1.., switch_case, (multispace0, "endswitch")),
        end,
    )
    .parse_next(input)?;

    if (0.0..=1.0).contains(&cases.iter().fold(0.0, |sum, case| sum + case.probability)) {
        Ok(Op::Switch { cases })
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

    #[test]
    fn test() {
        let trace = String::from(
            r#"

            fn cde() {
                0x00
            }

            fn abc() {
                0x00
            }

            fn abc() {
                0x00
            }

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

        println!("{:?}", AccessTrace::try_from(&mut trace.as_str()).unwrap());
    }
}
