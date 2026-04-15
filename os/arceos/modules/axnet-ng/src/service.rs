use ax_driver::prelude::NetIrqEvents;
use ax_hal::time::{NANOS_PER_MICROS, TimeValue, wall_time_nanos};
use smoltcp::{
    iface::{Interface, SocketSet},
    time::Instant,
    wire::{HardwareAddress, IpAddress, IpListenEndpoint},
};

use crate::router::Router;

fn now() -> Instant {
    Instant::from_micros_const((wall_time_nanos() / NANOS_PER_MICROS) as i64)
}

pub struct Service {
    pub iface: Interface,
    router: Router,
    next_deadline: Option<TimeValue>,
}
impl Service {
    pub fn new(mut router: Router) -> Self {
        let config = smoltcp::iface::Config::new(HardwareAddress::Ip);
        let iface = Interface::new(config, &mut router, now());

        Self {
            iface,
            router,
            next_deadline: None,
        }
    }

    pub fn poll(&mut self, sockets: &mut SocketSet) -> bool {
        let timestamp = now();

        self.router.poll(timestamp);
        self.iface.poll(timestamp, &mut self.router, sockets);
        let result = self.router.dispatch(timestamp);
        self.next_deadline = self
            .iface
            .poll_at(now(), sockets)
            .map(|t| TimeValue::from_micros(t.total_micros() as _));
        result
    }

    pub fn poll_device(&mut self, index: usize, budget: usize, sockets: &mut SocketSet) -> bool {
        let timestamp = now();
        let status = self.router.poll_device(index, budget, timestamp);
        if status.work_done == 0
            && !status.link_changed
            && let Some(deadline) = self.next_deadline
            && ax_hal::time::wall_time() < deadline
        {
            return false;
        }

        self.iface.poll(timestamp, &mut self.router, sockets);
        let result = self.router.dispatch(timestamp);
        self.next_deadline = self
            .iface
            .poll_at(now(), sockets)
            .map(|t| TimeValue::from_micros(t.total_micros() as _));
        result || status.more_rx || status.more_tx
    }

    pub fn poll_timers(&mut self, sockets: &mut SocketSet) -> bool {
        let timestamp = now();
        self.iface.poll(timestamp, &mut self.router, sockets);
        let result = self.router.dispatch(timestamp);
        self.next_deadline = self
            .iface
            .poll_at(now(), sockets)
            .map(|t| TimeValue::from_micros(t.total_micros() as _));
        result
    }

    pub fn get_source_address(&self, dst_addr: &IpAddress) -> IpAddress {
        let Some(rule) = self.router.table.lookup(dst_addr) else {
            panic!("no route to destination: {dst_addr}");
        };
        rule.src
    }

    pub fn device_mask_for(&self, endpoint: &IpListenEndpoint) -> u32 {
        match endpoint.addr {
            Some(addr) => self
                .router
                .table
                .lookup(&addr)
                .map_or(0, |it| 1u32 << it.dev),
            None => u32::MAX,
        }
    }

    pub fn next_deadline(&self) -> Option<TimeValue> {
        self.next_deadline
    }

    pub fn device_cpu(&self, index: usize) -> usize {
        index % ax_hal::cpu_num()
    }

    pub fn router_device_count(&self) -> usize {
        self.router.devices.len()
    }

    pub fn device_irq_num(&self, index: usize) -> Option<usize> {
        self.router.devices.get(index).and_then(|dev| dev.irq_num())
    }

    pub fn handle_device_irq(&mut self, index: usize) -> NetIrqEvents {
        self.router
            .devices
            .get_mut(index)
            .map_or(NetIrqEvents::empty(), |dev| dev.handle_irq())
    }

    pub fn set_device_irq_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(dev) = self.router.devices.get_mut(index) {
            dev.set_irq_enabled(enabled);
        }
    }
}
