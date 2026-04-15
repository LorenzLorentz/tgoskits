//! [ArceOS](https://github.com/rcore-os/arceos) network module.
//!
//! It provides unified networking primitives for TCP/UDP communication
//! using various underlying network stacks. Currently, only [smoltcp] is
//! supported.
//!
//! # Organization
//!
//! - [`tcp::TcpSocket`]: A TCP socket that provides POSIX-like APIs.
//! - [`udp::UdpSocket`]: A UDP socket that provides POSIX-like APIs.
//!
//! [smoltcp]: https://github.com/smoltcp-rs/smoltcp

#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

mod consts;
mod device;
mod general;
mod listen_table;
/// Socket option types and the [`Configurable`](options::Configurable) trait.
pub mod options;
mod router;
mod service;
mod socket;
pub(crate) mod state;
/// TCP socket implementation.
pub mod tcp;
/// UDP socket implementation.
pub mod udp;
/// Unix domain socket implementation.
pub mod unix;
/// Vsock socket implementation.
#[cfg(feature = "vsock")]
pub mod vsock;
mod wrapper;

use alloc::{boxed::Box, collections::VecDeque, sync::Arc};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use ax_driver::{AxDeviceContainer, prelude::*};
use ax_sync::Mutex;
use ax_task::{
    AxCpuMask, WaitQueue, register_timer_callback, set_current_affinity, spawn_with_name,
};
use axpoll::PollSet;
use smoltcp::wire::{EthernetAddress, Ipv4Address, Ipv4Cidr};
use spin::{Lazy, Once};

pub use self::socket::*;
use self::{
    consts::{GATEWAY, IP, IP_PREFIX},
    device::{EthernetDevice, LoopbackDevice},
    listen_table::ListenTable,
    router::{Router, Rule},
    service::Service,
    wrapper::SocketSetWrapper,
};

static LISTEN_TABLE: Lazy<ListenTable> = Lazy::new(ListenTable::new);
static SOCKET_SET: Lazy<SocketSetWrapper> = Lazy::new(SocketSetWrapper::new);
static SOCKET_WAITERS: Lazy<PollSet> = Lazy::new(PollSet::new);

static SERVICE: Once<Mutex<Service>> = Once::new();
static NET_CPUS: Once<alloc::vec::Vec<Arc<NetCpuState>>> = Once::new();
const NET_IRQ_SLOTS: usize = 8;
const UNBOUND_DEV: usize = usize::MAX;
static NET_IRQ_DEVICES: [AtomicUsize; NET_IRQ_SLOTS] = [
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
    AtomicUsize::new(UNBOUND_DEV),
];

const NET_POLL_BUDGET: usize = 64;

struct NetCpuState {
    queue: Mutex<VecDeque<usize>>,
    waitq: WaitQueue,
    timer_due: AtomicBool,
    next_deadline_ns: AtomicU64,
}

impl NetCpuState {
    fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            waitq: WaitQueue::new(),
            timer_due: AtomicBool::new(false),
            next_deadline_ns: AtomicU64::new(0),
        }
    }
}

fn get_service() -> ax_sync::MutexGuard<'static, Service> {
    SERVICE
        .get()
        .expect("Network service not initialized")
        .lock()
}

fn net_cpu_states() -> &'static [Arc<NetCpuState>] {
    NET_CPUS.get().expect("network cpu state not initialized")
}

pub(crate) fn register_socket_waiter(waker: &core::task::Waker) {
    SOCKET_WAITERS.register_waker(waker);
}

fn wake_socket_waiters() {
    SOCKET_WAITERS.wake();
}

fn refresh_deadline(cpu_id: usize, deadline: Option<ax_hal::time::TimeValue>) {
    let deadline_ns = deadline.map_or(0, |it| it.as_nanos() as u64);
    net_cpu_states()[cpu_id]
        .next_deadline_ns
        .store(deadline_ns, Ordering::Release);
}

fn enqueue_softirq(cpu_id: usize, dev_id: usize) {
    let state = &net_cpu_states()[cpu_id];
    state.queue.lock().push_back(dev_id);
    state.waitq.notify_one(false);
}

fn handle_net_irq(dev_id: usize) {
    let mut service = get_service();
    let cpu_id = service.device_cpu(dev_id);
    let events = service.handle_device_irq(dev_id);
    if events.is_empty() {
        return;
    }
    service.set_device_irq_enabled(dev_id, false);
    drop(service);
    enqueue_softirq(cpu_id, dev_id);
}

fn handle_slot_irq(slot: usize) {
    let dev_id = NET_IRQ_DEVICES[slot].load(Ordering::Acquire);
    if dev_id != UNBOUND_DEV {
        handle_net_irq(dev_id);
    }
}

fn net_irq_handler_0() {
    handle_slot_irq(0);
}

fn net_irq_handler_1() {
    handle_slot_irq(1);
}

fn net_irq_handler_2() {
    handle_slot_irq(2);
}

fn net_irq_handler_3() {
    handle_slot_irq(3);
}

fn net_irq_handler_4() {
    handle_slot_irq(4);
}

fn net_irq_handler_5() {
    handle_slot_irq(5);
}

fn net_irq_handler_6() {
    handle_slot_irq(6);
}

fn net_irq_handler_7() {
    handle_slot_irq(7);
}

fn irq_handler_for_slot(slot: usize) -> fn() {
    match slot {
        0 => net_irq_handler_0,
        1 => net_irq_handler_1,
        2 => net_irq_handler_2,
        3 => net_irq_handler_3,
        4 => net_irq_handler_4,
        5 => net_irq_handler_5,
        6 => net_irq_handler_6,
        7 => net_irq_handler_7,
        _ => panic!("too many network irq slots"),
    }
}

fn run_net_worker(cpu_id: usize) {
    let mut mask = AxCpuMask::new();
    mask.set(cpu_id, true);
    let _ = set_current_affinity(mask);

    let state = net_cpu_states()[cpu_id].clone();
    loop {
        let now_ns = ax_hal::time::wall_time_nanos();
        let deadline_ns = state.next_deadline_ns.load(Ordering::Acquire);
        if deadline_ns > 0 && now_ns >= deadline_ns {
            state.timer_due.store(true, Ordering::Release);
        }

        if state.queue.lock().is_empty() && !state.timer_due.load(Ordering::Acquire) {
            let timeout = if deadline_ns > 0 && deadline_ns > now_ns {
                core::time::Duration::from_nanos(deadline_ns - now_ns)
            } else {
                core::time::Duration::from_millis(50)
            };
            let _ = state.waitq.wait_timeout_until(timeout, || {
                !state.queue.lock().is_empty() || state.timer_due.load(Ordering::Acquire)
            });
        }

        let mut woke_sockets = false;
        while let Some(dev_id) = state.queue.lock().pop_front() {
            let mut service = get_service();
            let more = service.poll_device(dev_id, NET_POLL_BUDGET, &mut SOCKET_SET.inner.lock());
            refresh_deadline(cpu_id, service.next_deadline());
            service.set_device_irq_enabled(dev_id, !more);
            drop(service);
            woke_sockets = true;
            if more {
                state.queue.lock().push_back(dev_id);
            }
        }

        if state.timer_due.swap(false, Ordering::AcqRel) {
            let mut service = get_service();
            let more = service.poll_timers(&mut SOCKET_SET.inner.lock());
            refresh_deadline(cpu_id, service.next_deadline());
            drop(service);
            woke_sockets |= more;
        }

        if woke_sockets {
            wake_socket_waiters();
        }
    }
}

/// Initializes the network subsystem by NIC devices.
pub fn init_network(mut net_devs: AxDeviceContainer<AxNetDevice>) {
    info!("Initialize network subsystem...");

    NET_CPUS.call_once(|| {
        let mut states = alloc::vec::Vec::new();
        for _ in 0..ax_hal::cpu_num() {
            states.push(Arc::new(NetCpuState::new()));
        }
        states
    });

    let mut router = Router::new();
    let lo_dev = router.add_device(Box::new(LoopbackDevice::new()));

    let lo_ip = Ipv4Cidr::new(Ipv4Address::new(127, 0, 0, 1), 8);
    router.add_rule(Rule::new(
        lo_ip.into(),
        None,
        lo_dev,
        lo_ip.address().into(),
    ));

    let mut first_eth_ip = None;
    let mut nic_index = 0usize;
    while let Some(dev) = net_devs.take_one() {
        info!("  use NIC {}: {:?}", nic_index, dev.device_name());

        let name = alloc::format!("eth{nic_index}");
        let eth_address = EthernetAddress(dev.mac_address().0);
        let eth_ip = Ipv4Cidr::new(IP.parse().expect("Invalid IPv4 address"), IP_PREFIX);

        let eth_dev = router.add_device(Box::new(EthernetDevice::new(name.clone(), dev, eth_ip)));
        router.add_rule(Rule::new(
            Ipv4Cidr::new(Ipv4Address::UNSPECIFIED, 0).into(),
            Some(GATEWAY.parse().expect("Invalid gateway address")),
            eth_dev,
            eth_ip.address().into(),
        ));

        info!("{name}:");
        info!("  mac:  {}", eth_address);
        info!("  ip:   {}", eth_ip);

        if first_eth_ip.is_none() {
            first_eth_ip = Some(eth_ip);
        }

        nic_index += 1;
    }
    if nic_index == 0 {
        warn!("  No network device found!");
    }

    for dev in &router.devices {
        info!("Device: {}", dev.name());
    }

    let mut service = Service::new(router);
    service.iface.update_ip_addrs(|ip_addrs| {
        ip_addrs.push(lo_ip.into()).unwrap();
        if let Some(eth0_ip) = first_eth_ip {
            ip_addrs.push(eth0_ip.into()).unwrap();
        }
    });
    SERVICE.call_once(|| Mutex::new(service));

    register_timer_callback(|_| {
        for (cpu_id, state) in net_cpu_states().iter().enumerate() {
            let deadline_ns = state.next_deadline_ns.load(Ordering::Acquire);
            if deadline_ns != 0 && ax_hal::time::wall_time_nanos() >= deadline_ns {
                state.timer_due.store(true, Ordering::Release);
                state.waitq.notify_one(false);
                trace!("net timer due on cpu {}", cpu_id);
            }
        }
    });

    for cpu_id in 0..ax_hal::cpu_num() {
        spawn_with_name(
            move || run_net_worker(cpu_id),
            alloc::format!("ksoftirqd/net/{}", cpu_id),
        );
    }

    let service = get_service();
    let mut irq_slot = 0usize;
    for dev_id in 0..service.router_device_count() {
        if let Some(irq) = service.device_irq_num(dev_id) {
            NET_IRQ_DEVICES[irq_slot].store(dev_id, Ordering::Release);
            ax_hal::irq::register(irq, irq_handler_for_slot(irq_slot));
            irq_slot += 1;
        }
    }
    drop(service);
}

/// Init vsock subsystem by vsock devices.
#[cfg(feature = "vsock")]
pub fn init_vsock(mut vsock_devs: AxDeviceContainer<AxVsockDevice>) {
    use self::device::register_vsock_device;
    info!("Initialize vsock subsystem...");
    if let Some(dev) = vsock_devs.take_one() {
        info!("  use vsock 0: {:?}", dev.device_name());
        if let Err(e) = register_vsock_device(dev) {
            warn!("Failed to initialize vsock device: {:?}", e);
        }
    } else {
        warn!("  No vsock device found!");
    }
}

/// Poll all network interfaces for new events.
pub fn poll_interfaces() {
    while get_service().poll(&mut SOCKET_SET.inner.lock()) {
        wake_socket_waiters();
    }
}
