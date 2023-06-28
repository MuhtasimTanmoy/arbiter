use std::fs;
use std::io::Write;
use std::path::Path;
use chrono::{DateTime, Utc, Datelike};


pub(crate) fn create_simulation(simulation_name: &str) -> std::io::Result<()> {
    let now: DateTime<Utc> = Utc::now();

    let main = r#"fn main() { 
        simulation::run();
}"#;

    // TODO 
    let toml = format!(r#"[package]
name = "{} Simulation"
version = "0.1.0"
edition = "{}"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html
[dependencies]"#, simulation_name, now.year());

    let mod_rs = r#"pub async fn run(
    price_process: PriceProcess,
    output_storage: OutputStorage,
    label: usize,
) -> Result<(), Box<dyn Error>> {
    let _start = Instant::now();

    // Create a `SimulationManager` that runs simulations in their `SimulationEnvironment`.
    let mut manager = SimulationManager::new();

    // Run the startup script
    startup::run(&mut manager)?;
    let arbitrageur = manager.agents.get("arbitrageur").unwrap();
    let admin = manager.agents.get("admin").unwrap();

    // Intialize the arbitrageur with the prices from the two exchanges.
    let arbitrageur = match arbitrageur {
        AgentType::SimpleArbitrageur(base_arbitrageur) => base_arbitrageur,
        _ => panic!(),
    };
    let liquid_exchange = manager
    .deployed_contracts
    .get("liquid_exchange_xy")
    .unwrap();
let result = arbitrageur.call(liquid_exchange, "price", vec![])?;
assert!(result.is_success());

let liquid_exchange_xy_price: U256 =
    liquid_exchange.decode_output("price", unpack_execution(result)?)?;
    // etc
    
    todo()
}"#;

    let run = r#"pub(crate) fn run(manager: &mut SimulationManager) -> Result<(), Box<dyn Error>> {
        let weth_address = manager.deployed_contracts.get("weth").unwrap().address;
        deploy_contracts(manager, weth_address)?;
        let liquid_exchange_xy = manager
            .deployed_contracts
            .get("liquid_exchange_xy")
            .unwrap();
        let address = B160::from_low_u64_be(2);
        let event_filters = vec![SimulationEventFilter::new(
            liquid_exchange_xy,
            "PriceChange",
        )];
        let arbitrageur = SimpleArbitrageur::new(
            "arbitrageur",
            event_filters,
            U256::from(997_000_000_000_000_000u128).into(),
        );
        manager
            .activate_agent(AgentType::SimpleArbitrageur(arbitrageur), address)
            .unwrap();
    
        mint(
            &manager.deployed_contracts,
            manager.agents.get("admin").unwrap(),
            manager.agents.get("arbitrageur").unwrap(),
        )?;
        approve(
            manager.agents.get("admin").unwrap(),
            manager.agents.get("arbitrageur").unwrap(),
            &manager.deployed_contracts,
        )?;
    
        allocate(
            manager.agents.get("admin").unwrap(),
            &manager.deployed_contracts,
        )?;
    
        Ok()
    }
    pub fn deploy() {
    todo()
    }
    
    pub fn mint() {
    todo()
    }

    pub fn approve() {
    todo()
    }

    pub fn allocate() {
    todo()
    }
    "#;
    // Create a directory
    fs::create_dir_all("arbiter")?;

    // Create a subdirectory


    let src_path = Path::new("arbiter").join("src");
    fs::create_dir_all(&src_path)?;

    let bindings_path = src_path.join("bindings");
    fs::create_dir_all(bindings_path)?;

    let simulations_path = src_path.join("simulations");
    fs::create_dir_all(&simulations_path)?;

    let sim = simulations_path.join(simulation_name);
    fs::create_dir_all(&sim)?;


    // Create a file in the subdirectory
    let file_path = Path::new("arbiter").join("Cargo.toml");
    let mut file = fs::File::create(file_path)?;
    write!(file, "{}", toml)?;

    let file_path = sim.join("mod.rs");
    let mut file = fs::File::create(file_path)?;
    write!(file, "{}", mod_rs)?;

    let file_path = sim.join("startup.rs");
    let mut file = fs::File::create(file_path)?;
    write!(file, "{}", run)?;

    let file_path = sim.join("arbitrage.rs");
    fs::File::create(file_path)?;

    let file_path = src_path.join("main.rs");
    let mut file = fs::File::create(file_path)?;
    write!(file, "{}", main)?;

    Ok(())
}

#[test]
fn main() {
    create_simulation("portfolio").unwrap();
}


