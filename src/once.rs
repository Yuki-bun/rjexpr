#![allow(dead_code)]

use nom::{Err, IResult, Mode, OutputMode, PResult, error::ParseError};

pub trait ParserOnce<Input> {
    type Output;
    type Error: ParseError<Input>;

    fn process_once<OM: OutputMode>(
        self,
        input: Input,
    ) -> PResult<OM, Input, Self::Output, Self::Error>;
}

impl<I, O, E: ParseError<I>, F> ParserOnce<I> for F
where
    F: FnOnce(I) -> IResult<I, O, E>,
{
    type Output = O;
    type Error = E;

    fn process_once<OM: OutputMode>(self, i: I) -> PResult<OM, I, Self::Output, Self::Error> {
        let (i, o) = self(i).map_err(|e| match e {
            Err::Incomplete(i) => Err::Incomplete(i),
            Err::Error(e) => Err::Error(OM::Error::bind(|| e)),
            Err::Failure(e) => Err::Failure(e),
        })?;
        Ok((i, OM::Output::bind(|| o)))
    }
}

pub struct MapOnce<F, G> {
    f: F,
    g: G,
}

impl<I, O2, F, G> ParserOnce<I> for MapOnce<F, G>
where
    F: ParserOnce<I>,
    G: FnOnce(<F as ParserOnce<I>>::Output) -> O2,
{
    type Output = O2;

    type Error = <F as ParserOnce<I>>::Error;

    fn process_once<OM: OutputMode>(self, input: I) -> PResult<OM, I, Self::Output, Self::Error> {
        match self.f.process_once::<OM>(input) {
            Ok((i, o)) => Ok((i, OM::Output::map(o, |o| -> O2 { (self.g)(o) }))),
            Err(e) => Err(e),
        }
    }
}

pub struct OrOnce<P1, P2> {
    first: P1,
    second: P2,
}

impl<I, P1, P2, O, E> ParserOnce<I> for OrOnce<P1, P2>
where
    P1: ParserOnce<I, Output = O, Error = E>,
    P2: ParserOnce<I, Output = O, Error = E>,
    E: ParseError<I>,
    I: Clone,
{
    type Output = O;

    type Error = E;

    fn process_once<OM: OutputMode>(self, input: I) -> PResult<OM, I, Self::Output, Self::Error> {
        let Self { first, second } = self;
        match first.process_once::<OM>(input.clone()) {
            Err(Err::Error(e1)) => match second.process_once::<OM>(input) {
                Err(Err::Error(e2)) => {
                    Err(Err::Error(OM::Error::combine(e1, e2, |e1, e2| e1.or(e2))))
                }
                res => res,
            },
            res => res,
        }
    }
}
