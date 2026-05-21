use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, Ordering};

use ax_lazyinit::LazyInit;

static KALLSYMS_READY: AtomicBool = AtomicBool::new(false);

static KALLSYMS_TABLE: LazyInit<KallsymsTable> = LazyInit::new();

struct KallsymsTable {
    symbols: Vec<KallsymEntry>,
}

struct KallsymEntry {
    addr: u64,
    sym_type: u8,
    name: String,
}

impl KallsymsTable {
    fn from_kallsyms_str(data: &str) -> Self {
        let mut symbols = Vec::new();
        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() < 3 {
                continue;
            }
            let addr = match u64::from_str_radix(parts[0], 16) {
                Ok(a) => a,
                Err(_) => continue,
            };
            let sym_type = parts[1].as_bytes().first().copied().unwrap_or(b'?');
            let name = parts[2].to_string();
            symbols.push(KallsymEntry {
                addr,
                sym_type,
                name,
            });
        }
        symbols.sort_by_key(|e| e.addr);
        Self { symbols }
    }

    fn lookup_address(&self, addr: u64) -> Option<(&str, u64)> {
        let idx = self
            .symbols
            .binary_search_by_key(&addr, |e| e.addr)
            .unwrap_or_else(|idx| idx.saturating_sub(1));
        if idx >= self.symbols.len() {
            return None;
        }
        let entry = &self.symbols[idx];
        if addr < entry.addr {
            return None;
        }
        let next_addr = self
            .symbols
            .get(idx + 1)
            .map(|e| e.addr)
            .unwrap_or(entry.addr + 0x1000);
        if addr >= next_addr {
            return Some((&entry.name, addr - entry.addr));
        }
        Some((&entry.name, addr - entry.addr))
    }

    fn lookup_name(&self, name: &str) -> Option<u64> {
        self.symbols.iter().find(|e| e.name == name).map(|e| e.addr)
    }

    fn format_all(&self) -> String {
        let mut buf = String::with_capacity(self.symbols.len() * 60);
        for entry in &self.symbols {
            buf.push_str(&alloc::format!(
                "{:016x} {} {}\n",
                entry.addr,
                entry.sym_type as char,
                entry.name
            ));
        }
        buf
    }
}

pub fn kallsyms_init(data: &str) {
    if data.trim().is_empty() {
        info!("kallsyms: no symbol data available, skipping initialization");
        return;
    }
    let table = KallsymsTable::from_kallsyms_str(data);
    let count = table.symbols.len();
    KALLSYMS_TABLE.init_once(table);
    KALLSYMS_READY.store(true, Ordering::Release);
    info!("kallsyms: initialized with {count} symbols");
}

pub fn kallsyms_lookup_address(addr: u64) -> Option<(&'static str, u64)> {
    KALLSYMS_TABLE.get().and_then(|t| t.lookup_address(addr))
}

pub fn kallsyms_lookup_name(name: &str) -> Option<u64> {
    KALLSYMS_TABLE.get().and_then(|t| t.lookup_name(name))
}

pub fn kallsyms_format_all() -> Option<String> {
    KALLSYMS_TABLE.get().map(|t| t.format_all())
}

pub fn is_kallsyms_ready() -> bool {
    KALLSYMS_READY.load(Ordering::Acquire)
}
