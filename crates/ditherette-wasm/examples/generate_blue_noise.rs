//! Offline reference construction, never part of processor initialization.
#[path = "../src/spec/dither/blue_noise/generator.rs"]
mod generator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let construction = generator::generate()?;
    println!("{}", serde_json::to_string_pretty(&construction)?);
    if !construction.passes {
        return Err("blue-noise construction failed preregistered quality gates".into());
    }
    Ok(())
}
