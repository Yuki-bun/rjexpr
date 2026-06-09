use nom::{Err as NErr, Mode, OutputMode, Parser};

#[derive(Debug, Clone)]
pub struct DebugP<'a, P> {
    parser: P,
    name: &'a str,
}

impl<'a, I, P> Parser<I> for DebugP<'a, P>
where
    P: Parser<I>,
    P::Output: std::fmt::Debug,
    P::Error: std::error::Error,
    I: std::fmt::Debug + Clone,
{
    type Output = P::Output;

    type Error = P::Error;

    fn process<OM: OutputMode>(
        &mut self,
        input: I,
    ) -> nom::PResult<OM, I, Self::Output, Self::Error> {
        let result = (self.parser).process::<OM>(input.clone());
        match result {
            Ok((rest, output)) => {
                let output = OM::Output::map(output, |output| {
                    print!("{}: {:?} => ", self.name, input);
                    println!("({:?}, {:?})", rest, output);
                    output
                });
                Ok((rest, output))
            }
            Err(NErr::Error(err)) => {
                let err = OM::Error::map(err, |err| {
                    print!("{}: {:?} => ", self.name, input);
                    println!("failed: {err}");
                    err
                });
                Err(NErr::Error(err))
            }
            Err(NErr::Failure(err)) => {
                print!("{}: {:?} => ", self.name, input);
                println!("Failure: {err}");
                Err(NErr::Failure(err))
            }
            Err(NErr::Incomplete(err)) => {
                print!("{}: {:?} => ", self.name, input);
                println!("Incomplete: {err:?}");
                Err(NErr::Incomplete(err))
            }
        }
    }
}

pub fn dbg_p<I, P>(parser: P, name: &str) -> DebugP<'_, P>
where
    P: Parser<I>,
    P::Output: std::fmt::Debug,
    P::Error: std::error::Error,
    I: std::fmt::Debug + Clone,
{
    DebugP { parser, name }
}

#[macro_export]
macro_rules! dbgp {
    ($parser:ident) => {{
        let name = stringify!($parser);
        dbg_p($parser, name)
    }};

    ($parser:expr, $desc:literal) => {{ dbg_p($parser, $desc) }};
}
