//! Common traits and types for network device (NIC) drivers.

#![no_std]
#![cfg_attr(doc, feature(doc_cfg))]

extern crate alloc;

use core::cmp;

#[cfg(feature = "fxmac")]
/// fxmac driver for PhytiumPi
pub mod fxmac;
#[cfg(feature = "ixgbe")]
/// ixgbe NIC device driver.
pub mod ixgbe;

#[doc(no_inline)]
pub use ax_driver_base::{BaseDriverOps, DevError, DevResult, DeviceType};

mod net_buf;
pub use self::net_buf::{NetBuf, NetBufBox, NetBufPool, NetBufPtr};
use bitflags::bitflags;

/// The ethernet address of the NIC (MAC address).
pub struct EthernetAddress(pub [u8; 6]);

bitflags! {
    /// Event bits returned from the IRQ top half of a NIC.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct NetIrqEvents: u32 {
        /// Receive work is pending.
        const RX = 1 << 0;
        /// Transmit completion work is pending.
        const TX = 1 << 1;
        /// Link state changed.
        const LINK = 1 << 2;
        /// Device error needs service.
        const ERR = 1 << 3;
    }
}

/// Link state reported by a NIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetLinkState {
    /// The link is up.
    Up,
    /// The link is down.
    Down,
    /// The driver cannot currently determine the state.
    Unknown,
}

/// Result of one bottom-half polling round.
#[derive(Debug, Clone, Copy, Default)]
pub struct NetPollStatus {
    /// Total amount of work completed during this round.
    pub work_done: usize,
    /// Number of RX packets drained.
    pub rx_done: usize,
    /// Number of TX completions reclaimed.
    pub tx_done: usize,
    /// Whether more RX work is pending.
    pub more_rx: bool,
    /// Whether more TX work is pending.
    pub more_tx: bool,
    /// Whether link state changed while polling.
    pub link_changed: bool,
}

/// Operations that require a network device (NIC) driver to implement.
pub trait NetDriverOps: BaseDriverOps {
    /// The ethernet address of the NIC.
    fn mac_address(&self) -> EthernetAddress;

    /// Whether can transmit packets.
    fn can_transmit(&self) -> bool;

    /// Whether can receive packets.
    fn can_receive(&self) -> bool;

    /// Size of the receive queue.
    fn rx_queue_size(&self) -> usize;

    /// Size of the transmit queue.
    fn tx_queue_size(&self) -> usize;

    /// Gives back the `rx_buf` to the receive queue for later receiving.
    ///
    /// `rx_buf` should be the same as the one returned by
    /// [`NetDriverOps::receive`].
    fn recycle_rx_buffer(&mut self, rx_buf: NetBufPtr) -> DevResult;

    /// Poll the transmit queue and gives back the buffers for previous transmiting.
    /// returns [`DevResult`].
    fn recycle_tx_buffers(&mut self) -> DevResult;

    /// Transmits a packet in the buffer to the network, without blocking,
    /// returns [`DevResult`].
    fn transmit(&mut self, tx_buf: NetBufPtr) -> DevResult;

    /// Receives a packet from the network and store it in the [`NetBuf`],
    /// returns the buffer.
    ///
    /// Before receiving, the driver should have already populated some buffers
    /// in the receive queue by [`NetDriverOps::recycle_rx_buffer`].
    ///
    /// If currently no incomming packets, returns an error with type
    /// [`DevError::Again`].
    fn receive(&mut self) -> DevResult<NetBufPtr>;

    /// Allocate a memory buffer of a specified size for network transmission,
    /// returns [`DevResult`]
    fn alloc_tx_buffer(&mut self, size: usize) -> DevResult<NetBufPtr>;

    /// Enables or disables device interrupts.
    fn set_irq_enabled(&mut self, _enabled: bool) {}

    /// Handles the device IRQ top half and returns pending work bits.
    fn handle_irq(&mut self) -> NetIrqEvents {
        NetIrqEvents::RX | NetIrqEvents::TX
    }

    /// Polls RX packets in the bottom half.
    fn poll_rx(
        &mut self,
        budget: usize,
        sink: &mut dyn FnMut(NetBufPtr) -> DevResult,
    ) -> DevResult<NetPollStatus> {
        let mut status = NetPollStatus::default();
        for _ in 0..budget {
            match self.receive() {
                Ok(buf) => {
                    sink(buf)?;
                    status.work_done += 1;
                    status.rx_done += 1;
                }
                Err(DevError::Again) => return Ok(status),
                Err(err) => return Err(err),
            }
        }
        status.more_rx = self.can_receive();
        Ok(status)
    }

    /// Polls TX completions in the bottom half.
    fn poll_tx(&mut self, budget: usize) -> DevResult<NetPollStatus> {
        let mut status = NetPollStatus::default();
        if budget == 0 {
            return Ok(status);
        }
        self.recycle_tx_buffers()?;
        status.tx_done = cmp::min(1, budget);
        status.work_done = status.tx_done;
        status.more_tx = !self.can_transmit();
        Ok(status)
    }

    /// Returns the current link state.
    fn link_state(&self) -> NetLinkState {
        NetLinkState::Unknown
    }
}
