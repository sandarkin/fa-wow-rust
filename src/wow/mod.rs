mod proto;
use anyhow::Result;
use proto::{
    Challenge, ChallengeSolution, SolutionState, CHALLENGE_SIZE, SOLUTION_SIZE, SOLUTION_STATE_SIZE,
};
use rand::seq::SliceRandom;
use rand::Rng;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::mem::size_of;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;


pub const DEFAULT_DIFFICULTY: u8 = 4;

impl Challenge {
    pub fn new(difficulty: u8) -> Self {
        let value = rand::thread_rng().gen::<[u8; CHALLENGE_SIZE]>();
        Challenge { difficulty, value }
    }
}

impl Default for Challenge {
    fn default() -> Self {
        Self::new(DEFAULT_DIFFICULTY)
    }
}

pub struct ChallengeSolver<'a> {
    challenge: &'a Challenge,
    precomputed_hash: Sha256,
}

pub struct SolvingResult {
    pub solution: ChallengeSolution,
    pub hashes_tried: u128,
}

impl<'a> ChallengeSolver<'a> {
    pub fn new(challenge: &'a Challenge) -> Self {
        let mut precomputed_hash = Sha256::new();
        precomputed_hash.update(challenge.value);
        Self {
            challenge,
            precomputed_hash,
        }
    }

    pub fn is_valid_solution(&self, solution: &ChallengeSolution) -> bool {
        let mut hasher = self.precomputed_hash.clone();
        hasher.update(solution);

        let hash = hasher.finalize();
        let mut leading_zeros = 0;

        for c in hash.iter().take(self.challenge.difficulty as usize / 2 + 1) {
            if c >> 4 == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
            if c & 0xF == 0 {
                leading_zeros += 1;
            } else {
                break;
            }
        }

        log::debug!("hash: {:x}", hash);
        leading_zeros >= self.challenge.difficulty
    }

    pub fn solve(&self) -> SolvingResult {
        let mut rng = rand::thread_rng();
        let mut hashes_tried: u128 = 0;
        loop {
            let solution = rng.gen::<ChallengeSolution>();
            hashes_tried += 1;
            if self.is_valid_solution(&solution) {
                return SolvingResult {
                    solution,
                    hashes_tried,
                };
            }
        }
    }
}

pub struct Transport<T: Read + Write> {
    c: T,
}

impl<T> Transport<T>
where
    T: Read + Write,
{
    pub fn new(c: T) -> Self {
        Self { c }
    }

    pub fn send<V>(&mut self, value: &V) -> Result<()>
    where
        V: Serialize,
    {
        self.c.write_all(&bincode::serialize(value)?)?;
        Ok(())
    }

    pub fn send_with_varsize<V>(&mut self, value: &V) -> Result<()>
    where
        V: Serialize,
    {
        let data = bincode::serialize(value)?;
        let len = bincode::serialize(&data.len())?;
        self.c.write_all(&len)?;
        self.c.write_all(&data)?;
        Ok(())
    }

    pub fn receive<R>(&mut self, size: usize) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let mut buf: Vec<u8> = vec![0; size];
        self.c.read_exact(&mut buf)?;
        let result: R = bincode::deserialize(&buf)?;
        Ok(result)
    }

    pub fn receive_varsize<R>(&mut self) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let msg_size: usize = self.receive(size_of::<usize>())?;
        self.receive::<R>(msg_size)
    }
}

struct Connection {
    stream: TcpStream,
    state: ClientState,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            state: ClientState::Initial,
        }
    }
}

pub struct Server {
    responses: Vec<String>,
    difficulty: u8,
}

enum ClientState {
    Initial,
    ChallengeSent,
}

impl<'a> Server {
    pub fn new(responses: Vec<String>) -> Result<Self, Box<dyn Error>> {
        if responses.is_empty() {
            return Err("responses must not be empty".into());
        }
        Ok(Server {
            responses,
            difficulty: DEFAULT_DIFFICULTY,
        })
    }

    pub fn new_from_file(filename: &str) -> Result<Self, Box<dyn Error>> {
        let mut responses = Vec::<String>::new();
        for val in fs::read_to_string(filename)?.split("\n") {
            responses.push(val.trim_matches(&['\r', '\n', ' '][..]).into());
        }
        Self::new(responses)
    }

    pub fn set_difficulty(&mut self, difficulty: u8) {
        self.difficulty = difficulty;
    }

    pub fn run(self, address: &'a str) -> Result<(), Box<dyn Error>> {
        Arc::new(self).run_listener(address)?;
        Ok(())
    }

    fn run_listener(self: Arc<Self>, address: &'a str) -> Result<(), Box<dyn Error>> {
        let listener = TcpListener::bind(address)?;
        log::info!("server started on {}", address);

        for stream in listener.incoming() {
            let server_clone = self.clone();
            match stream {
                Ok(stream) => {
                    log::info!("receive connection from {}", stream.peer_addr()?);
                    thread::spawn(move || {
                        let mut conn = Connection::new(stream);
                        if let Err(e) = server_clone.handle_connection(&mut conn) {
                            eprintln!("refuse connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    log::error!("TCP connection error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(&self, conn: &mut Connection) -> Result<()> {
        let mut client = Transport::new(conn.stream.try_clone()?);
        let challenge = Challenge::new(self.difficulty);
        let solver = ChallengeSolver::new(&challenge);

        loop {
            match conn.state {
                ClientState::Initial => {
                    client.send(&challenge)?;
                    conn.state = ClientState::ChallengeSent;
                }
                ClientState::ChallengeSent => {
                    let solution: ChallengeSolution = client.receive(SOLUTION_SIZE)?;
                    log::info!("receive connection");

                    if solver.is_valid_solution(&solution) {
                        log::info!("is valid proof");
                        client.send(&SolutionState::Accepted)?;
                        client.send_with_varsize(self.random_response())?;
                    } else {
                        client.send(&SolutionState::Rejected)?;
                        log::error!("is not valid proof");
                    }

                    conn.stream.shutdown(Shutdown::Both)?;
                    break;
                }
            }
        }

        Ok(())
    }

    fn random_response(&self) -> &String {
        self.responses.choose(&mut rand::thread_rng()).unwrap()
    }
}

pub struct Client<'a> {
    address: &'a str,
}

impl<'a> Client<'a> {
    pub fn new(address: &'a str) -> Self {
        Self { address }
    }

    pub fn get_response(&self) -> Result<String, Box<dyn Error>> {
        let stream = TcpStream::connect(self.address)?;
        let mut server = Transport::new(stream.try_clone()?);

        let challenge: Challenge = server.receive(size_of::<Challenge>())?;
        log::info!("received challenge with difficulty {}", challenge.difficulty);

        let solver = ChallengeSolver::new(&challenge); // precomputes a hash to increase the performance
        let result = solver.solve();
        log::info!("proof calculated with {} attempts", result.hashes_tried);
        server.send(&result.solution)?;

        let result = match server.receive::<SolutionState>(SOLUTION_STATE_SIZE)? {
            SolutionState::Accepted => {
                log::info!("is valid proof");
                let server_msg: String = server.receive_varsize()?;
                Ok(server_msg)
            }
            SolutionState::Rejected => Err("is not valid proof".into()),
        };
        let _ = stream.shutdown(Shutdown::Both);
        result
    }
}
