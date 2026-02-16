use lib::{error::EventHandlerError, nightfall_events::NightfallEvent};

// Starknet events currently arrive already-decoded (no calldata fetch path yet).
// This service is a placeholder so we can later plug in real Starknet handling
// without coupling the poller directly to domain logic.
pub async fn process_starknet_event(
    event: NightfallEvent,
) -> Result<(), EventHandlerError> {
    log::info!("process_starknet_event: {event:?}");
    Ok(())
}
