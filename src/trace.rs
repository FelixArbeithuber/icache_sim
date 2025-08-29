use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use winnow::ascii::{line_ending, multispace0, space0, space1, till_line_ending};
use winnow::combinator::{
    alt, cut_err, delimited, eof, fail, opt, peek, preceded, repeat, repeat_till, separated_pair,
    terminated,
};
use winnow::error::{ContextError, ParseError, StrContext};
use winnow::stream::AsChar;
use winnow::token::take_while;
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
pub struct TraceFile<'a> {
    named_blocks: HashMap<&'a str, NamedBlock<'a>>,
}

impl<'a> TryFrom<&'a str> for TraceFile<'a> {
    type Error = TraceParseError<'a>;

    fn try_from(input: &'a str) -> Result<Self, Self::Error> {
        let blocks: Vec<NamedBlock<'a>> = repeat(0.., preceded(multispace, named_block))
            .context(StrContext::Label("trace blocks"))
            .parse(input)
            .map_err(TraceParseError::ParseError)?;

        let mut block_map = HashMap::new();
        for block in blocks {
            if block_map.contains_key(block.name) {
                return Err(TraceParseError::SyntaxError(format!(
                    "block '{}()' defined multiple times",
                    block.name
                )));
            }

            block_map.insert(block.name, block);
        }

        // the order we go through all statements does not matter
        // we just want to check if all functions mentioned have a corresponding definition
        let mut queue =
            Vec::<&Op<'a>>::from_iter(block_map.values().flat_map(|block| block.ops.iter()));
        while let Some(stmt) = queue.pop() {
            match stmt {
                Op::BlockCall {
                    block_name: function_name,
                } => {
                    if !block_map.contains_key(function_name) {
                        return Err(TraceParseError::SyntaxError(format!(
                            "unknown function '{function_name}()'"
                        )));
                    }
                }
                Op::Loop { block, .. } => {
                    queue.extend(block.ops.iter());
                }
                Op::Switch { cases } => {
                    for case in cases {
                        queue.extend(case.block.ops.iter());
                    }
                }
                _ => {}
            }
        }

        Ok(Self {
            named_blocks: block_map,
        })
    }
}

impl<'a> IntoIterator for TraceFile<'a> {
    type Item = (&'a str, std::vec::IntoIter<Instruction>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        fn block_iter<'a>(
            block: &NamedBlock<'a>,
            block_map: &HashMap<&'a str, NamedBlock<'a>>,
        ) -> std::vec::IntoIter<Instruction> {
            let mut rng: StdRng = StdRng::seed_from_u64(0);
            let mut addresses = Vec::new();

            let mut queue = Vec::<&Op<'a>>::from_iter(block.ops.iter().rev());
            while let Some(op) = queue.pop() {
                match op {
                    Op::Range {
                        addr_start,
                        instr_length,
                        addr_end,
                    } => addresses.extend((*addr_start..*addr_end).step_by(*instr_length / 8).map(
                        |address| Instruction {
                            address,
                            length: *instr_length,
                        },
                    )),
                    Op::BlockCall { block_name } => {
                        queue.extend(block_map.get(block_name).unwrap().ops.iter().rev());
                    }
                    Op::Loop { count, block } => {
                        for _ in 0..*count {
                            queue.extend(block.ops.iter().rev());
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
                                queue.extend(cases.get(i).unwrap().block.ops.iter().rev());
                                break;
                            }
                        }
                    }
                }
            }

            addresses.into_iter()
        }

        self.named_blocks
            .values()
            .filter(|&block| block.compare)
            .map(|block| (block.name, block_iter(block, &self.named_blocks)))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

pub struct Instruction {
    pub address: usize,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct NamedBlock<'a> {
    compare: bool,
    name: &'a str,
    ops: Vec<Op<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
struct Block<'a> {
    ops: Vec<Op<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
enum Op<'a> {
    Range {
        addr_start: usize,
        instr_length: usize,
        addr_end: usize,
    },
    BlockCall {
        block_name: &'a str,
    },
    Loop {
        count: usize,
        block: Block<'a>,
    },
    Switch {
        cases: Vec<SwitchCase<'a>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct SwitchCase<'a> {
    weight: usize,
    block: Block<'a>,
}

fn block_name<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_while(1.., (AsChar::is_alphanum, '_', '-')).parse_next(input)
}

fn named_block<'a>(input: &mut &'a str) -> ModalResult<NamedBlock<'a>> {
    (
        opt(terminated("compare", space1)).map(|cmp| cmp.is_some()),
        delimited('\'', cut_err(block_name), cut_err('\'')),
        cut_err(block),
    )
        .map(|(compare, name, Block { ops })| NamedBlock { compare, name, ops })
        .parse_next(input)
}

fn block<'a>(input: &mut &'a str) -> ModalResult<Block<'a>> {
    delimited(
        (multispace, '{').context(StrContext::Label("block start")),
        cut_err(
            repeat_till(
                0..,
                op,
                (multispace, '}').context(StrContext::Label("block end")),
            )
            .map(|(ops, _)| Block { ops }),
        ),
        end,
    )
    .parse_next(input)
}

fn op<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    // important: try 'range' before 'address' because of ambiguity
    preceded(multispace, alt((block_call, looop, switch, range)))
        .context(StrContext::Label("statement"))
        .parse_next(input)
}

fn range<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    fn range_inner(input: &mut &str) -> ModalResult<(usize, usize, usize)> {
        terminated((integer, delimited("..", integer, ".."), integer), end).parse_next(input)
    }

    let (addr_start, instr_length, addr_end) = peek(range_inner).parse_next(input)?;

    if addr_start >= addr_end {
        return fail
            .context(StrContext::Label("range: range is empty"))
            .parse_next(input)?;
    }

    if instr_length % 8 != 0 {
        return fail
            .context(StrContext::Label(
                "range: instruction size is not a multiple of 8 (bits)",
            ))
            .parse_next(input)?;
    }

    if (addr_end - addr_start) % (instr_length / 8) != 0 {
        return fail
            .context(StrContext::Label(
                "range: instruction size does not cleanly fit in range",
            ))
            .parse_next(input)?;
    }

    range_inner
        .parse_next(input)
        .map(|(addr_start, instr_length, addr_end)| Op::Range {
            addr_start,
            instr_length,
            addr_end,
        })
}

fn block_call<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    terminated(block_name, ("()", end))
        .map(|function_name| Op::BlockCall {
            block_name: function_name,
        })
        .parse_next(input)
        .map_err(|e| e.backtrack()) // remove cut_err from function_name
}

fn looop<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    preceded(
        "loop",
        cut_err((
            delimited((space, '(', space), decimal_integer, (space, ')', space))
                .context(StrContext::Label("loop count")),
            block,
        ))
        .map(|(count, block)| Op::Loop { count, block }),
    )
    .parse_next(input)
}

fn switch<'a>(input: &mut &'a str) -> ModalResult<Op<'a>> {
    preceded(
        ("switch:", cut_err(end)),
        cut_err(terminated(
            repeat_till(
                1..,
                switch_case,
                (multispace, "endswitch").context(StrContext::Label("switch end")),
            )
            .map(|(cases, _)| Op::Switch { cases }),
            end,
        )),
    )
    .parse_next(input)
}

fn switch_case<'a>(input: &mut &'a str) -> ModalResult<SwitchCase<'a>> {
    separated_pair(
        delimited((space, '(', space), decimal_integer, (space, ')', space)),
        (space, ':', space),
        block,
    )
    .map(|(weight, block)| SwitchCase { weight, block })
    .context(StrContext::Label("switch case"))
    .parse_next(input)
}

fn integer(input: &mut &str) -> ModalResult<usize> {
    alt((
        preceded(
            "0x",
            cut_err(take_while(1.., ('0'..='9', 'a'..='f', 'A'..='F')))
                .try_map(|s| usize::from_str_radix(s, 16))
                .context(StrContext::Label("hexadecimal number")),
        ),
        preceded(
            "0b",
            cut_err(take_while(1.., '0'..='1'))
                .try_map(|s| usize::from_str_radix(s, 2))
                .context(StrContext::Label("binary number")),
        ),
        preceded(
            "0o",
            cut_err(take_while(1.., '0'..='7'))
                .try_map(|s| usize::from_str_radix(s, 8))
                .context(StrContext::Label("octal number")),
        ),
        cut_err(decimal_integer),
    ))
    .parse_next(input)
}

fn decimal_integer(input: &mut &str) -> ModalResult<usize> {
    take_while(1.., '0'..='9')
        .context(StrContext::Label("decimal number"))
        .try_map(str::parse::<usize>)
        .parse_next(input)
}

fn end(input: &mut &str) -> ModalResult<()> {
    (space, alt((line_ending, eof)), multispace)
        .void()
        .context(StrContext::Label("newline"))
        .parse_next(input)
}

fn space(input: &mut &str) -> ModalResult<()> {
    (space0, opt(("//", till_line_ending)), space0)
        .void()
        .context(StrContext::Label("newline"))
        .parse_next(input)
}

fn multispace(input: &mut &str) -> ModalResult<()> {
    (
        multispace0,
        repeat::<_, _, (), _, _>(0.., ("//", till_line_ending, multispace0)),
        multispace0,
    )
        .void()
        .context(StrContext::Label("newline"))
        .parse_next(input)
}

#[cfg(test)]
mod test {
    use super::TraceFile;

    #[test]
    fn check_all_traces() {
        for file in std::fs::read_dir("./traces/").unwrap() {
            let file = file.unwrap();

            if file.metadata().unwrap().is_file() {
                let file_content = std::fs::read_to_string(file.path()).unwrap();
                let trace = TraceFile::try_from(file_content.as_str());
                assert!(
                    trace.is_ok(),
                    "failed to parse {:?}: {}",
                    file.file_name(),
                    trace.unwrap_err()
                )
            }
        }
    }
}
