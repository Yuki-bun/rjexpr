use nom::{
    Err, IResult, Needed, Parser,
    character::{char, complete::space0},
    error::ParseError,
};

pub struct CommaSeparated<'a, P, E> {
    input: &'a str,
    parser: P,
    state: State<E>,
}

enum State<E> {
    First,
    Rest,
    Done,
    Failure(E),
    Incomplete(Needed),
}

impl<E> State<E> {
    fn from_err(err: Err<E>) -> Self {
        match err {
            Err::Incomplete(i) => Self::Incomplete(i),
            Err::Error(_) => Self::Done,
            Err::Failure(e) => Self::Failure(e),
        }
    }
}

impl<'a, P, E> CommaSeparated<'a, P, E> {
    pub fn finish(self) -> IResult<&'a str, (), E> {
        match self.state {
            State::First | State::Rest | State::Done => Ok((self.input, ())),
            State::Failure(e) => Err(Err::Failure(e)),
            State::Incomplete(i) => Err(Err::Incomplete(i)),
        }
    }
}

pub fn comma_separated_iter<'a, P, E>(input: &'a str, parser: P) -> CommaSeparated<'a, P, E> {
    CommaSeparated {
        input,
        parser,
        state: State::First,
    }
}

impl<'a, Output, Error, P> Iterator for CommaSeparated<'a, P, Error>
where
    P: Parser<&'a str, Output = Output, Error = Error>,
    Error: ParseError<&'a str>,
{
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::First => match self.parser.parse(self.input) {
                Ok((i2, item)) => {
                    self.input = i2;
                    self.state = State::Rest;
                    Some(item)
                }
                Result::Err(e) => {
                    self.state = State::from_err(e);
                    None
                }
            },
            State::Rest => {
                let i2 = match char(',').and(space0).parse(self.input) {
                    Ok((i2, _)) => i2,
                    Err(e) => {
                        self.state = State::from_err(e);
                        return None;
                    }
                };
                match self.parser.parse(i2) {
                    Ok((i3, item)) => {
                        self.input = i3;
                        self.state = State::Done;
                        Some(item)
                    }
                    Err(e) => {
                        self.state = State::from_err(e);
                        None
                    }
                }
            }
            _ => None,
        }
    }
}
