#![warn(missing_docs)]

//! ## Agent trait and associated functionality
//!
//! An abstract representation of an agent on the EVM, to be used in simulations.
//! Some examples of agents are market makers or arbitrageurs.
//! All agents must implement the [`Agent`] traits through the [`AgentType`] enum.
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    thread,
};

use bytes::Bytes;
use crossbeam_channel::Receiver;
use ethers::types::H256;
use revm::primitives::{Address, ExecutionResult, Log, TransactTo, TxEnv, B160, U256};

use crate::{
    contract::{IsDeployed, SimulationContract},
    environment::SimulationEnvironment,
};

pub mod simple_arbitrageur;
pub mod user;

#[derive(Debug)]
/// Error type for the simulation manager.
pub struct AgentError(String);

impl Error for AgentError {}

impl Display for AgentError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.0)
    }
}

/// An agent is an entity that can interact with the simulation environment.
/// Agents can be various entities such as users, market makers, arbitrageurs, etc.
/// Only the [`User`] agent is currently implemented.
pub enum AgentType {
    /// A [`User`] is the most basic agent that can interact with the simulation environment.
    User,
    /// A [`SimpleArbitrageur`] is an agent that can perform arbitrage between two pools.
    SimpleArbitrageur,
}

/// Describes the gas settings for a transaction.
pub struct TransactSettings {
    /// Gas limit for the transaction for a simulation.
    pub gas_limit: u64,
    /// Gas limit for the transaction for a simulation.
    pub gas_price: U256,
}

/// Basic traits that every `Agent` must implement in order to properly interact with an EVM.
pub trait Agent: Sync {
    /// Returns the name of the agent.
    fn name(&self) -> String;
    /// Returns the address of the agent.
    fn address(&self) -> Address;
    /// Returns the transaction settings of the agent.
    fn transact_settings(&self) -> &TransactSettings;
    /// The event's channel receiver for the agent.
    fn receiver(&self) -> Receiver<Vec<Log>>;
    /// Gets the event filter for the [`Agent`]
    fn event_filters(&self) -> Vec<SimulationEventFilter>;

    /// Used to allow agents to make a generic call a specific smart contract.
    fn call_contract(
        &self,
        simulation_environment: &mut SimulationEnvironment,
        contract: &SimulationContract<IsDeployed>,
        call_data: Bytes,
        value: U256,
    ) -> ExecutionResult {
        let tx = self.build_call_transaction(contract.address, call_data, value);
        simulation_environment.execute(tx)
    }

    /// A constructor to build a `TxEnv` for an agent (uses agent data like `address` and `TransactSettings`).
    fn build_call_transaction(
        &self,
        receiver_address: B160,
        call_data: Bytes,
        value: U256,
    ) -> TxEnv {
        TxEnv {
            caller: self.address(),
            gas_limit: self.transact_settings().gas_limit,
            gas_price: self.transact_settings().gas_price,
            gas_priority_fee: None,
            transact_to: TransactTo::Call(receiver_address),
            value,
            data: call_data,
            chain_id: None,
            nonce: None,
            access_list: Vec::new(),
        }
    }

    // TODO: May be defunct to read logs now
    /// Gets the most current event (which is all that is stored in the event buffer).
    fn read_logs(&self) -> Result<Vec<Log>, AgentError> {
        match self.receiver().recv() {
            Ok(logs) => Ok(logs),
            Err(e) => Err(AgentError(format!("Error reading logs for agent: {}", e))),
        }
    }

    // TODO: This isn't totally tested yet, but it comes from the `test_event_monitoring()` function
    /// Monitor events for the agent.
    fn monitor_events(&self) {
        let receiver = self.receiver();
        let event_filters = self.event_filters();
        thread::spawn(move || {
            while let Ok(logs) = receiver.recv() {
                let filtered_logs = filter_events(event_filters.clone(), logs);
                println!("Filtered logs are: {:#?}", filtered_logs);
            }
        });
    }
}

#[derive(Debug, Clone)]
/// The filtering implmentation to be used with the [`Agent`] trait.
pub struct SimulationEventFilter {
    /// The addresses to filter for.
    pub address: B160,
    /// The event names to filter for.
    pub topic: H256,
}

/// Creates a filter for the agent to use to filter out events.
pub fn create_filter(
    contract: &SimulationContract<IsDeployed>,
    event_name: &str,
) -> SimulationEventFilter {
    // let abi = contracts[0].base_contract.abi();
    SimulationEventFilter {
        address: contract.address,
        topic: contract
            .base_contract
            .abi()
            .event(event_name)
            .unwrap()
            .signature(),
    }
}

/// Used to allow agents to filter out the events they choose to monitor.
fn filter_events(simulation_filters: Vec<SimulationEventFilter>, logs: Vec<Log>) -> Vec<Log> {
    if simulation_filters.is_empty() {
        return logs;
    }

    let mut events = vec![];

    for log in logs {
        for event_filter in simulation_filters.iter() {
            if event_filter.address == log.address && event_filter.topic == log.topics[0].into()
            {
                events.push(log.clone());
                break;
            }
        }
    }

    events
}

// #[cfg(test)]
// mod tests {

//     use std::error::Error;

//     use bindings::{arbiter_token, writer};
//     use revm::primitives::{ruint::Uint, B160};

//     use crate::{
//         agent::{create_filter, AgentType},
//         contract::SimulationContract,
//         manager::SimulationManager,
//     };

//     #[test]
//     fn agent_event_filter_through() -> Result<(), Box<dyn Error>> {
//         // Set up the execution manager and a user address.
//         let mut manager = SimulationManager::default();

//         // Create writer contract.
//         let writer =
//             SimulationContract::new(writer::WRITER_ABI.clone(), writer::WRITER_BYTECODE.clone());

//         // Deploy the writer.
//         let test_string = "Hello, world..?".to_string();
//         let writer = writer.deploy(
//             &mut manager.environment,
//             manager.agents.get("admin").unwrap(),
//             test_string,
//         );

//         // Create two agents with a filter.
//         manager.create_agent("alice", B160::from_low_u64_be(2), AgentType::User, None)?;

//         let event_filters = vec![create_filter(&writer, "WasWritten")];
//         manager.create_agent(
//             "bob",
//             B160::from_low_u64_be(3),
//             AgentType::User,
//             Some(event_filters),
//         )?;

//         let alice = manager.agents.get("alice").unwrap();
//         let bob = manager.agents.get("bob").unwrap();

//         println!("Alice's event filter: {:#?}", alice.event_filters());
//         println!("Bob's event filter: {:#?}", bob.event_filters());

//         // Make calls that alice and bob won't filter out.
//         let new_string = "Hello, world!".to_string();
//         let call_data = writer.encode_function("echoString", new_string)?;
//         manager.agents.get("admin").unwrap().call_contract(
//             &mut manager.environment,
//             &writer,
//             call_data,
//             Uint::ZERO,
//         );
//         // Test that the alice doesn't filter out these logs.
//         let unfiltered_events = alice.read_logs()?;
//         let filtered_events = alice.filter_events(unfiltered_events.clone());
//         println!(
//             "The filtered events for alice on the first call are: {:#?}",
//             &filtered_events
//         );
//         assert_eq!(filtered_events, unfiltered_events);
//         // Test that bob filters out these logs.
//         let unfiltered_events = bob.read_logs()?;
//         let filtered_events = bob.filter_events(unfiltered_events.clone());
//         println!(
//             "The filtered events for bob on the first call are: {:#?}",
//             &filtered_events
//         );

//         // Also try to filter out a different address.

//         Ok(())
//     }

//     #[test]
//     fn agent_event_filter_out() -> Result<(), Box<dyn Error>> {
//         // Set up the execution manager and a user address.
//         let mut manager = SimulationManager::default();

//         // Create writer contract.
//         let writer =
//             SimulationContract::new(writer::WRITER_ABI.clone(), writer::WRITER_BYTECODE.clone());
//         let test_string = "Hello, world..?".to_string();
//         let writer = writer.deploy(
//             &mut manager.environment,
//             manager.agents.get("admin").unwrap(),
//             test_string,
//         );

//         // Create writer contract.
//         let arbt = SimulationContract::new(
//             arbiter_token::ARBITERTOKEN_ABI.clone(),
//             arbiter_token::ARBITERTOKEN_BYTECODE.clone(),
//         );
//         let arbt = arbt.deploy(
//             &mut manager.environment,
//             manager.agents.get("admin").unwrap(),
//             ("ArbiterToken".to_string(), "ARBT".to_string(), 18_u8),
//         );

//         // Create agent with a filter.
//         let event_filters = vec![create_filter(&arbt, "Approval")];
//         manager.create_agent(
//             "alice",
//             B160::from_low_u64_be(2),
//             AgentType::User,
//             Some(event_filters),
//         )?;
//         let alice = manager.agents.get("alice").unwrap();

//         println!("Alice's event filter: {:#?}", alice.event_filters());

//         // Make calls that alice and bob won't filter out.
//         let new_string = "Hello, world!".to_string();
//         let call_data = writer.encode_function("echoString", new_string)?;
//         manager.agents.get("admin").unwrap().call_contract(
//             &mut manager.environment,
//             &writer,
//             call_data,
//             Uint::ZERO,
//         );
//         // Test that the alice doesn't filter out these logs.
//         let unfiltered_events = alice.read_logs()?;
//         let filtered_events = alice.filter_events(unfiltered_events.clone());
//         println!(
//             "The filtered events for alice on the first call are: {:#?}",
//             &filtered_events
//         );
//         assert_eq!(filtered_events, vec![]);
//         Ok(())
//     }
// }
