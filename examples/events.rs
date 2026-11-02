//! cargo run -p nous-hsr --example events -- --smoke
//! Without --smoke: waits until Enter; no game connection is required.
use nous_hsr::Sdk;
fn main() -> Result<(), String> {
    // An ephemeral store: this example never touches application account files.
    let sdk = Sdk::new(None)?;
    let events = sdk.subscribe();
    println!("initial state: {:?}", sdk.read());
    sdk.start_capture()?;
    println!("capture status: {:?}", sdk.status());
    let printer = std::thread::spawn(move || {
        while let Ok(event) = events.recv() {
            println!("{}: {}", event.name, event.payload);
        }
    });
    if !std::env::args().any(|a| a == "--smoke") {
        println!("Waiting for login. Press Enter to stop.");
        std::io::stdin()
            .read_line(&mut String::new())
            .map_err(|e| e.to_string())?;
    }
    sdk.stop_capture()?;
    println!("final state: {:?}", sdk.read());
    drop(sdk);
    printer
        .join()
        .map_err(|_| "event printer panicked".to_string())?;
    Ok(())
}
