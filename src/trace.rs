use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use winnow::ascii::{alphanumeric1, line_ending, multispace0, space0};
use winnow::combinator::{
    alt, delimited, dispatch, eof, fail, preceded, repeat, repeat_till, separated_pair, terminated,
};
use winnow::error::{ContextError, ErrMode, ParseError, StrContext, StrContextValue};
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
pub struct Trace<'a> {
    functions: HashMap<&'a str, Function<'a>>,
    main_block: Vec<Op<'a>>,
}

impl<'a> TryFrom<&mut &'a str> for Trace<'a> {
    type Error = TraceParseError<'a>;

    fn try_from(input: &mut &'a str) -> Result<Self, Self::Error> {
        let (functions_list, main_block): (Vec<(&'a str, Function<'a>)>, Vec<Op<'a>>) = (
            repeat(0.., function).context(StrContext::Label("function definitions")),
            preceded(("main", multispace0), block).context(StrContext::Label("main block")),
        )
            .parse(input)
            .map_err(TraceParseError::ParseError)?;

        let mut functions = HashMap::new();
        for (function_name, function) in functions_list {
            if functions.contains_key(function_name) {
                return Err(TraceParseError::SyntaxError(format!(
                    "function '{function_name}()' defined multiple times"
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
                            "unknown function '{function_name}()'"
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

impl<'a> IntoIterator for Trace<'a> {
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
                    let mut weights: Vec<(usize, usize)> = cases
                        .iter()
                        .enumerate()
                        .map(|(i, case)| (i, case.weight))
                        .collect();
                    weights.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

                    let total_weights = weights.iter().map(|(_, weight)| weight).sum();
                    let random = rng.random_range(0..=total_weights);

                    let mut sum = 0;
                    for (i, weight) in weights {
                        sum += weight;
                        if sum >= random {
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
    weight: usize,
    block: Vec<Op<'a>>,
}

fn function<'a>(input: &mut &'a str) -> ModalResult<(&'a str, Function<'a>)> {
    _ = (multispace0, "fn", space0)
        .context(StrContext::Label("function start"))
        .parse_next(input)?;

    separated_pair(
        function_name,
        "()".context(StrContext::Label("function brackets")),
        block.context(StrContext::Label("function block")),
    )
    .parse_next(input)
    .map_err(|e| e.cut())
    .map(|(function_name, block)| (function_name, Function { block }))
}

fn function_name<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    if input.chars().next().is_some_and(|c| c.is_alpha()) {
        alphanumeric1
            .context(StrContext::Label("function name"))
            .context(StrContext::Expected(StrContextValue::Description(
                "a function name consisting of ASCII characters (a-Z) and numbers (0-9)",
            )))
            .parse_next(input)
    } else {
        fail.context(StrContext::Label("function name"))
            .context(StrContext::Expected(StrContextValue::Description(
                "a function name starting with an ASCII characters (a-Z)",
            )))
            .parse_next(input)
    }
    .map_err(ErrMode::Cut)
}

fn block<'a>(input: &mut &'a str) -> ModalResult<Vec<Op<'a>>> {
    delimited(
        (multispace0, '{').context(StrContext::Label("block start")),
        repeat_till(
            1..,
            op,
            (multispace0, '}').context(StrContext::Label("block end")),
        ),
        end,
    )
    .parse_next(input)
    .map(|(op, _)| op)
    .map_err(|e| e.cut())
}

fn op<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    // important: try 'range' before 'address' because of ambiguity
    preceded(
        multispace0,
        alt((range, address, function_call, looop, switch)),
    )
    .context(StrContext::Label("op"))
    .context(StrContext::Expected(StrContextValue::Description("address ( 0x00 ), range( 0x00..0x00 ), function call ( a() ), switch ( switch: ... endswitch ), loop ( loop(1) {} )")))
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
        .context(StrContext::Label("range"))
        .parse_next(input)
        .map(|(addr_start, addr_end)| Op::Range {
            addr_start,
            addr_end,
        })
}

fn function_call<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    terminated(function_name, ("()", end))
        .context(StrContext::Label("function call"))
        .parse_next(input)
        .map(|function_name| Op::FunctionCall { function_name })
        .map_err(|e| e.backtrack())
}

fn looop<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    _ = "loop".parse_next(input)?;

    (
        delimited((space0, '(', space0), integer, (space0, ')'))
            .context(StrContext::Label("loop count")),
        block,
    )
        .context(StrContext::Label("loop"))
        .parse_next(input)
        .map(|(count, block)| Op::Loop { count, block })
        .map_err(|e| e.cut())
}

fn switch<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    _ = ("switch:", end)
        .context(StrContext::Label("switch start"))
        .parse_next(input)?;

    terminated(
        repeat_till(
            1..,
            switch_case,
            (multispace0, "endswitch").context(StrContext::Label("switch end")),
        ),
        end,
    )
    .parse_next(input)
    .map(|(cases, _)| Op::Switch { cases })
}

fn switch_case<'a>(input: &mut &'a str) -> ModalResult<SwitchCase<'a>> {
    separated_pair(
        delimited((space0, '(', space0), integer, (space0, ')', space0)),
        (space0, ':', space0),
        block,
    )
    .context(StrContext::Label("switch case"))
    .parse_next(input)
    .map(|(weight, block)| SwitchCase { weight, block })
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

fn end<'a>(input: &mut &'a str) -> ModalResult<(&'a str, &'a str, &'a str)> {
    (space0, alt((line_ending, eof)), multispace0)
        .context(StrContext::Label("newline"))
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

            fn abcd() {
                0x00
            }

            fn abc() {
                0x00
            }

            main {
                0x00
                0x00..0x20
                abc()

                switch:
                    (1): {
                        loop(10) {
                            0x05..0x06
                        }
                    }
                    (1): {
                        0x03..0x04
                    }
                endswitch

                loop(10) {
                    0x05..0x06
                }
            }
            "#,
        );

        println!("{:?}", Trace::try_from(&mut trace.as_str()).unwrap());
    }
}
