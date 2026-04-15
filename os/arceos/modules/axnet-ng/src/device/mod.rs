use ax_driver::prelude::{DevResult, NetIrqEvents, NetLinkState, NetPollStatus};
use smoltcp::{storage::PacketBuffer, time::Instant, wire::IpAddress};

mod ethernet;
mod loopback;
#[cfg(feature = "vsock")]
mod vsock;

pub use ethernet::*;
pub use loopback::*;
#[cfg(feature = "vsock")]
pub use vsock::*;

pub trait Device: Send + Sync {
    fn name(&self) -> &str;

    fn irq_num(&self) -> Option<usize> {
        None
    }

    fn set_irq_enabled(&mut self, _enabled: bool) {}

    fn handle_irq(&mut self) -> NetIrqEvents {
        NetIrqEvents::empty()
    }

    fn poll_rx(
        &mut self,
        budget: usize,
        buffer: &mut PacketBuffer<()>,
        timestamp: Instant,
    ) -> DevResult<NetPollStatus>;

    fn poll_tx(&mut self, budget: usize) -> DevResult<NetPollStatus> {
        let _ = budget;
        Ok(NetPollStatus::default())
    }

    #[allow(dead_code)]
    fn link_state(&self) -> NetLinkState {
        NetLinkState::Unknown
    }

    fn recv(&mut self, buffer: &mut PacketBuffer<()>, timestamp: Instant) -> bool;
    /// Sends a packet to the next hop.
    ///
    /// Returns `true` if this operation resulted in the readiness of receive
    /// operation. This is true for loopback devices and can be used to speed
    /// up packet processing.
    fn send(&mut self, next_hop: IpAddress, packet: &[u8], timestamp: Instant) -> bool;
}
