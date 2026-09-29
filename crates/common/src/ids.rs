//! Strongly typed newtype wrappers for database primary keys.
//!
//! Using newtypes prevents mixing up `UserId` and `TunnelId` at the type level.
//!
//! The database-side mapping is provided by the `admin` crate which depends on
//! `sqlx`; this crate only handles serialization / display so it stays
//! dependency-light.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_newtype {
    ($name:ident, $inner:ty) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub $inner);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<$inner> for $name {
            fn from(v: $inner) -> Self {
                Self(v)
            }
        }
    };
}

id_newtype!(UserId, i64);
id_newtype!(NodeId, i64);
id_newtype!(TunnelId, i64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_display_matches_inner() {
        assert_eq!(UserId(7).to_string(), "7");
    }
}