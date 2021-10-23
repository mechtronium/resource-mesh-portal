use std::str::FromStr;
use nom::error::{context, VerboseError};
use nom::sequence::{tuple, terminated, separated_pair};
use nom::character::complete::digit1;
use nom::bytes::complete::tag;
use anyhow::Error;
use nom::IResult;

pub mod v0_0_1;
pub mod latest;

lazy_static! {
  pub static ref VERSION_RANGE: VersionRange = VersionRange { min: Version{
                                                                     major: 1,
                                                                     minor: 0,
                                                                     patch: 0,
                                                                    },
                                                              max: Version{
                                                                     major: 1,
                                                                     minor: 0,
                                                                     patch: 0,
                                                                    },
    };
}

#[derive(Debug,Clone)]
pub struct Version {
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
}

impl ToString for Version{
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch )
    }
}

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;

fn parse_version(input: &str) -> Res<&str, (usize,usize,usize)> {
    context(
        "version_major_minor_patch",
        tuple((
            terminated(digit1, tag(".")),
            terminated(digit1, tag(".")),
            digit1,
        )),
    )(input)
        .map(|(next_input, res)| {
            (
                next_input,
                (
                    ( res.0.parse().unwrap(),
                        res.1.parse().unwrap(),
                        res.2.parse().unwrap(),
                    )
                ),
            )
        })
}

fn parse_version_range(input: &str) -> Res<&str, ((usize,usize,usize),(usize,usize,usize))> {
    context(
        "version_range",
        separated_pair(parse_version, tag("-"), parse_version),
    )(input)
        .map(|(next_input, (min,max))| {
            (
                next_input,
                (
                    (min,max)
                ),
            )
        })
}

impl FromStr for Version{
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        unimplemented!()
        /*
        let (remaining,version) = parse_version(s.as_str() )?;
        if remaining.len() > 0 {
            let err = format!("leftover characters when attempting to parse version: '{}' from '{}'", remaining, s );
            return Err(anyhow!(err));
        }

        Ok( Self {
            major: version.0,
            minor: version.1,
            patch: version.2,
        })

         */
    }
}

#[derive(Debug,Clone)]
pub struct VersionRange{
   pub min: Version,
   pub max: Version
}

impl ToString for VersionRange{
    fn to_string(&self) -> String {
        format!("{}-{}", self.min.to_string(), self.max.to_string() )
    }
}

impl FromStr for VersionRange {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
unimplemented!();
/*        let (remaining,range) = parse_version_range(s)?;
        if remaining.len() > 0 {
            let err = format!("leftover characters when attempting to parse version range: '{}' from '{}'", remaining, s );
            return Err(anyhow!(err));
        }
        Ok( Self {
            min: Version {
                major: range.0.0,
                minor: range.0.1,
                patch: range.0.2,
            },
            max: Version {
                major: range.1.0,
                minor: range.1.1,
                patch: range.1.2,
            }
        })
 */
    }
}

pub struct VersionNegotiator {

}

impl VersionNegotiator {
    pub fn negotiate( &self, range: String ) -> Result<Box<dyn VersionMigration<v0_0_1::portal::outlet::Frame>>,Error> {
        Err(anyhow!("blah"))
    }
}


pub trait VersionMigration<FRAME> {
    fn migrate( &self, frame: FRAME ) -> Option<Result<Vec<u8>,Error>>;
}


pub struct IdentityVersionMigration {
}

impl VersionMigration<v0_0_1::portal::outlet::Frame> for IdentityVersionMigration {
    fn migrate(&self, frame: v0_0_1::portal::outlet::Frame) -> Option<Result<Vec<u8>,Error>> {
        let result = bincode::serialize(&frame);
        match result {
            Ok(data) => {
                Option::Some(Ok(data))
            }
            Err(err) => {
                Option::Some(Result::Err(anyhow!(err)))
            }
        }
    }
}
