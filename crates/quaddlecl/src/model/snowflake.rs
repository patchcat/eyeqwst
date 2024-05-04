use chrono::{DateTime, NaiveDate, TimeDelta, Utc};

use crate::private::Sealed;



/// The Quaddle epoch.
pub const EPOCH: DateTime<Utc> = {
    // we have to use let-else because const .unwrap() has not been stabilized
    let Some(naived) = NaiveDate::from_ymd_opt(2024, 1, 1) else {
        panic!("Failed to convert epoch to NaiveDate")
    };

    let Some(naivedt) = naived.and_hms_opt(0, 0, 0) else {
        panic!("Failed to convert NaiveDate to NaiveDateTime")
    };

    naivedt.and_utc()
};

const TS_OFFSET: u64 = 22;

/// Marker trait for newtypes over snowflakes
pub trait Snowflake: Into<u64> + Clone + Sealed
where u64: Into<Self> {
    /// Gets the timestamp of the snowflake. Should never fail.
    fn timestamp(self) -> DateTime<Utc> {
        EPOCH + TimeDelta::milliseconds(i64::try_from(self.into() >> TS_OFFSET).unwrap())
    }
}

macro_rules! newtype_sf_impl {
    ($ty:ty) => {
        impl crate::private::Sealed for $ty {}

        impl From<$ty> for u64 {
            fn from(sf: $ty) -> u64 {
                sf.0
            }
        }

        impl From<u64> for $ty {
            fn from(sf: u64) -> Self {
                Self(sf)
            }
        }

        impl crate::model::snowflake::Snowflake for $ty {}
    }
}

macro_rules! extra_sf_impls {
    ($ty:ty) => {
        impl From<$ty> for ::chrono::DateTime<::chrono::Utc> {
            fn from(sf: $ty) -> Self {
                use crate::model::snowflake::Snowflake;
                sf.timestamp()
            }
        }

        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>)
                   -> Result<(), ::std::fmt::Error> {
                write!(f, "{id}",
                       id = self.0)
            }
        }

        impl ::std::str::FromStr for $ty {
            type Err = <u64 as ::std::str::FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(u64::from_str(s)?))
            }
        }
    }
}

pub(crate) use newtype_sf_impl;
pub(crate) use extra_sf_impls;

#[cfg(test)]
mod tests {
    use chrono::{Datelike, Timelike};

    use super::*;

    #[derive(Clone, Copy)]
    struct MeowId(u64);

    newtype_sf_impl!(MeowId);
    extra_sf_impls!(MeowId);

    #[test]
    fn test_timestamp() {
        let ts = MeowId(175928847299117063);
        let dt: DateTime<Utc> = ts.into();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 4);
        assert_eq!(dt.day(), 30);
        assert_eq!(dt.hour(), 11);
        assert_eq!(dt.minute(), 18);
        assert_eq!(dt.second(), 25);
    }
}
