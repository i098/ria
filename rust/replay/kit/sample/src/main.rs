// Ria original, no Ardia source.

mod find;

use std::error::Error;
use std::io;

fn main() -> Result<(), Box<dyn Error>> {
    let actions: Vec<find::Action> = serde_json::from_reader(io::stdin().lock())?;
    serde_json::to_writer(io::stdout().lock(), &find::replay(&actions))?;
    Ok(())
}
