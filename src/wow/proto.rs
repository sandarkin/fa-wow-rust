use serde::{Deserialize, Serialize};

pub const CHALLENGE_SIZE: usize = 16;
pub const SOLUTION_SIZE: usize = 16;
pub const SOLUTION_STATE_SIZE: usize = 4;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Challenge {
    pub difficulty: u8,
    pub value: [u8; CHALLENGE_SIZE],
}

pub type ChallengeSolution = [u8; SOLUTION_SIZE];

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub enum SolutionState {
    Accepted,
    Rejected,
}
