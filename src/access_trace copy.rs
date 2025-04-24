use winnow::ascii::{float, line_ending, multispace0, space0};
use winnow::combinator::{
    alt, delimited, dispatch, eof, fail, preceded, repeat, repeat_till, separated_pair, terminated,
};
use winnow::error::{ContextError, ErrMode};
use winnow::token::{take, take_while};
use winnow::{ModalResult, Parser};

#[derive(Debug, Clone, PartialEq)]
enum Op {
    Address(usize),
    Loop { count: usize, ops: Vec<Op> },
    If { offset: usize, probability: f32 },
}

pub struct AccessTrace {
    ops: Vec<Op>,
}

impl TryFrom<&mut &str> for AccessTrace {
    type Error = ErrMode<ContextError>;

    fn try_from(input: &mut &str) -> Result<Self, Self::Error> {
        trace.parse_next(input).map(|ops| AccessTrace { ops })
    }
}

fn trace(input: &mut &str) -> ModalResult<Vec<Op>> {
    terminated(repeat(0.., op), multispace0).parse_next(input)
}

fn op(input: &mut &str) -> ModalResult<Op> {
    delimited(multispace0, alt((address_op, loop_op, if_op)), end_of_op).parse_next(input)
}

fn address_op(input: &mut &str) -> ModalResult<Op> {
    integer.parse_next(input).map(Op::Address)
}

fn loop_op(input: &mut &str) -> ModalResult<Op> {
    preceded(
        "loop",
        (
            delimited((space0, '(', space0), integer, (space0, "):", space0)),
            repeat_till(0.., op, preceded(multispace0, "endloop")),
        ),
    )
    .parse_next(input)
    .map(|(count, (ops, _))| Op::Loop { count, ops })
}

fn if_op(input: &mut &str) -> ModalResult<Op> {
    delimited(
        "maybe(",
        separated_pair(
            delimited(delimited(space0, '+', space0), integer, space0),
            (space0, ",", space0),
            delimited(space0, float::<_, f32, _>, space0),
        ),
        ')',
    )
    .parse_next(input)
    .map(|(offset, probability)| Op::If {
        offset,
        probability,
    })
}

fn end_of_op<'a>(input: &mut &'a str) -> ModalResult<(&'a str, &'a str, &'a str)> {
    (space0, alt((line_ending, eof)), multispace0).parse_next(input)
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
    fn integer() {
        assert_eq!(super::integer.parse_peek("10"), Ok(("", 10)),);
        assert_eq!(
            super::integer.parse_peek("0b010101"),
            Ok(("", usize::from_str_radix("010101", 2).unwrap())),
        );
        assert_eq!(
            super::integer.parse_peek("0o234531"),
            Ok(("", usize::from_str_radix("234531", 8).unwrap())),
        );
        assert_eq!(
            super::integer.parse_peek("0x1FAB01"),
            Ok(("", usize::from_str_radix("1FAB01", 16).unwrap())),
        );
    }

    #[test]
    fn address_op() {
        assert_eq!(
            super::op.parse_peek(" \t 0x20\n"),
            Ok(("", Op::Address(usize::from_str_radix("20", 16).unwrap())))
        );
    }

    #[test]
    fn loop_op() {
        const LOOP: &str = r#"
            loop(10):
                0x80
                0x0
                0x20
            endloop
        "#;

        assert_eq!(
            super::op.parse_peek(LOOP),
            Ok((
                "",
                Op::Loop {
                    count: 10,
                    ops: vec![Op::Address(128), Op::Address(0), Op::Address(32)],
                }
            ))
        );
    }

    #[test]
    fn if_op() {
        let ok = Ok((
            "",
            Op::If {
                offset: 255,
                probability: 0.1,
            },
        ));

        assert_eq!(super::op.parse_peek("maybe(+255, 0.1)"), ok);
        assert_eq!(super::op.parse_peek("maybe(+ 255, 0.1)"), ok);
        assert_eq!(super::op.parse_peek("maybe(+ 255, 0.1)"), ok);
        assert_eq!(super::op.parse_peek("maybe( +255, 0.1)"), ok);
        assert_eq!(super::op.parse_peek("maybe( + 255, 0.1)"), ok);
        assert_eq!(super::op.parse_peek("maybe(+255 , 0.1)"), ok);
        assert_eq!(super::op.parse_peek("maybe(+255, 0.1 )"), ok);

        assert_eq!(super::op.parse_peek("maybe(+0xFF, 0.1 )"), ok);
        assert_eq!(super::op.parse_peek("maybe(+0b11111111, 0.1 )"), ok);
    }

    #[test]
    fn test() {
        const TRACE: &str = r#"
            0x00
            0x20
            0x40
            0x60
            loop(10):
                0x80
                0x0
                0x20
            endloop
            0x40
            0x60
            maybe(+5, 0.5)
            0x80
            0x00
            0x20
            0x40
            0x60
        "#;

        let trace = super::trace.parse_peek(TRACE).unwrap();
        assert_eq!(trace.0, "");
        assert_eq!(trace.1.len(), 13);
    }
}
