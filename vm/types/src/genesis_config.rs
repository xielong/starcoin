// Copyright (c) The Starcoin Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::move_resource::MoveResource;
use anyhow::{format_err, Result};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fmt::{self, Formatter};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Ord, Serialize)]
pub enum StdlibVersion {
    Latest,
    Version(VersionNumber),
}

type VersionNumber = u64;

impl StdlibVersion {
    pub fn new(version: u64) -> Self {
        StdlibVersion::Version(version)
    }

    pub fn as_string(&self) -> String {
        match self {
            StdlibVersion::Latest => "latest".to_string(),
            StdlibVersion::Version(version) => format!("{}", version),
        }
    }

    pub fn version(&self) -> u64 {
        match self {
            StdlibVersion::Latest => 0,
            StdlibVersion::Version(version) => *version,
        }
    }
}

impl Default for StdlibVersion {
    fn default() -> Self {
        StdlibVersion::Latest
    }
}

impl FromStr for StdlibVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "latest" => Ok(StdlibVersion::Latest),
            s => Ok(Self::new(s.parse()?)),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Serialize,
    IntoPrimitive,
    TryFromPrimitive,
)]
#[repr(u8)]
#[serde(tag = "type")]
pub enum ConsensusStrategy {
    Dummy = 0,
    Argon = 1,
    Keccak = 2,
    CryptoNight = 3,
}

impl ConsensusStrategy {
    pub fn value(self) -> u8 {
        self.into()
    }
}

impl Default for ConsensusStrategy {
    fn default() -> Self {
        ConsensusStrategy::Dummy
    }
}

impl fmt::Display for ConsensusStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsensusStrategy::Dummy => write!(f, "dummy"),
            ConsensusStrategy::Argon => write!(f, "argon"),
            ConsensusStrategy::Keccak => write!(f, "keccak"),
            ConsensusStrategy::CryptoNight => write!(f, "cryptonight"),
        }
    }
}

impl FromStr for ConsensusStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dummy" => Ok(ConsensusStrategy::Dummy),
            "argon" => Ok(ConsensusStrategy::Argon),
            "keccak" => Ok(ConsensusStrategy::Keccak),
            "cryptonight" => Ok(ConsensusStrategy::CryptoNight),
            s => Err(format_err!("Unknown ConsensusStrategy: {}", s)),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct ChainId {
    id: u8,
}

impl ChainId {
    pub fn new(id: u8) -> Self {
        Self { id }
    }

    pub fn id(self) -> u8 {
        self.id
    }

    pub fn test() -> Self {
        ChainId::new(255)
    }
}

impl fmt::Display for ChainId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl FromStr for ChainId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id: u8 = s.parse()?;
        Ok(ChainId::new(id))
    }
}

impl From<u8> for ChainId {
    fn from(id: u8) -> Self {
        Self::new(id)
    }
}

#[allow(clippy::from_over_into)]
impl Into<u8> for ChainId {
    fn into(self) -> u8 {
        self.id
    }
}

impl MoveResource for ChainId {
    const MODULE_NAME: &'static str = "ChainId";
    const STRUCT_NAME: &'static str = "ChainId";
}
